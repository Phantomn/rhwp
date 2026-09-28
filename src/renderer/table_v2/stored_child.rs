//! Corroborate a saved exclusion's frame transition from both sides of its cut.
//! A saved fragment's physical band is distinct from cached intact cell height.
//! No following paragraph is turned into a page break.
use crate::model::table::Table;

use super::{FlowBlock, GeometryError, SplitPolicy, TableContentPlan};

pub(super) fn qualify(
    plan: &mut TableContentPlan,
    table: &Table,
    scale: f64,
    following_origin: f64,
    top: f64,
    bottom: f64,
) -> Result<(), GeometryError> {
    let fail = || GeometryError::Unsupported("unqualified stored child frame geometry");
    // Two saved frames may meet between rows or inside one plain-text row.
    // Spanning owners and repeated headings need their own frame description.
    if plan.policy == SplitPolicy::Never
        || plan.header_rows != 0
        || plan.grid.iter().flatten().any(|c| c.row_span != 1)
        || table.cells.iter().flat_map(|c| &c.paragraphs).any(|p| {
            p.line_segs.is_empty()
                || p.stored_text_partition_is_dirty()
                || p.source_line_seg_vertical_pos.is_some()
                || p.layout_only_fill_lines != 0
                || !p.controls.is_empty()
        })
    {
        return Err(fail());
    }
    let cut_rows: Vec<_> = plan
        .rows
        .iter()
        .enumerate()
        .filter_map(|(row, input)| {
            input
                .cells
                .iter()
                .flat_map(|c| &c.blocks)
                .any(FlowBlock::has_stored_frame_cut)
                .then_some(row)
        })
        .collect();
    if !cut_rows.is_empty() {
        if cut_rows.len() != 1 || plan.policy != SplitPolicy::WithinCells {
            return Err(fail());
        }
        let row = cut_rows[0];
        let first = f64::from(table.common.height) * scale
            - plan.row_heights[..row].iter().sum::<f64>()
            - plan.row_spacing * row as f64;
        let last = following_origin
            - top
            - bottom
            - plan.row_heights[row + 1..].iter().sum::<f64>()
            - plan.row_spacing * (plan.rows.len() - row - 1) as f64;
        let frames = super::stored_child_frames::StoredRowFrames::qualify(plan, row, first, last)?;
        plan.row_heights[row] = first + last;
        plan.stored_row_bands.push((row, first + last));
        plan.height =
            plan.row_heights.iter().sum::<f64>() + plan.row_spacing * (plan.rows.len() - 1) as f64;
        plan.stored_cell_frames.push(frames);
        return Ok(());
    }
    let same = |a: f64, b: f64| (a - b).abs() <= 32.0 * f64::EPSILON * a.abs().max(b.abs());
    let first = f64::from(table.common.height) * scale;
    let matches: Vec<_> = (1..plan.rows.len())
        .filter_map(|row| {
            let prefix = plan.row_heights[..row - 1].iter().sum::<f64>()
                + plan.row_spacing * (row - 1) as f64;
            let band = first - prefix;
            let last = top
                + bottom
                + plan.row_heights[row..].iter().sum::<f64>()
                + plan.row_spacing * (plan.rows.len() - row - 1) as f64;
            // A whole-row cut has exactly one end band. Every cell's complete
            // content and padding must fit that band; content clipping is never
            // a way to make the saved frame valid. The suffix must independently
            // account for the following paragraph's source origin.
            (same(last, following_origin)
                && band > 0.0
                && (band <= plan.row_heights[row - 1] || same(band, plan.row_heights[row - 1]))
                && plan.grid[row - 1]
                    .iter()
                    .all(|track| track.content_height <= band))
            .then_some((row, band))
        })
        .collect();
    if matches.len() != 1 {
        return Err(fail());
    }
    let (cut, band) = matches[0];
    let row = cut - 1;
    if !same(band, plan.row_heights[row]) {
        // Resolve once, before any ancestor measures this child. Fit uses this
        // same physical band and alignment; paint has no independent correction.
        plan.row_heights[row] = band;
        plan.stored_row_bands.push((row, band));
        for track in &mut plan.grid[row] {
            let slack = band - track.content_height;
            track.content_offset_y = match track.alignment {
                super::content::VerticalAlignment::Top => 0.0,
                super::content::VerticalAlignment::Center => slack / 2.0,
                super::content::VerticalAlignment::Bottom => slack,
            };
        }
        plan.height =
            plan.row_heights.iter().sum::<f64>() + plan.row_spacing * (plan.rows.len() - 1) as f64;
    }
    plan.stored_row_starts = vec![cut];
    Ok(())
}
