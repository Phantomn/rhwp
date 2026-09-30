//! Unlike shared solid pens preserve cell-side paint order, not color priority.
use rhwp::{
    model::{control::Control, document::Document, table::Table},
    renderer::table_v2::DocumentV2Session,
};
use serde_json::Value;
const INPUT: &[u8] = include_bytes!("../fixtures/issue7353_color_border_review/saved.hwp");
fn table(d: &mut Document) -> &mut Table {
    d.sections[0].paragraphs[0]
        .controls
        .iter_mut()
        .find_map(|c| match c {
            Control::Table(t) => Some(t.as_mut()),
            _ => None,
        })
        .unwrap()
}
fn open(data: &[u8], dpi: f64) -> Value {
    let mut s = DocumentV2Session::from_bytes(
        data,
        &format!(r#"{{"dpi":{dpi},"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}}"#),
    )
    .unwrap();
    let p = serde_json::from_str(&s.next_page_json().unwrap().unwrap()).unwrap();
    assert!(s.next_page_json().unwrap().is_none());
    p
}
fn nodes<'a>(n: &'a Value, kind: &str) -> Vec<&'a Value> {
    let mut v = if n["node_type"].get(kind).is_some() {
        vec![n]
    } else {
        vec![]
    };
    for c in n["children"].as_array().unwrap() {
        v.extend(nodes(c, kind));
    }
    v
}
fn n(v: &Value, k: &str) -> f64 {
    v[k].as_f64().unwrap()
}
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-7
}
fn check(p: &Value, dpi: f64, row: u16, horizontal: bool, expected: &[(u32, f64)]) {
    let root = &p["render_tree"]["root"];
    let cell = nodes(root, "TableCell")
        .into_iter()
        .find(|c| {
            c["node_type"]["TableCell"]["row"] == row && c["node_type"]["TableCell"]["col"] == 4
        })
        .unwrap();
    let b = &cell["bbox"];
    let (axis, at, start, end) = if horizontal {
        (
            "y",
            n(b, "y") + n(b, "height"),
            n(b, "x"),
            n(b, "x") + n(b, "width"),
        )
    } else {
        (
            "x",
            n(b, "x") + n(b, "width"),
            n(b, "y"),
            n(b, "y") + n(b, "height"),
        )
    };
    let along = if horizontal { "x" } else { "y" };
    let selected: Vec<_> = nodes(root, "Line")
        .into_iter()
        .map(|l| &l["node_type"]["Line"])
        .filter(|l| {
            close(n(l, &format!("{axis}1")), at)
                && close(n(l, &format!("{axis}2")), at)
                && n(l, &format!("{along}1")) <= start + 1e-7
                && n(l, &format!("{along}2")) >= end - 1e-7
        })
        .collect();
    assert_eq!(selected.len(), expected.len());
    for (l, (color, width)) in selected.iter().zip(expected) {
        assert_eq!(l["style"]["color"], *color);
        assert!(close(n(&l["style"], "width"), width * dpi / 96.));
    }
    // Independent Hancom PDF: vertical shared stroke x368.562pt, y157.628pt
    // to185.437pt. Source HU and600dpi printer grid differ by <0.4pt.
    if !horizontal && row == 4 {
        for (actual, pdf) in [(at, 368.562), (start, 157.628), (end, 185.437)] {
            assert!((actual * 72. / dpi - pdf).abs() < 0.4, "{actual} vs {pdf}");
        }
    }
}
#[test]
fn normal_saved_cell_pens_keep_both_colors_and_widths() {
    for dpi in [96., 192.] {
        let p = open(INPUT, dpi);
        check(&p, dpi, 4, false, &[(0, 0.48), (0x4c4c4c, 0.32)]);
        assert_eq!(nodes(&p["render_tree"]["root"], "Table").len(), 2);
    }
}
#[test]
fn reversed_pen_colors_and_horizontal_edges_follow_cell_side_not_width_priority() {
    let data = include_bytes!("../fixtures/issue7353_color_border_review/variant.hwpx");
    let p = open(data, 96.);
    check(&p, 96., 4, false, &[(0x4c4c4c, 0.32), (0, 0.48)]);
    check(&p, 96., 4, true, &[(0, 0.48), (0x4c4c4c, 0.32)]);
}

#[test]
fn rowspan_pen_keeps_source_row_ownership_on_each_shared_interval() {
    let data = include_bytes!("../fixtures/issue7353_color_border_review/span.hwpx");
    let p = open(data, 96.);
    // Independent span PDF: at x368.562pt black first at y157.628..185.437,
    // but gray first at y185.437..213.247. The right rowspan starts on row4,
    // so its pen precedes the left cell on row5. Not always left-first.
    check(&p, 96., 4, false, &[(0, 0.48), (0x4c4c4c, 0.32)]);
    check(&p, 96., 5, false, &[(0x4c4c4c, 0.32), (0, 0.48)]);
}
#[test]
fn cell_vector_reordering_preserves_paint_and_geometry() {
    let mut d = rhwp::parse_document(INPUT).unwrap();
    let before = open(&rhwp::serializer::serialize_hwpx(&d).unwrap(), 96.);
    table(&mut d).cells.reverse();
    let after = open(&rhwp::serializer::serialize_hwpx(&d).unwrap(), 96.);
    let lines = |p: &Value| {
        nodes(&p["render_tree"]["root"], "Line")
            .into_iter()
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(lines(&before), lines(&after));
}
#[test]
fn border_colors_do_not_change_cell_or_following_table_geometry() {
    let mut d = rhwp::parse_document(INPUT).unwrap();
    let before = open(INPUT, 96.);
    for bf in &mut d.doc_info.border_fills {
        for b in &mut bf.borders {
            b.color = 0;
        }
    }
    let after = open(&rhwp::serializer::serialize_hwpx(&d).unwrap(), 96.);
    for kind in ["Table", "TableCell", "TextLine", "TextRun"] {
        let boxes = |p: &Value| {
            nodes(&p["render_tree"]["root"], kind)
                .iter()
                .map(|n| n["bbox"].clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(boxes(&before), boxes(&after), "{kind}");
    }
}
