//! Qualified saved cell cuts and their physical, per-fragment alignment.
//! Qualification and placement query the same cell flow; neither invents cuts.
use super::{
    content::VerticalAlignment, flow::FlowCursor, CellPlacement, FlowBlock, GeometryError, Rect,
    TableContentPlan,
};

#[derive(Debug)]
pub(super) struct StoredRowFrames {
    pub row: usize,
    pub frames: Vec<StoredRowFrame>,
}

#[derive(Debug)]
pub(super) struct StoredRowFrame {
    pub height: f64,
    cells: Vec<StoredCellFrame>,
}

#[derive(Debug)]
struct StoredCellFrame {
    start: FlowCursor,
    end_block: usize,
    offset: f64,
}

fn fail() -> GeometryError {
    GeometryError::Unsupported("unqualified stored child cell frames")
}

impl StoredRowFrames {
    pub fn qualify(
        plan: &TableContentPlan,
        row: usize,
        first: f64,
        last: f64,
    ) -> Result<Self, GeometryError> {
        if !first.is_finite()
            || !last.is_finite()
            || first <= 0.0
            || last <= 0.0
            || first > plan.row_heights[row]
        {
            return Err(fail());
        }
        let input = &plan.rows[row];
        let mut cursors = vec![FlowCursor::default(); input.cells.len()];
        let mut frames = Vec::new();
        for (index, height) in [first, last].into_iter().enumerate() {
            let mut cells = Vec::new();
            let mut has_cut = false;
            let mut occupied: f64 = 0.0;
            for (slot, cell) in input.cells.iter().enumerate() {
                let start = cursors[slot].clone();
                let fit = start.fit_cell_until(
                    cell,
                    Rect {
                        x: 0.0,
                        y: 0.0,
                        width: cell.width,
                        height,
                    },
                    cell.blocks.len(),
                    None,
                    true,
                )?;
                let done = fit.next.block == cell.blocks.len();
                let at_cut = matches!(
                    cell.blocks.get(fit.next.block),
                    Some(FlowBlock::StoredFrameStart)
                );
                if fit.required != 0.0
                    || fit.height > height
                    || (index == 0 && !done && !at_cut)
                    || (index == 1 && !done)
                {
                    return Err(fail());
                }
                has_cut |= at_cut;
                occupied = occupied.max(fit.height);
                let slack = height - fit.height;
                let offset = if start.block == cell.blocks.len() {
                    0.0
                } else {
                    match plan.grid[row][slot].alignment {
                        VerticalAlignment::Top => 0.0,
                        VerticalAlignment::Center => slack / 2.0,
                        VerticalAlignment::Bottom => slack,
                    }
                };
                cells.push(StoredCellFrame {
                    start,
                    end_block: fit.next.block,
                    offset,
                });
                cursors[slot] = fit.next;
            }
            // The source cut, not a tight budget, owns the first end. The last
            // frame must independently explain the follower's saved origin.
            let same =
                (occupied - height).abs() <= 32.0 * f64::EPSILON * height.abs().max(occupied.abs());
            if (index == 0 && !has_cut) || (index == 1 && !same) {
                return Err(fail());
            }
            frames.push(StoredRowFrame { height, cells });
        }
        Ok(Self { row, frames })
    }
}

impl StoredRowFrame {
    pub fn place(
        &self,
        plan: &TableContentPlan,
        row: usize,
        x: f64,
        y: f64,
    ) -> Result<(Vec<CellPlacement>, Vec<FlowCursor>), GeometryError> {
        let mut placements = Vec::new();
        let mut cursors = Vec::new();
        for (slot, geometry) in self.cells.iter().enumerate() {
            let cell = &plan.rows[row].cells[slot];
            let track = &plan.grid[row][slot];
            let left = x + track.left;
            let fit = geometry.start.fit_cell_until(
                cell,
                Rect {
                    x: left + cell.padding.left,
                    y: y + geometry.offset,
                    width: cell.width,
                    height: self.height - geometry.offset,
                },
                cell.blocks.len(),
                None,
                true,
            )?;
            if fit.required != 0.0 || fit.next.block != geometry.end_block {
                return Err(GeometryError::InconsistentAtomicPlan);
            }
            placements.push(CellPlacement {
                partial: true,
                row,
                row_span: 1,
                visible_rows: row..row + 1,
                column: track.column,
                column_span: track.span,
                bounds: Rect {
                    x: left,
                    y,
                    width: track.width,
                    height: self.height,
                },
                content_origin: (
                    left + cell.padding.left,
                    y + geometry.offset + cell.padding.top,
                ),
                lines: fit.lines,
                tables: fit.tables,
            });
            cursors.push(fit.next);
        }
        Ok((placements, cursors))
    }
}
