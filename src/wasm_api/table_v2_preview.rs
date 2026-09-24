//! Explicit opt-in experimental preview, isolated from HwpDocument/Studio.
use wasm_bindgen::prelude::*;

use crate::renderer::table_v2::TablePreviewExportSession;

/// Borderless selected-table preview. Unsupported V2 input throws; no automatic
/// Legacy fallback. This does NOT change the engine of an existing HwpDocument.
#[wasm_bindgen]
pub struct TableV2Preview {
    inner: TablePreviewExportSession,
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
