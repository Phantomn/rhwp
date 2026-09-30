//! Independent paired Hancom saves/PDFs: issue7353_hide_empty_review.
use rhwp::{
    model::{document::Document, paragraph::ColumnBreakType},
    renderer::{
        render_tree::{RenderNode, RenderNodeType},
        table_v2::{CellEndPolicy, HostedSectionSession},
    },
};

fn document(enabled: bool) -> Document {
    rhwp::parse_document(if enabled {
        include_bytes!("../fixtures/issue7353_hide_empty_review/on-saved.hwp")
    } else {
        include_bytes!("../fixtures/issue7353_hide_empty_review/off-saved.hwp")
    })
    .unwrap()
}
fn session(d: &Document, dpi: f64) -> HostedSectionSession {
    HostedSectionSession::from_document(d, 0, dpi, CellEndPolicy::OmitFinalParagraphGap).unwrap()
}
fn lines(n: &RenderNode, out: &mut Vec<(usize, f64, f64)>) {
    if let RenderNodeType::TextLine(t) = &n.node_type {
        if let Some(pi) = t.para_index {
            out.push((pi, n.bbox.y, n.bbox.height));
        }
    }
    for child in &n.children {
        lines(child, out);
    }
}
fn page(s: &HostedSectionSession, i: usize) -> Vec<(usize, f64, f64)> {
    let mut out = Vec::new();
    lines(&s.render_page(i).unwrap().root, &mut out);
    out
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}

#[test]
fn normal_saved_pair_hides_only_two_overflowing_empty_lines() {
    for dpi in [96., 192.] {
        let on_doc = document(true);
        let before = format!("{:?}", on_doc);
        let on = session(&on_doc, dpi);
        let off = session(&document(false), dpi);
        assert_eq!(on.pagination().pages.len(), 2);
        assert_eq!(off.pagination().pages.len(), 2);
        assert_eq!(
            on.pagination().hidden_empty_paras,
            [1, 2].into_iter().collect()
        );
        assert!(off.pagination().hidden_empty_paras.is_empty());
        assert_eq!(page(&on, 0), page(&off, 0));
        assert_eq!(page(&on, 0).len(), 7);
        assert_eq!(
            page(&on, 1).iter().map(|x| x.0).collect::<Vec<_>>(),
            [3, 4, 5]
        );
        assert_eq!(
            page(&off, 1).iter().map(|x| x.0).collect::<Vec<_>>(),
            [1, 2, 3, 4, 5]
        );
        // Saved Hancom LineSeg: third empty at0, AFTER at1800; OFF AFTER5400.
        // PDF AFTER ink top: ON97.708934pt, OFF133.622534pt (print scale .9976).
        near(page(&on, 1)[1].1, 9800. * dpi / 7200.);
        near(page(&off, 1)[3].1, 13400. * dpi / 7200.);
        near(page(&on, 1)[0].2, 1200. * dpi / 7200.);
        assert_eq!(
            on.render_page_json(1).unwrap(),
            on.render_page_json(1).unwrap()
        );
        assert_eq!(before, format!("{:?}", on_doc));
    }
}

#[test]
fn split_table_owner_and_cell_empty_lines_are_unchanged_by_body_option() {
    let mut d = rhwp::parse_document(include_bytes!(
        "../fixtures/issue7353_host_owner_review/split-saved.hwp"
    ))
    .unwrap();
    d.sections[0].section_def.hide_empty_line = false;
    let off = session(&d, 96.);
    d.sections[0].section_def.hide_empty_line = true;
    let on = session(&d, 96.);
    assert_eq!(on.pagination().pages.len(), off.pagination().pages.len());
    assert!(on.pagination().hidden_empty_paras.is_empty());
    for i in 0..on.pagination().pages.len() {
        assert_eq!(
            on.render_page_json(i).unwrap(),
            off.render_page_json(i).unwrap()
        );
    }
}

#[test]
fn exact_line_fit_preserves_a_blank_even_when_its_following_gap_overflows() {
    let mut d = document(true);
    // Source row ends12000HU, advances12600HU. A1200HU blank fits exactly
    // in13800HU; its600HU trailing gap is not a second line requirement.
    d.sections[0].section_def.page_def.margin_bottom = 18200;
    let s = session(&d, 96.);
    assert_eq!(page(&s, 0).last().unwrap().0, 1);
    assert_eq!(page(&s, 1).iter().map(|x| x.0).collect::<Vec<_>>(), [4, 5]);
    near(page(&s, 0).last().unwrap().1, 20600. / 75.);
    near(page(&s, 1)[0].1, 8000. / 75.);
}

#[test]
fn each_column_has_its_own_limit_and_the_next_page_keeps_the_third_line() {
    for dpi in [96., 192.] {
        let on = rhwp::parse_document(include_bytes!(
            "../fixtures/issue7353_hide_empty_review/columns-on-saved.hwp"
        ))
        .unwrap();
        let off = rhwp::parse_document(include_bytes!(
            "../fixtures/issue7353_hide_empty_review/columns-off-saved.hwp"
        ))
        .unwrap();
        let on = session(&on, dpi);
        let off = session(&off, dpi);
        assert_eq!(on.pagination().pages.len(), 2);
        assert_eq!(off.pagination().pages.len(), 2);
        let first = page(&on, 0);
        assert_eq!(first.iter().filter(|x| x.0 == 3).count(), 1);
        assert!(!first.iter().any(|x| [1, 2, 6, 7].contains(&x.0)));
        assert_eq!(page(&on, 1).iter().map(|x| x.0).collect::<Vec<_>>(), [8, 9]);
        near(
            first.iter().find(|x| x.0 == 4).unwrap().1,
            9800. * dpi / 7200.,
        );
        near(page(&on, 1)[1].1, 9800. * dpi / 7200.);
        near(
            page(&off, 0).iter().find(|x| x.0 == 4).unwrap().1,
            13400. * dpi / 7200.,
        );
        near(
            page(&off, 1).iter().find(|x| x.0 == 9).unwrap().1,
            17000. * dpi / 7200.,
        );
    }
}

#[test]
fn invisible_control_owner_is_not_a_suppressible_empty_line() {
    let mut d = document(true);
    let p = &mut d.sections[0].paragraphs[1];
    p.char_count = 9;
    p.controls
        .push(rhwp::model::control::Control::PageNumberPos(
            rhwp::model::control::PageNumberPos {
                format: 0,
                position: 5,
                user_symbol: '\0',
                prefix_char: '\0',
                suffix_char: '\0',
                dash_char: '-',
            },
        ));
    let s = session(&d, 96.);
    let out = page(&s, 1);
    assert_eq!(out.iter().filter(|x| (1..=3).contains(&x.0)).count(), 3);
    near(out.iter().find(|x| x.0 == 4).unwrap().1, 13400. / 75.);
    assert!(s
        .render_page_json(1)
        .unwrap()
        .contains("\"text\":\"- 2 -\""));
}

#[test]
fn fresh_composition_uses_the_same_physical_fit_rule() {
    // Edited/synthetic path, not a second Hancom fidelity claim.
    let mut d = document(true);
    for p in &mut d.sections[0].paragraphs {
        p.line_segs.clear();
    }
    let s = session(&d, 96.);
    assert_eq!(s.pagination().pages.len(), 2);
    assert_eq!(
        page(&s, 1).iter().map(|x| x.0).collect::<Vec<_>>(),
        [3, 4, 5]
    );
    near(page(&s, 1)[1].1, 9800. / 75.);
}

#[test]
fn empty_lines_inside_body_and_after_explicit_page_break_are_not_hidden() {
    for explicit in [false, true] {
        let mut d = document(true);
        if explicit {
            d.sections[0].paragraphs[1].column_type = ColumnBreakType::Page;
        } else {
            d.sections[0].section_def.page_def.margin_bottom = 1000;
        }
        let s = session(&d, 96.);
        let out = page(&s, usize::from(explicit));
        let blanks: Vec<_> = out.iter().filter(|x| (1..=3).contains(&x.0)).collect();
        assert_eq!(blanks.len(), 3);
        near(blanks[1].1 - blanks[0].1, 24.);
        near(blanks[2].1 - blanks[1].1, 24.);
    }
}
