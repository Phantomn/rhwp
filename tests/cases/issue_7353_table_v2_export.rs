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

fn picture_document(alignment: rhwp::model::style::Alignment, separate: bool) -> Document {
    use rhwp::model::{
        bin_data::{BinData, BinDataContent, BinDataType},
        image::Picture,
    };
    let image = image::RgbImage::from_fn(8, 8, |x, y| {
        image::Rgb(if x < 4 {
            if y < 4 {
                [255, 0, 0]
            } else {
                [0, 0, 255]
            }
        } else if y < 4 {
            [0, 255, 0]
        } else {
            [255, 255, 0]
        })
    });
    let mut png = std::io::Cursor::new(Vec::new());
    image.write_to(&mut png, image::ImageFormat::Png).unwrap();
    let mut p = paragraph("");
    p.char_count = 17;
    for _ in 0..2 {
        let mut pic = Picture::default();
        pic.common.treat_as_char = true;
        pic.common.width = 3000;
        pic.common.height = 1800;
        pic.shape_attr.original_width = 600;
        pic.shape_attr.original_height = 600;
        pic.shape_attr.current_width = 3000;
        pic.shape_attr.current_height = 1800;
        pic.image_attr.bin_data_id = 1;
        p.controls.push(Control::Picture(Box::new(pic)));
    }
    p.line_segs = (0..if separate { 2 } else { 1 })
        .map(|i| LineSeg {
            text_start: i * 8,
            vertical_pos: 7500 + i as i32 * 2250,
            column_start: 750,
            segment_width: 12750,
            line_height: 1800,
            text_height: 900,
            baseline_distance: 1800,
            line_spacing: 450,
            tag: LineSeg::TAG_SINGLE_SEGMENT_LINE,
        })
        .collect();
    let mut t = table();
    t.row_count = 1;
    t.cells.truncate(1);
    t.repeat_header = false;
    t.cells[0].is_header = false;
    t.cells[0].paragraphs = vec![paragraph("before"), p, paragraph("after")];
    let mut d = document(t);
    d.doc_info.para_shapes[0].alignment = alignment;
    d.doc_info.bin_data_list.push(BinData {
        storage_id: 1,
        data_type: BinDataType::Embedding,
        extension: Some("png".into()),
        ..Default::default()
    });
    d.bin_data_content.push(BinDataContent {
        id: 1,
        data: png.into_inner().into(),
        extension: "png".into(),
    });
    d
}

#[test]
fn stored_picture_rows_move_atomically_keep_resources_and_following_text() {
    use rhwp::model::style::Alignment;
    for (suffix, alignment, x) in [
        ("left", Alignment::Left, 30.0),
        ("center", Alignment::Center, 75.0),
        ("right", Alignment::Right, 120.0),
    ] {
        let d = picture_document(alignment, false);
        let data = rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap();
        let mut config = options(&data);
        config["pages"]["body"]["height"] = json!(42);
        let mut session = open(&data, &config);
        let pages = drain(&mut session);
        assert_eq!(pages.len(), 2);
        assert_eq!(labels(&pages[0]), ["before"]);
        assert_eq!(labels(&pages[1]), ["after"]);
        let images = collect(&pages[0], "Image");
        assert_eq!(images.len(), 2);
        for (i, n) in images.iter().enumerate() {
            assert_eq!(
                n["bbox"],
                json!({"x":x+i as f64*40.0,"y":48.0,"width":40.0,"height":24.0})
            );
        }
        assert!(collect(&pages[1], "Image").is_empty());
        assert_eq!(collect(&pages[1], "TextLine")[0]["bbox"]["y"], 36.0);
        assert!(pages[0]["svg"]
            .as_str()
            .unwrap()
            .contains("data:image/png;base64,"));
        capture(&format!("picture-{suffix}"), &data, &config, &pages);
        // Only 23px remain after text: the 24px image row must not be clipped.
        config["pages"]["body"]["height"] = json!(41);
        let mut short = open(&data, &config);
        let moved = drain(&mut short);
        assert_eq!(moved.len(), 3);
        assert!(collect(&moved[0], "Image").is_empty());
        assert_eq!(collect(&moved[1], "Image").len(), 2);
        assert_eq!(collect(&moved[1], "Image")[0]["bbox"]["y"], 30.0);
        assert_eq!(labels(&moved[2]), ["after"]);
        capture(&format!("picture-{suffix}-defer"), &data, &config, &moved);
    }
    let data =
        rhwp::serializer::hwpx::serialize_hwpx(&picture_document(Alignment::Left, true)).unwrap();
    let config = options(&data);
    let pages = drain(&mut open(&data, &config));
    assert_eq!(pages.len(), 4);
    assert_eq!(collect(&pages[1], "Image").len(), 1);
    assert_eq!(collect(&pages[2], "Image").len(), 1);
    assert_eq!(labels(&pages[3]), ["after"]);
    capture("picture-rows", &data, &config, &pages);
}

#[test]
fn stored_pictures_in_nested_cell_preserve_ancestor_and_following_flow() {
    let mut d = picture_document(rhwp::model::style::Alignment::Left, false);
    let Control::Table(child) = d.sections[0].paragraphs[0].controls.remove(0) else {
        panic!()
    };
    let mut parent = table();
    parent.row_count = 1;
    parent.cells.truncate(1);
    parent.repeat_header = false;
    parent.cells[0].is_header = false;
    parent.cells[0].paragraphs = vec![host("", *child), paragraph("tail")];
    d.sections[0].paragraphs = vec![host("", parent)];
    let data = rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap();
    let mut config = options(&data);
    config["pages"]["body"]["height"] = json!(42);
    let pages = drain(&mut open(&data, &config));
    assert_eq!(pages.len(), 3);
    assert_eq!(labels(&pages[0]), ["before"]);
    assert_eq!(labels(&pages[1]), ["after", ""]);
    assert_eq!(labels(&pages[2]), ["tail"]);
    for (page, heights) in pages
        .iter()
        .zip([vec![42.0, 42.0], vec![42.0, 24.0], vec![18.0]])
    {
        assert_eq!(
            collect(page, "TableCell")
                .iter()
                .map(|c| c["bbox"]["height"].as_f64().unwrap())
                .collect::<Vec<_>>(),
            heights
        );
    }
    assert_eq!(collect(&pages[0], "Image").len(), 2);
    assert!(pages[1..].iter().all(|p| collect(p, "Image").is_empty()));
    capture("picture-nested", &data, &config, &pages);
}

#[test]
fn stored_picture_margins_crop_and_document_resources_are_preserved() {
    use rhwp::model::{image::CropInfo, style::Alignment};
    use rhwp::renderer::table_v2::DocumentV2Session;
    let mut d = picture_document(Alignment::Center, false);
    d.sections[0].section_def.page_def = rhwp::model::page::PageDef {
        width: 30000,
        height: 15000,
        margin_left: 1500,
        margin_right: 6000,
        margin_top: 1500,
        margin_header: 750,
        margin_bottom: 5100,
        margin_footer: 2250,
        ..Default::default()
    };
    let Control::Table(t) = &mut d.sections[0].paragraphs[0].controls[0] else {
        panic!()
    };
    for ctrl in &mut t.cells[0].paragraphs[1].controls {
        let Control::Picture(pic) = ctrl else {
            panic!()
        };
        pic.common.height = 1200;
        pic.common.margin.top = 300;
        pic.common.margin.bottom = 300;
        pic.common.margin.left = 150;
        pic.common.margin.right = 225;
        pic.img_dim = (600, 600);
        // Top-left quarter of the independent quadrant bitmap is uniformly red.
        pic.crop = CropInfo {
            left: 0,
            top: 0,
            right: 300,
            bottom: 300,
        };
    }
    let mut data = rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap();
    let original = data.clone();
    let mut config = options(&data);
    config["pages"]["body"]["height"] = json!(42);
    let mut snapshot = open(&data, &config);
    let mut document =
        DocumentV2Session::from_bytes(&data, r#"{"dpi":96,"max_pages":10}"#).unwrap();
    data.fill(0);
    d.bin_data_content.clear();
    let pages = drain(&mut snapshot);
    assert_eq!(pages.len(), 2);
    let images = collect(&pages[0], "Image");
    for (i, n) in images.iter().enumerate() {
        // (170 - 2*(40+2+3))/2 = 40px of centering; 4px top/bottom margins.
        assert_eq!(
            n["bbox"],
            json!({"x":72.0+i as f64*45.0,"y":52.0,"width":40.0,"height":16.0})
        );
        assert_eq!(n["node_type"]["Image"]["crop"], json!([0, 0, 300, 300]));
        assert_eq!(
            n["node_type"]["Image"]["original_size_hu"],
            json!([600, 600])
        );
    }
    assert_eq!(images.len(), 2);
    assert_eq!(collect(&pages[0], "TableCell")[0]["bbox"]["height"], 42.0);
    capture("picture-crop-margin", &original, &config, &pages);
    let mut count = 0;
    while let Some(page) = document.next_page_json().unwrap() {
        let v: Value = serde_json::from_str(&page).unwrap();
        count += collect(&v, "Image").len();
    }
    assert_eq!(
        count, 2,
        "whole-document table binding carries the same resources"
    );
}

#[test]
fn stored_picture_rejects_missing_resource_effects_and_ambiguous_streams() {
    use rhwp::renderer::table_v2::{Rect, TablePreviewPages, TablePreviewSession, TableSelection};
    for variant in 0..12 {
        let mut d = picture_document(rhwp::model::style::Alignment::Center, false);
        let Control::Table(t) = &mut d.sections[0].paragraphs[0].controls[0] else {
            panic!()
        };
        let p = &mut t.cells[0].paragraphs[1];
        let Control::Picture(pic) = &mut p.controls[0] else {
            panic!()
        };
        match variant {
            0 => pic.image_attr.bin_data_id = 99,
            1 => pic.common.treat_as_char = false,
            2 => pic.shape_attr.rotation_angle = 90,
            3 => pic.image_attr.brightness = 30,
            4 => pic.common.height -= 1,
            5 => {
                p.text = "X".into();
                p.char_offsets = vec![16];
                p.char_count += 1;
            }
            6 => p.line_segs[0].line_spacing = -100,
            7 => pic.common.horizontal_offset = 1,
            8 => pic.shape_attr.render_tx = 1.0,
            9 => pic.shape_attr.render_sx = f64::NAN,
            10 => pic.shape_attr.render_sy = -1.0,
            _ => pic.crop.right = -1,
        }
        // A serializer may omit a picture with an unresolved resource. Inspect
        // the actual malformed source instead of testing that lossy derivative.
        assert!(
            TablePreviewSession::from_document(
                &d,
                TableSelection {
                    section: 0,
                    paragraph: 0,
                    control: 0
                },
                96.0,
                TablePreviewPages {
                    width: 400.0,
                    height: 400.0,
                    body: Rect {
                        x: 20.0,
                        y: 30.0,
                        width: 300.0,
                        height: 100.0
                    },
                    first_y: 30.0
                },
                10
            )
            .is_err(),
            "variant {variant}"
        );
    }
}

#[test]
fn original_6923_picture_rows_keep_their_declared_frames() {
    use rhwp::renderer::table_v2::{Rect, TablePreviewPages, TablePreviewSession, TableSelection};
    let data = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp"),
    )
    .unwrap();
    let source = rhwp::parse_document(&data).unwrap();
    let Control::Table(original) = &source.sections[0].paragraphs[0].controls[3] else {
        panic!()
    };
    for (ci, width, height) in [(1, 8021.0, 5064.0), (8, 10738.0, 4617.0)] {
        let mut d = source.clone();
        let mut cell = original.cells[ci].clone();
        cell.row = 0;
        cell.col = 0;
        cell.row_span = 1;
        cell.col_span = 1;
        cell.height = 0;
        cell.border_fill_id = 0;
        cell.paragraphs.truncate(1);
        let table = Table {
            row_count: 1,
            col_count: 1,
            cells: vec![cell],
            padding: original.padding,
            page_break: TablePageBreak::RowBreak,
            ..Default::default()
        };
        d.sections[0].paragraphs = vec![host("", table)];
        let mut s = TablePreviewSession::from_document(
            &d,
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
                    height: 150.0,
                },
                first_y: 30.0,
            },
            10,
        )
        .unwrap();
        let page = s.next_page().unwrap().unwrap();
        assert!(s.next_page().unwrap().is_none());
        let value = json!({"render_tree":page.tree});
        let images = collect(&value, "Image");
        assert_eq!(images.len(), 1);
        for (field, hu) in [("width", width), ("height", height)] {
            assert!((images[0]["bbox"][field].as_f64().unwrap() - hu / 75.0).abs() < 1e-9);
        }
        // This HWPX is an explicitly isolated derivative, NOT a replacement
        // Hancom document oracle. Keep its raw JSON to preserve f64 roundtrip.
        let data = rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap();
        let mut config = options(&data);
        config["pages"]["body"]["height"] = json!(150);
        let mut exported = open(&data, &config);
        let raw = exported.next_page_json().unwrap().unwrap();
        assert!(exported.next_page_json().unwrap().is_none());
        let v: Value = serde_json::from_str(&raw).unwrap();
        let image = collect(&v, "Image");
        assert_eq!(image.len(), 1);
        for (field, hu) in [("width", width), ("height", height)] {
            assert!((image[0]["bbox"][field].as_f64().unwrap() - hu / 75.0).abs() < 1e-9);
        }
        let name = format!("picture-source-{ci}");
        capture(&name, &data, &config, &[v]);
        if let Ok(dir) = std::env::var("ISSUE7353_EXPORT_DIR") {
            std::fs::write(format!("{dir}/{name}.native.json"), format!("[{raw}]")).unwrap();
        }
    }
}

#[test]
fn stored_margin_frames_export_with_empty_rows_and_continuation() {
    use rhwp::model::style::Alignment;
    for (suffix, alignment) in [
        ("left", Alignment::Left),
        ("center", Alignment::Center),
        ("right", Alignment::Right),
    ] {
        let mut t = table();
        t.row_count = 1;
        t.repeat_header = false;
        t.cells.truncate(1);
        t.cells[0].is_header = false;
        let row = |text_start, vertical_pos, column_start, segment_width| LineSeg {
            text_start,
            vertical_pos,
            column_start,
            segment_width,
            line_height: 900,
            text_height: 900,
            baseline_distance: 750,
            line_spacing: 450,
            tag: LineSeg::TAG_SINGLE_SEGMENT_LINE,
        };
        let mut p = paragraph("AB");
        p.line_segs = vec![row(0, 7500, 750, 12750), row(1, 8850, 1500, 12000)];
        let mut empty = paragraph("");
        empty.line_segs = vec![row(0, 10200, 750, 12750)];
        t.cells[0].paragraphs = vec![p, empty, paragraph("after")];
        let mut d = document(t);
        d.doc_info.para_shapes[0].alignment = alignment;
        // ParaShape margin units are twice LineSeg HU: 1500/2/75=10px.
        d.doc_info.para_shapes[0].margin_left = 1500;
        d.doc_info.para_shapes[0].margin_right = 3000;
        let data = rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap();
        let mut config = options(&data);
        config["pages"]["body"]["height"] = json!(18);
        let pages = drain(&mut open(&data, &config));
        assert_eq!(pages.len(), 4);
        for (i, page) in pages.iter().enumerate() {
            assert_eq!(labels(page), [["A", "B", "", "after"][i]]);
            let line = collect(page, "TextLine")[0];
            let (x, w) = if i == 1 { (40.0, 160.0) } else { (30.0, 170.0) };
            assert_eq!(
                line["bbox"],
                json!({"x":x,"y":30.0,"width":w,"height":12.0})
            );
            let b = &line["children"][0]["bbox"];
            let rw = b["width"].as_f64().unwrap();
            let rx = match alignment {
                Alignment::Center => x + (w - rw) / 2.0,
                Alignment::Right => x + w - rw,
                _ => x,
            };
            assert!((b["x"].as_f64().unwrap() - rx).abs() < 1e-9);
            assert_eq!(collect(page, "TableCell")[0]["bbox"]["height"], 18.0);
            if i == 2 {
                assert_eq!(rw, 0.0);
            }
        }
        capture(&format!("stored-margin-{suffix}"), &data, &config, &pages);
    }
}

#[test]
fn original_6923_margin_cells_keep_saved_frames() {
    let data = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp"
    ))
    .unwrap();
    let original = rhwp::parse_document(&data).unwrap();
    let Control::Table(t) = &original.sections[0].paragraphs[0].controls[3] else {
        panic!("table")
    };
    let styles = rhwp::renderer::style_resolver::resolve_styles(&original.doc_info, 96.0);
    for ci in [2, 4, 5, 6, 7, 10, 11] {
        let mut cell = t.cells[ci].clone();
        for p in &cell.paragraphs {
            let s = &styles.para_styles[p.para_shape_id as usize];
            assert_eq!(
                (s.margin_left, s.margin_right, s.indent),
                (500.0 / 75.0, 0.0, 0.0),
                "cell{ci} style={s:?}"
            );
            assert_eq!(p.line_segs[0].column_start, 500);
        }
        cell.row = 0;
        cell.col = 0;
        cell.row_span = 1;
        cell.col_span = 1;
        let mut probe = original.clone();
        probe.sections[0].paragraphs = vec![host(
            "",
            Table {
                row_count: 1,
                col_count: 1,
                page_break: t.page_break,
                padding: t.padding,
                cells: vec![cell.clone()],
                ..Default::default()
            },
        )];
        let bytes = rhwp::serializer::hwpx::serialize_hwpx(&probe).unwrap();
        let mut config = options(&bytes);
        config["pages"]["width"] = json!(800);
        config["pages"]["body"]["width"] = json!(700);
        config["pages"]["body"]["height"] = json!(200);
        let mut session = open(&bytes, &config);
        let raw = session.next_page_json().unwrap().unwrap();
        assert!(session.next_page_json().unwrap().is_none());
        let page: Value = serde_json::from_str(&raw).unwrap();
        let lines = collect(&page, "TextLine");
        assert_eq!(lines.len(), cell.paragraphs.len());
        for (line, p) in lines.iter().zip(&cell.paragraphs) {
            assert!((line["bbox"]["x"].as_f64().unwrap() - (20.0 + 641.0 / 75.0)).abs() < 1e-9);
            assert!(
                (line["bbox"]["width"].as_f64().unwrap()
                    - f64::from(p.line_segs[0].segment_width) / 75.0)
                    .abs()
                    < 1e-9
            );
        }
        let name = format!("stored-margin-source-{ci}");
        capture(&name, &bytes, &config, &[page]);
        if let Ok(dir) = std::env::var("ISSUE7353_EXPORT_DIR") {
            std::fs::write(format!("{dir}/{name}.native.json"), format!("[{raw}]")).unwrap();
        }
    }
}

#[test]
fn stored_line_frames_export_without_replacing_their_width_or_origin() {
    use rhwp::model::style::Alignment;
    for (name, alignment) in [
        ("stored-frame-left", Alignment::Left),
        ("stored-frame-center", Alignment::Center),
        ("stored-frame-right", Alignment::Right),
    ] {
        let mut t = table();
        t.row_count = 1;
        t.repeat_header = false;
        t.cells.truncate(1);
        t.cells[0].is_header = false;
        let mut p = paragraph("AB");
        p.line_segs = [(0, 7500, 750, 11250), (1, 8850, 1500, 12750)]
            .into_iter()
            .map(
                |(text_start, vertical_pos, column_start, segment_width)| LineSeg {
                    text_start,
                    vertical_pos,
                    column_start,
                    segment_width,
                    line_height: 900,
                    text_height: 900,
                    baseline_distance: 750,
                    line_spacing: 450,
                    tag: LineSeg::TAG_SINGLE_SEGMENT_LINE,
                },
            )
            .collect();
        t.cells[0].paragraphs = vec![p, paragraph("after")];
        let mut d = document(t);
        d.doc_info.para_shapes[0].alignment = alignment;
        let pages = colored_pages(name, &d);
        assert_eq!(pages.len(), 2);
        assert_eq!(labels(&pages[0]), ["A", "B"]);
        assert_eq!(labels(&pages[1]), ["after"]);
        for (line, x, y, width) in collect(&pages[0], "TextLine")
            .iter()
            .zip([(30.0, 30.0, 150.0), (40.0, 48.0, 170.0)])
            .map(|(l, (x, y, w))| (l, x, y, w))
        {
            assert_eq!(
                line["bbox"],
                json!({"x":x,"y":y,"width":width,"height":12.0})
            );
        }
        assert_eq!(
            collect(&pages[1], "TextLine")[0]["bbox"],
            json!({"x":20.0,"y":30.0,"width":200.0,"height":12.0})
        );
        assert_eq!(collect(&pages[0], "TableCell")[0]["bbox"]["height"], 36.0);
        assert_eq!(collect(&pages[1], "TableCell")[0]["bbox"]["height"], 18.0);
    }
}

#[test]
fn empty_stored_frame_retains_flow_but_caret_has_zero_advance() {
    use rhwp::model::style::Alignment;
    for (name, alignment, x) in [
        ("stored-frame-empty-center", Alignment::Center, 105.0),
        ("stored-frame-empty-right", Alignment::Right, 180.0),
    ] {
        let mut t = table();
        t.row_count = 1;
        t.repeat_header = false;
        t.cells.truncate(1);
        t.cells[0].is_header = false;
        let mut p = paragraph("");
        p.line_segs = vec![LineSeg {
            vertical_pos: 7500,
            column_start: 750,
            segment_width: 11250,
            line_height: 900,
            text_height: 900,
            baseline_distance: 750,
            line_spacing: 450,
            tag: LineSeg::TAG_SINGLE_SEGMENT_LINE,
            ..Default::default()
        }];
        t.cells[0].paragraphs = vec![p, paragraph("after")];
        let mut d = document(t);
        d.doc_info.para_shapes[0].alignment = alignment;
        let pages = colored_pages(name, &d);
        assert_eq!(pages.len(), 1);
        assert_eq!(labels(&pages[0]), ["", "after"]);
        let lines = collect(&pages[0], "TextLine");
        assert_eq!(
            lines[0]["bbox"],
            json!({"x":30.0,"y":30.0,"width":150.0,"height":12.0})
        );
        assert_eq!(
            lines[0]["children"][0]["bbox"],
            json!({"x":x,"y":30.0,"width":0.0,"height":12.0})
        );
        assert_eq!(lines[1]["bbox"]["y"], 48.0);
        assert_eq!(collect(&pages[0], "TableCell")[0]["bbox"]["height"], 36.0);
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
fn linear_cell_background_keeps_intact_row_geometry_and_repeated_header() {
    use rhwp::model::style::GradientFill;
    for (name, angle) in [
        ("gradient-horizontal", 90),
        ("gradient-vertical", 0),
        ("gradient-stepped", 90),
    ] {
        let mut t = table();
        t.page_break = TablePageBreak::RowBreak;
        for c in &mut t.cells {
            c.border_fill_id = 1;
        }
        let mut d = colored_document(t);
        d.doc_info.border_fills[0].fill = Fill {
            fill_type: FillType::Gradient,
            gradient: Some(GradientFill {
                gradient_type: 1,
                angle,
                colors: vec![0, 0xFFFFFF],
                blur: 0,
                step_center: 50,
                ..Default::default()
            }),
            ..Default::default()
        };
        if name == "gradient-stepped" {
            // Copy only the original gradient payload into an explicitly
            // synthetic row fixture; this is not the full #6923 document.
            let source = std::fs::read(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp"),
            )
            .unwrap();
            let original = rhwp::parse_document(&source).unwrap();
            d.doc_info.border_fills[0].fill = original.doc_info.border_fills[34].fill.clone();
            let g = d.doc_info.border_fills[0].fill.gradient.as_ref().unwrap();
            assert_eq!(
                (g.gradient_type, g.angle, g.blur, g.step_center),
                (1, 90, 50, 50)
            );
            assert_eq!(g.colors, [0, 0xEFEFEF]);
        }
        let pages = colored_pages(name, &d);
        assert_eq!(pages.len(), 2);
        for (i, page) in pages.iter().enumerate() {
            assert_eq!(labels(page), ["title", if i == 0 { "A" } else { "B" }]);
            assert_eq!(collect(page, "Rectangle").len(), 2);
            for (j, cell) in collect(page, "TableCell").iter().enumerate() {
                let rect = &cell["children"][0];
                assert_eq!(rect["bbox"], cell["bbox"]);
                assert_eq!(
                    rect["bbox"],
                    json!({"x":20.0,"y":30.0+18.0*j as f64,"width":200.0,"height":18.0})
                );
                let gradient = &rect["node_type"]["Rectangle"]["gradient"];
                assert_eq!(gradient["angle"], angle);
                if name == "gradient-stepped" {
                    let colors = gradient["colors"].as_array().unwrap();
                    let stops = gradient["positions"].as_array().unwrap();
                    assert_eq!(colors.len(), 100); // 50 flat bands, two ends each
                    assert_eq!(colors[0], 0);
                    assert_eq!(colors[99], 0xEFEFEF);
                    assert_eq!(stops[0], 0.0);
                    assert_eq!(stops[99], 1.0);
                    assert!(colors.chunks_exact(2).all(|v| v[0] == v[1]));
                } else {
                    assert_eq!(gradient["colors"], json!([0, 0xFFFFFF]));
                    assert_eq!(gradient["positions"], json!([0.0, 1.0]));
                }
            }
            assert!(page["svg"].as_str().unwrap().contains("linearGradient"));
        }
    }
}

#[test]
fn inactive_diagonal_line_style_is_not_an_enabled_diagonal() {
    let mut t = table();
    t.cells[0].border_fill_id = 1;
    let mut d = colored_document(t);
    d.doc_info.border_fills[0].diagonal.diagonal_type = 1;
    assert_eq!(d.doc_info.border_fills[0].attr, 0);
    let pages = colored_pages("inactive-diagonal", &d);
    assert_eq!(pages.len(), 2);
    assert!(pages.iter().all(|p| collect(p, "Line").is_empty()));
    assert_eq!(collect(&pages[0], "TableCell")[0]["bbox"]["height"], 18.0);
}

#[test]
fn intact_table_gradient_uses_whole_frame_not_each_cell() {
    let mut t = table();
    t.page_break = TablePageBreak::None;
    t.row_count = 2;
    t.cells.truncate(2);
    t.border_fill_id = 1;
    let mut d = colored_document(t);
    d.doc_info.border_fills[0].fill = Fill {
        fill_type: FillType::Gradient,
        gradient: Some(rhwp::model::style::GradientFill {
            gradient_type: 1,
            angle: 90,
            colors: vec![0, 0xFFFFFF],
            step_center: 50,
            ..Default::default()
        }),
        ..Default::default()
    };
    let pages = colored_pages("gradient-whole", &d);
    assert_eq!(pages.len(), 1);
    assert_eq!(labels(&pages[0]), ["title", "A"]);
    let frame = collect(&pages[0], "Table")[0];
    assert_eq!(collect(&pages[0], "Rectangle").len(), 1);
    assert_eq!(frame["children"][0]["bbox"], frame["bbox"]);
    assert_eq!(
        frame["bbox"],
        json!({"x":20.0,"y":30.0,"width":200.0,"height":36.0})
    );
    assert!(collect(&pages[0], "TableCell").iter().all(|cell| collect(
        &json!({"render_tree":{"root":cell}}),
        "Rectangle"
    )
    .is_empty()));
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
    // Intact-cell gradients are supported, not silently restarted across cuts.
    // Use source IR here so a serializer cannot normalize malformed stops.
    for index in 0..11 {
        let mut d = base.clone();
        let Control::Table(t) = &mut d.sections[0].paragraphs[0].controls[0] else {
            unreachable!()
        };
        t.page_break = TablePageBreak::RowBreak;
        let b = &mut d.doc_info.border_fills[0];
        b.fill = Fill {
            fill_type: FillType::Gradient,
            gradient: Some(rhwp::model::style::GradientFill {
                gradient_type: 1,
                angle: 90,
                colors: vec![0, 0xFFFFFF],
                step_center: 50,
                ..Default::default()
            }),
            ..Default::default()
        };
        let g = b.fill.gradient.as_mut().unwrap();
        match index {
            0 => t.page_break = TablePageBreak::CellBreak,
            1 => t.border_fill_id = 1, // row-split table frame is not intact
            2 => g.gradient_type = 2,
            3 => g.colors.clear(),
            4 => g.positions = vec![100, 0],
            5 => g.positions = vec![0],
            6 => g.positions = vec![0, 101],
            7 => g.blur = -1,
            8 => g.step_center = 101,
            9 => b.attr = 8, // real slash declaration, unlike dormant pen style
            _ => b.fill.alpha = 127,
        }
        assert!(
            matches!(
                open_document(&d),
                Err(TablePreviewError::Geometry(GeometryError::Unsupported(_)))
            ),
            "gradient rejection {index}"
        );
    }
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
