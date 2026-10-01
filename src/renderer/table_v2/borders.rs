//! Solid/double/dash edges of accepted table/cell fragments, including internal cuts.
//! Each physical fragment uses its source cell's four edges (None stays absent).
//! Explicit cell borderFill owns its edges, including None, over table outlines.
//! Qualified solid zone perimeters override cell edges.
use std::collections::{BTreeMap, BTreeSet};

mod dash;
mod double;
mod thin_thick;

use super::{GeometryError, TablePlacement};
use crate::{
    model::{
        style::{BorderLine, BorderLineType, BORDER_WIDTHS},
        table::Table,
    },
    renderer::{
        border_paint::border_width_to_px,
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
            resolve_edges(cell.border_fill_id, styles)?;
            if cell.border_fill_id != 0 {
                // A declared all-None border is not an absent reference.
                let edges = styles.border_styles[usize::from(cell.border_fill_id) - 1].borders;
                cells.insert((usize::from(cell.row), usize::from(cell.col)), edges);
            }
        }
        if cells
            .values()
            .all(|edges| edges.iter().all(|e| e.line_type == BorderLineType::None))
            && outline.is_none()
            && table.zones.is_empty()
        {
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
            .flat_map(|cell| cell.visible_rows.clone())
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
            let slot = slots[&cell.visible_rows.start];
            xs.insert(cell.column, cell.bounds.x);
            ys.insert(slot, cell.bounds.y);
        }
        for cell in &placement.cells {
            let slot = slots[&cell.visible_rows.start];
            // A shared boundary uses its observed start, not an independently
            // rounded prior start+width. Unobserved interior columns stay absent.
            xs.entry(cell.column + cell.column_span)
                .or_insert(cell.bounds.x + cell.bounds.width);
            ys.entry(slot + cell.visible_rows.len())
                .or_insert(cell.bounds.y + cell.bounds.height);
        }
        // (horizontal, boundary index) -> intervals in the other topology axis.
        let mut groups: BTreeMap<(bool, usize), Vec<Span>> = BTreeMap::new();
        // Shared unlike solid pens retain row/column paint ownership, not the
        // input vector's order. Hancom's regulatory table paints the left/top
        // cell first, then the right/bottom cell on the same centerline.
        let mut ordered_cells: Vec<_> = placement.cells.iter().collect();
        ordered_cells.sort_by_key(|cell| (cell.row, cell.column));
        for cell in ordered_cells {
            let Some(edges) = self.cells.get(&(cell.row, cell.column)) else {
                continue;
            };
            let row = slots[&cell.visible_rows.start];
            let row_end = row + cell.visible_rows.len();
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
            .map(|(key, spans)| Ok((key, union(&spans, true)?)))
            .collect::<Result<_, GeometryError>>()?;
        if let Some(outline) = &self.outline {
            // The normal Hancom title/outline-clean references preserve explicit
            // cell edges (including None), not the table-wide outline. Apply the
            // same ownership to each physical fragment. Missing cell references
            // are a different case: outline fallback is still unqualified.
            let last_column = xs
                .keys()
                .next_back()
                .copied()
                .ok_or(GeometryError::InconsistentAtomicPlan)?;
            let last_row = slots.len();
            for cell in &placement.cells {
                if self.cells.contains_key(&(cell.row, cell.column)) {
                    continue;
                }
                let row = slots[&cell.visible_rows.start];
                let on_outline = [
                    cell.column == 0,
                    cell.column + cell.column_span == last_column,
                    row == 0,
                    row + cell.visible_rows.len() == last_row,
                ];
                if on_outline
                    .iter()
                    .zip(outline)
                    .any(|(on, edge)| *on && edge.line_type != BorderLineType::None)
                {
                    return Err(GeometryError::Unsupported(
                        "V2 table/cell outline disagreement",
                    ));
                }
            }
            // Do not add an extra outline paint layer over cell-owned edges.
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
        let mut zone_edges: BTreeMap<(bool, usize), Vec<Span>> = BTreeMap::new();
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
                zone_edges
                    .entry(key)
                    .or_default()
                    .push(Span { start, end, style });
            }
        }
        // Equal per-side declarations alone do not prove compatibility: a
        // zone's right edge can meet another's differently styled left edge.
        // Resolve the ACTUAL fragment perimeters together before replacing cell
        // edges, so declaration order cannot choose the winning zone paint.
        for (key, zone_spans) in zone_edges {
            for Span { start, end, style } in union(&zone_spans, false)? {
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
                *spans = union(&replaced, true)?;
            }
        }
        let mut nodes = Vec::new();
        double::append(&edges, &xs, &ys, self.dpi, &mut nodes)?;
        thin_thick::append(&edges, &xs, &ys, self.dpi, &mut nodes)?;
        for ((horizontal, boundary), spans) in edges {
            for span in spans {
                if matches!(
                    span.style.line_type,
                    BorderLineType::Double | BorderLineType::ThinThickDouble
                ) {
                    continue;
                }
                let (x1, y1, x2, y2) = if horizontal {
                    (xs[&span.start], ys[&boundary], xs[&span.end], ys[&boundary])
                } else {
                    (xs[&boundary], ys[&span.start], xs[&boundary], ys[&span.end])
                };
                if matches!(
                    span.style.line_type,
                    BorderLineType::Dash | BorderLineType::Dot
                ) {
                    dash::append([x1, y1, x2, y2], span.style, self.dpi, &mut nodes)?;
                    continue;
                }
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
            | BorderLineType::Double
            | BorderLineType::Dash
            | BorderLineType::Dot
            | BorderLineType::ThinThickDouble
                if usize::from(edge.width) < BORDER_WIDTHS.len() && edge.color >> 24 == 0 =>
            {
                visible = true
            }
            _ => return Err(GeometryError::Unsupported("V2 cell border style")),
        }
    }
    Ok(visible.then_some(style.borders))
}

fn union(spans: &[Span], cell_layers: bool) -> Result<Vec<Span>, GeometryError> {
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
        let mut layers = vec![first.style];
        for span in active {
            let style = layers.last_mut().unwrap();
            if span.style == *style {
                continue;
            }
            // Coincident opaque solid strokes of one color have the union of
            // their widths. Resolve each topology interval independently: a
            // short thick edge must not thicken the rest of a merged cell edge.
            // Hancom's saved market-share table paints both widths on the same
            // centerline. This is not a priority rule for colors or line types.
            if style.line_type != BorderLineType::Solid
                || span.style.line_type != BorderLineType::Solid
            {
                return Err(GeometryError::Unsupported(
                    "conflicting shared V2 cell borders",
                ));
            }
            if style.color != span.style.color {
                // Unlike cell colors are separate opaque strokes, not a
                // winner-takes-all width rule. Zones have no cell-side ownership
                // and keep their independent conflict contract.
                if !cell_layers {
                    return Err(GeometryError::Unsupported(
                        "conflicting shared V2 cell borders",
                    ));
                }
                layers.push(span.style);
                continue;
            }
            if BORDER_WIDTHS[usize::from(span.style.width)].0
                > BORDER_WIDTHS[usize::from(style.width)].0
            {
                *style = span.style;
            }
        }
        for style in layers {
            if let Some(last) = result.last_mut() {
                if last.end == pair[0] && last.style == style {
                    last.end = pair[1];
                    continue;
                }
            }
            result.push(Span {
                start: pair[0],
                end: pair[1],
                style,
            });
        }
    }
    Ok(result)
}
