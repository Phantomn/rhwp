//! Intact, filled ClickHere values are stored text, not printable instructions.
//! Independent Hancom title PDF and source LineSeg define two lines, 3600HU apart.
use rhwp::{
    model::{
        control::{Control, FieldType},
        document::Document,
        paragraph::Paragraph,
    },
    renderer::table_v2::*,
};
use serde_json::Value;

const INPUT: &[u8] = include_bytes!("../fixtures/issue7353/filled-field/title-saved.hwp");
const OPTIONS: &str = r#"{"dpi":96,"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}"#;
fn document() -> Document {
    rhwp::parse_document(INPUT).unwrap()
}
fn field_para(d: &mut Document) -> &mut Paragraph {
    let Control::Table(t) = &mut d.sections[0].paragraphs[5].controls[0] else {
        panic!()
    };
    &mut t.cells[0].paragraphs[0]
}
fn preview(d: &Document) -> Result<TablePreviewSession, TablePreviewError> {
    TablePreviewSession::from_document_with_end_policy(
        d,
        TableSelection {
            section: 0,
            paragraph: 5,
            control: 0,
        },
        96.0,
        TablePreviewPages {
            width: 794.0,
            height: 1123.0,
            body: Rect {
                x: 20.0,
                y: 30.0,
                width: 750.0,
                height: 1000.0,
            },
            first_y: 100.0,
        },
        10,
        CellEndPolicy::OmitFinalParagraphGap,
    )
}
fn collect<'a>(n: &'a Value, kind: &str, result: &mut Vec<&'a Value>) {
    if n["node_type"].get(kind).is_some() {
        result.push(n)
    }
    if let Some(children) = n["children"].as_array() {
        for c in children {
            collect(c, kind, result)
        }
    }
}
fn nodes<'a>(n: &'a Value, kind: &str) -> Vec<&'a Value> {
    let mut v = vec![];
    collect(n, kind, &mut v);
    v
}
fn text(n: &Value) -> String {
    nodes(n, "TextRun")
        .iter()
        .map(|r| r["node_type"]["TextRun"]["text"].as_str().unwrap())
        .collect()
}
fn coord(n: &Value, k: &str) -> f64 {
    n["bbox"][k].as_f64().unwrap()
}
fn check_title(tree: &Value) {
    let cells: Vec<_> = nodes(tree, "TableCell")
        .into_iter()
        .filter(|cell| text(cell).starts_with("노후계획도시"))
        .collect();
    assert_eq!(cells.len(), 1);
    let lines = nodes(cells[0], "TextLine");
    assert_eq!(lines.len(), 2);
    assert_eq!(text(lines[0]), "노후계획도시 정비 및 지원에 관");
    assert_eq!(text(lines[1]), "한 특별법 시행령");
    assert!((coord(lines[1], "y") - coord(lines[0], "y") - 48.0).abs() < 1e-7);
    for line in lines {
        assert!((coord(line, "height") - 40.0).abs() < 1e-7);
        for run in nodes(line, "TextRun") {
            assert!(
                (run["node_type"]["TextRun"]["baseline"].as_f64().unwrap() - 34.0).abs() < 1e-7
            );
            for (axis, size) in [("x", "width"), ("y", "height")] {
                assert!(coord(run, axis) >= coord(cells[0], axis) - 1e-7);
                assert!(
                    coord(run, axis) + coord(run, size)
                        <= coord(cells[0], axis) + coord(cells[0], size) + 1e-7
                );
            }
        }
    }
    assert!(!text(tree).contains("안건명"));
    assert!(!text(tree).contains("Clickhere:"));
}

#[test]
fn filled_title_document_keeps_two_saved_lines_and_terminates() {
    let mut session = DocumentV2Session::from_bytes(INPUT, OPTIONS).unwrap();
    let page: Value = serde_json::from_str(&session.next_page_json().unwrap().unwrap()).unwrap();
    check_title(&page["render_tree"]["root"]);
    // Independent PDF trace: .119935/.119869 text transform; origins
    // (607,1803) and (1482,2103). Page/device quantization is below one96dpi px.
    let title = nodes(&page["render_tree"]["root"], "TableCell")
        .into_iter()
        .find(|c| text(c).starts_with("노후계획도시"))
        .unwrap();
    for (line, (x, y)) in nodes(title, "TextLine")
        .iter()
        .zip([(607.0, 1803.0), (1482.0, 2103.0)])
    {
        let run = nodes(line, "TextRun")[0];
        assert!((coord(run, "x") - x * 0.119935 * 96.0 / 72.0).abs() < 1.0);
        let baseline = coord(run, "y") + run["node_type"]["TextRun"]["baseline"].as_f64().unwrap();
        assert!((baseline - y * 0.119869 * 96.0 / 72.0).abs() < 1.0);
    }
    // Title cell clip/outer boundary in the same independent PDF, top-down pt.
    for (axis, pt) in [
        ("x", 58.049),
        ("y", 841.0 - 651.727),
        ("width", 537.431 - 58.049),
        ("height", 651.727 - 582.922),
    ] {
        assert!(
            (coord(title, axis) - pt * 96.0 / 72.0).abs() < 1.0,
            "{axis}"
        );
    }
    assert!(session.next_page_json().unwrap().is_none());
    assert!(session.next_page_json().unwrap().is_none());
}

#[test]
fn original_hwp_and_hwpx_title_cells_keep_geometry_and_field_ir() {
    for path in [
        "samples/86712_regulatory_analysis.hwp",
        "samples/issue1891/86712_regulatory_analysis.hwpx",
    ] {
        let d = rhwp::parse_document(
            &std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap(),
        )
        .unwrap();
        let before = serde_json::to_value(&d.sections[0].paragraphs).unwrap();
        let mut s = preview(&d).unwrap();
        check_title(&serde_json::to_value(s.next_page().unwrap().unwrap().tree).unwrap()["root"]);
        assert!(s.next_page().unwrap().is_none());
        assert_eq!(
            before,
            serde_json::to_value(&d.sections[0].paragraphs).unwrap()
        );
    }
}

#[test]
fn filled_field_state_does_not_admit_initial_guides_or_invalid_saved_partitions() {
    for case in [
        "initial", "ranges", "slots", "count", "edited", "lines", "empty", "outside", "inner",
        "overlap", "orphan",
    ] {
        let mut d = document();
        let p = field_para(&mut d);
        match case {
            "initial" => {
                let Control::Field(f) = &mut p.controls[0] else {
                    panic!()
                };
                f.properties &= !(1 << 15)
            }
            "ranges" => p.field_ranges.clear(),
            "slots" => p.char_offsets[0] = 0,
            "count" => p.char_count -= 8,
            "edited" => p.stored_text_partition_dirty = true,
            "lines" => p.line_segs.clear(),
            "empty" => p.field_ranges[0].end_char_idx = 0,
            "outside" => p.field_ranges[0].end_char_idx = 100,
            "inner" => p.field_ranges[0].inner_slot_count = 1,
            "overlap" => {
                p.controls.push(p.controls[0].clone());
                let mut r = p.field_ranges[0].clone();
                r.control_idx = 1;
                p.field_ranges.push(r)
            }
            "orphan" => p.orphan_field_ends.push(Default::default()),
            _ => unreachable!(),
        }
        assert!(
            matches!(
                preview(&d),
                Err(TablePreviewError::Geometry(GeometryError::Unsupported(
                    "unqualified stored field result"
                )))
            ),
            "{case}"
        );
    }
}

#[test]
fn filled_clickhere_admission_does_not_admit_unqualified_field_kinds() {
    let mut d = document();
    let Control::Field(f) = &mut field_para(&mut d).controls[0] else {
        panic!()
    };
    // Closed hyperlink results now have their own normal-save contract in
    // issue_7353_stored_hyperlink. Keep the unrelated field-kind exclusion.
    f.field_type = FieldType::Date;
    assert!(matches!(
        preview(&d),
        Err(TablePreviewError::Geometry(GeometryError::Unsupported(
            "non-table cell control"
        )))
    ));
}

#[test]
fn field_with_embedded_object_is_not_flattened_into_plain_stored_text() {
    for object in [
        Control::Table(Box::default()),
        Control::Picture(Default::default()),
    ] {
        let mut d = document();
        let p = field_para(&mut d);
        p.controls.push(object);
        p.field_ranges[0].inner_slot_count = 1;
        let before = serde_json::to_value(&d.sections[0].paragraphs).unwrap();
        assert!(matches!(
            preview(&d),
            Err(TablePreviewError::Geometry(GeometryError::Unsupported(
                "non-table cell control"
            )))
        ));
        assert_eq!(
            before,
            serde_json::to_value(&d.sections[0].paragraphs).unwrap()
        );
    }
}
