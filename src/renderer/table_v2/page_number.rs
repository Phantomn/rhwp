//! Page stories do not own body space or advance the table/paragraph cursor.
//! This boundary admits one first-paragraph decimal footer declaration only.
use crate::{
    model::{control::PageNumberPos, document::SectionDef},
    renderer::{
        layout::{estimate_text_width, format_page_number},
        page_layout::PageLayoutInfo,
        render_tree::{
            BoundingBox, FieldMarkerType, RenderNode, RenderNodeType, TextLineNode, TextRunNode,
        },
        TextStyle,
    },
};

use super::GeometryError;

pub(super) struct PageNumberStory {
    declaration: PageNumberPos,
    layout: PageLayoutInfo,
    first: u32,
}

impl PageNumberStory {
    pub fn new(
        declaration: &PageNumberPos,
        section: &SectionDef,
        layout: &PageLayoutInfo,
    ) -> Result<Self, GeometryError> {
        if declaration.format != 0
            || !matches!(declaration.position, 0 | 4..=6)
            || section.page_num_type != 0
            || [
                declaration.prefix_char,
                declaration.suffix_char,
                declaration.dash_char,
            ]
            .iter()
            .any(|c| *c != '\0' && c.is_control())
        {
            return Err(GeometryError::Unsupported("page-number format or position"));
        }
        Ok(Self {
            declaration: declaration.clone(),
            layout: layout.clone(),
            first: u32::from(section.page_num).max(1),
        })
    }

    pub fn render(&self, page_index: u32) -> Result<Option<RenderNode>, GeometryError> {
        if self.declaration.position == 0 {
            return Ok(None);
        }
        let number = self
            .first
            .checked_add(page_index)
            .filter(|n| *n <= u32::from(u16::MAX))
            .ok_or(GeometryError::Unsupported("page-number range"))?;
        let p = &self.declaration;
        let text = format_page_number(number, p.format, p.prefix_char, p.suffix_char, p.dash_char);
        // 10pt is the observed automatic page-number size, not the host's style.
        // Use the same style for measuring and painting; no character-count width.
        let size = 10.0 * self.layout.dpi / 72.0;
        let style = TextStyle {
            font_family: "바탕".into(),
            font_size: size,
            color: 0,
            ..Default::default()
        };
        let width = estimate_text_width(&text, &style);
        let area = self.layout.footer_area;
        let free = area.width - width;
        if !free.is_finite() || free < 0.0 {
            return Err(GeometryError::Unsupported("page-number width"));
        }
        let x = area.x
            + match p.position {
                4 => 0.0,
                5 => free / 2.0,
                6 => free,
                _ => unreachable!(),
            };
        // The no-border footer convention is body-bottom + footer distance/2
        // + one-third em. The run baseline is one em below this top. Keep this
        // outside body fit, with the line bbox enclosing the actual run (unlike
        // a baseline interpreted a second time as a line top).
        let footer_distance = self.layout.page_height - (area.y + area.height);
        let top = area.y + footer_distance / 2.0 + size / 3.0;
        let height = size * 1.2;
        if !top.is_finite() || top < 0.0 || top + height > self.layout.page_height {
            return Err(GeometryError::Unsupported("page-number outside page"));
        }
        let mut line = RenderNode::new(
            0,
            RenderNodeType::TextLine(TextLineNode::new(height, size)),
            BoundingBox::new(x, top, width, height),
        );
        line.children.push(RenderNode::new(
            0,
            RenderNodeType::TextRun(TextRunNode {
                text,
                style,
                char_shape_id: None,
                para_shape_id: None,
                section_index: None,
                para_index: None,
                char_start: None,
                cell_context: None,
                is_para_end: true,
                is_line_break_end: false,
                rotation: 0.0,
                is_vertical: false,
                char_overlap: None,
                border_fill_id: 0,
                baseline: size,
                field_marker: FieldMarkerType::None,
                layout_positions: None,
                display_text: None,
            }),
            BoundingBox::new(x, top, width, size),
        ));
        Ok(Some(line))
    }
}
