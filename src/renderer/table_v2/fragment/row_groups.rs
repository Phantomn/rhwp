//! Row-break tables may cut only boundaries not crossed by a spanning cell.
//! An accepted group owns complete cells; content cursors are never copied into
//! covered logical slots. Cell-internal rowspan cuts are rejected at binding.
use super::*;

impl TableCursor {
    fn group_end(&self, start: usize) -> usize {
        let mut end = start + 1;
        let mut row = start;
        while row < end {
            for track in &self.plan.grid[row] {
                end = end.max(row + track.row_span);
            }
            row += 1;
        }
        end
    }

    pub(super) fn fit_row_groups(
        &self,
        area: PageArea,
        end_row: usize,
        atomic: bool,
    ) -> Result<FragmentFit, GeometryError> {
        let plan = &self.plan;
        let b = area.bounds;
        let first_end = self.group_end(self.row);
        let required_end = if atomic || plan.policy == SplitPolicy::Never {
            end_row
        } else {
            first_end
        };
        let required = plan.row_heights[self.row..required_end].iter().sum::<f64>();
        if b.x + plan.width > b.x + b.width || b.y + required > b.y + b.height {
            return Ok(FragmentFit::DoesNotFit {
                required_width: plan.width,
                required_height: required,
            });
        }
        let mut cells = Vec::new();
        let mut height = 0.0;
        let mut row = self.row;
        while row < end_row {
            let group_end = self.group_end(row);
            if group_end > end_row {
                return Err(GeometryError::Unsupported(
                    "header boundary crosses rowspan",
                ));
            }
            let group_height = plan.row_heights[row..group_end].iter().sum::<f64>();
            if b.y + height + group_height > b.y + b.height {
                break;
            }
            let mut y = b.y + height;
            for r in row..group_end {
                for (slot, cell) in plan.rows[r].cells.iter().enumerate() {
                    let track = &plan.grid[r][slot];
                    let cell_height = plan.row_heights[r..r + track.row_span].iter().sum::<f64>();
                    let x = b.x + track.left;
                    let fit = FlowCursor::default().fit(
                        cell,
                        Rect {
                            x: x + cell.padding.left,
                            y: y + track.content_offset_y,
                            width: cell.width,
                            height: cell_height - track.content_offset_y,
                        },
                    )?;
                    if fit.next.block != cell.blocks.len() {
                        return Err(GeometryError::InconsistentAtomicPlan);
                    }
                    cells.push(CellPlacement {
                        row: r,
                        row_span: track.row_span,
                        column: track.column,
                        column_span: track.span,
                        bounds: Rect {
                            x,
                            y,
                            width: track.width,
                            height: cell_height,
                        },
                        content_origin: (
                            x + cell.padding.left,
                            y + track.content_offset_y + cell.padding.top,
                        ),
                        lines: fit.lines,
                        tables: fit.tables,
                    });
                }
                y += plan.row_heights[r];
            }
            height += group_height;
            row = group_end;
        }
        let mut next = self.clone();
        next.row = row;
        next.reset_row();
        Ok(FragmentFit::Placed(TableFragmentPlan {
            placement: TablePlacement {
                bounds: Rect {
                    width: plan.width,
                    height,
                    ..b
                },
                cells,
            },
            rows: self.row..row,
            continuation: next,
        }))
    }
}
