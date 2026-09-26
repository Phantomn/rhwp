//! Solid backgrounds and intact-cell linear gradients consume final bounds;
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
    /// Opaque solid/no-fill effects commute when their actual colors agree.
    /// IDs are references, not paint semantics. Gradients are not idempotent
    /// across different bounds and remain outside this overlapping-zone rule.
    pub fn same_solid_paint(&self, other: &Self) -> bool {
        self.gradient.is_none() && other.gradient.is_none() && self.color == other.color
    }

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
        super::diagonal::Diagonal::from_style(style)?;
        if style.pattern.is_some() || style.image_fill.is_some() {
            return Err(GeometryError::Unsupported(
                "V2 decoration supports only solid backgrounds",
            ));
        }
        // Qualified diagonals are bound and painted separately, above fills.
        // Gradient restarts on cut cells or
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
        for id in std::iter::once(table.border_fill_id)
            .chain(table.cells.iter().map(|c| c.border_fill_id))
            .chain(table.zones.iter().map(|z| z.border_fill_id))
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
                || !super::diagonal::supported_attr(b.attr)
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
        for paragraph in table.cells.iter().flat_map(|c| &c.paragraphs) {
            validate_paragraph_source(paragraph.para_shape_id, info)?;
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

/// A reference is not a decoration by itself. Qualify source-only effects
/// before style resolution can discard them; never erase the original ID.
pub(super) fn validate_paragraph_source(id: u16, info: &DocInfo) -> Result<(), GeometryError> {
    let para = info
        .para_shapes
        .get(usize::from(id))
        .ok_or(GeometryError::Unsupported("missing paragraph style"))?;
    if para.border_fill_id == 0 {
        return Ok(());
    }
    let b = info
        .border_fills
        .get(usize::from(para.border_fill_id) - 1)
        .ok_or(GeometryError::Unsupported(
            "missing paragraph borderFill reference",
        ))?;
    if !source_border_is_unpainted(b) {
        return Err(GeometryError::Unsupported("V2 paragraph decoration"));
    }
    Ok(())
}

/// A page/paragraph border-fill reference may name an explicitly unpainted
/// style. Inspect its effects without deleting or rewriting the source record.
pub(super) fn source_border_is_unpainted(b: &crate::model::style::BorderFill) -> bool {
    let no_fill = match b.fill.fill_type {
        FillType::None => true,
        FillType::Solid => b
            .fill
            .solid
            .as_ref()
            .is_some_and(|s| s.pattern_type <= 0 && s.background_color >> 24 != 0),
        _ => false,
    };
    !b.three_d
        && b.attr == 0
        && b.center_line == CenterLine::None
        && b.borders
            .iter()
            .all(|p| p.line_type == BorderLineType::None)
        && no_fill
}

/// Resolved-only callers also reject every visible decoration. Pen widths or
/// colors on a None edge, and spacing around a nonexistent border, do not paint.
pub(super) fn paragraph_is_unpainted(id: u16, styles: &ResolvedStyleSet) -> bool {
    id == 0
        || styles
            .border_styles
            .get(usize::from(id) - 1)
            .is_some_and(|b| {
                b.borders
                    .iter()
                    .all(|p| p.line_type == BorderLineType::None)
                    && b.diagonal_attr == 0
                    && b.center_line == CenterLine::None
                    && b.fill_color.is_none()
                    && b.pattern.is_none()
                    && b.gradient.is_none()
                    && b.image_fill.is_none()
            })
}
