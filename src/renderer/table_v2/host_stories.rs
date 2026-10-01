//! Document-owned page declarations. Selection is shared across sections;
//! geometry is composed at the destination page width by V2, never by Legacy HF.
use std::cell::RefCell;

use super::{text::TextComposer, GeometryError, HostedTableError, ParagraphItem};
use crate::{
    model::{
        control::{Control, PageNumberPos},
        document::Document,
        paragraph::Paragraph,
    },
    renderer::{
        page_layout::LayoutRect,
        pagination::{ActiveHeaderFooter, HeaderFooterRef},
        render_tree::{
            BoundingBox, HeaderFooterImageRef, HeaderFooterKind, RenderNode, RenderNodeType,
        },
        style_resolver::ResolvedStyleSet,
    },
};

#[derive(Default)]
pub(crate) struct StoryContext {
    pub header_footer: ActiveHeaderFooter,
    pub page_number: Option<PageNumberPos>,
}

/// Declarations follow their own accepted line, not the paragraph's last page.
pub(super) fn owner_line(p: &Paragraph, control: usize) -> Result<usize, GeometryError> {
    let pos = *p
        .control_utf16_positions()
        .get(control)
        .ok_or(GeometryError::InconsistentAtomicPlan)?;
    if p.line_segs.is_empty() || p.stored_text_partition_is_dirty() {
        // Fresh declaration placement after text requires composed source spans.
        // Entry declarations have no preceding text and unambiguously own line 0.
        if p.char_offsets.first().is_some_and(|first| pos < *first)
            || (p.text.is_empty() && pos == (control * 8) as u32)
        {
            return Ok(0);
        }
        return Err(GeometryError::Unsupported(
            "fresh page story within paragraph content",
        ));
    }
    (0..p.line_segs.len())
        .rfind(|&i| p.line_seg_text_start(i) <= pos)
        .ok_or(GeometryError::Unsupported("page story has no owner line"))
}

pub(super) fn render(
    source: &Document,
    reference: &HeaderFooterRef,
    header: bool,
    area: LayoutRect,
    number: u32,
    styles: &ResolvedStyleSet,
    dpi: f64,
) -> Result<RenderNode, HostedTableError> {
    let control = source
        .sections
        .get(reference.source_section_index)
        .and_then(|s| {
            crate::renderer::pagination::resolve_header_footer_control(&s.paragraphs, reference)
        })
        .ok_or(GeometryError::InconsistentAtomicPlan)?;
    let (paragraphs, attr, band_height) = match control {
        Control::Header(h) if header => (&h.paragraphs, h.list_attr, h.text_height),
        Control::Footer(f) if !header => (&f.paragraphs, f.list_attr, f.text_height),
        _ => return Err(GeometryError::InconsistentAtomicPlan.into()),
    };
    // Direction/wrapping and reserved alignment require an independent story rule.
    if attr & (0x1f << 16) != 0 || (attr >> 21) & 3 == 3 {
        return Err(GeometryError::Unsupported("header/footer direction or wrap policy").into());
    }
    let mut children = Vec::new();
    let mut next_origin = 0.;
    let mut occupied: f64 = 0.;
    for (pi, p) in paragraphs.iter().enumerate() {
        // Text edits invalidate saved partitions. Compose fresh geometry from
        // the current IR instead of admitting the command-side reflow cache as
        // an intact saved line box. The source document is never rewritten here.
        let fresh;
        let p = if p.stored_text_partition_is_dirty() {
            fresh = {
                let mut paragraph = p.clone();
                paragraph.line_segs.clear();
                paragraph
            };
            &fresh
        } else {
            p
        };
        super::char_border::validate_source(p, &source.doc_info)?;
        super::decoration::validate_paragraph_source(p.para_shape_id, &source.doc_info)?;
        if p.column_type != crate::model::paragraph::ColumnBreakType::None
            || !super::body_text::frame_starts(p)?.is_empty()
        {
            return Err(GeometryError::Unsupported("header/footer frame break").into());
        }
        let composer = TextComposer {
            styles,
            dpi,
            payloads: RefCell::new(Vec::new()),
        };
        let items = if p.controls.is_empty() {
            composer.compose_host(p, area.width)?
        } else if super::cell_page_field::qualify_story(p)? {
            composer.compose_page_field(p, area.width, Some(number))?
        } else {
            return Err(GeometryError::Unsupported(
                "header/footer content requires V2 story composition",
            )
            .into());
        };
        let end = items
            .iter()
            .find_map(|item| match item {
                ParagraphItem::End(end) => Some(end),
                _ => None,
            })
            .ok_or(GeometryError::InconsistentAtomicPlan)?;
        occupied = occupied.max(next_origin + end.occupied_end());
        let mut nodes = composer
            .payloads
            .into_inner()
            .pop()
            .ok_or(GeometryError::InconsistentAtomicPlan)?;
        for node in &mut nodes {
            super::text::translate(node, 0., next_origin);
            bind(node, reference, header, pi);
        }
        children.extend(nodes);
        next_origin += end.next_origin();
    }
    // Declared band alignment consumes exactly the composed occupied end, not
    // max line height per paragraph. Overflow remains visible; no glyph clamp.
    let band = f64::from(band_height) * dpi / 7200.;
    let slack = if band_height == 0 {
        0.
    } else {
        (band - occupied).max(0.)
    };
    let y = area.y
        + match (attr >> 21) & 3 {
            1 => slack / 2.,
            2 => slack,
            _ => 0.,
        };
    let mut node = RenderNode::new(
        0,
        if header {
            RenderNodeType::Header
        } else {
            RenderNodeType::Footer
        },
        BoundingBox::new(
            area.x,
            area.y,
            area.width,
            area.height.max(y - area.y + occupied),
        ),
    );
    bind(&mut node, reference, header, 0);
    for child in &mut children {
        super::text::translate(child, area.x, y);
    }
    node.children = children;
    Ok(node)
}

fn bind(node: &mut RenderNode, reference: &HeaderFooterRef, header: bool, paragraph: usize) {
    // Existing product cursor/selection queries reserve this paragraph namespace
    // for HF content; public RenderTree JSON decodes it with header_footer_source.
    // It is an address convention, not Legacy composition or placement.
    let marker = usize::MAX - paragraph;
    node.header_footer_source = Some((
        reference.source_section_index,
        HeaderFooterImageRef {
            outer_para_index: reference.para_index,
            outer_control_index: reference.control_index,
            kind: if header {
                HeaderFooterKind::Header
            } else {
                HeaderFooterKind::Footer
            },
        },
    ));
    match &mut node.node_type {
        RenderNodeType::TextLine(line) => {
            line.section_index = Some(reference.source_section_index);
            line.para_index = Some(marker);
        }
        RenderNodeType::TextRun(run) => {
            run.section_index = Some(reference.source_section_index);
            run.para_index = Some(marker);
        }
        _ => {}
    }
    for child in &mut node.children {
        bind(child, reference, header, paragraph);
    }
}
