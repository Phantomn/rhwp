//! Solid edges of accepted table/cell fragments, including cell-internal cuts.
//! Each physical fragment uses its source cell's four edges (None stays absent).
//! Matching table outlines share those edges. Qualified solid zone perimeters
//! override cell edges; other border-priority rules remain unsupported.
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
    outline: Option<[BorderLine; 4]>,
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
        let outline = resolve_edges(table.border_fill_id, styles)?;
        let mut cells = BTreeMap::new();
        for cell in &table.cells {
            if let Some(edges) = resolve_edges(cell.border_fill_id, styles)? {
                cells.insert((usize::from(cell.row), usize::from(cell.col)), edges);
            }
        }
        if cells.is_empty() && outline.is_none() && table.zones.is_empty() {
            return Ok(None);
        }
        Ok(Some(Self {
            cells,
            outline,
            dpi,
        }))
    }

    pub fn append(
        &self,
        placement: &TablePlacement,
        zones: &[super::zones::Zone],
        node: &mut RenderNode,
    ) -> Result<(), GeometryError> {
        // The IR adapter guarantees a contiguous complete grid and
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
        let visible_rows: BTreeSet<_> = placement
            .cells
            .iter()
            .flat_map(|cell| cell.row..cell.row + cell.row_span)
            .collect();
        for (slot, row) in visible_rows.into_iter().enumerate() {
            slots.insert(row, slot);
        }
        for cell in &placement.cells {
            // Logical row boundaries must have distinct physical coordinates.
            // Even an unbordered zero-height row would otherwise separate two
            // coincident visible edges into different union groups.
            if cell.bounds.height <= 0.0 {
                return Err(GeometryError::Unsupported(
                    "zero-height row in bordered table",
                ));
            }
            let slot = slots[&cell.row];
            xs.insert(cell.column, cell.bounds.x);
            ys.insert(slot, cell.bounds.y);
        }
        for cell in &placement.cells {
            let slot = slots[&cell.row];
            // A shared boundary uses its observed start, not an independently
            // rounded prior start+width. Unobserved interior columns stay absent.
            xs.entry(cell.column + cell.column_span)
                .or_insert(cell.bounds.x + cell.bounds.width);
            ys.entry(slot + cell.row_span)
                .or_insert(cell.bounds.y + cell.bounds.height);
        }
        // (horizontal, boundary index) -> intervals in the other topology axis.
        let mut groups: BTreeMap<(bool, usize), Vec<Span>> = BTreeMap::new();
        for cell in &placement.cells {
            let Some(edges) = self.cells.get(&(cell.row, cell.column)) else {
                continue;
            };
            let row = slots[&cell.row];
            let row_end = row + cell.row_span;
            let end = cell.column + cell.column_span;
            for (horizontal, boundary, start, stop, style) in [
                (false, cell.column, row, row_end, edges[0]),
                (false, end, row, row_end, edges[1]),
                (true, row, cell.column, end, edges[2]),
                (true, row_end, cell.column, end, edges[3]),
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
        let mut edges: BTreeMap<_, _> = groups
            .into_iter()
            .map(|(key, spans)| Ok((key, union(&spans)?)))
            .collect::<Result<_, GeometryError>>()?;
        if let Some(outline) = &self.outline {
            // Table/None precedence is not established (#6311/KTX counterexample).
            // Admit only an outline already fully represented by identical cell
            // edges. Check each ACCEPTED fragment, including header/body cuts:
            // agreement on the unsplit source alone does not prove agreement here.
            let last_column = xs
                .keys()
                .next_back()
                .copied()
                .ok_or(GeometryError::InconsistentAtomicPlan)?;
            let last_row = slots.len();
            for (key, end, style) in [
                ((false, 0), last_row, outline[0]),
                ((false, last_column), last_row, outline[1]),
                ((true, 0), last_column, outline[2]),
                ((true, last_row), last_column, outline[3]),
            ] {
                if style.line_type == BorderLineType::None {
                    continue;
                }
                let mut covered = 0;
                for span in edges.get(&key).into_iter().flatten() {
                    if span.start != covered || span.style != style {
                        break;
                    }
                    covered = span.end;
                }
                if covered != end {
                    return Err(GeometryError::Unsupported(
                        "V2 table/cell outline disagreement",
                    ));
                }
            }
            // The outline is the same geometric set, not another paint layer.
            // Drawing it again would change coverage/opacity at coincident edges.
        }
        // Visible zone perimeter overrides cell edges; None keeps cell edges.
        // Same physical bounds as the background, including continuation cuts.
        // Independent evidence: zone-lines Hancom PDF, both fragment perimeters.
        let coordinate = |axis: &BTreeMap<usize, f64>, value: f64| {
            axis.iter()
                .find_map(|(k, v)| ((*v - value).abs() < 1e-7).then_some(*k))
                .ok_or(GeometryError::Unsupported(
                    "V2 zone boundary without cell edge",
                ))
        };
        for zone in zones {
            let Some(styles) = zone.edges else {
                continue;
            };
            let Some(b) = zone.bounds(placement)? else {
                continue;
            };
            let left = coordinate(&xs, b.x)?;
            let right = coordinate(&xs, b.x + b.width)?;
            let top = coordinate(&ys, b.y)?;
            let bottom = coordinate(&ys, b.y + b.height)?;
            for (key, start, end, style) in [
                ((false, left), top, bottom, styles[0]),
                ((false, right), top, bottom, styles[1]),
                ((true, top), left, right, styles[2]),
                ((true, bottom), left, right, styles[3]),
            ] {
                if style.line_type == BorderLineType::None {
                    continue;
                }
                let spans = edges.entry(key).or_default();
                let mut replaced = Vec::new();
                for old in spans.iter() {
                    if old.end <= start || old.start >= end {
                        replaced.push(*old);
                        continue;
                    }
                    if old.start < start {
                        replaced.push(Span { end: start, ..*old });
                    }
                    if old.end > end {
                        replaced.push(Span { start: end, ..*old });
                    }
                }
                replaced.push(Span { start, end, style });
                *spans = union(&replaced)?;
            }
        }
        let mut nodes = Vec::new();
        for ((horizontal, boundary), spans) in edges {
            for span in spans {
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

pub(super) fn resolve_edges(
    id: u16,
    styles: &ResolvedStyleSet,
) -> Result<Option<[BorderLine; 4]>, GeometryError> {
    if id == 0 {
        return Ok(None);
    }
    let style = styles
        .border_styles
        .get(usize::from(id) - 1)
        .ok_or(GeometryError::Unsupported(
            "missing table borderFill reference",
        ))?;
    let mut visible = false;
    for edge in &style.borders {
        match edge.line_type {
            BorderLineType::None => {}
            BorderLineType::Solid
                if usize::from(edge.width) < BORDER_WIDTHS.len() && edge.color >> 24 == 0 =>
            {
                visible = true
            }
            _ => return Err(GeometryError::Unsupported("V2 cell border style")),
        }
    }
    Ok(visible.then_some(style.borders))
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
