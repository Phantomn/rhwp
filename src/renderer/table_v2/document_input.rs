//! Source qualification and body composition. Document state is not mutated;
//! stored rows, unsupported anchors and stories are never erased to admit input.
use std::{cell::RefCell, collections::HashMap};

use crate::{
    model::{
        control::Control,
        document::Document,
        page::{BindingMethod, ColumnDef},
        paragraph::ColumnBreakType,
        shape::HorzAlign,
    },
    renderer::{page_layout::PageLayoutInfo, render_tree::RenderNodeType},
};

use super::{
    document::BodyPlan, text::TextComposer, CellParagraphComposer, ControlOwner, DocumentV2Error,
    FlowBlock, FlowCellInput, GeometryError, Insets, LineBox, LineOwner, ParagraphItem,
    PreparedTextTable, Rect,
};

pub(super) fn prepare(
    document: &Document,
    dpi: f64,
    cell_end_policy: super::CellEndPolicy,
) -> Result<BodyPlan, DocumentV2Error> {
    if document.sections.len() != 1 {
        return Err(DocumentV2Error::Unsupported("one section required"));
    }
    let section = &document.sections[0];
    let def = &section.section_def;
    if def.flags != 0
        || def.line_grid != 0
        || def.char_grid != 0
        || def.text_direction != 0
        || def.hide_empty_line
        || !def.master_pages.is_empty()
        // Main/odd/even records can exist without naming a decoration. Their
        // spacing locates a border, not the body. A reference can also name an
        // unpainted style, as normal Hancom saves do. Keep all source records;
        // missing references and visible effects still need a renderer.
        || std::iter::once(&def.page_border_fill)
            .chain(&def.extra_page_border_fills)
            .any(|fill| {
                fill.border_fill_id != 0
                    && !document
                        .doc_info
                        .border_fills
                        .get(usize::from(fill.border_fill_id) - 1)
                        .is_some_and(super::decoration::source_border_is_unpainted)
            })
    {
        return Err(DocumentV2Error::Unsupported(
            "section decoration, grid or writing direction",
        ));
    }
    let page = &def.page_def;
    let (w, h) = if page.landscape {
        (page.height, page.width)
    } else {
        (page.width, page.height)
    };
    // Reject malformed page dimensions before the common page calculator can
    // apply its Legacy recovery fallback. Uniform pages only in this boundary.
    if w == 0
        || h == 0
        || w > i32::MAX as u32
        || h > i32::MAX as u32
        || u64::from(page.margin_left) + u64::from(page.margin_right) >= u64::from(w)
        || u64::from(page.margin_top)
            + u64::from(page.margin_header)
            + u64::from(page.margin_bottom)
            + u64::from(page.margin_footer)
            >= u64::from(h)
        || page.margin_gutter != 0
        || page.binding != BindingMethod::SingleSided
        || page.pagination_bottom_tolerance != 0
    {
        return Err(DocumentV2Error::Unsupported(
            "invalid or non-uniform page geometry",
        ));
    }
    let layout = PageLayoutInfo::from_page_def(page, &ColumnDef::default(), dpi);
    let area = layout.body_area;
    let body = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: area.height,
    };
    for v in [
        layout.page_width,
        layout.page_height,
        body.x,
        body.y,
        body.width,
        body.height,
    ] {
        super::contracts::nonnegative(v, "document page geometry")?;
    }
    let styles = super::source_units::resolve(document, dpi)?;
    super::text::validate_text_context(&styles, dpi)?;
    let composer = TextComposer {
        styles: &styles,
        dpi,
        payloads: RefCell::new(Vec::new()),
    };
    let mut blocks = Vec::new();
    let mut anchors = Vec::new();
    let mut page_breaks = Vec::new();
    let mut lines = HashMap::new();
    let mut tables = HashMap::new();
    let mut order = 0;
    let mut page_numbers = Vec::new();
    let body_ends = super::fields::body_ends(&section.paragraphs)
        .map_err(|(index, reason)| DocumentV2Error::Paragraph { index, reason })?;
    for (pi, source) in section.paragraphs.iter().enumerate() {
        let fail = |reason| DocumentV2Error::Paragraph { index: pi, reason };
        super::decoration::validate_paragraph_source(source.para_shape_id, &document.doc_info)
            .map_err(fail)?;
        match source.column_type {
            ColumnBreakType::None => {}
            ColumnBreakType::Section if pi == 0 => {}
            ColumnBreakType::Page if pi > 0 => {
                page_breaks.push((blocks.len(), anchors.len()));
            }
            _ => {
                return Err(fail(GeometryError::Unsupported(
                    "initial page break or body column/section break",
                )))
            }
        }
        // The body owns the transition; local paragraph/TAC composition owns
        // only its lines. Preserve source IR and every stored line metric.
        let mut local = source.clone();
        local.column_type = ColumnBreakType::None;
        let source = &local;
        // Validate structural declarations before dispatch without removing
        // their source slots. Qualified page numbers paint outside body flow.
        let mut section_seen = false;
        let mut column_seen = false;
        let mut table_count = 0;
        let tac_fields = super::fields::stored_tac_prefix(source);
        for (ci, control) in source.controls.iter().enumerate() {
            match control {
                Control::SectionDef(value)
                    if pi == 0 && !section_seen && !column_seen && table_count == 0 =>
                {
                    if serde_json::to_value(value).ok() != serde_json::to_value(def).ok() {
                        return Err(fail(GeometryError::Unsupported(
                            "different section definition",
                        )));
                    }
                    section_seen = true;
                }
                Control::ColumnDef(value)
                    if pi == 0
                        && !column_seen
                        && table_count == 0
                        && value.column_count <= 1
                        && value.widths.is_empty()
                        && value.gaps.is_empty()
                        && value.separator_type == 0 =>
                {
                    column_seen = true;
                }
                Control::Table(table) => {
                    table_count += 1;
                    super::page_number::collect_cell_stories(
                        table,
                        ControlOwner {
                            paragraph: pi,
                            control: ci,
                        },
                        def,
                        &layout,
                        &mut page_numbers,
                    )
                    .map_err(fail)?;
                }
                Control::Field(_) if tac_fields => {}
                Control::PageNumberPos(value) => {
                    let story = super::page_number::PageNumberStory::new(value, def, &layout)
                        .map_err(fail)?;
                    // Only a declaration at paragraph entry is qualified.
                    // Section/column slots may precede it; visible text or an
                    // earlier object requires a separate intra-paragraph rule.
                    super::page_number::validate_body_entry(source, ci).map_err(fail)?;
                    page_numbers.push((super::page_number::PageNumberHost::body(pi), story));
                }
                _ => {
                    return Err(fail(GeometryError::Unsupported(
                        "body control or multiple anchors",
                    )))
                }
            }
        }
        if table_count > 0
            && source.controls.iter().all(|c| match c {
                Control::Table(t) => t.common.treat_as_char,
                Control::SectionDef(_) | Control::ColumnDef(_) | Control::PageNumberPos(_) => true,
                Control::Field(_) => tac_fields,
                _ => false,
            })
        {
            let inline = super::body_inline::prepare(
                document,
                source,
                pi,
                body.width,
                &styles,
                dpi,
                cell_end_policy,
                section
                    .paragraphs
                    .get(pi + 1)
                    .is_none_or(|next| next.column_type == ColumnBreakType::Page),
            )
            .map_err(fail)?;
            let count = inline.lines.len() + inline.tables.len();
            blocks.extend(inline.blocks);
            lines.extend(
                inline
                    .lines
                    .into_iter()
                    .map(|(k, (n, v))| (k, (order + n, v))),
            );
            tables.extend(
                inline
                    .tables
                    .into_iter()
                    .map(|(k, (n, v))| (k, (order + n, v))),
            );
            order += count;
            continue;
        }
        // A saved exclusion host is a real zero-width line sharing the table
        // origin, not plain text before it. Bind the same composition and flow
        // block as cell-internal exclusions; this preserves the host once when
        // the child fragments, and the next authored blank as a separate line.
        if super::cell_anchor::candidate(source) {
            let composed = super::body_excluded::prepare(
                document,
                source,
                pi,
                body.width,
                layout.page_width - body.x,
                &styles,
                dpi,
                cell_end_policy,
            )
            .map_err(fail)?;
            blocks.extend(composed.blocks);
            lines.extend(
                composed
                    .lines
                    .into_iter()
                    .map(|(k, (n, v))| (k, (order + n, v))),
            );
            tables.extend(
                composed
                    .tables
                    .into_iter()
                    .map(|(k, (n, v))| (k, (order + n, v))),
            );
            order += 2;
            continue;
        }
        let mut table_control = None;
        for (ci, control) in source.controls.iter().enumerate() {
            match control {
                Control::SectionDef(_) | Control::ColumnDef(_) | Control::PageNumberPos(_) => {}
                Control::Table(table) if table_control.is_none() => {
                    table_control = Some((ci, table));
                }
                _ => {
                    return Err(fail(GeometryError::Unsupported(
                        "body control or multiple anchors",
                    )))
                }
            }
        }
        let mut deferred = None;
        if let Some((ci, table)) = table_control {
            let stored = !source.line_segs.is_empty();
            if !stored {
                super::ir::validate_anchor(table).map_err(fail)?;
            }
            let style = styles
                .para_styles
                .get(source.para_shape_id as usize)
                .ok_or_else(|| fail(GeometryError::Unsupported("missing paragraph style")))?;
            if [
                style.margin_left,
                style.margin_right,
                // Stored line composition already applies indentation to the
                // flagged text rows. It does not move the paragraph anchor.
                if stored { 0.0 } else { style.indent },
                style.spacing_before,
            ]
            .iter()
            .any(|v| *v != 0.0)
            {
                return Err(fail(GeometryError::Unsupported(
                    "anchored host paragraph insets",
                )));
            }
            super::decoration::validate_source(table, &document.doc_info).map_err(fail)?;
            let prepared = PreparedTextTable::prepare_with_end_policy(
                table,
                &styles,
                dpi,
                &document.bin_data_content,
                cell_end_policy,
            )
            .map_err(fail)?;
            let free = body.width - prepared.plan.width;
            if free < 0.0 {
                return Err(fail(GeometryError::Unsupported(
                    "table wider than document body",
                )));
            }
            let offset_x = match table.common.horz_align {
                HorzAlign::Left => 0.0,
                HorzAlign::Center => free / 2.0,
                HorzAlign::Right => free,
                _ => return Err(fail(GeometryError::InconsistentAtomicPlan)),
            };
            let owner = ControlOwner {
                paragraph: pi,
                control: ci,
            };
            if stored {
                deferred = Some((owner, table, prepared));
            } else {
                tables.insert(owner, (order, prepared.paint));
                blocks.push(FlowBlock::Table {
                    owner,
                    offset_x,
                    restart_top: 0.0,
                    plan: prepared.plan,
                });
                order += 1;
            }
        }
        // Structural controls do not occupy a line. Qualified TopAndBottom at
        // paragraph-top excludes the width: the host line follows its table.
        // Stored hosts keep their composed rows: a Square anchor may share
        // their vertical band using a disjoint side lane. No stored metric is
        // cleared; final placed exclusions are checked before page commit.
        let frame_starts = super::body_text::frame_starts(source).map_err(fail)?;
        if !frame_starts.is_empty() && deferred.is_some() {
            return Err(fail(GeometryError::Unsupported(
                "body anchor across stored frames",
            )));
        }
        let mut paragraph = source.clone();
        paragraph.controls.clear();
        paragraph.ctrl_data_records.clear();
        paragraph.column_type = ColumnBreakType::None;
        if !frame_starts.is_empty() {
            paragraph = super::stored_text::continuous_paragraph(&paragraph, &frame_starts)
                .map_err(fail)?;
        }
        let items = composer
            .compose_text_with_body_end(&paragraph, body.width, false, body_ends[pi].as_ref())
            .map_err(fail)?;
        let anchored = if let Some((owner, table, prepared)) = deferred {
            let Some(ParagraphItem::End(end)) = items.last() else {
                return Err(fail(GeometryError::InconsistentAtomicPlan));
            };
            let anchor = super::body_anchor::BodyAnchor::resolve(
                table,
                end,
                prepared.plan.width,
                body.width,
                layout.page_width - body.x,
                dpi,
            )
            .map_err(fail)?;
            Some((owner, table, prepared, anchor))
        } else {
            None
        };
        for item in items {
            match item {
                ParagraphItem::Space(h) => blocks.push(FlowBlock::Space(h)),
                ParagraphItem::End(end) => blocks.extend(end.into_body_tail()),
                ParagraphItem::Lines {
                    height,
                    advance,
                    lines: owned,
                } => {
                    if owned
                        .iter()
                        .skip(1)
                        .any(|(li, _)| frame_starts.contains(li))
                    {
                        return Err(fail(GeometryError::Unsupported(
                            "stored body frame inside atomic paragraph",
                        )));
                    }
                    if owned
                        .first()
                        .is_some_and(|(li, _)| frame_starts.contains(li))
                    {
                        super::body_text::end_frame(&mut blocks);
                        page_breaks.push((blocks.len(), anchors.len()));
                    }
                    blocks.push(FlowBlock::Lines {
                        height,
                        advance,
                        lines: owned
                            .into_iter()
                            .map(|(li, bounds)| LineBox {
                                owner: LineOwner {
                                    paragraph: pi,
                                    line: li,
                                },
                                bounds,
                            })
                            .collect(),
                    });
                }
                ParagraphItem::PositionedTable { .. }
                | ParagraphItem::ExcludedTable { .. }
                | ParagraphItem::ObjectRow { .. }
                | ParagraphItem::TableControl(_)
                | ParagraphItem::InlineTables { .. } => {
                    return Err(fail(GeometryError::InconsistentAtomicPlan))
                }
            }
        }
        let payloads = composer
            .payloads
            .borrow_mut()
            .pop()
            .ok_or_else(|| fail(GeometryError::InconsistentAtomicPlan))?;
        for (li, mut node) in payloads.into_iter().enumerate() {
            if let RenderNodeType::TextLine(value) = &mut node.node_type {
                value.section_index = Some(0);
                value.para_index = Some(pi);
            }
            for child in &mut node.children {
                if let RenderNodeType::TextRun(value) = &mut child.node_type {
                    value.section_index = Some(0);
                    value.para_index = Some(pi);
                }
            }
            lines.insert(
                LineOwner {
                    paragraph: pi,
                    line: li,
                },
                (order, node),
            );
            order += 1;
        }
        if let Some((owner, table, prepared, anchor)) = anchored {
            anchors.push(anchor.flow(
                document,
                table,
                owner,
                prepared.plan.clone(),
                blocks.len(),
                body.width,
                layout.page_width - body.x,
                dpi,
            ));
            tables.insert(owner, (order, prepared.paint));
            order += 1;
        }
    }
    Ok(BodyPlan {
        anchors,
        page_breaks,
        flow: FlowCellInput {
            padding: Insets::default(),
            minimum_height: 0.0,
            width: body.width,
            blocks,
        },
        lines,
        tables,
        page_numbers,
        body,
        page_width: layout.page_width,
        page_height: layout.page_height,
        first_page_number: u32::from(def.page_num).max(1),
    })
}
