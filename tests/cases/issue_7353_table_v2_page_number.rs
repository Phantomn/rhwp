//! Footer stories: source positions per HWP5 table148; 10pt automatic numbers.
//! Synthetic body=(20,30,300,72)px; paper height200, bottom margin68px.
//! With a footer allocation the10pt line ends at132px, baseline130px.
//! Independent Hancom margin fixtures below qualify the default footer rule.
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
    // Give the structural declaration its own source slot. Otherwise HWP's
    // writer inserts secd into the eight units intended for pgnp and moves the
    // latter after the visible text. HWPX happened to repair that old fixture.
    let definition = section.section_def.clone();
    insert(
        &mut section.paragraphs[0],
        Control::SectionDef(Box::new(definition)),
    );
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
                near(bbox(run, "y"), 132.0 - 40.0 / 3.0);
                near(bbox(line, "y"), bbox(run, "y"));
                near(bbox(line, "height"), 40.0 / 3.0);
                near(
                    bbox(run, "y") + run["node_type"]["TextRun"]["baseline"].as_f64().unwrap(),
                    130.0,
                );
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
    if let Control::SectionDef(def) = &mut d.sections[0].paragraphs[0].controls[0] {
        def.page_num = 9;
    }
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
                if let Control::PageNumberPos(p) = &mut d.sections[0].paragraphs[0].controls[1] {
                    p.position = 2;
                }
            }
            1 => {
                if let Control::PageNumberPos(p) = &mut d.sections[0].paragraphs[0].controls[1] {
                    p.format = 1;
                }
            }
            2 => {
                if let Control::PageNumberPos(p) = &mut d.sections[0].paragraphs[0].controls[1] {
                    p.prefix_char = '\n';
                }
            }
            3 => insert(
                &mut d.sections[0].paragraphs[0],
                Control::PageNumberPos(number(5)),
            ),
            4 => insert(
                &mut d.sections[0].paragraphs[2],
                // Later entry declarations are supported by the independent
                // page-number-timeline fixture; unsupported formats still fail.
                Control::PageNumberPos(PageNumberPos {
                    format: 1,
                    ..number(4)
                }),
            ),
            _ => {
                if let Control::Table(t) = &mut d.sections[0].paragraphs[1].controls[0] {
                    insert(
                        &mut t.cells[0].paragraphs[0],
                        Control::PageNumberPos(PageNumberPos {
                            format: 1,
                            ..number(5)
                        }),
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
fn cell_number_waits_for_its_actual_split_fragment_and_keeps_body_geometry() {
    for hwp in [true, false] {
        let reference = pages(&doc(None), hwp, "");
        let mut d = doc(None);
        let Control::Table(t) = &mut d.sections[0].paragraphs[1].controls[0] else {
            unreachable!()
        };
        // The saved fixture's first fragment does not contain D (checked
        // below). Its declaration belongs to the later fragment, not the
        // first fragment merely because both are owned by the same table.
        insert(
            &mut t.cells[0].paragraphs[3],
            Control::PageNumberPos(number(5)),
        );
        let actual = pages(
            &d,
            hwp,
            if hwp {
                "split-cell-number-hwp"
            } else {
                "split-cell-number-hwpx"
            },
        );
        assert_eq!(actual.len(), reference.len());
        let mut active = false;
        for (i, (page, before)) in actual.iter().zip(&reference).enumerate() {
            let children = page["render_tree"]["root"]["children"].as_array().unwrap();
            let body = &children[0];
            assert_eq!(body, &before["render_tree"]["root"]["children"][0]);
            fn contains_d(n: &Value) -> bool {
                n["node_type"]["TextRun"]["text"] == "D"
                    || n["children"].as_array().unwrap().iter().any(contains_d)
            }
            active |= contains_d(body);
            assert_eq!(children.len(), if active { 2 } else { 1 });
            if active {
                assert_eq!(
                    children[1]["children"][0]["node_type"]["TextRun"]["text"],
                    format!("- {} -", i + 1)
                );
            }
        }
        assert!(active);
        assert_eq!(
            actual[0]["render_tree"]["root"]["children"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
}

#[test]
fn nested_cell_declaration_follows_accepted_owner_not_visible_text() {
    for hwp in [true, false] {
        for blank in [false, true] {
            let mut d = doc(None);
            let Control::Table(mut child) = d.sections[0].paragraphs[1].controls.remove(0) else {
                unreachable!()
            };
            if blank {
                child.cells[0].paragraphs[3] = p("");
            }
            let mut carrier = p("");
            insert(&mut carrier, Control::Table(child));
            d.sections[0].paragraphs[1]
                .controls
                .push(Control::Table(Box::new(Table {
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
                        width: 18000,
                        row_span: 1,
                        col_span: 1,
                        paragraphs: vec![carrier],
                        ..Default::default()
                    }],
                    ..Default::default()
                })));
            let reference = pages(&d, hwp, "");
            let Control::Table(outer) = &mut d.sections[0].paragraphs[1].controls[0] else {
                unreachable!()
            };
            let Control::Table(child) = &mut outer.cells[0].paragraphs[0].controls[0] else {
                unreachable!()
            };
            insert(
                &mut child.cells[0].paragraphs[3],
                Control::PageNumberPos(number(5)),
            );
            let actual = pages(
                &d,
                hwp,
                if blank {
                    if hwp {
                        "nested-blank-number-hwp"
                    } else {
                        "nested-blank-number-hwpx"
                    }
                } else {
                    if hwp {
                        "nested-cell-number-hwp"
                    } else {
                        "nested-cell-number-hwpx"
                    }
                },
            );
            assert_eq!(actual.len(), reference.len());
            for (i, (a, b)) in actual.iter().zip(&reference).enumerate() {
                assert_eq!(
                    a["render_tree"]["root"]["children"][0],
                    b["render_tree"]["root"]["children"][0]
                );
                // A/B/C precede the cut. The fourth paragraph is the first
                // accepted child line on page2, even when it has no glyphs;
                // the page3 following body must retain the active story.
                let children = a["render_tree"]["root"]["children"].as_array().unwrap();
                assert_eq!(children.len(), if i == 0 { 1 } else { 2 });
                if i > 0 {
                    assert_eq!(
                        children[1]["children"][0]["node_type"]["TextRun"]["text"],
                        format!("- {} -", i + 1)
                    );
                }
            }
            assert_eq!(
                actual[0]["render_tree"]["root"]["children"]
                    .as_array()
                    .unwrap()
                    .len(),
                1
            );
            assert_eq!(
                actual.last().unwrap()["render_tree"]["root"]["children"][1]["children"][0]
                    ["node_type"]["TextRun"]["text"],
                format!("- {} -", actual.len())
            );
        }
    }
}

#[test]
fn cell_declaration_inside_text_is_not_silently_promoted_to_entry() {
    let mut d = doc(None);
    let Control::Table(t) = &mut d.sections[0].paragraphs[1].controls[0] else {
        unreachable!()
    };
    let para = &mut t.cells[0].paragraphs[0];
    // Text comes first. This is deliberately not an entry declaration.
    para.controls.push(Control::PageNumberPos(number(5)));
    para.char_count += 8;
    for hwp in [true, false] {
        assert!(DocumentV2Session::from_bytes(&encode(&d, hwp), OPTIONS).is_err());
    }
}

#[test]
fn stored_cell_declaration_waits_for_its_line_not_just_its_paragraph() {
    use rhwp::model::paragraph::LineSeg;
    let mut d = doc(None);
    let Control::Table(t) = &mut d.sections[0].paragraphs[1].controls[0] else {
        unreachable!()
    };
    let mut para = p("AD");
    // Synthetic ownership boundary, not a Hancom fidelity fixture: the page
    // declaration follows D on the second stored line of a split paragraph.
    para.line_segs = (0..2)
        .map(|i| LineSeg {
            text_start: i,
            vertical_pos: i as i32 * 2700,
            line_height: 900,
            text_height: 900,
            baseline_distance: 765,
            line_spacing: 1800,
            segment_width: 15000,
            tag: LineSeg::TAG_SINGLE_SEGMENT_LINE,
            ..Default::default()
        })
        .collect();
    para.controls.push(Control::PageNumberPos(number(5)));
    para.char_count += 8;
    t.cells[0].paragraphs = vec![para];
    let actual = pages(&d, true, "stored-second-line-number-hwp");
    assert!(actual.len() > 1);
    assert_eq!(
        actual[0]["render_tree"]["root"]["children"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        actual[1]["render_tree"]["root"]["children"][1]["children"][0]["node_type"]["TextRun"]
            ["text"],
        "- 2 -"
    );
}

#[test]
fn later_declaration_after_split_table_preserves_body_and_updates_accepted_page() {
    for hwp in [true, false] {
        let mut d = doc(Some(number(5)));
        let baseline = pages(&d, hwp, "");
        insert(
            &mut d.sections[0].paragraphs[2],
            Control::PageNumberPos(number(4)),
        );
        let actual = pages(&d, hwp, "");
        assert_eq!(actual.len(), baseline.len());
        assert_eq!(actual.len(), 2);
        for (i, page) in actual.iter().enumerate() {
            let c = &page["render_tree"]["root"]["children"];
            assert_eq!(c[0], baseline[i]["render_tree"]["root"]["children"][0]);
            if i == 0 {
                near(bbox(&c[1], "x") + bbox(&c[1], "width") / 2., 170.);
            } else {
                near(bbox(&c[1], "x"), 20.);
            }
            assert_eq!(
                c[1]["children"][0]["node_type"]["TextRun"]["text"],
                format!("- {} -", i + 1)
            );
        }
    }
}

#[test]
fn number_overflow_is_transactional_including_clone_resume() {
    let mut d = doc(Some(number(5)));
    d.sections[0].section_def.page_num = u16::MAX;
    if let Control::SectionDef(def) = &mut d.sections[0].paragraphs[0].controls[0] {
        def.page_num = u16::MAX;
    }
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
            near(bbox(run, "y"), (132.0 - 40.0 / 3.0) * scale);
            near(bbox(run, "x") + bbox(run, "width") / 2.0, 170.0 * scale);
            near(bbox(line, "height"), 40.0 / 3.0 * scale);
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
    // Zero margins alone do not place the number outside paper: independent
    // b0f0 PDF prints it just inside the bottom edge. Use a page genuinely
    // shorter than the default10pt number line, with a1pt body that still fits.
    page.height = 500;
    page.margin_top = 0;
    page.margin_header = 0;
    d.sections[0].paragraphs.truncate(1);
    d.doc_info.char_shapes[0].base_size = 100;
    d.doc_info.para_shapes[0].line_spacing = 200;
    let definition = d.sections[0].section_def.clone();
    d.sections[0].paragraphs[0].controls[0] = Control::SectionDef(Box::new(definition));
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

#[test]
fn late_single_declaration_keeps_its_blank_host_and_section_footer() {
    // Normal Hancom saves retain the declaration in paragraph25/40. The
    // matching PDFs number pages from the host page (no number on an earlier page).
    for (data, pi, host_page) in [
        (
            include_bytes!("../fixtures/issue7353/late-page-number/late-saved.hwp").as_slice(),
            25,
            0,
        ),
        (
            include_bytes!("../fixtures/issue7353/late-page-number/second-saved.hwp").as_slice(),
            40,
            1,
        ),
    ] {
        let mut d = rhwp::parse_document(data).unwrap();
        let para = &d.sections[0].paragraphs[pi];
        assert!(para.text.is_empty());
        assert!(matches!(
            para.controls.as_slice(),
            [Control::PageNumberPos(_)]
        ));
        assert_eq!(
            (
                para.line_segs[0].line_height,
                para.line_segs[0].line_spacing
            ),
            (1000, 500)
        );
        let mut session = DocumentV2Session::from_bytes(data, OPTIONS).unwrap();
        let mut actual: Vec<Value> = Vec::new();
        while let Some(page) = session.next_page_json().unwrap() {
            actual.push(serde_json::from_str(&page).unwrap());
        }
        assert_eq!(actual.len(), 2);
        assert!(session.next_page_json().unwrap().is_none());
        // HWPX serialization of the same saved IR exercises the other parser;
        // it is not a separately Hancom-authored fidelity fixture.
        assert_eq!(actual, pages(&d, false, ""));
        // Remove only the control in a semantic no-story control. Preserve its
        // real blank line, stored metrics and every other paragraph/slot.
        let para = &mut d.sections[0].paragraphs[pi];
        para.controls.clear();
        para.char_count -= 8;
        for hwp in [true, false] {
            let no_story = pages(&d, hwp, "");
            assert_eq!(no_story.len(), actual.len());
            for (i, page) in actual.iter().enumerate() {
                let children = page["render_tree"]["root"]["children"].as_array().unwrap();
                assert_eq!(children.len(), if i < host_page { 1 } else { 2 });
                assert_eq!(
                    children[0],
                    no_story[i]["render_tree"]["root"]["children"][0]
                );
                if i >= host_page {
                    assert_eq!(
                        children[1]["children"][0]["node_type"]["TextRun"]["text"],
                        format!("- {} -", i + 1)
                    );
                }
            }
        }
        let body = &actual[host_page]["render_tree"]["root"]["children"][0];
        let lines = body["children"].as_array().unwrap();
        let find = |owner| {
            lines
                .iter()
                .find(|l| l["node_type"]["TextLine"]["para_index"] == owner)
                .unwrap()
        };
        let blank = find(pi);
        let after = find(pi + 1);
        near(bbox(blank, "height"), 1000.0 / 75.0);
        near(bbox(after, "y") - bbox(blank, "y"), 1500.0 / 75.0);
    }
}

#[test]
fn delayed_number_activation_is_transactional_on_error_and_clone_resume() {
    let mut d = rhwp::parse_document(include_bytes!(
        "../fixtures/issue7353/late-page-number/second-saved.hwp"
    ))
    .unwrap();
    d.sections[0].section_def.page_num = u16::MAX;
    for control in &mut d.sections[0].paragraphs[0].controls {
        if let Control::SectionDef(def) = control {
            def.page_num = u16::MAX;
        }
    }
    let mut session = DocumentV2Session::from_bytes(&encode(&d, true), OPTIONS).unwrap();
    let first: Value = serde_json::from_str(&session.next_page_json().unwrap().unwrap()).unwrap();
    assert_eq!(
        first["render_tree"]["root"]["children"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let mut copy = session.clone();
    for _ in 0..2 {
        for s in [&mut session, &mut copy] {
            assert!(s
                .next_page_json()
                .unwrap_err()
                .to_string()
                .contains("page-number range"));
            assert_eq!(s.emitted_pages(), 1);
        }
    }
}

#[test]
fn intra_paragraph_declaration_is_not_mistaken_for_paragraph_entry() {
    let mut d = doc(None);
    let host = &mut d.sections[0].paragraphs[0];
    host.controls.push(Control::PageNumberPos(number(5)));
    host.char_count += 8;
    // Text offsets still precede pgnp. Its page of activation is not qualified
    // by the paragraph-entry contract, so do not silently apply it earlier.
    let error = DocumentV2Session::from_bytes(&encode(&d, true), OPTIONS)
        .err()
        .expect("intra-paragraph declaration must remain unsupported");
    assert!(error
        .to_string()
        .contains("page-number declaration within paragraph content"));
}

#[test]
fn footer_baselines_follow_independent_hancom_margin_matrix() {
    // PDF text origins from mutool trace, not V2's current formula. These
    // authored HWPX inputs have no manual stored lines; Hancom2020 printed them.
    // One96dpi pixel allows PDF page/device quantization, not an ink-bbox baseline.
    for (name, baseline_pt) in [
        ("b20f0", 565.13372),
        ("b20f1", 536.82906),
        ("b20f2", 536.82906),
        ("b20f5", 536.82906),
        ("b20f10", 536.82906),
        ("b20f15", 536.82906),
        ("b10f5", 565.13372),
        ("b20f0body", 565.13372),
        ("b15f10body", 550.98139),
        ("b20f1hu", 536.82906),
        ("b0f0", 593.43838),
    ] {
        let path = format!(
            "{}/tests/fixtures/issue7353/footer-position/{name}.hwpx",
            env!("CARGO_MANIFEST_DIR")
        );
        let bytes = std::fs::read(path).unwrap();
        let mut s = DocumentV2Session::from_bytes(&bytes, OPTIONS).unwrap();
        let page: Value = serde_json::from_str(&s.next_page_json().unwrap().unwrap()).unwrap();
        let line = &page["render_tree"]["root"]["children"][1];
        let run = &line["children"][0];
        let baseline = bbox(run, "y") + run["node_type"]["TextRun"]["baseline"].as_f64().unwrap();
        assert!(
            (baseline - baseline_pt * 96.0 / 72.0).abs() < 1.0,
            "{name}: baseline {baseline} vs independent PDF {}",
            baseline_pt * 96.0 / 72.0
        );
        assert!(bbox(line, "y") >= 0.0);
        assert!(
            bbox(line, "y") + bbox(line, "height")
                <= page["render_tree"]["root"]["bbox"]["height"]
                    .as_f64()
                    .unwrap()
        );
        assert!(s.next_page_json().unwrap().is_none());
    }
}
