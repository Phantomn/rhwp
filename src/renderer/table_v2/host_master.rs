//! Admission for page decorations. Selection and shape painting stay shared
//! with the document host; table layout must never escape into Legacy here.
use super::HostedTableError;
use crate::model::{
    control::Control,
    header_footer::MasterPage,
    shape::{ShapeObject, SizeCriterion, VertRelTo},
};

/// Page decorations use the common, stateless anchor resolver and V2 shape/text
/// paint. No LayoutEngine or table-layout dispatcher participates here.
pub(super) fn render(
    source: &MasterPage,
    layout: &crate::renderer::page_layout::PageLayoutInfo,
    styles: &crate::renderer::style_resolver::ResolvedStyleSet,
    resources: &[crate::model::bin_data::BinDataContent],
    page_number: u32,
    section_index: usize,
) -> Result<crate::renderer::render_tree::RenderNode, HostedTableError> {
    use crate::model::{
        shape::{HorzAlign, HorzRelTo, TextWrap},
        style::Alignment,
    };
    use crate::renderer::{
        float_placement::ObjectPlacementFrame,
        page_layout::LayoutRect,
        render_tree::{BoundingBox, RenderLayerInfo, RenderNode, RenderNodeType},
    };
    let paper = LayoutRect {
        x: 0.,
        y: 0.,
        width: layout.page_width,
        height: layout.page_height,
    };
    let column = LayoutRect {
        x: layout.body_area.x,
        width: layout.body_area.width,
        ..paper
    };
    let mut master = RenderNode::new(
        0,
        RenderNodeType::MasterPage,
        BoundingBox::new(0., 0., paper.width, paper.height),
    );
    master.layer = Some(RenderLayerInfo::new(None, 0, 0).for_master_page());
    for (pi, paragraph) in source.paragraphs.iter().enumerate() {
        let style = styles.para_styles.get(paragraph.para_shape_id as usize);
        let left = style.map_or(0., |s| s.margin_left);
        let right = style.map_or(0., |s| s.margin_right);
        let container = LayoutRect {
            x: column.x + left,
            width: column.width - left - right,
            ..column
        };
        for (ci, control) in paragraph.controls.iter().enumerate() {
            let Control::Shape(shape) = control else {
                return Err(HostedTableError::UnsupportedHost(
                    "master-page control not prepared",
                ));
            };
            let common = shape.common();
            let width = f64::from(common.width) * layout.dpi / 7200.;
            let height = f64::from(common.height) * layout.dpi / 7200.;
            let (mut x, y) = ObjectPlacementFrame {
                container: &container,
                column: &column,
                body: &layout.body_area,
                paper: &paper,
                paragraph_y: 0.,
                alignment: Alignment::Left,
                dpi: layout.dpi,
            }
            .position(common, width, height);
            if common.text_wrap == TextWrap::Square
                && matches!(common.horz_align, HorzAlign::Left | HorzAlign::Inside)
            {
                x += f64::from(common.margin.left) * layout.dpi / 7200.;
            }
            let bounds = BoundingBox::new(x, y, width, height);
            let mut node = if let ShapeObject::Line(line) = shape.as_ref() {
                let attr = &line.drawing.shape_attr;
                let transform = crate::renderer::render_tree::ShapeTransform {
                    rotation: f64::from(attr.rotation_angle),
                    horz_flip: attr.horz_flip,
                    vert_flip: attr.vert_flip,
                };
                let current_width = f64::from(attr.current_width) * layout.dpi / 7200.;
                let current_height = f64::from(attr.current_height) * layout.dpi / 7200.;
                let bounds =
                    if transform.has_transform() && current_width > 0. && current_height > 0. {
                        BoundingBox::new(
                            x + (width - current_width) / 2.,
                            y + (height - current_height) / 2.,
                            current_width,
                            current_height,
                        )
                    } else {
                        bounds
                    };
                crate::renderer::shape_paint::line(line, bounds, layout.dpi, transform)
            } else {
                super::shapes::node_with_page_number(
                    shape,
                    bounds,
                    styles,
                    layout.dpi,
                    resources,
                    Some(page_number),
                )?
            };
            match &mut node.node_type {
                RenderNodeType::Line(line) => {
                    line.section_index = Some(section_index);
                    line.para_index = Some(pi);
                    line.control_index = Some(ci);
                }
                RenderNodeType::Path(path) => {
                    path.section_index = Some(section_index);
                    path.para_index = Some(pi);
                    path.control_index = Some(ci);
                }
                _ => {}
            }
            let stable =
                ((pi.min(u16::MAX as usize) as u32) << 16) | ci.min(u16::MAX as usize) as u32;
            let mut layer = RenderLayerInfo::new(Some(common.text_wrap), common.z_order, stable)
                .for_master_page();
            // Preserve the common paper-background paint classification. This
            // does not change the source anchor or the body's occupied height.
            if common.text_wrap == TextWrap::InFrontOfText
                && common.horz_rel_to == HorzRelTo::Paper
                && common.vert_rel_to == VertRelTo::Paper
                && x.abs() <= 1.
                && y.abs() <= 1.
                && width >= paper.width * 0.95
                && height >= paper.height * 0.95
            {
                layer.text_wrap = Some(TextWrap::BehindText);
            }
            node.layer = Some(layer);
            master.children.push(node);
        }
    }
    Ok(master)
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
                        if !p.controls.is_empty() && !super::cell_page_field::qualify_textbox(p)? {
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
