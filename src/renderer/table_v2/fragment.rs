use std::ops::Range;
use std::sync::Arc;

use super::contracts::{finite, nonnegative};
use super::flow::FlowCursor;
use super::{
    CellPlacement, GeometryError, PageArea, Rect, SplitPolicy, TableContentPlan, TablePlacement,
};

mod row_groups;

/// A continuation is inseparable from its immutable content plan and child cuts.
#[derive(Debug, Clone)]
pub struct TableCursor {
    plan: Arc<TableContentPlan>,
    row: usize,
    cells: Vec<FlowCursor>,
    minimum_left: f64,
    stored_fragment: usize,
}

#[derive(Debug)]
pub enum FragmentFit {
    Placed(TableFragmentPlan),
    /// Explicit non-fit. The caller decides about another page or an oversize error.
    DoesNotFit {
        required_width: f64,
        required_height: f64,
    },
    Complete,
}

#[derive(Debug)]
pub struct TableFragmentPlan {
    placement: TablePlacement,
    rows: Range<usize>,
    continuation: TableCursor,
}

impl TableFragmentPlan {
    pub fn placement(&self) -> &TablePlacement {
        &self.placement
    }
    pub fn rows(&self) -> Range<usize> {
        self.rows.clone()
    }
    pub fn reserved_height(&self) -> f64 {
        self.placement.bounds.height
    }
    pub fn continuation(&self) -> TableCursor {
        self.continuation.clone()
    }
}

impl TableCursor {
    pub(super) fn new(plan: Arc<TableContentPlan>) -> Self {
        let mut cursor = Self {
            plan,
            row: 0,
            cells: Vec::new(),
            minimum_left: 0.0,
            stored_fragment: 0,
        };
        cursor.reset_row();
        cursor
    }

    fn reset_row(&mut self) {
        self.stored_fragment = 0;
        if let Some(row) = self.plan.rows.get(self.row) {
            self.cells = row.cells.iter().map(|_| FlowCursor::default()).collect();
            self.minimum_left = row
                .cells
                .iter()
                .map(|cell| cell.minimum_height)
                .fold(0.0, f64::max);
            if let Some((_, height)) = self
                .plan
                .stored_row_bands
                .iter()
                .find(|(r, _)| *r == self.row)
            {
                self.minimum_left = *height;
            }
        } else {
            self.cells.clear();
            self.minimum_left = 0.0;
        }
    }

    pub fn is_complete(&self) -> bool {
        self.row == self.plan.rows.len()
    }

    pub(super) fn starts_stored_frame(&self) -> bool {
        (self.plan.stored_row_starts.contains(&self.row)
            && self.cells.iter().all(|cursor| cursor.block == 0))
            || self.plan.rows.get(self.row).is_some_and(|row| {
                self.cells
                    .iter()
                    .zip(&row.cells)
                    .any(|(cursor, cell)| cursor.starts_stored_frame(cell))
            })
    }

    /// Pure query. Only a returned continuation advances accepted content.
    pub fn fit(&self, area: PageArea) -> Result<FragmentFit, GeometryError> {
        self.fit_with_page_height(area, None)
    }

    /// Query with a caller-owned fresh-page capacity, not the current remainder.
    /// A row-break row taller than that capacity must use its composed cell
    /// units. A saved cell-frame cut can also resume a row in a partial page;
    /// without that evidence, rows fitting a fresh page remain indivisible. Never-split and
    /// rowspan content retain their strict contracts. Row boundaries may cross
    /// a span once that cell's complete content is accepted.
    pub fn fit_in_page(
        &self,
        area: PageArea,
        page_height: f64,
    ) -> Result<FragmentFit, GeometryError> {
        nonnegative(page_height, "fresh page height")?;
        self.fit_with_page_height(area, Some(page_height))
    }

    pub(super) fn fit_with_page_height(
        &self,
        area: PageArea,
        page_height: Option<f64>,
    ) -> Result<FragmentFit, GeometryError> {
        self.fit_in_frame(area, page_height, false)
    }

    pub(super) fn fit_in_frame(
        &self,
        area: PageArea,
        page_height: Option<f64>,
        stored_frame: bool,
    ) -> Result<FragmentFit, GeometryError> {
        if self.plan.header_rows == 0
            || self.plan.policy == SplitPolicy::Never
            || self.is_complete()
        {
            return self.fit_rows(area, self.plan.rows.len(), false, page_height, stored_frame);
        }
        self.fit_with_header(area, page_height, stored_frame)
    }

    /// The header and body are one transaction. A fitting header alone cannot
    /// publish a fragment or advance the body cursor. Both use the SAME row fit
    /// and final placements, including recursive child output and physical bands.
    fn fit_with_header(
        &self,
        area: PageArea,
        page_height: Option<f64>,
        stored_frame: bool,
    ) -> Result<FragmentFit, GeometryError> {
        let prefix = Self::new(Arc::clone(&self.plan));
        let FragmentFit::Placed(header) =
            prefix.fit_rows(area, self.plan.header_rows, true, None, false)?
        else {
            return Ok(FragmentFit::DoesNotFit {
                required_width: self.plan.width,
                required_height: self.header_height(),
            });
        };
        let body = if self.row == 0 {
            header.continuation()
        } else {
            self.clone()
        };
        if body.is_complete() {
            return Ok(FragmentFit::Placed(header));
        }
        let overhead = header.reserved_height() + self.plan.row_spacing;
        let b = area.bounds;
        // Query at zero remaining height too: a non-fit must report the first
        // body unit's requirement, not merely the prefix height that already fit.
        let body_area = PageArea {
            bounds: Rect {
                y: b.y + overhead,
                height: (b.height - overhead).max(0.0),
                ..b
            },
        };
        // The title reduces this fragment's available area, not the definition
        // of a row taller than a fresh page. Keep that caller-owned capacity
        // unchanged, as for other prefixes. Subtracting the title twice (here
        // and in body_area) would classify an ordinary row's trailing line gap
        // as an oversized row and admit a title-only continuation.
        match body.fit_rows(
            body_area,
            self.plan.rows.len(),
            false,
            page_height,
            stored_frame,
        )? {
            FragmentFit::Placed(mut fragment) if overhead <= b.height => {
                let height = overhead + fragment.reserved_height();
                let mut cells = header.placement.cells;
                cells.append(&mut fragment.placement.cells);
                fragment.placement = TablePlacement {
                    bounds: Rect { height, ..b },
                    cells,
                };
                fragment.placement.bounds.width = self.plan.width;
                fragment.rows.start = self.row;
                Ok(FragmentFit::Placed(fragment))
            }
            FragmentFit::DoesNotFit {
                required_height, ..
            } => Ok(FragmentFit::DoesNotFit {
                required_width: self.plan.width,
                required_height: overhead + required_height,
            }),
            FragmentFit::Placed(_) => Ok(FragmentFit::DoesNotFit {
                required_width: self.plan.width,
                required_height: overhead,
            }),
            FragmentFit::Complete => Err(GeometryError::InconsistentAtomicPlan),
        }
    }

    fn header_height(&self) -> f64 {
        self.plan.row_heights[..self.plan.header_rows]
            .iter()
            .sum::<f64>()
            + self.plan.row_spacing * self.plan.header_rows.saturating_sub(1) as f64
    }

    fn fit_rows(
        &self,
        area: PageArea,
        end_row: usize,
        atomic: bool,
        page_height: Option<f64>,
        stored_frame: bool,
    ) -> Result<FragmentFit, GeometryError> {
        let b = area.bounds;
        finite(b.x, "page x")?;
        finite(b.y, "page y")?;
        nonnegative(b.width, "page width")?;
        nonnegative(b.height, "page height")?;
        finite(b.x + b.width, "page right")?;
        finite(b.y + b.height, "page bottom")?;
        if self.is_complete() {
            return Ok(FragmentFit::Complete);
        }
        if self
            .plan
            .grid
            .iter()
            .flatten()
            .any(|track| track.row_span > 1)
        {
            return self.fit_mixed_rows(area, end_row, atomic, page_height, stored_frame);
        }
        self.fit_plain_rows(area, end_row, atomic, page_height, stored_frame)
    }

    fn fit_plain_rows(
        &self,
        area: PageArea,
        end_row: usize,
        atomic: bool,
        page_height: Option<f64>,
        stored_frame: bool,
    ) -> Result<FragmentFit, GeometryError> {
        let b = area.bounds;
        let mut next = self.clone();
        let plan = &self.plan;
        let saved_row = |row: usize| {
            plan.rows[row].cells.iter().any(|cell| {
                cell.blocks
                    .iter()
                    .any(super::FlowBlock::has_stored_frame_cut)
            })
        };
        let splittable = |row: usize, intact_fits: bool| {
            !atomic
                && ((plan.policy == SplitPolicy::WithinCells
                    && (!intact_fits
                        || saved_row(row)
                        || (row == self.row && self.cells.iter().any(|cell| cell.block != 0))))
                    || (plan.policy == SplitPolicy::BetweenRows
                        && plan.grid[row].iter().enumerate().all(|(column, track)| {
                            // A CENTER/BOTTOM cell whose content fills the
                            // row has no alignment band to redistribute across
                            // pages. Use the resolved geometry, not the enum,
                            // to distinguish it from a genuinely shifted cell.
                            track.content_offset_y == 0.0
                                || (saved_row(row)
                                    && !plan.rows[row].cells[column]
                                        .blocks
                                        .iter()
                                        .any(super::FlowBlock::has_stored_frame_cut))
                        })
                        && (page_height.is_some_and(|height| plan.row_heights[row] > height)
                            || (saved_row(row)
                                && (stored_frame
                                    || !intact_fits
                                    || (row == self.row
                                        && self.cells.iter().any(|cell| cell.block != 0)))))))
        };
        let required = if atomic {
            self.header_height()
        } else {
            match plan.policy {
                SplitPolicy::Never => plan.height,
                SplitPolicy::BetweenRows
                    if !splittable(self.row, plan.row_heights[self.row] <= b.height) =>
                {
                    plan.row_heights[self.row]
                }
                SplitPolicy::BetweenRows => 0.0,
                SplitPolicy::WithinCells => 0.0,
            }
        };
        if plan.width > b.width || required > b.height {
            return Ok(FragmentFit::DoesNotFit {
                required_width: plan.width,
                required_height: required,
            });
        }
        let mut cells = Vec::new();
        let mut height = 0.0;
        let mut end = self.row;
        let mut progressed = false;
        let mut blocked: f64 = 0.0;
        while next.row < end_row {
            if stored_frame && progressed && plan.stored_row_starts.contains(&next.row) {
                break;
            }
            let gap = if progressed { plan.row_spacing } else { 0.0 };
            let offset = height + gap;
            if offset > b.height {
                break;
            }
            let available = (b.height - offset).max(0.0);
            if let Some(frames) = plan.stored_cell_frames.iter().find(|f| f.row == next.row) {
                if !stored_frame || atomic {
                    return Err(GeometryError::Unsupported(
                        "stored child frames need frame-aware fit",
                    ));
                }
                let frame = &frames.frames[next.stored_fragment];
                if offset + frame.height > b.height {
                    blocked = blocked.max(frame.height);
                    break;
                }
                let (mut placed, cursors) = frame.place(plan, next.row, b.x, b.y + offset)?;
                cells.append(&mut placed);
                next.cells = cursors;
                height = offset + frame.height;
                end = next.row + 1;
                progressed = true;
                next.stored_fragment += 1;
                next.minimum_left = frames.frames[next.stored_fragment..]
                    .iter()
                    .map(|f| f.height)
                    .sum();
                if next.stored_fragment < frames.frames.len() {
                    break;
                }
                next.row += 1;
                next.reset_row();
                continue;
            }
            // Use the same prefix + row sum as intact measurement, not a
            // rounded subtraction which can invent a tiny continuation tail.
            let split_row = splittable(next.row, offset + plan.row_heights[next.row] <= b.height);
            // Do not reject an entire table for CENTER/BOTTOM. Below, a cut
            // may align a fully accepted companion; an aligned cell whose own
            // content would be partial defers without consuming any prefix.
            if !split_row && offset + plan.row_heights[next.row] > b.height {
                break;
            }
            let row = &plan.rows[next.row];
            let mut fit_cells = Vec::new();
            let mut used: f64 = 0.0;
            let mut changed = false;
            let mut all_done = true;
            let mut alignment_blocked = false;
            for (column, cell) in row.cells.iter().enumerate() {
                let track = &plan.grid[next.row][column];
                let x = b.x + track.left;
                // A complete, non-cut companion cell (e.g. a centered row
                // label) is aligned in the accepted physical fragment below.
                // The cell carrying the saved cut must have no alignment band.
                let content_offset = if split_row {
                    0.0
                } else {
                    track.content_offset_y
                };
                let fit = next.cells[column].fit_cell_until(
                    cell,
                    Rect {
                        x: x + cell.padding.left,
                        y: b.y + offset + content_offset,
                        width: cell.width,
                        height: available - content_offset,
                    },
                    cell.blocks.len(),
                    page_height,
                    split_row,
                )?;
                if split_row && track.content_offset_y != 0.0 && fit.next.block != cell.blocks.len()
                {
                    blocked = blocked.max(track.content_height);
                    alignment_blocked = true;
                    break;
                }
                used = used.max(content_offset + fit.height);
                // from_flow_rows inserts the top inset as block0. Accepting
                // only that inset before a blocked first unit is not content
                // progress. If another cell progresses its padding can still
                // be committed with that row fragment; otherwise retry intact.
                let padding_only = next.cells[column].block == 0
                    && fit.next.block == 1
                    && fit.lines.is_empty()
                    && fit.tables.is_empty()
                    && fit.required > 0.0;
                changed |= fit.progressed && !padding_only;
                blocked = blocked.max(if padding_only {
                    content_offset + fit.height + fit.required
                } else {
                    fit.required
                });
                all_done &= fit.next.block == cell.blocks.len();
                fit_cells.push((x, fit));
            }
            if alignment_blocked || (!changed && !all_done) {
                break;
            }
            // A content cut does not consume the row's remaining physical band.
            // Reserve that band with EVERY accepted fragment, not only after
            // the final content unit. Otherwise early saved cuts lose space
            // which is incorrectly appended to later/empty pages. A blocked
            // first unit must still not manufacture physical-only progress;
            // all_done permits a genuine tail after the content is exhausted.
            if changed || all_done {
                let minimum = if offset + next.minimum_left <= b.height {
                    next.minimum_left
                } else {
                    available
                };
                used = used.max(minimum);
            }
            if !split_row && (!all_done || b.y + offset + used < b.y + offset + next.minimum_left) {
                return Err(GeometryError::InconsistentAtomicPlan);
            }
            changed |= used > 0.0;
            if !changed && !(all_done && next.minimum_left == 0.0) {
                break;
            }
            for (column, (x, mut fit)) in fit_cells.into_iter().enumerate() {
                let cell = &row.cells[column];
                let track = &plan.grid[next.row][column];
                let content_offset = if split_row {
                    if track.content_offset_y != 0.0 && next.cells[column].block == 0 {
                        let slack = used - track.content_height;
                        nonnegative(slack, "aligned fragment content slack")?;
                        let alignment_offset = match track.alignment {
                            super::content::VerticalAlignment::Top => 0.0,
                            super::content::VerticalAlignment::Center => slack / 2.0,
                            super::content::VerticalAlignment::Bottom => slack,
                        };
                        // Fit and paint consume this same placement. Re-query
                        // the intact companion at its final fragment origin;
                        // do not move only painted glyphs or reuse a full-row
                        // offset on the shorter first fragment.
                        fit = next.cells[column].fit_cell_until(
                            cell,
                            Rect {
                                x: x + cell.padding.left,
                                y: b.y + offset + alignment_offset,
                                width: cell.width,
                                height: used - alignment_offset,
                            },
                            cell.blocks.len(),
                            page_height,
                            false,
                        )?;
                        if fit.next.block != cell.blocks.len() {
                            return Err(GeometryError::InconsistentAtomicPlan);
                        }
                        alignment_offset
                    } else {
                        0.0
                    }
                } else {
                    track.content_offset_y
                };
                let origin_y = if next.cells[column].block == 0
                    || (split_row && next.cells[column].starts_stored_frame(cell))
                {
                    b.y + offset + content_offset + cell.padding.top
                } else {
                    b.y + offset
                };
                cells.push(CellPlacement {
                    partial: next.cells[column].block != 0 || !all_done || used < next.minimum_left,
                    row: next.row,
                    row_span: 1,
                    visible_rows: next.row..next.row + 1,
                    column: plan.grid[next.row][column].column,
                    column_span: plan.grid[next.row][column].span,
                    bounds: Rect {
                        x,
                        y: b.y + offset,
                        width: plan.grid[next.row][column].width,
                        height: used,
                    },
                    content_origin: (x + cell.padding.left, origin_y),
                    lines: fit.lines,
                    tables: fit.tables,
                });
                next.cells[column] = fit.next;
            }
            height = if used == available {
                b.height
            } else {
                offset + used
            };
            next.minimum_left = if b.y + offset + used >= b.y + offset + next.minimum_left {
                0.0
            } else {
                next.minimum_left - used
            };
            end = next.row + 1;
            progressed = true;
            if !all_done || next.minimum_left > 0.0 {
                break;
            }
            next.row += 1;
            next.reset_row();
        }
        if !progressed {
            return Ok(FragmentFit::DoesNotFit {
                required_width: plan.width,
                required_height: blocked,
            });
        }
        Ok(FragmentFit::Placed(TableFragmentPlan {
            placement: TablePlacement {
                bounds: Rect {
                    x: b.x,
                    y: b.y,
                    width: plan.width,
                    height,
                },
                cells,
            },
            rows: self.row..end,
            continuation: next,
        }))
    }
}
