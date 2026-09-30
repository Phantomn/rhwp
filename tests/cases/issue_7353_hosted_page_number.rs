//! Existing normal Hancom HWP/PDF timeline, now through the common section
//! host. PDF geometry and provenance: issue7353/page-number-timeline/README.md.
use rhwp::{
    model::{control::Control, document::Document, paragraph::ColumnBreakType},
    renderer::{
        render_tree::{RenderNode, RenderNodeType},
        table_v2::{CellEndPolicy, HostedSectionSession},
    },
};

const INPUT: &[u8] =
    include_bytes!("../fixtures/issue7353/page-number-timeline/timeline-saved.hwp");
fn document() -> Document {
    rhwp::parse_document(INPUT).unwrap()
}
fn session(d: &Document) -> HostedSectionSession {
    HostedSectionSession::from_document(d, 0, 96., CellEndPolicy::OmitFinalParagraphGap).unwrap()
}
fn text(n: &RenderNode) -> String {
    let mut out = match &n.node_type {
        RenderNodeType::TextRun(t) => t.text.clone(),
        _ => String::new(),
    };
    for c in &n.children {
        out.push_str(&text(c));
    }
    out
}
fn footer(s: &HostedSectionSession, page: usize) -> Option<RenderNode> {
    s.render_page(page)
        .unwrap()
        .root
        .children
        .into_iter()
        .find(|n| matches!(n.node_type, RenderNodeType::TextLine(_)))
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}

#[test]
fn saved_timeline_uses_accepted_page_and_preserves_body_and_source() {
    let d = document();
    let source = format!("{:?}", d);
    let s = session(&d);
    assert_eq!(s.pagination().pages.len(), 6);
    let expected = [
        Some("- 1 -"),
        Some("- 2 -"),
        Some("- 3 -"),
        Some("- 4 -"),
        None,
        Some("- 6 -"),
    ];
    for (i, expected) in expected.into_iter().enumerate() {
        let f = footer(&s, i);
        assert_eq!(f.as_ref().map(text).as_deref(), expected);
        assert_eq!(s.pagination().pages[i].page_number, i as u32 + 1);
        if let Some(f) = f {
            let (actual, pdf) = match i {
                0 => (f.bbox.x + f.bbox.width / 2., (193.319663 + 225.696092) / 2.),
                1 | 2 => (f.bbox.x, 39.670681),
                _ => (f.bbox.x + f.bbox.width, 379.345074),
            };
            assert!((actual - pdf * 4. / 3.).abs() < 1.);
            let RenderNodeType::TextLine(line) = f.node_type else {
                unreachable!()
            };
            assert!((f.bbox.y + line.baseline - 565.13372 * 4. / 3.).abs() < 1.);
        }
    }
    assert!(text(&s.render_page(3).unwrap().root).contains("FOUR center firstFOUR right last"));
    let mut disabled = d.clone();
    for p in &mut disabled.sections[0].paragraphs {
        for c in &mut p.controls {
            if let Control::PageNumberPos(n) = c {
                n.position = 0;
            }
        }
    }
    let off = session(&disabled);
    assert_eq!(off.pagination().pages.len(), 6);
    for i in 0..6 {
        assert!(footer(&off, i).is_none());
        let body = |s: &HostedSectionSession| {
            s.render_page(i)
                .unwrap()
                .root
                .children
                .into_iter()
                .filter(|n| matches!(n.node_type, RenderNodeType::Body { .. }))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            serde_json::to_value(body(&s)).unwrap(),
            serde_json::to_value(body(&off)).unwrap()
        );
        assert_eq!(
            s.render_page_json(i).unwrap(),
            s.render_page_json(i).unwrap()
        );
    }
    assert_eq!(format!("{:?}", d), source);
}

#[test]
fn declaration_that_does_not_fit_waits_for_its_real_line() {
    // Synthetic budget only; line1000HU + gap600 leaves500HU of2100HU.
    let mut d = document();
    d.sections[0].paragraphs.truncate(2);
    d.sections[0].paragraphs[1].column_type = ColumnBreakType::None;
    let p = &mut d.sections[0].section_def.page_def;
    p.height = p.margin_top + p.margin_bottom + p.margin_header + p.margin_footer + 2100;
    let s = session(&d);
    assert_eq!(s.pagination().pages.len(), 2);
    let a = footer(&s, 0).unwrap();
    let b = footer(&s, 1).unwrap();
    assert_eq!(text(&a), "- 1 -");
    assert_eq!(text(&b), "- 2 -");
    let body = s.pagination().pages[0].layout.body_area;
    near(a.bbox.x + a.bbox.width / 2., body.x + body.width / 2.);
    near(b.bbox.x, body.x);
}

#[test]
fn late_declaration_does_not_retroactively_paint_earlier_page() {
    // Keep every source slot: disable first declaration, retain second left.
    let mut d = document();
    d.sections[0].paragraphs.truncate(2);
    for c in &mut d.sections[0].paragraphs[0].controls {
        if let Control::PageNumberPos(n) = c {
            n.position = 0;
        }
    }
    let s = session(&d);
    assert_eq!(s.pagination().pages.len(), 2);
    assert!(footer(&s, 0).is_none());
    assert_eq!(text(&footer(&s, 1).unwrap()), "- 2 -");
}

#[test]
fn unsupported_format_or_mid_paragraph_declaration_remains_explicit() {
    for kind in 0..2 {
        let mut d = document();
        let p = &mut d.sections[0].paragraphs[1];
        if kind == 0 {
            p.controls.push(p.controls[0].clone());
            p.char_count += 8;
            for offset in &mut p.char_offsets {
                *offset += 8;
            }
        } else {
            let Control::PageNumberPos(n) = &mut p.controls[0] else {
                unreachable!()
            };
            n.format = 1;
        }
        assert!(HostedSectionSession::from_document(
            &d,
            0,
            96.,
            CellEndPolicy::OmitFinalParagraphGap
        )
        .is_err());
    }
}

#[test]
fn invisible_owner_is_a_real_line_and_activates_the_story() {
    // Synthetic empty text variant; retain the1000HU/600HU stored line, not
    // a claim that this edited input is a normal Hancom-generated fixture.
    let mut d = document();
    d.sections[0].paragraphs.truncate(3);
    let owner = &mut d.sections[0].paragraphs[1];
    owner.text.clear();
    owner.char_count = 9; //8-unit control plus paragraph end
    owner.char_offsets.clear();
    d.sections[0].paragraphs[2].column_type = ColumnBreakType::None;
    d.sections[0].paragraphs[2].line_segs[0].vertical_pos = 1600;
    let s = session(&d);
    assert_eq!(s.pagination().pages.len(), 2);
    assert_eq!(text(&footer(&s, 1).unwrap()), "- 2 -");
    fn collect<'a>(n: &'a RenderNode, out: &mut Vec<&'a RenderNode>) {
        if matches!(n.node_type, RenderNodeType::TextLine(_)) {
            out.push(n);
        }
        for c in &n.children {
            collect(c, out);
        }
    }
    let tree = s.render_page(1).unwrap();
    let mut lines = Vec::new();
    collect(&tree.root, &mut lines);
    let by_para = |pi| {
        *lines
            .iter()
            .find(|n| {
                matches!(&n.node_type,
        RenderNodeType::TextLine(l) if l.para_index == Some(pi))
            })
            .unwrap()
    };
    let blank = by_para(1);
    let following = by_para(2);
    assert_eq!(text(blank), "");
    near(blank.bbox.height, 1000. / 75.);
    near(following.bbox.y - blank.bbox.y, 1600. / 75.);
}
