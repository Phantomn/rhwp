//! Host transport contracts, not proof of full DocumentCore V2 integration.
//! Synthetic IR:9pt text=12px, fixed line advance18px, cell padding3/4px.
//! Existing page geometry gives two212px columns separated by16px. Expectations
//! below follow these inputs, not a height copied from the implementation.
use rhwp::{
    model::{
        control::Control,
        document::{Document, Section},
        page::{ColumnDef, ColumnDirection, PageDef},
        paragraph::{CharShapeRef, Paragraph},
        style::{CharShape, LineSpacingType, ParaShape},
        table::{Cell, Table, TablePageBreak},
        Padding,
    },
    renderer::{
        page_layout::PageLayoutInfo,
        render_tree::{PageRenderTree, RenderNode, RenderNodeType},
        table_v2::*,
    },
};

const SELECT: TableSelection = TableSelection {
    section: 0,
    paragraph: 0,
    control: 0,
};
fn document() -> Document {
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
    let t = Table {
        row_count: 1,
        col_count: 1,
        page_break: TablePageBreak::CellBreak,
        padding: Padding {
            left: 375,
            right: 525,
            top: 225,
            bottom: 300,
        },
        cells: vec![Cell {
            width: 15900,
            row_span: 1,
            col_span: 1,
            paragraphs: ["alpha", "beta", "gamma"]
                .iter()
                .map(|s| Paragraph {
                    text: (*s).into(),
                    char_count: s.len() as u32,
                    char_offsets: (0..s.len() as u32).collect(),
                    char_shapes: vec![CharShapeRef {
                        start_pos: 0,
                        char_shape_id: 0,
                    }],
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }],
        ..Default::default()
    };
    d.sections = vec![Section {
        paragraphs: vec![Paragraph {
            controls: vec![Control::Table(Box::new(t))],
            ..Default::default()
        }],
        ..Default::default()
    }];
    d
}
fn layout(direction: ColumnDirection, number: u32) -> PageLayoutInfo {
    PageLayoutInfo::from_page_def_for_page(
        &PageDef {
            width: 36000,
            height: 6150,
            margin_left: 1500,
            margin_right: 1500,
            margin_top: 2250,
            margin_bottom: 2250,
            ..Default::default()
        },
        &ColumnDef {
            column_count: 2,
            spacing: 1200,
            direction,
            ..Default::default()
        },
        96.,
        number,
    )
}
fn frame(
    l: &PageLayoutInfo,
    page: u32,
    column: Option<usize>,
    y: f64,
    height: f64,
) -> TableHostFrame {
    let lane = column.map(|i| l.column_areas[i]).unwrap_or(l.body_area);
    TableHostFrame::new(
        l,
        TableHostAddress {
            section: 0,
            page,
            column,
            revision: 0,
        },
        Rect {
            x: lane.x,
            y,
            width: lane.width,
            height,
        },
    )
    .unwrap()
}
fn session(d: &Document) -> HostedTableSession {
    HostedTableSession::from_document(d, SELECT, 96., CellEndPolicy::default()).unwrap()
}
fn placed(s: &HostedTableSession, f: TableHostFrame) -> HostedTableProposal {
    match s.query(f).unwrap() {
        HostedTableFit::Placed(p) => *p,
        _ => panic!("expected placement"),
    }
}
fn lines(n: &RenderNode) -> Vec<&RenderNode> {
    let mut r = vec![];
    if matches!(n.node_type, RenderNodeType::TextLine(_)) {
        r.push(n);
    }
    for c in &n.children {
        r.extend(lines(c));
    }
    r
}
fn text(n: &RenderNode) -> String {
    let mut s = String::new();
    if let RenderNodeType::TextRun(t) = &n.node_type {
        s.push_str(&t.text);
    }
    for c in &n.children {
        s.push_str(&text(c));
    }
    s
}
fn paint(packet: &HostedTableFragment) -> PageRenderTree {
    let mut tree = PageRenderTree::new(packet.frame().address().page, 480., 82.);
    packet.append_to(&mut tree).unwrap();
    tree
}

#[test]
fn host_reservations_cross_columns_then_pages_without_legacy_cuts() {
    let mut s = session(&document());
    let l = layout(ColumnDirection::LeftToRight, 1);
    // Host has prior content in column0. V2 may neither reset this origin nor
    // choose page1 instead of the host's remaining column1.
    for (page, col, y, h, word, line_y) in [
        (0, 0, 31., 21., "alpha", 34.),
        (0, 1, 30., 18., "beta", 30.),
        (1, 0, 30., 22., "gamma", 30.),
    ] {
        let f = frame(&l, page, Some(col), y, if y == 31. { 21. } else { 22. });
        let query = placed(&s, f);
        assert_eq!(
            query.occupied(),
            Rect {
                x: if col == 0 { 20. } else { 248. },
                y,
                width: 212.,
                height: h
            }
        );
        let packet = s.commit(query, f).unwrap();
        assert_eq!(packet.selection(), SELECT);
        assert_eq!(packet.occupied().y + packet.occupied().height, y + h);
        let tree = paint(&packet);
        let rendered = &tree.root.children[0];
        assert_eq!(rendered.bbox.y, y);
        assert_eq!(rendered.bbox.height, h);
        assert_eq!(text(rendered), word);
        assert_eq!(lines(rendered).len(), 1);
        assert_eq!(lines(rendered)[0].bbox.y, line_y);
        // Paint can be repeated independently of the pagination cursor.
        assert_eq!(
            serde_json::to_value(&tree).unwrap(),
            serde_json::to_value(paint(&packet)).unwrap()
        );
    }
    assert!(s.is_complete());
    assert!(matches!(
        s.query(frame(&l, 1, Some(0), 52., 0.)).unwrap(),
        HostedTableFit::Complete
    ));
}

#[test]
fn non_fit_and_abandoned_queries_do_not_consume_or_select_a_new_page() {
    let mut s = session(&document());
    let l = layout(ColumnDirection::LeftToRight, 1);
    let small = frame(&l, 0, Some(0), 38., 14.);
    for _ in 0..2 {
        match s.query(small).unwrap() {
            HostedTableFit::DoesNotFit {
                required_height, ..
            } => assert_eq!(required_height, 15.),
            _ => panic!("a12px occupied line plus3px top cannot fit14px"),
        }
    }
    let f = frame(&l, 0, Some(1), 30., 22.);
    drop(placed(&s, f));
    let query = placed(&s, f);
    assert_eq!(text(&paint(&s.commit(query, f).unwrap()).root), "alpha");
}

#[test]
fn stale_host_session_and_continuation_proposals_cannot_be_committed() {
    let mut s = session(&document());
    let mut other = session(&document());
    let l = layout(ColumnDirection::LeftToRight, 1);
    let f = frame(&l, 0, Some(0), 30., 22.);
    assert!(matches!(
        other.commit(placed(&s, f), f),
        Err(HostedTableError::StaleProposal)
    ));
    let changed = TableHostFrame::new(
        &l,
        TableHostAddress {
            revision: 1,
            ..f.address()
        },
        f.available(),
    )
    .unwrap();
    assert!(matches!(
        s.commit(placed(&s, f), changed),
        Err(HostedTableError::StaleProposal)
    ));
    let stale = placed(&s, f);
    let query = placed(&s, f);
    let first = s.commit(query, f).unwrap();
    assert!(matches!(
        s.commit(stale, f),
        Err(HostedTableError::StaleProposal)
    ));
    assert_eq!(text(&paint(&first).root), "alpha");
    assert!(!s.is_complete());
}

#[test]
fn mirror_order_and_body_wide_frames_use_existing_page_layout() {
    for (direction, number, x) in [
        (ColumnDirection::LeftToRight, 1, 20.),
        (ColumnDirection::RightToLeft, 1, 248.),
        (ColumnDirection::Mirror, 2, 248.),
    ] {
        let l = layout(direction, number);
        let s = session(&document());
        let f = frame(&l, number - 1, Some(0), 30., 22.);
        assert_eq!(placed(&s, f).occupied().x, x);
        let body = frame(&l, number - 1, None, 30., 22.);
        assert_eq!(placed(&s, body).occupied().x, 20.);
        assert_eq!(body.available().width, 440.);
    }
}

#[test]
fn invalid_frames_and_wrong_destination_fail_without_repositioning() {
    let l = layout(ColumnDirection::LeftToRight, 1);
    let f = frame(&l, 0, Some(0), 30., 22.);
    for bad in [
        Rect {
            y: 29.,
            ..f.available()
        },
        Rect {
            height: 121.,
            ..f.available()
        },
        Rect {
            width: f64::NAN,
            ..f.available()
        },
    ] {
        assert!(TableHostFrame::new(&l, f.address(), bad).is_err());
    }
    assert!(TableHostFrame::new(
        &l,
        TableHostAddress {
            column: Some(2),
            ..f.address()
        },
        f.available()
    )
    .is_err());
    let mut s = session(&document());
    let query = placed(&s, f);
    let p = s.commit(query, f).unwrap();
    let mut wrong = PageRenderTree::new(1, 480., 82.);
    let before = serde_json::to_value(&wrong).unwrap();
    assert!(matches!(
        p.append_to(&mut wrong),
        Err(HostedTableError::WrongDestination)
    ));
    assert_eq!(before, serde_json::to_value(&wrong).unwrap());
    assert_eq!(text(&paint(&p).root), "alpha");
}

#[test]
fn section_and_dpi_are_not_silently_rebound() {
    let l = layout(ColumnDirection::LeftToRight, 1);
    let f = frame(&l, 0, Some(0), 30., 22.);
    let s = session(&document());
    let section = TableHostFrame::new(
        &l,
        TableHostAddress {
            section: 1,
            ..f.address()
        },
        f.available(),
    )
    .unwrap();
    assert!(matches!(
        s.query(section),
        Err(HostedTableError::DifferentSectionOrDpi)
    ));
    let mut other_dpi = l.clone();
    other_dpi.dpi = 192.;
    let f = TableHostFrame::new(&other_dpi, f.address(), f.available()).unwrap();
    assert!(matches!(
        s.query(f),
        Err(HostedTableError::DifferentSectionOrDpi)
    ));
}

#[test]
fn edited_source_requires_a_new_session_and_narrower_host_does_not_reflow_silently() {
    let mut d = document();
    let mut original = session(&d);
    let Control::Table(table) = &mut d.sections[0].paragraphs[0].controls[0] else {
        unreachable!()
    };
    table.cells[0].paragraphs[0].text = "omega".into();
    let mut edited = session(&d);
    let l = layout(ColumnDirection::LeftToRight, 1);
    let f = frame(&l, 0, Some(0), 30., 22.);
    let narrow = TableHostFrame::new(
        &l,
        f.address(),
        Rect {
            width: 211.,
            ..f.available()
        },
    )
    .unwrap();
    assert!(matches!(
        original.query(narrow).unwrap(),
        HostedTableFit::DoesNotFit {
            required_width: 212.,
            ..
        }
    ));
    for (session, expected) in [(&mut original, "alpha"), (&mut edited, "omega")] {
        let query = placed(session, f);
        let packet = session.commit(query, f).unwrap();
        assert_eq!(text(&paint(&packet).root), expected);
    }
}

#[test]
fn real_saved_table_transport_keeps_existing_preview_tree_unchanged() {
    let bytes = include_bytes!("../fixtures/issue7353_color_border_review/saved.hwp");
    let d = rhwp::parse_document(bytes).unwrap();
    let ci = d.sections[0].paragraphs[0]
        .controls
        .iter()
        .position(|c| matches!(c, Control::Table(_)))
        .unwrap();
    let selection = TableSelection {
        control: ci,
        ..SELECT
    };
    let l = PageLayoutInfo::from_page_def(
        &d.sections[0].section_def.page_def,
        &ColumnDef::default(),
        96.,
    );
    let a = l.body_area;
    let body = Rect {
        x: a.x,
        y: a.y,
        width: a.width,
        height: a.height,
    };
    let f = TableHostFrame::new(
        &l,
        TableHostAddress {
            section: 0,
            page: 0,
            column: None,
            revision: 0,
        },
        body,
    )
    .unwrap();
    let mut host =
        HostedTableSession::from_document(&d, selection, 96., CellEndPolicy::OmitFinalParagraphGap)
            .unwrap();
    let mut preview = TablePreviewSession::from_document_with_end_policy(
        &d,
        selection,
        96.,
        TablePreviewPages {
            width: l.page_width,
            height: l.page_height,
            body,
            first_y: a.y,
        },
        10,
        CellEndPolicy::OmitFinalParagraphGap,
    )
    .unwrap();
    let query = placed(&host, f);
    let packet = host.commit(query, f).unwrap();
    let mut tree = PageRenderTree::new(0, l.page_width, l.page_height);
    packet.append_to(&mut tree).unwrap();
    let expected = preview.next_page().unwrap().unwrap();
    assert_eq!(
        serde_json::to_value(&tree).unwrap(),
        serde_json::to_value(expected.tree).unwrap()
    );
    assert!(preview.next_page().unwrap().is_none());
    assert!(host.is_complete());
}

#[test]
fn original_7008_centered_tac_and_bottom_aligned_shape_fit_without_fallback() {
    let d =
        rhwp::parse_document(include_bytes!("../../samples/21_언어_기출_편집가능본.hwp")).unwrap();
    let mut session = HostedTableSession::from_document(
        &d,
        TableSelection {
            control: 2,
            ..SELECT
        },
        96.,
        CellEndPolicy::OmitFinalParagraphGap,
    )
    .unwrap();
    // This caller-owned body frame tests transport, NOT the source Paper
    // anchor or full-document admission. No source property is rewritten.
    let layout = PageLayoutInfo::from_page_def(
        &d.sections[0].section_def.page_def,
        &ColumnDef::default(),
        96.,
    );
    let height = 13740. * (96. / 7200.);
    let short = frame(&layout, 0, None, layout.body_area.y, height - 1. / 75.);
    assert!(matches!(
        session.query(short).unwrap(),
        HostedTableFit::DoesNotFit { .. }
    ));
    let f = frame(&layout, 0, None, layout.body_area.y, height);
    let proposal = placed(&session, f);
    assert!((proposal.occupied().height - height).abs() < 1e-10);
    let packet = session.commit(proposal, f).unwrap();
    let mut tree = PageRenderTree::new(0, layout.page_width, layout.page_height);
    packet.append_to(&mut tree).unwrap();
    let painted = &tree.root.children[0];
    assert_eq!(painted.bbox.y, f.available().y);
    assert!((painted.bbox.height - height).abs() < 1e-10);
    let all = text(painted);
    assert!(all.contains("제 1 교시"), "{all}");
    assert!(all.contains("홀수형"), "{all}");
    assert!(session.is_complete());
}
