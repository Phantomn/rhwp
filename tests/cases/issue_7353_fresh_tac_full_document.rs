//! Approved derived input, not an oracle for the untouched #6923 original.
//! See fixture README for independent PDF edges and regeneration provenance.
use std::collections::BTreeMap;

use rhwp::{
    model::{control::Control, paragraph::Paragraph},
    renderer::table_v2::DocumentV2Session,
};
use serde_json::Value;

const INPUT: &[u8] = include_bytes!("../fixtures/issue7353/fresh-tac-full/refreshed-saved.hwp");

fn collect<'a>(node: &'a Value, kind: &str, out: &mut Vec<&'a Value>) {
    if node["node_type"].get(kind).is_some() {
        out.push(node);
    }
    for child in node["children"].as_array().unwrap() {
        collect(child, kind, out);
    }
}

fn nodes<'a>(root: &'a Value, kind: &str) -> Vec<&'a Value> {
    let mut out = Vec::new();
    collect(root, kind, &mut out);
    out
}

fn number(node: &Value, field: &str) -> f64 {
    node["bbox"][field].as_f64().unwrap()
}

fn bottom(node: &Value) -> f64 {
    number(node, "y") + number(node, "height")
}

fn add_text(counts: &mut BTreeMap<char, usize>, text: &str) {
    for ch in text
        .chars()
        .filter(|c| !c.is_whitespace() && !c.is_control())
    {
        *counts.entry(ch).or_default() += 1;
    }
}

fn source_text(paras: &[Paragraph], counts: &mut BTreeMap<char, usize>) {
    for para in paras {
        add_text(counts, &para.text);
        for control in &para.controls {
            if let Control::Table(table) = control {
                for cell in &table.cells {
                    source_text(&cell.paragraphs, counts);
                }
            }
        }
    }
}

#[test]
fn refreshed_document_preserves_parent_edges_fresh_tac_and_body_content() {
    let source = rhwp::parse_document(INPUT).unwrap();
    assert_eq!(source.sections.len(), 1);
    let section = &source.sections[0];
    let Control::Table(parent) = &section.paragraphs[5].controls[0] else {
        panic!("required parent source");
    };
    assert_eq!(parent.cells[0].height, 1000);
    let carrier = &section.paragraphs[29];
    assert!(carrier.line_segs.is_empty(), "must exercise actual reflow");
    let Control::Table(tac) = &carrier.controls[0] else {
        panic!("required fresh TAC source");
    };
    assert!(tac.common.treat_as_char);
    assert_eq!((tac.common.width, tac.common.height), (45918, 11156));

    let mut session = DocumentV2Session::from_bytes(
        INPUT,
        r#"{"dpi":96,"max_pages":20,"cell_end_policy":"omit_final_paragraph_gap"}"#,
    )
    .unwrap();
    let mut pages: Vec<Value> = Vec::new();
    while let Some(page) = session.next_page_json().unwrap() {
        pages.push(serde_json::from_str(&page).unwrap());
        assert!(pages.len() <= 7, "no unowned trailing page");
    }
    assert_eq!(pages.len(), 7, "independent PDF has seven physical pages");
    assert!(session.next_page_json().unwrap().is_none());

    // PDF stroke coordinates at96dpi, not V2 golden output. The PDF driver
    // maps84188HU to841pt. Convert only the numeric reference, never the render.
    let pdf_to_hwp = f64::from(section.section_def.page_def.height) / (841.0 * 100.0);
    let pdf_parent_bottoms = [
        998.588,
        1017.6066666667,
        1008.976,
        1010.2546666667,
        832.3693333333,
    ];
    let mut actual_text = BTreeMap::new();
    let mut fresh_tables = Vec::new();
    for (page_index, page) in pages.iter().enumerate() {
        assert_eq!(page["page_index"], page_index);
        let body = nodes(&page["render_tree"]["root"], "Body")[0];
        let tables = nodes(body, "Table");
        let parents: Vec<_> = tables
            .iter()
            .filter(|t| t["node_type"]["Table"]["para_index"] == 5)
            .collect();
        if page_index < 5 {
            assert_eq!(parents.len(), 1);
            let parent = parents[0];
            let expected = pdf_parent_bottoms[page_index] * pdf_to_hwp;
            assert!(
                // One96dpi output pixel is the independent visual resolution
                // contract, not a tolerance fitted to V2's observed residual.
                (bottom(parent) - expected).abs() < 1.0,
                "page{} parent bottom: {} vs independent PDF {}",
                page_index + 1,
                bottom(parent),
                expected
            );
            assert!(bottom(parent) <= bottom(body) + 1e-8);
            for child in nodes(parent, "Table").into_iter().skip(1) {
                assert!(number(child, "y") >= number(parent, "y") - 1e-8);
                assert!(bottom(child) <= bottom(parent) + 1e-8);
            }
        } else {
            assert!(
                parents.is_empty(),
                "parent must finish before later body tables"
            );
        }
        for table in tables {
            if table["node_type"]["Table"]["para_index"] == 29 {
                fresh_tables.push((page_index, table));
            }
        }
        for run in nodes(body, "TextRun") {
            add_text(
                &mut actual_text,
                run["node_type"]["TextRun"]["text"].as_str().unwrap(),
            );
        }
    }
    assert_eq!(
        fresh_tables.len(),
        1,
        "fresh TAC is not dropped or duplicated"
    );
    let (page_index, table) = fresh_tables[0];
    assert_eq!(
        page_index, 5,
        "Hancom PDF places this TAC on physical page6"
    );
    assert!((number(table, "width") - f64::from(tac.common.width) / 75.0).abs() < 1e-8);
    assert!((number(table, "height") - f64::from(tac.common.height) / 75.0).abs() < 1e-8);
    let body = nodes(&pages[5]["render_tree"]["root"], "Body")[0];
    let suffix = section.paragraphs[30].text.trim();
    assert!(!suffix.is_empty());
    let following = nodes(body, "TextLine")
        .into_iter()
        .find(|line| {
            let text: String = nodes(line, "TextRun")
                .iter()
                .map(|r| r["node_type"]["TextRun"]["text"].as_str().unwrap())
                .collect();
            text.trim() == suffix
        })
        .expect("following source paragraph remains on page6");
    assert!(number(following, "y") >= bottom(table));
    let mut expected_text = BTreeMap::new();
    source_text(&section.paragraphs, &mut expected_text);
    // Character multiplicities prove content preservation, not paragraph order
    // or typography; geometry and the suffix ordering are checked separately.
    assert_eq!(
        actual_text, expected_text,
        "body text omitted or duplicated"
    );
}
