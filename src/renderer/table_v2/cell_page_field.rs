//! A stored page-field line reserves its source box before its page is known.
//! Commit resolves only its displayed value through the shared paragraph painter;
//! it may not change that reservation or fall back to the saved assigned number.
use super::{text::TextComposer, GeometryError};
use crate::{
    model::{
        control::{AutoNumberType, Control},
        paragraph::Paragraph,
    },
    renderer::{
        render_tree::{RenderNode, RenderNodeType},
        style_resolver::ResolvedStyleSet,
    },
};
use std::{cell::RefCell, sync::Arc};

pub(super) fn qualify(p: &Paragraph) -> Result<bool, GeometryError> {
    if !p
        .controls
        .iter()
        .any(|c| matches!(c, Control::AutoNumber(_)))
    {
        return Ok(false);
    }
    let fail = || {
        GeometryError::Unsupported("cell page field requires one intact stored placeholder line")
    };
    let [Control::AutoNumber(n)] = p.controls.as_slice() else {
        return Err(fail());
    };
    if n.number_type != AutoNumberType::Page
        || n.format != 0
        || n.superscript
        || [n.user_symbol, n.prefix_char, n.suffix_char]
            .iter()
            .any(|c| *c != '\0')
        || p.text != " "
        || p.line_segs.len() != 1
        || p.stored_text_partition_is_dirty()
        || p.char_shapes.len() != 1
    {
        return Err(fail());
    }
    Ok(true)
}

pub(super) struct PageField {
    source: Paragraph,
    styles: Arc<ResolvedStyleSet>,
    width: f64,
    dpi: f64,
}

impl PageField {
    pub fn new(source: &Paragraph, styles: &ResolvedStyleSet, width: f64, dpi: f64) -> Self {
        Self {
            source: source.clone(),
            styles: Arc::new(styles.clone()),
            width,
            dpi,
        }
    }

    pub fn render(
        &self,
        number: Option<u32>,
        reserved: &RenderNode,
    ) -> Result<RenderNode, GeometryError> {
        let number = number
            .filter(|n| *n > 0 && *n <= u32::from(u16::MAX))
            .ok_or(GeometryError::Unsupported(
                "cell page field requires resolved printed page number",
            ))?;
        let composer = TextComposer {
            styles: &self.styles,
            dpi: self.dpi,
            payloads: RefCell::new(Vec::new()),
        };
        composer.compose_page_field(&self.source, self.width, Some(number))?;
        let mut rows = composer
            .payloads
            .into_inner()
            .pop()
            .ok_or(GeometryError::InconsistentAtomicPlan)?;
        if rows.len() != 1 {
            return Err(GeometryError::InconsistentAtomicPlan);
        }
        let row = rows.remove(0);
        let a = row.bbox;
        let b = reserved.bbox;
        if [a.x - b.x, a.y - b.y, a.width - b.width, a.height - b.height]
            .iter()
            .any(|v| v.abs() > 1e-7)
        {
            return Err(GeometryError::Unsupported(
                "page field changed reserved line box",
            ));
        }
        // The field display, not the blank source placeholder, owns ink width.
        if row.children.len() != 1 {
            return Err(GeometryError::InconsistentAtomicPlan);
        }
        let display = number.to_string();
        for child in &row.children {
            let RenderNodeType::TextRun(run) = &child.node_type else {
                return Err(GeometryError::InconsistentAtomicPlan);
            };
            if run.display_text.as_deref() != Some(display.as_str())
                || child.bbox.x < a.x - 1e-7
                || child.bbox.x + child.bbox.width > a.x + a.width + 1e-7
            {
                return Err(GeometryError::Unsupported(
                    "page field display outside reserved line",
                ));
            }
        }
        Ok(row)
    }
}
