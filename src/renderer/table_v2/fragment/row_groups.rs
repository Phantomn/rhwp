//! Logical row cuts, including boundaries inside a span.
//! Content stays atomic in this path: the first physical cell fragment owns it
//! once, and subsequent fragments carry only the remaining physical rows.
//! Cell-internal content cuts remain a separate, unsupported rowspan contract.
use super::*;

impl TableCursor {
    /// An unrelated rowspan must not make every other cell in the table atomic.
    /// Spanning owners still use the whole-content path. A row containing only
    /// non-spanning owners can use the same saved/oversize cuts as an ordinary
    /// table; any incoming span has already consumed its content in the prefix.
    pub(super) fn fit_mixed_rows(
        &self,
        area: PageArea,
        end_row: usize,
        atomic: bool,
        page_height: Option<f64>,
        stored_frame: bool,
    ) -> Result<FragmentFit, GeometryError> {
        let candidate = (self.row..end_row).find(|&row| {
            self.plan.grid[row].iter().all(|track| track.row_span == 1)
                && ((self.plan.policy == SplitPolicy::WithinCells
                    && self.plan.row_heights[self.row..=row].iter().sum::<f64>()
                        > area.bounds.height)
                    || self.plan.rows[row].cells.iter().any(|cell| {
                        cell.blocks
                            .iter()
                            .any(super::super::FlowBlock::has_stored_frame_cut)
                    })
                    || page_height.is_some_and(|height| self.plan.row_heights[row] > height))
        });
        if atomic || self.plan.policy == SplitPolicy::Never || candidate.is_none() {
            return self.fit_row_groups(area, end_row, atomic, true);
        }
        let start = candidate.unwrap();
        let mut prefix = if start > self.row {
            match self.fit_row_groups(area, start, false, false)? {
                FragmentFit::Placed(fragment) if fragment.continuation.row == start => {
                    Some(fragment)
                }
                _ => return self.fit_row_groups(area, start, false, true),
            }
        } else {
            None
        };
        let cursor = prefix
            .as_ref()
            .map_or_else(|| self.clone(), |p| p.continuation());
        let used = prefix.as_ref().map_or(0.0, |p| p.reserved_height());
        let remaining = PageArea {
            bounds: Rect {
                y: area.bounds.y + used,
                height: (area.bounds.height - used).max(0.0),
                ..area.bounds
            },
        };
        // Stop before a new spanning owner; it must go through row_cut_required.
        let stop = (start..end_row)
            .find(|&row| self.plan.grid[row].iter().any(|track| track.row_span > 1))
            .unwrap_or(end_row);
        let FragmentFit::Placed(mut suffix) =
            cursor.fit_plain_rows(remaining, stop, false, page_height, stored_frame)?
        else {
            if prefix.is_some() {
                return self.fit_row_groups(area, start, false, true);
            }
            return cursor.fit_plain_rows(remaining, stop, false, page_height, stored_frame);
        };
        // Incoming spans own no new content here. Their physical continuation
        // follows the accepted row fragments, not the intact row heights.
        for row in 0..start {
            for track in &self.plan.grid[row] {
                let source_end = row + track.row_span;
                if source_end <= start {
                    continue;
                }
                let end = source_end.min(suffix.rows.end);
                let bottom = suffix
                    .placement
                    .cells
                    .iter()
                    .filter(|cell| cell.row >= start && cell.row < end)
                    .map(|cell| cell.bounds.y + cell.bounds.height)
                    .fold(remaining.bounds.y, f64::max);
                if let Some(cell) = prefix.as_mut().and_then(|p| {
                    p.placement
                        .cells
                        .iter_mut()
                        .find(|cell| cell.row == row && cell.column == track.column)
                }) {
                    cell.bounds.height = bottom - cell.bounds.y;
                    cell.visible_rows.end = end;
                    cell.partial =
                        cell.visible_rows.start != row || suffix.continuation.row < source_end;
                } else {
                    suffix.placement.cells.push(CellPlacement {
                        partial: true,
                        row,
                        row_span: track.row_span,
                        visible_rows: start..end,
                        column: track.column,
                        column_span: track.span,
                        bounds: Rect {
                            x: area.bounds.x + track.left,
                            y: remaining.bounds.y,
                            width: track.width,
                            height: bottom - remaining.bounds.y,
                        },
                        content_origin: (area.bounds.x + track.left, remaining.bounds.y),
                        lines: Vec::new(),
                        tables: Vec::new(),
                    });
                }
            }
        }
        if let Some(mut prefix) = prefix {
            prefix.placement.cells.append(&mut suffix.placement.cells);
            suffix.placement.cells = prefix.placement.cells;
        }
        suffix.placement.bounds.y = area.bounds.y;
        suffix.placement.bounds.height += used;
        suffix.rows.start = self.row;
        if suffix.continuation.row == stop && stop < end_row {
            let rest = PageArea {
                bounds: Rect {
                    y: area.bounds.y + suffix.reserved_height(),
                    height: (area.bounds.height - suffix.reserved_height()).max(0.0),
                    ..area.bounds
                },
            };
            if let FragmentFit::Placed(tail) = suffix.continuation.fit_mixed_rows(
                rest,
                end_row,
                false,
                page_height,
                stored_frame,
            )? {
                suffix.placement.bounds.height += tail.reserved_height();
                suffix.rows.end = tail.rows.end;
                suffix.continuation = tail.continuation;
                for cell in tail.placement.cells {
                    if let Some(existing) = suffix
                        .placement
                        .cells
                        .iter_mut()
                        .find(|old| old.row == cell.row && old.column == cell.column)
                    {
                        // The same incoming span crossed an internal routing
                        // boundary, not a physical page. Merge its frame; do
                        // not paint a second cell border or replay its payload.
                        if !cell.lines.is_empty() || !cell.tables.is_empty() {
                            return Err(GeometryError::InconsistentAtomicPlan);
                        }
                        existing.bounds.height =
                            cell.bounds.y + cell.bounds.height - existing.bounds.y;
                        existing.visible_rows.end = cell.visible_rows.end;
                        existing.partial = existing.visible_rows.start != existing.row
                            || suffix.continuation.row < existing.row + existing.row_span;
                    } else {
                        suffix.placement.cells.push(cell);
                    }
                }
            }
        }
        // Row-boundary continuation can own a trailing physical band. A saved
        // cell-internal cut instead closes at its accepted line/padding envelope;
        // being next to an open span is not permission to stretch that frame.
        if suffix.continuation.row == suffix.rows.end
            && self.cut_crosses_span(suffix.continuation.row)
        {
            let bottom = area.bounds.y + area.bounds.height;
            for cell in &mut suffix.placement.cells {
                if cell.visible_rows.end == suffix.rows.end {
                    cell.bounds.height = bottom - cell.bounds.y;
                }
            }
            suffix.placement.bounds.height = area.bounds.height;
        }
        // Recompose only the already accepted first-fragment spanning owners
        // at their final physical alignment. No continuation owns their text.
        for placed in &mut suffix.placement.cells {
            if placed.row_span == 1 || placed.row < self.row {
                continue;
            }
            let slot = self.plan.grid[placed.row]
                .iter()
                .position(|track| track.column == placed.column)
                .unwrap();
            let track = &self.plan.grid[placed.row][slot];
            let input = &self.plan.rows[placed.row].cells[slot];
            let slack = placed.bounds.height - track.content_height;
            nonnegative(slack, "mixed spanning content slack")?;
            let (offset, available) = track
                .alignment
                .content_window(placed.bounds.height, track.content_height);
            let fit = FlowCursor::default().fit(
                input,
                Rect {
                    x: placed.bounds.x + input.padding.left,
                    y: placed.bounds.y + offset,
                    width: input.width,
                    height: available,
                },
            )?;
            if fit.next.block != input.blocks.len() {
                return Err(GeometryError::InconsistentAtomicPlan);
            }
            placed.content_origin = (
                placed.bounds.x + input.padding.left,
                placed.bounds.y + offset + input.padding.top,
            );
            placed.lines = fit.lines;
            placed.tables = fit.tables;
        }
        Ok(FragmentFit::Placed(suffix))
    }

    /// A cut may cross the cell's physical span, but cannot strand part of its
    /// content. Do not confuse minimum physical height with the occupied content.
    fn cut_crosses_span(&self, end: usize) -> bool {
        (0..end).any(|r| self.plan.grid[r].iter().any(|t| r + t.row_span > end))
    }

    fn row_cut_required(&self, end: usize) -> Option<f64> {
        let prefix = self.plan.row_heights[self.row..end].iter().sum::<f64>();
        let mut required = prefix;
        let crosses = self.cut_crosses_span(end);
        for r in self.row..end {
            for track in &self.plan.grid[r] {
                let stop = (r + track.row_span).min(end);
                let height = self.plan.row_heights[r..stop].iter().sum::<f64>();
                if track.content_height > height {
                    if !crosses || stop != end {
                        return None;
                    }
                    required = required.max(prefix + (track.content_height - height));
                }
            }
        }
        Some(required)
    }

    pub(super) fn fit_row_groups(
        &self,
        area: PageArea,
        end_row: usize,
        atomic: bool,
        seal_span: bool,
    ) -> Result<FragmentFit, GeometryError> {
        let plan = &self.plan;
        let b = area.bounds;
        let atomic = atomic || plan.policy == SplitPolicy::Never;
        let height_to = |end| plan.row_heights[self.row..end].iter().sum::<f64>();
        let mut required = height_to(end_row);
        let mut accepted = None;
        for end in self.row + 1..=end_row {
            if atomic && end != end_row {
                continue;
            }
            if let Some(minimum) = self.row_cut_required(end) {
                required = required.min(minimum);
                if minimum <= b.height {
                    accepted = Some(end);
                }
            }
        }
        if plan.width > b.width || required > b.height {
            return Ok(FragmentFit::DoesNotFit {
                required_width: plan.width,
                required_height: required,
            });
        }
        // Resolve the complete cut before emitting anything. A later cell that
        // cannot fit wholly may force the cut back before its owning row.
        let end = accepted.ok_or(GeometryError::InconsistentAtomicPlan)?;
        // A cut through a physical spanning cell closes at the available frame
        // edge. The final logical row owns that extra blank band; later rows
        // retain their source minima. Normal Hancom margin variants establish
        // this separately from content consumption (not paint-side stretching).
        let reserved = if seal_span && self.cut_crosses_span(end) {
            b.height
        } else {
            self.row_cut_required(end)
                .ok_or(GeometryError::InconsistentAtomicPlan)?
        };
        let tail_band = reserved - height_to(end);
        let mut cells = Vec::new();
        for r in 0..end {
            for (slot, cell) in plan.rows[r].cells.iter().enumerate() {
                let track = &plan.grid[r][slot];
                let source_end = r + track.row_span;
                if source_end <= self.row {
                    continue;
                }
                let start = r.max(self.row);
                let stop = source_end.min(end);
                let y = b.y + plan.row_heights[self.row..start].iter().sum::<f64>();
                let height = plan.row_heights[start..stop].iter().sum::<f64>()
                    + if stop == end { tail_band } else { 0.0 };
                let x = b.x + track.left;
                let first = r >= self.row;
                let (offset, available) = if first {
                    track.alignment.content_window(height, track.content_height)
                } else {
                    (0.0, height)
                };
                let (lines, tables) = if first {
                    let fit = FlowCursor::default().fit(
                        cell,
                        Rect {
                            x: x + cell.padding.left,
                            y: y + offset,
                            width: cell.width,
                            height: available,
                        },
                    )?;
                    if fit.next.block != cell.blocks.len() {
                        return Err(GeometryError::InconsistentAtomicPlan);
                    }
                    (fit.lines, fit.tables)
                } else {
                    (Vec::new(), Vec::new())
                };
                cells.push(CellPlacement {
                    partial: start != r || stop != source_end,
                    row: r,
                    row_span: track.row_span,
                    visible_rows: start..stop,
                    column: track.column,
                    column_span: track.span,
                    bounds: Rect {
                        x,
                        y,
                        width: track.width,
                        height,
                    },
                    content_origin: (
                        x + cell.padding.left,
                        y + offset + if first { cell.padding.top } else { 0.0 },
                    ),
                    lines,
                    tables,
                });
            }
        }
        let mut next = self.clone();
        next.row = end;
        next.reset_row();
        Ok(FragmentFit::Placed(TableFragmentPlan {
            placement: TablePlacement {
                bounds: Rect {
                    width: plan.width,
                    height: reserved,
                    ..b
                },
                cells,
            },
            rows: self.row..end,
            continuation: next,
        }))
    }
}
