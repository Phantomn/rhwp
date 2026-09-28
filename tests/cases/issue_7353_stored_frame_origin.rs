//! Normal Hancom save: a new paragraph starts the continued cell at its
//! resolved before-spacing, whereas an internal line starts at frame origin.
use rhwp::{
    model::{control::Control, document::Document, table::Table},
    renderer::table_v2::{DocumentV2Session, PreparedTextTable},
};
use serde_json::Value;

const INPUT: &[u8] = include_bytes!("../fixtures/issue7353/frame-origin/paragraph-start-saved.hwp");
fn child(d: &mut Document) -> &mut Table {
    let Control::Table(parent) = &mut d.sections[0].paragraphs[161].controls[0] else {
        panic!()
    };
    let Control::Table(child) = &mut parent.cells[13].paragraphs[0].controls[1] else {
        panic!()
    };
    child
}
fn nodes<'a>(n: &'a Value, kind: &str) -> Vec<&'a Value> {
    let mut out = vec![];
    if n["node_type"].get(kind).is_some() {
        out.push(n);
    }
    for c in n["children"].as_array().unwrap() {
        out.extend(nodes(c, kind));
    }
    out
}
fn text(n: &Value) -> String {
    nodes(n, "TextRun")
        .iter()
        .map(|n| n["node_type"]["TextRun"]["text"].as_str().unwrap())
        .collect()
}
fn b(n: &Value, key: &str) -> f64 {
    n["bbox"][key].as_f64().unwrap()
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}

#[test]
fn normal_saved_paragraph_frame_origin_preserves_before_spacing_once() {
    let mut source = rhwp::parse_document(INPUT).unwrap();
    let t = child(&mut source);
    assert_eq!(t.cells[0].paragraphs[10].line_segs[0].vertical_pos, 100);
    assert!(t.cells[0].paragraphs[9].text.is_empty());
    let expected: String = t.cells[0]
        .paragraphs
        .iter()
        .map(|p| p.text.as_str())
        .collect();
    for dpi in [96., 144.] {
        let scale = dpi / 7200.;
        let mut session = DocumentV2Session::from_bytes(
            INPUT,
            &format!(
                r#"{{"dpi":{dpi},"max_pages":100,"cell_end_policy":"omit_final_paragraph_gap"}}"#
            ),
        )
        .unwrap();
        let mut pages = vec![];
        while let Some(raw) = session.next_page_json().unwrap() {
            pages.push(serde_json::from_str::<Value>(&raw).unwrap());
        }
        assert_eq!(pages.len(), 26);
        assert!(session.next_page_json().unwrap().is_none());
        let mut actual = String::new();
        for (pi, page) in pages.iter().enumerate().skip(24) {
            let parent = nodes(&page["render_tree"]["root"], "Table")
                .into_iter()
                .find(|n| n["node_type"]["Table"]["para_index"] == 161)
                .unwrap();
            let tables = nodes(parent, "Table");
            assert_eq!(tables.len(), 2);
            let nested = tables[1];
            actual.push_str(&text(nested));
            let lines = nodes(nested, "TextLine");
            if pi == 24 {
                let blank = lines.last().unwrap();
                assert!(
                    text(blank).is_empty(),
                    "authored blank must remain on page25"
                );
                near(b(blank, "height"), 1200. * scale);
                assert!(!text(nested).contains("구성 승인 시점부터"));
            } else {
                assert_eq!(lines.len(), 2);
                assert!(text(lines[0]).contains("구성 승인 시점부터"));
                assert_eq!(
                    text(lines[1]),
                    "각 주민대표단별로 약 1년간 운영하는 것으로 가정"
                );
                // Source child padding141HU + resolved before100HU. Do not
                // count stored vpos100 again after the composer emits spacing.
                near(b(lines[0], "y") - b(nested, "y"), (141. + 100.) * scale);
                near(b(lines[1], "y") - b(lines[0], "y"), 1800. * scale);
                // Independent Hancom PDF26: glyph baseline608*.119869pt,
                // outer top/bottom at782.864/731.680pt (paper height841pt).
                let baseline = b(lines[0], "y")
                    + lines[0]["node_type"]["TextLine"]["baseline"]
                        .as_f64()
                        .unwrap();
                assert!((baseline - 608. * 0.119869 * dpi / 72.).abs() < 0.3 * dpi / 96.);
                for (actual, pdf) in [
                    (b(parent, "y"), (841. - 782.864) * dpi / 72.),
                    (
                        b(parent, "y") + b(parent, "height"),
                        (841. - 731.680) * dpi / 72.,
                    ),
                ] {
                    assert!((actual - pdf).abs() <= pdf * (1. - 0.119869 / 0.12) + 0.2 * dpi / 96.);
                }
                assert!(!text(parent).contains("근거설명"));
                let cell = parent["children"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|n| n["node_type"]["TableCell"]["col"] == 1)
                    .unwrap();
                let after = cell["children"].as_array().unwrap().last().unwrap();
                assert!(text(after).is_empty());
                near(b(after, "height"), 1300. * scale);
                near(b(after, "y"), b(nested, "y") + b(nested, "height"));
                near(
                    b(parent, "y") + b(parent, "height"),
                    b(after, "y") + (1300. + 223.) * scale,
                );
            }
            for l in lines {
                assert!(b(l, "y") >= b(nested, "y"));
                assert!(b(l, "y") + b(l, "height") <= b(nested, "y") + b(nested, "height") + 1e-7);
            }
        }
        assert_eq!(actual, expected, "all source units appear exactly once");
    }
}

#[test]
fn arbitrary_nonzero_reset_is_not_a_spacing_contract() {
    for origin in [1, 99, 101, 1200] {
        let mut d = rhwp::parse_document(INPUT).unwrap();
        child(&mut d).cells[0].paragraphs[10].line_segs[0].vertical_pos = origin;
        let styles = rhwp::renderer::style_resolver::resolve_styles(&d.doc_info, 96.);
        let error = PreparedTextTable::prepare(child(&mut d), &styles, 96.)
            .err()
            .unwrap();
        assert!(format!("{error:?}").contains("unqualified stored cell frame reset"));
    }
}

#[test]
fn internal_line_does_not_reapply_paragraph_before_spacing() {
    let bytes = include_bytes!("../fixtures/issue7353/local-column-anchor/prefix163-saved.hwp");
    let mut d = rhwp::parse_document(bytes).unwrap();
    child(&mut d).cells[0].paragraphs[9].line_segs[1].vertical_pos = 100;
    let styles = rhwp::renderer::style_resolver::resolve_styles(&d.doc_info, 96.);
    let error = PreparedTextTable::prepare(child(&mut d), &styles, 96.)
        .err()
        .unwrap();
    assert!(format!("{error:?}").contains("unqualified stored cell frame reset"));
}
