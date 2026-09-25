//! Solid backgrounds and intact-cell linear gradients consume final bounds;
//! they never change composition, fit, cuts, padding or cursor advancement.
use super::{GeometryError, Rect};
use crate::{
    model::{
        control::Control,
        document::DocInfo,
        style::{CenterLine, FillType},
        table::Table,
        ColorRef,
    },
    renderer::{
        render_tree::{BoundingBox, RectangleNode, RenderNode, RenderNodeType},
        style_resolver::ResolvedStyleSet,
        GradientFillInfo, ShapeStyle,
    },
};

#[derive(Default)]
pub(super) struct Background {
    pub id: u16,
    color: Option<ColorRef>,
    gradient: Option<Box<GradientFillInfo>>,
}

impl Background {
    pub fn resolve(
        id: u16,
        styles: &ResolvedStyleSet,
        intact: bool,
    ) -> Result<Self, GeometryError> {
        Self::resolve_inner(id, styles, intact)
    }

    pub fn resolve_cell(
        id: u16,
        styles: &ResolvedStyleSet,
        intact: bool,
    ) -> Result<Self, GeometryError> {
        Self::resolve_inner(id, styles, intact)
    }

    fn resolve_inner(
        id: u16,
        styles: &ResolvedStyleSet,
        intact: bool,
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
        // CellBorders qualifies both table and cell edge declarations; this
        // resolver owns backgrounds/effects, not edge precedence.
        if style.diagonal_attr != 0
            || style.center_line != CenterLine::None
            || style.pattern.is_some()
            || style.image_fill.is_some()
        {
            return Err(GeometryError::Unsupported(
                "V2 decoration supports only solid backgrounds",
            ));
        }
        // A stored pen style is not a diagonal declaration; attr/center_line
        // above select the actual lines. Gradient restarts on cut cells or
        // split table frames are not qualified by the intact-cell contract.
        if let Some(g) = &style.gradient {
            if !intact {
                return Err(GeometryError::Unsupported("V2 split gradient background"));
            }
            if g.gradient_type != 1
                || g.colors.len() < 2
                || g.colors.len() > 65534
                || g.colors.len() != g.positions.len()
                || g.colors.iter().any(|c| c >> 24 != 0)
                || g.positions
                    .iter()
                    .any(|p| !p.is_finite() || !(0.0..=1.0).contains(p))
                || g.positions.windows(2).any(|p| p[0] > p[1])
            {
                return Err(GeometryError::Unsupported("V2 invalid linear gradient"));
            }
        }
        Ok(Self {
            id,
            color: style.fill_color,
            gradient: style.gradient.clone(),
        })
    }

    pub fn append(&self, parent: &mut RenderNode, bounds: Rect) {
        if self.color.is_some() || self.gradient.is_some() {
            parent.children.push(RenderNode::new(
                0,
                RenderNodeType::Rectangle(RectangleNode::new(
                    0.0,
                    ShapeStyle {
                        fill_color: self.color,
                        stroke_color: None,
                        stroke_width: 0.0,
                        ..Default::default()
                    },
                    self.gradient.clone(),
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
                || !matches!(
                    b.fill.fill_type,
                    FillType::None | FillType::Solid | FillType::Gradient
                )
                || (b.fill.fill_type == FillType::Solid && b.fill.solid.is_none())
            {
                return Err(GeometryError::Unsupported("V2 source decoration effect"));
            }
            if b.fill.fill_type == FillType::Gradient {
                let g = b
                    .fill
                    .gradient
                    .as_ref()
                    .ok_or(GeometryError::Unsupported("V2 missing gradient payload"))?;
                if g.gradient_type != 1
                    || !(2..=64).contains(&g.colors.len())
                    || g.colors.iter().any(|c| c >> 24 != 0)
                    || g.blur < 0
                    || g.step_center > 100
                    || i32::from(g.center_x).abs() > 200
                    || i32::from(g.center_y).abs() > 200
                    || (!g.positions.is_empty()
                        && (g.positions.len() != g.colors.len()
                            || g.positions.iter().any(|p| !(0..=100).contains(p))
                            || g.positions.windows(2).any(|p| p[0] > p[1])))
                {
                    return Err(GeometryError::Unsupported("V2 invalid linear gradient"));
                }
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
