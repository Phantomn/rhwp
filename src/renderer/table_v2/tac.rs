//! Stored control-only inline rows. Source ownership is independent of table
//! measurement; full child plans are checked against the same occupied boxes.
use super::{ControlOwner, GeometryError, InlineTableInput, ParagraphItem, Rect, TableContentPlan};
use crate::model::{
    control::Control,
    paragraph::{LineSeg, Paragraph},
    style::Alignment,
};
use crate::renderer::style_resolver::ResolvedStyleSet;
use std::sync::Arc;

/// Source query in HWP units, relative to the first paragraph row. Not a claim
/// that the children's contents, styles or spanning grids are supported.
#[derive(Debug, Clone)]
pub struct StoredTacRow {
    pub source_line: usize,
    pub top: f64,
    pub height: f64,
    pub spacing: f64,
    pub tables: Vec<(usize, Rect)>,
}

fn unsupported() -> GeometryError {
    GeometryError::Unsupported("stored TAC carrier requires unambiguous intact rows")
}

// HWP control slots are source positions, not painted inline width. Keep the
// original indices when secd/cold precede a table. The parser's recorded HWPX
// shift is admitted only for that leading structural prefix; conversion itself
// remains the Paragraph IR accessor's responsibility.
fn complete_stream(para: &Paragraph) -> bool {
    let Some(end) = u32::try_from(para.controls.len())
        .ok()
        .and_then(|n| n.checked_mul(8))
        .and_then(|n| n.checked_add(1))
    else {
        return false;
    };
    let leading = para
        .controls
        .iter()
        .take_while(|c| matches!(c, Control::SectionDef(_) | Control::ColumnDef(_)))
        .count();
    para.text.is_empty()
        && para.char_offsets.is_empty()
        && para.title_marks.is_empty()
        && para.field_ranges.is_empty()
        && para.orphan_field_ends.is_empty()
        && !para.controls.is_empty()
        && para.char_count == end
        && para.controls.iter().all(Control::occupies_ctrl_char_slot)
        && (para.hwpx_axis_shift == 0
            || (para.hwpx_axis_shift.is_multiple_of(8)
                && para.hwpx_axis_shift as usize / 8 <= leading
                && matches!(para.controls.first(), Some(Control::SectionDef(_)))))
        && para
            .line_segs
            .iter()
            .all(|s| s.text_start.checked_add(para.hwpx_axis_shift).is_some())
}

/// Resolve complete 8-unit control streams, including multiple tables on a
/// saved row. No text/control order or page boundary is inferred from overlap.
/// Equal occupied envelopes are the qualified baseline case. Mixed text and
/// unequal envelope ascents/descents require the general inline composer.
pub fn stored_tac_rows(
    para: &Paragraph,
    width_hu: f64,
    alignment: Alignment,
) -> Result<Vec<StoredTacRow>, GeometryError> {
    if !complete_stream(para)
        || para.stored_text_partition_is_dirty()
        || para.source_line_seg_vertical_pos.is_some()
        || para.layout_only_fill_lines != 0
        || para.line_segs.is_empty()
        || !para.range_tags.is_empty()
        || !para.markpen_marks.is_empty()
        || !width_hu.is_finite()
        || width_hu <= 0.0
    {
        return Err(unsupported());
    }
    // A control-only HWPX carrier has no character offsets to distinguish an
    // already-HWP5 saved start from a raw HWPX start. Until provenance resolves
    // that ambiguity, only the invariant first start (zero) is qualified.
    if para.hwpx_axis_shift != 0 && para.line_segs.len() > 1 {
        return Err(GeometryError::Unsupported(
            "ambiguous structural TAC character axis",
        ));
    }
    let origin = para.line_segs[0].vertical_pos;
    if origin < 0 || para.line_seg_text_start(0) != 0 {
        return Err(unsupported());
    }
    let mut rows = Vec::new();
    for (i, row) in para.line_segs.iter().enumerate() {
        if row.tag & LineSeg::TAG_IMPLEMENTATION_PROPERTY != 0
            || !para.line_seg_text_start(i).is_multiple_of(8)
            || u64::from(para.line_seg_text_start(i)) > para.controls.len() as u64 * 8
            || row.tag & LineSeg::TAG_SINGLE_SEGMENT_LINE != LineSeg::TAG_SINGLE_SEGMENT_LINE
            || row.line_height <= 0
            || row.baseline_distance < 0
            || row.baseline_distance > row.line_height
            || row.column_start != 0
            || !same(f64::from(row.segment_width), width_hu)
            || (i > 0
                && (para.line_seg_text_start(i) <= para.line_seg_text_start(i - 1)
                    || i64::from(row.vertical_pos)
                        < i64::from(para.line_segs[i - 1].vertical_pos)
                            + i64::from(para.line_segs[i - 1].line_height)))
        {
            return Err(unsupported());
        }
        rows.push(StoredTacRow {
            source_line: i,
            top: f64::from(row.vertical_pos) - f64::from(origin),
            height: f64::from(row.line_height),
            spacing: f64::from(row.line_spacing),
            tables: Vec::new(),
        });
    }
    for (ci, ctrl) in para.controls.iter().enumerate() {
        // Source geometry only: admitting a page-number slot here does not
        // render it. Document admission separately rejects unhandled stories.
        if matches!(
            ctrl,
            Control::SectionDef(_) | Control::ColumnDef(_) | Control::PageNumberPos(_)
        ) {
            continue;
        }
        let Control::Table(table) = ctrl else {
            return Err(unsupported());
        };
        let a = &table.common;
        if !a.treat_as_char
            || a.width == 0
            || a.height == 0
            || a.horizontal_offset != 0
            || a.vertical_offset != 0
            || a.prevent_page_break != 0
        {
            return Err(unsupported());
        }
        let margins = [
            table.outer_margin_left,
            table.outer_margin_right,
            table.outer_margin_top,
            table.outer_margin_bottom,
        ];
        if margins.iter().any(|v| *v < 0) {
            return Err(unsupported());
        }
        let position = u32::try_from(ci).map_err(|_| unsupported())? * 8;
        let li = (0..rows.len())
            .rfind(|&i| para.line_seg_text_start(i) <= position)
            .ok_or_else(unsupported)?;
        let row = &mut rows[li];
        if f64::from(a.height) + f64::from(margins[2]) + f64::from(margins[3]) != row.height {
            return Err(GeometryError::Unsupported("unequal TAC occupied envelopes"));
        }
        let pen = row.tables.last().map_or(0.0, |(previous, r)| {
            let Control::Table(t) = &para.controls[*previous] else {
                unreachable!()
            };
            r.x + r.width + f64::from(t.outer_margin_right)
        });
        row.tables.push((
            ci,
            Rect {
                x: pen + f64::from(margins[0]),
                y: f64::from(margins[2]),
                width: f64::from(a.width),
                height: f64::from(a.height),
            },
        ));
    }
    for row in &mut rows {
        let Some((ci, last)) = row.tables.last() else {
            continue;
        };
        let Control::Table(table) = &para.controls[*ci] else {
            unreachable!()
        };
        let free = width_hu - last.x - last.width - f64::from(table.outer_margin_right);
        if free < 0.0 {
            return Err(GeometryError::Unsupported("TAC row exceeds stored width"));
        }
        let offset = match alignment {
            Alignment::Left => 0.0,
            Alignment::Justify if row.tables.len() == 1 => 0.0,
            Alignment::Center => free / 2.0,
            Alignment::Right => free,
            _ => return Err(GeometryError::Unsupported("TAC paragraph alignment")),
        };
        for (_, r) in &mut row.tables {
            r.x += offset;
        }
    }
    Ok(rows)
}

pub(super) fn same(a: f64, b: f64) -> bool {
    (a - b).abs() <= 32.0 * f64::EPSILON * a.abs().max(b.abs()).max(1.0)
}

pub(super) fn compose(
    para: &Paragraph,
    width: f64,
    styles: &ResolvedStyleSet,
    dpi: f64,
) -> Result<Vec<ParagraphItem>, GeometryError> {
    let style = styles
        .para_styles
        .get(para.para_shape_id as usize)
        .ok_or_else(unsupported)?;
    if style.border_fill_id != 0
        || style.head_type != crate::model::style::HeadType::None
        || style.keep_lines
        || style.keep_with_next
        || style.widow_orphan
        || style.page_break_before
        || style.margin_left != 0.0
        || style.margin_right != 0.0
        || style.indent != 0.0
    {
        return Err(GeometryError::Unsupported(
            "TAC carrier paragraph constraints",
        ));
    }
    super::contracts::nonnegative(style.spacing_before, "TAC spacing before")?;
    super::contracts::nonnegative(style.spacing_after, "TAC spacing after")?;
    let scale = dpi / 7200.0;
    let rows = stored_tac_rows(para, width / scale, style.alignment)?;
    if rows.iter().any(|r| r.spacing < 0.0) {
        return Err(GeometryError::Unsupported("negative TAC row spacing"));
    }
    let mut items = vec![ParagraphItem::Space(style.spacing_before)];
    let mut end = 0.0;
    for row in rows {
        if row.top < end {
            return Err(unsupported());
        }
        if row.top > end {
            items.push(ParagraphItem::Space((row.top - end) * scale));
        }
        end = row.top + row.height;
        if row.tables.is_empty() {
            items.push(ParagraphItem::Space(row.height * scale));
        } else {
            items.push(ParagraphItem::InlineTables {
                height: row.height * scale,
                tables: row
                    .tables
                    .into_iter()
                    .map(|(ci, r)| {
                        (
                            ci,
                            Rect {
                                x: r.x * scale,
                                y: r.y * scale,
                                width: r.width * scale,
                                height: r.height * scale,
                            },
                        )
                    })
                    .collect(),
            });
        }
    }
    let trailing = para.line_segs.last().ok_or_else(unsupported)?.line_spacing;
    items.push(ParagraphItem::Space(
        f64::from(trailing) * scale + style.spacing_after,
    ));
    Ok(items)
}

pub(super) fn bind(
    owner: ControlOwner,
    rect: Rect,
    plan: Arc<TableContentPlan>,
) -> Result<InlineTableInput, GeometryError> {
    if !same(rect.width, plan.width) || !same(rect.height, plan.height) {
        return Err(GeometryError::Unsupported(
            "TAC content changed stored occupied box",
        ));
    }
    Ok(InlineTableInput {
        owner,
        x: rect.x,
        y: rect.y,
        plan,
    })
}
