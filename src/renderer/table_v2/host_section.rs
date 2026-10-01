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
    layout: HostedSectionLayout,
}

/// Prepared geometry only. The product host retains the authoritative IR;
/// preparing multiple sections must not clone the whole document per section.
pub(crate) struct HostedSectionLayout {
    section: usize,
    dpi: f64,
    styles: ResolvedStyleSet,
    pagination: PaginationResult,
    page_stories: Vec<Vec<RenderNode>>,
    // Page-owned absolute objects do not add their margin position to the
    // column's text pen. Ownership and final geometry are bound before export.
    page_tables: Vec<Vec<RenderNode>>,
}

impl HostedSectionSession {
    pub fn from_document(
        source: &Document,
        section: usize,
        dpi: f64,
        policy: CellEndPolicy,
    ) -> Result<Self, HostedTableError> {
        let first = source
            .sections
            .get(section)
            .map_or(1, |s| u32::from(s.section_def.page_num).max(1));
        let layout = HostedSectionLayout::prepare(
            source,
            section,
            dpi,
            policy,
            0,
            first,
            None,
            &mut super::host_stories::StoryContext::default(),
        )
        .map_err(|mut error| {
            // Preserve the established preview API's typed rejection contract.
            // Product preparation retains the new paragraph/section address.
            while let HostedTableError::Paragraph { cause, .. } = error {
                error = *cause;
            }
            error
        })?;
        Ok(Self {
            source: Arc::new(source.clone()),
            layout,
        })
    }

    pub fn pagination(&self) -> &PaginationResult {
        &self.layout.pagination
    }

    pub fn render_page(&self, index: usize) -> Result<PageRenderTree, HostedTableError> {
        self.layout.render_page(&self.source, index)
    }

    /// Explicit preview export; product Canvas and SVG use DocumentCore instead.
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
}

impl HostedSectionLayout {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare(
        source: &Document,
        section: usize,
        dpi: f64,
        policy: CellEndPolicy,
        page_offset: u32,
        first_number: u32,
        product_styles: Option<&ResolvedStyleSet>,
        story_context: &mut super::host_stories::StoryContext,
    ) -> Result<Self, HostedTableError> {
        let fail = HostedTableError::UnsupportedHost;
        let sec = source
            .sections
            .get(section)
            .ok_or_else(|| fail("section index"))?;
        let def = &sec.section_def;
        if !dpi.is_finite()
            || dpi <= 0.0
            // First-page master suppression and the serialized master-kind
            // bits do not change body flow. Other section rules stay gated.
            || def.flags & !(0xe000_0000 | 0x0007 | 0x0008_0000) != 0
            || def.line_grid != 0
            || def.char_grid != 0
            || def.text_direction != 0
            || def.hide_border || def.hide_fill
            || def.first_page_border || def.first_page_fill
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
        super::host_master::validate(&def.master_pages)?;
        if def.page_num_type != 0 {
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
        let mut shape_paragraphs = std::collections::BTreeSet::new();
        let mut excluded_paragraphs = std::collections::BTreeSet::new();
        let mut positioned_paragraphs = BTreeMap::new();
        let mut absolute = BTreeMap::new();
        let mut stories = Vec::new();
        let mut hf_entries = Vec::new();
        let styles = match product_styles {
            Some(styles) => super::source_units::qualify(source, dpi, styles.clone())?,
            None => super::source_units::resolve(source, dpi)?,
        };
        for (pi, para) in sec.paragraphs.iter().enumerate() {
            (|| -> Result<(), HostedTableError> {
                super::char_border::validate_source(para, &source.doc_info)?;
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
                        Control::Shape(s)
                            if s.common().treat_as_char
                                && para.controls.iter().all(|c| {
                                    matches!(
                                        c,
                                        Control::Shape(_)
                                            | Control::ColumnDef(_)
                                            | Control::SectionDef(_)
                                    )
                                }) =>
                        {
                            shape_paragraphs.insert(pi);
                        }
                        Control::SectionDef(_) if pi == 0 => {}
                        Control::Header(_) | Control::Footer(_) => {
                            let (is_header, apply) = match control {
                                Control::Header(h) => (true, h.apply_to),
                                Control::Footer(f) => (false, f.apply_to),
                                _ => unreachable!(),
                            };
                            let line = super::host_stories::owner_line(para, ci)?;
                            let reference = crate::renderer::pagination::HeaderFooterRef {
                                para_index: pi,
                                control_index: ci,
                                source_section_index: section,
                                table_path: Vec::new(),
                            };
                            let layout = PageLayoutInfo::from_page_def(p, &columns, dpi);
                            super::host_stories::render(
                                source,
                                &reference,
                                is_header,
                                if is_header {
                                    layout.header_area
                                } else {
                                    layout.footer_area
                                },
                                first_number,
                                &styles,
                                dpi,
                            )?;
                            hf_entries.push((line, (pi, reference, is_header, apply)));
                        }
                        Control::ColumnDef(c) if pi == 0 && !seen_columns => {
                            columns = c.clone();
                            seen_columns = true;
                        }
                        Control::PageNumberPos(value) => {
                            let line = super::host_stories::owner_line(para, ci)?;
                            if para.controls.iter().any(|c| {
                                matches!(c, Control::Table(t)
                            if t.common.treat_as_char || t.common.text_wrap != TextWrap::Square)
                            }) {
                                return Err(fail(
                                    "body page-number declaration with table controls",
                                ));
                            }
                            let layout = PageLayoutInfo::from_page_def(p, &columns, dpi);
                            super::page_number::PageNumberStory::new(value, def, &layout)?;
                            stories.push((pi, line, value.clone()));
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
                                && matches!(
                                    t.common.vert_rel_to,
                                    VertRelTo::Paper | VertRelTo::Page
                                ) =>
                        {
                            if para
                                .controls
                                .iter()
                                .filter(|c| matches!(c, Control::Table(_)))
                                .count()
                                != 1
                                || has_page_number_story(t)
                            {
                                return Err(fail("absolute table control ownership"));
                            }
                            absolute.insert(
                                pi,
                                super::host_absolute::HostedAbsolute::prepare(
                                    source, para, ci, &styles, dpi, policy,
                                )?,
                            );
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
                        HostedTableSession::from_document_with_styles(
                            source,
                            TableSelection {
                                section,
                                paragraph: pi,
                                control,
                            },
                            dpi,
                            policy,
                            &styles,
                        )?,
                    );
                }
                Ok(())
            })()
            .map_err(|e| e.in_paragraph(pi))?;
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
            (|| -> Result<(), HostedTableError> {
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
                    return Ok(());
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
                    return Ok(());
                }
                let mut local = para.clone();
                // The already-qualified initial section/column controls are non-textual.
                // Retain saved offsets/line partitions and the authored empty line.
                local.controls.retain(|c| {
                    !matches!(
                        c,
                        Control::ColumnDef(_)
                            | Control::SectionDef(_)
                            | Control::PageNumberPos(_)
                            | Control::Header(_)
                            | Control::Footer(_)
                    ) && !(absolute.contains_key(&pi) && matches!(c, Control::Table(_)))
                });
                local.column_type = ColumnBreakType::None;
                if shape_paragraphs.contains(&pi) {
                    // A mixed TAC story's control slots and char_offsets are one
                    // source axis. Removing secd/cold but retaining their raw gaps
                    // assigns the shape to the first structural slot. The host has
                    // already consumed these declarations; keep them for the shared
                    // text/object walk, just as for inline tables below.
                    if super::shapes::mixed_inline_candidate(para) {
                        local = para.clone();
                        local.column_type = ColumnBreakType::None;
                    }
                    if super::shapes::mixed_inline_candidate(&local) {
                        super::host_border::validate_source(para.para_shape_id, &source.doc_info)?;
                    } else {
                        super::decoration::validate_paragraph_source(
                            para.para_shape_id,
                            &source.doc_info,
                        )?;
                    }
                    text.push(HostedParagraphPlan::prepare_shapes(
                        &local,
                        width,
                        &text_styles,
                        dpi,
                        &source.bin_data_content,
                    )?);
                    return Ok(());
                }
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
                    return Ok(());
                }
                if tables.contains_key(&pi) {
                    if let Some(owner) =
                        HostedParagraphPlan::prepare_table_owner(&local, width, &text_styles, dpi)?
                    {
                        text.push(owner);
                        return Ok(());
                    }
                }
                super::host_border::validate_source(para.para_shape_id, &source.doc_info)?;
                text.push(HostedParagraphPlan::prepare(
                    &local,
                    width,
                    &text_styles,
                    dpi,
                )?);
                Ok(())
            })()
            .map_err(|e| e.in_paragraph(pi))?;
        }
        let mut pagination = TypesetEngine::new(dpi).typeset_hosted_section(
            &sec.paragraphs,
            text,
            &styles,
            p,
            &columns,
            section,
            def.hide_empty_line,
            tables,
            page_offset,
            first_number,
        )?;
        // Packets already consumed the document numbering context during fit.
        // Bind the public host addresses before absolute objects/master paint.
        for (index, page) in pagination.pages.iter_mut().enumerate() {
            let index = u32::try_from(index).map_err(|_| fail("page-number range"))?;
            page.page_index = page_offset
                .checked_add(index)
                .ok_or_else(|| fail("page range"))?;
            page.page_number = first_number
                .checked_add(index)
                .ok_or_else(|| fail("page-number range"))?;
        }
        // Bind each absolute object to its accepted source control line. A
        // paragraph can span columns/pages: neither its first nor last packet
        // substitutes for the actual line containing the control.
        let mut page_tables = Vec::with_capacity(pagination.pages.len());
        for page in &pagination.pages {
            let mut placed = Vec::new();
            for column in &page.column_contents {
                for item in &column.items {
                    let crate::renderer::pagination::PageItem::HostedParagraph {
                        para_index,
                        fragment,
                    } = item
                    else {
                        continue;
                    };
                    if absolute
                        .get(para_index)
                        .is_some_and(|a| fragment.contains_line(a.owner_line))
                    {
                        let a = absolute
                            .remove(para_index)
                            .expect("qualified absolute owner");
                        let layout = column.zone_layout.as_ref().unwrap_or(&page.layout);
                        placed.push(a.place(
                            layout,
                            usize::from(column.column_index),
                            section,
                            *para_index,
                            page.page_number,
                        )?);
                    }
                }
            }
            // Stored line geometry is not permission to overlap. TopAndBottom
            // excludes a vertical band in each horizontally affected column,
            // including blank lines. Fresh reflow around these objects is not
            // silently simulated by pushing the table or ignoring its area.
            for (i, (_, outer)) in placed.iter().enumerate() {
                for column in &page.column_contents {
                    let layout = column.zone_layout.as_ref().unwrap_or(&page.layout);
                    let lane = layout.column_areas[usize::from(column.column_index)];
                    if outer.x < lane.x + lane.width && lane.x < outer.x + outer.width {
                        let band = super::Rect {
                            x: lane.x,
                            width: lane.width,
                            ..*outer
                        };
                        let boxes = column.items.iter().flat_map(|item| match item {
                            crate::renderer::pagination::PageItem::HostedParagraph {
                                fragment,
                                ..
                            } => fragment.flow_boxes().chain(fragment.exclusion()).collect(),
                            crate::renderer::pagination::PageItem::HostedTable {
                                fragment, ..
                            } => vec![fragment.occupied()],
                            _ => Vec::new(),
                        });
                        super::body_flow::validate_exclusion(band, boxes)?;
                    }
                }
                super::body_flow::validate_exclusion(
                    *outer,
                    placed
                        .iter()
                        .enumerate()
                        .filter(|(other, _)| *other != i)
                        .map(|(_, (_, b))| *b),
                )?;
            }
            page_tables.push(placed.into_iter().map(|(node, _)| node).collect());
        }
        if !absolute.is_empty() {
            return Err(fail("unconsumed absolute table owner"));
        }
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
        let mut consumed_stories = std::collections::BTreeSet::new();
        let mut consumed_hf = std::collections::BTreeSet::new();
        let mut page_stories = Vec::with_capacity(pagination.pages.len());
        for (index, page) in pagination.pages.iter_mut().enumerate() {
            for (entry_index, (pi, line, value)) in stories.iter().enumerate() {
                if !consumed_stories.contains(&entry_index)
                    && page.column_contents.iter().flat_map(|c| &c.items).any(|item| {
                        matches!(item, crate::renderer::pagination::PageItem::HostedParagraph { para_index, fragment }
                            if para_index == pi && fragment.contains_line(*line))
                    })
                {
                    story_context.page_number = Some(value.clone());
                    consumed_stories.insert(entry_index);
                }
            }
            page.page_number_pos = None;
            let mut nodes = Vec::new();
            if let Some(declaration) = &story_context.page_number {
                if let Some(node) =
                    super::page_number::PageNumberStory::new(declaration, def, &page.layout)?
                        .render_number(page.page_number)?
                {
                    nodes.push(node);
                }
            }
            for (entry_index, (line, entry)) in hf_entries.iter().enumerate() {
                if consumed_hf.contains(&entry_index) {
                    continue;
                }
                let accepted = page.column_contents.iter().flat_map(|c| &c.items).any(|item| {
                    matches!(item, crate::renderer::pagination::PageItem::HostedParagraph { para_index, fragment }
                        if *para_index == entry.0 && fragment.contains_line(*line))
                });
                if accepted {
                    story_context
                        .header_footer
                        .accumulate(std::slice::from_ref(entry), entry.0);
                    consumed_hf.insert(entry_index);
                }
            }
            let (header, footer) = story_context.header_footer.active(page.page_number);
            page.active_header = header.filter(|_| !(index == 0 && def.hide_header));
            page.active_footer = footer.filter(|_| !(index == 0 && def.hide_footer));
            for (reference, is_header, area) in [
                (&page.active_header, true, page.layout.header_area),
                (&page.active_footer, false, page.layout.footer_area),
            ] {
                if let Some(reference) = reference {
                    nodes.push(super::host_stories::render(
                        source,
                        reference,
                        is_header,
                        area,
                        page.page_number,
                        &styles,
                        dpi,
                    )?);
                }
            }
            page_stories.push(nodes);
        }
        if consumed_stories.len() != stories.len() || consumed_hf.len() != hf_entries.len() {
            return Err(fail("unconsumed page-story declaration owner"));
        }
        crate::renderer::master_page::assign_master_pages_for_section(
            &mut pagination,
            section,
            sec,
            &None,
            &None,
        );
        Ok(Self {
            section,
            dpi,
            styles,
            pagination,
            page_stories,
            page_tables,
        })
    }

    pub(crate) fn pagination(&self) -> &PaginationResult {
        &self.pagination
    }

    pub(crate) fn render_story_preview(
        &self,
        document: &Document,
        index: usize,
        reference: &crate::renderer::pagination::HeaderFooterRef,
        header: bool,
    ) -> Result<PageRenderTree, HostedTableError> {
        let mut tree = self.render_page(document, index)?;
        let page = &self.pagination.pages[index];
        let area = if header {
            page.layout.header_area
        } else {
            page.layout.footer_area
        };
        let mut story = super::host_stories::render(
            document,
            reference,
            header,
            area,
            page.page_number,
            &self.styles,
            self.dpi,
        )?;
        tree.root.children.retain(|node| {
            !matches!(
                (&node.node_type, header),
                (crate::renderer::render_tree::RenderNodeType::Header, true)
                    | (crate::renderer::render_tree::RenderNodeType::Footer, false)
            )
        });
        super::text::assign_ids(&mut story, tree.frame_mut());
        tree.root.children.push(story);
        Ok(tree)
    }

    pub(crate) fn render_page(
        &self,
        document: &Document,
        index: usize,
    ) -> Result<PageRenderTree, HostedTableError> {
        let page = self
            .pagination
            .pages
            .get(index)
            .ok_or(HostedTableError::WrongDestination)?;
        let master = page.active_master_page.as_ref().and_then(|reference| {
            document
                .sections
                .get(reference.section_index)?
                .section_def
                .master_pages
                .get(reference.master_page_index)
        });
        let mut tree = super::host_page::build(
            page,
            &document.sections[self.section].paragraphs,
            &self.styles,
        )?;
        if let Some(source) =
            master.filter(|_| !page.page_hide.as_ref().is_some_and(|h| h.hide_master_page))
        {
            let mut node = super::host_master::render(
                source,
                &page.layout,
                &self.styles,
                &document.bin_data_content,
                page.page_number,
                page.section_index,
            )?;
            super::text::assign_ids(&mut node, tree.frame_mut());
            tree.root.children.insert(1, node);
        }
        for mut table in self.page_tables[index].iter().cloned() {
            super::text::assign_ids(&mut table, tree.frame_mut());
            tree.root.children.push(table);
        }
        tree.root.children.retain(|n| {
            !matches!(
                n.node_type,
                crate::renderer::render_tree::RenderNodeType::Header
                    | crate::renderer::render_tree::RenderNodeType::Footer
            )
        });
        for mut story in self.page_stories[index].clone() {
            super::text::assign_ids(&mut story, tree.frame_mut());
            tree.root.children.push(story);
        }
        super::ownership::bind(&mut tree.root, self.section, None);
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
