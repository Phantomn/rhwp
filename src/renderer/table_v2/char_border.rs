//! Character outlines remain children of their composed line. They decorate
//! its box, do not create another flow unit, and move with the same payload.
use super::GeometryError;
use crate::{
    model::{
        document::DocInfo,
        paragraph::Paragraph,
        style::{BorderLine, BorderLineType, CenterLine},
    },
    renderer::{
        render_tree::{RenderNode, RenderNodeType},
        style_resolver::ResolvedStyleSet,
    },
};

/// Join complete outlines by adjacent text ownership, not by coincident
/// coordinates. Font/underline runs can change inside one character border.
/// This runs before fit: the resulting line is the single paint payload used
/// both for stored-metric validation and for continuation placement.
pub(super) fn connect_outlines(
    parent: &mut RenderNode,
    styles: &ResolvedStyleSet,
) -> Result<(), GeometryError> {
    for child in &parent.children {
        if matches!(child.node_type, RenderNodeType::Line(_)) {
            validate_child(child, parent)?;
        }
    }
    let mut output = Vec::new();
    let mut owner: Option<[BorderLine; 4]> = None;
    let mut outline: Vec<RenderNode> = Vec::new();
    let mut children = std::mem::take(&mut parent.children).into_iter().peekable();
    while let Some(child) = children.next() {
        match &child.node_type {
            RenderNodeType::TextRun(run) => {
                let key = run
                    .border_fill_id
                    .checked_sub(1)
                    .and_then(|id| styles.border_styles.get(id as usize))
                    .map(|b| b.borders)
                    .filter(|edges| edges.iter().all(|e| e.line_type == BorderLineType::Solid));
                if owner != key {
                    output.append(&mut outline);
                }
                owner = key;
                output.push(child);
            }
            RenderNodeType::Line(_) if owner.is_some() => {
                let mut edges = vec![child];
                while children
                    .peek()
                    .is_some_and(|n| matches!(n.node_type, RenderNodeType::Line(_)))
                {
                    edges.push(children.next().unwrap());
                }
                check_rectangle(&edges)?;
                if outline.is_empty() {
                    outline = edges;
                } else {
                    let first = line(&outline[0]);
                    let next = line(&edges[0]);
                    if first.y1 != next.y1 || first.y2 != next.y2 || next.x1 < first.x1 {
                        return Err(GeometryError::Unsupported("character border run geometry"));
                    }
                    outline[1] = edges[1].clone();
                    for side in [2, 3] {
                        let right = line(&edges[side]).x2;
                        let RenderNodeType::Line(edge) = &mut outline[side].node_type else {
                            unreachable!()
                        };
                        edge.x2 = right;
                        // Consume the common butt-cap ink geometry too.
                        let bbox = edge.ink_bbox();
                        outline[side].bbox = bbox;
                    }
                }
            }
            _ => {
                output.append(&mut outline);
                owner = None;
                output.push(child);
            }
        }
    }
    output.append(&mut outline);
    parent.children = output;
    Ok(())
}

fn line(node: &RenderNode) -> &crate::renderer::render_tree::LineNode {
    let RenderNodeType::Line(line) = &node.node_type else {
        unreachable!()
    };
    line
}

fn check_rectangle(edges: &[RenderNode]) -> Result<(), GeometryError> {
    if edges.len() != 4 {
        return Err(GeometryError::Unsupported(
            "character border edge ownership",
        ));
    }
    let [left, right, top, bottom] = std::array::from_fn(|i| line(&edges[i]));
    if left.x1 != left.x2
        || right.x1 != right.x2
        || top.y1 != top.y2
        || bottom.y1 != bottom.y2
        || left.x1 != top.x1
        || left.x1 != bottom.x1
        || right.x1 != top.x2
        || right.x1 != bottom.x2
        || left.y1 != top.y1
        || right.y1 != top.y1
        || left.y2 != bottom.y1
        || right.y2 != bottom.y1
    {
        return Err(GeometryError::Unsupported(
            "character border rectangle geometry",
        ));
    }
    Ok(())
}

pub(super) fn validate_source(p: &Paragraph, info: &DocInfo) -> Result<(), GeometryError> {
    for run in &p.char_shapes {
        let cs = info
            .char_shapes
            .get(run.char_shape_id as usize)
            .ok_or(GeometryError::Unsupported("missing character style"))?;
        if cs.border_fill_id == 0 {
            continue;
        }
        let mut effects = info
            .border_fills
            .get(cs.border_fill_id as usize - 1)
            .ok_or(GeometryError::Unsupported(
                "missing character borderFill reference",
            ))?
            .clone();
        for edge in &mut effects.borders {
            edge.line_type = BorderLineType::None;
        }
        if !super::decoration::source_border_is_unpainted(&effects) {
            return Err(GeometryError::Unsupported("V2 character border effect"));
        }
    }
    Ok(())
}

pub(super) fn qualify(p: &Paragraph, styles: &ResolvedStyleSet) -> Result<(), GeometryError> {
    for run in &p.char_shapes {
        let cs = styles
            .char_styles
            .get(run.char_shape_id as usize)
            .ok_or(GeometryError::Unsupported("missing character style"))?;
        if cs.border_fill_id == 0 {
            continue;
        }
        let b = styles
            .border_styles
            .get(cs.border_fill_id as usize - 1)
            .ok_or(GeometryError::Unsupported(
                "missing character borderFill reference",
            ))?;
        if b.fill_color.is_some()
            || b.gradient.is_some()
            || b.pattern.is_some()
            || b.image_fill.is_some()
            || b.diagonal_attr != 0
            || b.center_line != CenterLine::None
            || b.borders
                .iter()
                .any(|e| !matches!(e.line_type, BorderLineType::None | BorderLineType::Solid))
        {
            return Err(GeometryError::Unsupported("V2 character border geometry"));
        }
    }
    Ok(())
}

/// Only axis-aligned outline paths produced by qualified character borders.
/// Stroke ink extends half a pen outside the path by design; this is not a
/// second line box and must not change stored baselines or line advancement.
pub(super) fn validate_child(child: &RenderNode, parent: &RenderNode) -> Result<(), GeometryError> {
    let RenderNodeType::Line(line) = &child.node_type else {
        return Err(GeometryError::Unsupported(
            "text preview non-text paint payload",
        ));
    };
    let b = &parent.bbox;
    if !child.children.is_empty()
        || [line.x1, line.x2, line.y1, line.y2, line.style.width]
            .iter()
            .any(|v| !v.is_finite())
        || line.style.width < 0.0
        || (line.x1 != line.x2 && line.y1 != line.y2)
        || [line.x1, line.x2]
            .iter()
            .any(|x| *x < b.x - 1e-7 || *x > b.x + b.width + 1e-7)
        || [line.y1, line.y2]
            .iter()
            .any(|y| *y < b.y - 1e-7 || *y > b.y + b.height + 1e-7)
    {
        return Err(GeometryError::Unsupported(
            "character border outside occupied line",
        ));
    }
    Ok(())
}
