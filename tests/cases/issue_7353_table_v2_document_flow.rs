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
fn open(d: &Document) -> DocumentV2Session {
    DocumentV2Session::from_bytes(&bytes(d), r#"{"dpi":96,"max_pages":20}"#).unwrap()
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
fn capture(name: &str, d: &Document, pages: &[Value]) {
    if let Ok(dir) = std::env::var("ISSUE7353_EXPORT_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/{name}.hwpx"), bytes(d)).unwrap();
        std::fs::write(
            format!("{dir}/{name}.options.json"),
            r#"{"dpi":96,"max_pages":20}"#,
        )
        .unwrap();
        std::fs::write(
            format!("{dir}/{name}.native.json"),
            serde_json::to_vec_pretty(pages).unwrap(),
        )
        .unwrap();
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
fn real_6923_remains_unmodified_and_explicitly_unqualified() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp");
    let data = std::fs::read(path).expect("required original fixture");
    let d = rhwp::parse_document(&data).unwrap();
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
        Err(e) => e.to_string(),
        Ok(_) => panic!("update real admission evidence before claiming support"),
    };
    if let Ok(dir) = std::env::var("ISSUE7353_EXPORT_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/6923-admission.json"),serde_json::to_vec_pretty(&json!({
            "paragraphs":counts[0],"stored_paragraphs":counts[1],"tables":counts[2],
            "rowspan_cells":counts[3],"tac_tables":counts[4],"pictures":counts[5],"rejection":reason,
            "result":"UNSUPPORTED - not a fidelity pass"})).unwrap()).unwrap();
    }
}
