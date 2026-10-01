//! Product engine/result boundary. V2 prepares from the authoritative IR and
//! publishes one complete geometry generation; it never retries with Legacy.
use super::DocumentCore;
use crate::{error::HwpError, renderer::table_v2::HostedDocumentLayout};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TypesettingEngine {
    #[default]
    Legacy,
    V2,
}

impl DocumentCore {
    pub fn typesetting_engine(&self) -> TypesettingEngine {
        self.typesetting_engine
    }

    /// Failed/pending V2 preparation is not a successful empty Legacy page.
    pub fn ensure_typesetting_ready(&self) -> Result<(), HwpError> {
        if self.typesetting_engine == TypesettingEngine::Legacy {
            return Ok(());
        }
        match &self.v2_layout {
            Some(Ok(_)) => Ok(()),
            Some(Err(message)) => Err(HwpError::RenderError(message.clone())),
            None => Err(HwpError::RenderError(
                "V2: layout is not prepared for this document state".into(),
            )),
        }
    }

    pub(crate) fn paginate_v2(&mut self) {
        // document_mut/set_document may also replace DocInfo. Derive the style
        // generation from the current IR and font environment before fitting.
        self.rebuild_resolved_styles();
        self.invalidate_page_tree_cache();
        self.pending_pagination_job = None;
        self.deferred_pagination_descriptor = None;
        self.pagination.clear();
        let result = HostedDocumentLayout::prepare(&self.document, self.dpi, &self.styles);
        if let Ok(layout) = &result {
            self.pagination = layout.pagination();
            self.dirty_sections.fill(false);
        }
        self.v2_layout = Some(result.map_err(|e| e.to_string()));
    }

    pub(crate) fn invalidate_v2_layout(&mut self) {
        if self.typesetting_engine == TypesettingEngine::V2 {
            self.v2_layout = None;
            self.pagination.clear();
            self.invalidate_page_tree_cache();
        }
    }
}
