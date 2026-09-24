//! Solid edges of accepted cell fragments, including cell-internal page cuts.
//! Each physical fragment uses its source cell's four edges (None stays absent).
//! Conflicting-edge priority and special split-line effects are not supported.
use std::collections::{BTreeMap, BTreeSet};

use super::{GeometryError, TablePlacement};
use crate::{
    model::{
        style::{BorderLine, BorderLineType, BORDER_WIDTHS},
        table::Table,
    },
    renderer::{
        layout::border_width_to_px,
        render_tree::{LineNode, RenderNode, RenderNodeType},
        style_resolver::ResolvedStyleSet,
        LineStyle,
    },
};

pub(super) struct CellBorders {
    cells: BTreeMap<(usize, usize), [BorderLine; 4]>,
    dpi: f64,
}

#[derive(Clone, Copy)]
struct Span {
    start: usize,
    end: usize,
    style: BorderLine,
}

impl CellBorders {
    pub fn prepare(
        table: &Table,
        styles: &ResolvedStyleSet,
        dpi: f64,
    ) -> Result<Option<Self>, GeometryError> {
        let mut cells = BTreeMap::new();
        for cell in &table.cells {
            if cell.border_fill_id == 0 {
                continue;
            }
            let style = styles
                .border_styles
                .get(usize::from(cell.border_fill_id) - 1)
                .ok_or(GeometryError::Unsupported(
                    "missing table borderFill reference",
                ))?;
            let mut visible = false;
            for edge in &style.borders {
                match edge.line_type {
                    BorderLineType::None => continue,
                    BorderLineType::Solid
                        if usize::from(edge.width) < BORDER_WIDTHS.len()
                            && edge.color >> 24 == 0 =>
                    {
                        visible = true
                    }
                    _ => return Err(GeometryError::Unsupported("V2 cell border style")),
                }
            }
            if visible {
                cells.insert(
                    (usize::from(cell.row), usize::from(cell.col)),
                    style.borders,
                );
            }
        }
        if cells.is_empty() {
            return Ok(None);
        }
        Ok(Some(Self { cells, dpi }))
    }

    pub fn append(
        &self,
        placement: &TablePlacement,
        node: &mut RenderNode,
    ) -> Result<(), GeometryError> {
        // The IR adapter guarantees a contiguous complete grid, row_span=1 and
        // zero row spacing. Use the fragment's visible row order, NOT source row
        // adjacency: a repeated header may be followed by a later body row.
        // WithinCells supplies the same physical rectangles for partial cells,
        // completed siblings and remaining minimum-height bands. Close those
        // rectangles, not the source cell's full height or its remaining text.
        // Independent closure evidence: task3236 Hancom PDF p1/p2 (CELL,
        // breakCellSeparateLine=0). Represented special effects are rejected
        // upstream; unmodeled format attributes are outside this IR contract.
        let mut slots = BTreeMap::new();
        let mut xs = BTreeMap::new();
        let mut ys = BTreeMap::new();
        for cell in &placement.cells {
            // Logical row boundaries must have distinct physical coordinates.
            // Even an unbordered zero-height row would otherwise separate two
            // coincident visible edges into different union groups.
            if cell.bounds.height <= 0.0 {
                return Err(GeometryError::Unsupported(
                    "zero-height row in bordered table",
                ));
            }
            let next = slots.len();
            let slot = *slots.entry(cell.row).or_insert(next);
            xs.insert(cell.column, cell.bounds.x);
            ys.insert(slot, cell.bounds.y);
        }
        for cell in &placement.cells {
            let slot = slots[&cell.row];
            // A shared boundary uses its observed start, not an independently
            // rounded prior start+width. Unobserved interior columns stay absent.
            xs.entry(cell.column + cell.column_span)
                .or_insert(cell.bounds.x + cell.bounds.width);
            ys.entry(slot + 1)
                .or_insert(cell.bounds.y + cell.bounds.height);
        }
        // (horizontal, boundary index) -> intervals in the other topology axis.
        let mut groups: BTreeMap<(bool, usize), Vec<Span>> = BTreeMap::new();
        for cell in &placement.cells {
            let Some(edges) = self.cells.get(&(cell.row, cell.column)) else {
                continue;
            };
            let row = slots[&cell.row];
            let end = cell.column + cell.column_span;
            for (horizontal, boundary, start, stop, style) in [
                (false, cell.column, row, row + 1, edges[0]),
                (false, end, row, row + 1, edges[1]),
                (true, row, cell.column, end, edges[2]),
                (true, row + 1, cell.column, end, edges[3]),
            ] {
                if style.line_type != BorderLineType::None {
                    groups
                        .entry((horizontal, boundary))
                        .or_default()
                        .push(Span {
                            start,
                            end: stop,
                            style,
                        });
                }
            }
        }
        let mut nodes = Vec::new();
        for ((horizontal, boundary), spans) in groups {
            for span in union(&spans)? {
                let (x1, y1, x2, y2) = if horizontal {
                    (xs[&span.start], ys[&boundary], xs[&span.end], ys[&boundary])
                } else {
                    (xs[&boundary], ys[&span.start], xs[&boundary], ys[&span.end])
                };
                let line = LineNode::new(
                    x1,
                    y1,
                    x2,
                    y2,
                    LineStyle {
                        color: span.style.color,
                        width: border_width_to_px(span.style.width) * self.dpi / 96.0,
                        ..Default::default()
                    },
                );
                let bounds = line.ink_bbox();
                nodes.push(RenderNode::new(0, RenderNodeType::Line(line), bounds));
            }
        }
        // All edge conflicts are checked before attaching any nodes. Session
        // commit remains downstream of building the entire recursive tree.
        node.children.extend(nodes);
        Ok(())
    }
}

fn union(spans: &[Span]) -> Result<Vec<Span>, GeometryError> {
    let points: Vec<_> = spans
        .iter()
        .flat_map(|s| [s.start, s.end])
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut result: Vec<Span> = Vec::new();
    for pair in points.windows(2) {
        let mut active = spans
            .iter()
            .filter(|s| s.start <= pair[0] && s.end >= pair[1]);
        let Some(first) = active.next() else {
            continue;
        };
        if active.any(|s| s.style != first.style) {
            return Err(GeometryError::Unsupported(
                "conflicting shared V2 cell borders",
            ));
        }
        if let Some(last) = result.last_mut() {
            if last.end == pair[0] && last.style == first.style {
                last.end = pair[1];
                continue;
            }
        }
        result.push(Span {
            start: pair[0],
            end: pair[1],
            style: first.style,
        });
    }
    Ok(result)
}
