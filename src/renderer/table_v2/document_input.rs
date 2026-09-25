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
    renderer::{
        page_layout::PageLayoutInfo, render_tree::RenderNodeType,
        style_resolver::resolve_styles_for_document,
    },
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
        // spacing locates a border, not the body, and ID0 paints nothing. Keep
        // the original records; any nonzero reference still needs a renderer.
        || std::iter::once(&def.page_border_fill)
            .chain(&def.extra_page_border_fills)
            .any(|fill| fill.border_fill_id != 0)
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
    let styles = resolve_styles_for_document(document, dpi);
    super::text::validate_text_context(&styles, dpi)?;
    let composer = TextComposer {
        styles: &styles,
        dpi,
        payloads: RefCell::new(Vec::new()),
    };
    let mut blocks = Vec::new();
    let mut lines = HashMap::new();
    let mut tables = HashMap::new();
    let mut order = 0;
    let mut page_number = None;
    for (pi, source) in section.paragraphs.iter().enumerate() {
        let fail = |reason| DocumentV2Error::Paragraph { index: pi, reason };
        super::decoration::validate_paragraph_source(source.para_shape_id, &document.doc_info)
            .map_err(fail)?;
        if source.column_type != ColumnBreakType::None
            && !(pi == 0 && source.column_type == ColumnBreakType::Section)
        {
            return Err(fail(GeometryError::Unsupported(
                "explicit body page/column break",
            )));
        }
        // Validate structural declarations before dispatch without removing
        // their source slots. Qualified page numbers paint outside body flow.
        let mut section_seen = false;
        let mut column_seen = false;
        let mut table_count = 0;
        for control in &source.controls {
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
                Control::Table(_) => table_count += 1,
                Control::PageNumberPos(value) if pi == 0 && page_number.is_none() => {
                    page_number = Some(
                        super::page_number::PageNumberStory::new(value, def, &layout)
                            .map_err(fail)?,
                    );
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
                _ => false,
            })
        {
            for item in super::paragraph_end::into_flow_items(
                super::tac::compose(source, body.width, &styles, dpi).map_err(fail)?,
            ) {
                match item {
                    ParagraphItem::Space(h) => blocks.push(FlowBlock::Space(h)),
                    ParagraphItem::InlineTables {
                        height,
                        advance,
                        tables: owned,
                    } => {
                        let mut bound = Vec::new();
                        for (ci, rect) in owned {
                            let Control::Table(table) = &source.controls[ci] else {
                                unreachable!()
                            };
                            super::decoration::validate_source(table, &document.doc_info)
                                .map_err(fail)?;
                            let prepared = PreparedTextTable::prepare_with_end_policy(
                                table,
                                &styles,
                                dpi,
                                &document.bin_data_content,
                                cell_end_policy,
                            )
                            .map_err(fail)?;
                            let owner = ControlOwner {
                                paragraph: pi,
                                control: ci,
                            };
                            bound.push(super::tac::bind(owner, rect, prepared.plan).map_err(fail)?);
                            tables.insert(owner, (order, prepared.paint));
                            order += 1;
                        }
                        blocks.push(FlowBlock::InlineTables {
                            height,
                            advance,
                            tables: bound,
                        });
                    }
                    _ => return Err(fail(GeometryError::InconsistentAtomicPlan)),
                }
            }
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
        if let Some((ci, table)) = table_control {
            if !source.line_segs.is_empty() {
                return Err(fail(GeometryError::Unsupported(
                    "stored body anchor ownership",
                )));
            }
            super::ir::validate_anchor(table).map_err(fail)?;
            let style = styles
                .para_styles
                .get(source.para_shape_id as usize)
                .ok_or_else(|| fail(GeometryError::Unsupported("missing paragraph style")))?;
            if [
                style.margin_left,
                style.margin_right,
                style.indent,
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
            tables.insert(owner, (order, prepared.paint));
            blocks.push(FlowBlock::Table {
                owner,
                offset_x,
                plan: prepared.plan,
            });
            order += 1;
        }
        // Structural controls do not occupy a line. Qualified TopAndBottom at
        // paragraph-top excludes the width: the host line follows its table.
        // Keep all text, offsets, char shapes and saved rows (the latter reject).
        let mut paragraph = source.clone();
        paragraph.controls.clear();
        paragraph.ctrl_data_records.clear();
        paragraph.column_type = ColumnBreakType::None;
        let items = composer.compose(&paragraph, body.width).map_err(fail)?;
        for item in super::paragraph_end::into_flow_items(items) {
            match item {
                ParagraphItem::Space(h) => blocks.push(FlowBlock::Space(h)),
                ParagraphItem::End(_) => unreachable!("paragraph end already lowered"),
                ParagraphItem::Lines {
                    height,
                    advance,
                    lines: owned,
                } => blocks.push(FlowBlock::Lines {
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
                }),
                ParagraphItem::ObjectRow { .. }
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
    }
    Ok(BodyPlan {
        flow: FlowCellInput {
            padding: Insets::default(),
            minimum_height: 0.0,
            width: body.width,
            blocks,
        },
        lines,
        tables,
        page_number,
        body,
        page_width: layout.page_width,
        page_height: layout.page_height,
    })
}
