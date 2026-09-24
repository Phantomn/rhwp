//! Byte/JSON boundary shared with the real WASM class, not a document engine switch.
use rhwp::{
    model::{
        control::Control,
        document::{Document, Section},
        paragraph::{CharShapeRef, LineSeg, Paragraph},
        shape::{CommonObjAttr, HorzRelTo, TextWrap, VertRelTo},
        style::{
            BorderFill, BorderLine, BorderLineType, CharShape, Fill, FillType, LineSpacingType,
            ParaShape, SolidFill,
        },
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

fn solid(color: u32) -> BorderFill {
    BorderFill {
        borders: [BorderLine {
            line_type: BorderLineType::None,
            ..Default::default()
        }; 4],
        fill: Fill {
            fill_type: FillType::Solid,
            solid: Some(SolidFill {
                background_color: color,
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn colored_document(t: Table) -> Document {
    let mut d = document(t);
    // COLORREF is BGR: pale red / pale green / pale blue, then transparent / white.
    d.doc_info.border_fills = vec![
        solid(0xEEEEFF),
        solid(0xEEFFEE),
        solid(0xFFEEEE),
        solid(0xFFFFFFFF),
        solid(0xFFFFFF),
    ];
    d
}
fn colored_pages(name: &str, d: &Document) -> Vec<Value> {
    let data = rhwp::serializer::hwpx::serialize_hwpx(d).unwrap();
    let config = options(&data);
    let pages = drain(&mut open(&data, &config));
    capture(name, &data, &config, &pages);
    pages
}
fn verify_background(node: &Value, color: u32) {
    let background = &node["children"][0];
    assert_eq!(
        background["bbox"], node["bbox"],
        "paint uses final reserved fragment, including padding"
    );
    let style = &background["node_type"]["Rectangle"]["style"];
    assert_eq!(style["fill_color"], color);
    assert_eq!(style["stroke_color"], Value::Null);
    assert_eq!(style["stroke_width"], 0.0);
}

#[test]
fn solid_backgrounds_cover_colspan_and_repeated_headers_without_changing_geometry() {
    let mut t = table();
    t.border_fill_id = 3;
    for c in &mut t.cells {
        c.border_fill_id = if c.row == 0 { 1 } else { 2 };
    }
    let pages = colored_pages("fill-merged", &colored_document(t));
    assert_eq!(pages.len(), 2);
    for (i, page) in pages.iter().enumerate() {
        assert_eq!(labels(page), ["title", if i == 0 { "A" } else { "B" }]);
        let tables = collect(page, "Table");
        verify_background(tables[0], 0xFFEEEE);
        assert_eq!(tables[0]["bbox"]["height"], 36.0);
        for (j, cell) in collect(page, "TableCell").iter().enumerate() {
            verify_background(cell, if j == 0 { 0xEEEEFF } else { 0xEEFFEE });
            assert_eq!(cell["node_type"]["TableCell"]["col_span"], 2);
            assert_eq!(cell["bbox"]["x"], 20.0);
            assert_eq!(cell["bbox"]["y"], 30.0 + 18.0 * j as f64);
            assert_eq!(cell["bbox"]["width"], 200.0);
            assert_eq!(cell["bbox"]["height"], 18.0);
        }
        assert_eq!(collect(page, "Rectangle").len(), 3);
        let svg = page["svg"].as_str().unwrap();
        assert!(svg.contains("#ffeeee") && svg.contains("#eeffee") && svg.contains("#eeeeff"));
    }
}

#[test]
fn split_cell_fill_includes_empty_line_and_padding_without_duplication_or_height_change() {
    let mut t = table();
    t.row_count = 1;
    t.repeat_header = false;
    t.cells.truncate(1);
    t.padding.left = 750;
    t.padding.right = 750; // 10px each, not subtracted from background.
    let c = &mut t.cells[0];
    c.is_header = false;
    c.border_fill_id = 2;
    c.paragraphs = vec![
        paragraph("A"),
        paragraph(""),
        paragraph("B"),
        paragraph("C"),
    ];
    let pages = colored_pages("fill-split", &colored_document(t));
    assert_eq!(pages.len(), 2);
    assert_eq!(labels(&pages[0]), ["A", ""]);
    assert_eq!(labels(&pages[1]), ["B", "C"]);
    for page in &pages {
        let cells = collect(page, "TableCell");
        assert_eq!(cells.len(), 1);
        verify_background(cells[0], 0xEEFFEE);
        assert_eq!(cells[0]["bbox"]["x"], 20.0);
        assert_eq!(cells[0]["bbox"]["width"], 200.0);
        assert_eq!(cells[0]["bbox"]["y"], 30.0);
        assert_eq!(cells[0]["bbox"]["height"], 36.0);
        let lines = collect(page, "TextLine");
        assert_eq!(lines.len(), 2, "blank paragraph occupies a line");
        for (i, line) in lines.iter().enumerate() {
            assert_eq!(line["bbox"]["x"], 30.0);
            assert_eq!(line["bbox"]["y"], 30.0 + 18.0 * i as f64);
        }
        assert_eq!(collect(page, "Rectangle").len(), 1);
    }
}

#[test]
fn nested_backgrounds_remain_below_child_and_following_host_content() {
    let mut child = table();
    for c in &mut child.cells {
        c.border_fill_id = if c.row == 0 { 2 } else { 3 };
    }
    let mut parent = table();
    parent.row_count = 1;
    parent.repeat_header = false;
    parent.cells.truncate(1);
    parent.cells[0].is_header = false;
    parent.cells[0].border_fill_id = 1;
    parent.cells[0].paragraphs = vec![host("host", child), paragraph("after")];
    let pages = colored_pages("fill-nested", &colored_document(parent));
    assert_eq!(pages.len(), 3);
    assert_eq!(labels(&pages[2]), ["host", "after"]);
    for (i, page) in pages.iter().enumerate() {
        let cells = collect(page, "TableCell");
        verify_background(cells[0], 0xEEEEFF);
        assert_eq!(cells[0]["bbox"]["height"], 36.0);
        if i < 2 {
            assert_eq!(labels(page), ["title", if i == 0 { "A" } else { "B" }]);
            verify_background(cells[1], 0xEEFFEE);
            verify_background(cells[2], 0xFFEEEE);
            assert_eq!(
                cells[0]["children"][1]["node_type"]["Table"]["row_count"],
                3
            );
        } else {
            assert!(cells[0]["children"][1]["node_type"]
                .get("TextLine")
                .is_some());
        }
        assert_eq!(collect(page, "Rectangle").len(), if i < 2 { 3 } else { 1 });
    }
}

#[test]
fn transparent_reference_is_not_white_and_reopening_keeps_source_style() {
    let mut t = table();
    t.border_fill_id = 1;
    t.cells[0].border_fill_id = 0;
    t.cells[1].border_fill_id = 4;
    t.cells[2].border_fill_id = 5;
    let d = colored_document(t);
    let data = rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap();
    let config = options(&data);
    let mut session = open(&data, &config);
    let mut changed = d.clone();
    changed.doc_info.border_fills[0] = solid(0xFFEEEE);
    let pages = drain(&mut session);
    capture("fill-transparent", &data, &config, &pages);
    assert_eq!(pages, drain(&mut open(&data, &config)));
    assert_eq!(collect(&pages[0], "Rectangle").len(), 1); // Only table fill, neither transparent cell fills.
    assert_eq!(collect(&pages[1], "Rectangle").len(), 2); // White is an explicit opaque fill.
    verify_background(collect(&pages[1], "TableCell")[1], 0xFFFFFF);
    let changed_data = rhwp::serializer::hwpx::serialize_hwpx(&changed).unwrap();
    let changed_pages = drain(&mut open(&changed_data, &options(&changed_data)));
    verify_background(collect(&changed_pages[0], "Table")[0], 0xFFEEEE);
}

#[test]
fn background_retains_remaining_physical_height_after_last_text_line() {
    let mut t = table();
    t.row_count = 1;
    t.repeat_header = false;
    t.cells.truncate(1);
    let c = &mut t.cells[0];
    c.is_header = false;
    c.border_fill_id = 3;
    c.height = 6750; // 90px independent minimum; body fits 36px per page.
    c.paragraphs = vec![paragraph("A")];
    let pages = colored_pages("fill-band", &colored_document(t));
    assert_eq!(pages.len(), 3);
    for (i, page) in pages.iter().enumerate() {
        assert_eq!(labels(page), if i == 0 { vec!["A"] } else { vec![] });
        let cells = collect(page, "TableCell");
        assert_eq!(cells.len(), 1);
        verify_background(cells[0], 0xFFEEEE);
        assert_eq!(cells[0]["bbox"]["height"], if i < 2 { 36.0 } else { 18.0 });
        assert_eq!(collect(page, "Rectangle").len(), 1);
    }
}

#[test]
fn unsupported_or_missing_decoration_is_not_silently_dropped() {
    use rhwp::renderer::table_v2::{
        GeometryError, Rect, TablePreviewError, TablePreviewPages, TablePreviewSession,
        TableSelection,
    };
    let open_document = |d: &Document| {
        TablePreviewSession::from_document(
            d,
            TableSelection {
                section: 0,
                paragraph: 0,
                control: 0,
            },
            96.0,
            TablePreviewPages {
                width: 400.0,
                height: 400.0,
                body: Rect {
                    x: 20.0,
                    y: 30.0,
                    width: 300.0,
                    height: 36.0,
                },
                first_y: 30.0,
            },
            10,
        )
    };
    let mut t = table();
    t.cells[0].border_fill_id = 1;
    let base = colored_document(t);
    // Source checks precede lossy resolved-style conversion, including malformed complex fills.
    for index in 0..8 {
        let mut d = base.clone();
        let b = &mut d.doc_info.border_fills[0];
        match index {
            // Solid CellBreak edges are now supported; dashed edges are not.
            0 => b.borders[0].line_type = BorderLineType::Dash,
            1 => b.fill.alpha = 127,
            2 => b.fill.fill_type = FillType::Gradient,
            3 => b.fill.fill_type = FillType::Image,
            4 => b.fill.solid.as_mut().unwrap().pattern_type = 1,
            5 => b.three_d = true,
            6 => b.attr = 1,
            _ => {
                d.doc_info.border_fills.clear();
            }
        }
        assert!(
            matches!(
                open_document(&d),
                Err(TablePreviewError::Geometry(GeometryError::Unsupported(_)))
            ),
            "case {index}"
        );
        if index == 0 || index == 4 {
            let data = rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap();
            let parsed = rhwp::parse_document(&data).unwrap();
            if index == 0 {
                assert_eq!(
                    parsed.doc_info.border_fills[0].borders[0].line_type,
                    BorderLineType::Dash
                );
            } else {
                assert_eq!(
                    parsed.doc_info.border_fills[0]
                        .fill
                        .solid
                        .unwrap()
                        .pattern_type,
                    1
                );
            }
            let config = options(&data);
            assert!(matches!(
                TablePreviewExportSession::from_bytes(&data, &config.to_string()),
                Err(TablePreviewExportError::Preview(
                    TablePreviewError::Geometry(GeometryError::Unsupported(_))
                ))
            ));
            capture(
                if index == 0 {
                    "fill-border"
                } else {
                    "fill-pattern"
                },
                &data,
                &config,
                &[],
            );
        }
    }
    // Unreferenced styles must not block the selected table.
    let mut d = base;
    let mut unused = solid(0);
    unused.fill.fill_type = FillType::Image;
    d.doc_info.border_fills.push(unused);
    assert!(open_document(&d).is_ok());
    let mut snapshot = open_document(&d).unwrap();
    d.doc_info.border_fills[0] = solid(0xFFEEEE);
    let page = snapshot.next_page().unwrap().unwrap();
    verify_background(
        collect(&json!({"render_tree":page.tree}), "TableCell")[0],
        0xEEEEFF,
    );

    // Unsupported source effects in descendants cannot evade the source guard.
    let mut child = table();
    child.cells[0].border_fill_id = 1;
    let mut parent = table();
    parent.cells[0].paragraphs = vec![host("", child)];
    let mut d = colored_document(parent);
    d.doc_info.border_fills[0].fill.alpha = 127;
    assert!(matches!(
        open_document(&d),
        Err(TablePreviewError::Geometry(GeometryError::Unsupported(
            "V2 source decoration effect"
        )))
    ));
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
