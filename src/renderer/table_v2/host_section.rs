//! Explicit immutable host integration session. Unlike the standalone BodyPlan,
//! column/page advancement and final painting go through the existing engines.
//! Fresh and qualified stored text share fit/paint; unsupported source properties
//! are errors, never erased or retried via Legacy. This is NOT a Studio switch.
use super::{
    CellEndPolicy, HostedParagraphPlan, HostedTableError, HostedTableSession, TableSelection,
};
use crate::{
    model::{
        control::Control,
        document::Document,
        page::{BindingMethod, ColumnDef, ColumnType},
        paragraph::{ColumnBreakType, Paragraph},
        shape::{HorzAlign, HorzRelTo, TextWrap, VertAlign, VertRelTo},
    },
    renderer::{
        layout::LayoutEngine,
        page_layout::PageLayoutInfo,
        pagination::PaginationResult,
        render_tree::{PageRenderTree, RenderNode},
        style_resolver::ResolvedStyleSet,
        typeset::TypesetEngine,
    },
};
use std::{collections::BTreeMap, sync::Arc};

pub struct HostedSectionSession {
    source: Arc<Document>,
    paragraphs: Vec<Paragraph>,
    dpi: f64,
    styles: ResolvedStyleSet,
    pagination: PaginationResult,
    page_stories: Vec<Option<RenderNode>>,
}

impl HostedSectionSession {
    pub fn from_document(
        source: &Document,
        section: usize,
        dpi: f64,
        policy: CellEndPolicy,
    ) -> Result<Self, HostedTableError> {
        let fail = HostedTableError::UnsupportedHost;
        let sec = source
            .sections
            .get(section)
            .ok_or_else(|| fail("section index"))?;
        let def = &sec.section_def;
        if !dpi.is_finite()
            || dpi <= 0.0
            || def.flags != 0
            || def.hide_empty_line
            || def.line_grid != 0
            || def.char_grid != 0
            || def.text_direction != 0
            || !def.master_pages.is_empty()
            || std::iter::once(&def.page_border_fill)
                .chain(&def.extra_page_border_fills)
                .any(|fill| {
                    fill.border_fill_id != 0
                        && !source
                            .doc_info
                            .border_fills
                            .get(usize::from(fill.border_fill_id) - 1)
                            .is_some_and(super::decoration::source_border_is_unpainted)
                })
        {
            return Err(fail("section decoration/grid requires host admission"));
        }
        if def.page_num > 1 || def.page_num_type != 0 {
            return Err(fail(
                "section page-number origin requires host numbering context",
            ));
        }
        let p = &def.page_def;
        let (w, h) = if p.landscape {
            (p.height, p.width)
        } else {
            (p.width, p.height)
        };
        if p.binding != BindingMethod::SingleSided
            || p.margin_gutter != 0
            || p.pagination_bottom_tolerance != 0
            || u64::from(w) <= u64::from(p.margin_left) + u64::from(p.margin_right)
            || u64::from(h)
                <= u64::from(p.margin_top)
                    + u64::from(p.margin_bottom)
                    + u64::from(p.margin_header)
                    + u64::from(p.margin_footer)
        {
            return Err(fail("invalid or nonuniform host page"));
        }
        let mut columns = ColumnDef {
            column_count: 1,
            same_width: true,
            ..Default::default()
        };
        let mut seen_columns = false;
        let mut tables = BTreeMap::new();
        let mut inline_paragraphs = std::collections::BTreeSet::new();
        let mut excluded_paragraphs = std::collections::BTreeSet::new();
        let mut positioned_paragraphs = BTreeMap::new();
        let mut stories = BTreeMap::new();
        let styles = super::source_units::resolve(source, dpi)?;
        for (pi, para) in sec.paragraphs.iter().enumerate() {
            let style = styles
                .para_styles
                .get(para.para_shape_id as usize)
                .ok_or_else(|| fail("missing host paragraph style"))?;
            if style.head_type != crate::model::style::HeadType::None || style.keep_with_next {
                return Err(fail("host heading/keep-next needs section orchestration"));
            }
            if !matches!(
                para.column_type,
                ColumnBreakType::None | ColumnBreakType::Page | ColumnBreakType::Column
            ) && !(pi == 0 && para.column_type == ColumnBreakType::Section)
            {
                return Err(fail("explicit host boundary needs shared host admission"));
            }
            let mut table_control = None;
            let inline = para
                .controls
                .iter()
                .any(|c| matches!(c, Control::Table(t) if t.common.treat_as_char));
            for (ci, control) in para.controls.iter().enumerate() {
                match control {
                    Control::SectionDef(_) if pi == 0 => {}
                    Control::ColumnDef(c) if pi == 0 && !seen_columns => {
                        columns = c.clone();
                        seen_columns = true;
                    }
                    Control::PageNumberPos(value) => {
                        super::page_number::validate_body_entry(para, ci)?;
                        if para.controls.iter().any(|c| {
                            matches!(c, Control::Table(t)
                            if t.common.treat_as_char || t.common.text_wrap != TextWrap::Square)
                        }) {
                            return Err(fail("body page-number declaration with table controls"));
                        }
                        let layout = PageLayoutInfo::from_page_def(p, &columns, dpi);
                        stories.insert(
                            pi,
                            super::page_number::PageNumberStory::new(value, def, &layout)?,
                        );
                    }
                    Control::Table(t) if inline && t.common.treat_as_char => {
                        if has_page_number_story(t) {
                            return Err(fail(
                                "inline table page-number story requires host timeline",
                            ));
                        }
                        inline_paragraphs.insert(pi);
                    }
                    Control::Table(t)
                        if !inline
                            && super::cell_anchor::candidate(para)
                            && t.common.flow_with_text =>
                    {
                        if has_page_number_story(t) {
                            return Err(fail(
                                "positioned table page-number story requires host timeline",
                            ));
                        }
                        excluded_paragraphs.insert(pi);
                    }
                    Control::Table(t)
                        if !inline
                            && table_control.is_none()
                            && !positioned_paragraphs.contains_key(&pi)
                            && !para.line_segs.is_empty()
                            && matches!(
                                t.common.text_wrap,
                                TextWrap::TopAndBottom | TextWrap::Square
                            )
                            && t.common.horz_rel_to == HorzRelTo::Para
                            && t.common.flow_with_text =>
                    {
                        if has_page_number_story(t) {
                            return Err(fail(
                                "positioned table page-number story requires host timeline",
                            ));
                        }
                        positioned_paragraphs.insert(pi, ci);
                    }
                    Control::Table(t)
                        if !inline
                            && table_control.is_none()
                            && !positioned_paragraphs.contains_key(&pi) =>
                    {
                        let c = &t.common;
                        if !para.text.is_empty()
                            || c.treat_as_char
                            || c.text_wrap != TextWrap::TopAndBottom
                            || c.vert_rel_to != VertRelTo::Para
                            || c.vert_align != VertAlign::Top
                            || c.horz_rel_to != HorzRelTo::Column
                            || c.horz_align != HorzAlign::Left
                            || c.vertical_offset != 0
                            || c.horizontal_offset != 0
                            || c.prevent_page_break != 0
                            || c.margin.left != 0
                            || c.margin.right != 0
                            || c.margin.top != 0
                            || c.margin.bottom != 0
                            || t.outer_margin_left != 0
                            || t.outer_margin_right != 0
                            || t.outer_margin_top != 0
                            || t.outer_margin_bottom != 0
                            || t.caption.is_some()
                        {
                            return Err(fail(
                                "table host requires resolved anchor/line/margin policy",
                            ));
                        }
                        table_control = Some(ci);
                    }
                    _ => return Err(fail("host control or multiple tables on one paragraph")),
                }
            }
            if let Some(control) = table_control {
                if [
                    style.margin_left,
                    style.margin_right,
                    style.indent,
                    style.spacing_before,
                ]
                .iter()
                .any(|v| *v != 0.0)
                {
                    return Err(fail(
                        "table host paragraph insets require anchor resolution",
                    ));
                }
                tables.insert(
                    pi,
                    HostedTableSession::from_document(
                        source,
                        TableSelection {
                            section,
                            paragraph: pi,
                            control,
                        },
                        dpi,
                        policy,
                    )?,
                );
            }
        }
        if columns.column_count == 0
            || columns.column_type != ColumnType::Normal
            || !columns.same_width
            || columns.direction == crate::model::page::ColumnDirection::Mirror
        {
            return Err(fail("balanced or unequal host columns"));
        }
        // Text-flow projection, not source normalization. All paragraph slots,
        // character styles and empty owner lines remain. Only a V2-owned table
        // control is excluded from text composition to avoid measuring
        // or painting that object twice. The original snapshot stays untouched.
        let mut flow = sec.clone();
        for pi in tables.keys() {
            flow.paragraphs[*pi]
                .controls
                .retain(|c| !matches!(c, Control::Table(_)));
        }
        let width = PageLayoutInfo::from_page_def(p, &columns, dpi).column_areas[0].width;
        let layout = PageLayoutInfo::from_page_def(p, &columns, dpi);
        // Page-break-before is consumed by run_section from the original styles.
        // A local line composition must not apply that host boundary a second
        // time (or reject it as a cell paragraph property). Neither Document IR
        // nor the styles passed to the section driver/renderer are changed.
        let mut text_styles = styles.clone();
        for style in &mut text_styles.para_styles {
            style.page_break_before = false;
        }
        let mut text = Vec::with_capacity(flow.paragraphs.len());
        for (pi, para) in flow.paragraphs.iter().enumerate() {
            if let Some(control) = positioned_paragraphs.get(&pi) {
                if columns.column_count != 1 {
                    return Err(fail(
                        "positioned table requires single-column anchor context",
                    ));
                }
                text.push(HostedParagraphPlan::prepare_positioned(
                    source,
                    para,
                    *control,
                    width,
                    layout.page_width - layout.column_areas[0].x,
                    &text_styles,
                    dpi,
                    policy,
                )?);
                continue;
            }
            if excluded_paragraphs.contains(&pi) {
                // compose_body qualifies paragraph/column anchors in one lane.
                // A multi-column host needs its selected physical lane at fit,
                // not the first lane's paper edge reused for every column.
                if columns.column_count != 1 {
                    return Err(fail(
                        "positioned table requires single-column anchor context",
                    ));
                }
                let mut carrier = para.clone();
                carrier.column_type = ColumnBreakType::None;
                text.push(HostedParagraphPlan::prepare_excluded(
                    source,
                    &carrier,
                    width,
                    layout.page_width - layout.column_areas[0].x,
                    &text_styles,
                    dpi,
                    policy,
                )?);
                continue;
            }
            let mut local = para.clone();
            // The already-qualified initial section/column controls are non-textual.
            // Retain saved offsets/line partitions and the authored empty line.
            local.controls.retain(|c| {
                !matches!(
                    c,
                    Control::ColumnDef(_) | Control::SectionDef(_) | Control::PageNumberPos(_)
                )
            });
            local.column_type = ColumnBreakType::None;
            if inline_paragraphs.contains(&pi) {
                // Keep the complete source control slots for the TAC character
                // axis. Only the section driver consumes the paragraph break.
                let mut carrier = para.clone();
                carrier.column_type = ColumnBreakType::None;
                let frame_end = sec.paragraphs.get(pi + 1).is_none_or(|next| {
                    matches!(
                        next.column_type,
                        ColumnBreakType::Page | ColumnBreakType::Column
                    ) || styles
                        .para_styles
                        .get(next.para_shape_id as usize)
                        .is_some_and(|s| s.page_break_before)
                });
                text.push(HostedParagraphPlan::prepare_inline(
                    source,
                    &carrier,
                    width,
                    &text_styles,
                    dpi,
                    policy,
                    frame_end,
                )?);
                continue;
            }
            if tables.contains_key(&pi) {
                if let Some(owner) =
                    HostedParagraphPlan::prepare_table_owner(&local, width, &text_styles, dpi)?
                {
                    text.push(owner);
                    continue;
                }
            }
            text.push(HostedParagraphPlan::prepare(
                &local,
                width,
                &text_styles,
                dpi,
                columns.column_count == 1,
            )?);
        }
        let mut pagination = TypesetEngine::new(dpi).typeset_hosted_section(
            &sec.paragraphs,
            text,
            &styles,
            p,
            &columns,
            section,
            tables,
        )?;
        // Validate all accepted geometry, including preceding and following
        // paragraphs/tables. A side box is not allowed to publish overlapping
        // stored lanes merely because its owner paragraph was safe.
        for page in &pagination.pages {
            let packets: Vec<_> = page
                .column_contents
                .iter()
                .flat_map(|c| &c.items)
                .filter_map(|item| match item {
                    crate::renderer::pagination::PageItem::HostedParagraph { fragment, .. } => {
                        Some((
                            fragment.flow_boxes().collect::<Vec<_>>(),
                            fragment.exclusion(),
                        ))
                    }
                    crate::renderer::pagination::PageItem::HostedTable { fragment, .. } => {
                        Some((vec![fragment.occupied()], None))
                    }
                    _ => None,
                })
                .collect();
            for (index, (_, exclusion)) in packets.iter().enumerate() {
                if let Some(exclusion) = exclusion {
                    super::body_flow::validate_exclusion(
                        *exclusion,
                        packets
                            .iter()
                            .enumerate()
                            .filter(|(other, _)| *other != index)
                            .flat_map(|(_, (boxes, _))| boxes.iter().copied()),
                    )?;
                }
            }
        }
        // The legacy section finalizer collects one global PageNumberPos. V2
        // instead activates declarations from committed owner lines, including
        // invisible blank lines, never from an attempted fit or a spacing-only
        // packet. Final page order and physical numbers remain host-owned.
        let mut active = None;
        let mut page_stories = Vec::with_capacity(pagination.pages.len());
        for (index, page) in pagination.pages.iter_mut().enumerate() {
            let accepted = page
                .column_contents
                .iter()
                .flat_map(|c| &c.items)
                .filter_map(|item| match item {
                    crate::renderer::pagination::PageItem::HostedParagraph {
                        para_index,
                        fragment,
                    } if fragment.has_body_line() && stories.contains_key(para_index) => {
                        Some(*para_index)
                    }
                    _ => None,
                })
                .max();
            active = active.max(accepted);
            page.page_number_pos = None;
            page_stories.push(match active {
                Some(pi) => stories[&pi]
                    .render(u32::try_from(index).map_err(|_| fail("page-number range"))?)?,
                None => None,
            });
        }
        Ok(Self {
            source: Arc::new(source.clone()),
            paragraphs: sec.paragraphs.clone(),
            dpi,
            styles,
            pagination,
            page_stories,
        })
    }

    pub fn pagination(&self) -> &PaginationResult {
        &self.pagination
    }

    /// Explicit preview export for both Native and fresh WASM. This preserves
    /// the actual host pagination/paint result, not a standalone body emulation.
    pub fn render_page_json(&self, index: usize) -> Result<String, HostedTableError> {
        let tree = self.render_page(index)?;
        let mut svg = crate::renderer::svg::SvgRenderer::new();
        svg.render_tree(&tree);
        serde_json::to_string(&serde_json::json!({
            "schema_version": 1, "engine": "table_v2_hosted_section",
            "page_index": index, "svg": svg.output(), "render_tree": tree,
        }))
        .map_err(|e| HostedTableError::Serialize(e.to_string()))
    }

    pub fn render_page(&self, index: usize) -> Result<PageRenderTree, HostedTableError> {
        let page = self
            .pagination
            .pages
            .get(index)
            .ok_or(HostedTableError::WrongDestination)?;
        let mut tree = LayoutEngine::new(self.dpi).build_render_tree(
            page,
            &self.paragraphs,
            &[],
            &[],
            &[],
            &self.styles,
            &Default::default(),
            &self.source.bin_data_content,
            None,
            &[],
            None,
            0,
            &[],
        );
        if let Some(mut story) = self.page_stories[index].clone() {
            super::text::assign_ids(&mut story, tree.frame_mut());
            tree.root.children.push(story);
        }
        Ok(tree)
    }
}

// Cell AutoNumber fields consume the physical number during packet paint.
// PageNumberPos instead activates a separate page story, not a cell glyph.
// That timeline is not yet bound for inline-host packets; do not drop it.
fn has_page_number_story(table: &crate::model::table::Table) -> bool {
    table
        .cells
        .iter()
        .flat_map(|c| &c.paragraphs)
        .flat_map(|p| &p.controls)
        .any(|c| match c {
            Control::PageNumberPos(_) => true,
            Control::Table(t) => has_page_number_story(t),
            _ => false,
        })
}
