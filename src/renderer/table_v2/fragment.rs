use std::ops::Range;
use std::sync::Arc;

use super::contracts::{finite, nonnegative};
use super::flow::FlowCursor;
use super::{
    CellPlacement, GeometryError, PageArea, Rect, SplitPolicy, TableContentPlan, TablePlacement,
};

/// A continuation is inseparable from its immutable content plan and child cuts.
#[derive(Debug, Clone)]
pub struct TableCursor {
    plan: Arc<TableContentPlan>,
    row: usize,
    cells: Vec<FlowCursor>,
    minimum_left: f64,
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
        };
        cursor.reset_row();
        cursor
    }

    fn reset_row(&mut self) {
        if let Some(row) = self.plan.rows.get(self.row) {
            self.cells = row.cells.iter().map(|_| FlowCursor::default()).collect();
            self.minimum_left = row
                .cells
                .iter()
                .map(|cell| cell.minimum_height)
                .fold(0.0, f64::max);
        } else {
            self.cells.clear();
            self.minimum_left = 0.0;
        }
    }

    pub fn is_complete(&self) -> bool {
        self.row == self.plan.rows.len()
    }

    /// Pure query. Only a returned continuation advances accepted content.
    pub fn fit(&self, area: PageArea) -> Result<FragmentFit, GeometryError> {
        if self.plan.header_rows == 0
            || self.plan.policy == SplitPolicy::Never
            || self.is_complete()
        {
            return self.fit_rows(area, self.plan.rows.len(), false);
        }
        self.fit_with_header(area)
    }

    /// The header and body are one transaction. A fitting header alone cannot
    /// publish a fragment or advance the body cursor. Both use the SAME row fit
    /// and final placements, including recursive child output and physical bands.
    fn fit_with_header(&self, area: PageArea) -> Result<FragmentFit, GeometryError> {
        let prefix = Self::new(Arc::clone(&self.plan));
        let FragmentFit::Placed(header) = prefix.fit_rows(area, self.plan.header_rows, true)?
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
        match body.fit_rows(body_area, self.plan.rows.len(), false)? {
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
        let mut next = self.clone();
        let plan = &self.plan;
        let required = if atomic {
            self.header_height()
        } else {
            match plan.policy {
                SplitPolicy::Never => plan.height,
                SplitPolicy::BetweenRows => plan.row_heights[self.row],
                SplitPolicy::WithinCells => 0.0,
            }
        };
        if b.x + plan.width > b.x + b.width || b.y + required > b.y + b.height {
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
            let gap = if progressed { plan.row_spacing } else { 0.0 };
            let offset = height + gap;
            if b.y + offset > b.y + b.height {
                break;
            }
            let available = (b.height - offset).max(0.0);
            if (atomic || plan.policy != SplitPolicy::WithinCells)
                && b.y + offset + plan.row_heights[next.row] > b.y + b.height
            {
                break;
            }
            let row = &plan.rows[next.row];
            let mut fit_cells = Vec::new();
            let mut used: f64 = 0.0;
            let mut changed = false;
            let mut all_done = true;
            let mut x = b.x;
            for (column, cell) in row.cells.iter().enumerate() {
                let fit = next.cells[column].fit(
                    cell,
                    Rect {
                        x: x + cell.padding.left,
                        y: b.y + offset,
                        width: cell.width,
                        height: available,
                    },
                )?;
                used = used.max(fit.height);
                changed |= fit.progressed;
                blocked = blocked.max(fit.required);
                all_done &= fit.next.block == cell.blocks.len();
                fit_cells.push((x, fit));
                x += plan.column_widths[column];
            }
            // Minimum height is a remaining physical band, not already consumed
            // text. Do not manufacture blank progress in front of a blocked unit.
            if all_done {
                used = used.max(next.minimum_left.min(available));
            }
            if (atomic || plan.policy != SplitPolicy::WithinCells)
                && (!all_done || b.y + offset + used < b.y + offset + next.minimum_left)
            {
                return Err(GeometryError::InconsistentAtomicPlan);
            }
            changed |= used > 0.0;
            if !changed && !(all_done && next.minimum_left == 0.0) {
                break;
            }
            for (column, (x, fit)) in fit_cells.into_iter().enumerate() {
                let cell = &row.cells[column];
                let origin_y = if next.cells[column].block == 0 {
                    b.y + offset + cell.padding.top
                } else {
                    b.y + offset
                };
                cells.push(CellPlacement {
                    row: next.row,
                    column,
                    bounds: Rect {
                        x,
                        y: b.y + offset,
                        width: plan.column_widths[column],
                        height: used,
                    },
                    content_origin: (x + cell.padding.left, origin_y),
                    lines: fit.lines,
                    tables: fit.tables,
                });
                next.cells[column] = fit.next;
            }
            height = offset + used;
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
