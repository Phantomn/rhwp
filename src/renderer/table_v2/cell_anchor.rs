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
        && p.line_segs.len() == 1
        && p.line_segs[0].segment_width == 0
        && excluded_slot(p).is_some()
}

// Keep the structural column slot in the source. Geometry and paint must bind
// the table at its real index, not a copy with controls removed/reordered.
fn excluded_slot(p: &Paragraph) -> Option<usize> {
    match p.controls.as_slice() {
        [Control::Table(t)] if !t.common.treat_as_char => Some(0),
        [Control::ColumnDef(_), Control::Table(t)] if !t.common.treat_as_char => Some(1),
        _ => None,
    }
}

pub(super) fn following_candidate(p: &Paragraph) -> bool {
    // This slice qualifies saved text-bearing hosts (including literal spaces).
    // Truly empty, positive-width control carriers need independent evidence;
    // they are not converted from the zero-width exclusion contract.
    !p.text.is_empty()
        && p.controls.len() == 1
        && p.line_segs.len() == 1
        && p.line_segs[0].segment_width > 0
        && matches!(&p.controls[0], Control::Table(t) if !t.common.treat_as_char)
}

/// A full-width saved host follows the paragraph-top exclusion. TextComposer
/// independently qualifies its stored partition and supplies the real line
/// envelope/advance; no visibility test or inferred zero-height spacer is used.
pub(super) fn compose_following(
    p: &Paragraph,
    width: f64,
    styles: &ResolvedStyleSet,
    dpi: f64,
) -> Result<ParagraphItem, GeometryError> {
    let fail = || GeometryError::Unsupported("stored following cell anchor");
    if !following_candidate(p) {
        return Err(fail());
    }
    let Control::Table(t) = &p.controls[0] else {
        return Err(fail());
    };
    let a = &t.common;
    let style = styles
        .para_styles
        .get(p.para_shape_id as usize)
        .ok_or_else(fail)?;
    if a.text_wrap != TextWrap::TopAndBottom
        || a.vert_rel_to != VertRelTo::Para
        || a.vert_align != VertAlign::Top
        || a.horz_rel_to != HorzRelTo::Para
        || a.horz_align != HorzAlign::Left
        || a.vertical_offset != 0
        || (a.horizontal_offset as i32) < 0
        || !a.flow_with_text
        || a.allow_overlap
        || a.prevent_page_break != 0
        || style.margin_left != 0.0
        || style.margin_right != 0.0
        || style.spacing_before != 0.0
        || style.spacing_after != 0.0
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
    if x > width {
        return Err(fail());
    }
    Ok(ParagraphItem::PositionedTable {
        control: 0,
        x,
        top: hwpunit_to_px(i32::from(a.margin.top), dpi),
        bottom: hwpunit_to_px(i32::from(a.margin.bottom), dpi),
    })
}

pub(super) fn compose(
    p: &Paragraph,
    width: f64,
    styles: &ResolvedStyleSet,
    dpi: f64,
    single_column: bool,
) -> Result<(ParagraphItem, RenderNode), GeometryError> {
    compose_in_frame(p, width, styles, dpi, single_column, true)
}

/// The document adapter has qualified a single body column. With zero paragraph
/// side margins its left origin is also the paragraph reference. A cell does not
/// own that document column. A cell's explicit initial single lane is qualified
/// separately below, using the same declaration contract as IR binding.
pub(super) fn compose_body(
    p: &Paragraph,
    width: f64,
    styles: &ResolvedStyleSet,
    dpi: f64,
) -> Result<(ParagraphItem, RenderNode), GeometryError> {
    compose_in_frame(p, width, styles, dpi, true, false)
}

fn compose_in_frame(
    p: &Paragraph,
    width: f64,
    styles: &ResolvedStyleSet,
    dpi: f64,
    single_column: bool,
    cell_frame: bool,
) -> Result<(ParagraphItem, RenderNode), GeometryError> {
    let fail = || GeometryError::Unsupported("stored excluded cell anchor");
    if !candidate(p) {
        return Err(fail());
    }
    let control = excluded_slot(p).ok_or_else(fail)?;
    // bind_table has already checked that this is the cell's first paragraph.
    // Reuse its lane qualification here; never admit a multi-column/shifted
    // reference just because the control array starts with a ColumnDef.
    let local_column = control == 1 && super::ir::initial_cell_column(p, 0)?;
    let Control::Table(t) = &p.controls[control] else {
        return Err(fail());
    };
    let a = &t.common;
    let row = &p.line_segs[0];
    let style = styles
        .para_styles
        .get(p.para_shape_id as usize)
        .ok_or_else(fail)?;
    // A column-relative table does not inherit the text's horizontal insets.
    // Its host still starts after paragraph-before spacing, while Para/Top
    // anchors the object at the paragraph origin BEFORE that text spacing.
    // Normal saved-document PDF independently distinguishes these origins.
    // Keep both origins
    // in the same ExcludedTable result consumed by fitting and painting.
    let inset_column =
        cell_frame && (single_column || local_column) && a.horz_rel_to == HorzRelTo::Column;
    let before = if inset_column {
        style.spacing_before
    } else {
        0.0
    };
    if p.stored_text_partition_is_dirty()
        || !p.char_offsets.is_empty()
        || p.source_line_seg_vertical_pos.is_some()
        || p.layout_only_fill_lines != 0
        || !p.field_ranges.is_empty()
        || !p.range_tags.is_empty()
        || !p.title_marks.is_empty()
        || !p.markpen_marks.is_empty()
        || row.text_start != 0
        || if inset_column {
            (hwpunit_to_px(row.column_start, dpi) - style.margin_left).abs() > 1e-7
                || style.margin_left < 0.0
                || style.margin_right < 0.0
                || style.margin_left + style.margin_right >= width
                || !before.is_finite()
                || before < 0.0
        } else {
            row.column_start != 0
        }
        || row.line_height <= 0
        || row.text_height != row.line_height
        || row.baseline_distance < 0
        || row.baseline_distance > row.line_height
        || row.line_spacing < 0
        || row.tag != LineSeg::TAG_SINGLE_SEGMENT_LINE
        || a.text_wrap != TextWrap::TopAndBottom
        || a.vert_rel_to != VertRelTo::Para
        || a.vert_align != VertAlign::Top
        || !(a.horz_rel_to == HorzRelTo::Para
            || ((single_column || local_column) && a.horz_rel_to == HorzRelTo::Column))
        || a.horz_align != HorzAlign::Left
        || (a.vertical_offset as i32) < 0
        || (a.horizontal_offset as i32) < 0
        || !a.flow_with_text
        || a.allow_overlap
        || a.prevent_page_break != 0
        || style.head_type != HeadType::None
        || style.keep_lines
        || style.keep_with_next
        || style.widow_orphan
        || style.page_break_before
        || (!inset_column
            && (style.margin_left != 0.0
                || style.margin_right != 0.0
                || style.spacing_before != 0.0))
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
        x: hwpunit_to_px(row.column_start, dpi),
        y: before,
        width: 0.0,
        height,
    };
    let node = RenderNode::new(
        0,
        RenderNodeType::TextLine(TextLineNode::new(
            height,
            hwpunit_to_px(row.baseline_distance, dpi),
        )),
        BoundingBox::new(host.x, host.y, 0.0, height),
    );
    Ok((
        ParagraphItem::ExcludedTable {
            control,
            line: 0,
            host,
            host_advance: before + height + hwpunit_to_px(row.line_spacing, dpi),
            x,
            offset_y: hwpunit_to_px(a.vertical_offset as i32, dpi),
            top: hwpunit_to_px(i32::from(a.margin.top), dpi),
            bottom: hwpunit_to_px(i32::from(a.margin.bottom), dpi),
        },
        node,
    ))
}
