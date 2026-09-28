//! Table Dash/Dot pens differ from the generic shape dash pattern. Emit the same
//! physical strokes for every backend, after topology union removes duplicates.
use super::*;

pub(super) fn append(
    [x1, y1, x2, y2]: [f64; 4],
    style: BorderLine,
    dpi: f64,
    nodes: &mut Vec<RenderNode>,
) -> Result<(), GeometryError> {
    // Independent normally saved pen catalog (600dpi printer grid). The
    // entries are format width indices, never document IDs or layout sizes.
    // tests/fixtures/issue7353/cell-dash/pen-catalog-2020.pdf establishes
    // all16 standard pens, including horizontal and vertical strokes. The
    // printer's tiny page-axis scaling is not a layout correction here.
    const PENS: [(f64, f64); 16] = [
        (17., 9.),
        (20., 12.),
        (26., 14.),
        (34., 21.),
        (43., 27.),
        (52., 29.),
        (69., 42.),
        (86., 51.),
        (104., 62.),
        (121., 72.),
        (173., 105.),
        (259., 156.),
        (347., 206.),
        (520., 311.),
        (693., 417.),
        (866., 519.),
    ];
    // Normally saved Dot catalog, both axes at all16 format pen indices:
    // tests/fixtures/issue7353_inline_bounds_review/dot-2020.pdf.
    const DOT_PENS: [(f64, f64); 16] = [
        (3., 4.),
        (4., 6.),
        (5., 7.),
        (7., 10.),
        (9., 13.),
        (10., 15.),
        (14., 21.),
        (17., 25.),
        (21., 31.),
        (24., 36.),
        (35., 52.),
        (52., 78.),
        (69., 103.),
        (104., 156.),
        (139., 208.),
        (173., 259.),
    ];
    let pens = if style.line_type == BorderLineType::Dot {
        &DOT_PENS
    } else {
        &PENS
    };
    let &(on, off) = pens
        .get(usize::from(style.width))
        .ok_or(GeometryError::Unsupported("V2 dash pen width"))?;
    let on = on * dpi / 600.;
    let period = on + off * dpi / 600.;
    let horizontal = y1 == y2;
    let length = if horizontal { x2 - x1 } else { y2 - y1 };
    let count = (length / period).ceil() as usize;
    for i in 0..count {
        let start = i as f64 * period;
        // A final partial stroke is the requested pen clipped at its edge end,
        // not a clamp of cell or content geometry. No stroke fills a final gap.
        let end = (start + on).min(length);
        let (a, b, c, d) = if horizontal {
            (x1 + start, y1, x1 + end, y2)
        } else {
            (x1, y1 + start, x2, y1 + end)
        };
        let line = LineNode::new(
            a,
            b,
            c,
            d,
            LineStyle {
                color: style.color,
                width: border_width_to_px(style.width) * dpi / 96.,
                ..Default::default()
            },
        );
        let bounds = line.ink_bbox();
        nodes.push(RenderNode::new(0, RenderNodeType::Line(line), bounds));
    }
    Ok(())
}
