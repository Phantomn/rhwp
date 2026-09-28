//! Normally saved cell Dash borders must reach the common final paint tree.
use rhwp::renderer::table_v2::DocumentV2Session;
use serde_json::Value;

fn collect<'a>(node: &'a Value, kind: &str, out: &mut Vec<&'a Value>) {
    if node["node_type"].get(kind).is_some() {
        out.push(node);
    }
    for child in node["children"].as_array().unwrap() {
        collect(child, kind, out);
    }
}

fn render(data: &[u8], dpi: f64) -> Vec<Value> {
    let mut s = DocumentV2Session::from_bytes(
        data,
        &format!(r#"{{"dpi":{dpi},"max_pages":100,"cell_end_policy":"omit_final_paragraph_gap"}}"#),
    )
    .unwrap();
    let mut pages = Vec::new();
    while let Some(p) = s.next_page_json().unwrap() {
        pages.push(serde_json::from_str(&p).unwrap());
    }
    assert!(s.next_page_json().unwrap().is_none());
    pages
}

#[test]
fn saved_regulatory_table_preserves_dash_edges_and_cell_geometry() {
    let data = include_bytes!("../fixtures/issue7353/cell-dash/prefix31-saved.hwp");
    for dpi in [96., 192.] {
        let pages = render(data, dpi);
        let mut tables = Vec::new();
        for page in &pages {
            collect(&page["render_tree"]["root"], "Table", &mut tables);
        }
        let table = tables
            .iter()
            .find(|n| n["node_type"]["Table"]["para_index"] == 30)
            .unwrap();
        let mut lines = Vec::new();
        collect(table, "Line", &mut lines);
        let dash_len = 20. * dpi / 600.;
        let short: Vec<_> = lines
            .iter()
            .filter(|n| {
                let l = &n["node_type"]["Line"];
                l["y1"] == l["y2"]
                    && (l["x2"].as_f64().unwrap() - l["x1"].as_f64().unwrap() - dash_len).abs()
                        < 1e-8
            })
            .collect();
        // The independent PDF has two dashed horizontal grid boundaries,
        // each continuously spanning columns1..3, not restarting at column2.
        assert!(short.len() > 150);
        let mut ys: Vec<_> = short
            .iter()
            .map(|n| n["node_type"]["Line"]["y1"].as_f64().unwrap())
            .collect();
        ys.sort_by(f64::total_cmp);
        ys.dedup();
        assert_eq!(ys.len(), 2);
        for y in ys {
            let mut xs: Vec<_> = short
                .iter()
                .filter(|n| n["node_type"]["Line"]["y1"] == y)
                .map(|n| n["node_type"]["Line"]["x1"].as_f64().unwrap())
                .collect();
            xs.sort_by(f64::total_cmp);
            for pair in xs.windows(2) {
                assert!((pair[1] - pair[0] - 32. * dpi / 600.).abs() < 1e-8);
            }
        }
        let mut cells = Vec::new();
        collect(table, "TableCell", &mut cells);
        assert_eq!(cells.len(), 10);
        assert!(lines
            .iter()
            .all(|n| n["node_type"]["Line"]["style"]["dash"] == "Solid"));
    }
}

#[test]
fn normal_saved_catalog_pens_reach_selected_table_paint_without_backend_dashes() {
    use rhwp::renderer::table_v2::{Rect, TablePreviewPages, TablePreviewSession, TableSelection};
    let doc = rhwp::parse_document(include_bytes!(
        "../fixtures/issue7353/cell-dash/pen-catalog-saved.hwp"
    ))
    .unwrap();
    // Catalog normal-save source is independent of the output implementation.
    // Its document host insets remain unsupported; this test explicitly selects
    // each table, not the whole catalog's page layout.
    let on_units = [
        17., 20., 26., 34., 43., 52., 69., 86., 104., 121., 173., 259., 347., 520., 693., 866.,
    ];
    for dpi in [96., 192.] {
        for (pi, on) in on_units.into_iter().enumerate() {
            let control = doc.sections[0].paragraphs[pi]
                .controls
                .iter()
                .position(|c| matches!(c, rhwp::model::control::Control::Table(_)))
                .unwrap();
            let mut s = TablePreviewSession::from_document(
                &doc,
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
            let page = s.next_page().unwrap().unwrap();
            assert!(s.next_page().unwrap().is_none());
            let tree = serde_json::to_value(&page.tree).unwrap();
            let mut strokes = Vec::new();
            collect(&tree["root"], "Line", &mut strokes);
            assert!(!strokes.is_empty());
            let mut full = 0;
            let mut keys = std::collections::BTreeSet::new();
            for n in strokes {
                let l = &n["node_type"]["Line"];
                let c = ["x1", "y1", "x2", "y2"].map(|k| l[k].as_f64().unwrap());
                let length = (c[2] - c[0]).abs() + (c[3] - c[1]).abs();
                assert!(length > 0. && length <= on * dpi / 600. + 1e-8);
                if (length - on * dpi / 600.).abs() < 1e-8 {
                    full += 1;
                }
                assert_eq!(l["style"]["dash"], "Solid");
                assert!(keys.insert(c.map(f64::to_bits)), "duplicate stroke");
            }
            assert!(full > 0);
        }
    }
}
