//! Cell-local consumption. Physical spaces and recursive child cursors are distinct.
use std::sync::Arc;

use super::{
    FlowBlock, FlowCellInput, FragmentFit, GeometryError, LinePlacement, NestedTablePlacement,
    PageArea, Rect, SplitPolicy, TableCursor,
};

#[derive(Debug, Clone, Default)]
pub(super) struct FlowCursor {
    pub block: usize,
    space_left: Option<f64>,
    child: Option<Box<TableCursor>>,
    anchor_started: bool,
    anchor_tail: Option<f64>,
}

pub(super) struct FlowFit {
    pub next: FlowCursor,
    pub height: f64,
    /// Next flow origin, distinct from occupied height for overlapping line gaps.
    pub advance: f64,
    pub progressed: bool,
    pub required: f64,
    pub lines: Vec<LinePlacement>,
    pub tables: Vec<NestedTablePlacement>,
    /// Only body composition can admit an indivisible TAC row on an empty
    /// page. Its real extent is retained; this is not a larger generic budget.
    pub body_inline_overflow: bool,
}

/// Preserve a proven complete child extent when subtraction loses a low bit
/// (1.2 - 1.0 < 0.2). Acceptance is in the parent's local coordinate system,
/// using the same prefix + child height that advances its pen. A real deficit
/// still passes only the remaining budget to the child's fragmentation query.
fn child_budget(total: f64, pen: f64, prefix: f64, child_height: f64) -> f64 {
    let remaining = ((total - pen).max(0.0) - prefix).max(0.0);
    if pen + (prefix + child_height) <= total {
        remaining.max(child_height)
    } else {
        remaining
    }
}

impl FlowCursor {
    pub(super) fn starts_stored_frame(&self, cell: &FlowCellInput) -> bool {
        matches!(
            cell.blocks.get(self.block),
            Some(FlowBlock::StoredFrameStart)
        ) || self
            .child
            .as_ref()
            .is_some_and(|child| child.starts_stored_frame())
    }

    pub fn fit(&self, cell: &FlowCellInput, area: Rect) -> Result<FlowFit, GeometryError> {
        self.fit_with_page_height(cell, area, None)
    }

    pub fn fit_with_page_height(
        &self,
        cell: &FlowCellInput,
        area: Rect,
        page_height: Option<f64>,
    ) -> Result<FlowFit, GeometryError> {
        self.fit_until(cell, area, cell.blocks.len(), page_height)
    }

    pub fn fit_until(
        &self,
        cell: &FlowCellInput,
        area: Rect,
        end: usize,
        page_height: Option<f64>,
    ) -> Result<FlowFit, GeometryError> {
        self.fit_cell_until(cell, area, end, page_height, false)
    }

    pub(super) fn fit_cell_until(
        &self,
        cell: &FlowCellInput,
        area: Rect,
        end: usize,
        page_height: Option<f64>,
        stored_frames: bool,
    ) -> Result<FlowFit, GeometryError> {
        self.fit_frame_until(cell, area, end, page_height, stored_frames, false)
    }

    pub(super) fn fit_body_until(
        &self,
        cell: &FlowCellInput,
        area: Rect,
        end: usize,
        page_height: f64,
    ) -> Result<FlowFit, GeometryError> {
        self.fit_frame_until(
            cell,
            area,
            end,
            Some(page_height),
            false,
            area.height == page_height,
        )
    }

    fn fit_frame_until(
        &self,
        cell: &FlowCellInput,
        area: Rect,
        end: usize,
        page_height: Option<f64>,
        stored_frames: bool,
        empty_body: bool,
    ) -> Result<FlowFit, GeometryError> {
        let mut result = FlowFit {
            next: self.clone(),
            height: 0.0,
            advance: 0.0,
            progressed: false,
            required: 0.0,
            lines: Vec::new(),
            tables: Vec::new(),
            body_inline_overflow: false,
        };
        let mut pen = 0.0;
        let initial_frame = stored_frames
            && self.block == 0
            && cell
                .blocks
                .iter()
                .any(|block| matches!(block, FlowBlock::StoredFrameStart));
        if initial_frame {
            // from_flow_rows inserts the cell top inset as block0. Like the
            // inset after a saved marker, it is not authored content and must
            // be accepted with the first real unit, never on its own page.
            pen = cell.padding.top;
            result.next.block = 1;
        }
        if stored_frames
            && self
                .child
                .as_ref()
                .is_some_and(|child| child.starts_stored_frame())
        {
            // A cut owned by a descendant opens a new physical frame for this
            // ancestor too. Keep the inset here, not in the child's text origin.
            pen += cell.padding.top;
        }
        let mut required_inset = 0.0;
        while let Some(block) = cell.blocks[..end].get(result.next.block) {
            // Consume bookkeeping/following-line gap on the owner page, but
            // never place a second content unit in the overflowing row's tail.
            if result.body_inline_overflow
                && !matches!(block, FlowBlock::FollowingLineGap(_))
                && !matches!(block, FlowBlock::Space(h) if *h == 0.0)
            {
                break;
            }
            // The last accepted content unit owns its terminating physical
            // inset. Reserve it BEFORE fitting that unit, not afterwards in
            // paint and not as an empty padding-only continuation page.
            let terminal_inset = if stored_frames {
                match cell.blocks.get(result.next.block + 1) {
                    Some(FlowBlock::StoredFrameTail {
                        paragraph_after, ..
                    }) => cell.padding.bottom + paragraph_after,
                    Some(FlowBlock::StoredFrameStart)
                        if !matches!(block, FlowBlock::StoredFrameTail { .. }) =>
                    {
                        cell.padding.bottom
                    }
                    _ if matches!(
                        block,
                        FlowBlock::Table { .. } | FlowBlock::AnchoredTable { .. }
                    ) && block.has_stored_frame_cut() =>
                    {
                        cell.padding.bottom
                    }
                    _ => 0.0,
                }
            } else {
                0.0
            };
            required_inset = terminal_inset;
            if terminal_inset > area.height {
                result.required = block.height();
                break;
            }
            let area = Rect {
                height: (area.height - terminal_inset).max(0.0),
                ..area
            };
            let available = (area.height - pen).max(0.0);
            match block {
                FlowBlock::StoredFrameStart => {
                    // Leave the marker with the following content. A retry on
                    // the next fragment consumes it without creating a blank
                    // page or duplicating the preceding line/physical space.
                    if stored_frames && result.progressed {
                        // Page ownership is the unconsumed marker, not a claim
                        // on all remaining paper. End at the same occupied
                        // envelope used by placement, including the cell inset.
                        pen = result.height.max(pen) + cell.padding.bottom;
                        result.height = pen;
                        break;
                    }
                    if stored_frames {
                        // A saved frame restarts at the padded cell origin.
                        // This is physical inset, not another authored blank
                        // line. Do not replay it on ordinary content cuts.
                        pen += cell.padding.top;
                    }
                }
                FlowBlock::StoredFrameTail {
                    spaces,
                    paragraph_after,
                } => {
                    if stored_frames {
                        pen = result.height.max(pen) + paragraph_after;
                    } else {
                        // Intact/Never queries retain the ordinary tail and
                        // its original addition order (including zero bands).
                        for space in spaces {
                            pen += space;
                        }
                    }
                }
                FlowBlock::AnchoredTable {
                    owner,
                    host,
                    host_advance,
                    offset_x,
                    offset_y,
                    available_width,
                    top,
                    bottom,
                    plan,
                } => {
                    let host_height = host.as_ref().map_or(0.0, |h| h.bounds.y + h.bounds.height);
                    if result.next.anchor_tail.is_none() {
                        let first = !result.next.anchor_started;
                        // Host and first child fragment are one acceptance transaction.
                        if first && host_height > available {
                            result.required = host_height;
                            break;
                        }
                        let cursor = result
                            .next
                            .child
                            .as_deref()
                            .cloned()
                            .unwrap_or_else(|| TableCursor::new(Arc::clone(plan)));
                        // The source paragraph displacement belongs only to the
                        // first accepted fragment. Margin belongs to every frame.
                        // A failed query consumes neither the host nor this band.
                        let initial_top = top + if first { *offset_y } else { 0.0 };
                        if initial_top > available {
                            result.required = initial_top;
                            break;
                        }
                        // Every floating fragment stays inside its outer
                        // margin. Line-vs-cell break permission does not make
                        // that physical inset disappear on continuations.
                        let fragment_bottom = *bottom;
                        if initial_top + fragment_bottom > available {
                            result.required = initial_top + fragment_bottom;
                            break;
                        }
                        let budget = child_budget(
                            area.height,
                            pen,
                            initial_top + fragment_bottom,
                            plan.height,
                        );
                        match cursor.fit_in_frame(
                            PageArea {
                                bounds: Rect {
                                    x: area.x + offset_x,
                                    y: area.y + pen + initial_top,
                                    width: *available_width,
                                    height: budget,
                                },
                            },
                            page_height.map(|h| (h - top - fragment_bottom).max(0.0)),
                            stored_frames,
                        )? {
                            FragmentFit::Placed(fragment) => {
                                let child_end = initial_top + fragment.reserved_height();
                                let used = if first {
                                    child_end.max(host_height)
                                } else {
                                    child_end
                                };
                                if first {
                                    if let Some(host) = host {
                                        result.lines.push(LinePlacement {
                                            owner: host.owner,
                                            bounds: Rect {
                                                x: area.x + host.bounds.x,
                                                y: area.y + pen + host.bounds.y,
                                                ..host.bounds
                                            },
                                        });
                                    }
                                }
                                result.tables.push(NestedTablePlacement {
                                    owner: *owner,
                                    placement: Box::new(fragment.placement().clone()),
                                });
                                let next = fragment.continuation();
                                let physical = if next.is_complete() {
                                    used
                                } else {
                                    (child_end + fragment_bottom).max(if first {
                                        host_height
                                    } else {
                                        0.0
                                    })
                                };
                                pen = if fragment.reserved_height() == budget {
                                    area.height
                                        - if next.is_complete() {
                                            fragment_bottom
                                        } else {
                                            0.0
                                        }
                                } else {
                                    pen + physical
                                };
                                result.height = result.height.max(pen);
                                result.progressed = true;
                                result.next.anchor_started = true;
                                if !next.is_complete() {
                                    if stored_frames && next.starts_stored_frame() {
                                        pen += cell.padding.bottom;
                                        result.height = result.height.max(pen);
                                    }
                                    result.next.child = Some(Box::new(next));
                                    break;
                                }
                                result.next.child = None;
                                let end = if first {
                                    (child_end + bottom).max(*host_advance).max(used)
                                } else {
                                    child_end + bottom
                                };
                                result.next.anchor_tail = Some(end - used);
                            }
                            FragmentFit::DoesNotFit {
                                required_height, ..
                            } => {
                                result.required = (initial_top + fragment_bottom + required_height)
                                    .max(if first { host_height } else { 0.0 });
                                break;
                            }
                            FragmentFit::Complete => {
                                return Err(GeometryError::InconsistentAtomicPlan)
                            }
                        }
                    }
                    // Trailing physical margin is not child content. Carry only
                    // its remainder; never restart the already completed child.
                    let tail = result.next.anchor_tail.unwrap();
                    let take = tail.min((area.height - pen).max(0.0));
                    pen += take;
                    result.height = result.height.max(pen);
                    result.progressed |= take > 0.0;
                    if take < tail {
                        result.next.anchor_tail = Some(tail - take);
                        result.required = tail - take;
                        break;
                    }
                    result.next.anchor_tail = None;
                    result.next.anchor_started = false;
                }
                FlowBlock::FollowingLineGap(height) => {
                    // Only composition can identify following-line spacing.
                    // It has no physical remainder in the next page frame.
                    pen += height.min((area.height - pen).max(0.0));
                }
                FlowBlock::Space(height) => {
                    let left = result.next.space_left.unwrap_or(*height);
                    // Compare in the same local frame used to measure content.
                    // Adding an unrelated page origin on both sides changes
                    // rounding and can leave a phantom fraction of padding.
                    let taken = if pen + left <= area.height {
                        left
                    } else {
                        left.min(available)
                    };
                    pen += taken;
                    result.height = result.height.max(pen);
                    if left > taken {
                        result.next.space_left = Some(left - taken);
                        result.progressed |= taken > 0.0;
                        result.required = left - taken;
                        break;
                    }
                    result.next.space_left = None;
                }
                FlowBlock::Lines {
                    height,
                    advance,
                    lines,
                } => {
                    if area.y + pen + height > area.y + area.height {
                        result.required = *height;
                        break;
                    }
                    result.lines.extend(lines.iter().map(|line| LinePlacement {
                        owner: line.owner,
                        bounds: Rect {
                            x: area.x + line.bounds.x,
                            y: area.y + pen + line.bounds.y,
                            width: line.bounds.width,
                            height: line.bounds.height,
                        },
                    }));
                    result.height = result.height.max(pen + height);
                    pen += advance;
                }
                FlowBlock::InlineTables {
                    height,
                    advance,
                    tables,
                    lines,
                } => {
                    // A signed top margin can reach into preceding space,
                    // but not above this physical fragment. First-fragment
                    // overhang needs an explicit origin contract; do not
                    // invent padding, clip it, or silently keep retrying.
                    if tables.iter().any(|child| pen + child.y < 0.0) {
                        return Err(GeometryError::Unsupported(
                            "inline table extends above fragment origin",
                        ));
                    }
                    if pen + height > area.height {
                        // An unbreakable TAC row cannot be deferred forever
                        // when it is taller than a fresh page. Hancom keeps
                        // that first row intact, even beyond the body/paper.
                        // A partially occupied page still defers the row, and
                        // cell/anchor fits never receive this permission.
                        if !empty_body
                            || pen != 0.0
                            || !result.lines.is_empty()
                            || !result.tables.is_empty()
                            || tables.is_empty()
                        {
                            result.required = *height;
                            break;
                        }
                        result.body_inline_overflow = true;
                    }
                    // Fit the entire group transactionally. No child cursor is
                    // committed if any sibling cannot preserve its complete box.
                    let mut placed = Vec::with_capacity(tables.len());
                    for child in tables {
                        let fit = TableCursor::new(child.plan.clone()).fit(PageArea {
                            bounds: Rect {
                                x: area.x + child.x,
                                y: area.y + pen + child.y,
                                width: child.plan.width,
                                height: child.plan.height,
                            },
                        })?;
                        let FragmentFit::Placed(fragment) = fit else {
                            return Err(GeometryError::InconsistentAtomicPlan);
                        };
                        if !fragment.continuation().is_complete() {
                            return Err(GeometryError::InconsistentAtomicPlan);
                        }
                        placed.push(NestedTablePlacement {
                            owner: child.owner,
                            placement: Box::new(fragment.placement().clone()),
                        });
                    }
                    result.tables.extend(placed);
                    result.lines.extend(lines.iter().map(|line| LinePlacement {
                        owner: line.owner,
                        bounds: Rect {
                            x: area.x + line.bounds.x,
                            y: area.y + pen + line.bounds.y,
                            width: line.bounds.width,
                            height: line.bounds.height,
                        },
                    }));
                    result.height = result.height.max(pen + height);
                    pen += advance;
                }
                FlowBlock::Table {
                    owner,
                    offset_x,
                    restart_top,
                    plan,
                } => {
                    // The source offset belongs to the initial anchor only.
                    // A resumed fragment (including atomic deferral) reserves
                    // its top outer margin in the same budget used for paint.
                    let prefix = if result.next.child.is_some() {
                        *restart_top
                    } else {
                        0.0
                    };
                    let cursor = result
                        .next
                        .child
                        .as_deref()
                        .cloned()
                        .unwrap_or_else(|| TableCursor::new(Arc::clone(plan)));
                    let budget = child_budget(area.height, pen, prefix, plan.height);
                    match cursor.fit_in_frame(
                        PageArea {
                            bounds: Rect {
                                x: area.x + offset_x,
                                y: area.y + pen + prefix,
                                width: area.width - offset_x,
                                height: budget,
                            },
                        },
                        page_height.map(|h| (h - restart_top).max(0.0)),
                        stored_frames,
                    )? {
                        FragmentFit::Placed(fragment) => {
                            let reserved = prefix + fragment.reserved_height();
                            // Child fit already owns the fragment budget check.
                            // Do not recheck its accepted height by subtracting
                            // the parent pen (1.2 - 1.0 is not exactly 0.2).
                            // Only guard the prefix when its clamped child area
                            // could otherwise admit an empty zero-height child.
                            if area.y + pen + prefix > area.y + area.height {
                                result.required = reserved;
                                result.next.child = Some(Box::new(cursor));
                                break;
                            }
                            pen = if fragment.reserved_height() == budget {
                                area.height
                            } else {
                                pen + reserved
                            };
                            result.height = result.height.max(pen);
                            result.tables.push(NestedTablePlacement {
                                owner: *owner,
                                placement: Box::new(fragment.placement().clone()),
                            });
                            result.progressed = true;
                            let next = fragment.continuation();
                            if !next.is_complete() {
                                if stored_frames && next.starts_stored_frame() {
                                    pen += cell.padding.bottom;
                                    result.height = result.height.max(pen);
                                }
                                result.next.child = Some(Box::new(next));
                                break;
                            }
                            result.next.child = None;
                        }
                        FragmentFit::DoesNotFit {
                            required_height, ..
                        } => {
                            result.required = prefix + required_height;
                            result.next.child = Some(Box::new(cursor));
                            break;
                        }
                        FragmentFit::Complete => {
                            result.next.child = None;
                        }
                    }
                }
            }
            result.height = result.height.max(pen);
            result.next.block += 1;
            // Zero padding is bookkeeping, not enough progress to emit a blank
            // fragment in front of a blocked line/table.
            result.progressed |= !matches!(block, FlowBlock::StoredFrameStart)
                && !matches!(block, FlowBlock::Space(h) if *h == 0.0)
                && !matches!(block, FlowBlock::FollowingLineGap(_));
        }
        if result.required > 0.0 || required_inset > area.height {
            result.required += required_inset;
        }
        if !result.progressed && stored_frames && (initial_frame || self.starts_stored_frame(cell))
        {
            // A frame inset alone cannot commit an empty fragment. A failed
            // first-unit query keeps both marker and inset for the retry.
            result.required += pen;
            result.next = self.clone();
            result.height = 0.0;
            pen = 0.0;
        }
        result.advance = pen;
        Ok(result)
    }
}
