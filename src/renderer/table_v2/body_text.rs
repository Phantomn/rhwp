//! Stored body frames are owned by the document, not by paragraph-local paint.
use crate::model::{control::Control, paragraph::Paragraph};

use super::{FlowBlock, GeometryError};

/// Qualify page-local origins before lifting them for shared composition.
/// The caller has established uniform frames and owns their progression (one
/// column for standalone, normal equal-width columns for the shared host).
/// A decreasing origin returning to zero starts its next body frame;
/// overlapping line bottoms alone do not. Nonzero resets remain unsupported.
pub(super) fn frame_starts(para: &Paragraph) -> Result<Vec<usize>, GeometryError> {
    frame_starts_inner(para, false)
}

/// Only the host mixed-shape composer has line-owned object payloads. A TAC
/// object follows its stored line across a frame cut, never the whole paragraph.
pub(super) fn inline_shape_frame_starts(para: &Paragraph) -> Result<Vec<usize>, GeometryError> {
    super::shapes::validate_inline(para)?;
    frame_starts_inner(para, true)
}

fn frame_starts_inner(para: &Paragraph, inline_shapes: bool) -> Result<Vec<usize>, GeometryError> {
    let mut starts = Vec::new();
    for (i, pair) in para.line_segs.windows(2).enumerate() {
        if pair[1].vertical_pos < pair[0].vertical_pos {
            if pair[1].vertical_pos != 0
                || para.stored_text_partition_is_dirty()
                || para.source_line_seg_vertical_pos.is_some()
                || para.layout_only_fill_lines != 0
                // Document admission already validated these non-occupying
                // declarations (including the section slots inserted on save).
                || para.controls.iter().any(|c| {
                    !matches!(c, Control::SectionDef(_) | Control::ColumnDef(_) | Control::PageNumberPos(_))
                        && !(inline_shapes && matches!(c, Control::Shape(_)))
                })
                || !para.field_ranges.is_empty()
                || !para.orphan_field_ends.is_empty()
            {
                return Err(GeometryError::Unsupported(
                    "unqualified stored body frame reset",
                ));
            }
            starts.push(i + 1);
        }
    }
    Ok(starts)
}

/// A reset's preceding interline gap belongs to the old frame. It can exhaust
/// that page, but has no physical remainder on the continuation page. The next
/// LineOwner remains the first unconsumed unit; no text or empty row is removed.
pub(super) fn end_frame(blocks: &mut [FlowBlock]) {
    if let Some(FlowBlock::Space(gap)) = blocks.last() {
        let gap = *gap;
        *blocks.last_mut().expect("last space exists") = FlowBlock::FollowingLineGap(gap);
    }
}
