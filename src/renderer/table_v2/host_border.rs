//! Body paragraph outlines consume accepted host packets, never Legacy height
//! guesses. Empty lines own an outline too. Connected neighbours omit the
//! shared horizontal edge; a page/column cut does not close a continuing group.
use super::{GeometryError, HostedParagraphFragment};
use crate::{
    model::{
        document::DocInfo,
        paragraph::Paragraph,
        style::{BorderLineType, CenterLine},
    },
    renderer::{
        layout::border_width_to_px,
        render_tree::{LineNode, PageLayoutContext, RenderNode, RenderNodeType},
        style_resolver::{ResolvedParaStyle, ResolvedStyleSet},
        LineStyle,
    },
};

pub(super) fn validate_source(id: u16, info: &DocInfo) -> Result<(), GeometryError> {
    let p = info
        .para_shapes
        .get(id as usize)
        .ok_or(GeometryError::Unsupported("missing paragraph style"))?;
    if p.border_fill_id == 0 {
        return Ok(());
    }
    let b =
        info.border_fills
            .get(p.border_fill_id as usize - 1)
            .ok_or(GeometryError::Unsupported(
                "missing paragraph borderFill reference",
            ))?;
    // Reuse the source-effect check without discarding the original edges.
    let mut effects = b.clone();
    for edge in &mut effects.borders {
        edge.line_type = BorderLineType::None;
    }
    if !super::decoration::source_border_is_unpainted(&effects) {
        return Err(GeometryError::Unsupported(
            "V2 host paragraph border effect",
        ));
    }
    Ok(())
}

pub(super) fn qualify(
    p: &ResolvedParaStyle,
    styles: &ResolvedStyleSet,
) -> Result<(), GeometryError> {
    if super::decoration::paragraph_is_unpainted(p.border_fill_id, styles) {
        return Ok(());
    }
    let b = styles
        .border_styles
        .get(p.border_fill_id as usize - 1)
        .ok_or(GeometryError::Unsupported(
            "missing paragraph borderFill reference",
        ))?;
    // Insets/fills need a separate occupancy/background contract. This slice
    // supports zero-inset solid/absent edges, including asymmetric pens.
    if p.border_spacing != [0.; 4]
        || b.fill_color.is_some()
        || b.gradient.is_some()
        || b.pattern.is_some()
        || b.image_fill.is_some()
        || b.diagonal_attr != 0
        || b.center_line != CenterLine::None
        || b.borders
            .iter()
            .any(|e| !matches!(e.line_type, BorderLineType::None | BorderLineType::Solid))
    {
        return Err(GeometryError::Unsupported(
            "V2 host paragraph border geometry",
        ));
    }
    Ok(())
}

fn connects(a: &ResolvedParaStyle, b: &ResolvedParaStyle, styles: &ResolvedStyleSet) -> bool {
    if !a.border_connect || a.border_fill_id == 0 || b.border_fill_id == 0 {
        return false;
    }
    match (
        styles.border_styles.get(a.border_fill_id as usize - 1),
        styles.border_styles.get(b.border_fill_id as usize - 1),
    ) {
        (Some(a), Some(b)) => a.borders == b.borders,
        _ => false,
    }
}

pub(crate) fn append(
    fragment: &HostedParagraphFragment,
    index: usize,
    paragraphs: &[Paragraph],
    styles: &ResolvedStyleSet,
    dpi: f64,
    context: &mut PageLayoutContext,
    parent: &mut RenderNode,
) {
    let style_at = |i: usize| {
        paragraphs
            .get(i)
            .and_then(|p| styles.para_styles.get(p.para_shape_id as usize))
    };
    let Some(style) = style_at(index) else {
        return;
    };
    if style.border_fill_id == 0 {
        return;
    }
    let Some(border) = styles.border_styles.get(style.border_fill_id as usize - 1) else {
        return;
    };
    // Plain body and mixed inline-shape packets share this outline owner.
    // Other packet kinds keep their existing admission checks. The accepted
    // fragment owns all geometry, including stored inline object line extents.
    let before = index
        .checked_sub(1)
        .and_then(style_at)
        .is_some_and(|p| connects(p, style, styles));
    let after = style_at(index + 1).is_some_and(|p| connects(style, p, styles));
    let b = fragment.occupied();
    let bottom = (b.y + b.height).max(fragment.next_y());
    let top_edge = fragment.is_first() && !before;
    let bottom_edge = fragment.is_last() && !after;
    for (edge, enabled, x1, y1, x2, y2) in [
        (0, true, b.x, b.y, b.x, bottom),
        (1, true, b.x + b.width, b.y, b.x + b.width, bottom),
        (2, top_edge, b.x, b.y, b.x + b.width, b.y),
        (3, bottom_edge, b.x, bottom, b.x + b.width, bottom),
    ] {
        let pen = border.borders[edge];
        if !enabled || pen.line_type == BorderLineType::None {
            continue;
        }
        let line = LineNode::new(
            x1,
            y1,
            x2,
            y2,
            LineStyle {
                color: pen.color,
                width: border_width_to_px(pen.width) * dpi / 96.,
                ..Default::default()
            },
        );
        let bounds = line.ink_bbox();
        parent.children.push(RenderNode::new(
            context.next_id(),
            RenderNodeType::Line(line),
            bounds,
        ));
    }
}
