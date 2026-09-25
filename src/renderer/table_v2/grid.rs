//! Resolve cell topology in integer HWP units before composing any paragraphs.
//! A merged cell's width is not divided between invented hidden columns.
use super::{
    content::{CellTrack, VerticalAlignment},
    GeometryError,
};
use crate::model::table::{Cell, Table, VerticalAlign};

pub(super) struct ResolvedGrid<'a> {
    pub rows: Vec<Vec<&'a Cell>>,
    pub tracks: Vec<Vec<CellTrack>>,
    pub width: f64,
}

pub(super) fn resolve(table: &Table, scale: f64) -> Result<ResolvedGrid<'_>, GeometryError> {
    let nr = table.row_count as usize;
    let nc = table.col_count as usize;
    if nr == 0 || nc == 0 {
        return Err(GeometryError::EmptyTable);
    }
    let mut rows = vec![Vec::new(); nr];
    for cell in &table.cells {
        if cell.row as usize >= nr
            || cell.row_span == 0
            || cell.row as usize + cell.row_span as usize > nr
            || cell.col_span == 0
            || cell.col as usize + cell.col_span as usize > nc
        {
            return Err(GeometryError::Unsupported("cell address or span"));
        }
        rows[cell.row as usize].push(cell);
    }
    // Only observed edges have coordinates. All rows must agree at shared edges;
    // an unobserved internal edge of a merged region is deliberately left unknown.
    let mut edges = vec![None; nc + 1];
    let mut tracks = Vec::with_capacity(nr);
    for row in &mut rows {
        row.sort_by_key(|cell| cell.col);
    }
    let mut covering: Vec<&Cell> = Vec::new();
    for (r, row) in rows.iter().enumerate() {
        // A spanning cell owns every covered grid slot, but its content is bound
        // only at its original row. Never duplicate it into continuation rows.
        covering.retain(|cell| r < cell.row as usize + cell.row_span as usize);
        covering.extend(row.iter().copied());
        covering.sort_by_key(|cell| cell.col);
        let mut column = 0;
        let mut left = 0_i64;
        let mut resolved = Vec::with_capacity(row.len());
        for cell in &covering {
            if cell.col as usize != column || cell.width == 0 {
                return Err(GeometryError::Unsupported(
                    "incomplete, overlapping or zero-width grid",
                ));
            }
            let end = column + cell.col_span as usize;
            let right = left + i64::from(cell.width);
            for (index, position) in [(column, left), (end, right)] {
                if edges[index].is_some_and(|old| old != position) {
                    return Err(GeometryError::Unsupported("inconsistent grid boundary"));
                }
                edges[index] = Some(position);
            }
            if cell.row as usize == r {
                resolved.push(CellTrack {
                    column,
                    span: cell.col_span as usize,
                    row_span: cell.row_span as usize,
                    left: left as f64 * scale,
                    width: cell.width as f64 * scale,
                    alignment: match cell.vertical_align {
                        VerticalAlign::Top => VerticalAlignment::Top,
                        VerticalAlign::Center => VerticalAlignment::Center,
                        VerticalAlign::Bottom => VerticalAlignment::Bottom,
                    },
                    content_offset_y: 0.0,
                });
            }
            left = right;
            column = end;
        }
        if column != nc {
            return Err(GeometryError::Unsupported("incomplete grid row"));
        }
        tracks.push(resolved);
    }
    // Observed column edges must also be ordered when they come from DIFFERENT
    // rows. Shared endpoints alone do not exclude crossing internal boundaries.
    let mut previous = None;
    for position in edges.iter().flatten() {
        if previous.is_some_and(|p| p >= *position) {
            return Err(GeometryError::Unsupported("non-increasing grid boundary"));
        }
        previous = Some(*position);
    }
    let width = edges[nc].ok_or(GeometryError::EmptyTable)? as f64 * scale;
    Ok(ResolvedGrid {
        rows,
        tracks,
        width,
    })
}
