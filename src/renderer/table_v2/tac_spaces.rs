//! Spaces remain source-owned inline advances, not deleted carrier text.
use super::GeometryError;
use crate::{
    model::paragraph::Paragraph,
    renderer::{
        layout::{compute_char_positions, resolved_to_text_style},
        render_tree::{BoundingBox, RenderNode, RenderNodeType, TextRunNode},
        style_resolver::{detect_lang_category, ResolvedStyleSet},
    },
};

pub(super) struct SpaceRun {
    pub position: u32,
    pub width_hu: f64,
    pub node: RenderNode,
}

pub(super) fn compose(
    para: &Paragraph,
    styles: &ResolvedStyleSet,
    dpi: f64,
) -> Result<Vec<SpaceRun>, GeometryError> {
    let invalid =
        || GeometryError::Unsupported("stored TAC mixed text requires qualified space advances");
    if para.text.is_empty() {
        return Ok(Vec::new());
    }
    // Tabs, NBSP and visible text have different breaking/painting contracts.
    if !para.text.bytes().all(|c| c == b' ')
        || para.char_offsets.len() != para.text.len()
        || para.char_offsets.windows(2).any(|p| p[0] >= p[1])
        || para.hwpx_axis_shift != 0
    {
        return Err(invalid());
    }
    let mut runs = Vec::new();
    for (i, &position) in para.char_offsets.iter().enumerate() {
        let id = para
            .char_shapes
            .iter()
            .rfind(|s| s.start_pos <= position)
            .ok_or_else(invalid)?
            .char_shape_id;
        let cs = styles.char_styles.get(id as usize).ok_or_else(invalid)?;
        if !cs.font_size.is_finite()
            || cs.font_size <= 0.0
            || cs.superscript
            || cs.subscript
            || cs.emphasis_dot != 0
            || !super::decoration::paragraph_is_unpainted(cs.border_fill_id, styles)
            || cs.underline != crate::model::style::UnderlineType::None
            || cs.strikethrough
            || cs.shade_color & 0x00ffffff != 0x00ffffff
        {
            return Err(invalid());
        }
        let style = resolved_to_text_style(styles, id, detect_lang_category(' '));
        let positions = compute_char_positions(" ", &style);
        let width = *positions.last().ok_or_else(invalid)?;
        if !width.is_finite() || width < 0.0 {
            return Err(invalid());
        }
        let height = style.font_size;
        runs.push(SpaceRun {
            position,
            width_hu: width * 7200.0 / dpi,
            node: RenderNode::new(
                0,
                RenderNodeType::TextRun(TextRunNode {
                    text: " ".into(),
                    style,
                    char_shape_id: Some(id),
                    para_shape_id: Some(para.para_shape_id),
                    section_index: None,
                    para_index: None,
                    char_start: Some(i),
                    cell_context: None,
                    is_para_end: false,
                    is_line_break_end: false,
                    rotation: 0.0,
                    is_vertical: false,
                    char_overlap: None,
                    border_fill_id: 0,
                    baseline: 0.0,
                    field_marker: Default::default(),
                    layout_positions: Some(positions),
                    display_text: None,
                }),
                BoundingBox::new(0.0, 0.0, width, height),
            ),
        });
    }
    Ok(runs)
}
