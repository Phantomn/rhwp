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

/// SQUEEZE's saved line partition is reusable without reflow or widening the
/// cell. Normal localization and final paint validation still apply afterward.
/// Recomposition needs a no-wrap shaping policy, not the default BREAK path.
pub(super) fn validate_cell_wrap(para: &Paragraph, line_wrap: u8) -> Result<(), GeometryError> {
    if line_wrap == 0 {
        return Ok(());
    }
    if line_wrap != crate::model::table::CELL_LINE_WRAP_SQUEEZE {
        return Err(GeometryError::Unsupported("cell line wrap policy"));
    }
    if para.line_segs.is_empty()
        || para.stored_text_partition_is_dirty()
        || !para.controls.is_empty()
    {
        return Err(GeometryError::Unsupported(
            "SQUEEZE requires intact stored text rows",
        ));
    }
    Ok(())
}

/// Convert a saved CENTER alignment reference to the glyph baseline consumed
/// by shared text paint. Source IR is untouched. With a uniform em box, CENTER
/// and BASELINE have identical glyph placement; Hancom stores half the em for
/// the former and the font baseline for the latter. Do not mistake the CENTER
/// reference for an undersized glyph ascent and clamp it.
/// Mixed em sizes share the line's center, not its largest glyph's baseline.
/// The returned centers are consumed before final paint nodes are measured and
/// lowered into flow units. Scripts remain outside this plain-text contract.
pub(super) fn resolve_vertical_alignment(
    para: &mut Paragraph,
    styles: &crate::renderer::style_resolver::ResolvedStyleSet,
    dpi: f64,
    stored: bool,
) -> Result<Option<Vec<f64>>, GeometryError> {
    use crate::renderer::{composer, style_resolver::ParagraphVerticalAlignment};
    let style = &styles.para_styles[para.para_shape_id as usize];
    match style.vertical_alignment {
        ParagraphVerticalAlignment::Baseline => return Ok(None),
        ParagraphVerticalAlignment::Center => {}
        _ => return Err(GeometryError::Unsupported("text vertical alignment")),
    }
    let composed = composer::compose_paragraph(para);
    let mut centers = Vec::with_capacity(composed.lines.len());
    for (row, line) in para.line_segs.iter_mut().zip(&composed.lines) {
        let fonts: Vec<_> = line.runs.iter().map(|r| r.text_style(styles)).collect();
        let size = fonts
            .iter()
            .map(|f| f.font_size)
            .reduce(f64::max)
            .unwrap_or_else(|| {
                styles.char_styles[para.char_shapes[0].char_shape_id as usize].font_size
            });
        if fonts.iter().any(|f| f.superscript || f.subscript) {
            return Err(GeometryError::Unsupported("scripted CENTER text"));
        }
        let metrics = composer::frame_metrics_for_line(
            size,
            size,
            style.line_spacing_type,
            style.line_spacing,
            dpi,
        );
        if stored {
            // Qualified normal saves only: a CENTER reference inside an em box.
            // Larger object-owned boxes and stale metrics are separate contracts.
            if row.text_height != metrics.text_height
                || i64::from(row.baseline_distance) * 2 != i64::from(row.text_height)
            {
                return Err(GeometryError::Unsupported("stored CENTER reference"));
            }
            row.baseline_distance = metrics.baseline_distance;
        }
        centers.push(hwpunit_to_px(row.text_height, dpi) / 2.0);
    }
    Ok(Some(centers))
}

/// Apply the shared em-center result to the final run payload, not to a single
/// output backend. Keep line boxes/advance intact: CENTER changes glyph origins,
/// not the line partition or the occupied maximum-em envelope.
pub(super) fn align_center_runs(
    centers: &[f64],
    nodes: &mut [RenderNode],
    style: &crate::renderer::style_resolver::ResolvedParaStyle,
    dpi: f64,
) -> Result<(), GeometryError> {
    if centers.len() != nodes.len() {
        return Err(GeometryError::Unsupported("CENTER line ownership"));
    }
    for (center, line) in centers.iter().zip(nodes) {
        for node in &mut line.children {
            let RenderNodeType::TextRun(run) = &mut node.node_type else {
                return Err(GeometryError::Unsupported("CENTER non-text payload"));
            };
            let size = run.style.font_size;
            let metrics = crate::renderer::composer::frame_metrics_for_line(
                size,
                size,
                style.line_spacing_type,
                style.line_spacing,
                dpi,
            );
            run.baseline =
                line.bbox.y - node.bbox.y + center + hwpunit_to_px(metrics.baseline_distance, dpi)
                    - hwpunit_to_px(metrics.text_height, dpi) / 2.0;
        }
    }
    Ok(())
}

/// A normal Hancom save can retain a minimum-width text lane even when the
/// padded cell is narrower. Reuse the common rule, not a maximum observed sw.
/// This extension qualifies saved plain rows only; it does not enlarge object
/// frames, invent fresh line breaks or change physical cell padding/borders.
pub(super) fn cell_lane_width(
    paragraphs: &[Paragraph],
    cell_width: f64,
    padding: super::Insets,
    scale: f64,
) -> Result<Option<f64>, GeometryError> {
    let physical = cell_width - padding.left - padding.right;
    let minimum = crate::renderer::composer::cell_inner_text_width(
        cell_width,
        padding.left,
        padding.right,
        scale * 7200.0,
    );
    if minimum <= physical
        || !paragraphs.iter().any(|p| {
            p.line_segs
                .iter()
                .any(|s| s.column_start == 0 && same(f64::from(s.segment_width) * scale, minimum))
        })
    {
        return Ok(None);
    }
    if paragraphs
        .iter()
        .any(|p| !p.controls.is_empty() || p.line_segs.is_empty())
    {
        return Err(GeometryError::Unsupported(
            "minimum cell lane requires saved plain rows",
        ));
    }
    // localize still validates source partition, row tags, margins, indentation
    // and final bounds. Invalid/stale widths do not become a new width oracle.
    Ok(Some(minimum))
}

fn unsupported() -> GeometryError {
    GeometryError::Unsupported("stored text requires intact single-segment rows")
}

/// Explicit stored LEFT advances use the common glyph walk unchanged. Missing
/// widths, leaders and other tab alignments need separate replay contracts.
/// The common inline-tab walker currently converts HU at96dpi; do not silently
/// accept a different scale here. Reserved extension words are not semantics
/// (normal HWP saves may write either zero or space into them).
pub(super) fn validate_tabs(para: &Paragraph, dpi: f64) -> Result<(), GeometryError> {
    let chars: Vec<_> = para.text.chars().collect();
    let tabs: Vec<_> = chars
        .iter()
        .enumerate()
        .filter_map(|(i, c)| (*c == '\t').then_some(i))
        .collect();
    if tabs.is_empty() {
        return Ok(());
    }
    // The common multi-row/run replay does not yet slice tab ordinals for all
    // consumers. Admit the proven single-row/single-tab path, not a wrong width.
    if dpi != 96.0
        || para.line_segs.len() != 1
        || tabs.len() != 1
        || para.stored_text_partition_is_dirty()
        || para.char_offsets.len() != chars.len()
        || tabs.len() != para.tab_extended.len()
        || para
            .tab_extended
            .iter()
            .any(|ext| ext[0] == 0 || ext[1] != 0 || ext[2] != 0x0100 || ext[6] != 9)
        || tabs.iter().any(|&i| {
            let end = para.char_offsets[i].checked_add(8);
            end.is_none_or(|end| {
                para.char_offsets
                    .get(i + 1)
                    .copied()
                    .unwrap_or(para.char_count)
                    < end
                    || para
                        .line_segs
                        .iter()
                        .any(|row| row.text_start > para.char_offsets[i] && row.text_start < end)
            })
        })
    {
        return Err(GeometryError::Unsupported(
            "stored inline LEFT tab contract",
        ));
    }
    Ok(())
}

/// Qualify paragraph-boundary resets in one saved cell story. Comparing line
/// origins (not their overlapping bottoms) is essential: negative spacing is
/// not a page break. Fresh/edited/mixed stories have no reusable page frames.
/// Individual paragraphs still pass the composer's width/partition admission.
pub(super) fn cell_frame_starts(
    paragraphs: &[Paragraph],
    styles: &crate::renderer::style_resolver::ResolvedStyleSet,
    dpi: f64,
) -> Result<Vec<(usize, usize)>, GeometryError> {
    Ok(cell_frames(paragraphs, styles, dpi)?.0)
}

pub(super) fn child_frame_tails(
    paragraphs: &[Paragraph],
    styles: &crate::renderer::style_resolver::ResolvedStyleSet,
    dpi: f64,
) -> Result<Vec<(usize, f64)>, GeometryError> {
    Ok(cell_frames(paragraphs, styles, dpi)?.1)
}

type CellFrames = (Vec<(usize, usize)>, Vec<(usize, f64)>);

fn cell_frames(
    paragraphs: &[Paragraph],
    styles: &crate::renderer::style_resolver::ResolvedStyleSet,
    dpi: f64,
) -> Result<CellFrames, GeometryError> {
    if paragraphs.iter().any(|p| {
        p.line_segs.is_empty()
            || p.stored_text_partition_is_dirty()
            || p.source_line_seg_vertical_pos.is_some()
            || p.layout_only_fill_lines != 0
    }) {
        return Ok((Vec::new(), Vec::new()));
    }
    let mut starts = Vec::new();
    let mut tails = Vec::new();
    let mut previous = None;
    for (pi, p) in paragraphs.iter().enumerate() {
        for (li, line) in p.line_segs.iter().enumerate() {
            if line.vertical_pos < 0 {
                return Err(GeometryError::Unsupported("negative stored cell origin"));
            }
            // Equality alone is not a frame boundary: local zero origins and
            // non-advancing/overlapping paragraph flows are legitimate. A
            // normal saved plain paragraph can, however, restart at the SAME
            // positive before-spacing origin as the preceding frame's sole
            // paragraph. Its authored advance rules out same-frame overlap.
            let repeated_frame_origin = li == 0
                && pi > 0
                && line.vertical_pos > 0
                && previous == Some(line.vertical_pos)
                && p.controls.is_empty()
                // Initial single-column metadata is not an inline occupant;
                // bind_table independently validates its position and shape.
                && paragraphs[pi - 1].controls.iter().all(|control| {
                    matches!(control, crate::model::control::Control::ColumnDef(_))
                })
                && styles
                    .para_styles
                    .get(p.para_shape_id as usize)
                    .is_some_and(|style| {
                        same(hwpunit_to_px(line.vertical_pos, dpi), style.spacing_before)
                            && styles
                                .para_styles
                                .get(paragraphs[pi - 1].para_shape_id as usize)
                                .zip(paragraphs[pi - 1].line_segs.last())
                                .is_some_and(|(prior_style, prior_line)| {
                                    prior_line.line_height > 0
                                        // Negative gaps can express overlap;
                                        // equality does not disambiguate those.
                                        && prior_line.line_spacing >= 0
                                        && prior_style.spacing_after >= 0.0
                                })
                    });
            if previous.is_some_and(|v| line.vertical_pos < v) || repeated_frame_origin {
                // A paragraph's first row may start after its authored before
                // spacing (normal Hancom saves retain that nonzero origin).
                // An internal continuation does not reapply paragraph spacing.
                // Qualify against resolved style, not a numeric reset threshold.
                let at_paragraph_start = li == 0
                    && styles
                        .para_styles
                        .get(p.para_shape_id as usize)
                        .is_some_and(|style| {
                            (hwpunit_to_px(line.vertical_pos, dpi) - style.spacing_before).abs()
                                < 1e-7
                        });
                if line.vertical_pos != 0 && !at_paragraph_start {
                    // A split exclusion may finish on this frame. This is a
                    // candidate only: binding must corroborate BOTH child
                    // fragments, not silently discard this source transition.
                    if li == 0 && pi > 0 && super::cell_anchor::candidate(&paragraphs[pi - 1]) {
                        tails.push((pi - 1, hwpunit_to_px(line.vertical_pos, dpi)));
                        previous = Some(line.vertical_pos);
                        continue;
                    }
                    return Err(GeometryError::Unsupported(
                        "unqualified stored cell frame reset",
                    ));
                }
                starts.push((pi, li));
            }
            previous = Some(line.vertical_pos);
        }
    }
    Ok((starts, tails))
}

/// Lift admitted frame coordinates into one paragraph-local story for the
/// shared text composer. Text, UTF-16 offsets and line ownership are untouched.
/// The IR adapter lowers the same source cuts back into physical frame markers.
/// Cell and document adapters must each qualify and lower their own frame cuts;
/// this helper alone is not permission to reuse edited caches or page resets.
pub(super) fn continuous_paragraph(
    para: &Paragraph,
    starts: &[usize],
) -> Result<Paragraph, GeometryError> {
    // Qualified field markers have source slots, but no object geometry.
    // Lift only their row origins; keep ranges/offsets intact across the cut.
    if (!para.controls.is_empty() && !super::fields::stored_result(para)?)
        || para.stored_text_partition_is_dirty()
    {
        return Err(GeometryError::Unsupported(
            "stored cell continuation controls",
        ));
    }
    let mut local = para.clone();
    let mut offset = 0_i64;
    for (i, row) in para.line_segs.iter().enumerate() {
        if starts.contains(&i) {
            let previous = &local.line_segs[i - 1];
            offset = i64::from(previous.vertical_pos)
                + i64::from(previous.line_height)
                + i64::from(previous.line_spacing);
            // These flags describe the source frame already owned by starts.
            local.line_segs[i].tag &=
                !(LineSeg::TAG_FIRST_LINE_OF_PAGE | LineSeg::TAG_FIRST_LINE_OF_COLUMN);
        }
        local.line_segs[i].vertical_pos = i32::try_from(i64::from(row.vertical_pos) + offset)
            .map_err(|_| GeometryError::Unsupported("stored cell story extent"))?;
    }
    Ok(local)
}

/// Preserve source text partitions and vertical metrics. Resolve the saved
/// indentation flag into the physical row box once, before shared composition.
/// The caller owns the paragraph origin; page/column resets need a
/// separate continuation contract and are deliberately not admitted here.
pub(super) fn localize(
    para: &Paragraph,
    content: std::ops::Range<f64>,
    indent: f64,
    dpi: f64,
) -> Result<(Paragraph, Vec<std::ops::Range<f64>>), GeometryError> {
    validate_tabs(para, dpi)?;
    if para.stored_text_partition_is_dirty()
        || para
            .text
            .chars()
            .any(|c| c.is_control() && c != '\t' && c != '\n')
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
    // A source LF owns a visual boundary, not a printable glyph. Reuse a
    // stored partition only when it records the following row at that exact
    // UTF-16 boundary. Otherwise composition would invent rows with copied
    // metrics. This also preserves leading/consecutive author-owned blanks.
    let starts: Vec<_> = (0..para.line_segs.len())
        .map(|i| para.line_seg_text_start(i))
        .collect();
    for (i, ch) in para.text.chars().enumerate() {
        if ch == '\n'
            && para.char_offsets[i]
                .checked_add(1)
                .is_none_or(|end| starts[1..].binary_search(&end).is_err())
        {
            return Err(unsupported());
        }
    }
    // The final LF may be followed by an explicitly stored empty row and
    // the paragraph terminator, even though no visible character starts it.
    let terminal_blank = para
        .text
        .ends_with('\n')
        .then(|| {
            para.char_offsets.last().copied().and_then(|offset| {
                let end = offset.checked_add(1)?;
                (end.checked_add(1) == Some(para.char_count)).then_some(end)
            })
        })
        .flatten();
    for (i, row) in para.line_segs.iter().enumerate() {
        // Saved lines own a physical interval inside the available frame.
        // It need not fill the frame (rounding, insets or a narrower lane).
        // A contained stored interval already accounts for paragraph margins.
        // Do not add them a second time in the physical-row paint path. If the
        // source interval violates those insets, reject it rather than shrinking
        // or moving glyphs while retaining stale line membership. Stored cs/sw
        // includes margins, but not the indentation indicated by bit20.
        let left = hwpunit_to_px(row.column_start, dpi);
        let right = left + hwpunit_to_px(row.segment_width, dpi);
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
            || i64::from(row.line_height) + i64::from(row.line_spacing) < 0
            || row.column_start < 0
            || row.segment_width <= 0
            || (left < content.start && !same(left, content.start))
            || (right > content.end && !same(right, content.end))
            || row.vertical_pos.checked_sub(first).is_none_or(|v| v < 0)
        {
            return Err(unsupported());
        }
        if i > 0 {
            let previous = &para.line_segs[i - 1];
            if i64::from(row.vertical_pos)
                < i64::from(previous.vertical_pos)
                    + i64::from(previous.line_height)
                    + i64::from(previous.line_spacing.min(0))
                || para.line_seg_text_start(i) <= para.line_seg_text_start(i - 1)
                || (!super::fields::is_row_start(para, para.line_seg_text_start(i))?
                    && terminal_blank != Some(para.line_seg_text_start(i)))
            {
                return Err(unsupported());
            }
        }
    }
    // Resolved indentation is signed: positive first-line / negative hanging.
    // The stored row flag, not a newly guessed row index, owns its application.
    // Resolved URC insets can be fractional HU; LineSeg has integer HU. Keep
    // the physical intervals separately instead of rounding them back
    // into the source record. Measurement and glyph layout consume these boxes.
    let inset = indent.abs();
    if !inset.is_finite() {
        return Err(GeometryError::Unsupported("stored indentation precision"));
    }
    let mut local = para.clone();
    let mut boxes = Vec::with_capacity(local.line_segs.len());
    for row in &mut local.line_segs {
        row.vertical_pos -= first;
        let mut start = hwpunit_to_px(row.column_start, dpi);
        let end = start + hwpunit_to_px(row.segment_width, dpi);
        if row.has_indentation() {
            if inset >= end - start {
                return Err(GeometryError::Unsupported(
                    "stored indentation exceeds line",
                ));
            }
            start += inset;
        }
        boxes.push(start..end);
    }
    Ok((local, boxes))
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
    boxes: &[std::ops::Range<f64>],
    nodes: &[RenderNode],
    end: f64,
    before: f64,
    after: f64,
    dpi: f64,
) -> Result<(), GeometryError> {
    if nodes.len() != para.line_segs.len() || boxes.len() != nodes.len() {
        return Err(unsupported());
    }
    let mut painted = String::new();
    let chars: Vec<_> = para.text.chars().collect();
    for (i, (node, row)) in nodes.iter().zip(&para.line_segs).enumerate() {
        let RenderNodeType::TextLine(line) = &node.node_type else {
            return Err(unsupported());
        };
        if !same(node.bbox.x, boxes[i].start)
            || !same(node.bbox.width, boxes[i].end - boxes[i].start)
            || !same(node.bbox.y, before + hwpunit_to_px(row.vertical_pos, dpi))
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
        let hard_break = chars[start..stop].last() == Some(&'\n');
        let visible_stop = stop - usize::from(hard_break);
        let painted_break = node.children.last().is_some_and(
            |node| matches!(&node.node_type, RenderNodeType::TextRun(run) if run.is_line_break_end),
        );
        if row_text != chars[start..visible_stop].iter().collect::<String>()
            || painted_break != hard_break
        {
            return Err(GeometryError::Unsupported(
                "shared text paint changes stored line membership",
            ));
        }
        painted.push_str(&row_text);
        if hard_break {
            painted.push('\n');
        }
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
