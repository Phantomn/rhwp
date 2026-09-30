//! Explicit opt-in experimental preview, isolated from HwpDocument/Studio.
use wasm_bindgen::prelude::*;

use crate::renderer::table_v2::{DocumentV2Session, TablePreviewExportSession};

/// Opt-in host integration, distinct from standalone DocumentV2 and Studio.
#[wasm_bindgen]
pub struct HostedSectionV2 {
    inner: crate::renderer::table_v2::HostedSectionSession,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HostedOptions {
    section: usize,
    dpi: f64,
    #[serde(default)]
    cell_end_policy: crate::renderer::table_v2::CellEndPolicy,
}

#[wasm_bindgen]
impl HostedSectionV2 {
    #[wasm_bindgen(constructor)]
    pub fn new(data: &[u8], options_json: &str) -> Result<Self, JsValue> {
        let options: HostedOptions =
            serde_json::from_str(options_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let source = crate::parse_document(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let inner = crate::renderer::table_v2::HostedSectionSession::from_document(
            &source,
            options.section,
            options.dpi,
            options.cell_end_policy,
        )
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(Self { inner })
    }

    #[wasm_bindgen(js_name = pageCount)]
    pub fn page_count(&self) -> usize {
        self.inner.pagination().pages.len()
    }

    #[wasm_bindgen(js_name = renderPage)]
    pub fn render_page(&self, index: usize) -> Result<String, JsValue> {
        self.inner
            .render_page_json(index)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

/// Borderless selected-table preview. Unsupported V2 input throws; no automatic
/// Legacy fallback. This does NOT change the engine of an existing HwpDocument.
#[wasm_bindgen]
pub struct TableV2Preview {
    inner: TablePreviewExportSession,
}

/// Explicit experimental document-body path with source PageDef geometry.
/// Does not replace HwpDocument or silently reuse Legacy table pagination.
#[wasm_bindgen]
pub struct DocumentV2 {
    inner: DocumentV2Session,
}

#[wasm_bindgen]
impl DocumentV2 {
    #[wasm_bindgen(constructor)]
    pub fn new(data: &[u8], options_json: &str) -> Result<Self, JsValue> {
        DocumentV2Session::from_bytes(data, options_json)
            .map(|inner| Self { inner })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = nextPage)]
    pub fn next_page(&mut self) -> Result<Option<String>, JsValue> {
        self.inner
            .next_page_json()
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = emittedPages)]
    pub fn emitted_pages(&self) -> u32 {
        self.inner.emitted_pages()
    }
}

#[wasm_bindgen]
impl TableV2Preview {
    /// Parse a private snapshot. See TablePreviewExportSession for options JSON.
    #[wasm_bindgen(constructor)]
    pub fn new(data: &[u8], options_json: &str) -> Result<Self, JsValue> {
        TablePreviewExportSession::from_bytes(data, options_json)
            .map(|inner| Self { inner })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// One JSON envelope with SVG and actual WASM render tree; undefined at end.
    #[wasm_bindgen(js_name = nextPage)]
    pub fn next_page(&mut self) -> Result<Option<String>, JsValue> {
        self.inner
            .next_page_json()
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = emittedPages)]
    pub fn emitted_pages(&self) -> u32 {
        self.inner.emitted_pages()
    }
}
