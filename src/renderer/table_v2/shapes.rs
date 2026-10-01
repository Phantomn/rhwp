//! Cell-local rectangles reuse the common outline painter and the V2 text
//! composer. A floating foreground decoration does not become inline occupancy.
use super::{text::TextComposer, CellParagraphComposer, GeometryError, ParagraphItem};
use crate::{
    model::{
        bin_data::BinDataContent,
        control::Control,
        paragraph::Paragraph,
        shape::{HorzAlign, HorzRelTo, ShapeObject, SizeCriterion, TextWrap, VertAlign, VertRelTo},
        style::FillType,
    },
    renderer::{
        render_tree::{BoundingBox, GroupNode, PageLayoutContext, RenderNode, RenderNodeType},
        style_resolver::ResolvedStyleSet,
    },
};
use std::cell::RefCell;

fn unsupported() -> GeometryError {
    GeometryError::Unsupported("V2 cell rectangle/textbox geometry")
}

pub(super) fn mixed_inline_candidate(para: &Paragraph) -> bool {
    para.text.chars().any(|c| !c.is_whitespace())
        && para.controls.iter().any(|c| matches!(c, Control::Shape(_)))
        && para.controls.iter().all(|c| {
            matches!(
                c,
                Control::Shape(_) | Control::ColumnDef(_) | Control::SectionDef(_)
            )
        })
}

pub(super) fn validate_inline(para: &Paragraph) -> Result<(), GeometryError> {
    if para.line_segs.is_empty()
        || para.stored_text_partition_is_dirty()
        || !super::tac::qualified_structural_axis(para)
        || !mixed_inline_candidate(para)
    {
        return Err(unsupported());
    }
    for c in &para.controls {
        if let Control::Shape(shape) = c {
            let c = shape.common();
            if !c.treat_as_char
                || c.horizontal_offset != 0
                || c.vertical_offset != 0
                || c.prevent_page_break != 0
                || [c.margin.left, c.margin.right, c.margin.top, c.margin.bottom]
                    .iter()
                    .any(|v| *v < 0)
            {
                return Err(unsupported());
            }
        }
    }
    Ok(())
}

/// Reuse the shared paragraph painter's horizontal text/object walk, including
/// stored row membership, indentation and justification. The V2 shape painter
/// consumes that x and the saved baseline; no separate text-width estimate.
pub(super) fn attach_inline(
    para: &Paragraph,
    context: &PageLayoutContext,
    lines: &mut [RenderNode],
    items: &mut [ParagraphItem],
    styles: &ResolvedStyleSet,
    dpi: f64,
    resources: &[BinDataContent],
) -> Result<(), GeometryError> {
    let scale = dpi / 7200.;
    let positions = para.control_utf16_positions();
    let text_positions = para.control_text_positions();
    for (ci, ctrl) in para.controls.iter().enumerate() {
        let Control::Shape(shape) = ctrl else {
            continue;
        };
        let position = *positions.get(ci).ok_or_else(unsupported)?;
        let li = (0..para.line_segs.len())
            .rfind(|i| para.line_seg_text_start(*i) <= position)
            .ok_or_else(unsupported)?;
        let line = lines.get_mut(li).ok_or_else(unsupported)?;
        let (x, _) = context
            .get_inline_shape_position(0, 0, ci, None)
            .ok_or_else(unsupported)?;
        let c = shape.common();
        let baseline = f64::from(para.line_segs[li].baseline_distance) * scale;
        let height = f64::from(c.height) * scale;
        // Stored reference is integer HU, while 85% of an object may not be.
        // Use its integer-HU ascent just as the saved line baseline does.
        let ascent = (super::tac_metrics::ascent(f64::from(c.height))).floor() * scale;
        let bounds = BoundingBox::new(
            x,
            line.bbox.y + baseline - ascent,
            f64::from(c.width) * scale,
            height,
        );
        if bounds.x - f64::from(c.margin.left) * scale < line.bbox.x - 1e-7
            || bounds.x + bounds.width + f64::from(c.margin.right) * scale
                > line.bbox.x + line.bbox.width + 1e-7
            || bounds.y - f64::from(c.margin.top) * scale < line.bbox.y - 1e-7
            || bounds.y + bounds.height + f64::from(c.margin.bottom) * scale
                > line.bbox.y + line.bbox.height + 1e-7
        {
            return Err(GeometryError::Unsupported(
                "inline shape outside saved line envelope",
            ));
        }
        let child = node(shape, bounds, styles, dpi, resources)?;
        let text_pos = *text_positions.get(ci).ok_or_else(unsupported)?;
        let at=line.children.iter().position(|n|matches!(&n.node_type,RenderNodeType::TextRun(r) if r.char_start.is_some_and(|s|s>=text_pos))).unwrap_or(line.children.len());
        line.children.insert(at, child);
        let item = items
            .iter_mut()
            .find(|item| match item {
                ParagraphItem::Lines { lines, .. } => lines.iter().any(|(owner, _)| *owner == li),
                ParagraphItem::ObjectRow { line, .. } => *line == li,
                _ => false,
            })
            .ok_or_else(unsupported)?;
        match item {
            ParagraphItem::Lines {
                height,
                advance,
                lines,
            } if lines.len() == 1 && height == advance => {
                *item = ParagraphItem::ObjectRow {
                    line: li,
                    bounds: lines[0].1,
                    controls: vec![ci],
                };
            }
            ParagraphItem::ObjectRow { controls, .. } => controls.push(ci),
            _ => {
                return Err(GeometryError::Unsupported(
                    "mixed inline shape grouped or overlapping row",
                ))
            }
        }
    }
    Ok(())
}

pub(super) fn node(
    shape: &ShapeObject,
    bounds: BoundingBox,
    styles: &ResolvedStyleSet,
    dpi: f64,
    resources: &[BinDataContent],
) -> Result<RenderNode, GeometryError> {
    node_with_page_number(shape, bounds, styles, dpi, resources, None)
}

pub(super) fn node_with_page_number(
    shape: &ShapeObject,
    bounds: BoundingBox,
    styles: &ResolvedStyleSet,
    dpi: f64,
    resources: &[BinDataContent],
    page_number: Option<u32>,
) -> Result<RenderNode, GeometryError> {
    if let ShapeObject::Line(line) = shape {
        return line_node(line, bounds, dpi);
    }
    let ShapeObject::Rectangle(rect) = shape else {
        return Err(unsupported());
    };
    let a = &rect.drawing.shape_attr;
    let c = &rect.common;
    if c.width == 0
        || c.height == 0
        || c.width_criterion != SizeCriterion::Absolute
        || c.height_criterion != SizeCriterion::Absolute
        || c.drop_cap_style != crate::model::shape::DropCapStyle::None
        || a.group_level != 0
        || a.horz_flip
        || a.vert_flip
        || a.rotation_angle != 0
        || a.render_b != 0.0
        || a.render_c != 0.0
        || !a.render_sx.is_finite()
        || !a.render_sy.is_finite()
        || a.render_sx <= 0.0
        || a.render_sy <= 0.0
        || rect.round_rate > 100
        || rect.drawing.caption.is_some()
        || rect.drawing.shadow_type != 0
        || rect.drawing.border_line.attr & 0x3f > 1
        || (rect.drawing.border_line.attr & 0x3f == 1 && rect.drawing.border_line.width <= 0)
        || !matches!(
            rect.drawing.fill.fill_type,
            FillType::None | FillType::Solid
        )
        || rect.drawing.fill.gradient.is_some()
        || rect.drawing.fill.image.is_some()
        || rect.x_coords != [0, a.original_width as i32, a.original_width as i32, 0]
        || rect.y_coords != [0, 0, a.original_height as i32, a.original_height as i32]
    {
        return Err(unsupported());
    }

    // Geometry was resolved by the owning line/anchor. Ungrouped component
    // offsets are not another paragraph translation. Reapplying source scaling
    // to current common.width/height would scale the rectangle twice.
    let (mut style, gradient) = crate::renderer::layout::drawing_to_shape_style(&rect.drawing);
    // The source pen is in HU, and the accepted owner already resolved bounds.
    style.stroke_width = if rect.drawing.border_line.attr & 0x3f == 1 {
        f64::from(rect.drawing.border_line.width) * dpi / 7200.0
    } else {
        0.0
    };
    let mut node = RenderNode::new(
        0,
        RenderNodeType::Rectangle(crate::renderer::render_tree::RectangleNode::new(
            f64::from(rect.round_rate) / 100.0 * bounds.width.min(bounds.height),
            style,
            gradient,
        )),
        bounds,
    );
    node.set_rectangle_control_kind(rect.control_kind());
    if let Some(t) = &rect.drawing.text_box {
        // This bounded path does not delegate textbox measurement to Legacy.
        // The very same composed payload supplies physical extent and paint.
        if t.vertical_all || t.paragraphs.len() != 1 || t.list_attr & !0x60 != 0 {
            return Err(unsupported());
        }
        let scale = dpi / 7200.0;
        let left = f64::from(t.margin_left) * scale;
        let right = f64::from(t.margin_right) * scale;
        let top = f64::from(t.margin_top) * scale;
        let bottom = f64::from(t.margin_bottom) * scale;
        let width = bounds.width - left - right;
        let height = bounds.height - top - bottom;
        if [left, right, top, bottom, width, height]
            .iter()
            .any(|v| !v.is_finite() || *v < 0.0)
            || width == 0.0
        {
            return Err(unsupported());
        }
        let p = &t.paragraphs[0];
        let composer = TextComposer {
            styles,
            dpi,
            payloads: RefCell::new(Vec::new()),
        };
        let page_field = super::cell_page_field::qualify_textbox(p)?;
        if page_field {
            composer.compose_page_field(p, width, None)?;
        } else {
            composer.compose(p, width)?;
        }
        let mut lines = composer
            .payloads
            .into_inner()
            .pop()
            .ok_or_else(unsupported)?;
        if page_field {
            let [reserved] = lines.as_slice() else {
                return Err(GeometryError::InconsistentAtomicPlan);
            };
            lines = vec![
                super::cell_page_field::PageField::new(p, styles, width, dpi)
                    .render(page_number, reserved)?,
            ];
        }
        let first = lines.first().ok_or_else(unsupported)?.bbox.y;
        let end = lines
            .iter()
            .map(|n| n.bbox.y + n.bbox.height)
            .fold(first, f64::max);
        let content_height = end - first;
        if content_height > height {
            return Err(unsupported());
        }
        let offset = match t.vertical_align {
            crate::model::table::VerticalAlign::Center => (height - content_height) / 2.0,
            crate::model::table::VerticalAlign::Bottom => height - content_height,
            _ => p
                .line_segs
                .first()
                .map_or(first, |r| f64::from(r.vertical_pos) * scale),
        };
        if offset < 0.0 || offset + content_height > height {
            return Err(unsupported());
        }
        for line in &mut lines {
            super::text::translate(line, bounds.x + left, bounds.y + top + offset - first);
        }
        node.children.extend(lines);
    }
    Ok(node)
}

/// The already-composed object box is authoritative. Line endpoint coordinates
/// are in the original component frame; map them once to the current box.
fn line_node(
    line: &crate::model::shape::LineShape,
    bounds: BoundingBox,
    dpi: f64,
) -> Result<RenderNode, GeometryError> {
    let a = &line.drawing.shape_attr;
    let c = &line.common;
    if c.width == 0
        || c.height == 0
        || c.width_criterion != SizeCriterion::Absolute
        || c.height_criterion != SizeCriterion::Absolute
        || c.drop_cap_style != crate::model::shape::DropCapStyle::None
        || a.original_width == 0
        || a.original_height == 0
        || a.group_level != 0
        || a.horz_flip
        || a.vert_flip
        || a.rotation_angle != 0
        || a.render_b != 0.0
        || a.render_c != 0.0
        || !a.render_sx.is_finite()
        || !a.render_sy.is_finite()
        || a.render_sx <= 0.0
        || a.render_sy <= 0.0
        || line.connector.is_some()
        || line.drawing.text_box.is_some()
        || line.drawing.caption.is_some()
        || line.drawing.shadow_type != 0
        || line.drawing.border_line.attr & 0x3f != 1
        || line.drawing.border_line.attr & 0x3fff_fc00 != 0
        || line.drawing.border_line.width <= 0
        || [line.start, line.end].iter().any(|p| {
            p.x < 0 || p.y < 0 || p.x as u32 > a.original_width || p.y as u32 > a.original_height
        })
    {
        return Err(unsupported());
    }
    let mut style = crate::renderer::layout::drawing_to_line_style(&line.drawing);
    style.width = f64::from(line.drawing.border_line.width) * dpi / 7200.0;
    let x = |v| bounds.x + f64::from(v) / f64::from(a.original_width) * bounds.width;
    let y = |v| bounds.y + f64::from(v) / f64::from(a.original_height) * bounds.height;
    Ok(RenderNode::new(
        0,
        RenderNodeType::Line(crate::renderer::render_tree::LineNode::new(
            x(line.start.x),
            y(line.start.y),
            x(line.end.x),
            y(line.end.y),
            style,
        )),
        bounds,
    ))
}

pub(super) fn floating_candidate(para: &Paragraph) -> bool {
    matches!(para.controls.as_slice(), [Control::Shape(s)] if !s.common().treat_as_char)
}

pub(super) fn compose_floating(
    para: &Paragraph,
    width: f64,
    styles: &ResolvedStyleSet,
    dpi: f64,
    resources: &[BinDataContent],
) -> Result<(Vec<ParagraphItem>, Vec<RenderNode>), GeometryError> {
    let [Control::Shape(shape)] = para.controls.as_slice() else {
        return Err(unsupported());
    };
    let c = shape.common();
    let style = super::tac::carrier_style(para, styles)?;
    if !para.text.is_empty()
        || para.char_count != 9
        || !para.char_offsets.is_empty()
        || para.line_segs.len() != 1
        || para.line_segs[0].text_start != 0
        || c.text_wrap != TextWrap::InFrontOfText
        || c.vert_rel_to != VertRelTo::Para
        || c.vert_align != VertAlign::Top
        || !matches!(c.horz_rel_to, HorzRelTo::Column | HorzRelTo::Para)
        || c.horz_align != HorzAlign::Left
        || c.prevent_page_break != 0
        || c.affect_line_spacing
        || [style.margin_left, style.margin_right, style.indent]
            .iter()
            .any(|v| *v != 0.0)
        || [c.margin.left, c.margin.right, c.margin.top, c.margin.bottom]
            .iter()
            .any(|v| *v != 0)
    {
        return Err(unsupported());
    }
    let mut host = para.clone();
    host.controls.clear();
    host.ctrl_data_records.clear();
    let composer = TextComposer {
        styles,
        dpi,
        payloads: RefCell::new(Vec::new()),
    };
    let mut items = composer.compose(&host, width)?;
    let mut nodes = composer
        .payloads
        .into_inner()
        .pop()
        .ok_or_else(unsupported)?;
    if nodes.len() != 1 {
        return Err(unsupported());
    }
    let host = nodes.pop().ok_or_else(unsupported)?;
    let b = host.bbox;
    let scale = dpi / 7200.0;
    let shape_bounds = BoundingBox::new(
        f64::from(c.horizontal_offset as i32) * scale,
        // Para/Top is the paragraph origin, before spacing_before. The host
        // line's b.y includes that spacing and must remain an occupied line,
        // but it is NOT a second translation of the foreground object's anchor.
        f64::from(c.vertical_offset as i32) * scale,
        f64::from(c.width) * scale,
        f64::from(c.height) * scale,
    );
    let mut group = RenderNode::new(
        0,
        RenderNodeType::Group(GroupNode {
            section_index: None,
            para_index: None,
            control_index: None,
        }),
        b,
    );
    group.children.push(host);
    group
        .children
        .push(node(shape, shape_bounds, styles, dpi, resources)?);
    for item in &mut items {
        if let ParagraphItem::Lines {
            height,
            advance,
            lines,
        } = item
        {
            if height != advance || lines.len() != 1 {
                return Err(unsupported());
            }
            *item = ParagraphItem::ObjectRow {
                line: 0,
                bounds: lines[0].1,
                controls: vec![0],
            };
        }
    }
    Ok((items, vec![group]))
}
