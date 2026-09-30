//! Experimental byte/JSON transport shared by Native and WASM. This is a
//! selected-table preview, not a DocumentCore engine switch or a document render.
use serde::{Deserialize, Serialize};

use super::{TablePreviewError, TablePreviewPages, TablePreviewSession, TableSelection};
use crate::renderer::{render_tree::PageRenderTree, svg::SvgRenderer};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    selection: TableSelection,
    dpi: f64,
    pages: TablePreviewPages,
    max_pages: u32,
    #[serde(default)]
    cell_end_policy: super::CellEndPolicy,
    first_page_number: Option<u32>,
}

#[derive(Debug)]
pub enum TablePreviewExportError {
    Options(String),
    Parse(String),
    Preview(TablePreviewError),
    Serialize(String),
}

impl std::fmt::Display for TablePreviewExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "table_v2 export: {self:?}")
    }
}
impl std::error::Error for TablePreviewExportError {}

#[derive(Serialize)]
struct PageOutput {
    schema_version: u32,
    engine: &'static str,
    scope: &'static str,
    page_index: u32,
    svg: String,
    render_tree: PageRenderTree,
}

/// Each instance snapshots one explicitly selected table. Reopen original bytes
/// to start another session; no shared Legacy cursor, document cache or fallback.
pub struct TablePreviewExportSession {
    session: TablePreviewSession,
}

impl TablePreviewExportSession {
    /// Strict JSON: {selection:{section,paragraph,control}, dpi,
    /// pages:{width,height,body:{x,y,width,height},first_y}, max_pages}.
    /// Addresses and page indices are zero-based. Geometry is in pixels.
    /// Parsing uses the normal format detector, but never constructs DocumentCore
    /// (which would perform Legacy measurement before the V2 selection).
    pub fn from_bytes(data: &[u8], options: &str) -> Result<Self, TablePreviewExportError> {
        let request: Request = serde_json::from_str(options)
            .map_err(|e| TablePreviewExportError::Options(e.to_string()))?;
        let document = crate::parse_document(data)
            .map_err(|e| TablePreviewExportError::Parse(e.to_string()))?;
        let mut session = TablePreviewSession::from_document_with_end_policy(
            &document,
            request.selection,
            request.dpi,
            request.pages,
            request.max_pages as usize,
            request.cell_end_policy,
        )
        .map_err(TablePreviewExportError::Preview)?;
        if let Some(number) = request.first_page_number {
            session = session
                .with_first_page_number(number)
                .map_err(TablePreviewExportError::Preview)?;
        }
        Ok(Self { session })
    }

    pub fn emitted_pages(&self) -> u32 {
        // The request's u32 max_pages bounds all successful steps.
        self.session.emitted_pages() as u32
    }

    /// SVG and tree are exported from the SAME committed fragment. No second
    /// pagination or Native replacement tree. None means end, not an empty page.
    /// Failed fit/export leaves both cursor and count unchanged.
    pub fn next_page_json(&mut self) -> Result<Option<String>, TablePreviewExportError> {
        let mut candidate = self.session.clone();
        let Some(page) = candidate
            .next_page()
            .map_err(TablePreviewExportError::Preview)?
        else {
            return Ok(None);
        };
        let mut renderer = SvgRenderer::new();
        renderer.render_tree(&page.tree);
        let output = PageOutput {
            schema_version: 1,
            engine: "table_v2",
            scope: "selected_table",
            page_index: page.page_index,
            svg: renderer.output().to_owned(),
            render_tree: page.tree,
        };
        let json = serde_json::to_string(&output)
            .map_err(|e| TablePreviewExportError::Serialize(e.to_string()))?;
        self.session = candidate;
        Ok(Some(json))
    }
}
