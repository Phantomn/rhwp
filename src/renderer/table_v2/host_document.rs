//! Document host for V2 geometry. Owns no IR snapshot: DocumentCore remains the
//! single source owner. Admission and preparation finish before publication.
use super::{host_section::HostedSectionLayout, CellEndPolicy};
use crate::{
    error::HwpError,
    model::{control::Control, document::Document, paragraph::Paragraph},
    renderer::{pagination::PaginationResult, render_tree::PageRenderTree},
};

pub(crate) struct HostedDocumentLayout {
    sections: Vec<HostedSectionLayout>,
}

impl HostedDocumentLayout {
    pub(crate) fn prepare(
        source: &Document,
        dpi: f64,
        styles: &crate::renderer::style_resolver::ResolvedStyleSet,
    ) -> Result<Self, HwpError> {
        if source.sections.is_empty() {
            return Err(HwpError::RenderError("V2: document has no section".into()));
        }
        let mut sections = Vec::with_capacity(source.sections.len());
        let mut page_offset = 0u32;
        let mut next_number = 1u32;
        let mut stories = super::host_stories::StoryContext::default();
        for (si, section) in source.sections.iter().enumerate() {
            // These stories can inherit across section boundaries. A section
            // preview's empty carry is not a document-level implementation.
            if source.sections.len() > 1 {
                if !section.section_def.master_pages.is_empty() {
                    return Err(failure(si, "master-page inheritance is not supported"));
                }
                for (pi, para) in section.paragraphs.iter().enumerate() {
                    if has_cross_section_story(para) {
                        return Err(failure(
                            si,
                            format!("paragraph {pi}: page story/note inheritance is not supported"),
                        ));
                    }
                }
            }
            let first_number = match section.section_def.page_num {
                0 => next_number,
                number => u32::from(number),
            };
            let layout = HostedSectionLayout::prepare(
                source,
                si,
                dpi,
                CellEndPolicy::OmitFinalParagraphGap,
                page_offset,
                first_number,
                Some(styles),
                &mut stories,
            )
            .map_err(|e| failure(si, e))?;
            let count = u32::try_from(layout.pagination().pages.len())
                .map_err(|_| failure(si, "page range"))?;
            page_offset = page_offset
                .checked_add(count)
                .ok_or_else(|| failure(si, "page range"))?;
            next_number = first_number
                .checked_add(count)
                .ok_or_else(|| failure(si, "page-number range"))?;
            sections.push(layout);
        }
        Ok(Self { sections })
    }

    pub(crate) fn pagination(&self) -> Vec<PaginationResult> {
        // Page fragments are Arc-owned immutable compositions, shared with paint.
        self.sections
            .iter()
            .map(|s| s.pagination().clone())
            .collect()
    }

    pub(crate) fn render_page(
        &self,
        source: &Document,
        index: u32,
    ) -> Result<PageRenderTree, HwpError> {
        for (si, section) in self.sections.iter().enumerate() {
            if let Some(local) = section
                .pagination()
                .pages
                .iter()
                .position(|p| p.page_index == index)
            {
                return section
                    .render_page(source, local)
                    .map_err(|e| failure(si, e));
            }
        }
        Err(HwpError::PageOutOfRange(index))
    }

    pub(crate) fn render_story_preview(
        &self,
        source: &Document,
        index: u32,
        reference: &crate::renderer::pagination::HeaderFooterRef,
        header: bool,
    ) -> Result<PageRenderTree, HwpError> {
        for (si, section) in self.sections.iter().enumerate() {
            if let Some(local) = section
                .pagination()
                .pages
                .iter()
                .position(|p| p.page_index == index)
            {
                return section
                    .render_story_preview(source, local, reference, header)
                    .map_err(|e| failure(si, e));
            }
        }
        Err(HwpError::PageOutOfRange(index))
    }
}

fn failure(section: usize, reason: impl std::fmt::Display) -> HwpError {
    HwpError::RenderError(format!("V2 section {section}: {reason}"))
}

fn has_cross_section_story(para: &Paragraph) -> bool {
    para.controls.iter().any(|c| match c {
        Control::Footnote(_) | Control::Endnote(_) | Control::NewNumber(_) => true,
        Control::Table(table) => table
            .cells
            .iter()
            .flat_map(|c| &c.paragraphs)
            .any(has_cross_section_story),
        _ => false,
    })
}
