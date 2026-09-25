//! Footer stories: source positions per HWP5 table148; 10pt automatic numbers.
//! Synthetic source body=(20,30,300,72)px. Footer distance=30px, therefore
//! run top=102+15+40/9 and baseline=top+40/3. Not a full Hancom fidelity claim.
use rhwp::{
    model::{
        control::{Control, PageNumberPos},
        document::{Document, Section},
        page::PageDef,
        paragraph::{CharShapeRef, Paragraph},
        shape::{CommonObjAttr, HorzRelTo, TextWrap, VertRelTo},
        style::{CharShape, LineSpacingType, ParaShape},
        table::{Cell, Table, TablePageBreak},
    },
    renderer::table_v2::DocumentV2Session,
};
use serde_json::Value;

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
fn insert(para: &mut Paragraph, c: Control) {
    para.controls.insert(0, c);
    para.char_count += 8;
    for offset in &mut para.char_offsets {
        *offset += 8;
    }
}
fn doc(number: Option<PageNumberPos>) -> Document {
    let mut before = p("before");
    if let Some(n) = number {
        insert(&mut before, Control::PageNumberPos(n));
    }
    let mut host = p("host");
    insert(
        &mut host,
        Control::Table(Box::new(Table {
            row_count: 1,
            col_count: 1,
            page_break: TablePageBreak::CellBreak,
            common: CommonObjAttr {
                text_wrap: TextWrap::TopAndBottom,
                horz_rel_to: HorzRelTo::Para,
                vert_rel_to: VertRelTo::Para,
                ..Default::default()
            },
            cells: vec![Cell {
                width: 15000,
                row_span: 1,
                col_span: 1,
                paragraphs: ["A", "B", "C", "D", "E"].map(p).into(),
                ..Default::default()
            }],
            ..Default::default()
        })),
    );
    let mut section = Section {
        paragraphs: vec![before, host, p("after")],
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
    let mut d = Document::default();
    d.sections.push(section);
    d.doc_info.char_shapes.push(CharShape {
        base_size: 900,
        ..Default::default()
    });
    d.doc_info.para_shapes.push(ParaShape {
        line_spacing_type: LineSpacingType::Fixed,
        line_spacing: 2700,
        ..Default::default()
    });
    d
}
fn number(position: u8) -> PageNumberPos {
    PageNumberPos {
        position,
        dash_char: '-',
        ..Default::default()
    }
}
fn encode(d: &Document, hwp: bool) -> Vec<u8> {
    if hwp {
        rhwp::serializer::cfb_writer::serialize_hwp(d).unwrap()
    } else {
        rhwp::serializer::hwpx::serialize_hwpx(d).unwrap()
    }
}
const OPTIONS: &str = r#"{"dpi":96,"max_pages":10}"#;
fn pages(d: &Document, hwp: bool, name: &str) -> Vec<Value> {
    let bytes = encode(d, hwp);
    let mut session = DocumentV2Session::from_bytes(&bytes, OPTIONS).unwrap();
    let mut raw = Vec::new();
    while let Some(page) = session.next_page_json().unwrap() {
        raw.push(page);
        assert!(raw.len() < 10);
    }
    assert_eq!(session.emitted_pages() as usize, raw.len());
    assert!(session.next_page_json().unwrap().is_none());
    if !name.is_empty() {
        if let Ok(dir) = std::env::var("ISSUE7353_EXPORT_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            let ext = if hwp { "hwp" } else { "hwpx" };
            std::fs::write(format!("{dir}/{name}.{ext}"), bytes).unwrap();
            std::fs::write(format!("{dir}/{name}.options.json"), OPTIONS).unwrap();
            std::fs::write(
                format!("{dir}/{name}.native.json"),
                format!("[{}]", raw.join(",")),
            )
            .unwrap();
        }
    }
    raw.iter()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn near(v: f64, expected: f64) {
    assert!(
        (v - expected).abs() <= 32.0 * f64::EPSILON * expected.abs().max(1.0),
        "{v} != {expected}"
    );
}
fn bbox(n: &Value, k: &str) -> f64 {
    n["bbox"][k].as_f64().unwrap()
}

#[test]
fn decimal_footer_uses_its_own_geometry_and_never_changes_body_fit() {
    for hwp in [false, true] {
        let reference = pages(&doc(None), hwp, "");
        for position in [4, 5, 6] {
            let name = format!(
                "document-number-{position}{}",
                if hwp { "-hwp" } else { "" }
            );
            let actual = pages(&doc(Some(number(position))), hwp, &name);
            assert_eq!(actual.len(), 2);
            for (i, page) in actual.iter().enumerate() {
                let children = page["render_tree"]["root"]["children"].as_array().unwrap();
                assert_eq!(children.len(), 2);
                assert_eq!(
                    children[0], reference[i]["render_tree"]["root"]["children"][0],
                    "body geometry, ownership and complete fragments unchanged"
                );
                let line = &children[1];
                let run = &line["children"][0];
                assert_eq!(
                    run["node_type"]["TextRun"]["text"],
                    format!("- {} -", i + 1)
                );
                near(
                    run["node_type"]["TextRun"]["style"]["font_size"]
                        .as_f64()
                        .unwrap(),
                    40.0 / 3.0,
                );
                near(bbox(run, "y"), 102.0 + 15.0 + 40.0 / 9.0);
                near(bbox(line, "y"), bbox(run, "y"));
                near(bbox(line, "height"), 16.0);
                near(bbox(run, "width"), bbox(line, "width"));
                let anchor = match position {
                    4 => bbox(run, "x"),
                    5 => bbox(run, "x") + bbox(run, "width") / 2.0,
                    _ => bbox(run, "x") + bbox(run, "width"),
                };
                near(
                    anchor,
                    match position {
                        4 => 20.0,
                        5 => 170.0,
                        _ => 320.0,
                    },
                );
                assert!(bbox(run, "y") > 102.0);
                assert!(bbox(line, "y") + bbox(line, "height") < 200.0);
            }
        }
    }
}

#[test]
fn declared_start_decoration_and_hidden_position_are_preserved() {
    let mut d = doc(Some(PageNumberPos {
        position: 5,
        prefix_char: '[',
        suffix_char: ']',
        dash_char: '\0',
        ..Default::default()
    }));
    d.sections[0].section_def.page_num = 9;
    // HWP stores prefix/suffix; HWPX pageNum only has sideChar.
    let actual = pages(&d, true, "document-number-start-hwp");
    for (i, p) in actual.iter().enumerate() {
        assert_eq!(
            p["render_tree"]["root"]["children"][1]["children"][0]["node_type"]["TextRun"]["text"],
            format!("[{}]", 9 + i)
        );
    }
    assert_eq!(
        pages(&doc(Some(number(0))), false, "document-number-hidden"),
        pages(&doc(None), false, "")
    );
}

#[test]
fn unsupported_stories_are_not_silently_discarded() {
    for kind in 0..6 {
        let mut d = doc(Some(number(5)));
        match kind {
            0 => {
                if let Control::PageNumberPos(p) = &mut d.sections[0].paragraphs[0].controls[0] {
                    p.position = 2;
                }
            }
            1 => {
                if let Control::PageNumberPos(p) = &mut d.sections[0].paragraphs[0].controls[0] {
                    p.format = 1;
                }
            }
            2 => {
                if let Control::PageNumberPos(p) = &mut d.sections[0].paragraphs[0].controls[0] {
                    p.prefix_char = '\n';
                }
            }
            3 => insert(
                &mut d.sections[0].paragraphs[0],
                Control::PageNumberPos(number(5)),
            ),
            4 => insert(
                &mut d.sections[0].paragraphs[2],
                Control::PageNumberPos(number(4)),
            ),
            _ => {
                if let Control::Table(t) = &mut d.sections[0].paragraphs[1].controls[0] {
                    insert(
                        &mut t.cells[0].paragraphs[0],
                        Control::PageNumberPos(number(5)),
                    );
                }
            }
        }
        assert!(
            DocumentV2Session::from_bytes(&encode(&d, true), OPTIONS).is_err(),
            "case {kind}"
        );
    }
}

#[test]
fn number_overflow_is_transactional_including_clone_resume() {
    let mut d = doc(Some(number(5)));
    d.sections[0].section_def.page_num = u16::MAX;
    let mut s = DocumentV2Session::from_bytes(&encode(&d, false), OPTIONS).unwrap();
    assert!(s.next_page_json().unwrap().unwrap().contains("65535"));
    let mut copy = s.clone();
    for _ in 0..2 {
        for session in [&mut s, &mut copy] {
            assert!(session
                .next_page_json()
                .unwrap_err()
                .to_string()
                .contains("page-number range"));
            assert_eq!(session.emitted_pages(), 1);
        }
    }
}

#[test]
fn footer_point_geometry_scales_with_dpi_without_changing_pagination() {
    let input = encode(&doc(Some(number(5))), true);
    for dpi in [72.0, 144.0] {
        let options = format!(r#"{{"dpi":{dpi},"max_pages":10}}"#);
        let mut session = DocumentV2Session::from_bytes(&input, &options).unwrap();
        for _ in 0..2 {
            let page: Value =
                serde_json::from_str(&session.next_page_json().unwrap().unwrap()).unwrap();
            let line = &page["render_tree"]["root"]["children"][1];
            let run = &line["children"][0];
            let scale = dpi / 96.0;
            near(bbox(run, "y"), (102.0 + 15.0 + 40.0 / 9.0) * scale);
            near(bbox(run, "x") + bbox(run, "width") / 2.0, 170.0 * scale);
            near(bbox(line, "height"), 16.0 * scale);
        }
        assert!(session.next_page_json().unwrap().is_none());
    }
}

#[test]
fn footer_outside_physical_page_is_rejected_without_consuming_body() {
    let mut d = doc(Some(number(5)));
    let page = &mut d.sections[0].section_def.page_def;
    page.margin_bottom = 0;
    page.margin_footer = 0;
    let mut session = DocumentV2Session::from_bytes(&encode(&d, true), OPTIONS).unwrap();
    for _ in 0..2 {
        assert!(session
            .next_page_json()
            .unwrap_err()
            .to_string()
            .contains("page-number outside page"));
        assert_eq!(session.emitted_pages(), 0);
    }
}
