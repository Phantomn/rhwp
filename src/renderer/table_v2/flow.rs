//! Cell-local consumption. Physical spaces and recursive child cursors are distinct.
use std::sync::Arc;

use super::{
    FlowBlock, FlowCellInput, FragmentFit, GeometryError, LinePlacement, NestedTablePlacement,
    PageArea, Rect, TableCursor,
};

#[derive(Debug, Clone, Default)]
pub(super) struct FlowCursor {
    pub block: usize,
    space_left: Option<f64>,
    child: Option<Box<TableCursor>>,
}

pub(super) struct FlowFit {
    pub next: FlowCursor,
    pub height: f64,
    pub progressed: bool,
    pub required: f64,
    pub lines: Vec<LinePlacement>,
    pub tables: Vec<NestedTablePlacement>,
}

impl FlowCursor {
    pub fn fit(&self, cell: &FlowCellInput, area: Rect) -> Result<FlowFit, GeometryError> {
        let mut result = FlowFit {
            next: self.clone(),
            height: 0.0,
            progressed: false,
            required: 0.0,
            lines: Vec::new(),
            tables: Vec::new(),
        };
        let mut pen = 0.0;
        while let Some(block) = cell.blocks.get(result.next.block) {
            let available = (area.height - pen).max(0.0);
            match block {
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
                } => {
                    if area.y + pen + height > area.y + area.height {
                        result.required = *height;
                        break;
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
                    match cursor.fit(PageArea {
                        bounds: Rect {
                            x: area.x + offset_x,
                            y: area.y + pen + prefix,
                            width: area.width - offset_x,
                            height: (available - prefix).max(0.0),
                        },
                    })? {
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
                            pen += reserved;
                            result.height = result.height.max(pen);
                            result.tables.push(NestedTablePlacement {
                                owner: *owner,
                                placement: Box::new(fragment.placement().clone()),
                            });
                            result.progressed = true;
                            let next = fragment.continuation();
                            if !next.is_complete() {
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
            result.progressed |= !matches!(block, FlowBlock::Space(h) if *h == 0.0);
        }
        Ok(result)
    }
}
