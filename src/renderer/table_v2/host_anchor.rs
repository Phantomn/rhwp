//! Positioned reservation adapter. Uses the same anchor, child cuts, margins
//! and paint as BodyCursor; the existing section driver owns page transitions.
use super::{
    body_flow::AnchoredFlow, flow::FlowCursor, text::TextPaint, CellEndPolicy, ControlOwner,
    GeometryError, HostedParagraphFragment, ParagraphEnd, PreparedTextTable, Rect,
};
use crate::{
    model::{control::Control, document::Document, paragraph::Paragraph},
    renderer::{render_tree::RenderNodeType, style_resolver::ResolvedStyleSet},
};
use std::sync::Arc;

pub(crate) struct HostedAnchor {
    flow: AnchoredFlow,
    paint: Arc<TextPaint>,
    cursor: FlowCursor,
    deferred: bool,
    emitted: bool,
    control: usize,
}

impl HostedAnchor {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare(
        document: &Document,
        para: &Paragraph,
        control: usize,
        end: &ParagraphEnd,
        width: f64,
        paper_right: f64,
        styles: &ResolvedStyleSet,
        dpi: f64,
        policy: CellEndPolicy,
    ) -> Result<Self, GeometryError> {
        let Control::Table(table) = &para.controls[control] else {
            return Err(GeometryError::InconsistentAtomicPlan);
        };
        super::decoration::validate_source(table, &document.doc_info)?;
        let prepared = PreparedTextTable::prepare_with_end_policy(
            table,
            styles,
            dpi,
            &document.bin_data_content,
            policy,
        )?;
        let anchor = super::body_anchor::BodyAnchor::resolve(
            table,
            end,
            prepared.plan.width,
            width,
            paper_right,
            dpi,
        )?;
        Ok(Self {
            flow: anchor.flow(
                document,
                table,
                ControlOwner {
                    paragraph: 0,
                    control,
                },
                prepared.plan,
                0,
                width,
                paper_right,
                dpi,
            ),
            paint: prepared.paint,
            cursor: FlowCursor::default(),
            deferred: false,
            emitted: false,
            control,
        })
    }

    pub(crate) fn defer(&mut self) {
        debug_assert!(!self.emitted);
        self.deferred = true;
        self.cursor = FlowCursor::default();
    }

    pub(crate) fn is_side_wrap(&self) -> bool {
        self.flow.side_wrap.is_some()
    }

    pub(crate) fn complete(&self) -> bool {
        self.cursor.block == self.input().blocks.len()
    }

    fn input(&self) -> &super::FlowCellInput {
        if self.deferred {
            &self.flow.deferred
        } else {
            &self.flow.initial
        }
    }

    pub(crate) fn fit(
        &mut self,
        area: Rect,
        page_height: f64,
        section: usize,
        paragraph: usize,
        page_number: u32,
    ) -> Result<Option<HostedParagraphFragment>, GeometryError> {
        // The same signed paragraph-origin result as BodyCursor. A Square
        // box may begin beside the host, not necessarily below its last line.
        let area = Rect {
            y: area.y + self.flow.host_end_offset,
            height: area.height - self.flow.host_end_offset,
            ..area
        };
        let fit = self
            .flow
            .fit(&self.cursor, self.input(), area, page_height)?;
        if self.is_side_wrap()
            && (self.deferred
                || fit.next.block != self.input().blocks.len()
                || fit.tables.len() != 1)
        {
            return Err(GeometryError::Unsupported(
                "stored body side-wrap across pages",
            ));
        }
        // The requested offset alone is not an accepted table fragment. The
        // caller can leave following prose here and retry the object next page.
        if fit.tables.is_empty() && fit.next.block != self.input().blocks.len() {
            return Ok(None);
        }
        if !fit.progressed {
            return Ok(None);
        }
        let mut nodes = Vec::new();
        for table in &fit.tables {
            let mut node = self.paint.build_node(&table.placement, Some(page_number))?;
            if let RenderNodeType::Table(t) = &mut node.node_type {
                t.section_index = Some(section);
                t.para_index = Some(paragraph);
                t.control_index = Some(self.control);
            }
            nodes.push(node);
        }
        let exclusion = self.flow.side_wrap.map(|margin| {
            super::body_flow::exclusion_bounds(fit.tables[0].placement.bounds, margin)
        });
        let packet = HostedParagraphFragment::anchored(
            Rect {
                height: fit.height,
                ..area
            },
            area.y + fit.advance,
            // Its paragraph entry was already emitted by the owner line;
            // a deferred first table fragment is not another paragraph entry.
            false,
            nodes,
            exclusion,
        );
        self.cursor = fit.next;
        self.emitted = true;
        Ok(Some(packet))
    }
}
