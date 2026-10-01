//! Spaces remain source-owned inline advances, not deleted carrier text.
use super::GeometryError;
use crate::{
    model::paragraph::Paragraph,
    renderer::{
        render_tree::{BoundingBox, RenderNode, RenderNodeType, TextRunNode},
        style_resolver::{detect_lang_category, ResolvedStyleSet},
        text_measurement::{compute_char_positions, resolved_to_text_style},
    },
};

pub(super) struct SpaceRun {
    pub position: u32,
    pub source_len: u32,
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
    // A qualified saved LEFT tab already contains its resolved advance.
    // Do not resolve its stop again against replacement-font metrics.
    super::stored_text::validate_tabs(para, dpi)?;
    if !para.text.bytes().all(|c| c == b' ' || c == b'\t')
        || para.char_offsets.len() != para.text.len()
        || para.char_offsets.windows(2).any(|p| p[0] >= p[1])
        // Character offsets already use the common IR source axis. A qualified
        // leading structural prefix occupies source slots, not inline width.
        // object_rows separately validates full coverage and stored row ownership.
        || !super::tac::qualified_structural_axis(para)
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
        let is_tab = para.text.as_bytes()[i] == b'\t';
        let text = if is_tab { "\t" } else { " " };
        let mut style = resolved_to_text_style(styles, id, detect_lang_category(' '));
        if is_tab {
            style.inline_tabs = para.tab_extended.clone();
        }
        let positions = compute_char_positions(text, &style);
        let width = *positions.last().ok_or_else(invalid)?;
        if !width.is_finite() || width < 0.0 {
            return Err(invalid());
        }
        let height = style.font_size;
        runs.push(SpaceRun {
            position,
            source_len: if is_tab { 8 } else { 1 },
            width_hu: width * 7200.0 / dpi,
            node: RenderNode::new(
                0,
                RenderNodeType::TextRun(TextRunNode {
                    text: text.into(),
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
