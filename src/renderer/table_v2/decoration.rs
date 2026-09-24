//! Solid table/cell backgrounds only. Decorations consume final fragment bounds;
//! they never change composition, fit, cuts, padding or cursor advancement.
use super::{GeometryError, Rect};
use crate::{
    model::{
        control::Control,
        document::DocInfo,
        style::{BorderLineType, CenterLine, FillType},
        table::Table,
        ColorRef,
    },
    renderer::{
        render_tree::{BoundingBox, RectangleNode, RenderNode, RenderNodeType},
        style_resolver::ResolvedStyleSet,
        ShapeStyle,
    },
};

#[derive(Default)]
pub(super) struct Background {
    pub id: u16,
    color: Option<ColorRef>,
}

impl Background {
    pub fn resolve(id: u16, styles: &ResolvedStyleSet) -> Result<Self, GeometryError> {
        Self::resolve_inner(id, styles, false)
    }

    pub fn resolve_cell(id: u16, styles: &ResolvedStyleSet) -> Result<Self, GeometryError> {
        // CellBorders qualifies and owns the edge styles separately.
        Self::resolve_inner(id, styles, true)
    }

    fn resolve_inner(
        id: u16,
        styles: &ResolvedStyleSet,
        cell: bool,
    ) -> Result<Self, GeometryError> {
        if id == 0 {
            return Ok(Self::default());
        }
        let style =
            styles
                .border_styles
                .get(usize::from(id) - 1)
                .ok_or(GeometryError::Unsupported(
                    "missing table borderFill reference",
                ))?;
        if style.break_cell_separate_line {
            return Err(GeometryError::Unsupported("V2 separate split-cell border"));
        }
        if (!cell
            && style
                .borders
                .iter()
                .any(|b| b.line_type != BorderLineType::None))
            || style.diagonal_attr != 0
            || style.diagonal.diagonal_type != 0
            || style.center_line != CenterLine::None
            || style.pattern.is_some()
            || style.gradient.is_some()
            || style.image_fill.is_some()
        {
            return Err(GeometryError::Unsupported(
                "V2 decoration supports only solid backgrounds",
            ));
        }
        Ok(Self {
            id,
            color: style.fill_color,
        })
    }

    pub fn append(&self, parent: &mut RenderNode, bounds: Rect) {
        if let Some(color) = self.color {
            parent.children.push(RenderNode::new(
                0,
                RenderNodeType::Rectangle(RectangleNode::new(
                    0.0,
                    ShapeStyle {
                        fill_color: Some(color),
                        stroke_color: None,
                        stroke_width: 0.0,
                        ..Default::default()
                    },
                    None,
                )),
                BoundingBox::new(bounds.x, bounds.y, bounds.width, bounds.height),
            ));
        }
    }
}

/// Reject source effects lost by general style resolution. The lower-level
/// PreparedTextTable API takes already-resolved styles, not source DocInfo.
pub(super) fn validate_source(table: &Table, info: &DocInfo) -> Result<(), GeometryError> {
    fn visit(table: &Table, info: &DocInfo, depth: usize) -> Result<(), GeometryError> {
        if depth >= 64 {
            return Err(GeometryError::Unsupported("table nesting resource limit"));
        }
        if !table.zones.is_empty() {
            return Err(GeometryError::Unsupported("V2 table background zones"));
        }
        for id in std::iter::once(table.border_fill_id)
            .chain(table.cells.iter().map(|c| c.border_fill_id))
        {
            if id == 0 {
                continue;
            }
            let b =
                info.border_fills
                    .get(usize::from(id) - 1)
                    .ok_or(GeometryError::Unsupported(
                        "missing table borderFill reference",
                    ))?;
            if b.break_cell_separate_line {
                return Err(GeometryError::Unsupported("V2 separate split-cell border"));
            }
            if b.three_d
                || b.attr != 0
                || !matches!(b.fill.alpha, 0 | 255)
                || !matches!(b.fill.fill_type, FillType::None | FillType::Solid)
                || (b.fill.fill_type == FillType::Solid && b.fill.solid.is_none())
            {
                return Err(GeometryError::Unsupported("V2 source decoration effect"));
            }
        }
        for child in table
            .cells
            .iter()
            .flat_map(|c| &c.paragraphs)
            .flat_map(|p| &p.controls)
        {
            if let Control::Table(child) = child {
                visit(child, info, depth + 1)?;
            }
        }
        Ok(())
    }
    visit(table, info, 0)
}
