//! Cell-addressed solid area decorations. Geometry comes from accepted cells,
//! never from declared heights or a second pagination pass.
use super::{decoration::Background, GeometryError, Rect, TablePlacement};
use crate::{
    model::{style::BorderLine, table::Table},
    renderer::{render_tree::RenderNode, style_resolver::ResolvedStyleSet},
};

pub(super) struct Zone {
    row: usize,
    end_row: usize,
    column: usize,
    end_column: usize,
    background: Background,
    pub edges: Option<[BorderLine; 4]>,
    diagonal: Option<super::diagonal::Diagonal>,
}

impl Zone {
    pub fn prepare(table: &Table, styles: &ResolvedStyleSet) -> Result<Vec<Self>, GeometryError> {
        let mut result: Vec<Self> = Vec::new();
        for source in &table.zones {
            let row = usize::from(source.start_row);
            let column = usize::from(source.start_col);
            let end_cell = table
                .cells
                .iter()
                .find(|c| c.row == source.end_row && c.col == source.end_col)
                .ok_or(GeometryError::Unsupported("V2 zone end cell address"))?;
            // Addresses name cells, not the last occupied grid square. The
            // Hancom zone-lines fixture ends at a two-column merged cell.
            let end_row = usize::from(end_cell.row) + usize::from(end_cell.row_span);
            let end_column = usize::from(end_cell.col) + usize::from(end_cell.col_span);
            if row >= end_row
                || column >= end_column
                || end_row > usize::from(table.row_count)
                || end_column > usize::from(table.col_count)
                || !table
                    .cells
                    .iter()
                    .any(|c| usize::from(c.row) == row && usize::from(c.col) == column)
            {
                return Err(GeometryError::Unsupported("V2 zone cell range"));
            }
            for cell in &table.cells {
                let r = usize::from(cell.row);
                let c = usize::from(cell.col);
                let er = r + usize::from(cell.row_span);
                let ec = c + usize::from(cell.col_span);
                if r < end_row
                    && er > row
                    && c < end_column
                    && ec > column
                    && !(r >= row && er <= end_row && c >= column && ec <= end_column)
                {
                    return Err(GeometryError::Unsupported("V2 zone cuts merged cell"));
                }
            }
            let background = Background::resolve(source.border_fill_id, styles, false)?;
            let diagonal = super::diagonal::Diagonal::resolve(source.border_fill_id, styles)?;
            if diagonal.is_some() {
                // Permission to split does not mean this zone was cut. bounds()
                // rejects an actual partial cell before emitting the diagonal.
                for cell in &table.cells {
                    if usize::from(cell.row) >= row
                        && usize::from(cell.row) < end_row
                        && usize::from(cell.col) >= column
                        && usize::from(cell.col) < end_column
                        && super::diagonal::Diagonal::resolve(cell.border_fill_id, styles)?
                            .is_some()
                    {
                        return Err(GeometryError::Unsupported(
                            "V2 zone/cell diagonal precedence",
                        ));
                    }
                }
            }
            let edges = super::borders::resolve_edges(source.border_fill_id, styles)?;
            if edges.is_some_and(|edges| {
                edges.iter().any(|e| {
                    matches!(
                        e.line_type,
                        crate::model::style::BorderLineType::Double
                            | crate::model::style::BorderLineType::ThinThickDouble
                    )
                })
            }) {
                return Err(GeometryError::Unsupported("V2 double zone perimeter"));
            }
            if edges.is_some_and(|edges| {
                edges
                    .iter()
                    .any(|e| e.line_type == crate::model::style::BorderLineType::Dash)
            }) {
                // Dash zone/cell intersections have not been qualified by the
                // cell-only pen catalog. Do not widen zone admission implicitly.
                return Err(GeometryError::Unsupported("V2 dash zone perimeter"));
            }
            // Normal Hancom zones-saved confirms same-paint nested perimeters.
            // They compose without choosing a winner; conflicting effects and
            // diagonals still require an independently qualified precedence rule.
            if result.iter().any(|z| {
                row <= z.end_row
                    && end_row >= z.row
                    && column <= z.end_column
                    && end_column >= z.column
                    && !(background.same_solid_paint(&z.background)
                        && edges == z.edges
                        && diagonal.is_none()
                        && z.diagonal.is_none())
            }) {
                return Err(GeometryError::Unsupported(
                    "V2 overlapping zone decorations",
                ));
            }
            result.push(Self {
                row,
                end_row,
                column,
                end_column,
                background,
                edges,
                diagonal,
            });
        }
        Ok(result)
    }

    /// Project the source area onto this physical fragment. Repeated headers
    /// separated from their body zone require multiple areas and remain explicit.
    pub fn bounds(&self, placement: &TablePlacement) -> Result<Option<Rect>, GeometryError> {
        let cells: Vec<_> = placement
            .cells
            .iter()
            .filter(|c| {
                c.row >= self.row
                    && c.row + c.row_span <= self.end_row
                    && c.column >= self.column
                    && c.column + c.column_span <= self.end_column
            })
            .collect();
        let Some(first) = cells.first() else {
            return Ok(None);
        };
        if self.diagonal.is_some() && cells.iter().any(|cell| cell.partial) {
            return Err(GeometryError::Unsupported(
                "V2 cell-internal zone diagonal split",
            ));
        }
        let mut bounds = first.bounds;
        let mut area = 0.0;
        for cell in cells {
            let right = (bounds.x + bounds.width).max(cell.bounds.x + cell.bounds.width);
            let bottom = (bounds.y + bounds.height).max(cell.bounds.y + cell.bounds.height);
            bounds.x = bounds.x.min(cell.bounds.x);
            bounds.y = bounds.y.min(cell.bounds.y);
            bounds.width = right - bounds.x;
            bounds.height = bottom - bounds.y;
            area += cell.bounds.width * cell.bounds.height;
        }
        if (area - bounds.width * bounds.height).abs() > 1e-7 * area.max(1.0) {
            return Err(GeometryError::Unsupported("V2 disconnected zone fragment"));
        }
        Ok(Some(bounds))
    }

    pub fn append_background(
        &self,
        placement: &TablePlacement,
        node: &mut RenderNode,
    ) -> Result<(), GeometryError> {
        if let Some(bounds) = self.bounds(placement)? {
            self.background.append(node, bounds);
        }
        Ok(())
    }

    pub fn append_diagonal(
        &self,
        placement: &TablePlacement,
        node: &mut RenderNode,
        dpi: f64,
    ) -> Result<(), GeometryError> {
        if let (Some(diagonal), Some(bounds)) = (&self.diagonal, self.bounds(placement)?) {
            diagonal.append(node, bounds, dpi);
        }
        Ok(())
    }
}
