//! Independent normal-save Dot pen catalog and original nested table geometry.
use rhwp::{model::control::Control, renderer::table_v2::*};
use serde_json::Value;
fn collect<'a>(n: &'a Value, kind: &str, out: &mut Vec<&'a Value>) {
    if n["node_type"].get(kind).is_some() {
        out.push(n);
    }
    for c in n["children"].as_array().unwrap() {
        collect(c, kind, out);
    }
}
#[test]
fn normal_dot_catalog_preserves_all_pens_both_axes_and_backend_independent_strokes() {
    let d = rhwp::parse_document(include_bytes!(
        "../fixtures/issue7353_inline_bounds_review/dot-saved.hwp"
    ))
    .unwrap();
    // Independent PDF printer-grid observations in600dpi units, not exported
    // renderer constants. The short final stroke is clipped to its edge endpoint.
    let expected = [
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
    for dpi in [96., 192.] {
        for (pi, (on, off)) in expected.into_iter().enumerate() {
            let control = d.sections[0].paragraphs[pi]
                .controls
                .iter()
                .position(|c| matches!(c, Control::Table(_)))
                .unwrap();
            let mut session = TablePreviewSession::from_document(
                &d,
                TableSelection {
                    section: 0,
                    paragraph: pi,
                    control,
                },
                dpi,
                TablePreviewPages {
                    width: 2100.,
                    height: 2100.,
                    body: Rect {
                        x: 20.,
                        y: 30.,
                        width: 2000.,
                        height: 2000.,
                    },
                    first_y: 30.,
                },
                10,
            )
            .unwrap();
            let page = session.next_page().unwrap().unwrap();
            assert!(session.next_page().unwrap().is_none());
            let tree = serde_json::to_value(page.tree).unwrap();
            let mut nodes = vec![];
            collect(&tree["root"], "Line", &mut nodes);
            for horizontal in [true, false] {
                let axis = if horizontal { "x" } else { "y" };
                let other = if horizontal { "y" } else { "x" };
                let strokes: Vec<_> = nodes
                    .iter()
                    .map(|n| &n["node_type"]["Line"])
                    .filter(|l| l[format!("{other}1")] == l[format!("{other}2")])
                    .collect();
                assert!(strokes.len() > 2);
                assert!(strokes.iter().all(|l| l["style"]["dash"] == "Solid"));
                assert!(strokes
                    .iter()
                    .any(|l| (l[format!("{axis}2")].as_f64().unwrap()
                        - l[format!("{axis}1")].as_f64().unwrap()
                        - on * dpi / 600.)
                        .abs()
                        < 1e-8));
                let first = strokes[0];
                let mut same_axis: Vec<_> = strokes
                    .iter()
                    .filter(|l| l[format!("{other}1")] == first[format!("{other}1")])
                    .collect();
                same_axis.sort_by(|a, b| {
                    a[format!("{axis}1")]
                        .as_f64()
                        .unwrap()
                        .total_cmp(&b[format!("{axis}1")].as_f64().unwrap())
                });
                assert!(same_axis
                    .windows(2)
                    .any(|p| (p[1][format!("{axis}1")].as_f64().unwrap()
                        - p[0][format!("{axis}1")].as_f64().unwrap()
                        - (on + off) * dpi / 600.)
                        .abs()
                        < 1e-8));
            }
        }
    }
}

#[test]
fn normal_nested_table_preserves_all_rows_and_uses_bound_child_height() {
    let data = include_bytes!("../fixtures/issue7353_inline_bounds_review/table-saved.hwp");
    let d = rhwp::parse_document(data).unwrap();
    let parent = d.sections[0].paragraphs[0]
        .controls
        .iter()
        .find_map(|c| {
            if let Control::Table(t) = c {
                Some(t)
            } else {
                None
            }
        })
        .unwrap();
    for dpi in [96., 192.] {
        let mut s = DocumentV2Session::from_bytes(
            data,
            &format!(
                r#"{{"dpi":{dpi},"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}}"#
            ),
        )
        .unwrap();
        let page: Value = serde_json::from_str(&s.next_page_json().unwrap().unwrap()).unwrap();
        assert!(s.next_page_json().unwrap().is_none());
        let mut tables = vec![];
        let mut runs = vec![];
        collect(&page["render_tree"]["root"], "Table", &mut tables);
        collect(&page["render_tree"]["root"], "TextRun", &mut runs);
        assert_eq!(tables.len(), 4);
        let text: String = runs
            .iter()
            .map(|r| r["node_type"]["TextRun"]["text"].as_str().unwrap())
            .collect();
        for expected in ["4조 8,837억원", "1,561억원", "6,355명", "벤처투자(VC)"] {
            assert_eq!(text.matches(expected).count(), 1, "{text}");
        }
        let origin = tables[0]["bbox"]["y"].as_f64().unwrap();
        // Declared parent minimum is a lower bound; every child uses its complete
        // source box. Do not grow a clipped child in paint after reserving less.
        assert!(
            tables[0]["bbox"]["height"].as_f64().unwrap() + 1e-9
                >= parent.common.height as f64 * dpi / 7200.
        );
        for child in &tables[1..] {
            let b = &child["bbox"];
            assert!(b["y"].as_f64().unwrap() >= origin);
            assert!(
                b["y"].as_f64().unwrap() + b["height"].as_f64().unwrap()
                    <= origin + tables[0]["bbox"]["height"].as_f64().unwrap() + 1e-9
            );
        }
    }
}
