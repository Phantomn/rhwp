//! Qualified, single-segment stored text in a paragraph-local frame.
//! This does not infer page breaks from overlapping boxes or regenerate saved rows.
use crate::{
    model::paragraph::{LineSeg, Paragraph},
    renderer::{
        hwpunit_to_px,
        render_tree::{RenderNode, RenderNodeType},
    },
};

use super::GeometryError;

fn unsupported() -> GeometryError {
    GeometryError::Unsupported("stored text requires intact single-segment rows")
}

/// Preserve source text partitions and metrics, changing only the coordinate
/// origin. The caller owns the paragraph origin; page/column resets need a
/// separate continuation contract and are deliberately not admitted here.
pub(super) fn localize(para: &Paragraph, width: f64, dpi: f64) -> Result<Paragraph, GeometryError> {
    if para.stored_text_partition_is_dirty()
        || para.text.chars().any(char::is_control)
        || para.line_segs.is_empty()
        || para.char_offsets.len() != para.text.chars().count()
        || para.char_offsets.windows(2).any(|p| p[0] >= p[1])
    {
        return Err(unsupported());
    }
    let first = para.line_segs[0].vertical_pos;
    if para.line_seg_text_start(0) != 0 {
        return Err(unsupported());
    }
    for (i, row) in para.line_segs.iter().enumerate() {
        if row.tag & LineSeg::TAG_IMPLEMENTATION_PROPERTY != 0
            || row.tag & LineSeg::TAG_SINGLE_SEGMENT_LINE != LineSeg::TAG_SINGLE_SEGMENT_LINE
            || row.tag & (LineSeg::TAG_AUTO_HYPHENATION | LineSeg::TAG_PARAGRAPH_HEAD) != 0
            || (i > 0
                && row.tag & (LineSeg::TAG_FIRST_LINE_OF_PAGE | LineSeg::TAG_FIRST_LINE_OF_COLUMN)
                    != 0)
            || row.line_height <= 0
            || row.text_height != row.line_height
            || row.baseline_distance < 0
            || row.baseline_distance > row.line_height
            || row.line_spacing < 0
            || row.column_start != 0
            || row.segment_width <= 0
            || !same(hwpunit_to_px(row.segment_width, dpi), width)
            || row.vertical_pos.checked_sub(first).is_none_or(|v| v < 0)
        {
            return Err(unsupported());
        }
        if i > 0 {
            let previous = &para.line_segs[i - 1];
            if i64::from(row.vertical_pos)
                < i64::from(previous.vertical_pos) + i64::from(previous.line_height)
                || para.line_seg_text_start(i) <= para.line_seg_text_start(i - 1)
                || !para.char_offsets.contains(&para.line_seg_text_start(i))
            {
                return Err(unsupported());
            }
        }
    }
    let mut local = para.clone();
    for row in &mut local.line_segs {
        row.vertical_pos -= first;
    }
    Ok(local)
}

fn same(a: f64, b: f64) -> bool {
    (a - b).abs() <= 32.0 * f64::EPSILON * a.abs().max(b.abs()).max(1.0)
}

/// Shared glyph layout must actually honor the admitted stored geometry. A
/// downstream correction is an unsupported path, not permission to overwrite
/// the measurement or clamp its paint. Geometry and paint then consume these
/// very same nodes through the ordinary V2 content pipeline.
pub(super) fn validate_paint(
    para: &Paragraph,
    nodes: &[RenderNode],
    end: f64,
    before: f64,
    after: f64,
    dpi: f64,
) -> Result<(), GeometryError> {
    if nodes.len() != para.line_segs.len() {
        return Err(unsupported());
    }
    let mut painted = String::new();
    let chars: Vec<_> = para.text.chars().collect();
    for (i, (node, row)) in nodes.iter().zip(&para.line_segs).enumerate() {
        let RenderNodeType::TextLine(line) = &node.node_type else {
            return Err(unsupported());
        };
        if !same(node.bbox.y, before + hwpunit_to_px(row.vertical_pos, dpi))
            || !same(node.bbox.height, hwpunit_to_px(row.line_height, dpi))
            || !same(line.baseline, hwpunit_to_px(row.baseline_distance, dpi))
        {
            return Err(GeometryError::Unsupported(
                "shared text paint changes stored metrics",
            ));
        }
        let mut row_text = String::new();
        for run in &node.children {
            let RenderNodeType::TextRun(run) = &run.node_type else {
                return Err(unsupported());
            };
            row_text.push_str(&run.text);
        }
        let start = para
            .char_offsets
            .partition_point(|&p| p < para.line_seg_text_start(i));
        let stop = if i + 1 < nodes.len() {
            para.char_offsets
                .partition_point(|&p| p < para.line_seg_text_start(i + 1))
        } else {
            chars.len()
        };
        if row_text != chars[start..stop].iter().collect::<String>() {
            return Err(GeometryError::Unsupported(
                "shared text paint changes stored line membership",
            ));
        }
        painted.push_str(&row_text);
    }
    let last = para.line_segs.last().ok_or_else(unsupported)?;
    let expected_end = before
        + hwpunit_to_px(last.vertical_pos, dpi)
        + hwpunit_to_px(last.line_height, dpi)
        + hwpunit_to_px(last.line_spacing, dpi)
        + after;
    if painted != para.text || !same(end, expected_end) {
        return Err(GeometryError::Unsupported(
            "shared text paint changes stored ownership or advance",
        ));
    }
    Ok(())
}
