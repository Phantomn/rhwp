use std::ops::Range;
use std::sync::Arc;

use super::contracts::{finite, nonnegative};
use super::{
    CellPlacement, GeometryError, LinePlacement, PageArea, Rect, SplitPolicy, TableContentPlan,
    TablePlacement,
};

/// Continuation is bound to one immutable content plan, not a free row integer.
#[derive(Debug, Clone)]
pub struct TableCursor {
    pub(super) plan: Arc<TableContentPlan>,
    pub(super) next_row: usize,
}

#[derive(Debug)]
pub enum FragmentFit {
    Placed(TableFragmentPlan),
    /// The caller decides whether to retry on another page or report oversize.
    /// No clipping, no forced fit and no implicit Legacy fallback.
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

    /// Height to reserve is the *same field* used for the final table bounds.
    /// External captions/footnotes and paragraph anchoring are not yet supported.
    pub fn reserved_height(&self) -> f64 {
        self.placement.bounds.height
    }

    pub fn continuation(&self) -> TableCursor {
        self.continuation.clone()
    }
}

impl TableCursor {
    pub fn is_complete(&self) -> bool {
        self.next_row == self.plan.rows.len()
    }

    /// Pure fit query. A rejected or uncommitted query consumes neither content
    /// nor page space; advance only with the chosen fragment's continuation.
    pub fn fit(&self, area: PageArea) -> Result<FragmentFit, GeometryError> {
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
        let plan = &self.plan;
        let required_height = if plan.policy == SplitPolicy::Never {
            // Same accumulation order as the fit loop, including floating point
            // rounding: an atomic table must not accidentally become a prefix.
            plan.row_heights
                .iter()
                .enumerate()
                .fold(0.0, |height, (row, h)| {
                    height + if row == 0 { 0.0 } else { plan.row_spacing } + h
                })
        } else {
            plan.row_heights[self.next_row]
        };
        if b.width < plan.width || b.height < required_height {
            return Ok(FragmentFit::DoesNotFit {
                required_width: plan.width,
                required_height,
            });
        }
        let mut height = 0.0;
        let mut end = self.next_row;
        let mut row_offsets = Vec::new();
        for &row_height in &plan.row_heights[self.next_row..] {
            let gap = if end == self.next_row {
                0.0
            } else {
                plan.row_spacing
            };
            let offset = height + gap;
            let candidate = offset + row_height;
            if candidate > b.height {
                break;
            }
            height = candidate;
            row_offsets.push(offset);
            end += 1;
        }
        let mut cells = Vec::new();
        for (index, offset) in row_offsets.into_iter().enumerate() {
            let row = self.next_row + index;
            let y = b.y + offset;
            let mut x = b.x;
            for (column, cell) in plan.rows[row].cells.iter().enumerate() {
                let content_origin = (x + cell.padding.left, y + cell.padding.top);
                let lines = cell
                    .content
                    .lines
                    .iter()
                    .map(|line| LinePlacement {
                        owner: line.owner,
                        bounds: Rect {
                            x: content_origin.0 + line.bounds.x,
                            y: content_origin.1 + line.bounds.y,
                            width: line.bounds.width,
                            height: line.bounds.height,
                        },
                    })
                    .collect();
                cells.push(CellPlacement {
                    row,
                    column,
                    bounds: Rect {
                        x,
                        y,
                        width: plan.column_widths[column],
                        height: plan.row_heights[row],
                    },
                    content_origin,
                    lines,
                });
                x += plan.column_widths[column];
            }
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
            rows: self.next_row..end,
            continuation: TableCursor {
                plan: Arc::clone(plan),
                next_row: end,
            },
        }))
    }
}
