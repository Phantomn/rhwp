//! A normally saved zero-width exclusion host is not a deleted blank paragraph.
//! Its line and positioned child share the paragraph origin; a subsequent empty
//! paragraph remains an ordinary line with its own height and spacing.
use super::{GeometryError, ParagraphItem, Rect};
use crate::{
    model::{
        control::Control,
        paragraph::{LineSeg, Paragraph},
        shape::{HorzAlign, HorzRelTo, TextWrap, VertAlign, VertRelTo},
        style::HeadType,
    },
    renderer::{
        hwpunit_to_px,
        render_tree::{BoundingBox, RenderNode, RenderNodeType, TextLineNode},
        style_resolver::ResolvedStyleSet,
    },
};

pub(super) fn candidate(p: &Paragraph) -> bool {
    p.text.is_empty()
        && p.controls.len() == 1
        && p.line_segs.len() == 1
        && p.line_segs[0].segment_width == 0
        && matches!(&p.controls[0], Control::Table(t) if !t.common.treat_as_char)
}

pub(super) fn compose(
    p: &Paragraph,
    width: f64,
    styles: &ResolvedStyleSet,
    dpi: f64,
) -> Result<(ParagraphItem, RenderNode), GeometryError> {
    let fail = || GeometryError::Unsupported("stored excluded cell anchor");
    let Control::Table(t) = &p.controls[0] else {
        return Err(fail());
    };
    let a = &t.common;
    let row = &p.line_segs[0];
    let style = styles
        .para_styles
        .get(p.para_shape_id as usize)
        .ok_or_else(fail)?;
    if !candidate(p)
        || p.stored_text_partition_is_dirty()
        || !p.char_offsets.is_empty()
        || p.source_line_seg_vertical_pos.is_some()
        || p.layout_only_fill_lines != 0
        || !p.field_ranges.is_empty()
        || !p.range_tags.is_empty()
        || !p.title_marks.is_empty()
        || !p.markpen_marks.is_empty()
        || row.text_start != 0
        || row.column_start != 0
        || row.line_height <= 0
        || row.text_height != row.line_height
        || row.baseline_distance < 0
        || row.baseline_distance > row.line_height
        || row.line_spacing < 0
        || row.tag != LineSeg::TAG_SINGLE_SEGMENT_LINE
        || a.text_wrap != TextWrap::TopAndBottom
        || a.vert_rel_to != VertRelTo::Para
        || a.vert_align != VertAlign::Top
        || a.horz_rel_to != HorzRelTo::Para
        || a.horz_align != HorzAlign::Left
        || a.vertical_offset != 0
        || (a.horizontal_offset as i32) < 0
        || !a.flow_with_text
        || a.allow_overlap
        || a.prevent_page_break != 0
        || style.head_type != HeadType::None
        || style.keep_lines
        || style.keep_with_next
        || style.widow_orphan
        || style.page_break_before
        || style.margin_left != 0.0
        || style.margin_right != 0.0
        || style.spacing_before != 0.0
        || style.spacing_after != 0.0
        || !super::decoration::paragraph_is_unpainted(style.border_fill_id, styles)
    {
        return Err(fail());
    }
    for (value, mirror) in [
        (a.margin.left, t.outer_margin_left),
        (a.margin.right, t.outer_margin_right),
        (a.margin.top, t.outer_margin_top),
        (a.margin.bottom, t.outer_margin_bottom),
    ] {
        if value < 0 || value != mirror {
            return Err(fail());
        }
    }
    let x = hwpunit_to_px(a.horizontal_offset as i32, dpi)
        + hwpunit_to_px(i32::from(a.margin.left), dpi);
    // Width is checked again against the actual resolved child plan by binding.
    if x > width {
        return Err(fail());
    }
    let height = hwpunit_to_px(row.line_height, dpi);
    let host = Rect {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height,
    };
    let node = RenderNode::new(
        0,
        RenderNodeType::TextLine(TextLineNode::new(
            height,
            hwpunit_to_px(row.baseline_distance, dpi),
        )),
        BoundingBox::new(0.0, 0.0, 0.0, height),
    );
    Ok((
        ParagraphItem::ExcludedTable {
            control: 0,
            line: 0,
            host,
            host_advance: height + hwpunit_to_px(row.line_spacing, dpi),
            x,
            top: hwpunit_to_px(i32::from(a.margin.top), dpi),
            bottom: hwpunit_to_px(i32::from(a.margin.bottom), dpi),
        },
        node,
    ))
}
