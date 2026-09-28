//! Normal Hancom save: a cell lane and nested filled fields share source slots.
//! Field nesting is ownership, not an extra line, height, or printable command.
use rhwp::{
    model::{control::Control, document::Document, paragraph::Paragraph},
    renderer::table_v2::*,
};
use serde_json::Value;

const INPUT: &[u8] = include_bytes!("../fixtures/issue7353/nested-field/prefix224-saved.hwp");
fn paragraph(d: &mut Document) -> &mut Paragraph {
    let Control::Table(t) = &mut d.sections[0].paragraphs[222].controls[0] else {
        panic!()
    };
    &mut t.cells[0].paragraphs[0]
}
fn nodes<'a>(n: &'a Value, kind: &str) -> Vec<&'a Value> {
    let mut found = vec![];
    if n["node_type"].get(kind).is_some() {
        found.push(n);
    }
    for c in n["children"].as_array().unwrap() {
        found.extend(nodes(c, kind));
    }
    found
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
fn preview(d: &Document, height: f64) -> Result<TablePreviewSession, TablePreviewError> {
    TablePreviewSession::from_document_with_end_policy(
        d,
        TableSelection {
            section: 0,
            paragraph: 222,
            control: 0,
        },
        96.,
        TablePreviewPages {
            width: 794.,
            height: 1123.,
            body: Rect {
                x: 20.,
                y: 30.,
                width: 750.,
                height,
            },
            first_y: 30.,
        },
        10,
        CellEndPolicy::OmitFinalParagraphGap,
    )
}
fn check_lines(lines: &[Value], p: &Paragraph) {
    assert_eq!(lines.len(), p.line_segs.len());
    let chars: Vec<_> = p.text.chars().collect();
    for (i, line) in lines.iter().enumerate() {
        let lo = p
            .char_offsets
            .partition_point(|o| *o < p.line_seg_text_start(i));
        let hi = if i + 1 < p.line_segs.len() {
            p.char_offsets
                .partition_point(|o| *o < p.line_seg_text_start(i + 1))
        } else {
            chars.len()
        };
        assert_eq!(text(line), chars[lo..hi].iter().collect::<String>());
        assert!((b(line, "height") - f64::from(p.line_segs[i].line_height) / 75.).abs() < 1e-7);
    }
    assert_eq!(lines.iter().map(text).collect::<String>(), p.text);
}

#[test]
fn normal_nested_field_preserves_saved_lines_and_actual_cell_bounds() {
    let mut d = rhwp::parse_document(INPUT).unwrap();
    let p = paragraph(&mut d).clone();
    assert_eq!(p.controls.len(), 3);
    assert_eq!(p.field_ranges.len(), 2);
    assert_eq!(p.char_offsets[0], 16); // column slot + outer begin
    assert_eq!(
        p.field_ranges
            .iter()
            .map(|r| r.inner_slot_count)
            .sum::<usize>(),
        1
    );
    let mut session = DocumentV2Session::from_bytes(
        INPUT,
        r#"{"dpi":96,"max_pages":100,"cell_end_policy":"omit_final_paragraph_gap"}"#,
    )
    .unwrap();
    let mut tables = vec![];
    while let Some(raw) = session.next_page_json().unwrap() {
        let page: Value = serde_json::from_str(&raw).unwrap();
        tables.extend(
            nodes(&page["render_tree"]["root"], "Table")
                .into_iter()
                .filter(|t| t["node_type"]["Table"]["para_index"] == 222)
                .cloned(),
        );
    }
    assert!(session.next_page_json().unwrap().is_none());
    assert_eq!(tables.len(), 1);
    let lines: Vec<_> = nodes(&tables[0], "TextLine").into_iter().cloned().collect();
    check_lines(&lines, &p);
    let cell = nodes(&tables[0], "TableCell")[0];
    // Independent Hancom PDF p36: glyph baselines in a 0.12 pt transform.
    // Source integer-printer rounding is below one output pixel at 96 dpi.
    let pdf_baselines = [1931., 2118., 2306., 2494., 2681., 2869.];
    for (i, l) in lines.iter().enumerate() {
        // Same saved row box, no space or paint inserted for nested markers.
        assert!((b(l, "y") - b(cell, "y") - (141. + i as f64 * 2252.) / 75.).abs() < 1e-7);
        assert!(b(l, "y") + b(l, "height") <= b(cell, "y") + b(cell, "height") + 1e-7);
        let run = nodes(l, "TextRun")[0];
        let baseline = b(run, "y") + run["node_type"]["TextRun"]["baseline"].as_f64().unwrap();
        assert!((baseline - pdf_baselines[i] * 0.12 * 96. / 72.).abs() < 1.);
    }
    assert!(!text(cell).contains("상세 근거 제시"));
}

#[test]
fn nested_field_continuation_consumes_each_stored_line_once() {
    let mut d = rhwp::parse_document(INPUT).unwrap();
    let p = paragraph(&mut d).clone();
    let mut s = preview(&d, 80.).unwrap();
    let mut lines = vec![];
    let mut count = 0;
    while let Some(page) = s.next_page().unwrap() {
        count += 1;
        let tree = serde_json::to_value(page.tree).unwrap();
        for line in nodes(&tree["root"], "TextLine") {
            assert!(b(line, "y") >= 30.);
            assert!(b(line, "y") + b(line, "height") <= 110. + 1e-7);
            lines.push(line.clone());
        }
    }
    assert!(count > 1);
    check_lines(&lines, &p);
}

#[test]
fn malformed_nested_slots_and_lane_or_field_states_stay_rejected() {
    for mutation in [
        "crossing",
        "inner_count",
        "slot",
        "char_count",
        "duplicate",
        "guide",
        "multicolumn",
        "edited",
    ] {
        let mut d = rhwp::parse_document(INPUT).unwrap();
        let p = paragraph(&mut d);
        let outer = p
            .field_ranges
            .iter()
            .position(|r| r.control_idx == 1)
            .unwrap();
        let inner = p
            .field_ranges
            .iter()
            .position(|r| r.control_idx == 2)
            .unwrap();
        match mutation {
            "crossing" => p.field_ranges[outer].end_char_idx = 100,
            "inner_count" => p.field_ranges[outer].inner_slot_count = 0,
            "slot" => p.char_offsets[3] -= 1,
            "char_count" => p.char_count += 8,
            "duplicate" => p.field_ranges[inner].control_idx = 1,
            "guide" => {
                let Control::Field(f) = &mut p.controls[2] else {
                    panic!()
                };
                f.properties &= !(1 << 15);
            }
            "multicolumn" => {
                let Control::ColumnDef(c) = &mut p.controls[0] else {
                    panic!()
                };
                c.column_count = 2;
            }
            "edited" => p.line_segs.clear(),
            _ => unreachable!(),
        }
        assert!(preview(&d, 900.).is_err(), "{mutation}");
    }
}
