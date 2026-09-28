//! Normal Hancom-saved first 14 paragraphs: the row8/9 boundary is visible
//! on PDF p2, and the continued row8 spanning titles are absent on PDF p3.
use rhwp::renderer::table_v2::DocumentV2Session;
use serde_json::Value;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/issue7353/rowbreak-span")
            .join(name),
    )
    .unwrap()
}

fn render(bytes: &[u8]) -> Vec<Value> {
    let mut session = DocumentV2Session::from_bytes(
        bytes,
        r#"{"dpi":96,"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}"#,
    )
    .unwrap();
    let mut pages = Vec::new();
    while let Some(page) = session.next_page_json().unwrap() {
        pages.push(serde_json::from_str(&page).unwrap());
    }
    pages
}

fn nodes<'a>(node: &'a Value, kind: &str) -> Vec<&'a Value> {
    let mut found = Vec::new();
    if node["node_type"].get(kind).is_some() {
        found.push(node);
    }
    for child in node["children"].as_array().unwrap() {
        found.extend(nodes(child, kind));
    }
    found
}

#[test]
fn normal_saved_margin_variants_close_at_frame_without_consuming_the_next_row() {
    use rhwp::model::control::Control;
    let mut ends = Vec::new();
    for name in [
        "prefix14-saved.hwp",
        "margin-saved.hwp",
        "no-bottom-saved.hwp",
    ] {
        let bytes = fixture(name);
        let doc = rhwp::parse_document(&bytes).unwrap();
        let page = &doc.sections[0].section_def.page_def;
        let Control::Table(source) = &doc.sections[0].paragraphs[13].controls[0] else {
            panic!()
        };
        // HWP paper settings reserve footer as well as the lower paper margin.
        let expected = (page.height - page.margin_bottom - page.margin_footer) as f64 / 75.0
            - f64::from(source.common.margin.bottom) / 75.0;
        let pages = render(&bytes);
        assert_eq!(pages.len(), 3);
        let table = nodes(&pages[1]["render_tree"]["root"], "Table")[0];
        let end = table["bbox"]["y"].as_f64().unwrap() + table["bbox"]["height"].as_f64().unwrap();
        assert!((end - expected).abs() < 1e-8, "{name}: {end} vs {expected}");
        ends.push(end);
        let cells = nodes(table, "TableCell");
        for (r, c) in [(8, 0), (8, 1), (9, 2)] {
            let cell = cells
                .iter()
                .find(|n| {
                    n["node_type"]["TableCell"]["row"] == r
                        && n["node_type"]["TableCell"]["col"] == c
                })
                .unwrap();
            assert!(
                (cell["bbox"]["y"].as_f64().unwrap() + cell["bbox"]["height"].as_f64().unwrap()
                    - end)
                    .abs()
                    < 1e-8
            );
        }
        assert!(!cells
            .iter()
            .any(|n| n["node_type"]["TableCell"]["row"] == 10));
        let next = nodes(&pages[2]["render_tree"]["root"], "TableCell");
        let row10 = next
            .iter()
            .find(|n| {
                n["node_type"]["TableCell"]["row"] == 10 && n["node_type"]["TableCell"]["col"] == 2
            })
            .unwrap();
        assert!((row10["bbox"]["y"].as_f64().unwrap() - 77.48).abs() < 1e-8);
        for c in [0, 1] {
            let continued = next
                .iter()
                .find(|n| {
                    n["node_type"]["TableCell"]["row"] == 8
                        && n["node_type"]["TableCell"]["col"] == c
                })
                .unwrap();
            assert!(nodes(continued, "TextLine").is_empty());
        }
    }
    // Independent normal-save PDFs move this edge7.9907px for600HU more
    // page margin, and1.7587px when141HU table margin is removed. Preserve
    // exact source-HU deltas; do not bake PDF printer rounding into layout.
    assert!((ends[0] - ends[1] - 8.0).abs() < 1e-8);
    assert!((ends[2] - ends[0] - 1.88).abs() < 1e-8);
}

#[test]
fn normal_saved_rowbreak_cuts_through_spans_without_replaying_titles() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/issue7353/rowbreak-span/prefix14-saved.hwp");
    let bytes = std::fs::read(path).unwrap();
    let mut session = DocumentV2Session::from_bytes(
        &bytes,
        r#"{"dpi":96,"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}"#,
    )
    .unwrap();
    let mut pages: Vec<Value> = vec![];
    while let Some(page) = session.next_page_json().unwrap() {
        pages.push(serde_json::from_str(&page).unwrap());
    }
    assert_eq!(pages.len(), 3);
    let root = |p: usize| &pages[p]["render_tree"]["root"];
    let at = |p, r, c| {
        nodes(root(p), "TableCell")
            .into_iter()
            .find(|n| {
                n["node_type"]["TableCell"]["row"] == r && n["node_type"]["TableCell"]["col"] == c
            })
            .unwrap_or_else(|| panic!("missing page{p} row{r} col{c}"))
    };
    // PDF p2 ends after the regulated-party row; it is not an intact rowspan group.
    let regulated = at(1, 9, 2);
    assert!(nodes(regulated, "TextRun")
        .iter()
        .any(|n| n["node_type"]["TextRun"]["text"] == "피규제자"));
    for c in [0, 1] {
        let first = at(1, 8, c);
        let continued = at(2, 8, c);
        assert!(!nodes(first, "TextLine").is_empty());
        assert!(nodes(continued, "TextLine").is_empty());
        // Each first-fragment line must fit its actual cell, not merely exist.
        for line in nodes(first, "TextLine") {
            let y = line["bbox"]["y"].as_f64().unwrap();
            let h = line["bbox"]["height"].as_f64().unwrap();
            let b = &first["bbox"];
            assert!(y >= b["y"].as_f64().unwrap() - 1e-8);
            assert!(y + h <= b["y"].as_f64().unwrap() + b["height"].as_f64().unwrap() + 1e-8);
        }
        // Independent paper top margin 5811 HU = 77.48px on continuation page.
        assert!((continued["bbox"]["y"].as_f64().unwrap() - 77.48).abs() < 0.01);
    }
    let unregulated = at(2, 10, 2);
    assert!((unregulated["bbox"]["y"].as_f64().unwrap() - 77.48).abs() < 0.01);
    for p in [1, 2] {
        let table = nodes(root(p), "Table")[0];
        let bottom =
            table["bbox"]["y"].as_f64().unwrap() + table["bbox"]["height"].as_f64().unwrap();
        for cell in table["children"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| n["node_type"].get("TableCell").is_some())
        {
            assert!(
                cell["bbox"]["y"].as_f64().unwrap() + cell["bbox"]["height"].as_f64().unwrap()
                    <= bottom + 1e-8
            );
        }
    }
    assert!(session.next_page_json().unwrap().is_none());
}
