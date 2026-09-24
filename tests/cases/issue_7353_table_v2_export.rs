//! Byte/JSON boundary shared with the real WASM class, not a document engine switch.
use rhwp::{
    model::{
        control::Control,
        document::{Document, Section},
        paragraph::{CharShapeRef, LineSeg, Paragraph},
        shape::{CommonObjAttr, HorzRelTo, TextWrap, VertRelTo},
        style::{CharShape, LineSpacingType, ParaShape},
        table::{Cell, Table, TablePageBreak},
    },
    renderer::table_v2::{TablePreviewExportError, TablePreviewExportSession},
};
use serde_json::{json, Value};

fn paragraph(text: &str) -> Paragraph {
    Paragraph {
        text: text.into(),
        char_count: text.len() as u32,
        char_offsets: (0..text.len() as u32).collect(),
        char_shapes: vec![CharShapeRef {
            start_pos: 0,
            char_shape_id: 0,
        }],
        ..Default::default()
    }
}
fn table() -> Table {
    Table {
        row_count: 3,
        col_count: 2,
        repeat_header: true,
        page_break: TablePageBreak::CellBreak,
        common: CommonObjAttr {
            text_wrap: TextWrap::TopAndBottom,
            horz_rel_to: HorzRelTo::Para,
            vert_rel_to: VertRelTo::Para,
            ..Default::default()
        },
        cells: ["title", "A", "B"]
            .into_iter()
            .enumerate()
            .map(|(row, text)| Cell {
                row: row as u16,
                col: 0,
                row_span: 1,
                col_span: 2,
                width: 15000,
                is_header: row == 0,
                paragraphs: vec![paragraph(text)],
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}
fn host(text: &str, table: Table) -> Paragraph {
    let mut p = paragraph(text);
    p.char_count += 8;
    p.char_offsets.iter_mut().for_each(|v| *v += 8);
    p.controls.push(Control::Table(Box::new(table)));
    p
}
fn document(t: Table) -> Document {
    let mut d = Document::default();
    d.doc_info.char_shapes.push(CharShape {
        base_size: 900,
        ..Default::default()
    });
    d.doc_info.para_shapes.push(ParaShape {
        line_spacing_type: LineSpacingType::Fixed,
        line_spacing: 2700,
        ..Default::default()
    });
    d.sections = vec![Section {
        paragraphs: vec![host("", t)],
        ..Default::default()
    }];
    d
}
fn bytes(t: Table) -> Vec<u8> {
    rhwp::serializer::hwpx::serialize_hwpx(&document(t)).unwrap()
}
fn options(data: &[u8]) -> Value {
    // HWPX reopening inserts section controls; addresses refer to parsed IR,
    // not to the pre-serialization fixture's control array.
    let parsed = rhwp::parse_document(data).unwrap();
    let control = parsed.sections[0].paragraphs[0]
        .controls
        .iter()
        .position(|c| matches!(c, Control::Table(_)))
        .unwrap();
    json!({"selection":{"section":0,"paragraph":0,"control":control},"dpi":96,
        "pages":{"width":400,"height":400,"body":{"x":20,"y":30,"width":300,"height":36},"first_y":30},
        "max_pages":100})
}
fn open(data: &[u8], config: &Value) -> TablePreviewExportSession {
    TablePreviewExportSession::from_bytes(data, &config.to_string()).unwrap()
}
fn walk<'a>(node: &'a Value, kind: &str, out: &mut Vec<&'a Value>) {
    if node["node_type"].get(kind).is_some() {
        out.push(node);
    }
    for child in node["children"].as_array().unwrap() {
        walk(child, kind, out);
    }
}
fn collect<'a>(page: &'a Value, kind: &str) -> Vec<&'a Value> {
    let mut result = Vec::new();
    walk(&page["render_tree"]["root"], kind, &mut result);
    result
}
fn labels(page: &Value) -> Vec<&str> {
    collect(page, "TextRun")
        .into_iter()
        .map(|n| n["node_type"]["TextRun"]["text"].as_str().unwrap())
        .collect()
}
fn drain(s: &mut TablePreviewExportSession) -> Vec<Value> {
    let mut result = Vec::new();
    while let Some(page) = s.next_page_json().unwrap() {
        result.push(serde_json::from_str(&page).unwrap());
    }
    result
}
fn capture(name: &str, data: &[u8], config: &Value, pages: &[Value]) {
    if let Ok(dir) = std::env::var("ISSUE7353_EXPORT_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/{name}.hwpx"), data).unwrap();
        std::fs::write(format!("{dir}/{name}.options.json"), config.to_string()).unwrap();
        std::fs::write(
            format!("{dir}/{name}.native.json"),
            serde_json::to_vec_pretty(pages).unwrap(),
        )
        .unwrap();
        for (i, page) in pages.iter().enumerate() {
            std::fs::write(
                format!("{dir}/{name}.native-{i}.svg"),
                page["svg"].as_str().unwrap(),
            )
            .unwrap();
        }
    }
}

#[test]
fn byte_transport_emits_actual_tree_and_svg_with_header_colspan_and_end() {
    let data = bytes(table());
    let mut s = open(&data, &options(&data));
    assert_eq!(s.emitted_pages(), 0);
    let pages = drain(&mut s);
    assert_eq!(pages.len(), 2);
    assert_eq!(labels(&pages[0]), ["title", "A"]);
    assert_eq!(labels(&pages[1]), ["title", "B"]);
    for (i, page) in pages.iter().enumerate() {
        assert_eq!(page["schema_version"], 1);
        assert_eq!(page["engine"], "table_v2");
        assert_eq!(page["scope"], "selected_table");
        assert_eq!(page["page_index"], i);
        assert!(page["svg"].as_str().unwrap().contains("<svg"));
        let cells = collect(page, "TableCell");
        assert_eq!(cells.len(), 2);
        for (j, c) in cells.iter().enumerate() {
            assert_eq!(c["node_type"]["TableCell"]["col_span"], 2);
            assert_eq!(c["bbox"]["x"], 20.0);
            assert_eq!(c["bbox"]["y"], 30.0 + j as f64 * 18.0);
            assert_eq!(c["bbox"]["width"], 200.0);
            assert_eq!(c["bbox"]["height"], 18.0);
        }
    }
    assert_eq!(s.emitted_pages(), 2);
    assert!(s.next_page_json().unwrap().is_none());
    capture("merged", &data, &options(&data), &pages);
}

#[test]
fn nested_snapshot_survives_input_buffer_change_and_preserves_following_paragraphs() {
    let mut parent = table();
    parent.row_count = 1;
    parent.repeat_header = false;
    parent.cells.truncate(1);
    parent.cells[0].is_header = false;
    parent.cells[0].paragraphs = vec![host("host", table()), paragraph("after")];
    let mut data = bytes(parent);
    let original = data.clone();
    let config = options(&data);
    let mut s = open(&data, &config);
    data.fill(0);
    let pages = drain(&mut s);
    assert_eq!(pages.len(), 3);
    assert_eq!(labels(&pages[0]), ["title", "A"]);
    assert_eq!(labels(&pages[1]), ["title", "B"]);
    assert_eq!(labels(&pages[2]), ["host", "after"]);
    let mut reopened = open(&original, &config);
    assert_eq!(drain(&mut reopened), pages);
    capture("nested", &original, &config, &pages);
}

#[test]
fn partial_first_page_does_not_manufacture_blank_output() {
    let data = bytes(table());
    let mut config = options(&data);
    config["pages"]["first_y"] = json!(60);
    let pages = drain(&mut open(&data, &config));
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0]["page_index"], 1);
    assert_eq!(pages[1]["page_index"], 2);
    capture("partial", &data, &config, &pages);
}

#[test]
fn fit_and_limit_errors_do_not_advance_or_fallback() {
    let data = bytes(table());
    let mut config = options(&data);
    config["max_pages"] = json!(1);
    let mut s = open(&data, &config);
    assert!(s.next_page_json().unwrap().is_some());
    for _ in 0..2 {
        assert!(format!("{:?}", s.next_page_json().unwrap_err()).contains("PageLimit"));
        assert_eq!(s.emitted_pages(), 1);
    }
    config["pages"]["body"]["height"] = json!(18);
    let mut s = open(&data, &config);
    for _ in 0..2 {
        assert!(format!("{:?}", s.next_page_json().unwrap_err()).contains("DoesNotFit"));
        assert_eq!(s.emitted_pages(), 0);
    }
}

#[test]
fn strict_options_reject_missing_unknown_overflow_and_invalid_geometry() {
    let data = bytes(table());
    for bad in ["{}", "[]", "null", "{broken"] {
        assert!(matches!(
            TablePreviewExportSession::from_bytes(&data, bad),
            Err(TablePreviewExportError::Options(_))
        ));
    }
    for mode in 0..8 {
        let mut c = options(&data);
        match mode {
            0 => {
                c["engine"] = json!("legacy");
            }
            1 => {
                c["pages"]["body"]["widht"] = json!(300);
            }
            2 => {
                c["selection"]["control"] = json!(-1);
            }
            3 => {
                c["max_pages"] = json!(4294967296_u64);
            }
            4 => {
                c["dpi"] = json!(0);
            }
            5 => {
                c["pages"]["body"]["height"] = json!(500);
            }
            6 => {
                c["max_pages"] = json!(0);
            }
            _ => {
                c["selection"]["control"] = json!(999);
            }
        }
        assert!(
            TablePreviewExportSession::from_bytes(&data, &c.to_string()).is_err(),
            "{c}"
        );
    }
    assert!(matches!(
        TablePreviewExportSession::from_bytes(b"invalid", &options(&data).to_string()),
        Err(TablePreviewExportError::Parse(_))
    ));
}

#[test]
fn stored_lines_and_unsupported_rowspan_are_not_silently_reflowed() {
    for name in ["stored", "rowspan"] {
        let mut t = table();
        if name == "stored" {
            t.cells[0].paragraphs[0].line_segs = vec![LineSeg {
                line_height: 1350,
                text_height: 900,
                baseline_distance: 720,
                line_spacing: 450,
                segment_width: 15000,
                tag: LineSeg::TAG_SINGLE_SEGMENT_LINE,
                ..Default::default()
            }];
        } else {
            t.cells[0].row_span = 2;
        }
        let data = bytes(t);
        let config = options(&data);
        let parsed = rhwp::parse_document(&data).unwrap();
        let Control::Table(reopened) = &parsed.sections[0].paragraphs[0].controls
            [config["selection"]["control"].as_u64().unwrap() as usize]
        else {
            panic!("table");
        };
        if name == "stored" {
            assert_eq!(reopened.cells[0].paragraphs[0].line_segs.len(), 1);
        } else {
            assert_eq!(reopened.cells[0].row_span, 2);
        }
        let result = TablePreviewExportSession::from_bytes(&data, &config.to_string()).map(drop);
        assert!(
            matches!(
                result,
                Err(TablePreviewExportError::Preview(
                    rhwp::renderer::table_v2::TablePreviewError::Geometry(
                        rhwp::renderer::table_v2::GeometryError::Unsupported(_)
                    )
                ))
            ),
            "{name}: {result:?}"
        );
        capture(name, &data, &config, &[]);
    }
}
