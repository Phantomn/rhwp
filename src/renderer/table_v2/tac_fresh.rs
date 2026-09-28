//! Fresh control/space rows. This produces placement ownership directly, never
//! fabricates saved LineSegs or invokes the Legacy table measurer.
use super::{GeometryError, ParagraphItem, Rect};
use crate::{
    model::{
        control::Control,
        paragraph::Paragraph,
        style::{Alignment, LineSpacingType},
    },
    renderer::{
        composer::layout_paragraph_in_physical_frame,
        layout_frame::ParagraphBox,
        render_tree::{BoundingBox, RenderNode, RenderNodeType, TextLineNode},
        style_resolver::ResolvedStyleSet,
    },
};

fn unsupported() -> GeometryError {
    GeometryError::Unsupported("fresh TAC requires qualified control/space rows")
}

enum Token {
    Table(usize, Rect, f64, f64),
    Space(usize),
    Break,
    Structure,
}

pub(super) fn compose(
    para: &Paragraph,
    width: f64,
    styles: &ResolvedStyleSet,
    dpi: f64,
) -> Result<(Vec<ParagraphItem>, Vec<RenderNode>), GeometryError> {
    compose_with_dimensions(para, width, styles, dpi, &[])
}

/// Body callers prepare children once, before composing their host rows. These
/// physical dimensions are consumed unchanged by both line occupancy and paint.
/// The cell adapter still supplies declared boxes and checks them when binding;
/// this entry point does not relax that stored/nested ownership contract.
pub(super) fn compose_with_dimensions(
    para: &Paragraph,
    width: f64,
    styles: &ResolvedStyleSet,
    dpi: f64,
    dimensions: &[(usize, f64, f64)],
) -> Result<(Vec<ParagraphItem>, Vec<RenderNode>), GeometryError> {
    let style = super::tac::carrier_style(para, styles)?;
    if style.vertical_alignment
        != crate::renderer::style_resolver::ParagraphVerticalAlignment::Baseline
    {
        return Err(GeometryError::Unsupported(
            "TAC paragraph vertical alignment",
        ));
    }
    if !para.line_segs.is_empty()
        || para.source_line_seg_vertical_pos.is_some()
        || para.layout_only_fill_lines != 0
        || para.hwpx_axis_shift != 0
        || !para.field_ranges.is_empty()
        || !para.range_tags.is_empty()
        || !para.orphan_field_ends.is_empty()
        || !para.title_marks.is_empty()
        || !para.markpen_marks.is_empty()
        || style.indent != 0.0
        || !matches!(
            style.alignment,
            Alignment::Left | Alignment::Center | Alignment::Right | Alignment::Justify
        )
        || para.text.chars().any(|c| c != ' ' && c != '\n')
        || para.text.chars().count() != para.char_offsets.len()
    {
        return Err(unsupported());
    }
    for v in [width, style.margin_left, style.margin_right] {
        super::contracts::nonnegative(v, "fresh TAC width/inset")?;
    }
    let lane = width - style.margin_left - style.margin_right;
    if lane <= 0.0 || !lane.is_finite() {
        return Err(unsupported());
    }
    let scale = dpi / 7200.0;
    let font = para
        .char_shapes
        .first()
        .and_then(|r| styles.char_styles.get(r.char_shape_id as usize))
        .ok_or_else(unsupported)?
        .font_size;
    if !font.is_finite()
        || font <= 0.0
        || para.char_shapes.iter().any(|r| {
            styles
                .char_styles
                .get(r.char_shape_id as usize)
                .is_none_or(|s| s.font_size != font)
        })
    {
        return Err(unsupported());
    }
    // A normal empty text line supplies the shared font/paragraph spacing
    // metrics. It is not emitted and has no invented table or saved cache.
    let mut strut = para.clone();
    strut.text.clear();
    strut.char_offsets.clear();
    strut.controls.clear();
    strut.char_count = 1;
    strut.char_shapes.truncate(1);
    strut.char_shapes[0].start_pos = 0;
    let mut frame = ParagraphBox::content(0..(lane / scale).floor() as i32).frame(0);
    let metrics = layout_paragraph_in_physical_frame(&strut, &mut frame, styles, dpi)
        .ok_or_else(unsupported)?;
    if metrics.len() != 1 {
        return Err(unsupported());
    }
    let strut_height = f64::from(metrics[0].line_height) * scale;
    let strut_gap = f64::from(metrics[0].line_spacing) * scale;

    let mut space_para = para.clone();
    space_para.text = para.text.chars().filter(|c| *c == ' ').collect();
    space_para.char_offsets = para
        .text
        .chars()
        .zip(&para.char_offsets)
        .filter_map(|(c, p)| (c == ' ').then_some(*p))
        .collect();
    let spaces = super::tac_spaces::compose(&space_para, styles, dpi)?;
    let positions = para.control_utf16_positions();
    if positions.len() != para.controls.len() {
        return Err(unsupported());
    }
    let mut tokens = Vec::new();
    for (ci, c) in para.controls.iter().enumerate() {
        let token = match c {
            Control::SectionDef(_) | Control::ColumnDef(_) | Control::PageNumberPos(_) => {
                Token::Structure
            }
            Control::Table(t) => {
                let a = &t.common;
                let m = [
                    t.outer_margin_left,
                    t.outer_margin_right,
                    t.outer_margin_top,
                    t.outer_margin_bottom,
                ];
                if !a.treat_as_char
                    || a.width == 0
                    || a.height == 0
                    || a.vertical_offset != 0
                    || a.horizontal_offset != 0
                    || a.prevent_page_break != 0
                    || a.affect_line_spacing
                    || m.iter().any(|v| *v < 0)
                {
                    return Err(unsupported());
                }
                let (table_width, table_height) =
                    dimensions.iter().find(|(index, _, _)| *index == ci).map_or(
                        (f64::from(a.width) * scale, f64::from(a.height) * scale),
                        |&(_, w, h)| (w, h),
                    );
                super::contracts::nonnegative(table_width, "fresh TAC table width")?;
                super::contracts::nonnegative(table_height, "fresh TAC table height")?;
                let height = table_height + (f64::from(m[2]) + f64::from(m[3])) * scale;
                if height < strut_height {
                    return Err(unsupported());
                }
                Token::Table(
                    ci,
                    Rect {
                        x: f64::from(m[0]) * scale,
                        y: f64::from(m[2]) * scale,
                        width: table_width,
                        height: table_height,
                    },
                    table_width + (f64::from(m[0]) + f64::from(m[1])) * scale,
                    height,
                )
            }
            _ => return Err(unsupported()),
        };
        tokens.push((positions[ci], 8, token));
    }
    tokens.extend(
        spaces
            .iter()
            .enumerate()
            .map(|(i, s)| (s.position, 1, Token::Space(i))),
    );
    tokens.extend(
        para.text
            .chars()
            .zip(&para.char_offsets)
            .filter_map(|(c, p)| (c == '\n').then_some((*p, 1, Token::Break))),
    );
    tokens.sort_by_key(|t| t.0);
    let mut end = 0;
    for (p, n, _) in &tokens {
        if *p != end {
            return Err(unsupported());
        }
        end = end.checked_add(*n).ok_or_else(unsupported)?;
    }
    if end.checked_add(1) != Some(para.char_count) {
        return Err(unsupported());
    }
    let mut items = vec![ParagraphItem::Space(style.spacing_before)];
    let mut nodes = Vec::new();
    let mut row = Vec::new();
    let mut used = 0.0;
    let mut last_gap = 0.0;
    let mut emit =
        |row: &mut Vec<(Token, f64)>, used: f64, final_row: bool| -> Result<(), GeometryError> {
            if !final_row
                && style.alignment == Alignment::Justify
                && row
                    .iter()
                    .any(|(token, _)| matches!(token, Token::Space(_)))
            {
                return Err(GeometryError::Unsupported("fresh TAC distributed spaces"));
            }
            let band =
                super::tac_metrics::TableBand::measure(row.iter().filter_map(|(token, _)| {
                    if let Token::Table(_, r, _, h) = token {
                        Some((r.height, r.y, h - r.height - r.y))
                    } else {
                        None
                    }
                }));
            let height = band
                .as_ref()
                .map_or(strut_height, |b| b.height.max(strut_height));
            let baseline = band.as_ref().map_or(strut_height * 0.85, |b| b.baseline);
            let gap = match style.line_spacing_type {
                LineSpacingType::Percent | LineSpacingType::SpaceOnly => strut_gap,
                LineSpacingType::Fixed | LineSpacingType::Minimum => {
                    (style.line_spacing - height).max(0.0)
                }
            };
            if height + gap <= 0.0 {
                return Err(unsupported());
            }
            let x = style.margin_left
                + match style.alignment {
                    Alignment::Center => (lane - used) / 2.0,
                    Alignment::Right => lane - used,
                    _ => 0.0,
                };
            let mut node = RenderNode::new(
                0,
                RenderNodeType::TextLine(TextLineNode::new(height, baseline)),
                BoundingBox::new(style.margin_left, 0.0, lane, height),
            );
            let mut tables = Vec::new();
            for (token, pen) in row.drain(..) {
                match token {
                    Token::Table(ci, mut r, _, _) => {
                        r.x += x + pen;
                        r.y = band.as_ref().unwrap().top(r.height);
                        tables.push((ci, r));
                    }
                    Token::Space(si) => {
                        let mut child = spaces[si].node.clone();
                        child.bbox.x = x + pen;
                        child.bbox.y = 0.0;
                        child.bbox.height = height;
                        if let RenderNodeType::TextRun(run) = &mut child.node_type {
                            run.baseline = baseline;
                        }
                        node.children.push(child);
                    }
                    _ => unreachable!(),
                }
            }
            let mut lines = Vec::new();
            if tables.is_empty() || !node.children.is_empty() {
                lines.push((
                    nodes.len(),
                    Rect {
                        x: style.margin_left,
                        y: 0.0,
                        width: lane,
                        height,
                    },
                ));
                nodes.push(node);
            }
            let advance = height.min(height + gap);
            items.push(if tables.is_empty() {
                ParagraphItem::Lines {
                    height,
                    advance,
                    lines,
                }
            } else {
                ParagraphItem::InlineTables {
                    height,
                    advance,
                    tables,
                    lines,
                }
            });
            if gap > 0.0 {
                items.push(ParagraphItem::Space(gap));
            }
            last_gap = gap.max(0.0);
            Ok(())
        };
    for (_, _, token) in tokens {
        if matches!(token, Token::Structure) {
            continue;
        }
        if matches!(token, Token::Break) {
            emit(&mut row, used, false)?;
            used = 0.0;
            continue;
        }
        let width = match &token {
            Token::Table(_, _, w, _) => *w,
            Token::Space(si) => spaces[*si].width_hu * scale,
            _ => unreachable!(),
        };
        if width > lane {
            return Err(GeometryError::Unsupported("fresh TAC object exceeds lane"));
        }
        if !row.is_empty() && used + width > lane {
            emit(&mut row, used, false)?;
            used = 0.0;
        }
        row.push((token, used));
        used += width;
    }
    emit(&mut row, used, true)?;
    // Only the final gap belongs to ParagraphEnd. Intermediate row gaps are
    // explicit space, preserving the existing cut and ending-policy contract.
    if last_gap > 0.0 {
        items.pop();
    }
    let ending = super::ParagraphEnd::from_composed(
        &items,
        vec![last_gap, style.spacing_after],
        style.spacing_after,
    )?;
    items.push(ParagraphItem::End(ending));
    Ok((items, nodes))
}
