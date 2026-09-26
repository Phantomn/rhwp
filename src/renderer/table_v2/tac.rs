//! Stored control-only inline rows. Source ownership is independent of table
//! measurement; full child plans are checked against the same occupied boxes.
use super::{ControlOwner, GeometryError, InlineTableInput, ParagraphItem, Rect, TableContentPlan};
use crate::model::{
    control::Control,
    paragraph::{LineSeg, Paragraph},
    style::Alignment,
};
use crate::renderer::render_tree::{BoundingBox, RenderNode, RenderNodeType, TextLineNode};
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
    pub spaces: Vec<(usize, Rect)>,
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
    stored_object_rows(para, width_hu, alignment, false)
}

// Same source-slot/line query for images. `tables` in the public legacy result
// names control indices; only the private image adapter consumes image slots.
pub(super) fn stored_object_rows(
    para: &Paragraph,
    width_hu: f64,
    alignment: Alignment,
    pictures: bool,
) -> Result<Vec<StoredTacRow>, GeometryError> {
    object_rows(para, width_hu, alignment, pictures, &[])
}

fn object_rows(
    para: &Paragraph,
    width_hu: f64,
    alignment: Alignment,
    pictures: bool,
    spaces: &[super::tac_spaces::SpaceRun],
) -> Result<Vec<StoredTacRow>, GeometryError> {
    let positions = para.control_utf16_positions();
    let complete = if spaces.is_empty() {
        complete_stream(para)
    } else {
        let mut spans: Vec<_> = positions
            .iter()
            .map(|&p| (p, 8u32))
            .chain(spaces.iter().map(|s| (s.position, 1)))
            .collect();
        spans.sort_unstable();
        let mut end = 0u32;
        let contiguous = spans.iter().all(|&(p, n)| {
            if p != end {
                return false;
            }
            let Some(next) = end.checked_add(n) else {
                return false;
            };
            end = next;
            true
        });
        contiguous
            && end.checked_add(1) == Some(para.char_count)
            && positions.len() == para.controls.len()
            && para.controls.iter().all(Control::occupies_ctrl_char_slot)
            && para.title_marks.is_empty()
            && para.field_ranges.is_empty()
            && para.orphan_field_ends.is_empty()
    };
    if !complete
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
            || (if spaces.is_empty() {
                !para.line_seg_text_start(i).is_multiple_of(8)
                    || u64::from(para.line_seg_text_start(i)) > para.controls.len() as u64 * 8
            } else {
                !positions.contains(&para.line_seg_text_start(i))
                    && !spaces
                        .iter()
                        .any(|s| s.position == para.line_seg_text_start(i))
            })
            || row.tag & LineSeg::TAG_SINGLE_SEGMENT_LINE != LineSeg::TAG_SINGLE_SEGMENT_LINE
            || row.line_height <= 0
            || row.baseline_distance < 0
            || row.baseline_distance > row.line_height
            || row.column_start < 0
            || row.segment_width <= 0
            || (f64::from(row.column_start) + f64::from(row.segment_width) > width_hu
                && !same(
                    f64::from(row.column_start) + f64::from(row.segment_width),
                    width_hu,
                ))
            || (i > 0
                && (para.line_seg_text_start(i) <= para.line_seg_text_start(i - 1)
                    || row.vertical_pos <= para.line_segs[i - 1].vertical_pos))
        {
            return Err(unsupported());
        }
        rows.push(StoredTacRow {
            source_line: i,
            top: f64::from(row.vertical_pos) - f64::from(origin),
            height: f64::from(row.line_height),
            spacing: f64::from(row.line_spacing),
            tables: Vec::new(),
            spaces: Vec::new(),
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
        let (a, margins) = object_box(ctrl, pictures)?;
        if !a.treat_as_char
            || a.width == 0
            || a.height == 0
            || a.horizontal_offset != 0
            || a.vertical_offset != 0
            || a.prevent_page_break != 0
        {
            return Err(unsupported());
        }
        // HWPUNIT16 margins are signed. Table line advance and its physical
        // rectangle differ with negative margins; the table composer carries
        // both below. The picture adapter does not yet have that contract.
        if (pictures && margins.iter().any(|v| *v < 0))
            || f64::from(a.width) + f64::from(margins[0]) + f64::from(margins[1]) <= 0.0
        {
            return Err(unsupported());
        }
        let position = positions[ci];
        let li = (0..rows.len())
            .rfind(|&i| para.line_seg_text_start(i) <= position)
            .ok_or_else(unsupported)?;
        let row = &mut rows[li];
        if f64::from(a.height) + f64::from(margins[2]) + f64::from(margins[3]) != row.height {
            return Err(GeometryError::Unsupported("unequal TAC occupied envelopes"));
        }
        let mut pen = if let Some((previous, r)) = row.tables.last() {
            r.x + r.width + f64::from(object_box(&para.controls[*previous], pictures)?.1[1])
        } else {
            0.0
        };
        let start = row
            .tables
            .last()
            .map_or(para.line_seg_text_start(li), |(previous, _)| {
                positions[*previous] + 8
            });
        pen += spaces
            .iter()
            .filter(|s| s.position >= start && s.position < position)
            .map(|s| s.width_hu)
            .sum::<f64>();
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
        let source_row = &para.line_segs[row.source_line];
        let stop = rows_stop(para, row.source_line);
        let (occupied, after_last) = if let Some((ci, last)) = row.tables.last() {
            let (_, margins) = object_box(&para.controls[*ci], pictures)?;
            (
                last.x + last.width + f64::from(margins[1]),
                positions[*ci] + 8,
            )
        } else {
            (0.0, para.line_seg_text_start(row.source_line))
        };
        let trailing: f64 = spaces
            .iter()
            .filter(|s| s.position >= after_last && s.position < stop)
            .map(|s| s.width_hu)
            .sum();
        let mut free = f64::from(source_row.segment_width) - occupied - trailing;
        if free < 0.0 {
            // A single unbreakable table can itself exceed its saved line.
            // Normal Hancom saves retain its full width and outside margins,
            // starting at the line origin even for right alignment. There is
            // no spare alignment space, not a smaller object/physical box.
            // Multiple objects or leading content need the general breaking
            // composer; trailing spaces remain owned and painted below.
            let single_overwide_table = !pictures
                && matches!(
                    alignment,
                    Alignment::Left | Alignment::Justify | Alignment::Right
                )
                && row.tables.len() == 1
                && row.tables.first().is_some_and(|(ci, rect)| {
                    let (_, margins) = object_box(&para.controls[*ci], false).unwrap();
                    rect.x == f64::from(margins[0])
                        && occupied > f64::from(source_row.segment_width)
                });
            if !single_overwide_table {
                return Err(GeometryError::Unsupported("TAC row exceeds stored width"));
            }
            free = 0.0;
        }
        let offset = f64::from(source_row.column_start)
            + match alignment {
                Alignment::Left => 0.0,
                // Non-final mixed rows can distribute word spaces. Their
                // justification result is not supplied by this saved-box query.
                Alignment::Justify
                    if row.tables.is_empty()
                        || (row.tables.len() == 1
                            && (spaces.is_empty()
                                || row.source_line + 1 == para.line_segs.len())) =>
                {
                    0.0
                }
                Alignment::Center => free / 2.0,
                Alignment::Right => free,
                _ => return Err(GeometryError::Unsupported("TAC paragraph alignment")),
            };
        for (_, r) in &mut row.tables {
            r.x += offset;
        }
        for (si, space) in spaces.iter().enumerate().filter(|(_, s)| {
            s.position >= para.line_seg_text_start(row.source_line) && s.position < stop
        }) {
            let before_tables: f64 = row
                .tables
                .iter()
                .filter(|(ci, _)| positions[*ci] < space.position)
                .map(|(ci, r)| {
                    let (_, m) = object_box(&para.controls[*ci], pictures).unwrap();
                    r.width + f64::from(m[0]) + f64::from(m[1])
                })
                .sum();
            let before_spaces: f64 = spaces
                .iter()
                .filter(|s| {
                    s.position >= para.line_seg_text_start(row.source_line)
                        && s.position < space.position
                })
                .map(|s| s.width_hu)
                .sum();
            row.spaces.push((
                si,
                Rect {
                    x: offset + before_tables + before_spaces,
                    y: 0.0,
                    width: space.width_hu,
                    height: row.height,
                },
            ));
        }
    }
    if rows.iter().map(|r| r.spaces.len()).sum::<usize>() != spaces.len() {
        return Err(GeometryError::Unsupported(
            "TAC spaces without occupied object row",
        ));
    }
    Ok(rows)
}

fn rows_stop(para: &Paragraph, line: usize) -> u32 {
    if line + 1 < para.line_segs.len() {
        para.line_seg_text_start(line + 1)
    } else {
        para.char_count
    }
}

fn object_box(
    ctrl: &Control,
    pictures: bool,
) -> Result<(&crate::model::shape::CommonObjAttr, [i32; 4]), GeometryError> {
    match ctrl {
        Control::Table(t) if !pictures => Ok((
            &t.common,
            [
                t.outer_margin_left.into(),
                t.outer_margin_right.into(),
                t.outer_margin_top.into(),
                t.outer_margin_bottom.into(),
            ],
        )),
        Control::Picture(p) if pictures => Ok((
            &p.common,
            [
                p.common.margin.left.into(),
                p.common.margin.right.into(),
                p.common.margin.top.into(),
                p.common.margin.bottom.into(),
            ],
        )),
        _ => Err(unsupported()),
    }
}

pub(super) fn same(a: f64, b: f64) -> bool {
    (a - b).abs() <= 32.0 * f64::EPSILON * a.abs().max(b.abs()).max(1.0)
}

pub(super) fn carrier_style<'a>(
    para: &Paragraph,
    styles: &'a ResolvedStyleSet,
) -> Result<&'a crate::renderer::style_resolver::ResolvedParaStyle, GeometryError> {
    let style = styles
        .para_styles
        .get(para.para_shape_id as usize)
        .ok_or_else(unsupported)?;
    // A reference is not itself a painted paragraph decoration. Match the
    // text adapter's resolved-style contract; source effects are checked by
    // validate_paragraph_source before this shared table/picture boundary.
    if !super::decoration::paragraph_is_unpainted(style.border_fill_id, styles)
        || style.head_type != crate::model::style::HeadType::None
        || style.keep_lines
        || style.keep_with_next
        || style.widow_orphan
        || style.page_break_before
    {
        return Err(GeometryError::Unsupported(
            "TAC carrier paragraph constraints",
        ));
    }
    super::contracts::nonnegative(style.spacing_before, "TAC spacing before")?;
    super::contracts::nonnegative(style.spacing_after, "TAC spacing after")?;
    Ok(style)
}

fn physical_frame(
    para: &Paragraph,
    width: f64,
    style: &crate::renderer::style_resolver::ResolvedParaStyle,
    dpi: f64,
) -> Result<Paragraph, GeometryError> {
    // Margins already belong to saved cs/sw. Only the stored indentation flag
    // requests an additional inset; never apply hanging indentation to row 0 by guess.
    let scale = dpi / 7200.0;
    let mut local = para.clone();
    for row in &mut local.line_segs {
        let left = f64::from(row.column_start) * scale;
        let right = left + f64::from(row.segment_width) * scale;
        if !style.margin_left.is_finite()
            || !style.margin_right.is_finite()
            || !style.indent.is_finite()
            || style.margin_left < 0.0
            || style.margin_right < 0.0
            || (left < style.margin_left && !same(left, style.margin_left))
            || (right > width - style.margin_right && !same(right, width - style.margin_right))
        {
            return Err(unsupported());
        }
        if row.has_indentation() {
            let inset = style.indent.abs() / scale;
            if inset > f64::from(i32::MAX)
                || !same(inset, inset.round())
                || inset >= f64::from(row.segment_width)
            {
                return Err(unsupported());
            }
            row.column_start = row
                .column_start
                .checked_add(inset.round() as i32)
                .ok_or_else(unsupported)?;
            row.segment_width -= inset.round() as i32;
        }
    }
    Ok(local)
}

pub(super) fn compose(
    para: &Paragraph,
    width: f64,
    styles: &ResolvedStyleSet,
    dpi: f64,
) -> Result<(Vec<ParagraphItem>, Vec<RenderNode>), GeometryError> {
    let style = carrier_style(para, styles)?;
    let scale = dpi / 7200.0;
    let local = physical_frame(para, width, style, dpi)?;
    let spaces = super::tac_spaces::compose(para, styles, dpi)?;
    let rows = object_rows(&local, width / scale, style.alignment, false, &spaces)?;
    let mut nodes = Vec::new();
    let mut items = vec![ParagraphItem::Space(style.spacing_before)];
    let mut end = 0.0;
    for (i, row) in rows.iter().enumerate() {
        // Source positions choose row origins; occupied envelopes remain intact.
        // Spacing affects the next origin, never the child's physical fit box.
        let delta = rows
            .get(i + 1)
            .map_or(row.height + row.spacing, |next| next.top - row.top);
        if !delta.is_finite() || delta <= 0.0 {
            return Err(GeometryError::Unsupported("non-forward TAC row advance"));
        }
        let advance = delta.min(row.height);
        if row.top < end {
            return Err(unsupported());
        }
        if row.top > end {
            items.push(ParagraphItem::Space((row.top - end) * scale));
        }
        end = row.top + advance;
        if row.tables.is_empty() && row.spaces.is_empty() {
            if advance != row.height {
                return Err(GeometryError::Unsupported("overlapping empty TAC row"));
            }
            items.push(ParagraphItem::Space(row.height * scale));
        } else {
            let mut lines = Vec::new();
            if !row.spaces.is_empty() {
                let source = &local.line_segs[row.source_line];
                let bounds = Rect {
                    x: f64::from(source.column_start) * scale,
                    y: 0.0,
                    width: f64::from(source.segment_width) * scale,
                    height: row.height * scale,
                };
                let mut node = RenderNode::new(
                    0,
                    RenderNodeType::TextLine(TextLineNode::new(
                        bounds.height,
                        f64::from(source.baseline_distance) * scale,
                    )),
                    BoundingBox::new(bounds.x, 0.0, bounds.width, bounds.height),
                );
                for (si, r) in &row.spaces {
                    let mut child = spaces[*si].node.clone();
                    if child.bbox.height > bounds.height {
                        return Err(unsupported());
                    }
                    child.bbox.x = r.x * scale;
                    child.bbox.y = 0.0;
                    // A no-ink space owns its advance across this stored line's
                    // ascent/descent, not a font-height box with a baseline
                    // outside it when an inline object raises the line.
                    child.bbox.height = bounds.height;
                    if let RenderNodeType::TextRun(run) = &mut child.node_type {
                        run.baseline = f64::from(source.baseline_distance) * scale;
                    }
                    node.children.push(child);
                }
                lines.push((nodes.len(), bounds));
                nodes.push(node);
            }
            if row.tables.is_empty() {
                // A no-ink source row is still an owned, indivisible line.
                // Its saved box and advance must survive before the TAC row,
                // including when the following table defers to another page.
                items.push(ParagraphItem::Lines {
                    height: row.height * scale,
                    advance: advance * scale,
                    lines,
                });
                continue;
            }
            items.push(ParagraphItem::InlineTables {
                // Logical row height controls the next origin. Reserve the
                // actual child bottom too; negative bottom margins must not
                // shrink the physical fit budget. Use placement arithmetic.
                height: row.tables.iter().fold(row.height * scale, |end, (_, r)| {
                    end.max(r.y * scale + r.height * scale)
                }),
                advance: advance * scale,
                lines,
                tables: row
                    .tables
                    .iter()
                    .map(|(ci, r)| {
                        (
                            *ci,
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
    // Negative spacing is already owned by the final inline row's advance.
    // Positive trailing whitespace remains splittable physical space.
    let mut tail = Vec::new();
    if trailing > 0 {
        tail.push(f64::from(trailing) * scale);
    }
    tail.push(style.spacing_after);
    let ending = super::ParagraphEnd::from_composed(&items, tail, style.spacing_after)?;
    items.push(ParagraphItem::End(ending));
    Ok((items, nodes))
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
