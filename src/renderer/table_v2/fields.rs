//! Read-only replay of intact stored formula results, not formula evaluation.
//! Field begin/end slots own source positions but have no painted advance.
use super::GeometryError;
use crate::model::{
    control::{Control, FieldType},
    paragraph::Paragraph,
};

pub(super) fn stored_formula_result(para: &Paragraph) -> Result<bool, GeometryError> {
    if para.controls.is_empty()
        || !para
            .controls
            .iter()
            .all(|c| matches!(c, Control::Field(f) if f.field_type == FieldType::Formula))
    {
        return Ok(false);
    }
    let invalid = || GeometryError::Unsupported("unqualified stored formula result");
    if para.line_segs.is_empty()
        || para.stored_text_partition_is_dirty()
        || para.source_line_seg_vertical_pos.is_some()
        || para.layout_only_fill_lines != 0
        || !para.orphan_field_ends.is_empty()
        || para.hwpx_axis_shift != 0
        || para.field_ranges.len() != para.controls.len()
    {
        return Err(invalid());
    }
    let chars: Vec<_> = para.text.chars().collect();
    if chars.len() != para.char_offsets.len() {
        return Err(invalid());
    }
    let mut ranges: Vec<_> = para.field_ranges.iter().collect();
    ranges.sort_by_key(|r| (r.start_char_idx, r.end_char_idx));
    let mut seen = vec![false; para.controls.len()];
    let mut slots = vec![0u64; chars.len() + 1];
    let mut previous_end = 0;
    for range in ranges {
        if range.start_char_idx < previous_end
            || range.start_char_idx >= range.end_char_idx
            || range.end_char_idx > chars.len()
            || range.inner_slot_count != 0
            || range.control_idx >= seen.len()
            || seen[range.control_idx]
        {
            return Err(invalid());
        }
        let Control::Field(field) = &para.controls[range.control_idx] else {
            return Err(invalid());
        };
        if field.command.is_empty() || !field.memo_paragraphs.is_empty() {
            return Err(invalid());
        }
        seen[range.control_idx] = true;
        previous_end = range.end_char_idx;
        slots[range.start_char_idx] += 8;
        slots[range.end_char_idx] += 8;
    }
    // Verify both marker slots and all literal prefix/suffix characters.
    // Never rebuild offsets from a guessed display string or parse the formula.
    let mut position = 0u64;
    for (i, c) in chars.iter().enumerate() {
        position += slots[i];
        if u64::from(para.char_offsets[i]) != position {
            return Err(invalid());
        }
        position += c.len_utf16() as u64;
    }
    position += slots[chars.len()];
    if position + 1 != u64::from(para.char_count) {
        return Err(invalid());
    }
    Ok(true)
}
