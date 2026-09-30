//! One composed paragraph snapshot supplies host fit and final paint. The host
//! owns column/page transitions; saved line partitions never undergo a second
//! Legacy reflow during painting.
use std::{cell::RefCell, collections::HashMap, sync::Arc};

use crate::{
    model::paragraph::{LineSeg, Paragraph},
    renderer::{
        hwpunit_to_px,
        render_tree::{BoundingBox, PageLayoutContext, RenderNode, RenderNodeType, TextLineNode},
        style_resolver::ResolvedStyleSet,
    },
};

use super::{
    flow::FlowCursor, text::TextComposer, CellParagraphComposer, FlowBlock, FlowCellInput,
    GeometryError, Insets, LineBox, LineOwner, ParagraphItem, Rect,
};

pub(crate) struct HostedParagraphPlan {
    flow: FlowCellInput,
    payloads: Vec<RenderNode>,
    cursor: FlowCursor,
    emitted: bool,
    frame_breaks: Vec<usize>,
    next_break: usize,
    anchor_required: Option<f64>,
    tables: HashMap<super::ControlOwner, (usize, Arc<super::text::TextPaint>)>,
    end: Option<super::ParagraphEnd>,
    anchor: Option<super::host_anchor::HostedAnchor>,
}

/// Immutable absolute-coordinate packet, emitted only after successful fit.
#[derive(Debug)]
pub struct HostedParagraphFragment {
    bounds: Rect,
    next_y: f64,
    first: bool,
    nodes: Vec<RenderNode>,
    exclusion: Option<Rect>,
}

impl HostedParagraphFragment {
    pub(super) fn contains_line(&self, line: usize) -> bool {
        self.nodes.iter().any(|n| {
            matches!(&n.node_type,
            RenderNodeType::TextLine(t) if t.line_index.map(|i| i as usize) == Some(line))
        })
    }
    pub(crate) fn line_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|n| matches!(n.node_type, RenderNodeType::TextLine(_)))
            .count()
    }

    pub(super) fn flow_boxes(&self) -> impl Iterator<Item = Rect> + '_ {
        self.nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.node_type,
                    RenderNodeType::TextLine(_) | RenderNodeType::Table(_)
                )
            })
            .map(|n| Rect {
                x: n.bbox.x,
                y: n.bbox.y,
                width: n.bbox.width,
                height: n.bbox.height,
            })
    }

    pub(super) fn exclusion(&self) -> Option<Rect> {
        self.exclusion
    }
    pub(crate) fn line_end(&self) -> Option<f64> {
        self.nodes
            .iter()
            .filter(|n| matches!(n.node_type, RenderNodeType::TextLine(_)))
            .map(|n| n.bbox.y + n.bbox.height)
            .reduce(f64::max)
    }

    pub(super) fn anchored(
        bounds: Rect,
        next_y: f64,
        first: bool,
        nodes: Vec<RenderNode>,
        exclusion: Option<Rect>,
    ) -> Self {
        Self {
            bounds,
            next_y,
            first,
            nodes,
            exclusion,
        }
    }
    pub fn occupied(&self) -> Rect {
        self.bounds
    }
    pub(crate) fn next_y(&self) -> f64 {
        self.next_y
    }
    pub(crate) fn is_first(&self) -> bool {
        self.first
    }
    pub(crate) fn has_body_line(&self) -> bool {
        self.nodes
            .iter()
            .any(|node| matches!(node.node_type, RenderNodeType::TextLine(_)))
    }
    pub(crate) fn render_nodes(&self, context: &mut PageLayoutContext) -> Vec<RenderNode> {
        let mut nodes = self.nodes.clone();
        for n in &mut nodes {
            super::text::assign_ids(n, context);
        }
        nodes
    }
}

impl HostedParagraphPlan {
    pub(crate) fn planned_line_count(&self) -> usize {
        self.payloads.len()
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_positioned(
        document: &crate::model::document::Document,
        para: &Paragraph,
        control: usize,
        width: f64,
        paper_right: f64,
        styles: &ResolvedStyleSet,
        dpi: f64,
        policy: super::CellEndPolicy,
    ) -> Result<Self, GeometryError> {
        if para.line_segs.is_empty() || !super::body_text::frame_starts(para)?.is_empty() {
            return Err(GeometryError::Unsupported(
                "stored body anchor frame ownership",
            ));
        }
        super::decoration::validate_paragraph_source(para.para_shape_id, &document.doc_info)?;
        let style = styles
            .para_styles
            .get(para.para_shape_id as usize)
            .ok_or(GeometryError::Unsupported("missing paragraph style"))?;
        if [style.margin_left, style.margin_right, style.spacing_before]
            .iter()
            .any(|v| *v != 0.0)
        {
            return Err(GeometryError::Unsupported("anchored host paragraph insets"));
        }
        let mut local = para.clone();
        local.controls.clear();
        local.ctrl_data_records.clear();
        local.column_type = crate::model::paragraph::ColumnBreakType::None;
        let mut plan = Self::prepare(&local, width, styles, dpi)?;
        plan.anchor = Some(super::host_anchor::HostedAnchor::prepare(
            document,
            para,
            control,
            plan.end
                .as_ref()
                .ok_or(GeometryError::InconsistentAtomicPlan)?,
            width,
            paper_right,
            styles,
            dpi,
            policy,
        )?);
        Ok(plan)
    }

    pub(crate) fn take_anchor(&mut self) -> Option<super::host_anchor::HostedAnchor> {
        self.anchor.take()
    }

    /// Reuse body TAC composition, including saved ownership and fresh wrapping.
    /// No document/page loop is delegated to the standalone preview engine.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_inline(
        document: &crate::model::document::Document,
        para: &Paragraph,
        width: f64,
        styles: &ResolvedStyleSet,
        dpi: f64,
        policy: super::CellEndPolicy,
        frame_end: bool,
    ) -> Result<Self, GeometryError> {
        super::decoration::validate_paragraph_source(para.para_shape_id, &document.doc_info)?;
        let inline =
            super::body_inline::prepare(document, para, 0, width, styles, dpi, policy, frame_end)?;
        Self::from_composition(inline, width)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_excluded(
        document: &crate::model::document::Document,
        para: &Paragraph,
        width: f64,
        paper_right: f64,
        styles: &ResolvedStyleSet,
        dpi: f64,
        policy: super::CellEndPolicy,
    ) -> Result<Self, GeometryError> {
        let composition = super::body_excluded::prepare(
            document,
            para,
            0,
            width,
            paper_right,
            styles,
            dpi,
            policy,
        )?;
        Self::from_composition(composition, width)
    }

    fn from_composition(
        inline: super::body_inline::InlineParagraph,
        width: f64,
    ) -> Result<Self, GeometryError> {
        let mut payloads: Vec<_> = inline.lines.into_iter().collect();
        payloads.sort_by_key(|(owner, _)| owner.line);
        for (index, (owner, _)) in payloads.iter().enumerate() {
            if owner.line != index {
                return Err(GeometryError::InconsistentAtomicPlan);
            }
        }
        Ok(Self {
            flow: FlowCellInput {
                width,
                minimum_height: 0.0,
                padding: Insets::default(),
                blocks: inline.blocks,
            },
            payloads: payloads.into_iter().map(|(_, (_, node))| node).collect(),
            tables: inline.tables,
            cursor: FlowCursor::default(),
            emitted: false,
            frame_breaks: Vec::new(),
            next_break: 0,
            anchor_required: None,
            end: None,
            anchor: None,
        })
    }

    /// A qualified TopAndBottom table can exclude the entire text lane while
    /// retaining its owner's real line metrics. That line shares the table's
    /// anchor; it is not a free blank paragraph following all continuations.
    /// The caller has admitted exactly one non-inline, zero-offset table.
    pub(crate) fn prepare_table_owner(
        para: &Paragraph,
        width: f64,
        styles: &ResolvedStyleSet,
        dpi: f64,
    ) -> Result<Option<Self>, GeometryError> {
        if !para.line_segs.iter().any(|s| s.segment_width == 0) {
            return Ok(None);
        }
        let fail = || GeometryError::Unsupported("occluded table owner line");
        let [row] = para.line_segs.as_slice() else {
            return Err(fail());
        };
        let style = styles
            .para_styles
            .get(para.para_shape_id as usize)
            .ok_or_else(fail)?;
        if !para.text.is_empty()
            || !para.char_offsets.is_empty()
            || para.stored_text_partition_is_dirty()
            || !para.controls.is_empty()
            || para.source_line_seg_vertical_pos.is_some()
            || para.layout_only_fill_lines != 0
            || !para.field_ranges.is_empty()
            || !para.orphan_field_ends.is_empty()
            || !para.range_tags.is_empty()
            || !para.title_marks.is_empty()
            || !para.markpen_marks.is_empty()
            || row.text_start != 0
            || row.segment_width != 0
            || row.column_start < 0
            || hwpunit_to_px(row.column_start, dpi) > width
            || row.tag & LineSeg::TAG_SINGLE_SEGMENT_LINE != LineSeg::TAG_SINGLE_SEGMENT_LINE
            || row.tag
                & (LineSeg::TAG_IMPLEMENTATION_PROPERTY
                    | LineSeg::TAG_AUTO_HYPHENATION
                    | LineSeg::TAG_PARAGRAPH_HEAD)
                != 0
            || row.line_height <= 0
            || row.text_height != row.line_height
            || row.baseline_distance < 0
            || row.baseline_distance > row.line_height
            || i64::from(row.line_height) + i64::from(row.line_spacing) < 0
            || style.spacing_before != 0.0
            || style.spacing_after != 0.0
            || !super::decoration::paragraph_is_unpainted(style.border_fill_id, styles)
            || style.keep_with_next
            || style.widow_orphan
        {
            return Err(fail());
        }
        let height = hwpunit_to_px(row.line_height, dpi);
        let advance = (f64::from(row.line_height) + f64::from(row.line_spacing)) * dpi / 7200.0;
        let bounds = Rect {
            x: hwpunit_to_px(row.column_start, dpi),
            y: 0.0,
            width: 0.0,
            height,
        };
        let mut line = TextLineNode::new(height, hwpunit_to_px(row.baseline_distance, dpi));
        line.line_index = Some(0);
        line.vpos = Some(row.vertical_pos);
        Ok(Some(Self {
            flow: FlowCellInput {
                width,
                minimum_height: 0.0,
                padding: Insets::default(),
                blocks: vec![FlowBlock::Lines {
                    height,
                    advance,
                    lines: vec![LineBox {
                        owner: LineOwner {
                            paragraph: 0,
                            line: 0,
                        },
                        bounds,
                    }],
                }],
            },
            payloads: vec![RenderNode::new(
                0,
                RenderNodeType::TextLine(line),
                BoundingBox::new(bounds.x, bounds.y, bounds.width, bounds.height),
            )],
            cursor: FlowCursor::default(),
            emitted: false,
            frame_breaks: Vec::new(),
            next_break: 0,
            anchor_required: Some(height.max(advance)),
            end: None,
            anchor: None,
            tables: HashMap::new(),
        }))
    }

    /// Query only; a failed joint table/line fit consumes neither owner.
    pub(crate) fn anchor_required(&self) -> Option<f64> {
        self.anchor_required.filter(|_| !self.complete())
    }

    pub(crate) fn prepare(
        para: &Paragraph,
        width: f64,
        styles: &ResolvedStyleSet,
        dpi: f64,
    ) -> Result<Self, GeometryError> {
        Self::prepare_content(para, width, styles, dpi, None)
    }

    pub(crate) fn prepare_shapes(
        para: &Paragraph,
        width: f64,
        styles: &ResolvedStyleSet,
        dpi: f64,
        resources: &[crate::model::bin_data::BinDataContent],
    ) -> Result<Self, GeometryError> {
        Self::prepare_content(para, width, styles, dpi, Some(resources))
    }

    fn prepare_content(
        para: &Paragraph,
        width: f64,
        styles: &ResolvedStyleSet,
        dpi: f64,
        shape_resources: Option<&[crate::model::bin_data::BinDataContent]>,
    ) -> Result<Self, GeometryError> {
        // Admission has selected uniform normal columns in source-flow order.
        // A qualified zero reset consumes the next host frame: next column,
        // then next page, never an independently inferred absolute page number.
        let starts = super::body_text::frame_starts(para)?;
        let continuous;
        let para = if starts.is_empty() {
            para
        } else {
            continuous = super::stored_text::continuous_paragraph(para, &starts)?;
            &continuous
        };
        let composer = TextComposer {
            styles,
            dpi,
            payloads: RefCell::new(Vec::new()),
        };
        let items = if let Some(resources) = shape_resources {
            if super::shapes::mixed_inline_candidate(para) {
                composer.compose_stored_inline_shapes(para, width, resources)?
            } else {
                let (items, nodes) = super::pictures::compose(para, width, styles, dpi, resources)?;
                composer.payloads.borrow_mut().push(nodes);
                items
            }
        } else {
            composer.compose(para, width)?
        };
        let paragraph_end = items.iter().find_map(|item| match item {
            ParagraphItem::End(end) => Some(end.clone()),
            _ => None,
        });
        let mut blocks = Vec::new();
        let mut frame_breaks = Vec::new();
        for item in items {
            // The shape adapters already resolved baseline + margins and
            // painted the object into this same saved line envelope. Do not
            // sum object heights or invent another page-owned shape pass.
            let item = match item {
                ParagraphItem::ObjectRow { line, bounds, .. } if shape_resources.is_some() => {
                    ParagraphItem::Lines {
                        height: bounds.height,
                        advance: bounds.height,
                        lines: vec![(line, bounds)],
                    }
                }
                other => other,
            };
            match item {
                ParagraphItem::Space(h) => blocks.push(FlowBlock::Space(h)),
                ParagraphItem::End(end) => blocks.extend(end.into_body_tail()),
                ParagraphItem::Lines {
                    height,
                    advance,
                    lines,
                } => {
                    if lines.iter().skip(1).any(|(line, _)| starts.contains(line)) {
                        return Err(GeometryError::Unsupported(
                            "host stored frame inside atomic paragraph",
                        ));
                    }
                    if lines.first().is_some_and(|(line, _)| starts.contains(line)) {
                        super::body_text::end_frame(&mut blocks);
                        frame_breaks.push(blocks.len());
                    }
                    blocks.push(FlowBlock::Lines {
                        height,
                        advance,
                        lines: lines
                            .into_iter()
                            .map(|(line, bounds)| LineBox {
                                owner: LineOwner { paragraph: 0, line },
                                bounds,
                            })
                            .collect(),
                    });
                }
                _ => return Err(GeometryError::Unsupported("host text control occupancy")),
            }
        }
        let payloads = composer
            .payloads
            .into_inner()
            .pop()
            .ok_or(GeometryError::InconsistentAtomicPlan)?;
        Ok(Self {
            flow: FlowCellInput {
                width,
                minimum_height: 0.0,
                padding: Insets::default(),
                blocks,
            },
            payloads,
            cursor: FlowCursor::default(),
            emitted: false,
            frame_breaks,
            next_break: 0,
            anchor_required: None,
            end: paragraph_end,
            anchor: None,
            tables: HashMap::new(),
        })
    }

    pub(crate) fn complete(&self) -> bool {
        self.cursor.block == self.flow.blocks.len()
    }

    pub(crate) fn take_frame_break(&mut self) -> bool {
        if self.frame_breaks.get(self.next_break) == Some(&self.cursor.block) {
            self.next_break += 1;
            true
        } else {
            false
        }
    }

    pub(crate) fn fit(
        &mut self,
        area: Rect,
        section: usize,
        paragraph: usize,
        page_height: f64,
        page_number: u32,
    ) -> Result<Option<HostedParagraphFragment>, GeometryError> {
        let end = self
            .frame_breaks
            .get(self.next_break)
            .copied()
            .unwrap_or(self.flow.blocks.len());
        let fit = self
            .cursor
            .fit_body_until(&self.flow, area, end, page_height)?;
        if !fit.progressed {
            return Ok(None);
        }
        let mut nodes = Vec::new();
        for line in &fit.lines {
            let mut node = self
                .payloads
                .get(line.owner.line)
                .ok_or(GeometryError::InconsistentAtomicPlan)?
                .clone();
            let dx = line.bounds.x - node.bbox.x;
            let dy = line.bounds.y - node.bbox.y;
            super::text::translate(&mut node, dx, dy);
            if let RenderNodeType::TextLine(n) = &mut node.node_type {
                n.section_index = Some(section);
                n.para_index = Some(paragraph);
            }
            for child in &mut node.children {
                if let RenderNodeType::TextRun(n) = &mut child.node_type {
                    n.section_index = Some(section);
                    n.para_index = Some(paragraph);
                }
            }
            nodes.push(node);
        }
        let mut tables: Vec<_> = fit
            .tables
            .iter()
            .map(|table| {
                let (order, paint) = self
                    .tables
                    .get(&table.owner)
                    .ok_or(GeometryError::InconsistentAtomicPlan)?;
                let mut node = paint.build_node(&table.placement, Some(page_number))?;
                if let RenderNodeType::Table(value) = &mut node.node_type {
                    value.section_index = Some(section);
                    value.para_index = Some(paragraph);
                    value.control_index = Some(table.owner.control);
                }
                Ok((*order, node))
            })
            .collect::<Result<_, GeometryError>>()?;
        tables.sort_by_key(|(order, _)| *order);
        nodes.extend(tables.into_iter().map(|(_, node)| node));
        let packet = HostedParagraphFragment {
            exclusion: None,
            bounds: Rect {
                height: fit.height,
                x: if self.anchor_required.is_some() {
                    nodes[0].bbox.x
                } else {
                    area.x
                },
                width: if self.anchor_required.is_some() {
                    0.0
                } else {
                    area.width
                },
                ..area
            },
            next_y: area.y + fit.advance,
            // A split anchored table remains block0 on later pages. First
            // ownership is determined by committed packets, not block index.
            first: !self.emitted,
            nodes,
        };
        self.cursor = fit.next;
        self.emitted = true;
        Ok(Some(packet))
    }
}
