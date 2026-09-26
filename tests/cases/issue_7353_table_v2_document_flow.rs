//! Document-body contract, not a fake wrapper table or a Hancom fidelity oracle.
//! Source page:400x200px; body=(20,30,300,72). Every fresh line occupies18px.
//! A90px CellBreak table follows18px prose:54px then36px. Its host/after lines
//! fit on page2. Source body geometry, ownership and actual paint are checked.
use rhwp::{
    model::{
        control::Control,
        document::{Document, Section},
        page::PageDef,
        paragraph::{CharShapeRef, Paragraph},
        shape::{CommonObjAttr, HorzAlign, HorzRelTo, TextWrap, VertRelTo},
        style::{BorderFill, BorderLine, BorderLineType, CharShape, LineSpacingType, ParaShape},
        table::{Cell, Table, TablePageBreak},
    },
    renderer::table_v2::{DocumentV2Error, DocumentV2Session},
};
use serde_json::{json, Value};

fn p(text: &str) -> Paragraph {
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
fn table(texts: &[&str], policy: TablePageBreak) -> Table {
    Table {
        row_count: 1,
        col_count: 1,
        page_break: policy,
        common: CommonObjAttr {
            text_wrap: TextWrap::TopAndBottom,
            vert_rel_to: VertRelTo::Para,
            horz_rel_to: HorzRelTo::Para,
            horz_align: HorzAlign::Center,
            ..Default::default()
        },
        cells: vec![Cell {
            width: 15000,
            row_span: 1,
            col_span: 1,
            border_fill_id: 1,
            paragraphs: texts.iter().map(|v| p(v)).collect(),
            ..Default::default()
        }],
        ..Default::default()
    }
}
fn host(text: &str, t: Table) -> Paragraph {
    let mut para = p(text);
    para.char_count += 8;
    para.char_offsets.iter_mut().for_each(|v| *v += 8);
    para.controls.push(Control::Table(Box::new(t)));
    para
}
fn source(paragraphs: Vec<Paragraph>) -> Document {
    let mut d = Document::default();
    d.doc_info.border_fills.push(BorderFill {
        borders: [BorderLine {
            line_type: BorderLineType::Solid,
            width: 7,
            color: 0x332211,
        }; 4],
        ..Default::default()
    });
    d.doc_info.char_shapes.push(CharShape {
        base_size: 900,
        ..Default::default()
    });
    d.doc_info.para_shapes.push(ParaShape {
        line_spacing_type: LineSpacingType::Fixed,
        line_spacing: 2700,
        ..Default::default()
    });
    let mut section = Section {
        paragraphs,
        ..Default::default()
    };
    section.section_def.page_def = PageDef {
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
    d.sections.push(section);
    d
}
fn bytes(d: &Document) -> Vec<u8> {
    rhwp::serializer::hwpx::serialize_hwpx(d).unwrap()
}

#[test]
fn fresh_indent_uses_the_filled_interval_for_actual_placement() {
    // Independent geometry: body x20 + paragraph margin10 + first inset20.
    // No stored rows: both line breaking and final placement must own this box.
    let mut d = source(vec![p("after")]);
    let s = &mut d.doc_info.para_shapes[0];
    s.alignment = rhwp::model::style::Alignment::Left;
    s.margin_left = 1500;
    s.margin_right = 1500;
    s.indent = 3000;
    let pages = drain(&mut open(&d));
    assert_eq!(labels(&pages[0]), ["after"]);
    let lines = nodes(&pages[0], "TextLine");
    near(&lines[0]["bbox"]["x"], 50.0);
    near(&lines[0]["bbox"]["width"], 260.0);
}

#[test]
fn fresh_indentation_matches_hancom_origins_without_injecting_saved_rows() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/issue7353_fresh_indent_review");
    let input = std::fs::read(root.join("fresh-input.hwpx")).unwrap();
    let source = rhwp::parse_document(&input).unwrap();
    let saved =
        rhwp::parse_document(&std::fs::read(root.join("fresh-saved.hwp")).unwrap()).unwrap();
    let get_table = |d: &Document| {
        d.sections[0].paragraphs[0]
            .controls
            .iter()
            .find_map(|c| {
                if let Control::Table(t) = c {
                    Some(t.as_ref().clone())
                } else {
                    None
                }
            })
            .unwrap()
    };
    let table = get_table(&source);
    let reference = get_table(&saved);
    assert!(source.sections[0]
        .paragraphs
        .iter()
        .all(|p| p.line_segs.is_empty()));
    assert!(table.cells[0]
        .paragraphs
        .iter()
        .all(|p| p.line_segs.is_empty()));
    let pages = drain(&mut DocumentV2Session::from_bytes(&input, TERMINAL_OPTIONS).unwrap());
    assert_eq!(pages.len(), 1);
    let cell = nodes(&pages[0], "TableCell")[0];
    near(&cell["bbox"]["height"], 28000.0 / 75.0);
    let mut lines = Vec::new();
    collect(cell, "TextLine", &mut lines);
    assert_eq!(lines.len(), 13);
    let mut index = 0;
    for (authored, hancom) in table.cells[0]
        .paragraphs
        .iter()
        .zip(&reference.cells[0].paragraphs)
    {
        assert_eq!(authored.text, hancom.text);
        let expected: Vec<_> = authored.text.split('\n').collect();
        assert_eq!(hancom.line_segs.len(), expected.len());
        for (row, text) in hancom.line_segs.iter().zip(expected) {
            let line = lines[index];
            let inset = if row.has_indentation() { 1500.0 } else { 0.0 };
            near(&line["bbox"]["x"], (3969.0 + 283.0 + 500.0 + inset) / 75.0);
            near(
                &line["bbox"]["y"],
                (5669.0 + 283.0 + f64::from(row.vertical_pos)) / 75.0,
            );
            // Declared width32000 - cell pads566 - paragraph margins1000.
            // Hancom's saved sw30432 is two HU narrower than this fresh box;
            // do not silently import or clamp to the saved width.
            assert_eq!(row.segment_width, 30432);
            near(&line["bbox"]["width"], (30434.0 - inset) / 75.0);
            let mut runs = Vec::new();
            collect(line, "TextRun", &mut runs);
            let actual: String = runs
                .iter()
                .map(|r| r["node_type"]["TextRun"]["text"].as_str().unwrap())
                .collect();
            assert_eq!(actual, text);
            index += 1;
        }
    }
    let body_lines = nodes(&pages[0], "TextLine");
    near(&body_lines[13]["bbox"]["y"], (5669.0 + 28000.0) / 75.0);
    near(&body_lines[14]["bbox"]["y"], (5669.0 + 29760.0) / 75.0);
    assert_eq!(*labels(&pages[0]).last().unwrap(), "AFTER REFLOW TABLE");
}

const TERMINAL_OPTIONS: &str =
    r#"{"dpi":96,"max_pages":20,"cell_end_policy":"omit_final_line_gap"}"#;

#[test]
fn terminal_paragraph_after_matches_hancom_table_and_following_body() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/issue7353_indent_review/diagnostic/indent-saved.hwp");
    let input = std::fs::read(path).unwrap();
    let d = rhwp::parse_document(&input).unwrap();
    let t = d.sections[0].paragraphs[0]
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
    let last = t.cells[0].paragraphs.last().unwrap();
    assert_eq!(
        d.doc_info.para_shapes[last.para_shape_id as usize].spacing_after,
        800
    );
    // Independent Hancom normal save/PDF: same table height as the separate
    // zero-after control, despite the authored terminal400HU paragraph gap.
    assert_eq!(t.cells[0].height, 28000); // Declared minimum, not occupied height.
    let options = r#"{"dpi":96,"max_pages":20,"cell_end_policy":"omit_final_paragraph_gap"}"#;
    let pages = drain(&mut DocumentV2Session::from_bytes(&input, options).unwrap());
    assert_eq!(pages.len(), 1);
    let cell = nodes(&pages[0], "TableCell")[0];
    near(&cell["bbox"]["height"], 29266.0 / 75.0);
    let mut lines = Vec::new();
    collect(cell, "TextLine", &mut lines);
    assert_eq!(lines.len(), 16);
    let rows: Vec<_> = t.cells[0]
        .paragraphs
        .iter()
        .flat_map(|p| &p.line_segs)
        .collect();
    for (line, row) in lines.iter().zip(rows) {
        near(
            &line["bbox"]["y"],
            (8787.0 + 283.0 + f64::from(row.vertical_pos)) / 75.0,
        );
    }
    // Body's own paragraph-after / stored advance is not a cell end.
    let body = nodes(&pages[0], "TextLine");
    near(&body.last().unwrap()["bbox"]["y"], 38620.0 / 75.0);
    assert_eq!(*labels(&pages[0]).last().unwrap(), "AFTER INDENTED TABLE");
    // A cell-end option must not trim a body paragraph's following origin.
    let mut body = source(vec![p("before"), p("after")]);
    body.doc_info.para_shapes[0].spacing_after = 600; // 300HU = 4px.
    let pages = drain(&mut DocumentV2Session::from_bytes(&bytes(&body), options).unwrap());
    let lines = nodes(&pages[0], "TextLine");
    assert_eq!(labels(&pages[0]), ["before", "after"]);
    near(&lines[0]["bbox"]["y"], 30.0);
    near(&lines[1]["bbox"]["y"], 52.0); // 18px pitch + authored4px.
}

#[test]
fn document_terminal_policy_keeps_body_paragraph_advance_after_anchored_table() {
    let d = source(vec![
        host("host", table(&["A", "B"], TablePageBreak::CellBreak)),
        p("after"),
    ]);
    let data = bytes(&d);
    let pages = drain(&mut DocumentV2Session::from_bytes(&data, TERMINAL_OPTIONS).unwrap());
    assert_eq!(pages.len(), 1);
    assert_eq!(labels(&pages[0]), ["A", "B", "host", "after"]);
    // Source font12px/pitch18px: cell12+6+12=30; body lines retain18px.
    near(&nodes(&pages[0], "Table")[0]["bbox"]["height"], 30.0);
    for (line, y) in nodes(&pages[0], "TextLine")
        .iter()
        .zip([30.0, 48.0, 60.0, 78.0])
    {
        near(&line["bbox"]["y"], y);
    }
    let default = drain(&mut open(&d));
    near(&nodes(&default[0], "Table")[0]["bbox"]["height"], 36.0);
    near(&nodes(&default[0], "TextLine")[3]["bbox"]["y"], 84.0);
    capture_terminal("document-terminal-anchor", &data, &pages);
}

fn capture_terminal(name: &str, data: &[u8], pages: &[Value]) {
    if let Ok(dir) = std::env::var("ISSUE7353_EXPORT_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/{name}.hwpx"), data).unwrap();
        std::fs::write(format!("{dir}/{name}.options.json"), TERMINAL_OPTIONS).unwrap();
        // Keep renderer JSON verbatim: Value round-tripping can change the
        // last bits of floating-point coordinates before browser comparison.
        let mut session = DocumentV2Session::from_bytes(data, TERMINAL_OPTIONS).unwrap();
        let mut raw = Vec::new();
        while let Some(page) = session.next_page_json().unwrap() {
            raw.push(page);
        }
        assert_eq!(raw.len(), pages.len());
        std::fs::write(
            format!("{dir}/{name}.native.json"),
            format!("[{}]", raw.join(",")),
        )
        .unwrap();
    }
}

#[test]
fn document_terminal_policy_original_carrier_preserves_blank_and_following_origin() {
    let data = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp"
    ))
    .unwrap();
    let mut d = rhwp::parse_document(&data).unwrap();
    let original_carrier = serde_json::to_value(&d.sections[0].paragraphs[0]).unwrap();
    // Scoped derivative: keep original first carrier and all its source table
    // data, append synthetic blank/tail. This is NOT a full-document oracle.
    d.sections[0].paragraphs.truncate(1);
    let char_id = d.doc_info.char_shapes.len() as u32;
    d.doc_info.char_shapes.push(CharShape {
        base_size: 900,
        ..Default::default()
    });
    let para_id = d.doc_info.para_shapes.len() as u16;
    d.doc_info.para_shapes.push(ParaShape {
        line_spacing_type: LineSpacingType::Fixed,
        line_spacing: 2700,
        ..Default::default()
    });
    for text in ["", "after"] {
        let mut para = p(text);
        para.para_shape_id = para_id;
        para.char_shapes[0].char_shape_id = char_id;
        d.sections[0].paragraphs.push(para);
    }
    let data = rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap();
    let reparsed = rhwp::parse_document(&data).unwrap();
    assert_eq!(
        serde_json::to_value(&reparsed.sections[0].paragraphs[0]).unwrap(),
        original_carrier
    );
    let pages = drain(&mut DocumentV2Session::from_bytes(&data, TERMINAL_OPTIONS).unwrap());
    assert_eq!(pages.len(), 1);
    let tables = nodes(&pages[0], "Table");
    assert_eq!(tables.len(), 1);
    // PageDef body=(5669,7087)HU, stored control=(350,283), height11102.
    for (key, hu) in [("x", 6019.0), ("y", 7370.0), ("height", 11102.0)] {
        near(&tables[0]["bbox"][key], hu / 75.0);
    }
    assert_eq!(nodes(&pages[0], "Image").len(), 2);
    let mut lines = Vec::new();
    collect(nodes(&pages[0], "Body")[0], "TextLine", &mut lines);
    let end = &lines[lines.len() - 2..];
    // Saved body row11668HU advances by11668-800, then blank12px/pitch18px.
    for (line, y) in end
        .iter()
        .zip([(7087.0 + 10868.0) / 75.0, (7087.0 + 10868.0) / 75.0 + 18.0])
    {
        near(&line["bbox"]["y"], y);
        near(&line["bbox"]["height"], 12.0);
    }
    assert_eq!(
        labels(&pages[0]).iter().filter(|t| **t == "after").count(),
        1
    );
    capture_terminal("document-terminal-source", &data, &pages);
}

#[test]
fn document_terminal_policy_original_full_admission_advances_without_fallback() {
    let data = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp"
    ))
    .unwrap();
    let result = DocumentV2Session::from_bytes(&data, TERMINAL_OPTIONS);
    let Err(error) = result else {
        panic!("qualify full source output before updating admission")
    };
    eprintln!("original terminal admission: {error}");
    assert!(matches!(
        error,
        DocumentV2Error::Paragraph {
            index: 5,
            reason: rhwp::renderer::table_v2::GeometryError::Unsupported(
                "text preview stored rows or controls"
            )
        }
    ));
    if let Ok(dir) = std::env::var("ISSUE7353_EXPORT_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            format!("{dir}/6923-terminal-admission.txt"),
            error.to_string(),
        )
        .unwrap();
        let original = rhwp::parse_document(&data).unwrap();
        let p = &original.sections[0].paragraphs[5];
        let shape = &original.doc_info.para_shapes[p.para_shape_id as usize];
        std::fs::write(
            format!("{dir}/6923-terminal-next-source.json"),
            serde_json::to_vec_pretty(&json!({"paragraph":p,"para_shape":shape,
                "border_fills":original.doc_info.border_fills}))
            .unwrap(),
        )
        .unwrap();
    }
}
fn open(d: &Document) -> DocumentV2Session {
    DocumentV2Session::from_bytes(&bytes(d), r#"{"dpi":96,"max_pages":20}"#).unwrap()
}

#[test]
fn document_noop_border_reference_preserves_body_and_cell_lines() {
    let mut d = source(vec![
        p(""),
        host("host", table(&["A", ""], TablePageBreak::CellBreak)),
        p("after"),
    ]);
    let reference = drain(&mut open(&d));
    d.doc_info.border_fills.push(BorderFill {
        borders: [BorderLine {
            line_type: BorderLineType::None,
            ..Default::default()
        }; 4],
        ..Default::default()
    });
    // HWPX explicitly encodes no fill. Original HWP CLR_INVALID is exercised
    // separately by the unchanged source paragraph below.
    d.doc_info.para_shapes[0].border_fill_id = 2;
    let actual = drain(&mut open(&d));
    assert_eq!(
        actual, reference,
        "a non-painting reference must not alter any line or advance"
    );
    capture("document-noop-border", &d, &actual);
}

#[test]
fn document_noop_border_original_blank_keeps_saved_height_and_advance() {
    let data = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp"
    ))
    .unwrap();
    let mut d = rhwp::parse_document(&data).unwrap();
    let original = serde_json::to_value(&d.sections[0].paragraphs[..2]).unwrap();
    d.sections[0].paragraphs.truncate(2);
    let char_id = d.doc_info.char_shapes.len() as u32;
    d.doc_info.char_shapes.push(CharShape {
        base_size: 900,
        ..Default::default()
    });
    let para_id = d.doc_info.para_shapes.len() as u16;
    d.doc_info.para_shapes.push(ParaShape {
        line_spacing_type: LineSpacingType::Fixed,
        line_spacing: 2700,
        ..Default::default()
    });
    let mut after = p("after");
    after.para_shape_id = para_id;
    after.char_shapes[0].char_shape_id = char_id;
    d.sections[0].paragraphs.push(after);
    let encoded = rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap();
    let reparsed = rhwp::parse_document(&encoded).unwrap();
    assert_eq!(
        serde_json::to_value(&reparsed.sections[0].paragraphs[..2]).unwrap(),
        original
    );
    let pages = drain(&mut DocumentV2Session::from_bytes(&encoded, TERMINAL_OPTIONS).unwrap());
    assert_eq!(pages.len(), 1);
    let mut lines = Vec::new();
    collect(nodes(&pages[0], "Body")[0], "TextLine", &mut lines);
    let end = &lines[lines.len() - 2..];
    // Original body7087HU + TAC advance10868HU; blank height1000/gap200HU.
    near(&end[0]["bbox"]["y"], 17955.0 / 75.0);
    near(&end[0]["bbox"]["height"], 1000.0 / 75.0);
    near(&end[0]["bbox"]["x"], 5669.0 / 75.0);
    near(&end[0]["bbox"]["width"], 48188.0 / 75.0);
    near(&end[1]["bbox"]["y"], 19155.0 / 75.0);
    assert_eq!(
        labels(&pages[0]).iter().filter(|t| **t == "after").count(),
        1
    );
    capture_terminal("document-noop-source", &encoded, &pages);
}

#[test]
fn document_noop_border_does_not_admit_visible_or_lost_effects() {
    use rhwp::model::style::{CenterLine, FillType, SolidFill};
    for variant in 0..7 {
        let mut d = source(vec![p("text")]);
        d.doc_info.border_fills.push(BorderFill {
            borders: [BorderLine {
                line_type: BorderLineType::None,
                ..Default::default()
            }; 4],
            ..Default::default()
        });
        d.doc_info.para_shapes[0].border_fill_id = 2;
        let b = &mut d.doc_info.border_fills[1];
        match variant {
            0 => b.borders[0].line_type = BorderLineType::Solid,
            1 => {
                b.fill.fill_type = FillType::Solid;
                b.fill.solid = Some(SolidFill {
                    background_color: 0xffffff,
                    ..Default::default()
                });
            }
            2 => {
                b.three_d = true;
                b.attr = 1;
            }
            3 => b.center_line = CenterLine::Cross,
            4 => {
                b.fill.fill_type = FillType::Solid;
                b.fill.solid = Some(SolidFill {
                    background_color: 0xffffffff,
                    pattern_type: 1,
                    ..Default::default()
                });
            }
            _ => d.doc_info.para_shapes[0].border_fill_id = 99,
        }
        assert!(
            DocumentV2Session::from_bytes(&bytes(&d), TERMINAL_OPTIONS).is_err(),
            "variant {variant}"
        );
        let bad_style = d.doc_info.para_shapes[0].clone();
        d.doc_info.para_shapes[0].border_fill_id = 0;
        d.doc_info.para_shapes.push(bad_style);
        let mut nested = table(&["inside"], TablePageBreak::CellBreak);
        nested.cells[0].paragraphs[0].para_shape_id = 1;
        d.sections[0].paragraphs = vec![host("host", nested)];
        assert!(
            DocumentV2Session::from_bytes(&bytes(&d), TERMINAL_OPTIONS).is_err(),
            "nested variant {variant}"
        );
    }
}

#[test]
fn document_original_title_keeps_trailing_space_and_saved_flow() {
    let data = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp"
    ))
    .unwrap();
    let mut d = rhwp::parse_document(&data).unwrap();
    let original = serde_json::to_value(&d.sections[0].paragraphs[..3]).unwrap();
    d.sections[0].paragraphs.truncate(3);
    // A copied blank is a derivative suffix, not part of the original prefix.
    // Its geometry checks title advance without replacing the title's style.
    let blank = d.sections[0].paragraphs[1].clone();
    d.sections[0].paragraphs.push(blank);
    let encoded = rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap();
    let reparsed = rhwp::parse_document(&encoded).unwrap();
    assert_eq!(
        serde_json::to_value(&reparsed.sections[0].paragraphs[..3]).unwrap(),
        original
    );
    let pages = drain(&mut DocumentV2Session::from_bytes(&encoded, TERMINAL_OPTIONS).unwrap());
    assert_eq!(pages.len(), 1);
    let mut lines = Vec::new();
    collect(nodes(&pages[0], "Body")[0], "TextLine", &mut lines);
    let title = lines[lines.len() - 2];
    // Stored body origin + prior TAC/blank advance; 1900HU title + 380HU gap.
    near(&title["bbox"]["y"], 19155.0 / 75.0);
    near(&title["bbox"]["height"], 1900.0 / 75.0);
    near(&title["bbox"]["width"], 48188.0 / 75.0);
    near(&lines.last().unwrap()["bbox"]["y"], 21435.0 / 75.0);
    let mut runs = Vec::new();
    collect(title, "TextRun", &mut runs);
    let text: String = runs
        .iter()
        .map(|r| r["node_type"]["TextRun"]["text"].as_str().unwrap())
        .collect();
    assert_eq!(text, d.sections[0].paragraphs[2].text);
    let right = title["bbox"]["x"].as_f64().unwrap() + 48188.0 / 75.0;
    let tail = runs.last().unwrap();
    assert_eq!(tail["node_type"]["TextRun"]["text"], " ");
    assert!(tail["bbox"]["x"].as_f64().unwrap() + tail["bbox"]["width"].as_f64().unwrap() > right);
    for run in &runs[..runs.len() - 1] {
        assert!(
            run["bbox"]["x"].as_f64().unwrap() + run["bbox"]["width"].as_f64().unwrap()
                <= right + 1e-7
        );
    }
    capture_terminal("document-trailing-source", &encoded, &pages);
}
#[test]
fn document_original_negative_gap_preserves_source_and_following_origin() {
    let data = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp"
    ))
    .unwrap();
    let mut d = rhwp::parse_document(&data).unwrap();
    let original = serde_json::to_value(&d.sections[0].paragraphs[..5]).unwrap();
    d.sections[0].paragraphs.truncate(5);
    let blank = d.sections[0].paragraphs[1].clone();
    d.sections[0].paragraphs.push(blank);
    let encoded = rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap();
    let reparsed = rhwp::parse_document(&encoded).unwrap();
    assert_eq!(
        serde_json::to_value(&reparsed.sections[0].paragraphs[..5]).unwrap(),
        original
    );
    let pages = drain(&mut DocumentV2Session::from_bytes(&encoded, TERMINAL_OPTIONS).unwrap());
    assert_eq!(pages.len(), 1);
    let mut lines = Vec::new();
    collect(nodes(&pages[0], "Body")[0], "TextLine", &mut lines);
    let subtitle = lines[lines.len() - 2];
    // Unmodified saved vpos16028, height1400, gap-140HU; body origin7087HU.
    near(&subtitle["bbox"]["y"], 23115.0 / 75.0);
    near(&subtitle["bbox"]["height"], 1400.0 / 75.0);
    near(&lines.last().unwrap()["bbox"]["y"], 24375.0 / 75.0);
    capture_terminal("document-negative-source", &encoded, &pages);
}

#[test]
fn document_fresh_negative_gap_fits_occupied_boxes_not_only_advance() {
    let mut d = source(vec![p("A"), p(""), p("B")]);
    // 1200HU at75% gives an exact -300HU gap without fractional quantization.
    d.doc_info.char_shapes[0].base_size = 1200;
    d.doc_info.para_shapes[0].line_spacing_type = LineSpacingType::Percent;
    d.doc_info.para_shapes[0].line_spacing = 75;
    // 29px body: 16px rows at12px pitch fit two, not three.
    d.sections[0].section_def.page_def.height = 11775;
    let pages = drain(&mut open(&d));
    assert_eq!(pages.len(), 2);
    for (page, ys) in pages.iter().zip([vec![30.0, 42.0], vec![30.0]]) {
        let lines = nodes(page, "TextLine");
        assert_eq!(lines.len(), ys.len());
        for (line, y) in lines.iter().zip(ys) {
            near(&line["bbox"]["y"], y);
            near(&line["bbox"]["height"], 16.0);
        }
    }
    capture("document-negative-fresh", &d, &pages);
}

fn drain(session: &mut DocumentV2Session) -> Vec<Value> {
    let mut pages = Vec::new();
    while let Some(page) = session.next_page_json().unwrap() {
        pages.push(serde_json::from_str(&page).unwrap());
        assert!(pages.len() < 20);
    }
    assert_eq!(session.emitted_pages() as usize, pages.len());
    assert!(session.next_page_json().unwrap().is_none());
    pages
}

fn stored_anchor(text: &str, policy: TablePageBreak, rows: &[&str]) -> Paragraph {
    use rhwp::model::paragraph::LineSeg;
    let mut t = table(rows, policy);
    t.common.horz_align = HorzAlign::Left;
    t.common.flow_with_text = true;
    t.common.vertical_offset = 1350; //18px from paragraph, not its end
    t.common.horizontal_offset = 375; //5px
    t.common.margin.left = 225; //3px
    t.common.margin.right = 225;
    t.common.margin.top = 450; //6px
    t.common.margin.bottom = 600; //8px
    t.outer_margin_left = 225;
    t.outer_margin_right = 225;
    t.outer_margin_top = 450;
    t.outer_margin_bottom = 600;
    let mut para = host(text, t);
    para.line_segs.push(LineSeg {
        text_start: 0,
        vertical_pos: 1000,
        line_height: 900,
        text_height: 900,
        baseline_distance: 765,
        line_spacing: 450,
        segment_width: 22500,
        tag: LineSeg::TAG_SINGLE_SEGMENT_LINE,
        ..Default::default()
    });
    para
}

#[test]
fn stored_anchor_reserves_offset_once_and_preserves_host_and_following_rows() {
    // Manual boundary contract: stored host12px + gap6px; anchor18+6px.
    // It is not a normal Hancom saved specimen or a PDF fidelity assertion.
    for text in ["", "host"] {
        let d = source(vec![
            stored_anchor(text, TablePageBreak::CellBreak, &["A", "B", "C", "D"]),
            p("after"),
        ]);
        for hwp in [false, true] {
            let encoded = if hwp {
                rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap()
            } else {
                bytes(&d)
            };
            let mut session =
                DocumentV2Session::from_bytes(&encoded, r#"{"dpi":96,"max_pages":20}"#).unwrap();
            let pages = drain(&mut session);
            assert_eq!(pages.len(), 2);
            assert_eq!(labels(&pages[0]), vec![text, "A", "B", "C"]);
            assert_eq!(labels(&pages[1]), ["D", "after"]);
            // Three12px line boxes at18px pitch occupy48px exactly. The last
            //6px paragraph tail is a physical band on the continuation page,
            // not a reason to discard C or repeat its glyphs.
            for (page, (y, height)) in pages.iter().zip([(54.0, 48.0), (36.0, 24.0)]) {
                let t = nodes(page, "Table");
                assert_eq!(t.len(), 1);
                near(&t[0]["bbox"]["x"], 28.0);
                near(&t[0]["bbox"]["y"], y);
                near(&t[0]["bbox"]["height"], height);
                near(&t[0]["bbox"]["width"], 200.0);
                near(&nodes(page, "TableCell")[0]["bbox"]["height"], height);
            }
            let line = nodes(&pages[0], "TextLine")[0];
            near(&line["bbox"]["y"], 30.0);
            near(&line["bbox"]["height"], 12.0);
            for (page, ys) in pages
                .iter()
                .zip([vec![30.0, 54.0, 72.0, 90.0], vec![42.0, 68.0]])
            {
                for (line, y) in nodes(page, "TextLine").iter().zip(ys) {
                    near(&line["bbox"]["y"], y);
                }
            }
            if !text.is_empty() {
                capture(
                    if hwp {
                        "document-anchor-split-hwp"
                    } else {
                        "document-anchor-split"
                    },
                    &d,
                    &pages,
                );
            }
        }
    }
}

#[test]
fn stored_anchor_atomic_defer_does_not_repeat_host_or_initial_band() {
    let d = source(vec![
        p("before"),
        stored_anchor("host", TablePageBreak::None, &["A", "B", "C"]),
        p("after"),
    ]);
    let pages = drain(&mut open(&d));
    // The independent Hancom defer fixture keeps the following story on the
    // source page. A failed floating reservation is not a story page break.
    assert_eq!(pages.len(), 2);
    assert_eq!(labels(&pages[0]), ["before", "host", "after"]);
    assert_eq!(labels(&pages[1]), ["A", "B", "C"]);
    assert!(nodes(&pages[0], "Table").is_empty());
    let t = nodes(&pages[1], "Table")[0];
    near(&t["bbox"]["y"], 36.0);
    near(&t["bbox"]["height"], 54.0);
    near(&nodes(&pages[0], "TextLine")[2]["bbox"]["y"], 66.0);
    capture("document-anchor-defer", &d, &pages);
}

#[test]
fn hancom_deferred_anchor_preserves_prose_on_source_page() {
    let input = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/issue7353_stored_anchor_review/variants/defer-saved.hwp"
    ))
    .unwrap();
    let mut session = DocumentV2Session::from_bytes(&input, TERMINAL_OPTIONS).unwrap();
    let mut clone = session.clone();
    let pages = drain(&mut session);
    assert_eq!(
        drain(&mut clone),
        pages,
        "query snapshots preserve both cursors"
    );
    assert_eq!(pages.len(), 2);
    assert!(nodes(&pages[0], "Table").is_empty());
    let text = |page: &Value| labels(page).join("");
    assert_eq!(text(&pages[0]), "표 시작 위치 확인표 종료 후 본문입니다.");
    assert_eq!(
        text(&pages[1]),
        (1..=3)
            .map(|i| format!("자료 {i:02} : 표 안의 문단과 페이지 연결 확인"))
            .collect::<String>()
    );
    // Untouched saved HWP: 1100HU line + 660HU gap. Independent PDF p1 has
    // this consecutive pair of lines and no table; p2 has just the three rows.
    near(&nodes(&pages[0], "TextLine")[0]["bbox"]["y"], 5669.0 / 75.0);
    near(
        &nodes(&pages[0], "TextLine")[1]["bbox"]["y"],
        (5669.0 + 1760.0) / 75.0,
    );
    near(
        &nodes(&pages[1], "Table")[0]["bbox"]["y"],
        (5669.0 + 283.0) / 75.0,
    );
    near(
        &nodes(&pages[1], "Table")[0]["bbox"]["height"],
        6978.0 / 75.0,
    );
    capture_terminal("document-anchor-defer-hancom", &input, &pages);
}

#[test]
fn deferred_anchor_preserves_blank_lines_and_excludes_remaining_story_on_next_page() {
    let d = source(vec![
        p("before"),
        stored_anchor("host", TablePageBreak::None, &["A", "B", "C"]),
        p(""),
        p("prose1"),
        p("prose2"),
    ]);
    let pages = drain(&mut open(&d));
    assert_eq!(pages.len(), 3);
    assert_eq!(labels(&pages[0]), ["before", "host", "", "prose1"]);
    assert_eq!(labels(&pages[1]), ["A", "B", "C"]);
    assert_eq!(labels(&pages[2]), ["prose2"]);
    near(&nodes(&pages[0], "TextLine")[2]["bbox"]["y"], 66.0);
    near(&nodes(&pages[0], "TextLine")[3]["bbox"]["y"], 84.0);
    near(&nodes(&pages[1], "Table")[0]["bbox"]["y"], 36.0);
    near(&nodes(&pages[2], "TextLine")[0]["bbox"]["y"], 30.0);
}

#[test]
fn deferred_oversize_anchor_does_not_publish_margin_only_pages() {
    let d = source(vec![
        stored_anchor("host", TablePageBreak::None, &["A", "B", "C", "D", "E"]),
        p("after"),
    ]);
    let mut session = open(&d);
    let first: Value = serde_json::from_str(&session.next_page_json().unwrap().unwrap()).unwrap();
    assert_eq!(labels(&first), ["host", "after"]);
    for _ in 0..2 {
        assert!(matches!(
            session.next_page_json(),
            Err(DocumentV2Error::DoesNotFit { page: 1, .. })
        ));
        assert_eq!(session.emitted_pages(), 1);
    }
}

#[test]
fn fitting_anchor_reserves_table_and_bottom_margin_before_following_story() {
    let d = source(vec![
        stored_anchor("host", TablePageBreak::None, &["A"]),
        p("after"),
    ]);
    let pages = drain(&mut open(&d));
    assert_eq!(pages.len(), 1);
    assert_eq!(labels(&pages[0]), ["host", "A", "after"]);
    near(&nodes(&pages[0], "Table")[0]["bbox"]["y"], 54.0);
    near(&nodes(&pages[0], "Table")[0]["bbox"]["height"], 18.0);
    near(&nodes(&pages[0], "TextLine")[2]["bbox"]["y"], 80.0);
}

#[test]
fn deferred_table_survives_story_end_and_offset_outside_page() {
    let mut para = stored_anchor("host", TablePageBreak::None, &["A"]);
    let Control::Table(table) = &mut para.controls[0] else {
        unreachable!()
    };
    // Position is not a multi-page blank paragraph. Defer the object once,
    // preserving its content and outer margin, even with no following prose.
    table.common.vertical_offset = 75000;
    let d = source(vec![para]);
    let pages = drain(&mut open(&d));
    assert_eq!(pages.len(), 2);
    assert_eq!(labels(&pages[0]), ["host"]);
    assert_eq!(labels(&pages[1]), ["A"]);
    near(&nodes(&pages[1], "Table")[0]["bbox"]["y"], 36.0);
}

#[test]
fn multiple_deferred_tables_preserve_owners_without_repeating_story() {
    let mut first = stored_anchor("host1", TablePageBreak::None, &["A"]);
    let mut second = stored_anchor("host2", TablePageBreak::None, &["B"]);
    for para in [&mut first, &mut second] {
        let Control::Table(table) = &mut para.controls[0] else {
            unreachable!()
        };
        table.common.vertical_offset = 75000;
    }
    let d = source(vec![first, p(""), second, p("after")]);
    let pages = drain(&mut open(&d));
    assert_eq!(pages.len(), 2);
    assert_eq!(labels(&pages[0]), ["host1", "", "host2", "after"]);
    assert_eq!(labels(&pages[1]), ["A", "B"]);
    let tables = nodes(&pages[1], "Table");
    assert_eq!(tables.len(), 2);
    near(&tables[0]["bbox"]["y"], 36.0);
    near(&tables[1]["bbox"]["y"], 68.0); //6 +18 +8 +6, not source offsets
    assert_eq!(tables[0]["node_type"]["Table"]["para_index"], 0);
    assert_eq!(tables[1]["node_type"]["Table"]["para_index"], 2);
}

#[test]
fn stored_anchor_rejects_overlap_and_other_placement_modes_without_erasing_rows() {
    for variant in 0..6 {
        let mut para = stored_anchor("", TablePageBreak::CellBreak, &["A"]);
        let Control::Table(t) = &mut para.controls[0] else {
            unreachable!()
        };
        match variant {
            0 => t.common.vertical_offset = 0, //blank line still occupies12px
            1 => t.common.flow_with_text = false,
            2 => t.common.text_wrap = TextWrap::Square,
            3 => t.common.horz_align = HorzAlign::Center,
            4 => t.common.horizontal_offset = 22500,
            5 => t.common.vertical_offset = (-1_i32) as u32,
            _ => {
                //8px advance is NOT the12px occupied end. top10px intersects
                //the host even though it follows the next logical origin.
                para.line_segs[0].line_spacing = -300;
                t.common.vertical_offset = 300;
            }
        }
        let d = source(vec![para]);
        let error = DocumentV2Session::from_bytes(&bytes(&d), r#"{"dpi":96,"max_pages":20}"#)
            .err()
            .unwrap();
        assert!(
            matches!(error, DocumentV2Error::Paragraph { index: 0, .. }),
            "{error}"
        );
    }
}

#[test]
fn stored_anchor_original_host_isolation_preserves_source_offset_and_blank() {
    let input = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp"
    ))
    .unwrap();
    let mut d = rhwp::parse_document(&input).unwrap();
    d.sections[0].paragraphs.truncate(6);
    let original_prefix = serde_json::to_value(&d.sections[0].paragraphs[..5]).unwrap();
    let original_host = d.sections[0].paragraphs[5].clone();
    let Control::Table(original_table) = &original_host.controls[0] else {
        unreachable!()
    };
    // Explicit isolation derivative, NOT original-table admission: its87 cell
    // paragraphs/189665HU cell height are replaced by one fresh18px text row.
    // Keep the actual host, common anchor and both margin representations.
    let mut probe = table(&["anchor-probe"], TablePageBreak::RowBreak);
    probe.common = original_table.common.clone();
    probe.outer_margin_left = original_table.outer_margin_left;
    probe.outer_margin_right = original_table.outer_margin_right;
    probe.outer_margin_top = original_table.outer_margin_top;
    probe.outer_margin_bottom = original_table.outer_margin_bottom;
    probe.cells[0].width = original_table.cells[0].width;
    let char_id = d.doc_info.char_shapes.len() as u32;
    d.doc_info.char_shapes.push(CharShape {
        base_size: 900,
        ..Default::default()
    });
    let para_id = d.doc_info.para_shapes.len() as u16;
    d.doc_info.para_shapes.push(ParaShape {
        line_spacing_type: LineSpacingType::Fixed,
        line_spacing: 2700,
        ..Default::default()
    });
    probe.cells[0].paragraphs[0].char_shapes[0].char_shape_id = char_id;
    probe.cells[0].paragraphs[0].para_shape_id = para_id;
    d.sections[0].paragraphs[5].controls[0] = Control::Table(Box::new(probe));
    let blank = d.sections[0].paragraphs[1].clone();
    d.sections[0].paragraphs.push(blank);
    let data = rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap();
    let parsed = rhwp::parse_document(&data).unwrap();
    assert_eq!(
        serde_json::to_value(&parsed.sections[0].paragraphs[..5]).unwrap(),
        original_prefix
    );
    let mut parsed_host = parsed.sections[0].paragraphs[5].clone();
    let Control::Table(parsed_table) = &parsed_host.controls[0] else {
        unreachable!()
    };
    assert_eq!(
        serde_json::to_value(&parsed_table.common).unwrap(),
        serde_json::to_value(&original_table.common).unwrap()
    );
    parsed_host.controls = original_host.controls.clone();
    assert_eq!(
        serde_json::to_value(&parsed_host).unwrap(),
        serde_json::to_value(&original_host).unwrap()
    );
    let pages = drain(&mut DocumentV2Session::from_bytes(&data, TERMINAL_OPTIONS).unwrap());
    assert_eq!(pages.len(), 1);
    let tables = nodes(&pages[0], "Table");
    assert_eq!(tables.len(), 2);
    let t = tables[1];
    // Page body(5669,7087), host vpos17288, Para/Top offset720 and margin283.
    near(&t["bbox"]["x"], (5669.0 + 283.0) / 75.0);
    near(&t["bbox"]["y"], (7087.0 + 17288.0 + 720.0 + 283.0) / 75.0);
    near(&t["bbox"]["width"], 47901.0 / 75.0);
    near(&t["bbox"]["height"], 12.0); //terminal policy on the synthetic cell
    let body = nodes(&pages[0], "Body")[0];
    let direct: Vec<_> = body["children"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["node_type"].get("TextLine").is_some())
        .collect();
    let host = direct[direct.len() - 2];
    near(&host["bbox"]["y"], 24375.0 / 75.0);
    near(&host["bbox"]["height"], 8.0);
    near(
        &direct.last().unwrap()["bbox"]["y"],
        (25378.0 + 900.0 + 283.0) / 75.0,
    );
    capture_terminal("document-anchor-isolation", &data, &pages);
}
fn collect<'a>(node: &'a Value, kind: &str, out: &mut Vec<&'a Value>) {
    if node["node_type"].get(kind).is_some() {
        out.push(node);
    }
    for c in node["children"].as_array().unwrap() {
        collect(c, kind, out);
    }
}
fn nodes<'a>(page: &'a Value, kind: &str) -> Vec<&'a Value> {
    let mut result = Vec::new();
    collect(&page["render_tree"]["root"], kind, &mut result);
    result
}
fn labels(page: &Value) -> Vec<&str> {
    nodes(page, "TextRun")
        .iter()
        .map(|v| v["node_type"]["TextRun"]["text"].as_str().unwrap())
        .collect()
}
fn near(value: &Value, expected: f64) {
    let value = value.as_f64().unwrap();
    assert!((value - expected).abs() < 1e-6, "{value} != {expected}");
}

#[test]
fn unqualified_cell_internal_diagonal_is_not_silently_redrawn() {
    let input = diagonal_fixture("diagonal-cell-saved.hwp");
    assert!(matches!(
        DocumentV2Session::from_bytes(&input, TERMINAL_OPTIONS),
        Err(DocumentV2Error::Paragraph {
            reason: rhwp::renderer::table_v2::GeometryError::Unsupported(
                "V2 cell-internal diagonal split"
            ),
            ..
        })
    ));
    // Direct resolved entrypoint retains invalid width rather than the HWPX
    // serializer normalizing it to a supported width name.
    let d = source(vec![]);
    let mut styles = rhwp::renderer::style_resolver::resolve_styles(&d.doc_info, 96.0);
    styles.border_styles[0].diagonal_attr = 8;
    styles.border_styles[0].diagonal = rhwp::model::style::DiagonalLine {
        diagonal_type: 1,
        width: 255,
        color: 0,
    };
    assert!(matches!(
        rhwp::renderer::table_v2::PreparedTextTable::prepare(
            &table(&["text"], TablePageBreak::RowBreak),
            &styles,
            96.0
        ),
        Err(rhwp::renderer::table_v2::GeometryError::Unsupported(
            "V2 diagonal pen"
        ))
    ));
}

fn zone_fixture() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/issue7353_zone_review/zone-lines-saved.hwp"
    ))
    .unwrap()
}

fn diagonal_fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/issue7353_diagonal_review")
            .join(name),
    )
    .unwrap()
}

fn slanted_lines(page: &Value) -> Vec<&Value> {
    nodes(page, "Line")
        .into_iter()
        .filter(|n| {
            let l = &n["node_type"]["Line"];
            l["x1"] != l["x2"] && l["y1"] != l["y2"]
        })
        .collect()
}

fn assert_diagonal(n: &Value, x: f64, y: f64, width: f64, height: f64, slash: bool, color: u32) {
    let line = &n["node_type"]["Line"];
    near(&line["x1"], x);
    near(&line["x2"], x + width);
    near(&line["y1"], y + if slash { height } else { 0.0 });
    near(&line["y2"], y + if slash { 0.0 } else { height });
    assert_eq!(line["style"]["color"], color);
    // Width7: source 0.5mm -> 600dpi half-up grid -> 12/600in at96dpi.
    near(&line["style"]["width"], 1.92);
}

#[test]
fn hancom_straight_diagonals_use_cell_and_zone_fragment_corners() {
    let pages = drain(
        &mut DocumentV2Session::from_bytes(
            &diagonal_fixture("diagonal-saved.hwp"),
            TERMINAL_OPTIONS,
        )
        .unwrap(),
    );
    assert_eq!(pages.len(), 2);
    let x = 3969.0 / 75.0;
    let y = 8787.0 / 75.0;
    let row = 2326.0 / 75.0;
    let width = 32000.0 / 75.0;
    let first = slanted_lines(&pages[0]);
    assert_eq!(
        first.len(),
        5,
        "one slash, backslash, cross pair and one zone diagonal"
    );
    assert_diagonal(first[0], x, y, width / 2.0, row, true, 255);
    assert_diagonal(first[1], x, y + row, width / 2.0, row, false, 0xff0000);
    assert_diagonal(first[2], x, y + 2.0 * row, width / 2.0, row, true, 0x8000);
    assert_diagonal(first[3], x, y + 2.0 * row, width / 2.0, row, false, 0x8000);
    assert_diagonal(first[4], x, y + 17.0 * row, width, 2.0 * row, true, 255);
    let second = slanted_lines(&pages[1]);
    assert_eq!(second.len(), 2);
    // Child cell paints before zone paint: final merged row24, then zone rows20..21.
    assert_diagonal(
        second[0],
        x,
        5952.0 / 75.0 + 4.0 * row,
        width,
        row,
        true,
        255,
    );
    assert_diagonal(second[1], x, 5952.0 / 75.0, width, 2.0 * row, true, 255);
    for r in 1..=24 {
        for c in 1..=if r == 24 { 1 } else { 2 } {
            let label = format!("ROW {r} / COL {c}");
            assert_eq!(
                pages
                    .iter()
                    .flat_map(labels)
                    .filter(|s| *s == label)
                    .count(),
                1
            );
        }
    }
    assert_eq!(labels(&pages[1]).last(), Some(&"AFTER DIAGONAL TABLE"));
    near(
        &nodes(&pages[1], "TextLine").last().unwrap()["bbox"]["y"],
        18149.0 / 75.0,
    );
}

#[test]
fn diagonal_paint_does_not_change_stored_geometry_or_line_membership() {
    let input = diagonal_fixture("diagonal-saved.hwp");
    let actual = drain(&mut DocumentV2Session::from_bytes(&input, TERMINAL_OPTIONS).unwrap());
    let mut d = rhwp::parse_document(&input).unwrap();
    for b in &mut d.doc_info.border_fills {
        b.attr = 0;
        b.raw_data = None;
    }
    let control = drain(
        &mut DocumentV2Session::from_bytes(
            &rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap(),
            TERMINAL_OPTIONS,
        )
        .unwrap(),
    );
    assert_eq!(actual.len(), control.len());
    for (a, b) in actual.iter().zip(&control) {
        assert_eq!(labels(a), labels(b));
        for kind in ["Table", "TableCell", "TextLine", "TextRun"] {
            let geometry = |p: &Value| {
                nodes(p, kind)
                    .into_iter()
                    .map(|n| n["bbox"].clone())
                    .collect::<Vec<_>>()
            };
            assert_eq!(geometry(a), geometry(b), "{kind}");
        }
        assert!(slanted_lines(b).is_empty());
    }
}

#[test]
fn nested_diagonal_consumes_final_child_origin_once() {
    let mut child = table(&["child"], TablePageBreak::RowBreak);
    child.cells[0].border_fill_id = 2;
    let mut parent = table(&[], TablePageBreak::CellBreak);
    parent.cells[0].width = 18000;
    parent.cells[0].apply_inner_margin = true;
    parent.cells[0].padding.left = 750;
    parent.cells[0].paragraphs = vec![host("", child)];
    let mut d = source(vec![host("", parent), p("after")]);
    let mut b = d.doc_info.border_fills[0].clone();
    b.attr = 8;
    b.diagonal = rhwp::model::style::DiagonalLine {
        diagonal_type: 1,
        width: 7,
        color: 255,
    };
    d.doc_info.border_fills.push(b);
    let pages = drain(&mut open(&d));
    assert_eq!(pages.len(), 1);
    let lines = slanted_lines(&pages[0]);
    assert_eq!(lines.len(), 1);
    let cells = nodes(&pages[0], "TableCell");
    let child = cells
        .iter()
        .find(|n| n["node_type"]["TableCell"]["border_fill_id"] == 2)
        .unwrap();
    let b = &child["bbox"];
    assert_diagonal(
        lines[0],
        b["x"].as_f64().unwrap(),
        b["y"].as_f64().unwrap(),
        b["width"].as_f64().unwrap(),
        b["height"].as_f64().unwrap(),
        true,
        255,
    );
    assert_eq!(labels(&pages[0]), vec!["child", "", "", "after"]);
}

#[test]
fn unqualified_diagonal_shapes_pens_and_layers_remain_explicit() {
    for mode in 0..8 {
        let mut d = synthetic_zone_source();
        let b = &mut d.doc_info.border_fills[1];
        b.attr = 8;
        b.diagonal = rhwp::model::style::DiagonalLine {
            diagonal_type: 1,
            width: 7,
            color: 255,
        };
        match mode {
            0 => b.attr = 12,
            1 => b.attr = 8 | (1 << 8),
            2 => b.attr = 8 | (1 << 11),
            3 => b.diagonal.diagonal_type = 2,
            4 => b.attr = 1, // binary 3D flag
            5 => b.center_line = rhwp::model::style::CenterLine::Cross,
            6 => {
                if let Control::Table(t) = &mut d.sections[0].paragraphs[0].controls[0] {
                    t.border_fill_id = 2;
                }
            }
            _ => {
                if let Control::Table(t) = &mut d.sections[0].paragraphs[0].controls[0] {
                    t.cells[0].border_fill_id = 2;
                }
            }
        }
        assert!(
            DocumentV2Session::from_bytes(&bytes(&d), TERMINAL_OPTIONS).is_err(),
            "mode {mode}"
        );
    }
}

#[test]
fn hancom_zone_background_and_perimeter_follow_both_fragments() {
    // Untouched Hancom saved HWP and its PDF, not hand-authored LineSegs.
    // PDF: p1 rows1..19, p2 rows20..24; last cell spans both columns.
    let pages =
        drain(&mut DocumentV2Session::from_bytes(&zone_fixture(), TERMINAL_OPTIONS).unwrap());
    assert_eq!(pages.len(), 2);
    let x = 3969.0 / 75.0;
    let width = 32000.0 / 75.0;
    let row = 2326.0 / 75.0;
    for (index, (top, count)) in [(8787.0 / 75.0, 19), (5952.0 / 75.0, 5)]
        .into_iter()
        .enumerate()
    {
        let page = &pages[index];
        let tables = nodes(page, "Table");
        assert_eq!(tables.len(), 1);
        near(&tables[0]["bbox"]["x"], x);
        near(&tables[0]["bbox"]["y"], top);
        near(&tables[0]["bbox"]["height"], row * f64::from(count));
        let background = &tables[0]["children"][0];
        assert_eq!(
            background["node_type"]["Rectangle"]["style"]["fill_color"],
            0xffeeee
        );
        let zone_top = top + if index == 0 { row } else { 0.0 };
        let zone_bottom = top + row * f64::from(count);
        near(&background["bbox"]["x"], x);
        near(&background["bbox"]["width"], width);
        near(&background["bbox"]["y"], zone_top);
        near(&background["bbox"]["height"], zone_bottom - zone_top);
        let red: Vec<_> = nodes(page, "Line")
            .into_iter()
            .filter(|n| n["node_type"]["Line"]["style"]["color"] == 255)
            .collect();
        assert_eq!(red.len(), 4, "one perimeter, no duplicate cell edge paint");
        for (x1, y1, x2, y2) in [
            (x, zone_top, x, zone_bottom),
            (x + width, zone_top, x + width, zone_bottom),
            (x, zone_top, x + width, zone_top),
            (x, zone_bottom, x + width, zone_bottom),
        ] {
            assert!(red.iter().any(|n| {
                let l = &n["node_type"]["Line"];
                [("x1", x1), ("y1", y1), ("x2", x2), ("y2", y2)]
                    .iter()
                    .all(|(k, v)| (l[*k].as_f64().unwrap() - v).abs() < 1e-6)
            }));
        }
        for line in nodes(page, "Line") {
            let l = &line["node_type"]["Line"];
            if l["style"]["color"] != 0 {
                continue;
            }
            let y1 = l["y1"].as_f64().unwrap();
            let y2 = l["y2"].as_f64().unwrap();
            let x1 = l["x1"].as_f64().unwrap();
            let x2 = l["x2"].as_f64().unwrap();
            assert!(
                !(y1 >= zone_top - 1e-6
                    && y2 > zone_top + 1e-6
                    && (x1 - x2).abs() < 1e-6
                    && ((x1 - x).abs() < 1e-6 || (x1 - x - width).abs() < 1e-6)),
                "black perimeter must be replaced"
            );
        }
    }
    let yellow = nodes(&pages[0], "TableCell")
        .into_iter()
        .find(|n| {
            n["node_type"]["TableCell"]["row"] == 2 && n["node_type"]["TableCell"]["col"] == 0
        })
        .unwrap();
    let rect = &yellow["children"][0];
    assert_eq!(
        rect["node_type"]["Rectangle"]["style"]["fill_color"],
        0xffff
    );
    assert_eq!(
        rect["bbox"], yellow["bbox"],
        "cell fill paints over zone, within the same cell"
    );
    for row in 1..=24 {
        for col in 1..=if row == 24 { 1 } else { 2 } {
            let label = format!("ROW {row} / COL {col}");
            assert_eq!(
                pages
                    .iter()
                    .flat_map(labels)
                    .filter(|s| *s == label)
                    .count(),
                1
            );
        }
    }
    assert_eq!(labels(&pages[1]).last(), Some(&"AFTER ZONE TABLE"));
    near(
        &nodes(&pages[1], "TextLine").last().unwrap()["bbox"]["y"],
        18149.0 / 75.0,
    );
}

fn synthetic_zone_source() -> Document {
    use rhwp::model::{
        style::{Fill, FillType, SolidFill},
        table::TableZone,
    };
    let mut t = table(&[], TablePageBreak::RowBreak);
    t.row_count = 2;
    t.col_count = 2;
    t.cells = (0..2)
        .flat_map(|r| {
            (0..2).map(move |c| Cell {
                row: r,
                col: c,
                row_span: 1,
                col_span: 1,
                width: 7500,
                height: 2250,
                border_fill_id: 1,
                paragraphs: vec![p("cell")],
                ..Default::default()
            })
        })
        .collect();
    t.zones = vec![TableZone {
        start_row: 0,
        start_col: 0,
        end_row: 1,
        end_col: 1,
        border_fill_id: 2,
    }];
    let mut d = source(vec![host("", t), p("after")]);
    d.doc_info.border_fills.push(BorderFill {
        borders: [BorderLine {
            line_type: BorderLineType::None,
            ..Default::default()
        }; 4],
        fill: Fill {
            fill_type: FillType::Solid,
            solid: Some(SolidFill {
                background_color: 0xffeeee,
                pattern_type: -1,
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    });
    d
}

#[test]
fn zone_none_edges_preserve_cells_and_do_not_change_layout() {
    let d = synthetic_zone_source();
    let actual = drain(&mut open(&d));
    let mut control = d.clone();
    let Control::Table(t) = &mut control.sections[0].paragraphs[0].controls[0] else {
        panic!()
    };
    t.zones.clear();
    let expected = drain(&mut open(&control));
    assert_eq!(actual.len(), expected.len());
    for (a, b) in actual.iter().zip(&expected) {
        for kind in ["Table", "TableCell", "TextLine", "TextRun", "Line"] {
            let values = |p: &Value| {
                nodes(p, kind)
                    .iter()
                    .map(|n| json!([n["bbox"], n["node_type"]]))
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                values(a),
                values(b),
                "{kind}: decoration must not alter flow or preserved None edges"
            );
        }
    }
}

#[test]
fn nested_zone_uses_child_page_coordinates_once() {
    let mut d = synthetic_zone_source();
    let child = d.sections[0].paragraphs.remove(0);
    let mut parent = table(&[], TablePageBreak::CellBreak);
    parent.cells[0].width = 18000;
    parent.cells[0].apply_inner_margin = true;
    parent.cells[0].padding.left = 750;
    parent.cells[0].paragraphs = vec![child];
    d.sections[0].paragraphs.insert(0, host("", parent));
    let pages = drain(&mut open(&d));
    for page in &pages {
        for t in nodes(page, "Table")
            .into_iter()
            .filter(|n| n["node_type"]["Table"]["col_count"] == 2)
        {
            let rect = &t["children"][0];
            for key in ["x", "y", "width", "height"] {
                near(&rect["bbox"][key], t["bbox"][key].as_f64().unwrap());
            }
            assert_eq!(
                rect["node_type"]["Rectangle"]["style"]["fill_color"],
                0xffeeee
            );
        }
    }
    assert_eq!(
        pages
            .iter()
            .flat_map(labels)
            .filter(|s| *s == "cell")
            .count(),
        4
    );
}

#[test]
fn zone_missing_reference_effects_and_invalid_ranges_are_not_silently_dropped() {
    for mode in 0..5 {
        let mut d = synthetic_zone_source();
        let Control::Table(t) = &mut d.sections[0].paragraphs[0].controls[0] else {
            panic!()
        };
        match mode {
            0 => t.zones[0].border_fill_id = 999,
            1 => t.zones[0].end_row = 9,
            2 => t.zones[0].start_col = 2,
            3 => d.doc_info.border_fills[1].attr = 12, // multi-ray shape, not qualified straight slash
            _ => t.zones.push(t.zones[0].clone()),
        }
        assert!(
            DocumentV2Session::from_bytes(&bytes(&d), TERMINAL_OPTIONS).is_err(),
            "mode {mode}"
        );
    }
}

#[test]
fn zone_background_tracks_cell_internal_cut_without_repeating_lines() {
    let mut d = synthetic_zone_source();
    let Control::Table(t) = &mut d.sections[0].paragraphs[0].controls[0] else {
        panic!()
    };
    t.row_count = 1;
    t.col_count = 1;
    t.page_break = TablePageBreak::CellBreak;
    t.cells.truncate(1);
    t.cells[0].width = 15000;
    t.cells[0].height = 0;
    t.cells[0].paragraphs = (1..=5).map(|i| p(&format!("line-{i}"))).collect();
    t.zones[0].end_row = 0;
    t.zones[0].end_col = 0;
    let pages = drain(&mut open(&d));
    assert_eq!(pages.len(), 2);
    for (page, height) in pages.iter().zip([72.0, 18.0]) {
        let t = nodes(page, "Table")[0];
        let bg = &t["children"][0];
        near(&bg["bbox"]["height"], height);
        for key in ["x", "y", "width", "height"] {
            near(&bg["bbox"][key], t["bbox"][key].as_f64().unwrap());
        }
    }
    for i in 1..=5 {
        assert_eq!(
            pages
                .iter()
                .flat_map(labels)
                .filter(|s| *s == format!("line-{i}"))
                .count(),
            1
        );
    }
    assert_eq!(labels(&pages[1]).last(), Some(&"after"));
}

#[test]
fn zone_range_cannot_silently_crop_a_merged_cell() {
    let mut d = synthetic_zone_source();
    let Control::Table(t) = &mut d.sections[0].paragraphs[0].controls[0] else {
        panic!()
    };
    t.cells[0].col_span = 2;
    t.cells[0].width = 15000;
    t.cells.remove(1);
    t.zones[0].end_col = 0;
    let error = DocumentV2Session::from_bytes(&bytes(&d), TERMINAL_OPTIONS)
        .err()
        .unwrap();
    assert!(
        error.to_string().contains("V2 zone cuts merged cell"),
        "{error}"
    );
}
fn capture(name: &str, d: &Document, pages: &[Value]) {
    if let Ok(dir) = std::env::var("ISSUE7353_EXPORT_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        let (extension, input) = if name.ends_with("-hwp") {
            (
                "hwp",
                rhwp::serializer::cfb_writer::serialize_hwp(d).unwrap(),
            )
        } else {
            ("hwpx", bytes(d))
        };
        std::fs::write(format!("{dir}/{name}.{extension}"), &input).unwrap();
        std::fs::write(
            format!("{dir}/{name}.options.json"),
            r#"{"dpi":96,"max_pages":20}"#,
        )
        .unwrap();
        if name.ends_with("-hwp") {
            // Preserve renderer JSON bytes: parsing through Value without
            // float_roundtrip can turn 120.00000000000001 into 120.0.
            let mut session =
                DocumentV2Session::from_bytes(&input, r#"{"dpi":96,"max_pages":20}"#).unwrap();
            let mut raw = Vec::new();
            while let Some(page) = session.next_page_json().unwrap() {
                raw.push(page);
            }
            assert_eq!(raw.len(), pages.len());
            std::fs::write(
                format!("{dir}/{name}.native.json"),
                format!("[{}]", raw.join(",")),
            )
            .unwrap();
        } else {
            std::fs::write(
                format!("{dir}/{name}.native.json"),
                serde_json::to_vec_pretty(pages).unwrap(),
            )
            .unwrap();
        }
    }
}

#[test]
fn source_body_page_budget_and_table_continuation_share_final_geometry() {
    let d = source(vec![
        p("before"),
        host(
            "host",
            table(&["A", "B", "C", "D", "E"], TablePageBreak::CellBreak),
        ),
        p("after"),
    ]);
    let pages = drain(&mut open(&d));
    assert_eq!(pages.len(), 2);
    assert_eq!(labels(&pages[0]), ["before", "A", "B", "C"]);
    assert_eq!(labels(&pages[1]), ["D", "E", "host", "after"]);
    for (i, page) in pages.iter().enumerate() {
        assert_eq!(page["scope"], "document_body");
        assert_eq!(page["page_index"], i);
        let body = nodes(page, "Body");
        assert_eq!(body.len(), 1);
        for (key, value) in [("x", 20.), ("y", 30.), ("width", 300.), ("height", 72.)] {
            near(&body[0]["bbox"][key], value);
        }
        let tables = nodes(page, "Table");
        assert_eq!(tables.len(), 1, "no invented wrapper table");
        near(&tables[0]["bbox"]["x"], 70.);
        near(&tables[0]["bbox"]["y"], if i == 0 { 48. } else { 30. });
        near(&tables[0]["bbox"]["height"], if i == 0 { 54. } else { 36. });
        for (line, y) in nodes(page, "TextLine").iter().zip([30., 48., 66., 84.]) {
            near(&line["bbox"]["y"], y);
        }
    }
    capture("document-split", &d, &pages);
}

#[test]
fn complete_table_moves_to_fresh_page_without_consuming_host_or_following_text() {
    let d = source(vec![
        p("before1"),
        p("before2"),
        host("host", table(&["A", "B", "C"], TablePageBreak::None)),
        p("after"),
    ]);
    let pages = drain(&mut open(&d));
    assert_eq!(pages.len(), 3);
    assert_eq!(labels(&pages[0]), ["before1", "before2"]);
    assert_eq!(labels(&pages[1]), ["A", "B", "C", "host"]);
    assert_eq!(labels(&pages[2]), ["after"]);
    assert!(nodes(&pages[0], "Table").is_empty());
    near(&nodes(&pages[1], "Table")[0]["bbox"]["height"], 54.);
    capture("document-atomic", &d, &pages);
}

#[test]
fn hwp_container_uses_the_same_document_body_contract() {
    let d = source(vec![
        p("before"),
        host(
            "host",
            table(&["A", "B", "C", "D", "E"], TablePageBreak::CellBreak),
        ),
        p("after"),
    ]);
    let data = rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap();
    let options = r#"{"dpi":96,"max_pages":20}"#;
    let pages = drain(&mut DocumentV2Session::from_bytes(&data, options).unwrap());
    assert_eq!(pages.len(), 2);
    assert_eq!(labels(&pages[0]), ["before", "A", "B", "C"]);
    assert_eq!(labels(&pages[1]), ["D", "E", "host", "after"]);
    for (i, page) in pages.iter().enumerate() {
        let tables = nodes(page, "Table");
        assert_eq!(tables.len(), 1);
        near(&tables[0]["bbox"]["x"], 70.);
        near(&tables[0]["bbox"]["y"], if i == 0 { 48. } else { 30. });
        near(&tables[0]["bbox"]["height"], if i == 0 { 54. } else { 36. });
        for (line, y) in nodes(page, "TextLine").iter().zip([30., 48., 66., 84.]) {
            near(&line["bbox"]["y"], y);
        }
    }
    if let Ok(dir) = std::env::var("ISSUE7353_EXPORT_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/document-hwp.hwp"), data).unwrap();
        std::fs::write(format!("{dir}/document-hwp.options.json"), options).unwrap();
        std::fs::write(
            format!("{dir}/document-hwp.native.json"),
            serde_json::to_vec_pretty(&pages).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn nested_continuation_preserves_body_prose_blank_lines_and_child_host() {
    let mut parent = table(&[], TablePageBreak::CellBreak);
    let mut child = table(&["A", "B", "C", "D"], TablePageBreak::CellBreak);
    child.cells[0].width = 7500;
    parent.cells[0].paragraphs = vec![p(""), host("inner", child), p("tail")];
    let d = source(vec![p("before"), host("host", parent), p("after")]);
    let pages = drain(&mut open(&d));
    assert_eq!(pages.len(), 3);
    assert_eq!(labels(&pages[0]), ["before", "", "A", "B"]);
    assert_eq!(labels(&pages[1]), ["C", "D", "inner", "tail"]);
    assert_eq!(labels(&pages[2]), ["host", "after"]);
    near(&nodes(&pages[0], "Table")[1]["bbox"]["x"], 120.);
    near(&nodes(&pages[1], "Table")[1]["bbox"]["x"], 120.);
    capture("document-nested", &d, &pages);
}

#[test]
fn oversize_and_page_limit_errors_do_not_advance_the_snapshot() {
    let d = source(vec![
        p("before"),
        host(
            "host",
            table(&["A", "B", "C", "D", "E"], TablePageBreak::None),
        ),
    ]);
    let mut session = open(&d);
    assert!(session.next_page_json().unwrap().is_some());
    for _ in 0..2 {
        assert!(matches!(
            session.next_page_json(),
            Err(DocumentV2Error::DoesNotFit { page: 1, .. })
        ));
        assert_eq!(session.emitted_pages(), 1);
    }
    let d = source(vec![p("1"), p("2"), p("3"), p("4"), p("5")]);
    let data = bytes(&d);
    let mut a = DocumentV2Session::from_bytes(&data, r#"{"dpi":96,"max_pages":1}"#).unwrap();
    let mut b = open(&d);
    assert_eq!(a.next_page_json().unwrap(), b.next_page_json().unwrap());
    for _ in 0..2 {
        assert!(matches!(
            a.next_page_json(),
            Err(DocumentV2Error::PageLimit(1))
        ));
        assert_eq!(a.emitted_pages(), 1);
    }
    assert!(b.next_page_json().unwrap().is_some());
}

#[test]
fn no_silent_admission_of_stored_rows_anchors_or_invalid_page_geometry() {
    let mut d = source(vec![p("A")]);
    d.sections[0].paragraphs[0]
        .line_segs
        .push(rhwp::model::paragraph::LineSeg {
            line_height: 900,
            segment_width: 22500,
            ..Default::default()
        });
    assert!(DocumentV2Session::from_bytes(&bytes(&d), r#"{"dpi":96,"max_pages":20}"#).is_err());
    let mut d = source(vec![host("host", table(&["A"], TablePageBreak::None))]);
    if let Control::Table(t) = &mut d.sections[0].paragraphs[0].controls[0] {
        t.common.treat_as_char = true;
    }
    assert!(DocumentV2Session::from_bytes(&bytes(&d), r#"{"dpi":96,"max_pages":20}"#).is_err());
    let mut d = source(vec![p("A")]);
    d.sections[0].section_def.page_def.margin_left = 30000;
    assert!(DocumentV2Session::from_bytes(&bytes(&d), r#"{"dpi":96,"max_pages":20}"#).is_err());
    for options in [
        r#"{"dpi":0,"max_pages":20}"#,
        r#"{"dpi":96,"max_pages":0}"#,
        r#"{"dpi":96,"max_pages":20,"engine":"legacy"}"#,
    ] {
        assert!(DocumentV2Session::from_bytes(&bytes(&source(vec![p("A")])), options).is_err());
    }
}

#[test]
fn empty_page_decoration_records_preserve_body_and_split_table_output() {
    use rhwp::model::page::PageBorderFill;
    let mut d = source(vec![
        p("before"),
        host(
            "host",
            table(&["A", "B", "C", "D", "E"], TablePageBreak::CellBreak),
        ),
        p("after"),
    ]);
    let reference = drain(&mut open(&d));
    // A page decoration's spacing positions its border; it is not a body
    // margin. ID0 names no decoration, even when odd/even records are present.
    // Deliberately larger than the page to catch accidental body reservation.
    let empty = PageBorderFill {
        attr: 1,
        spacing_left: 30000,
        spacing_right: 30000,
        spacing_top: 30000,
        spacing_bottom: 30000,
        ..Default::default()
    };
    d.sections[0].section_def.page_border_fill = empty.clone();
    d.sections[0].section_def.extra_page_border_fills = vec![empty.clone(), empty];
    for (name, encoded) in [
        ("document-empty-page-borders", bytes(&d)),
        (
            "document-empty-page-borders-hwp",
            rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap(),
        ),
    ] {
        let parsed = rhwp::parse_document(&encoded).unwrap();
        let def = &parsed.sections[0].section_def;
        assert_eq!(
            def.extra_page_border_fills.len(),
            2,
            "records must survive parsing"
        );
        for record in std::iter::once(&def.page_border_fill).chain(&def.extra_page_border_fills) {
            assert_eq!(record.border_fill_id, 0);
            assert_eq!(record.spacing_top, 30000);
        }
        let mut session =
            DocumentV2Session::from_bytes(&encoded, r#"{"dpi":96,"max_pages":20}"#).unwrap();
        let mut raw = Vec::new();
        while let Some(page) = session.next_page_json().unwrap() {
            raw.push(page);
        }
        let actual: Vec<Value> = raw
            .iter()
            .map(|p| serde_json::from_str(p).unwrap())
            .collect();
        assert_eq!(
            actual, reference,
            "all nodes, owners, geometry and SVG must be unchanged"
        );
        assert_eq!(actual.len(), 2);
        if let Ok(dir) = std::env::var("ISSUE7353_EXPORT_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            let ext = if name.ends_with("-hwp") {
                "hwp"
            } else {
                "hwpx"
            };
            std::fs::write(format!("{dir}/{name}.{ext}"), encoded).unwrap();
            std::fs::write(
                format!("{dir}/{name}.options.json"),
                r#"{"dpi":96,"max_pages":20}"#,
            )
            .unwrap();
            std::fs::write(
                format!("{dir}/{name}.native.json"),
                format!("[{}]", raw.join(",")),
            )
            .unwrap();
        }
    }
}

#[test]
fn hancom_saved_anchor_review_preserves_rows_and_following_paragraph() {
    let input = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/issue7353_stored_anchor_review/anchor-review-saved.hwp"
    ))
    .unwrap();
    let document = rhwp::parse_document(&input).unwrap();
    assert_ne!(
        document.sections[0]
            .section_def
            .page_border_fill
            .border_fill_id,
        0
    );
    let pages = drain(&mut DocumentV2Session::from_bytes(&input, TERMINAL_OPTIONS).unwrap());
    // Independent matching PDF: p1 has rows01..19, p2 rows20..24 + following
    // paragraph. Independent 0/1/2mm Hancom variants show that continuation
    // reserves the top outer margin, not the original paragraph-relative offset.
    assert_eq!(pages.len(), 2);
    let mut first = vec!["표 시작 위치 확인".to_owned()];
    first.extend((1..=19).map(|i| format!("자료 {i:02} : 표 안의 문단과 페이지 연결 확인")));
    let mut second: Vec<_> = (20..=24)
        .map(|i| format!("자료 {i:02} : 표 안의 문단과 페이지 연결 확인"))
        .collect();
    second.push("표 종료 후 본문입니다.".to_owned());
    let line_texts = |page: &Value| {
        nodes(page, "TextLine")
            .into_iter()
            .map(|line| {
                let mut runs = Vec::new();
                collect(line, "TextRun", &mut runs);
                runs.iter()
                    .map(|run| run["node_type"]["TextRun"]["text"].as_str().unwrap())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(line_texts(&pages[0]), first);
    assert_eq!(line_texts(&pages[1]), second);
    // Source body top5669HU + outer top283HU. PDF top79.317708px differs by
    // <0.05px from this unrounded coordinate; do not copy PDF driver rounding.
    near(&nodes(&pages[1], "Table")[0]["bbox"]["y"], 5952.0 / 75.0);
    for page in &pages {
        let body = nodes(page, "Body")[0];
        let tables = nodes(page, "Table");
        assert_eq!(tables.len(), 1);
        let table = tables[0];
        let bottom = |node: &Value| {
            node["bbox"]["y"].as_f64().unwrap() + node["bbox"]["height"].as_f64().unwrap()
        };
        assert!(bottom(table) <= bottom(body));
    }
}

#[test]
fn hancom_anchor_variants_repeat_margin_not_source_offset() {
    // Unmodified Hancom saves and matching PDFs, not hand-written LineSeg.
    // PDF p2 border tops:75.479167 /82.994792 /79.317708px at96dpi.
    for (name, margin) in [
        ("top0", 0.0),
        ("top2mm", 567.0),
        ("offset20mm", 283.0),
        ("cellbreak", 283.0),
        ("defer", 283.0),
    ] {
        let input = std::fs::read(format!(
            "{}/tests/fixtures/issue7353_stored_anchor_review/variants/{name}-saved.hwp",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let pages = drain(&mut DocumentV2Session::from_bytes(&input, TERMINAL_OPTIONS).unwrap());
        assert_eq!(pages.len(), 2, "{name}");
        let t = nodes(&pages[1], "Table")[0];
        near(&t["bbox"]["y"], (5669.0 + margin) / 75.0);
        let end = t["bbox"]["y"].as_f64().unwrap() + t["bbox"]["height"].as_f64().unwrap();
        let after = nodes(&pages[1], "TextLine").last().copied().unwrap();
        // Source bottom outer margin567HU separates final table and following prose.
        if name != "defer" {
            near(&after["bbox"]["y"], end + 567.0 / 75.0);
        }
        // In the atomic control Hancom leaves following prose on p1. This
        // test establishes table origin only, not that unresolved story order.
        let texts = pages
            .iter()
            .flat_map(|page| nodes(page, "TextRun"))
            .map(|run| run["node_type"]["TextRun"]["text"].as_str().unwrap())
            .collect::<String>();
        let count = if name == "defer" { 3 } else { 24 };
        for row in 1..=count {
            assert_eq!(
                texts.matches(&format!("자료 {row:02}")).count(),
                1,
                "{name} row{row}"
            );
        }
        if name == "defer" {
            assert!(nodes(&pages[0], "Table").is_empty());
        }
    }
}

#[test]
fn unpainted_page_border_references_preserve_full_output() {
    use rhwp::model::page::PageBorderFill;
    use rhwp::model::style::{FillType, SolidFill};
    let mut d = source(vec![
        p("before"),
        host(
            "host",
            table(&["A", "B", "C", "D"], TablePageBreak::CellBreak),
        ),
        p("after"),
    ]);
    let expected = drain(&mut open(&d));
    let mut border = BorderFill::default();
    for pen in &mut border.borders {
        pen.line_type = BorderLineType::None;
    }
    border.fill.fill_type = FillType::Solid;
    border.fill.solid = Some(SolidFill {
        background_color: 0xffffffff,
        pattern_type: -1,
        ..Default::default()
    });
    d.doc_info.border_fills.push(border);
    for slot in 0..3 {
        let def = &mut d.sections[0].section_def;
        def.page_border_fill = PageBorderFill::default();
        def.extra_page_border_fills = vec![PageBorderFill::default(); 2];
        let target = if slot == 0 {
            &mut def.page_border_fill
        } else {
            &mut def.extra_page_border_fills[slot - 1]
        };
        target.border_fill_id = 2;
        target.spacing_top = 30000;
        for encoded in [
            bytes(&d),
            rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap(),
        ] {
            let actual = drain(
                &mut DocumentV2Session::from_bytes(&encoded, r#"{"dpi":96,"max_pages":20}"#)
                    .unwrap(),
            );
            assert_eq!(
                actual, expected,
                "unpainted reference must not change flow or final paint"
            );
        }
    }
}

#[test]
fn page_decoration_on_any_page_variant_is_not_silently_dropped() {
    use rhwp::model::page::PageBorderFill;
    for slot in 0..3 {
        let mut d = source(vec![p("A")]);
        let def = &mut d.sections[0].section_def;
        def.extra_page_border_fills = vec![PageBorderFill::default(); 2];
        let target = if slot == 0 {
            &mut def.page_border_fill
        } else {
            &mut def.extra_page_border_fills[slot - 1]
        };
        target.border_fill_id = 1;
        for encoded in [
            bytes(&d),
            rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap(),
        ] {
            assert!(
                matches!(
                    DocumentV2Session::from_bytes(&encoded, r#"{"dpi":96,"max_pages":20}"#),
                    Err(DocumentV2Error::Unsupported(
                        "section decoration, grid or writing direction"
                    ))
                ),
                "decorated slot {slot}"
            );
        }
    }
}

#[test]
fn stored_body_rows_keep_partition_and_following_paragraph_origin() {
    use rhwp::model::paragraph::LineSeg;
    let mut saved = p("onetwo");
    saved.line_segs = (0..2)
        .map(|i| LineSeg {
            text_start: i * 3,
            vertical_pos: 1000 + i as i32 * 1350,
            line_height: 900,
            text_height: 900,
            baseline_distance: 765,
            line_spacing: 450,
            segment_width: 22500,
            tag: LineSeg::TAG_SINGLE_SEGMENT_LINE,
            ..Default::default()
        })
        .collect();
    let mut d = source(vec![p("before"), saved, p("after"), p("next")]);
    // Manual saved-row boundary fixture: explicit left alignment avoids treating
    // the short first row as a justified full-width line. Not a Hancom specimen.
    d.doc_info.para_shapes[0].alignment = rhwp::model::style::Alignment::Left;
    let pages = drain(&mut open(&d));
    assert_eq!(pages.len(), 2);
    assert_eq!(labels(&pages[0]), ["before", "one", "two", "after"]);
    assert_eq!(labels(&pages[1]), ["next"]);
    for (line, y) in nodes(&pages[0], "TextLine")
        .iter()
        .zip([30.0, 48.0, 66.0, 84.0])
    {
        near(&line["bbox"]["y"], y);
        near(&line["bbox"]["height"], 12.0);
    }
    near(&nodes(&pages[1], "TextLine")[0]["bbox"]["y"], 30.0);
    assert_eq!(d.sections[0].paragraphs[1].line_segs[0].vertical_pos, 1000);
    capture("document-stored", &d, &pages);
}

fn indentation_fixture() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/issue7353_indent_review/indent-saved.hwp"
    ))
    .unwrap()
}

#[test]
fn saved_body_indent_flags_do_not_shift_following_plain_paragraphs() {
    use rhwp::model::{paragraph::LineSeg, style::Alignment};
    let mut saved = p("AB");
    saved.line_segs = (0..2)
        .map(|i| LineSeg {
            text_start: i,
            vertical_pos: 1800 + i as i32 * 1350,
            column_start: 750,
            segment_width: 21000,
            line_height: 900,
            text_height: 900,
            baseline_distance: 765,
            line_spacing: 450,
            tag: LineSeg::TAG_SINGLE_SEGMENT_LINE
                | if i == 1 { LineSeg::TAG_INDENTATION } else { 0 },
        })
        .collect();
    for indent in [3000, -3000] {
        // A fresh first paragraph owns the serialized section controls; saved
        // text offsets here are paragraph-local, not section-control offsets.
        let mut d = source(vec![p("before"), saved.clone(), p(""), p("after")]);
        d.sections[0].section_def.page_def.height += 1350;
        let s = &mut d.doc_info.para_shapes[0];
        s.alignment = Alignment::Left;
        s.margin_left = 1500;
        s.margin_right = 1500;
        let mut indented = s.clone();
        indented.indent = indent;
        d.doc_info.para_shapes.push(indented);
        d.sections[0].paragraphs[1].para_shape_id = 1;
        let pages = drain(&mut open(&d));
        assert_eq!(pages.len(), 1);
        assert_eq!(labels(&pages[0]), ["before", "A", "B", "", "after"]);
        let lines = nodes(&pages[0], "TextLine");
        for (i, line) in lines.iter().skip(1).enumerate() {
            // Following plain paragraphs must not inherit the saved inset.
            let shifted = i == 1;
            near(&line["bbox"]["x"], 30.0 + if shifted { 20.0 } else { 0.0 });
            near(
                &line["bbox"]["width"],
                280.0 - if shifted { 20.0 } else { 0.0 },
            );
            near(&line["bbox"]["y"], 48.0 + i as f64 * 18.0);
        }
        assert_eq!(d.sections[0].paragraphs[1].line_segs, saved.line_segs);
    }
}

#[test]
fn hancom_saved_cell_indentation_keeps_rows_and_final_line_boxes() {
    let data = indentation_fixture();
    let d = rhwp::parse_document(&data).unwrap();
    let pages = drain(&mut DocumentV2Session::from_bytes(&data, TERMINAL_OPTIONS).unwrap());
    assert_eq!(pages.len(), 1);
    let cell = nodes(&pages[0], "TableCell")[0];
    let mut lines = Vec::new();
    collect(cell, "TextLine", &mut lines);
    assert_eq!(lines.len(), 16);
    let t = d.sections[0].paragraphs[0]
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
    let ps = &t.cells[0].paragraphs;
    // Independent saved HWP + PDF: 500HU paragraph margin; 1500HU indentation
    // on first rows of positive paragraphs, later rows of hanging paragraphs.
    // The saved segment does not include that indentation (cs500/sw30432).
    for (pi, para) in ps.iter().enumerate() {
        for (ri, row) in para.line_segs.iter().enumerate() {
            let applies = if pi % 2 == 0 { ri == 0 } else { ri > 0 };
            let line = lines[pi * 4 + ri];
            assert_eq!((row.column_start, row.segment_width), (500, 30432));
            let extra = if applies { 1500.0 } else { 0.0 };
            near(
                &line["bbox"]["x"],
                cell["bbox"]["x"].as_f64().unwrap()
                    + (f64::from(t.cells[0].padding.left) + 500.0 + extra) / 75.0,
            );
            near(&line["bbox"]["width"], (30432.0 - extra) / 75.0);
            let mut runs = Vec::new();
            collect(line, "TextRun", &mut runs);
            let actual: String = runs
                .iter()
                .map(|r| r["node_type"]["TextRun"]["text"].as_str().unwrap())
                .collect();
            let chars: Vec<_> = para.text.chars().collect();
            let start = para.line_seg_text_start(ri) as usize;
            let end = para
                .line_segs
                .get(ri + 1)
                .map(|_| para.line_seg_text_start(ri + 1) as usize)
                .unwrap_or(chars.len());
            assert_eq!(actual, chars[start..end].iter().collect::<String>());
            let left = line["bbox"]["x"].as_f64().unwrap();
            let right = left + line["bbox"]["width"].as_f64().unwrap();
            let first = runs.first().unwrap()["bbox"]["x"].as_f64().unwrap();
            let last = runs.last().unwrap();
            let end = last["bbox"]["x"].as_f64().unwrap() + last["bbox"]["width"].as_f64().unwrap();
            if pi < 2 {
                assert!((first - left).abs() < 1e-7);
            }
            // Left
            else if pi == 2 {
                assert!(((first - left) - (right - end)).abs() < 1e-7);
            }
            // Center
            else {
                // Right alignment excludes plain trailing blanks from painted
                // width but keeps their caret advance and text ownership.
                // The final row has no suffix spaces: its actual run end must
                // equal the indented box's unchanged right edge. Other rows'
                // visible right edges are additionally checked in the PDF sweep.
                if !actual.ends_with(' ') {
                    assert!((right - end).abs() < 1e-7, "{right} != {end}");
                }
            } // Right
        }
    }
    assert_eq!(*labels(&pages[0]).last().unwrap(), "AFTER INDENTED TABLE");
}

#[test]
fn real_6923_remains_unmodified_and_explicitly_unqualified() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp");
    let data = std::fs::read(path).expect("required original fixture");
    let d = rhwp::parse_document(&data).unwrap();
    let def = &d.sections[0].section_def;
    assert_eq!(def.extra_page_border_fills.len(), 2);
    assert!(std::iter::once(&def.page_border_fill)
        .chain(&def.extra_page_border_fills)
        .all(|b| b.border_fill_id == 0));
    assert!(matches!(
        d.sections[0].paragraphs[0].controls[2],
        Control::PageNumberPos(_)
    ));
    // Unmodified first HWP paragraph: secd/cold/page number occupy source
    // slots 0/8/16, but not inline width. Table remains control 3, not 0.
    let first = rhwp::renderer::table_v2::stored_tac_rows(
        &d.sections[0].paragraphs[0],
        48188.0,
        rhwp::model::style::Alignment::Center,
    )
    .unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].height, 11668.0);
    assert_eq!(first[0].spacing, -800.0);
    let frame = rhwp::renderer::table_v2::stored_tac_rows(
        &d.sections[0].paragraphs[0],
        48190.0,
        rhwp::model::style::Alignment::Center,
    )
    .unwrap();
    assert_eq!(frame[0].tables, first[0].tables);
    assert_eq!(
        d.sections[0].paragraphs[1].line_segs[0].vertical_pos,
        11668 - 800
    );
    assert_eq!(
        d.sections[0].paragraphs[2].line_segs[0].vertical_pos,
        10868 + 1000 + 200
    );
    // The source's saved row is 48188HU, while the PageDef content width is
    // 48190HU. The saved segment is an independent, contained alignment frame;
    // neither its width nor the signed spacing is rewritten to admit it.
    assert_eq!(
        def.page_def.width - def.page_def.margin_left - def.page_def.margin_right,
        48190
    );
    assert_eq!(
        d.sections[0].paragraphs[0].line_segs[0].segment_width,
        48188
    );
    assert_eq!(
        first[0].tables,
        vec![(
            3,
            rhwp::renderer::table_v2::Rect {
                x: 350.0,
                y: 283.0,
                width: 47488.0,
                height: 11102.0,
            }
        )]
    );
    // Original HWP control carrier, not a rewritten synthetic table. Query only:
    // children still contain features the experimental engine does not admit.
    let carrier = &d.sections[0].paragraphs[24];
    let rows = rhwp::renderer::table_v2::stored_tac_rows(
        carrier,
        48188.0,
        rhwp::model::style::Alignment::Right,
    )
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].height, 23793.0);
    assert_eq!(rows[0].tables.len(), 1);
    assert_eq!(rows[0].tables[0].0, 0);
    assert_eq!(
        rows[0].tables[0].1,
        rhwp::renderer::table_v2::Rect {
            x: 1898.0,
            y: 141.0,
            width: 46149.0,
            height: 23511.0,
        }
    );
    let mut counts = [0usize; 6]; //paragraphs,stored,table,rowspan,TAC,picture
    fn visit(paragraphs: &[Paragraph], counts: &mut [usize; 6]) {
        for p in paragraphs {
            counts[0] += 1;
            counts[1] += usize::from(!p.line_segs.is_empty());
            for ctrl in &p.controls {
                match ctrl {
                    Control::Table(t) => {
                        counts[2] += 1;
                        counts[4] += usize::from(t.common.treat_as_char);
                        for c in &t.cells {
                            counts[3] += usize::from(c.row_span > 1);
                            visit(&c.paragraphs, counts);
                        }
                    }
                    Control::Picture(_) => counts[5] += 1,
                    _ => {}
                }
            }
        }
    }
    for section in &d.sections {
        visit(&section.paragraphs, &mut counts);
    }
    assert!(
        counts.iter().all(|v| *v > 0),
        "fixture contains all tracked feature classes"
    );
    let reason = match DocumentV2Session::from_bytes(&data, r#"{"dpi":96,"max_pages":30}"#) {
        Err(
            e @ DocumentV2Error::Paragraph {
                index: 0,
                reason:
                    rhwp::renderer::table_v2::GeometryError::Unsupported(
                        "TAC content changed stored occupied box",
                    ),
            },
        ) => e.to_string(),
        Err(e) => panic!("unexpected source admission boundary: {e}"),
        Ok(_) => panic!("update real admission evidence before claiming support"),
    };
    if let Ok(dir) = std::env::var("ISSUE7353_EXPORT_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/6923-admission.json"),serde_json::to_vec_pretty(&json!({
            "paragraphs":counts[0],"stored_paragraphs":counts[1],"tables":counts[2],
            "rowspan_cells":counts[3],"tac_tables":counts[4],"pictures":counts[5],"rejection":reason,
            "result":"UNSUPPORTED - not a fidelity pass",
            "section": d.sections[0].section_def,
            "border_fills": d.doc_info.border_fills})).unwrap()).unwrap();
    }
}

#[test]
fn overlapping_saved_envelopes_keep_distinct_rows_and_physical_ends() {
    let mut carrier = inline_carrier(true);
    carrier.line_segs[1].vertical_pos = 3925; // origin1000 +39px, envelope40px
    carrier.line_segs[0].line_spacing = -75;
    carrier.line_segs[1].line_spacing = -75;
    let mut d = source(vec![carrier, p("after")]);
    d.doc_info.para_shapes[0].alignment = rhwp::model::style::Alignment::Center;
    d.sections[0].section_def.page_def.margin_bottom = 1500; // 200-30-20-30=120px
    add_leading_structure(&mut d);
    let pages = drain(
        &mut DocumentV2Session::from_bytes(
            &rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap(),
            r#"{"dpi":96,"max_pages":20}"#,
        )
        .unwrap(),
    );
    assert_eq!(pages.len(), 1);
    assert_eq!(labels(&pages[0]), ["A", "a", "B", "b", "after"]);
    let ts = nodes(&pages[0], "Table");
    assert_eq!(ts.len(), 2);
    for (t, y) in ts.iter().zip([32.0, 71.0]) {
        near(&t["bbox"]["x"], 130.0);
        near(&t["bbox"]["y"], y);
        near(&t["bbox"]["height"], 36.0);
    }
    near(&nodes(&pages[0], "TextLine")[4]["bbox"]["y"], 108.0);
    capture("document-signed-rows-hwp", &d, &pages);
}

fn inline_carrier(separate: bool) -> Paragraph {
    use rhwp::model::paragraph::LineSeg;
    let mut para = p("");
    para.char_count = 17; // two eight-unit controls and paragraph terminator
    for texts in [["A", "a"], ["B", "b"]] {
        let mut t = table(&texts, TablePageBreak::CellBreak);
        t.common.treat_as_char = true;
        t.common.width = 6000;
        t.common.height = 2700;
        t.cells[0].width = 6000;
        t.outer_margin_left = 150;
        t.outer_margin_right = 150;
        t.outer_margin_top = 150;
        t.outer_margin_bottom = 150;
        // Both IR mirrors carry the same serialized object margin. HWP uses
        // common.margin, HWPX uses the table fields; neither is extra padding.
        t.common.margin.left = 150;
        t.common.margin.right = 150;
        t.common.margin.top = 150;
        t.common.margin.bottom = 150;
        para.controls.push(Control::Table(Box::new(t)));
    }
    para.line_segs = (0..if separate { 2 } else { 1 })
        .map(|i| LineSeg {
            text_start: i * 8,
            vertical_pos: 1000 + i as i32 * 3300,
            line_height: 3000,
            text_height: 3000,
            baseline_distance: 2550,
            line_spacing: 300,
            segment_width: 22500,
            tag: LineSeg::TAG_SINGLE_SEGMENT_LINE,
            ..Default::default()
        })
        .collect();
    para
}

#[test]
fn inline_spaces_and_tables_share_the_row_cut_and_physical_insets() {
    use rhwp::model::{paragraph::LineSeg, style::Alignment};
    // Independent ownership: space at0, table at1, space at9, table at10,
    // terminator18. The second object's start includes the intervening space.
    for nested in [false, true] {
        for active_indent in [false, true] {
            for separate in [false, true] {
                for alignment in [Alignment::Left, Alignment::Center, Alignment::Right] {
                    let mut carrier = inline_carrier(separate);
                    carrier.text = "  ".into();
                    carrier.char_offsets = vec![0, 9];
                    carrier.char_count = 19;
                    carrier.para_shape_id = 1;
                    for (i, row) in carrier.line_segs.iter_mut().enumerate() {
                        row.column_start = 750;
                        row.segment_width = 21000;
                        row.text_start = i as u32 * 9;
                    }
                    if active_indent {
                        carrier.line_segs[0].tag |= LineSeg::TAG_INDENTATION;
                    }
                    let mut d =
                        source(vec![p("before"), p("before2"), carrier.clone(), p("after")]);
                    let mut style = d.doc_info.para_shapes[0].clone();
                    style.alignment = alignment;
                    style.margin_left = 1500; // paragraph margins/indent are doubled HWP units
                    style.margin_right = 1500;
                    style.indent = -3000;
                    d.doc_info.para_shapes.push(style);
                    if nested {
                        let mut parent = table(&[], TablePageBreak::CellBreak);
                        parent.cells[0].width = 22500;
                        parent.cells[0].paragraphs =
                            vec![p("before"), p("before2"), carrier, p("after")];
                        d.sections[0].paragraphs = vec![host("", parent)];
                    }
                    let pages = drain(&mut open(&d));
                    // Before consumes36px of a72px body. The40px occupied inline row
                    // must defer atomically; neither space nor either child stays behind.
                    assert!(nodes(&pages[0], "TextRun")
                        .iter()
                        .all(|n| n["node_type"]["TextRun"]["text"] != " "));
                    let all_tables: Vec<_> = pages.iter().flat_map(|p| nodes(p, "Table")).collect();
                    let children: Vec<_> = all_tables
                        .into_iter()
                        .filter(|t| t["bbox"]["width"].as_f64() == Some(80.0))
                        .collect();
                    assert_eq!(children.len(), 2);
                    let spaces: Vec<_> = pages
                        .iter()
                        .flat_map(|p| nodes(p, "TextRun"))
                        .filter(|n| n["node_type"]["TextRun"]["text"] == " ")
                        .collect();
                    assert_eq!(spaces.len(), 2);
                    // Stored margin10px plus flagged20px. Dormant hanging indent does not shift row0.
                    let x = |n: &Value| n["bbox"]["x"].as_f64().unwrap();
                    let width = |n: &Value| n["bbox"]["width"].as_f64().unwrap();
                    let inset = if active_indent { 20.0 } else { 0.0 };
                    let factor = match alignment {
                        Alignment::Left => 0.0,
                        Alignment::Center => 0.5,
                        _ => 1.0,
                    };
                    let occupied = if separate {
                        84.0 + width(spaces[0])
                    } else {
                        168.0 + width(spaces[0]) + width(spaces[1])
                    };
                    near(
                        &spaces[0]["bbox"]["x"],
                        30.0 + inset + factor * (280.0 - inset - occupied),
                    );
                    near(
                        &children[0]["bbox"]["x"],
                        x(spaces[0]) + width(spaces[0]) + 2.0,
                    );
                    if separate {
                        near(
                            &spaces[1]["bbox"]["x"],
                            30.0 + factor * (280.0 - 84.0 - width(spaces[1])),
                        );
                        assert_eq!(
                            nodes(&pages[1], "TextRun")
                                .iter()
                                .filter(|n| n["node_type"]["TextRun"]["text"] == " ")
                                .count(),
                            1
                        );
                    } else {
                        near(&spaces[1]["bbox"]["x"], x(children[0]) + 80.0 + 2.0);
                    }
                    near(
                        &children[1]["bbox"]["x"],
                        x(spaces[1]) + width(spaces[1]) + 2.0,
                    );
                    near(&children[0]["bbox"]["y"], 32.0);
                    near(&children[1]["bbox"]["y"], 32.0);
                    for text in ["before", "after", "A", "a", "B", "b"] {
                        assert_eq!(
                            pages.iter().flat_map(labels).filter(|s| *s == text).count(),
                            1
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn inline_spaces_reject_stale_axis_painted_text_and_unqualified_rows() {
    for kind in 0..6 {
        let mut carrier = inline_carrier(false);
        carrier.text = " ".into();
        carrier.char_offsets = vec![0];
        carrier.char_count = 18;
        match kind {
            0 => carrier.char_offsets[0] = 2, // incomplete source coverage
            1 => carrier.text = "x".into(),
            2 => carrier.text = "\t".into(),
            3 => carrier.line_segs[0].segment_width = 12600, // tables alone fit, space does not
            4 => {
                carrier.line_segs.clear();
            } // edited/recomposed stream is not a stored row
            _ => {
                carrier = inline_carrier(true);
                carrier.text = "  ".into();
                carrier.char_offsets = vec![0, 9];
                carrier.char_count = 19;
                carrier.line_segs[1].text_start = 9;
                // Justified non-final mixed rows require a distribution result.
            }
        }
        let d = source(vec![carrier]);
        assert!(
            DocumentV2Session::from_bytes(&bytes(&d), TERMINAL_OPTIONS).is_err(),
            "kind={kind}"
        );
    }
}

#[test]
fn hancom_saved_space_before_tac_preserves_child_and_following_origins() {
    let input = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/issue7353_tac_space_review/space-saved.hwp"
    ))
    .unwrap();
    let source = rhwp::parse_document(&input).unwrap();
    let Control::Table(parent) = &source.sections[0].paragraphs[0].controls[2] else {
        panic!()
    };
    let carrier = &parent.cells[0].paragraphs[0];
    assert_eq!(carrier.text, " ");
    assert_eq!(carrier.char_offsets, [0]);
    assert_eq!(carrier.control_text_positions(), [1]);
    assert_eq!(
        source.doc_info.para_shapes[carrier.para_shape_id as usize].indent,
        -4624
    );
    assert!(!carrier.line_segs[0].has_indentation());
    let pages = drain(&mut DocumentV2Session::from_bytes(&input, TERMINAL_OPTIONS).unwrap());
    assert_eq!(pages.len(), 1);
    assert_eq!(
        labels(&pages[0]),
        [
            " ",
            "SPACE BEFORE TABLE",
            "AFTER CELL: no missing or duplicated text"
        ]
    );
    let tables = nodes(&pages[0], "Table");
    assert_eq!(tables.len(), 2);
    near(&tables[0]["bbox"]["x"], 5669.0 / 75.0);
    near(&tables[0]["bbox"]["y"], 7087.0 / 75.0);
    near(&tables[0]["bbox"]["height"], 15000.0 / 75.0);
    let runs = nodes(&pages[0], "TextRun");
    let space = runs[0];
    near(&space["bbox"]["x"], (5669.0 + 283.0) / 75.0);
    near(&space["bbox"]["height"], 5000.0 / 75.0);
    near(&space["node_type"]["TextRun"]["baseline"], 4250.0 / 75.0);
    near(
        &tables[1]["bbox"]["x"],
        space["bbox"]["x"].as_f64().unwrap() + space["bbox"]["width"].as_f64().unwrap(),
    );
    near(&tables[1]["bbox"]["y"], (7087.0 + 283.0) / 75.0);
    near(&tables[1]["bbox"]["height"], 5000.0 / 75.0);
    // Independent Hancom PDF vertical strokes: parent x56.609375pt,
    // child x66.804688pt. Allow one300dpi printer dot, not a page-count baseline.
    let delta = tables[1]["bbox"]["x"].as_f64().unwrap() - tables[0]["bbox"]["x"].as_f64().unwrap();
    assert!((delta - (66.804688 - 56.609375) * 96.0 / 72.0).abs() < 96.0 / 300.0);
    let after = &source.sections[0].paragraphs[1].line_segs[0];
    near(
        &runs[2]["bbox"]["y"],
        (7087.0 + f64::from(after.vertical_pos)) / 75.0,
    );
}

#[test]
fn hancom_narrow_nested_cell_keeps_minimum_text_lane_without_widening_table() {
    let input = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/issue7353_narrow_cell_review/narrow-saved.hwp"
    ))
    .unwrap();
    let source = rhwp::parse_document(&input).unwrap();
    let Control::Table(parent) = &source.sections[0].paragraphs[0].controls[2] else {
        panic!()
    };
    let Control::Table(child) = &parent.cells[0].paragraphs[0].controls[0] else {
        panic!()
    };
    let narrow = &child.cells[1];
    assert_eq!(narrow.width, 1303);
    assert!(!narrow.apply_inner_margin);
    assert_eq!((child.padding.left, child.padding.right), (510, 510));
    assert_eq!(narrow.paragraphs[0].text, "A");
    assert_eq!(narrow.paragraphs[0].line_segs[0].segment_width, 1440);
    let pages = drain(&mut DocumentV2Session::from_bytes(&input, TERMINAL_OPTIONS).unwrap());
    assert_eq!(pages.len(), 1);
    assert_eq!(
        labels(&pages[0]),
        [
            " ",
            "LEFT",
            "A",
            "RIGHT - narrow middle cell",
            "AFTER CELL: no missing or duplicated text"
        ]
    );
    let tables = nodes(&pages[0], "Table");
    assert_eq!(tables.len(), 2);
    near(&tables[0]["bbox"]["width"], 48182.0 / 75.0);
    near(&tables[0]["bbox"]["height"], 15000.0 / 75.0);
    near(&tables[1]["bbox"]["width"], 27303.0 / 75.0);
    near(&tables[1]["bbox"]["height"], 5000.0 / 75.0);
    let cells = nodes(&pages[0], "TableCell");
    let middle = cells
        .iter()
        .find(|c| (c["bbox"]["width"].as_f64().unwrap() - 1303.0 / 75.0).abs() < 1e-9)
        .unwrap();
    let line = nodes(&pages[0], "TextLine")
        .into_iter()
        .find(|l| {
            l["children"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["node_type"]["TextRun"]["text"] == "A")
        })
        .unwrap();
    near(
        &line["bbox"]["x"],
        middle["bbox"]["x"].as_f64().unwrap() + 510.0 / 75.0,
    );
    near(&line["bbox"]["width"], 1440.0 / 75.0);
    near(&line["bbox"]["height"], 1000.0 / 75.0);
    // Independent source widths keep the right neighbour at the cell edge,
    // not at the wider text lane edge. No clipping hides the extending lane.
    let right = cells.last().unwrap();
    near(
        &right["bbox"]["x"],
        tables[1]["bbox"]["x"].as_f64().unwrap() + (6000.0 + 1303.0) / 75.0,
    );
    let after = nodes(&pages[0], "TextLine").last().copied().unwrap();
    near(
        &after["bbox"]["y"],
        (7087.0 + f64::from(source.sections[0].paragraphs[1].line_segs[0].vertical_pos)) / 75.0,
    );
}

#[test]
fn hancom_excluded_cell_anchor_preserves_child_and_authored_blank_line() {
    let input = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/issue7353_cell_anchor_review/anchor-saved.hwp"
    ))
    .unwrap();
    let pages = drain(&mut DocumentV2Session::from_bytes(&input, TERMINAL_OPTIONS).unwrap());
    assert_eq!(pages.len(), 1);
    assert_eq!(
        labels(&pages[0]),
        [
            "POSITIONED CHILD",
            "",
            "AFTER ANCHOR",
            "AFTER CELL: no missing or duplicated text"
        ]
    );
    let tables = nodes(&pages[0], "Table");
    assert_eq!(tables.len(), 2);
    let x = tables[0]["bbox"]["x"].as_f64().unwrap() + 283.0 / 75.0;
    let y = tables[0]["bbox"]["y"].as_f64().unwrap() + 283.0 / 75.0;
    // Hancom normal-save: owner vpos0/sw0, child5000HU with141HU margins,
    // following authored blank at5282, following ink at7426 (1500+644 later).
    near(&tables[1]["bbox"]["x"], x + (1980.0 + 141.0) / 75.0);
    near(&tables[1]["bbox"]["y"], y + 141.0 / 75.0);
    near(&tables[1]["bbox"]["width"], 44957.0 / 75.0);
    near(&tables[1]["bbox"]["height"], 5000.0 / 75.0);
    let lines = nodes(&pages[0], "TextLine");
    let owner = lines
        .iter()
        .find(|l| l["bbox"]["width"].as_f64() == Some(0.0))
        .unwrap();
    near(&owner["bbox"]["y"], y);
    near(&owner["bbox"]["height"], 1500.0 / 75.0);
    assert!(lines
        .iter()
        .any(|l| (l["bbox"]["y"].as_f64().unwrap() - (y + 5282.0 / 75.0)).abs() < 1e-9));
    let after = lines
        .iter()
        .find(|l| {
            l["children"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["node_type"]["TextRun"]["text"] == "AFTER ANCHOR")
        })
        .unwrap();
    near(&after["bbox"]["y"], y + 7426.0 / 75.0);
}

#[test]
fn following_cell_anchor_uses_saved_lines_after_exclusion_not_visibility() {
    for name in ["anchor", "visible"] {
        let input = std::fs::read(format!(
            "{}/tests/fixtures/issue7353_following_anchor_review/{name}-saved.hwp",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let pages = drain(&mut DocumentV2Session::from_bytes(&input, TERMINAL_OPTIONS).unwrap());
        assert_eq!(pages.len(), 1);
        let tables = nodes(&pages[0], "Table");
        assert_eq!(tables.len(), 2);
        // Independently saved Hancom line origins:0,6766,7886,9086HU.
        // 1200 + top283 + child5000 + bottom283 =6766, then1400-280.
        let y = tables[0]["bbox"]["y"].as_f64().unwrap() + 141.0 / 75.0;
        let x = tables[0]["bbox"]["x"].as_f64().unwrap() + 141.0 / 75.0;
        near(&tables[1]["bbox"]["x"], x + 1903.0 / 75.0);
        near(&tables[1]["bbox"]["y"], y + 1483.0 / 75.0);
        near(&tables[1]["bbox"]["height"], 5000.0 / 75.0);
        near(&tables[0]["bbox"]["height"], 10568.0 / 75.0);
        let lines = nodes(&pages[0], "TextLine");
        for (origin, height) in [(6766.0, 1400.0), (7886.0, 1200.0), (9086.0, 1200.0)] {
            assert!(lines
                .iter()
                .any(
                    |n| (n["bbox"]["y"].as_f64().unwrap() - y - origin / 75.0).abs() < 1e-9
                        && (n["bbox"]["height"].as_f64().unwrap() - height / 75.0).abs() < 1e-9
                ));
        }
        let text = labels(&pages[0]);
        assert_eq!(text[0], "BEFORE ANCHOR");
        assert_eq!(text[1], "POSITIONED CHILD");
        let after_index = text.iter().position(|s| *s == "AFTER HOST").unwrap();
        assert_eq!(
            text[after_index..after_index + 2],
            ["AFTER HOST", "AFTER CELL"]
        );
        if name == "visible" {
            assert!(text.contains(&"HOST TEXT"));
        } else {
            assert!(text
                .iter()
                .any(|s| !s.is_empty() && s.chars().all(|c| c == ' ')));
        }
        near(
            &lines
                .iter()
                .find(|n| n.to_string().contains("AFTER CELL"))
                .unwrap()["bbox"]["y"],
            tables[0]["bbox"]["y"].as_f64().unwrap() + 10568.0 / 75.0,
        );
    }
}

#[test]
fn following_anchor_cuts_preserve_negative_gap_host_and_child_tail() {
    use rhwp::renderer::{style_resolver::resolve_styles, table_v2::*};
    let d = rhwp::parse_document(
        &std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/issue7353_following_anchor_review/visible-saved.hwp"
        ))
        .unwrap(),
    )
    .unwrap();
    let Control::Table(t) = d.sections[0].paragraphs[0]
        .controls
        .iter()
        .find(|c| matches!(c, Control::Table(_)))
        .unwrap()
    else {
        panic!()
    };
    let styles = resolve_styles(&d.doc_info, 96.0);
    let mut t = t.clone();
    t.page_break = TablePageBreak::CellBreak;
    t.cells[0].height = 0;
    t.padding = Default::default();
    t.cells[0].padding = Default::default();
    t.cells[0].vertical_align = rhwp::model::table::VerticalAlign::Top;
    let Control::Table(child) = &mut t.cells[0].paragraphs[1].controls[0] else {
        panic!()
    };
    child.page_break = TablePageBreak::CellBreak;
    child.padding = Default::default();
    child.cells[0].padding = Default::default();
    child.cells[0].vertical_align = rhwp::model::table::VerticalAlign::Top;
    let prepared = PreparedTextTable::prepare_with_end_policy(
        &t,
        &styles,
        96.0,
        &[],
        CellEndPolicy::OmitFinalParagraphGap,
    )
    .unwrap();
    let area = |height| PageArea {
        bounds: Rect {
            x: 20.0,
            y: 30.0,
            width: 800.0,
            height,
        },
    };
    for budget in [20.0, 30.0, 40.0, 1000.0] {
        let mut cursor = prepared.start();
        let mut text = Vec::new();
        let mut done = false;
        let mut child_height = 0.0;
        let mut parts = 0;
        for _ in 0..30 {
            match cursor.fit(area(budget)).unwrap() {
                TextFragmentFit::Complete => {
                    done = true;
                    break;
                }
                TextFragmentFit::Placed(part) => {
                    assert!(part.geometry().reserved_height() <= budget + 1e-9);
                    if parts == 0 && budget <= 30.0 {
                        // Only BEFORE fits. Do not consume a top-margin-only
                        // fragment when the first child line cannot fit with it.
                        assert!((part.geometry().reserved_height() - 1200.0 / 75.0).abs() < 1e-9);
                    }
                    parts += 1;
                    let mut page =
                        rhwp::renderer::render_tree::PageRenderTree::new(0, 850.0, 1100.0);
                    part.append_to(&mut page).unwrap();
                    let raw = json!({"render_tree":page});
                    child_height += nodes(&raw, "Table")
                        .iter()
                        .skip(1)
                        .map(|n| n["bbox"]["height"].as_f64().unwrap())
                        .sum::<f64>();
                    text.extend(labels(&raw).into_iter().map(str::to_owned));
                    for n in nodes(&raw, "TextLine")
                        .into_iter()
                        .chain(nodes(&raw, "Table"))
                    {
                        let b = &n["bbox"];
                        assert!(b["y"].as_f64().unwrap() >= 30.0 - 1e-9);
                        assert!(
                            b["y"].as_f64().unwrap() + b["height"].as_f64().unwrap()
                                <= 30.0 + budget + 1e-9
                        );
                    }
                    cursor = part.continuation();
                }
                _ => panic!("unexpected blocked cut at {budget}"),
            }
        }
        assert!(done);
        assert!((child_height - 5000.0 / 75.0).abs() < 1e-9);
        assert_eq!(
            text,
            [
                "BEFORE ANCHOR",
                "POSITIONED CHILD",
                "HOST TEXT",
                "",
                "AFTER HOST"
            ]
        );
    }
    // Other anchor frames/wrapping and inconsistent source margins stay closed.
    for kind in 0..3 {
        let mut bad = t.clone();
        let Control::Table(child) = &mut bad.cells[0].paragraphs[1].controls[0] else {
            panic!()
        };
        match kind {
            0 => child.common.text_wrap = TextWrap::Square,
            1 => child.common.vertical_offset = 100,
            _ => child.outer_margin_left += 1,
        }
        assert!(PreparedTextTable::prepare(&bad, &styles, 96.0).is_err());
    }
}

#[test]
fn excluded_cell_anchor_cuts_preserve_host_child_tail_and_following_lines() {
    use rhwp::renderer::{style_resolver::resolve_styles, table_v2::*};
    let d = rhwp::parse_document(
        &std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/issue7353_cell_anchor_review/anchor-saved.hwp"
        ))
        .unwrap(),
    )
    .unwrap();
    let Control::Table(t) = &d.sections[0].paragraphs[0].controls[2] else {
        panic!()
    };
    let styles = resolve_styles(&d.doc_info, 96.0);
    let mut t = t.clone();
    t.page_break = TablePageBreak::CellBreak;
    t.cells[0].height = 0;
    t.padding = Default::default();
    t.cells[0].padding = Default::default();
    let Control::Table(child) = &mut t.cells[0].paragraphs[0].controls[0] else {
        panic!()
    };
    child.page_break = TablePageBreak::CellBreak;
    child.padding = Default::default();
    child.cells[0].padding = Default::default();
    let prepared = PreparedTextTable::prepare_with_end_policy(
        &t,
        &styles,
        96.0,
        &[],
        CellEndPolicy::OmitFinalParagraphGap,
    )
    .unwrap();
    let area = |height| PageArea {
        bounds: Rect {
            x: 20.0,
            y: 30.0,
            width: 800.0,
            height,
        },
    };
    let cursor = prepared.start();
    // Host line alone fits20px, but child line+top padding do not. No host-only
    // fragment may commit and separate the source owner from the deferred child.
    assert!(matches!(
        cursor.fit(area(20.0)).unwrap(),
        TextFragmentFit::DoesNotFit { .. }
    ));
    let whole = match cursor.fit(area(1000.0)).unwrap() {
        TextFragmentFit::Placed(f) => f,
        TextFragmentFit::DoesNotFit {
            required_height, ..
        } => panic!("whole requires {required_height}"),
        TextFragmentFit::Complete => panic!("whole already complete"),
    };
    let render = |part: &TextFragment| {
        let mut page = rhwp::renderer::render_tree::PageRenderTree::new(0, 800.0, 1000.0);
        part.append_to(&mut page).unwrap();
        json!({"render_tree":page})
    };
    let raw = render(&whole);
    assert_eq!(labels(&raw), ["POSITIONED CHILD", "", "AFTER ANCHOR"]);
    for budget in [30.0, 40.0, 50.0] {
        let mut cursor = prepared.start();
        let mut text = Vec::new();
        let mut hosts = 0;
        let mut blank = 0;
        let mut done = false;
        for _ in 0..20 {
            match cursor.fit(area(budget)).unwrap() {
                TextFragmentFit::Complete => {
                    done = true;
                    break;
                }
                TextFragmentFit::Placed(part) => {
                    assert!(part.geometry().reserved_height() <= budget + 1e-9);
                    let raw = render(&part);
                    text.extend(labels(&raw).into_iter().map(str::to_owned));
                    for line in nodes(&raw, "TextLine") {
                        let b = &line["bbox"];
                        assert!(b["y"].as_f64().unwrap() >= 30.0 - 1e-9);
                        assert!(
                            b["y"].as_f64().unwrap() + b["height"].as_f64().unwrap()
                                <= 30.0 + budget + 1e-9
                        );
                        if b["width"].as_f64() == Some(0.0) {
                            hosts += 1;
                        } else if line["children"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .all(|r| r["node_type"]["TextRun"]["text"] == "")
                        {
                            blank += 1;
                        }
                    }
                    cursor = part.continuation();
                }
                _ => panic!("unexpected blocked cut"),
            }
        }
        assert!(done);
        assert_eq!(text, ["POSITIONED CHILD", "", "AFTER ANCHOR"]);
        assert_eq!(hosts, 1);
        assert_eq!(blank, 1);
    }
    // Qualifying a normally saved blank host cannot admit ink, side wrap,
    // stale widths or conflicting margin records by the same shortcut.
    for kind in 0..4 {
        let mut bad = t.clone();
        let p = &mut bad.cells[0].paragraphs[0];
        match kind {
            0 => {
                p.text = "X".into();
                p.char_offsets = vec![8];
            }
            1 => {
                let Control::Table(c) = &mut p.controls[0] else {
                    panic!()
                };
                c.common.text_wrap = TextWrap::Square;
            }
            2 => p.line_segs[0].segment_width = 1,
            _ => {
                let Control::Table(c) = &mut p.controls[0] else {
                    panic!()
                };
                c.outer_margin_left += 1;
            }
        }
        assert!(PreparedTextTable::prepare(&bad, &styles, 96.0).is_err());
    }
}

#[test]
fn unpainted_tac_carrier_reference_keeps_body_and_nested_geometry() {
    use rhwp::model::style::Alignment;
    for nested in [false, true] {
        for separate in [false, true] {
            let mut d = source(vec![p("before"), inline_carrier(separate), p("after")]);
            d.doc_info.para_shapes[0].alignment = Alignment::Center;
            if nested {
                let mut outer = table(&[], TablePageBreak::CellBreak);
                outer.cells[0].width = 22500;
                outer.cells[0].paragraphs = vec![inline_carrier(separate)];
                d.sections[0].paragraphs[1] = host("host", outer);
            }
            let expected = drain(&mut open(&d));
            d.doc_info.border_fills.push(BorderFill {
                borders: [BorderLine {
                    line_type: BorderLineType::None,
                    ..Default::default()
                }; 4],
                ..Default::default()
            });
            let mut shape = d.doc_info.para_shapes[0].clone();
            shape.border_fill_id = 2;
            d.doc_info.para_shapes.push(shape);
            let carrier = if nested {
                let Control::Table(t) = &mut d.sections[0].paragraphs[1].controls[0] else {
                    unreachable!()
                };
                &mut t.cells[0].paragraphs[0]
            } else {
                &mut d.sections[0].paragraphs[1]
            };
            carrier.para_shape_id = 1;
            assert_eq!(
                drain(&mut open(&d)),
                expected,
                "nested={nested}, separate={separate}"
            );
            // Missing references and real decorations must not disappear.
            for reference in [1, 3] {
                d.doc_info.para_shapes[1].border_fill_id = reference;
                assert!(matches!(
                    DocumentV2Session::from_bytes(&bytes(&d), r#"{"dpi":96,"max_pages":20}"#),
                    Err(DocumentV2Error::Paragraph { .. })
                ));
            }
        }
    }
}

#[test]
fn hancom_saved_justified_cell_preserves_line_end_and_following_body() {
    let data = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/issue7353_justify_review/justify-saved.hwp"
    ))
    .unwrap();
    let d = rhwp::parse_document(&data).unwrap();
    let host = &d.sections[0].paragraphs[0];
    let table = host
        .controls
        .iter()
        .find_map(|c| match c {
            Control::Table(t) => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(table.common.treat_as_char);
    assert_eq!(host.line_segs[0].line_height, 7500);
    assert_eq!(d.sections[0].paragraphs[1].line_segs[0].vertical_pos, 7764);
    let rows = &table.cells[0].paragraphs[0].line_segs;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].segment_width, 23172);
    assert_eq!(rows[1].text_start, 30);
    assert_eq!(rows[1].vertical_pos, 1364);
    let pages = drain(
        &mut DocumentV2Session::from_bytes(
            &data,
            r#"{"dpi":96,"max_pages":20,"cell_end_policy":"omit_final_paragraph_gap"}"#,
        )
        .unwrap(),
    );
    assert_eq!(pages.len(), 1);
    assert_eq!(
        nodes(&pages[0], "TextLine")
            .iter()
            .map(|line| {
                line["children"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|run| run["node_type"]["TextRun"]["text"].as_str().unwrap())
                    .collect::<String>()
            })
            .collect::<Vec<_>>(),
        [
            "○ 나머지 약 79.7%(291,679천병)에는 병당 ",
            "2.6%∼100%의 암반수가 들어간 것으로 확인",
            "AFTER CELL: no missing or duplicated text",
        ]
    );
    let tables = nodes(&pages[0], "Table");
    assert_eq!(tables.len(), 1);
    for (key, hu) in [
        ("x", 3969.0),
        ("y", 5669.0),
        ("width", 23741.0),
        ("height", 7500.0),
    ] {
        near(&tables[0]["bbox"][key], hu / 75.0);
    }
    let lines = nodes(&pages[0], "TextLine");
    // Saved rows + authored padding283HU; hanging indent is2064HU.
    for (line, (x, y)) in lines
        .iter()
        .zip([(4252.0, 5952.0), (6316.0, 7316.0), (3969.0, 13433.0)])
    {
        near(&line["bbox"]["x"], x / 75.0);
        near(&line["bbox"]["y"], y / 75.0);
        near(&line["bbox"]["height"], 1100.0 / 75.0);
    }
}

#[test]
fn hancom_saved_nested_tac_with_unpainted_paragraph_border_is_admitted() {
    let data = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/issue7353_tac_noop_review/noop-saved.hwp"
    ))
    .unwrap();
    let parsed = rhwp::parse_document(&data).unwrap();
    let outer = parsed.sections[0].paragraphs[0]
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
    let carrier = &outer.cells[0].paragraphs[1];
    assert_eq!(
        parsed.doc_info.para_shapes[carrier.para_shape_id as usize].border_fill_id,
        1
    );
    assert_eq!(carrier.line_segs[0].vertical_pos, 1760);
    assert_eq!(carrier.line_segs[0].line_height, 5000);
    assert_eq!(carrier.line_segs[0].segment_width, 31432);
    let pages = drain(
        &mut DocumentV2Session::from_bytes(
            &data,
            r#"{"dpi":96,"max_pages":20,"cell_end_policy":"omit_final_paragraph_gap"}"#,
        )
        .unwrap(),
    );
    assert_eq!(pages.len(), 1);
    assert_eq!(
        labels(&pages[0]),
        [
            "NO-PAINT PARAGRAPH STYLE: NESTED INLINE TABLE",
            "CELL BEFORE",
            "INLINE CHILD TABLE",
            "CELL AFTER",
            "AFTER PARENT TABLE"
        ]
    );
    let tables = nodes(&pages[0], "Table");
    assert_eq!(tables.len(), 2);
    // Authored outer minimum18000HU, offset2835 + outer top283, page top5669.
    // Saved carrier local y1760 and width31432; center the24000HU child once.
    for (t, [x, y, w, h]) in tables.iter().zip([
        [3969.0, 8787.0, 32000.0, 18000.0],
        [
            3969.0 + 283.0 + (31432.0 - 24000.0) / 2.0,
            8787.0 + 283.0 + 1760.0,
            24000.0,
            5000.0,
        ],
    ]) {
        for (key, value) in [("x", x), ("y", y), ("width", w), ("height", h)] {
            near(&t["bbox"][key], value / 75.0);
        }
    }
    let lines = nodes(&pages[0], "TextLine");
    for (line, y) in lines
        .iter()
        .zip([5669.0, 9070.0, 11113.0, 16490.0, 5669.0 + 21685.0])
    {
        near(&line["bbox"]["y"], y / 75.0);
        near(&line["bbox"]["height"], 1100.0 / 75.0);
    }
}

#[test]
fn stored_inline_frames_align_inside_the_saved_segment_not_the_container() {
    use rhwp::model::style::Alignment;
    for alignment in [Alignment::Left, Alignment::Center, Alignment::Right] {
        let mut carrier = inline_carrier(false);
        carrier.line_segs[0].column_start = 750;
        carrier.line_segs[0].segment_width = 18000;
        let mut d = source(vec![carrier, p("after")]);
        d.doc_info.para_shapes[0].alignment = alignment;
        let pages = drain(&mut open(&d));
        assert_eq!(pages.len(), 1);
        // Saved frame x=10,width=240; two84px envelopes leave72px.
        let x = 32.0
            + match alignment {
                Alignment::Left => 0.0,
                Alignment::Center => 36.0,
                _ => 72.0,
            };
        for (i, t) in nodes(&pages[0], "Table").iter().enumerate() {
            near(&t["bbox"]["x"], x + 84.0 * i as f64);
            near(&t["bbox"]["y"], 32.0);
            near(&t["bbox"]["height"], 36.0);
        }
        near(
            &nodes(&pages[0], "TextLine").last().unwrap()["bbox"]["y"],
            74.0,
        );
        let name = match alignment {
            Alignment::Left => "document-frame-left",
            Alignment::Center => "document-frame-center",
            _ => "document-frame-right",
        };
        capture(name, &d, &pages);
    }
}

#[test]
fn signed_inline_advance_does_not_shrink_the_physical_fit_budget() {
    let mut carrier = inline_carrier(false);
    carrier.line_segs[0].line_spacing = -75; // advance39px, occupied40px
    let mut d = source(vec![p("before"), carrier, p("after")]);
    d.doc_info.para_shapes[0].alignment = rhwp::model::style::Alignment::Center;
    //57px body: after18px prose,39px remains. The40px row must move intact.
    d.sections[0].section_def.page_def.margin_bottom = 6225;
    let pages = drain(&mut open(&d));
    assert_eq!(pages.len(), 2);
    assert_eq!(labels(&pages[0]), ["before"]);
    assert_eq!(labels(&pages[1]), ["A", "a", "B", "b", "after"]);
    assert!(nodes(&pages[0], "Table").is_empty());
    for t in nodes(&pages[1], "Table") {
        near(&t["bbox"]["y"], 32.0);
        near(&t["bbox"]["height"], 36.0);
    }
    near(
        &nodes(&pages[1], "TextLine").last().unwrap()["bbox"]["y"],
        69.0,
    );
    capture("document-inline-signed-budget", &d, &pages);
}

#[test]
fn signed_nested_inline_rows_reserve_the_complete_physical_envelope() {
    let mut carrier = inline_carrier(false);
    carrier.line_segs[0].line_spacing = -75;
    let mut outer = table(&[], TablePageBreak::CellBreak);
    outer.cells[0].width = 22500;
    outer.cells[0].paragraphs = vec![carrier];
    let mut d = source(vec![host("tail", outer)]);
    d.doc_info.para_shapes[0].alignment = rhwp::model::style::Alignment::Center;
    let pages = drain(&mut open(&d));
    assert_eq!(pages.len(), 1);
    assert_eq!(labels(&pages[0]), ["A", "a", "B", "b", "tail"]);
    near(&nodes(&pages[0], "Table")[0]["bbox"]["height"], 40.0);
    near(
        &nodes(&pages[0], "TextLine").last().unwrap()["bbox"]["y"],
        70.0,
    );
    capture("document-inline-signed-nested", &d, &pages);
}

#[test]
fn terminal_policy_nested_tac_preserves_physical_budget_and_validates_children() {
    use rhwp::renderer::{style_resolver::resolve_styles, table_v2::*};
    for spacing in [-75, 300] {
        let mut carrier = inline_carrier(false);
        carrier.line_segs[0].line_spacing = spacing;
        let mut outer = table(&[], TablePageBreak::CellBreak);
        outer.cells[0].width = 22500;
        // Synthetic declared minimum36px makes the child envelope independent
        // of whether its final line advances by6px. Do not weaken TAC binding.
        for c in &mut carrier.controls {
            if let Control::Table(t) = c {
                t.cells[0].height = 2700;
            }
        }
        outer.cells[0].paragraphs = vec![carrier];
        let mut d = source(vec![host("", outer.clone())]);
        d.doc_info.para_shapes[0].alignment = rhwp::model::style::Alignment::Left;
        let styles = resolve_styles(&d.doc_info, 96.0);
        let prepared = PreparedTextTable::prepare_with_end_policy(
            &outer,
            &styles,
            96.0,
            &[],
            CellEndPolicy::OmitFinalLineGap,
        )
        .unwrap();
        let area = |height| PageArea {
            bounds: Rect {
                x: 20.0,
                y: 30.0,
                width: 300.0,
                height,
            },
        };
        assert!(matches!(
            prepared.start().fit(area(39.0)).unwrap(),
            TextFragmentFit::DoesNotFit { .. }
        ));
        let TextFragmentFit::Placed(f) = prepared.start().fit(area(40.0)).unwrap() else {
            panic!("fit")
        };
        assert_eq!(f.geometry().reserved_height(), 40.0);
        let mut page = rhwp::renderer::render_tree::PageRenderTree::new(0, 400.0, 200.0);
        f.append_to(&mut page).unwrap();
        let value = serde_json::json!({"render_tree":page});
        assert_eq!(labels(&value), ["A", "a", "B", "b"]);
        for t in nodes(&value, "Table").into_iter().skip(1) {
            near(&t["bbox"]["y"], 32.0);
            near(&t["bbox"]["height"], 36.0);
        }
        assert!(matches!(
            f.continuation().fit(area(40.0)).unwrap(),
            TextFragmentFit::Complete
        ));
        // Without that source minimum the hand-authored36px TAC metadata no
        // longer describes the actual30px child. It remains an explicit error.
        for c in &mut outer.cells[0].paragraphs[0].controls {
            if let Control::Table(t) = c {
                t.cells[0].height = 0;
            }
        }
        assert!(PreparedTextTable::prepare_with_end_policy(
            &outer,
            &styles,
            96.0,
            &[],
            CellEndPolicy::OmitFinalLineGap
        )
        .is_err());
    }
}

#[test]
fn stored_inline_row_is_reserved_once_and_defers_all_siblings() {
    let mut d = source(vec![
        p("before"),
        p("lead"),
        inline_carrier(false),
        p("after"),
    ]);
    d.doc_info.para_shapes[0].alignment = rhwp::model::style::Alignment::Center;
    let pages = drain(&mut open(&d));
    assert_eq!(pages.len(), 2);
    assert_eq!(labels(&pages[0]), ["before", "lead"]);
    assert!(nodes(&pages[0], "Table").is_empty());
    assert_eq!(labels(&pages[1]), ["A", "a", "B", "b", "after"]);
    let ts = nodes(&pages[1], "Table");
    assert_eq!(ts.len(), 2);
    // 80px tables + four 2px horizontal margins: center a168px row in300px.
    for (t, x) in ts.iter().zip([88.0, 172.0]) {
        near(&t["bbox"]["x"], x);
        near(&t["bbox"]["y"], 32.0);
        near(&t["bbox"]["width"], 80.0);
        near(&t["bbox"]["height"], 36.0);
    }
    // Saved occupied row40px, then4px spacing; no ghost host line or sum80px.
    near(&nodes(&pages[1], "TextLine")[4]["bbox"]["y"], 74.0);
    capture("document-inline", &d, &pages);
}

#[test]
fn stored_inline_distinct_rows_and_nested_path_preserve_source_ownership() {
    let mut d = source(vec![p("before"), inline_carrier(true)]);
    d.doc_info.para_shapes[0].alignment = rhwp::model::style::Alignment::Center;
    let pages = drain(&mut open(&d));
    assert_eq!(pages.len(), 2);
    assert_eq!(labels(&pages[0]), ["before", "A", "a"]);
    assert_eq!(labels(&pages[1]), ["B", "b"]);
    for (i, page) in pages.iter().enumerate() {
        let ts = nodes(page, "Table");
        assert_eq!(ts.len(), 1);
        near(&ts[0]["bbox"]["x"], 130.0);
        near(&ts[0]["bbox"]["y"], if i == 0 { 50.0 } else { 32.0 });
    }
    capture("document-inline-rows", &d, &pages);
    let mut outer = table(&[], TablePageBreak::CellBreak);
    outer.cells[0].width = 22500;
    outer.cells[0].paragraphs = vec![inline_carrier(false)];
    d.sections[0].paragraphs = vec![host("tail", outer)];
    let pages = drain(&mut open(&d));
    assert_eq!(pages.len(), 1);
    assert_eq!(labels(&pages[0]), ["A", "a", "B", "b", "tail"]);
    let ts = nodes(&pages[0], "Table");
    assert_eq!(ts.len(), 3);
    near(&ts[0]["bbox"]["height"], 44.0);
    for (t, x) in ts[1..].iter().zip([88.0, 172.0]) {
        near(&t["bbox"]["x"], x);
        near(&t["bbox"]["y"], 32.0);
    }
    near(&nodes(&pages[0], "TextLine")[4]["bbox"]["y"], 74.0);
    capture("document-inline-nested", &d, &pages);
}

#[test]
fn first_body_inline_rows_keep_structural_slots_and_final_geometry() {
    use rhwp::{model::style::Alignment, renderer::table_v2::stored_tac_rows};
    for separate in [false, true] {
        let mut d = source(vec![inline_carrier(separate), p("after")]);
        add_leading_structure(&mut d);
        d.doc_info.para_shapes[0].alignment = Alignment::Center;
        let parsed = rhwp::parse_document(&bytes(&d)).unwrap();
        let hwpx_para = &parsed.sections[0].paragraphs[0];
        // secPr is out-of-axis; the separately serialized ctrl/colPr is not.
        assert_eq!(hwpx_para.hwpx_axis_shift, 8);
        let hwp = rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap();
        let hwp_parsed = rhwp::parse_document(&hwp).unwrap();
        let para = &hwp_parsed.sections[0].paragraphs[0];
        assert_eq!(para.hwpx_axis_shift, 0);
        assert_eq!(para.char_count, 33);
        assert!(matches!(para.controls[0], Control::SectionDef(_)));
        assert!(matches!(para.controls[1], Control::ColumnDef(_)));
        let rows = stored_tac_rows(para, 22500.0, Alignment::Center).unwrap();
        assert_eq!(
            rows.iter()
                .flat_map(|r| r.tables.iter().map(|t| t.0))
                .collect::<Vec<_>>(),
            [2, 3]
        );
        if separate {
            assert_eq!(para.line_segs[1].text_start, 24);
            assert_eq!(para.line_seg_text_start(1), 24);
            assert_eq!(rows[0].tables.len(), 1);
            assert_eq!(rows[1].tables[0].0, 3);
            assert!(matches!(
                DocumentV2Session::from_bytes(&bytes(&d), r#"{"dpi":96,"max_pages":20}"#),
                Err(DocumentV2Error::Paragraph {
                    reason: rhwp::renderer::table_v2::GeometryError::Unsupported(
                        "ambiguous structural TAC character axis"
                    ),
                    ..
                })
            ));
        }
        let pages = drain(
            &mut DocumentV2Session::from_bytes(&hwp, r#"{"dpi":96,"max_pages":20}"#).unwrap(),
        );
        if !separate {
            let hwpx_pages = drain(&mut open(&d));
            assert_eq!(hwpx_pages.len(), pages.len());
            for (a, b) in hwpx_pages.iter().zip(&pages) {
                assert_eq!(labels(a), labels(b));
                assert_eq!(
                    nodes(a, "Table")
                        .iter()
                        .map(|n| &n["bbox"])
                        .collect::<Vec<_>>(),
                    nodes(b, "Table")
                        .iter()
                        .map(|n| &n["bbox"])
                        .collect::<Vec<_>>()
                );
            }
        }
        assert_eq!(pages.len(), if separate { 2 } else { 1 });
        if separate {
            assert_eq!(labels(&pages[0]), ["A", "a"]);
            assert_eq!(labels(&pages[1]), ["B", "b", "after"]);
        } else {
            assert_eq!(labels(&pages[0]), ["A", "a", "B", "b", "after"]);
        }
        for page in &pages {
            let ts = nodes(page, "Table");
            assert_eq!(ts.len(), if separate { 1 } else { 2 });
            for (i, t) in ts.iter().enumerate() {
                near(
                    &t["bbox"]["x"],
                    if separate {
                        130.0
                    } else {
                        88.0 + 84.0 * i as f64
                    },
                );
                near(&t["bbox"]["y"], 32.0);
                near(&t["bbox"]["width"], 80.0);
                near(&t["bbox"]["height"], 36.0);
            }
        }
        let final_lines = nodes(pages.last().unwrap(), "TextLine");
        near(&final_lines.last().unwrap()["bbox"]["y"], 74.0);
        if separate {
            if let Ok(dir) = std::env::var("ISSUE7353_EXPORT_DIR") {
                std::fs::write(format!("{dir}/document-inline-first-rows.hwp"), &hwp).unwrap();
                std::fs::write(
                    format!("{dir}/document-inline-first-rows.options.json"),
                    r#"{"dpi":96,"max_pages":20}"#,
                )
                .unwrap();
                std::fs::write(
                    format!("{dir}/document-inline-first-rows.native.json"),
                    serde_json::to_vec_pretty(&pages).unwrap(),
                )
                .unwrap();
                std::fs::write(format!("{dir}/unsupported-structural-axis.hwpx"), bytes(&d))
                    .unwrap();
            }
        } else {
            capture("document-inline-first", &d, &drain(&mut open(&d)));
        }
        let mut bad = para.clone();
        bad.hwpx_axis_shift = 24;
        assert!(stored_tac_rows(&bad, 22500.0, Alignment::Center).is_err());
        bad.hwpx_axis_shift = 7;
        assert!(stored_tac_rows(&bad, 22500.0, Alignment::Center).is_err());
    }
}

// Hand-authored IR with explicit HWP5 slots, not a saved-row cache made before
// the serializer inserts the section's metadata. This is a boundary contract,
// not a Hancom generated source or a repair of the preserved old fixture.
fn add_leading_structure(d: &mut Document) {
    let def = d.sections[0].section_def.clone();
    let para = &mut d.sections[0].paragraphs[0];
    para.controls.insert(0, Control::SectionDef(Box::new(def)));
    para.controls
        .insert(1, Control::ColumnDef(Default::default()));
    para.char_count += 16;
    for line in &mut para.line_segs {
        if line.text_start != 0 {
            line.text_start += 16;
        }
    }
}

#[test]
fn footer_story_preserves_first_paragraph_tac_slots_and_body_fragments() {
    use rhwp::{
        model::{control::PageNumberPos, style::Alignment},
        renderer::table_v2::stored_tac_rows,
    };
    let mut d = source(vec![inline_carrier(true), p("after")]);
    add_leading_structure(&mut d);
    d.doc_info.para_shapes[0].alignment = Alignment::Center;
    let encode = |d: &Document| rhwp::serializer::cfb_writer::serialize_hwp(d).unwrap();
    let options = r#"{"dpi":96,"max_pages":20}"#;
    let mut reference = drain(&mut DocumentV2Session::from_bytes(&encode(&d), options).unwrap());
    // Inserting a source control shifts both tables' source identities, but
    // must not alter any of their geometry, fragments or descendant paint.
    fn shift_table_owners(node: &mut Value) {
        if let Some(table) = node["node_type"].get_mut("Table") {
            table["control_index"] = Value::from(table["control_index"].as_u64().unwrap() + 1);
        }
        for child in node["children"].as_array_mut().unwrap() {
            shift_table_owners(child);
        }
    }
    for page in &mut reference {
        shift_table_owners(&mut page["render_tree"]["root"]);
    }
    let para = &mut d.sections[0].paragraphs[0];
    para.controls.insert(
        2,
        Control::PageNumberPos(PageNumberPos {
            position: 5,
            dash_char: '-',
            ..Default::default()
        }),
    );
    para.char_count += 8;
    for line in &mut para.line_segs {
        if line.text_start != 0 {
            line.text_start += 8;
        }
    }
    let input = encode(&d);
    let parsed = rhwp::parse_document(&input).unwrap();
    let rows = stored_tac_rows(
        &parsed.sections[0].paragraphs[0],
        22500.0,
        Alignment::Center,
    )
    .unwrap();
    assert_eq!(
        rows.iter()
            .flat_map(|r| r.tables.iter().map(|t| t.0))
            .collect::<Vec<_>>(),
        [3, 4]
    );
    let mut session = DocumentV2Session::from_bytes(&input, options).unwrap();
    let mut raw = Vec::new();
    while let Some(page) = session.next_page_json().unwrap() {
        raw.push(page);
    }
    assert_eq!(raw.len(), 2);
    for (i, page) in raw.iter().enumerate() {
        let page: Value = serde_json::from_str(page).unwrap();
        assert_eq!(nodes(&page, "Body"), nodes(&reference[i], "Body"));
        assert_eq!(labels(&page).last().unwrap(), &format!("- {} -", i + 1));
    }
    if let Ok(dir) = std::env::var("ISSUE7353_EXPORT_DIR") {
        let name = "document-number-tac-hwp";
        std::fs::write(format!("{dir}/{name}.hwp"), input).unwrap();
        std::fs::write(format!("{dir}/{name}.options.json"), options).unwrap();
        std::fs::write(
            format!("{dir}/{name}.native.json"),
            format!("[{}]", raw.join(",")),
        )
        .unwrap();
    }
}

#[test]
fn structural_carriers_do_not_silently_drop_unhandled_stories_or_nonforward_spacing() {
    use rhwp::renderer::table_v2::GeometryError;
    let mut d = source(vec![inline_carrier(false)]);
    d.doc_info.para_shapes[0].alignment = rhwp::model::style::Alignment::Center;
    add_leading_structure(&mut d);
    d.sections[0].paragraphs[0].line_segs[0].line_spacing = -3000;
    assert!(matches!(
        DocumentV2Session::from_bytes(
            &rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap(),
            r#"{"dpi":96,"max_pages":20}"#
        ),
        Err(DocumentV2Error::Paragraph {
            index: 0,
            reason: GeometryError::Unsupported("non-forward TAC row advance")
        })
    ));
    let mut d = source(vec![inline_carrier(false)]);
    add_leading_structure(&mut d);
    d.sections[0].paragraphs[0]
        .controls
        .push(Control::PageNumberPos(
            rhwp::model::control::PageNumberPos {
                format: 1,
                ..Default::default()
            },
        ));
    d.sections[0].paragraphs[0].char_count += 8;
    assert!(matches!(
        DocumentV2Session::from_bytes(&bytes(&d), r#"{"dpi":96,"max_pages":20}"#),
        Err(DocumentV2Error::Paragraph {
            index: 0,
            reason: GeometryError::Unsupported("page-number format or position")
        })
    ));
    // HWP preserves these declarations as individual source slots. Duplicate,
    // multi-column and later-paragraph metadata must not disappear in dispatch.
    for kind in 0..3 {
        let mut d = source(vec![inline_carrier(false)]);
        d.doc_info.para_shapes[0].alignment = rhwp::model::style::Alignment::Center;
        add_leading_structure(&mut d);
        match kind {
            0 => {
                d.sections[0].paragraphs[0]
                    .controls
                    .insert(2, Control::ColumnDef(Default::default()));
                d.sections[0].paragraphs[0].char_count += 8;
            }
            1 => {
                let Control::ColumnDef(c) = &mut d.sections[0].paragraphs[0].controls[1] else {
                    unreachable!()
                };
                c.column_count = 2;
            }
            _ => {
                let mut para = p("");
                para.controls.push(Control::ColumnDef(Default::default()));
                para.char_count = 9;
                d.sections[0].paragraphs.push(para);
            }
        }
        let encoded = rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap();
        let error = DocumentV2Session::from_bytes(&encoded, r#"{"dpi":96,"max_pages":20}"#)
            .err()
            .expect("structural guard");
        assert!(
            matches!(
                error,
                DocumentV2Error::Paragraph {
                    reason: GeometryError::Unsupported("body control or multiple anchors"),
                    ..
                }
            ),
            "{kind}: {error}"
        );
    }
}

#[test]
fn stored_inline_rejects_incomplete_ownership_and_changed_content_boxes() {
    use rhwp::{model::style::Alignment, renderer::table_v2::stored_tac_rows};
    let source_para = inline_carrier(false);
    assert_eq!(
        stored_tac_rows(&source_para, 22500.0, Alignment::Center).unwrap()[0]
            .tables
            .len(),
        2
    );
    let mut invalid = source_para.clone();
    invalid.char_count -= 1;
    assert!(stored_tac_rows(&invalid, 22500.0, Alignment::Center).is_err());
    let mut invalid = inline_carrier(true);
    invalid.line_segs[1].text_start = 7;
    assert!(stored_tac_rows(&invalid, 22500.0, Alignment::Center).is_err());
    invalid.line_segs[1].text_start = 8;
    invalid.line_segs[1].vertical_pos = invalid.line_segs[0].vertical_pos;
    assert!(stored_tac_rows(&invalid, 22500.0, Alignment::Center).is_err());
    for (start, width) in [(-1, 22500), (1, 22500), (0, 0), (0, 12000)] {
        let mut invalid = source_para.clone();
        invalid.line_segs[0].column_start = start;
        invalid.line_segs[0].segment_width = width;
        assert!(stored_tac_rows(&invalid, 22500.0, Alignment::Center).is_err());
    }
    assert!(stored_tac_rows(&source_para, 22499.0, Alignment::Center).is_err());
    let mut d = source(vec![p("before"), source_para]);
    d.doc_info.para_shapes[0].alignment = Alignment::Center;
    if let Control::Table(t) = &mut d.sections[0].paragraphs[1].controls[0] {
        let extra = t.cells[0].paragraphs[0].clone();
        t.cells[0].paragraphs.push(extra);
    }
    match DocumentV2Session::from_bytes(&bytes(&d), r#"{"dpi":96,"max_pages":20}"#) {
        Err(DocumentV2Error::Paragraph {
            index: 1,
            reason:
                rhwp::renderer::table_v2::GeometryError::Unsupported(
                    "TAC content changed stored occupied box",
                ),
        }) => {}
        _ => panic!("changed child content must reject its stale stored box"),
    }
}
