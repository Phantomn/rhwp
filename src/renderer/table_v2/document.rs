//! Opt-in document body flow. Uses the same recursive fit/paint as V2 cells,
//! without inventing an enclosing table or constructing Legacy DocumentCore.
//! Unsupported document features fail before publishing a partial document.
use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::renderer::{
    render_tree::{BoundingBox, PageRenderTree, RenderNode, RenderNodeType},
    svg::SvgRenderer,
};

use super::{
    body_flow::{AnchoredFlow, BodyCursor},
    text::{assign_ids, translate, TextPaint},
    ControlOwner, FlowCellInput, GeometryError, LineOwner, Rect,
};

#[derive(Debug)]
pub enum DocumentV2Error {
    Options(String),
    Parse(String),
    Unsupported(&'static str),
    Paragraph { index: usize, reason: GeometryError },
    Geometry(GeometryError),
    PageLimit(u32),
    DoesNotFit { page: u32, required_height: f64 },
    Serialize(String),
}

impl std::fmt::Display for DocumentV2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "table_v2 document: {self:?}")
    }
}
impl std::error::Error for DocumentV2Error {}
impl From<GeometryError> for DocumentV2Error {
    fn from(value: GeometryError) -> Self {
        Self::Geometry(value)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Options {
    dpi: f64,
    max_pages: u32,
    #[serde(default)]
    cell_end_policy: super::CellEndPolicy,
}

pub(super) struct BodyPlan {
    pub flow: FlowCellInput,
    pub anchors: Vec<AnchoredFlow>,
    /// (story block, preceding anchor count): an explicit paragraph-entry or
    /// qualified stored-line body frame break.
    /// The count orders an earlier floating table before a coincident boundary.
    pub page_breaks: Vec<(usize, usize)>,
    pub lines: HashMap<LineOwner, (usize, RenderNode)>,
    pub tables: HashMap<ControlOwner, (usize, Arc<TextPaint>)>,
    pub body: Rect,
    pub page_width: f64,
    pub page_height: f64,
    pub first_page_number: u32,
    /// Source-ordered declarations. The first accepted host flow unit activates
    /// a story; a later declaration on the same page supersedes an earlier one.
    pub page_numbers: Vec<(
        super::page_number::PageNumberHost,
        super::page_number::PageNumberStory,
    )>,
}

#[derive(Serialize)]
struct Output {
    schema_version: u32,
    engine: &'static str,
    scope: &'static str,
    page_index: u32,
    svg: String,
    render_tree: PageRenderTree,
}

/// Experimental snapshot, NOT a switch on a live Studio document. Initially
/// supports one uniform horizontal section, fresh/qualified stored text and the V2
/// zero-offset TopAndBottom tables, disjoint stored paragraph-relative anchors,
/// and qualified atomic stored TAC rows.
/// No edits, hidden fallback or saved-row purge.
#[derive(Clone)]
pub struct DocumentV2Session {
    plan: Arc<BodyPlan>,
    cursor: BodyCursor,
    emitted: u32,
    max_pages: u32,
    number_active: Option<usize>,
}

impl DocumentV2Session {
    /// Strict options: {"dpi":96,"max_pages":100}. Page/body geometry comes
    /// from the source PageDef, never caller-supplied preview coordinates.
    /// Optional `cell_end_policy` is applied inside tables only. Body paragraph
    /// advances, stored TAC envelopes and the default policy remain unchanged.
    pub fn from_bytes(bytes: &[u8], options: &str) -> Result<Self, DocumentV2Error> {
        let options: Options =
            serde_json::from_str(options).map_err(|e| DocumentV2Error::Options(e.to_string()))?;
        if !options.dpi.is_finite() || options.dpi <= 0.0 || options.max_pages == 0 {
            return Err(DocumentV2Error::Options(
                "positive DPI and page limit required".into(),
            ));
        }
        let document =
            crate::parse_document(bytes).map_err(|e| DocumentV2Error::Parse(e.to_string()))?;
        Ok(Self {
            plan: Arc::new(super::document_input::prepare(
                &document,
                options.dpi,
                options.cell_end_policy,
            )?),
            cursor: BodyCursor::default(),
            emitted: 0,
            max_pages: options.max_pages,
            number_active: None,
        })
    }

    pub fn emitted_pages(&self) -> u32 {
        self.emitted
    }

    /// Fit -> final nodes -> SVG/JSON -> commit. A paint/fit/serialization error
    /// does not consume any source unit, including an already started child.
    pub fn next_page_json(&mut self) -> Result<Option<String>, DocumentV2Error> {
        if self.cursor.complete(&self.plan) {
            return Ok(None);
        }
        if self.emitted == self.max_pages {
            return Err(DocumentV2Error::PageLimit(self.max_pages));
        }
        let fit = self.cursor.fit(&self.plan)?;
        super::body_flow::validate_side_wraps(&self.plan, &fit)?;
        if !fit.progressed {
            return Err(DocumentV2Error::DoesNotFit {
                page: self.emitted,
                required_height: fit.required,
            });
        }
        let b = self.plan.body;
        if fit.height > b.height && !fit.body_inline_overflow {
            return Err(GeometryError::InconsistentAtomicPlan.into());
        }
        let mut children = Vec::new();
        for line in &fit.lines {
            let (order, payload) = self
                .plan
                .lines
                .get(&line.owner)
                .ok_or(GeometryError::InconsistentAtomicPlan)?;
            let mut node = payload.clone();
            let dx = line.bounds.x - node.bbox.x;
            let dy = line.bounds.y - node.bbox.y;
            translate(&mut node, dx, dy);
            children.push((*order, node));
        }
        for table in &fit.tables {
            let (order, paint) = self
                .plan
                .tables
                .get(&table.owner)
                .ok_or(GeometryError::InconsistentAtomicPlan)?;
            let number = self
                .plan
                .first_page_number
                .checked_add(self.emitted)
                .ok_or(GeometryError::Unsupported("page-number range"))?;
            let mut node = paint.build_node(&table.placement, Some(number))?;
            if let RenderNodeType::Table(value) = &mut node.node_type {
                value.section_index = Some(0);
                value.para_index = Some(table.owner.paragraph);
                value.control_index = Some(table.owner.control);
            }
            children.push((*order, node));
        }
        children.sort_by_key(|(order, _)| *order);
        let mut body = RenderNode::new(
            0,
            RenderNodeType::Body { clip_rect: None },
            BoundingBox::new(b.x, b.y, b.width, b.height),
        );
        body.children = children.into_iter().map(|(_, node)| node).collect();
        let mut tree =
            PageRenderTree::new(self.emitted, self.plan.page_width, self.plan.page_height);
        assign_ids(&mut body, tree.frame_mut());
        tree.root.children.push(body);
        // Only accepted geometry activates a declaration, never a pending or
        // failed fit. Source order, not HashMap/paint order, chooses the last
        // declaration on this page (normal Hancom timeline fixture). Continuing
        // an older host cannot rewind the already committed story.
        let number_active = self.number_active.max(
            self.plan
                .page_numbers
                .iter()
                .rposition(|(host, _)| host.accepted(&fit.lines, &fit.tables)),
        );
        if let Some(index) = number_active {
            if let Some(mut node) = self.plan.page_numbers[index].1.render(self.emitted)? {
                assign_ids(&mut node, tree.frame_mut());
                tree.root.children.push(node);
            }
        }
        let mut svg = SvgRenderer::new();
        svg.render_tree(&tree);
        let result = serde_json::to_string(&Output {
            schema_version: 1,
            engine: "table_v2",
            scope: "document_body",
            page_index: self.emitted,
            svg: svg.output().to_owned(),
            render_tree: tree,
        })
        .map_err(|e| DocumentV2Error::Serialize(e.to_string()))?;
        self.cursor = fit.next;
        self.number_active = number_active;
        self.emitted += 1;
        Ok(Some(result))
    }
}
