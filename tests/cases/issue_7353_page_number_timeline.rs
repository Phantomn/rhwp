//! Repeated paragraph-entry declarations, independently qualified by Hancom
//! saved HWP/PDF. Synthetic mutations below test fit/transaction boundaries,
//! not visual equivalence to a hand-authored saved-row cache.
use rhwp::{
    model::{control::Control, document::Document, paragraph::ColumnBreakType},
    renderer::table_v2::{DocumentV2Error, DocumentV2Session},
};
use serde_json::Value;
const TIMELINE: &[u8] =
    include_bytes!("../fixtures/issue7353/page-number-timeline/timeline-saved.hwp");
const PREFIX: &[u8] =
    include_bytes!("../fixtures/issue7353/page-number-timeline/prefix133-saved.hwp");
const OPTIONS: &str = r#"{"dpi":96,"max_pages":100,"cell_end_policy":"omit_final_paragraph_gap"}"#;
fn encoded(d: &Document) -> Vec<u8> {
    rhwp::serializer::cfb_writer::serialize_hwp(d).unwrap()
}
fn render(input: &[u8]) -> Vec<Value> {
    let mut s = DocumentV2Session::from_bytes(input, OPTIONS).unwrap();
    let mut out = vec![];
    while let Some(p) = s.next_page_json().unwrap() {
        out.push(serde_json::from_str(&p).unwrap());
    }
    out
}
fn children(p: &Value) -> &[Value] {
    p["render_tree"]["root"]["children"].as_array().unwrap()
}
fn text(n: &Value) -> String {
    let mut s = n["node_type"]["TextRun"]["text"]
        .as_str()
        .unwrap_or("")
        .to_owned();
    for c in n["children"].as_array().unwrap() {
        s.push_str(&text(c));
    }
    s
}
fn number(p: &Value) -> Option<&Value> {
    assert!(children(p).len() <= 2);
    children(p).get(1)
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}
fn n(v: &Value, k: &str) -> f64 {
    v["bbox"][k].as_f64().unwrap()
}
fn set_position(d: &mut Document, pi: usize, position: u8) {
    let c = d.sections[0].paragraphs[pi]
        .controls
        .iter_mut()
        .find(|c| matches!(c, Control::PageNumberPos(_)))
        .unwrap();
    let Control::PageNumberPos(c) = c else {
        unreachable!()
    };
    c.position = position;
}
#[test]
fn normal_saved_timeline_changes_position_without_resetting_number_or_body() {
    let pages = render(TIMELINE);
    assert_eq!(pages.len(), 6); // independent Hancom PDF:6 physical pages
    let expected = [
        Some("- 1 -"),
        Some("- 2 -"),
        Some("- 3 -"),
        Some("- 4 -"),
        None,
        Some("- 6 -"),
    ];
    for (i, p) in pages.iter().enumerate() {
        assert_eq!(number(p).map(text).as_deref(), expected[i]);
        if let Some(footer) = number(p) {
            let b = &children(p)[0];
            // Independent PDF text line extents (pt) and traced baseline.
            // Compare aligned edge/center, not substituted font glyph widths.
            let (actual, pdf) = match i {
                0 => (
                    n(footer, "x") + n(footer, "width") / 2.,
                    (193.319663 + 225.696092) / 2.,
                ),
                1 | 2 => (n(footer, "x"), 39.670681),
                _ => (n(footer, "x") + n(footer, "width"), 379.345074),
            };
            assert!((actual - pdf * 4. / 3.).abs() < 1.);
            let baseline = footer["node_type"]["TextLine"]["baseline"]
                .as_f64()
                .unwrap();
            assert!((n(footer, "y") + baseline - 565.13372 * 4. / 3.).abs() < 1.);
            match i {
                0 => near(
                    n(footer, "x") + n(footer, "width") / 2.,
                    n(b, "x") + n(b, "width") / 2.,
                ),
                1 | 2 => near(n(footer, "x"), n(b, "x")),
                _ => near(
                    n(footer, "x") + n(footer, "width"),
                    n(b, "x") + n(b, "width"),
                ),
            }
        }
    }
    assert!(text(&children(&pages[3])[0]).contains("FOUR center firstFOUR right last"));
    let mut d = rhwp::parse_document(TIMELINE).unwrap();
    for i in [0, 1, 3, 4, 5, 6] {
        set_position(&mut d, i, 0);
    }
    let disabled = render(&encoded(&d));
    assert_eq!(disabled.len(), pages.len());
    for (a, b) in pages.iter().zip(&disabled) {
        assert!(number(b).is_none());
        assert_eq!(children(a)[0], children(b)[0]);
    }
}

#[test]
fn unqualified_mid_paragraph_or_nondecimal_declarations_still_fail() {
    let original = rhwp::parse_document(TIMELINE).unwrap();
    for kind in 0..2 {
        let mut d = original.clone();
        let p = &mut d.sections[0].paragraphs[1];
        if kind == 0 {
            p.controls.push(p.controls[0].clone());
            p.char_count += 8;
            p.char_offsets.iter_mut().for_each(|v| *v += 8);
        } else {
            let Control::PageNumberPos(c) = &mut p.controls[0] else {
                unreachable!()
            };
            c.format = 1;
        }
        let err = DocumentV2Session::from_bytes(&encoded(&d), OPTIONS)
            .err()
            .unwrap();
        let expected = if kind == 0 {
            "page-number declaration within paragraph content"
        } else {
            "page-number format or position"
        };
        assert!(
            matches!(err, DocumentV2Error::Paragraph { index: 1,
            reason: rhwp::renderer::table_v2::GeometryError::Unsupported(reason)
        } if reason == expected),
            "{err}"
        );
    }
}
#[test]
fn repeated_declaration_keeps_real_empty_host_line_and_following_page_break() {
    let pages = render(PREFIX);
    assert_eq!(pages.len(), 23);
    assert_eq!(text(number(&pages[21]).unwrap()), "- 22 -");
    assert_eq!(text(number(&pages[22]).unwrap()), "- 23 -");
    let lines = children(&pages[21])[0]["children"].as_array().unwrap();
    let owned = |pi| {
        lines
            .iter()
            .filter(move |n| n["node_type"]["TextLine"]["para_index"].as_u64() == Some(pi))
    };
    let blank: Vec<_> = owned(129).collect();
    assert_eq!(blank.len(), 1);
    assert_eq!(text(blank[0]), "");
    near(n(blank[0], "height"), 1500. / 75.); // source stored line, not inferred from paint
    let last = owned(128).next_back().unwrap();
    near(n(blank[0], "y") - n(last, "y"), (1500. + 752.) / 75.);
    let first = &children(&pages[22])[0]["children"][0];
    assert!(text(first).contains("Ⅳ. 추진계획 및 종합결론"));
    near(n(first, "y"), n(&children(&pages[22])[0], "y"));
    let mut d = rhwp::parse_document(PREFIX).unwrap();
    set_position(&mut d, 129, 0);
    let disabled = render(&encoded(&d));
    assert_eq!(disabled.len(), pages.len());
    assert!(number(&disabled[21]).is_none());
    assert!(number(&disabled[22]).is_none());
    for (a, b) in pages.iter().zip(&disabled) {
        assert_eq!(children(a)[0], children(b)[0]);
    }
}
#[test]
fn declaration_waiting_for_space_does_not_change_previous_page() {
    let mut d = rhwp::parse_document(TIMELINE).unwrap();
    d.sections[0].paragraphs.truncate(2);
    d.sections[0].paragraphs[1].column_type = ColumnBreakType::None;
    // Synthetic fit boundary:1000HU line +600 gap leaves only500HU; the
    // next1000HU line must move, retaining center onpage1 and left onpage2.
    let page = &mut d.sections[0].section_def.page_def;
    page.height =
        page.margin_top + page.margin_bottom + page.margin_header + page.margin_footer + 2100;
    let page = page.clone();
    for p in &mut d.sections[0].paragraphs {
        for c in &mut p.controls {
            if let Control::SectionDef(s) = c {
                s.page_def = page.clone();
            }
        }
    }
    let pages = render(&encoded(&d));
    assert_eq!(pages.len(), 2);
    let a = number(&pages[0]).unwrap();
    let b = number(&pages[1]).unwrap();
    assert_eq!(text(a), "- 1 -");
    assert_eq!(text(b), "- 2 -");
    near(
        n(a, "x") + n(a, "width") / 2.,
        n(&children(&pages[0])[0], "x") + n(&children(&pages[0])[0], "width") / 2.,
    );
    near(n(b, "x"), n(&children(&pages[1])[0], "x"));
}
#[test]
fn failed_story_paint_does_not_commit_cursor_or_number_state() {
    let mut d = rhwp::parse_document(TIMELINE).unwrap();
    d.sections[0].section_def.page_num = u16::MAX;
    for c in &mut d.sections[0].paragraphs[0].controls {
        if let Control::SectionDef(s) = c {
            s.page_num = u16::MAX;
        }
    }
    let mut s = DocumentV2Session::from_bytes(&encoded(&d), OPTIONS).unwrap();
    assert!(s.next_page_json().unwrap().is_some());
    for _ in 0..2 {
        assert!(matches!(
            s.next_page_json(),
            Err(DocumentV2Error::Geometry(
                rhwp::renderer::table_v2::GeometryError::Unsupported("page-number range")
            ))
        ));
        assert_eq!(s.emitted_pages(), 1);
    }
}
