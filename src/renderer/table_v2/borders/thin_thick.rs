//! Open thin/thick table edges. The discrete HWP width catalog uses table
//! pens, not the generic shape profile. Independent normal-save PDF catalog:
//! tests/fixtures/issue7353_thin_thick_review (all 16 widths, four directions).
//! Quantities below are 600dpi pen-grid units: (thin, thick, thin-centre inset).
//! The thick centre is +thin; the thin centre is -inset, also on right/bottom
//! edges. Index zero is a single centred two-unit pen in the reference.
use super::*;

const PENS: [(u8, u8, u8); 16] = [
    (0, 2, 0),
    (1, 3, 2),
    (1, 3, 2),
    (1, 3, 2),
    (1, 4, 3),
    (1, 5, 3),
    (2, 5, 3),
    (3, 6, 5),
    (3, 8, 6),
    (4, 9, 6),
    (6, 12, 9),
    (8, 19, 13),
    (11, 25, 18),
    (17, 37, 27),
    (23, 49, 36),
    (29, 60, 45),
];

pub(super) fn append(
    edges: &BTreeMap<(bool, usize), Vec<Span>>,
    xs: &BTreeMap<usize, f64>,
    ys: &BTreeMap<usize, f64>,
    dpi: f64,
    nodes: &mut Vec<RenderNode>,
) -> Result<(), GeometryError> {
    for (&(horizontal, boundary), spans) in edges {
        let (along, across) = if horizontal { (xs, ys) } else { (ys, xs) };
        for span in spans {
            if span.style.line_type != BorderLineType::ThinThickDouble {
                continue;
            }
            // No corner/T/cross precedence is inferred from isolated pens.
            // Check the entire united span, not only its two endpoints.
            for (&(other_horizontal, other_boundary), others) in edges {
                if other_horizontal != horizontal
                    && (span.start..=span.end).contains(&other_boundary)
                    && others
                        .iter()
                        .any(|s| s.start <= boundary && boundary <= s.end)
                {
                    return Err(GeometryError::Unsupported("V2 thin-thick border junction"));
                }
            }
            if spans.iter().any(|other| {
                (other.end == span.start || other.start == span.end) && other.style != span.style
            }) {
                return Err(GeometryError::Unsupported("V2 thin-thick border junction"));
            }
            let (thin, thick, inset) = PENS[usize::from(span.style.width)];
            for (width, offset) in [(thin, -f64::from(inset)), (thick, f64::from(thin))] {
                if width == 0 {
                    continue;
                }
                let at = across[&boundary] + offset * dpi / 600.0;
                let from = along[&span.start];
                let to = along[&span.end];
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
                        width: f64::from(width) * dpi / 600.0,
                        ..Default::default()
                    },
                );
                let bounds = line.ink_bbox();
                nodes.push(RenderNode::new(0, RenderNodeType::Line(line), bounds));
            }
        }
    }
    Ok(())
}
