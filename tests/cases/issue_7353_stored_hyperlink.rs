//! Normal Hancom save: literal hyperlink results, marker-aligned row starts,
//! and a stored page cut inside a field-bearing paragraph. PDF has21pages.
use rhwp::{
    model::{control::Control, document::Document, paragraph::Paragraph},
    renderer::table_v2::*,
};
use serde_json::Value;
const INPUT: &[u8] = include_bytes!("../fixtures/issue7353/cell-control/prefix102-saved.hwp");
const OPTIONS: &str = r#"{"dpi":96,"max_pages":100,"cell_end_policy":"omit_final_paragraph_gap"}"#;
fn nodes<'a>(n: &'a Value, k: &str) -> Vec<&'a Value> {
    let mut v = vec![];
    if n["node_type"].get(k).is_some() {
        v.push(n);
    }
    for c in n["children"].as_array().unwrap() {
        v.extend(nodes(c, k));
    }
    v
}
fn text(n: &Value) -> String {
    nodes(n, "TextRun")
        .iter()
        .map(|r| r["node_type"]["TextRun"]["text"].as_str().unwrap())
        .collect()
}
fn table(d: &mut Document) -> &mut rhwp::model::table::Table {
    let Control::Table(t) = &mut d.sections[0].paragraphs[101].controls[0] else {
        panic!()
    };
    t
}
fn para(d: &mut Document, i: usize) -> &mut Paragraph {
    &mut table(d).cells[0].paragraphs[i]
}
fn preview(d: &Document) -> Result<TablePreviewSession, TablePreviewError> {
    TablePreviewSession::from_document_with_end_policy(
        d,
        TableSelection {
            section: 0,
            paragraph: 101,
            control: 0,
        },
        96.,
        TablePreviewPages {
            width: 794.,
            height: 1123.,
            body: Rect {
                x: 75.6,
                y: 75.6,
                width: 645.,
                height: 971.,
            },
            first_y: 365.73333333333335,
        },
        100,
        CellEndPolicy::OmitFinalParagraphGap,
    )
}
fn partitions(p: &Paragraph) -> Vec<String> {
    let chars: Vec<_> = p.text.chars().collect();
    (0..p.line_segs.len())
        .map(|i| {
            let lo = p
                .char_offsets
                .partition_point(|v| *v < p.line_seg_text_start(i));
            let hi = if i + 1 < p.line_segs.len() {
                p.char_offsets
                    .partition_point(|v| *v < p.line_seg_text_start(i + 1))
            } else {
                chars.len()
            };
            chars[lo..hi].iter().collect()
        })
        .collect()
}
fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-7, "{actual} != {expected}");
}
#[test]
fn normal_saved_document_preserves_every_cell_line_and_page_cut() {
    let mut d = rhwp::parse_document(INPUT).unwrap();
    let t = table(&mut d);
    assert_eq!(t.cells[0].height, 282);
    assert_eq!(
        t.cells[0].vertical_align,
        rhwp::model::table::VerticalAlign::Center
    );
    let expected: Vec<_> = t.cells[0].paragraphs.iter().flat_map(partitions).collect();
    let source_text: String = t.cells[0]
        .paragraphs
        .iter()
        .map(|p| p.text.as_str())
        .collect();
    let p = &t.cells[0].paragraphs[75];
    assert_eq!(p.line_segs[1].text_start, 128);
    assert!(!p.char_offsets.contains(&128));
    assert_eq!(p.char_offsets[48], 136); // eight-unit start marker before first glyph
    let continued = partitions(&t.cells[0].paragraphs[69]);
    assert_eq!(t.cells[0].paragraphs[69].line_segs[2].vertical_pos, 0);
    let mut session = DocumentV2Session::from_bytes(INPUT, OPTIONS).unwrap();
    let mut pages = vec![];
    while let Some(p) = session.next_page_json().unwrap() {
        pages.push(serde_json::from_str::<Value>(&p).unwrap());
    }
    assert_eq!(pages.len(), 21);
    assert!(session.next_page_json().unwrap().is_none());
    let mut actual = vec![];
    let mut continuity = vec![];
    for (pi, page) in pages.iter().enumerate() {
        let root = &page["render_tree"]["root"];
        for t in nodes(root, "Table")
            .into_iter()
            .filter(|t| t["node_type"]["Table"]["para_index"] == 101)
        {
            let cells = nodes(t, "TableCell");
            assert_eq!(cells.len(), 1);
            let cell = cells[0];
            let cy = cell["bbox"]["y"].as_f64().unwrap();
            let bottom = cy + cell["bbox"]["height"].as_f64().unwrap();
            let body = nodes(root, "Body")[0];
            assert!(
                bottom
                    <= body["bbox"]["y"].as_f64().unwrap()
                        + body["bbox"]["height"].as_f64().unwrap()
                        + 1e-7
            );
            if pi == 17 {
                // Source body origin + paragraph vpos + physical top margin.
                near(cy, (5670. + 21760. + 283.) / 75.);
            }
            if pi == 20 {
                // Independent PDF21 horizontal edges, page height841pt.
                // One96dpi pixel allows printer quantization, not a layout fix.
                assert!((cy - (841. - 781.425) * 4. / 3.).abs() < 1.);
                assert!((bottom - (841. - 522.149) * 4. / 3.).abs() < 1.);
            }
            for line in nodes(cell, "TextLine") {
                let value = text(line);
                let y = line["bbox"]["y"].as_f64().unwrap();
                let h = line["bbox"]["height"].as_f64().unwrap();
                assert!(y >= cy - 1e-7 && y + h <= bottom + 1e-7);
                near(h, 16.); // saved1200HU, not measured output as oracle
                if continued.contains(&value) {
                    continuity.push((pi + 1, value.clone(), y - cy));
                }
                actual.push(value);
            }
        }
    }
    assert_eq!(actual, expected); // includes empty lines, all slots exactly once
    assert_eq!(actual.concat(), source_text);
    assert!(!actual.concat().contains("javascript"));
    assert_eq!(
        continuity.iter().map(|(p, _, _)| *p).collect::<Vec<_>>(),
        vec![20, 20, 21]
    );
    near(continuity[2].2, 141. / 75.); // saved cell top padding on continued frame
}
#[test]
fn invalid_link_ranges_and_marker_interiors_are_not_accepted() {
    for mode in 0..5 {
        let mut d = rhwp::parse_document(INPUT).unwrap();
        let p = para(&mut d, 75);
        match mode {
            0 => p.line_segs[1].text_start = 129, // inside eight-unit marker
            1 => p.field_ranges[0].end_char_idx = 0,
            2 => p.field_ranges[1].control_idx = 0,
            3 => p.char_offsets[48] += 1,
            _ => p.field_ranges.clear(), // open hyperlinks are not ClickHere prefix results
        }
        assert!(preview(&d).is_err(), "mode{mode}");
    }
}
#[test]
fn neutral_alignment_and_opaque_commands_do_not_change_geometry() {
    let d = rhwp::parse_document(INPUT).unwrap();
    let collect = |d: &Document| {
        let mut s = preview(d).unwrap();
        let mut pages = vec![];
        while let Some(raw) = s.next_page().unwrap() {
            pages.push(serde_json::to_value(raw.tree).unwrap());
        }
        pages
    };
    let expected = collect(&d);
    for align in [
        rhwp::model::table::VerticalAlign::Top,
        rhwp::model::table::VerticalAlign::Bottom,
    ] {
        let mut changed = d.clone();
        table(&mut changed).cells[0].vertical_align = align;
        assert_eq!(collect(&changed), expected);
    }
    let mut changed = d.clone();
    for p in &mut table(&mut changed).cells[0].paragraphs {
        for c in &mut p.controls {
            if let Control::Field(f) = c {
                f.command = "https://example.invalid/not-requested".into();
            }
        }
    }
    assert_eq!(collect(&changed), expected); // display geometry never evaluates command
}
#[test]
fn genuine_alignment_band_and_never_split_remain_atomic() {
    use rhwp::model::table::TablePageBreak;
    let mut d = rhwp::parse_document(INPUT).unwrap();
    // Independent declaration creates real slack beyond this long content.
    table(&mut d).cells[0].height = 300_000;
    let mut s = preview(&d).unwrap();
    assert!(s.next_page().is_err());
    let mut d = rhwp::parse_document(INPUT).unwrap();
    table(&mut d).page_break = TablePageBreak::None;
    let mut s = preview(&d).unwrap();
    assert!(s.next_page().is_err());
}
