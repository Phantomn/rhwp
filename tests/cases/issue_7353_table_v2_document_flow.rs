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

#[test]
fn real_6923_remains_unmodified_and_explicitly_unqualified() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp");
    let data = std::fs::read(path).expect("required original fixture");
    let d = rhwp::parse_document(&data).unwrap();
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
    let structural = bytes(&source(vec![inline_carrier(true)]));
    assert!(matches!(
        DocumentV2Session::from_bytes(&structural, r#"{"dpi":96,"max_pages":20}"#),
        Err(DocumentV2Error::Paragraph {
            index: 0,
            reason: rhwp::renderer::table_v2::GeometryError::Unsupported(
                "body control or multiple anchors"
            )
        })
    ));
    if let Ok(dir) = std::env::var("ISSUE7353_EXPORT_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/unsupported-structural-tac.hwpx"), structural).unwrap();
    }
    // Keep structural SectionDef/ColumnDef outside the TAC carrier. Their saved
    // character-axis mapping is a separate, still unsupported source boundary.
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
    invalid.line_segs[1].vertical_pos = 2000;
    assert!(stored_tac_rows(&invalid, 22500.0, Alignment::Center).is_err());
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
