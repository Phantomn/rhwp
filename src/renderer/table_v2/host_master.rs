//! Admission for page decorations. Selection and shape painting stay shared
//! with the document host; table layout must never escape into Legacy here.
use super::HostedTableError;
use crate::model::{
    control::Control,
    header_footer::MasterPage,
    shape::{ShapeObject, SizeCriterion, VertRelTo},
};

/// Keep the established Paper/Page origin resolver, but do not send textbox
/// content through its independent Legacy paragraph-height calculation.
pub(super) fn outlines(page: &MasterPage) -> MasterPage {
    let mut page = page.clone();
    for p in &mut page.paragraphs {
        for c in &mut p.controls {
            if let Control::Shape(shape) = c {
                if let ShapeObject::Rectangle(r) = shape.as_mut() {
                    r.drawing.text_box = None;
                }
            }
        }
    }
    page
}

/// Consume the resolved outline origin exactly once. V2's same composed
/// textbox extent determines vertical alignment and final text placement.
pub(super) fn complete(
    master: &mut crate::renderer::render_tree::RenderNode,
    source: &MasterPage,
    styles: &crate::renderer::style_resolver::ResolvedStyleSet,
    dpi: f64,
    resources: &[crate::model::bin_data::BinDataContent],
) -> Result<(), HostedTableError> {
    use crate::renderer::render_tree::RenderNodeType;
    for node in &mut master.children {
        if let RenderNodeType::Rectangle(r) = &node.node_type {
            let source_shape = r
                .para_index
                .zip(r.control_index)
                .and_then(|(p, c)| source.paragraphs.get(p as usize)?.controls.get(c as usize))
                .and_then(|c| {
                    if let Control::Shape(s) = c {
                        Some(s.as_ref())
                    } else {
                        None
                    }
                })
                .ok_or(super::GeometryError::InconsistentAtomicPlan)?;
            let mut composed =
                super::shapes::node(source_shape, node.bbox, styles, dpi, resources)?;
            composed.layer = node.layer.clone();
            *node = composed;
        }
    }
    Ok(())
}

pub(super) fn validate(pages: &[MasterPage]) -> Result<(), HostedTableError> {
    let fail = || HostedTableError::UnsupportedHost("master-page content requires host admission");
    for page in pages {
        if page.is_extension
            || page.page_front
            || page.text_direction != 0
            || page.text_width == 0
            || page.text_height == 0
        {
            return Err(fail());
        }
        for p in &page.paragraphs {
            if !p.text.is_empty() || p.controls.is_empty() || !p.field_ranges.is_empty() {
                return Err(fail());
            }
            for c in &p.controls {
                let Control::Shape(shape) = c else {
                    return Err(fail());
                };
                let drawing = match shape.as_ref() {
                    ShapeObject::Rectangle(r) => &r.drawing,
                    ShapeObject::Line(l) if l.drawing.text_box.is_none() => &l.drawing,
                    _ => return Err(fail()),
                };
                let a = shape.common();
                if a.treat_as_char
                    || !matches!(a.vert_rel_to, VertRelTo::Paper | VertRelTo::Page)
                    || a.width_criterion != SizeCriterion::Absolute
                    || a.height_criterion != SizeCriterion::Absolute
                    || drawing.caption.is_some()
                {
                    return Err(fail());
                }
                if let Some(t) = &drawing.text_box {
                    for p in &t.paragraphs {
                        if !p.controls.is_empty() {
                            // In particular, a nested table must use V2 before
                            // this story can be admitted. Never drop it.
                            return Err(fail());
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
