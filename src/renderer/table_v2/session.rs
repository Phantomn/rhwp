//! Isolated, resumable V2 table preview. Not document/body pagination, and not a
//! DocumentCore engine switch. No Legacy measurements, caches or fallback.
use crate::{
    model::{control::Control, document::Document},
    renderer::{render_tree::PageRenderTree, style_resolver::resolve_styles_for_document},
};

use super::{
    GeometryError, PageArea, PreparedTextTable, Rect, TextFragment, TextFragmentFit,
    TextTableCursor,
};

/// Root table selection; descendants remain owned by the prepared table plan.
/// This address is not a cursor/hit-test binding or a document anchor position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableSelection {
    pub section: usize,
    pub paragraph: usize,
    pub control: usize,
}

/// Caller-resolved preview page geometry, in pixels. The first page can have a
/// partly occupied body. Following pages use the full body, at the same width.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TablePreviewPages {
    pub width: f64,
    pub height: f64,
    pub body: Rect,
    pub first_y: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TablePreviewError {
    Geometry(GeometryError),
    InvalidSelection(TableSelection),
    InvalidPages,
    PageLimit {
        limit: usize,
    },
    PageIndexOverflow,
    DoesNotFit {
        page_index: u32,
        available: Rect,
        required_width: f64,
        required_height: f64,
    },
}

impl From<GeometryError> for TablePreviewError {
    fn from(error: GeometryError) -> Self {
        Self::Geometry(error)
    }
}

impl std::fmt::Display for TablePreviewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "table_v2 preview: {self:?}")
    }
}
impl std::error::Error for TablePreviewError {}

/// One committed table fragment and its exact output. Page index is physical in
/// this preview, not a source-document printed page/section number.
pub struct TablePreviewPage {
    pub page_index: u32,
    pub fragment: TextFragment,
    pub tree: PageRenderTree,
}

/// Each successful step owns a fresh page; errors leave cursor/index unchanged.
/// Sessions cannot change engine, width, or DPI midway. Preparing again from the
/// original Document is explicit; editing a Document never changes this snapshot.
#[derive(Clone)]
pub struct TablePreviewSession {
    cursor: TextTableCursor,
    pages: TablePreviewPages,
    next_page_index: u64,
    emitted_pages: usize,
    max_pages: usize,
    selection: Option<TableSelection>,
}

impl TablePreviewPages {
    fn validate(self) -> Result<(), TablePreviewError> {
        let b = self.body;
        if [
            self.width,
            self.height,
            b.x,
            b.y,
            b.width,
            b.height,
            self.first_y,
            b.x + b.width,
            b.y + b.height,
        ]
        .iter()
        .any(|v| !v.is_finite())
            || self.width <= 0.0
            || self.height <= 0.0
            || b.x < 0.0
            || b.y < 0.0
            || b.width <= 0.0
            || b.height <= 0.0
            || b.x + b.width > self.width
            || b.y + b.height > self.height
            || self.first_y < b.y
            || self.first_y > b.y + b.height
        {
            return Err(TablePreviewError::InvalidPages);
        }
        Ok(())
    }

    fn area(self, index: u64) -> PageArea {
        let y = if index == 0 {
            self.first_y
        } else {
            self.body.y
        };
        PageArea {
            bounds: Rect {
                y,
                height: self.body.y + self.body.height - y,
                ..self.body
            },
        }
    }
}

impl TablePreviewSession {
    /// Preview a prepared table, including explicitly composed nested flow.
    pub fn new(
        prepared: &PreparedTextTable,
        pages: TablePreviewPages,
        max_pages: usize,
    ) -> Result<Self, TablePreviewError> {
        pages.validate()?;
        if max_pages == 0 {
            return Err(TablePreviewError::PageLimit { limit: 0 });
        }
        Ok(Self {
            cursor: prepared.start(),
            pages,
            next_page_index: 0,
            emitted_pages: 0,
            max_pages,
            selection: None,
        })
    }

    /// Resolve source styles and snapshot one selected table's *contents*.
    /// The caller supplies a preview viewport: the outer host paragraph/anchor,
    /// section headers/footers/columns and document pagination are not rendered.
    /// IR admission includes fresh text and supported zero-offset TopAndBottom
    /// children. Saved rows/unsupported controls produce an error, not a cache
    /// purge, silent reflow, or fallback to Legacy.
    pub fn from_document(
        document: &Document,
        selection: TableSelection,
        dpi: f64,
        pages: TablePreviewPages,
        max_pages: usize,
    ) -> Result<Self, TablePreviewError> {
        pages.validate()?;
        super::contracts::finite(dpi, "preview DPI")?;
        if dpi <= 0.0 {
            return Err(GeometryError::InvalidNumber("preview DPI").into());
        }
        let control = document
            .sections
            .get(selection.section)
            .and_then(|s| s.paragraphs.get(selection.paragraph))
            .and_then(|p| p.controls.get(selection.control));
        let Some(Control::Table(table)) = control else {
            return Err(TablePreviewError::InvalidSelection(selection));
        };
        super::decoration::validate_source(table, &document.doc_info)?;
        let styles = resolve_styles_for_document(document, dpi);
        let prepared = PreparedTextTable::prepare_with_resources(
            table,
            &styles,
            dpi,
            &document.bin_data_content,
        )?;
        let mut session = Self::new(&prepared, pages, max_pages)?;
        session.selection = Some(selection);
        Ok(session)
    }

    pub fn selection(&self) -> Option<TableSelection> {
        self.selection
    }
    pub fn emitted_pages(&self) -> usize {
        self.emitted_pages
    }

    /// Step one page. Retry non-fit only from the partially occupied first page
    /// to one fresh page. An oversized unit is returned to the caller, never
    /// retried forever or painted outside the body. A skipped first page is not
    /// fabricated as an empty output page; the returned physical index records it.
    pub fn next_page(&mut self) -> Result<Option<TablePreviewPage>, TablePreviewError> {
        if self.cursor.is_complete() {
            return Ok(None);
        }
        if self.emitted_pages == self.max_pages {
            return Err(TablePreviewError::PageLimit {
                limit: self.max_pages,
            });
        }
        let mut index = self.next_page_index;
        loop {
            let page_index =
                u32::try_from(index).map_err(|_| TablePreviewError::PageIndexOverflow)?;
            let area = self.pages.area(index);
            match self.cursor.fit(area)? {
                TextFragmentFit::Placed(fragment) => {
                    let mut tree =
                        PageRenderTree::new(page_index, self.pages.width, self.pages.height);
                    fragment.append_to(&mut tree)?;
                    // Commit only after paint succeeded. No partially built page
                    // or consumed geometry escapes on an error.
                    self.cursor = fragment.continuation();
                    self.next_page_index = index + 1;
                    self.emitted_pages += 1;
                    return Ok(Some(TablePreviewPage {
                        page_index,
                        fragment,
                        tree,
                    }));
                }
                TextFragmentFit::DoesNotFit {
                    required_width,
                    required_height,
                } => {
                    if index == 0
                        && self.pages.first_y > self.pages.body.y
                        && required_width <= area.bounds.width
                    {
                        index = 1;
                        continue;
                    }
                    return Err(TablePreviewError::DoesNotFit {
                        page_index,
                        available: area.bounds,
                        required_width,
                        required_height,
                    });
                }
                TextFragmentFit::Complete => return Ok(None),
            }
        }
    }
}
