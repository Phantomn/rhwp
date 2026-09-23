use std::collections::HashSet;
use std::sync::Arc;

use super::contracts::nonnegative;
use super::{GeometryError, RowInput, SplitPolicy, TableCursor};

/// Immutable, width-bound content and physical row geometry. No pointer-key cache.
/// Recomposition creates a new plan; an old cursor can only resume its own plan.
#[derive(Debug)]
pub struct TableContentPlan {
    pub(super) column_widths: Vec<f64>,
    pub(super) rows: Vec<RowInput>,
    pub(super) row_heights: Vec<f64>,
    pub(super) row_spacing: f64,
    pub(super) width: f64,
    pub(super) policy: SplitPolicy,
}

impl TableContentPlan {
    pub fn new(
        column_widths: Vec<f64>,
        rows: Vec<RowInput>,
        row_spacing: f64,
        policy: SplitPolicy,
    ) -> Result<Self, GeometryError> {
        if policy == SplitPolicy::WithinCells {
            return Err(GeometryError::UnsupportedCellSplit);
        }
        if column_widths.is_empty() || rows.is_empty() {
            return Err(GeometryError::EmptyTable);
        }
        nonnegative(row_spacing, "row spacing")?;
        for &width in &column_widths {
            nonnegative(width, "column width")?;
            if width == 0.0 {
                return Err(GeometryError::InvalidNumber("zero column width"));
            }
        }
        let width = column_widths.iter().sum();
        nonnegative(width, "table width")?;
        let mut row_heights = Vec::with_capacity(rows.len());
        for (row, input) in rows.iter().enumerate() {
            if input.cells.len() != column_widths.len() {
                return Err(GeometryError::CellCount { row });
            }
            let mut height: f64 = 0.0;
            for (column, cell) in input.cells.iter().enumerate() {
                let p = cell.padding;
                for value in [p.left, p.right, p.top, p.bottom] {
                    nonnegative(value, "padding")?;
                }
                nonnegative(cell.minimum_height, "minimum cell height")?;
                nonnegative(cell.content.width, "content width")?;
                nonnegative(cell.content.height, "content height")?;
                // Do not reuse lines composed for another width or silently squeeze them.
                let inner_width = column_widths[column] - p.left - p.right;
                if inner_width < 0.0 || inner_width != cell.content.width {
                    return Err(GeometryError::ContentWidth { row, column });
                }
                let mut owners = HashSet::new();
                for line in &cell.content.lines {
                    let b = line.bounds;
                    for value in [b.x, b.y, b.width, b.height] {
                        nonnegative(value, "line bounds")?;
                    }
                    let right = b.x + b.width;
                    let bottom = b.y + b.height;
                    if !right.is_finite()
                        || !bottom.is_finite()
                        || right > inner_width
                        || bottom > cell.content.height
                    {
                        return Err(GeometryError::ContentBounds { row, column });
                    }
                    if !owners.insert(line.owner) {
                        return Err(GeometryError::DuplicateLineOwner { row, column });
                    }
                }
                // Overlapping line boxes are allowed: they do not imply page breaks.
                let physical = p.top + cell.content.height + p.bottom;
                nonnegative(physical, "physical cell height")?;
                height = height.max(physical).max(cell.minimum_height);
            }
            row_heights.push(height);
        }
        let total = row_heights.iter().sum::<f64>() + row_spacing * (rows.len() - 1) as f64;
        nonnegative(total, "physical table height")?;
        Ok(Self {
            column_widths,
            rows,
            row_heights,
            row_spacing,
            width,
            policy,
        })
    }

    /// Explicit opt-in to V2 geometry. Does not change any document engine setting.
    pub fn start(self) -> TableCursor {
        TableCursor {
            plan: Arc::new(self),
            next_row: 0,
        }
    }
}
