//! Double table pens use a 1:2:1 stroke/gap/stroke profile, not the generic
//! shape double-line profile. Hancom normal-save grid references (0.1, 0.2,
//! 0.5, 1.5 mm) and the original #6923 title cell establish the pen widths
//! and the open gaps at L/T/cross junctions. Layout rectangles remain intact.
use super::*;

type Edges = BTreeMap<(bool, usize), Vec<Span>>;

fn pen(style: BorderLine, dpi: f64) -> f64 {
    // Split the DECLARED width first, then quantize each pen to the 600dpi
    // grid. Dividing a quantized solid stroke gives a different result.
    let hu = (BORDER_WIDTHS[usize::from(style.width)].0 * 7200.0 / 25.4 / 4.0).round();
    (hu / 12.0).round().max(1.0) * dpi / 600.0
}

fn incident(edges: &Edges, key: (bool, usize), point: usize, positive: bool) -> Option<BorderLine> {
    edges.get(&key)?.iter().find_map(|s| {
        let contains = if positive {
            s.start <= point && point < s.end
        } else {
            s.start < point && point <= s.end
        };
        contains.then_some(s.style)
    })
}

struct Junction<'a> {
    edges: &'a Edges,
    horizontal: bool,
    boundary: usize,
    style: BorderLine,
    dpi: f64,
}

impl Junction<'_> {
    fn inset(&self, point: usize, positive: bool, start: bool) -> Result<f64, GeometryError> {
        let perpendicular = (!self.horizontal, point);
        let same_side = incident(self.edges, perpendicular, self.boundary, positive);
        let other_side = incident(self.edges, perpendicular, self.boundary, !positive);
        // A continuous, same-color solid crosses the whole junction. Both
        // double pens stop at its near ink edge; the solid itself stays intact.
        // Normal-save mixed-inner grids and the regulatory overview PDF confirm
        // this T/cross topology. One-sided corners and color precedence are
        // different rules and remain explicit unsupported cases below.
        if let (Some(a), Some(b)) = (same_side, other_side) {
            if a == b && a.line_type == BorderLineType::Solid && a.color == self.style.color {
                return Ok(border_width_to_px(a.width) * self.dpi / 96.0 / 2.0);
            }
        }
        // Color/unequal-width/mixed-style junction precedence is not established
        // by these references. Reject it instead of painting through the gap.
        for other in [same_side, other_side].into_iter().flatten() {
            if other != self.style {
                return Err(GeometryError::Unsupported(
                    "V2 mixed double-border junction",
                ));
            }
        }
        let perpendicular_pen = pen(self.style, self.dpi);
        if same_side.is_some() {
            // The offset line meets the nearer pen on that side of the vertex.
            // Butt caps reach the perpendicular pen's outside ink edge.
            return Ok(perpendicular_pen);
        }
        let continues = incident(self.edges, (self.horizontal, self.boundary), point, !start);
        if let Some(other) = continues {
            if other != self.style {
                return Err(GeometryError::Unsupported(
                    "V2 mixed double-border junction",
                ));
            }
            return Ok(0.0);
        }
        if other_side.is_some() {
            // Outside L corner: extend to the outside of the opposite pen.
            return Ok(-2.0 * perpendicular_pen);
        }
        // An isolated open end has no perpendicular pen or invented closure.
        Ok(0.0)
    }
}

pub(super) fn append(
    edges: &Edges,
    xs: &BTreeMap<usize, f64>,
    ys: &BTreeMap<usize, f64>,
    dpi: f64,
    nodes: &mut Vec<RenderNode>,
) -> Result<(), GeometryError> {
    for (&(horizontal, boundary), spans) in edges {
        let (along, across) = if horizontal { (xs, ys) } else { (ys, xs) };
        for span in spans {
            if span.style.line_type != BorderLineType::Double {
                continue;
            }
            // The union removes duplicate shared edges, but a perpendicular
            // branch can meet inside a united span. Split there before offsetting
            // so inner strokes cannot bridge the white T/cross junction gap.
            let points: Vec<_> = along
                .range(span.start..=span.end)
                .map(|(&k, _)| k)
                .collect();
            let junction = Junction {
                edges,
                horizontal,
                boundary,
                style: span.style,
                dpi,
            };
            let width = pen(span.style, dpi);
            for pair in points.windows(2) {
                for positive in [false, true] {
                    let from = along[&pair[0]] + junction.inset(pair[0], positive, true)?;
                    let to = along[&pair[1]] - junction.inset(pair[1], positive, false)?;
                    if to <= from {
                        return Err(GeometryError::Unsupported(
                            "V2 double-border junction exceeds edge",
                        ));
                    }
                    let offset = if positive { 1.5 * width } else { -1.5 * width };
                    let at = across[&boundary] + offset;
                    let (x1, y1, x2, y2) = if horizontal {
                        (from, at, to, at)
                    } else {
                        (at, from, at, to)
                    };
                    let line = LineNode::new(
                        x1,
                        y1,
                        x2,
                        y2,
                        LineStyle {
                            color: span.style.color,
                            width,
                            ..Default::default()
                        },
                    );
                    let bounds = line.ink_bbox();
                    nodes.push(RenderNode::new(0, RenderNodeType::Line(line), bounds));
                }
            }
        }
    }
    Ok(())
}
