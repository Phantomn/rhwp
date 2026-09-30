//! Original p181 supplies the line partition, object owner and zero reset.
//! Synthetic host frames test ownership/fit, not original page fidelity.
use rhwp::{
    model::{
        control::Control,
        document::{Document, Section},
        page::PageDef,
        shape::ShapeObject,
    },
    renderer::{
        render_tree::{RenderNode, RenderNodeType},
        table_v2::{CellEndPolicy, HostedSectionSession},
    },
};
fn source(height: u32) -> Document {
    let mut d =
        rhwp::parse_document(include_bytes!("../../samples/21_언어_기출_편집가능본.hwp")).unwrap();
    let p = d.sections[0].paragraphs[181].clone();
    let mut s = Section::default();
    s.section_def.page_def = PageDef {
        width: 31888,
        height,
        ..Default::default()
    };
    s.paragraphs = vec![p];
    d.sections = vec![s];
    d
}
fn walk(n: &RenderNode) -> Vec<&RenderNode> {
    std::iter::once(n)
        .chain(n.children.iter().flat_map(walk))
        .collect()
}
fn check(d: &Document, cut: Option<usize>) {
    let s = HostedSectionSession::from_document(d, 0, 96., CellEndPolicy::OmitFinalParagraphGap)
        .unwrap();
    if cut.is_some() {
        assert_eq!(s.pagination().pages.len(), 2);
    }
    let p = &d.sections[0].paragraphs[0];
    let mut text = String::new();
    let mut owners = vec![];
    let mut shapes = 0;
    let mut horizontal = 0;
    for page in 0..s.pagination().pages.len() {
        let tree = s.render_page(page).unwrap();
        let rows: Vec<_> = walk(&tree.root)
            .into_iter()
            .filter(|n| matches!(n.node_type, RenderNodeType::TextLine(_)) && n.bbox.width > 300.)
            .collect();
        assert!(!rows.is_empty(), "no extra blank page");
        let first = owners.len();
        for row in &rows {
            let RenderNodeType::TextLine(line) = &row.node_type else {
                unreachable!()
            };
            let owner = line.line_index.unwrap() as usize;
            owners.push(owner);
            let saved = &p.line_segs[owner];
            assert!((row.bbox.height - f64::from(saved.line_height) / 75.).abs() < 1e-7);
            if owner > first {
                assert!(
                    (row.bbox.y
                        - rows[0].bbox.y
                        - f64::from(saved.vertical_pos - p.line_segs[first].vertical_pos) / 75.)
                        .abs()
                        < 1e-7
                );
            }
            for child in &row.children {
                if let RenderNodeType::TextRun(run) = &child.node_type {
                    text.push_str(&run.text);
                }
            }
            let rects: Vec<_> = walk(row)
                .into_iter()
                .filter(|n| matches!(n.node_type, RenderNodeType::Rectangle(_)))
                .collect();
            if owner == 2 {
                assert_eq!(rects.len(), 1);
                assert!((rects[0].bbox.y - row.bbox.y).abs() < 1e-7);
                assert!((rects[0].bbox.height - 1417. / 75.).abs() < 1e-7);
            } else {
                assert!(rects.is_empty());
            }
            shapes += rects.len();
        }
        if let Some(cut) = cut {
            assert_eq!(first, if page == 0 { 0 } else { cut });
            assert_eq!(rows.len(), if page == 0 { cut } else { 13 - cut });
        }
        for n in walk(&tree.root) {
            if let RenderNodeType::Line(l) = &n.node_type {
                if l.y1 == l.y2 {
                    horizontal += 1;
                }
            }
        }
    }
    assert_eq!(owners, (0..13).collect::<Vec<_>>());
    assert_eq!(text, p.text);
    assert_eq!(shapes, 1);
    assert_eq!(horizontal, 2, "only opening/closing edges across frames");
}
#[test]
fn saved_reset_preserves_object_owner_and_every_line_once() {
    check(&source(92000), Some(8));
}

#[test]
fn saved_reset_uses_next_column_before_next_page() {
    let mut d = source(92000);
    let original =
        rhwp::parse_document(include_bytes!("../../samples/21_언어_기출_편집가능본.hwp")).unwrap();
    let mut prefix = original.sections[0].paragraphs[144].clone();
    prefix
        .controls
        .push(Control::ColumnDef(rhwp::model::page::ColumnDef {
            column_count: 2,
            same_width: true,
            spacing: 1200,
            ..Default::default()
        }));
    d.sections[0].paragraphs.insert(0, prefix);
    d.sections[0].section_def.page_def.width = 2 * 31888 + 1200;
    let s = HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::OmitFinalParagraphGap)
        .unwrap();
    assert_eq!(s.pagination().pages.len(), 1);
    let tree = s.render_page(0).unwrap();
    let rows: Vec<_> = walk(&tree.root)
        .into_iter()
        .filter(|n| matches!(n.node_type, RenderNodeType::TextLine(_)) && n.bbox.width > 300.)
        .collect();
    assert_eq!(rows.len(), 14); // authored blank followed by all 13 source rows
    let one_column = HostedSectionSession::from_document(
        &source(92000),
        0,
        96.,
        CellEndPolicy::OmitFinalParagraphGap,
    )
    .unwrap();
    for (column, actual) in [&rows[1..9], &rows[9..]].into_iter().enumerate() {
        let reference = one_column.render_page(column).unwrap();
        let expected: Vec<_> = walk(&reference.root)
            .into_iter()
            .filter(|n| matches!(n.node_type, RenderNodeType::TextLine(_)) && n.bbox.width > 300.)
            .collect();
        for (a, b) in actual.iter().zip(expected) {
            // Translation invariant: moving an unchanged source story from a
            // page into an equal-width column adds only its physical origin.
            assert!((a.bbox.x - b.bbox.x - column as f64 * (31888. + 1200.) / 75.).abs() < 1e-7);
        }
    }
    assert!(
        rows[9].bbox.y.abs() < 1e-7,
        "continuation starts at next column origin"
    );
    assert_eq!(
        walk(&tree.root)
            .iter()
            .filter(|n| matches!(n.node_type, RenderNodeType::Rectangle(_)))
            .count(),
        1
    );
}
#[test]
fn object_owned_first_line_on_continuation_uses_that_frames_origin() {
    let mut d = source(92000);
    let p = &mut d.sections[0].paragraphs[0];
    let mut y = 0;
    for (i, row) in p.line_segs.iter_mut().enumerate() {
        if i == 2 {
            y = 0;
        }
        row.vertical_pos = y;
        y += row.line_height + row.line_spacing;
    }
    check(&d, Some(2));
}
#[test]
fn physical_budget_before_saved_cut_does_not_duplicate_or_drop_object() {
    check(&source(3000), None);
}
#[test]
fn nonzero_reset_or_floating_object_is_not_admitted() {
    for mode in [0, 1, 2, 3] {
        let mut d = source(92000);
        let p = &mut d.sections[0].paragraphs[0];
        if mode == 0 {
            p.line_segs[8].vertical_pos = 1;
        } else if mode == 3 {
            p.invalidate_layout_inputs();
        } else {
            let Control::Shape(s) = &mut p.controls[0] else {
                panic!()
            };
            let ShapeObject::Rectangle(r) = s.as_mut() else {
                panic!()
            };
            if mode == 1 {
                r.common.treat_as_char = false;
            } else {
                r.common.height += 100;
            }
        }
        assert!(HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()).is_err());
    }
}

#[test]
fn continued_paragraph_keeps_following_authored_blank_and_next_text() {
    let mut d = source(92000);
    let original =
        rhwp::parse_document(include_bytes!("../../samples/21_언어_기출_편집가능본.hwp")).unwrap();
    let blank = original.sections[0].paragraphs[144].clone();
    let after = original.sections[0].paragraphs[5].clone();
    assert!(blank.text.trim().is_empty());
    d.sections[0].paragraphs.extend([blank, after]);
    let s = HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::OmitFinalParagraphGap)
        .unwrap();
    assert_eq!(s.pagination().pages.len(), 2);
    let tree = s.render_page(1).unwrap();
    let rows: Vec<_> = walk(&tree.root)
        .into_iter()
        .filter(|n| matches!(n.node_type, RenderNodeType::TextLine(_)) && n.bbox.width > 300.)
        .collect();
    assert_eq!(rows.len(), 7);
    // Last continued row -> intentional blank -> following text. Interline
    // advances come from independent saved LineSegs, not a computed plan total.
    for (previous, next, saved) in [
        (rows[4], rows[5], &d.sections[0].paragraphs[0].line_segs[12]),
        (rows[5], rows[6], &d.sections[0].paragraphs[1].line_segs[0]),
    ] {
        assert!(
            (next.bbox.y
                - previous.bbox.y
                - f64::from(saved.line_height + saved.line_spacing) / 75.)
                .abs()
                < 1e-7
        );
    }
    let actual: String = rows[6]
        .children
        .iter()
        .filter_map(|n| {
            if let RenderNodeType::TextRun(r) = &n.node_type {
                Some(r.text.as_str())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(actual, d.sections[0].paragraphs[2].text);
}
