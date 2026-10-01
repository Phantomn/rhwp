//! Page/column ownership of already accepted V2 fragments. No table cuts,
//! remeasurement, saved-position snapping or Legacy compatibility state.
use std::collections::HashSet;

use crate::{
    model::page::{ColumnDef, PageDef},
    renderer::{
        page_layout::PageLayoutInfo,
        pagination::{ColumnContent, PageContent, PageItem, PaginationResult},
    },
};

pub(super) struct PageFlow {
    pub layout: PageLayoutInfo,
    pub section_index: usize,
    pub pages: Vec<PageContent>,
    pub current_column: u16,
    pub current_height: f64,
    pub current_items: Vec<PageItem>,
    pub hide_empty_line: bool,
    hidden_empty_paras: HashSet<usize>,
}

impl PageFlow {
    pub fn new(page: &PageDef, columns: &ColumnDef, section: usize, hide: bool, dpi: f64) -> Self {
        let mut state = Self {
            layout: PageLayoutInfo::from_page_def(page, columns, dpi),
            section_index: section,
            pages: Vec::new(),
            current_column: 0,
            current_height: 0.0,
            current_items: Vec::new(),
            hide_empty_line: hide,
            hidden_empty_paras: HashSet::new(),
        };
        state.push_page();
        state
    }

    pub fn available_height(&self) -> f64 {
        self.layout.available_body_height()
    }

    pub fn hide_empty_paragraph(&mut self, index: usize) {
        self.hidden_empty_paras.insert(index);
    }

    pub fn append_item(&mut self, item: PageItem) {
        self.current_items.push(item);
    }

    pub fn align_flow_to(&mut self, height: f64) {
        self.current_height = height;
    }

    fn flush_column(&mut self) {
        if self.current_items.is_empty() {
            return;
        }
        let origin = self.layout.column_areas[usize::from(self.current_column)].y;
        // A side-wrapping object occupies a physical band without advancing
        // the prose pen. Preserve both, rather than inferring height in paint.
        let used_height = self
            .current_items
            .iter()
            .fold(self.current_height, |height, item| {
                let bounds = match item {
                    PageItem::HostedParagraph { fragment, .. } => fragment.occupied(),
                    PageItem::HostedTable { fragment, .. } => fragment.occupied(),
                    _ => unreachable!("only accepted V2 packets enter the V2 page owner"),
                };
                height.max(bounds.y + bounds.height - origin)
            });
        self.pages
            .last_mut()
            .expect("page initialized")
            .column_contents
            .push(ColumnContent {
                column_index: self.current_column,
                start_height: 0.0,
                endnote_flow: false,
                items: std::mem::take(&mut self.current_items),
                used_height,
                zone_layout: None,
                zone_y_offset: 0.0,
                wrap_around_paras: Vec::new(),
                wrap_anchors: Default::default(),
                overlay_continuations: Vec::new(),
                overlay_cuts: Vec::new(),
                inline_placements: Default::default(),
                inline_flow_plans: Default::default(),
                paragraph_float_placements: Default::default(),
            });
    }

    pub fn advance_column_or_new_page(&mut self) {
        self.flush_column();
        if usize::from(self.current_column) + 1 < self.layout.column_areas.len() {
            self.current_column += 1;
            self.current_height = 0.0;
        } else {
            self.push_page();
        }
    }

    pub fn force_new_page(&mut self) {
        self.flush_column();
        self.push_page();
    }

    fn push_page(&mut self) {
        self.layout
            .apply_column_page_number(self.pages.len() as u32 + 1);
        self.pages.push(PageContent {
            page_index: self.pages.len() as u32,
            page_number: 0,
            page_number_restarted: false,
            section_index: self.section_index,
            layout: self.layout.clone(),
            column_contents: Vec::new(),
            active_header: None,
            active_footer: None,
            page_number_pos: None,
            page_hide: None,
            footnotes: Vec::new(),
            active_master_page: None,
            extra_master_pages: Vec::new(),
            ladder_band_tables: Vec::new(),
        });
        self.current_column = 0;
        self.current_height = 0.0;
    }

    pub fn finish(mut self) -> PaginationResult {
        self.flush_column();
        // Page stories are finalized by host_section from committed source
        // lines. Never discard authored blank packets at section termination.
        PaginationResult {
            pages: self.pages,
            hidden_empty_paras: self.hidden_empty_paras,
            wrap_around_paras: Vec::new(),
            pre_emitted_host_paras: Default::default(),
            pre_emitted_host_heights: Default::default(),
            endnotes: Vec::new(),
            endnote_paragraphs: Vec::new(),
            endnote_para_sources: Vec::new(),
            endnote_between_notes_hu: 0,
            endnote_separator_above_hu: 0,
            endnote_separator_below_hu: 0,
        }
    }
}
