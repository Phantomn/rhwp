//! Read-only replay of intact formula results, filled ClickHere values and
//! literal stored text following open ClickHere prefix markers, and closed
//! hyperlink display text. Link commands are opaque and are never executed.
//! This does not evaluate fields or display initial ClickHere guide text.
//! Field begin/end slots own source positions but have no painted advance.
use super::GeometryError;
use crate::model::{
    control::{Control, FieldType},
    paragraph::Paragraph,
};

/// Identity proof for trailing closing markers in an intact body paragraph.
pub(super) struct BodyFieldEnd(());

pub(super) fn body_ends(
    paragraphs: &[Paragraph],
) -> Result<Vec<Option<BodyFieldEnd>>, (usize, GeometryError)> {
    let mut open: Vec<(usize, &crate::model::control::Field)> = Vec::new();
    let mut result = Vec::with_capacity(paragraphs.len());
    for (pi, p) in paragraphs.iter().enumerate() {
        let invalid = || (pi, GeometryError::Unsupported("unqualified body field end"));
        if p.orphan_field_ends.is_empty() {
            result.push(None);
        } else {
            // Only the trailing markers are nonprinting. Literal text (even
            // spaces) and a genuinely blank paragraph keep their saved lines.
            // A scalar boundary identifies an end; source slots use UTF-16.
            let mut source_end = 0u64;
            let chars = p.text.chars().count();
            let intact_text = p.char_offsets.len() == chars
                && p.text.chars().zip(&p.char_offsets).all(|(ch, offset)| {
                    let matches = u64::from(*offset) == source_end;
                    source_end += if ch == '\t' { 8 } else { ch.len_utf16() as u64 };
                    matches
                });
            if !intact_text
                || !p.controls.is_empty()
                || !p.field_ranges.is_empty()
                || p.line_segs.is_empty()
                || p.stored_text_partition_is_dirty()
                || u64::from(p.char_count) != source_end + p.orphan_field_ends.len() as u64 * 8 + 1
            {
                return Err(invalid());
            }
            for end in &p.orphan_field_ends {
                let Some((begin_pi, field)) = open.pop() else {
                    return Err(invalid());
                };
                if end.char_idx != chars
                    || end.begin_id_ref == 0
                    || end.begin_id_ref != field.field_id
                    || end.begin_ctrl_id != field.ctrl_id
                    || field.field_type != FieldType::ClickHere
                    || !(stored_tac_prefix(&paragraphs[begin_pi])
                        || stored_result(&paragraphs[begin_pi]).unwrap_or(false))
                    || open
                        .iter()
                        .any(|(_, other)| other.field_id == field.field_id)
                {
                    return Err(invalid());
                }
            }
            result.push(Some(BodyFieldEnd(())));
        }
        // Local closed ranges and cell stories cannot supply a body end's ID.
        // Unclosed begins remain permitted under the existing prefix contract.
        for (ci, c) in p.controls.iter().enumerate() {
            if let Control::Field(f) = c {
                if !p.field_ranges.iter().any(|r| r.control_idx == ci) {
                    open.push((pi, f));
                }
            }
        }
    }
    Ok(result)
}

/// Filled open ClickHere prefixes can precede inserted inline tables. Their
/// begin slots keep their original indices/UTF-16 positions but have no painted
/// width. This is not field evaluation or a guessed cross-paragraph end range.
/// Closed/interleaved fields and fresh streams need their own ownership contract.
pub(super) fn stored_tac_prefix(para: &Paragraph) -> bool {
    if para.line_segs.is_empty()
        || para.stored_text_partition_is_dirty()
        || !para.field_ranges.is_empty()
        || !para.orphan_field_ends.is_empty()
    {
        return false;
    }
    let mut fields = 0;
    let mut tables = 0;
    for control in &para.controls {
        match control {
            Control::SectionDef(_) | Control::ColumnDef(_) | Control::PageNumberPos(_)
                if fields == 0 && tables == 0 => {}
            Control::Field(f)
                if tables == 0
                    && f.field_type == FieldType::ClickHere
                    && f.is_dirty()
                    && !f.command.is_empty()
                    && f.memo_paragraphs.is_empty()
                    && f.guide_residue.is_none() =>
            {
                fields += 1;
            }
            Control::Table(t) if t.common.treat_as_char => {
                tables += 1;
            }
            _ => return false,
        }
    }
    fields > 0 && tables > 0
}

pub(super) fn stored_result(para: &Paragraph) -> Result<bool, GeometryError> {
    let structural = usize::from(matches!(para.controls.first(), Some(Control::ColumnDef(_))));
    let fields = &para.controls[structural..];
    if fields.is_empty()
        || !para
            .controls
            .iter()
            .skip(structural)
            .all(|c| matches!(c, Control::Field(f) if matches!(f.field_type, FieldType::Formula | FieldType::ClickHere | FieldType::Hyperlink)))
    {
        return Ok(false);
    }
    let only_formulas = fields
        .iter()
        .all(|c| matches!(c, Control::Field(f) if f.field_type == FieldType::Formula));
    let invalid = || {
        GeometryError::Unsupported(if only_formulas {
            "unqualified stored formula result"
        } else {
            "unqualified stored field result"
        })
    };
    // The cell adapter separately enforces that this lane declaration occurs
    // only at story entry. It owns eight source units, not a field or text box.
    if structural != 0 {
        super::ir::initial_cell_column(para, 0)?;
    }
    if para.line_segs.is_empty()
        || para.stored_text_partition_is_dirty()
        || para.source_line_seg_vertical_pos.is_some()
        || para.layout_only_fill_lines != 0
        || !para.orphan_field_ends.is_empty()
        || para.hwpx_axis_shift != 0
    {
        return Err(invalid());
    }
    let chars: Vec<_> = para.text.chars().collect();
    if chars.len() != para.char_offsets.len() {
        return Err(invalid());
    }
    if para.field_ranges.is_empty() {
        return open_prefix_text(para).then_some(true).ok_or_else(invalid);
    }
    if para.field_ranges.len() != fields.len() {
        return Err(invalid());
    }
    let mut ranges: Vec<_> = para.field_ranges.iter().collect();
    ranges.sort_by_key(|r| {
        (
            r.start_char_idx,
            std::cmp::Reverse(r.end_char_idx),
            r.control_idx,
        )
    });
    let mut seen = vec![false; para.controls.len()];
    let mut slots = vec![0u64; chars.len() + 1];
    slots[0] = structural as u64 * 8;
    let mut stack = Vec::<usize>::new();
    for (ordinal, range) in ranges.iter().enumerate() {
        while stack.last().is_some_and(|&end| range.start_char_idx >= end) {
            stack.pop();
        }
        // Nested ranges are legitimate source ownership, crossing ranges are
        // not. Each begin/end contributes one slot irrespective of depth.
        let descendants = ranges[ordinal + 1..]
            .iter()
            .take_while(|r| r.start_char_idx < range.end_char_idx)
            .count();
        if stack.last().is_some_and(|&end| range.end_char_idx > end)
            || range.start_char_idx >= range.end_char_idx
            || range.end_char_idx > chars.len()
            || range.inner_slot_count != descendants
            || range.control_idx != structural + ordinal
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
        // HWP field property bit15 / HWPX dirty distinguishes a filled value
        // from initial guide text. Never print a guide as if it were a value.
        // Stored-layout validity above is independent of this field state bit:
        // an edited paragraph still needs a new composition, even with dirty=1.
        if field.field_type == FieldType::ClickHere
            && (!field.is_dirty() || field.guide_residue.is_some())
        {
            return Err(invalid());
        }
        seen[range.control_idx] = true;
        stack.push(range.end_char_idx);
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

/// A normally saved document can contain begin markers with no local end or
/// attributed result range. Those markers do not own a painted box and do not
/// authorize inventing an end, evaluating a command or deleting following text.
/// Qualify only an exact prefix of ClickHere source slots followed by literal
/// text. Interleaved markers and cross-paragraph ends need a story-level contract.
fn open_prefix_text(para: &Paragraph) -> bool {
    if para.text.is_empty() {
        return false;
    }
    for control in para.controls.iter().skip(usize::from(matches!(
        para.controls.first(),
        Some(Control::ColumnDef(_))
    ))) {
        let Control::Field(field) = control else {
            return false;
        };
        if field.field_type != FieldType::ClickHere
            || field.command.is_empty()
            || !field.memo_paragraphs.is_empty()
            || field.guide_residue.is_some()
        {
            return false;
        }
        // Dirty=0 alone does not identify an initial prompt: unclosed markers
        // also occur around pasted, filled document text. Require a known guide
        // and keep an ambiguous prompt unsupported instead of printing it.
        if !field.is_dirty()
            && field
                .guide_text()
                .is_none_or(|guide| para.text.trim_end() == guide.trim_end())
        {
            return false;
        }
    }
    let mut position = para.controls.len() as u64 * 8;
    for (offset, c) in para.char_offsets.iter().zip(para.text.chars()) {
        if u64::from(*offset) != position {
            return false;
        }
        position += c.len_utf16() as u64;
    }
    position + 1 == u64::from(para.char_count)
}

/// A stored row can begin at the field marker immediately before its first
/// visible character. `char_offsets` only names visible characters; it must not
/// be used to reject a qualified eight-unit marker boundary or to accept an
/// arbitrary position inside that marker. No offsets or row cuts are rewritten.
pub(super) fn is_row_start(para: &Paragraph, position: u32) -> Result<bool, GeometryError> {
    if para.char_offsets.contains(&position) {
        return Ok(true);
    }
    if !stored_result(para)? {
        return Ok(false);
    }
    let mut slots = vec![0u32; para.char_offsets.len() + 1];
    if para.field_ranges.is_empty() {
        slots[0] = para.controls.len() as u32;
    } else {
        slots[0] = u32::from(matches!(para.controls.first(), Some(Control::ColumnDef(_))));
        for range in &para.field_ranges {
            slots[range.start_char_idx] += 1;
            slots[range.end_char_idx] += 1;
        }
    }
    // Require a following visible character. A trailing marker alone does not
    // establish a new printable row or authorize manufacturing an empty line.
    Ok(para.char_offsets.iter().zip(slots).any(|(&offset, count)| {
        position < offset && offset - position <= count * 8 && (offset - position).is_multiple_of(8)
    }))
}
