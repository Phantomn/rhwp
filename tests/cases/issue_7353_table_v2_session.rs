//! Table-local session contracts. Fresh synthetic Document IR is not a Hancom
//! fixture, and the preview viewport is not the document's actual body layout.
use rhwp::{
    model::{
        control::Control,
        document::{Document, Section},
        paragraph::{CharShapeRef, LineSeg, Paragraph},
        style::{CharShape, LineSpacingType, ParaShape},
        table::{Cell, Table, TablePageBreak},
        Padding,
    },
    renderer::{
        render_tree::{RenderNode, RenderNodeType},
        style_resolver::resolve_styles,
        svg::SvgRenderer,
        table_v2::*,
    },
};

const SELECT: TableSelection = TableSelection {
    section: 1,
    paragraph: 1,
    control: 1,
};

fn paragraph(text: &str) -> Paragraph {
    Paragraph {
        text: text.into(),
        char_count: text.encode_utf16().count() as u32,
        char_offsets: text
            .char_indices()
            .map(|(i, _)| text[..i].encode_utf16().count() as u32)
            .collect(),
        char_shapes: vec![CharShapeRef {
            start_pos: 0,
            char_shape_id: 0,
        }],
        ..Default::default()
    }
}

fn table(texts: &[&str]) -> Table {
    Table {
        row_count: 1,
        col_count: 1,
        page_break: TablePageBreak::CellBreak,
        // 96 DPI: width212px; left5/right7/top3/bottom4 pixels.
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
            paragraphs: texts.iter().map(|s| paragraph(s)).collect(),
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn document(texts: &[&str]) -> Document {
    let mut d = Document::default();
    // 9pt at96dpi =12px. Fixed spacing in IR is twice HU:2700/2/75 =18px.
    d.doc_info.char_shapes.push(CharShape {
        base_size: 900,
        ..Default::default()
    });
    d.doc_info.para_shapes.push(ParaShape {
        line_spacing_type: LineSpacingType::Fixed,
        line_spacing: 2700,
        ..Default::default()
    });
    d.sections = vec![
        Section::default(),
        Section {
            paragraphs: vec![
                paragraph("unselected"),
                Paragraph {
                    text: "outer host is not a previewed cell".into(),
                    controls: vec![
                        Control::ColumnDef(Default::default()),
                        Control::Table(Box::new(table(texts))),
                    ],
                    ..Default::default()
                },
            ],
            ..Default::default()
        },
    ];
    d
}

fn selected(d: &mut Document) -> &mut Table {
    match &mut d.sections[1].paragraphs[1].controls[1] {
        Control::Table(t) => t,
        _ => unreachable!(),
    }
}

fn pages(height: f64) -> TablePreviewPages {
    TablePreviewPages {
        width: 400.0,
        height: 180.0,
        body: Rect {
            x: 20.0,
            y: 30.0,
            width: 212.0,
            height,
        },
        first_y: 30.0,
    }
}

fn collect<'a>(n: &'a RenderNode, lines: &mut Vec<&'a RenderNode>) {
    if matches!(n.node_type, RenderNodeType::TextLine(_)) {
        lines.push(n);
    }
    for child in &n.children {
        collect(child, lines);
    }
}

fn text(n: &RenderNode) -> String {
    n.children
        .iter()
        .filter_map(|c| match &c.node_type {
            RenderNodeType::TextRun(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn document_selection_styles_pagination_and_output_share_one_snapshot() {
    let d = document(&["alpha", "beta", "gamma"]);
    let mut s = TablePreviewSession::from_document(&d, SELECT, 96.0, pages(22.0), 3).unwrap();
    assert_eq!(s.selection(), Some(SELECT));
    for (index, (expected, y, height)) in [
        ("alpha", 33.0, 21.0),
        ("beta", 30.0, 18.0),
        ("gamma", 30.0, 22.0),
    ]
    .into_iter()
    .enumerate()
    {
        let page = s.next_page().unwrap().unwrap();
        assert_eq!(page.page_index, index as u32);
        assert_eq!(page.fragment.geometry().reserved_height(), height);
        let RenderNodeType::Page(root) = &page.tree.root.node_type else {
            panic!("not a page");
        };
        assert_eq!(root.page_index, index as u32);
        assert_eq!((root.width, root.height), (400.0, 180.0));
        let mut lines = Vec::new();
        collect(&page.tree.root, &mut lines);
        assert_eq!(lines.len(), 1);
        assert_eq!(text(lines[0]), expected);
        assert_eq!(
            (lines[0].bbox.x, lines[0].bbox.y, lines[0].bbox.height),
            (25.0, y, 12.0)
        );
        assert_eq!(page.tree.root.children[0].bbox.height, height);
        if let Ok(dir) = std::env::var("ISSUE7353_PREVIEW_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            let mut svg = SvgRenderer::new();
            svg.render_tree(&page.tree);
            std::fs::write(format!("{dir}/session-{index}.svg"), svg.output()).unwrap();
        }
    }
    assert_eq!(s.emitted_pages(), 3);
    assert!(s.next_page().unwrap().is_none()); // completion wins over exact page limit
    assert!(s.next_page().unwrap().is_none()); // no spurious trailing page
    let Control::Table(t) = &d.sections[1].paragraphs[1].controls[1] else {
        unreachable!()
    };
    assert!(t.cells[0].paragraphs.iter().all(|p| p.line_segs.is_empty()));
}

#[test]
fn original_document_edit_requires_an_explicit_new_session() {
    let mut d = document(&["old"]);
    let mut old = TablePreviewSession::from_document(&d, SELECT, 96.0, pages(100.0), 2).unwrap();
    selected(&mut d).cells[0].paragraphs[0] = paragraph("new");
    d.doc_info.char_shapes[0].base_size = 1350; //18px
    d.doc_info.para_shapes[0].line_spacing = 3600; //24px advance
    let mut new = TablePreviewSession::from_document(&d, SELECT, 96.0, pages(100.0), 2).unwrap();
    for (session, expected, box_height, advance) in
        [(&mut old, "old", 12.0, 18.0), (&mut new, "new", 18.0, 24.0)]
    {
        let page = session.next_page().unwrap().unwrap();
        let mut lines = Vec::new();
        collect(&page.tree.root, &mut lines);
        assert_eq!(text(lines[0]), expected);
        assert_eq!(lines[0].bbox.height, box_height);
        assert_eq!(page.fragment.geometry().reserved_height(), 7.0 + advance);
    }
}

#[test]
fn atomic_nonfit_moves_to_fresh_page_without_emitting_a_fake_empty_page() {
    let mut d = document(&["alpha", "beta"]);
    selected(&mut d).page_break = TablePageBreak::None;
    let mut geometry = pages(60.0);
    geometry.first_y = 80.0; //10px left; table needs43
    let mut s = TablePreviewSession::from_document(&d, SELECT, 96.0, geometry, 1).unwrap();
    let page = s.next_page().unwrap().unwrap();
    assert_eq!(page.page_index, 1);
    assert_eq!(page.fragment.geometry().reserved_height(), 43.0);
    assert_eq!(page.tree.root.children[0].bbox.y, 30.0);
    let mut lines = Vec::new();
    collect(&page.tree.root, &mut lines);
    assert_eq!(
        lines
            .iter()
            .map(|n| (text(n), n.bbox.y))
            .collect::<Vec<_>>(),
        [("alpha".into(), 33.0), ("beta".into(), 51.0)]
    );
    assert_eq!(s.emitted_pages(), 1);
    assert!(s.next_page().unwrap().is_none());
}

#[test]
fn oversized_errors_do_not_advance_or_loop_and_do_not_fallback() {
    let mut d = document(&["alpha", "beta"]);
    selected(&mut d).page_break = TablePageBreak::None;
    let mut geometry = pages(22.0);
    geometry.first_y = 50.0;
    let mut s = TablePreviewSession::from_document(&d, SELECT, 96.0, geometry, 10).unwrap();
    for _ in 0..2 {
        assert!(matches!(
            s.next_page(),
            Err(TablePreviewError::DoesNotFit {
                page_index: 1,
                required_height: 43.0,
                ..
            })
        ));
        assert_eq!(s.emitted_pages(), 0);
    }
    geometry.body.width = 100.0;
    let mut narrow = TablePreviewSession::from_document(&d, SELECT, 96.0, geometry, 10).unwrap();
    assert!(matches!(
        narrow.next_page(),
        Err(TablePreviewError::DoesNotFit {
            page_index: 0,
            required_width: 212.0,
            ..
        })
    ));
    assert_eq!(narrow.emitted_pages(), 0);
}

#[test]
fn page_limit_preserves_pending_cursor_and_committed_page_is_independent() {
    let d = document(&["alpha", "beta", "gamma"]);
    let mut s = TablePreviewSession::from_document(&d, SELECT, 96.0, pages(22.0), 1).unwrap();
    let mut page = s.next_page().unwrap().unwrap();
    page.tree.root.children.clear(); // caller owns this output, not session state
    for _ in 0..2 {
        assert!(matches!(
            s.next_page(),
            Err(TablePreviewError::PageLimit { limit: 1 })
        ));
        assert_eq!(s.emitted_pages(), 1);
    }
    let mut fresh = TablePreviewSession::from_document(&d, SELECT, 96.0, pages(22.0), 3).unwrap();
    assert_eq!(fresh.next_page().unwrap().unwrap().page_index, 0);
}

#[test]
fn malformed_viewports_and_selection_are_explicit_errors() {
    let d = document(&["alpha"]);
    for selection in [
        TableSelection {
            section: 3,
            ..SELECT
        },
        TableSelection {
            paragraph: 3,
            ..SELECT
        },
        TableSelection {
            control: 0,
            ..SELECT
        },
    ] {
        assert!(
            matches!(TablePreviewSession::from_document(&d, selection, 96.0, pages(22.0), 2), Err(TablePreviewError::InvalidSelection(s)) if s == selection)
        );
    }
    let mut invalid = Vec::new();
    let mut p = pages(22.0);
    p.width = f64::NAN;
    invalid.push(p);
    let mut p = pages(22.0);
    p.body.x = -1.0;
    invalid.push(p);
    let mut p = pages(22.0);
    p.first_y = 53.0;
    invalid.push(p);
    let mut p = pages(22.0);
    p.body.height = 0.0;
    invalid.push(p);
    let mut p = pages(22.0);
    p.body.width = 500.0;
    invalid.push(p);
    for p in invalid {
        assert!(matches!(
            TablePreviewSession::from_document(&d, SELECT, 96.0, p, 2),
            Err(TablePreviewError::InvalidPages)
        ));
    }
    assert!(matches!(
        TablePreviewSession::from_document(&d, SELECT, 96.0, pages(22.0), 0),
        Err(TablePreviewError::PageLimit { limit: 0 })
    ));
    assert!(TablePreviewSession::from_document(&d, SELECT, f64::INFINITY, pages(22.0), 2).is_err());
}

#[test]
fn stored_rows_are_rejected_without_rewriting_the_source() {
    let mut d = document(&["alpha"]);
    selected(&mut d).cells[0].paragraphs[0]
        .line_segs
        .push(LineSeg {
            vertical_pos: 777,
            ..Default::default()
        });
    assert!(matches!(
        TablePreviewSession::from_document(&d, SELECT, 96.0, pages(22.0), 2),
        Err(TablePreviewError::Geometry(GeometryError::Unsupported(_)))
    ));
    assert_eq!(
        selected(&mut d).cells[0].paragraphs[0].line_segs[0].vertical_pos,
        777
    );
}

#[test]
fn prepared_nested_session_keeps_parent_child_following_text_on_same_plan() {
    let d = document(&[]);
    let styles = resolve_styles(&d.doc_info, 96.0);
    let leaf = PreparedTextTable::prepare(&table(&["alpha", "beta"]), &styles, 96.0).unwrap();
    let parent = PreparedTextTable::from_flow_rows(
        vec![230.0],
        vec![TextFlowRow {
            cells: vec![TextFlowCell {
                padding: Insets {
                    left: 2.0,
                    right: 2.0,
                    top: 1.0,
                    bottom: 2.0,
                },
                minimum_height: 0.0,
                blocks: vec![
                    TextFlowBlock::Table {
                        owner: ControlOwner {
                            paragraph: 0,
                            control: 0,
                        },
                        table: leaf,
                    },
                    TextFlowBlock::Paragraph {
                        owner: 1,
                        paragraph: Box::new(paragraph("after")),
                    },
                ],
            }],
        }],
        0.0,
        SplitPolicy::WithinCells,
        &styles,
        96.0,
    )
    .unwrap();
    let mut geometry = pages(22.0);
    geometry.body.width = 230.0;
    let mut s = TablePreviewSession::new(&parent, geometry, 3).unwrap();
    assert_eq!(s.selection(), None);
    // Parent top1 + leaf43 + following18 + bottom2 =64:22,22,20.
    for (expected, x, y, h) in [
        ("alpha", 27.0, 34.0, 22.0),
        ("beta", 27.0, 30.0, 22.0),
        ("after", 22.0, 30.0, 20.0),
    ] {
        let page = s.next_page().unwrap().unwrap();
        let mut lines = Vec::new();
        collect(&page.tree.root, &mut lines);
        assert_eq!(lines.len(), 1);
        assert_eq!(text(lines[0]), expected);
        assert_eq!((lines[0].bbox.x, lines[0].bbox.y), (x, y));
        assert_eq!(page.fragment.geometry().reserved_height(), h);
    }
    assert!(s.next_page().unwrap().is_none());
}

#[test]
fn oversized_child_after_committed_content_preserves_error_and_remaining_flow() {
    let d = document(&[]);
    let styles = resolve_styles(&d.doc_info, 96.0);
    let mut t = table(&["alpha", "beta"]);
    t.page_break = TablePageBreak::None; //43px indivisible child
    let leaf = PreparedTextTable::prepare(&t, &styles, 96.0).unwrap();
    let parent = PreparedTextTable::from_flow_rows(
        vec![230.0],
        vec![TextFlowRow {
            cells: vec![TextFlowCell {
                padding: Insets::default(),
                minimum_height: 0.0,
                blocks: vec![
                    TextFlowBlock::Paragraph {
                        owner: 0,
                        paragraph: Box::new(paragraph("before")),
                    },
                    TextFlowBlock::Table {
                        owner: ControlOwner {
                            paragraph: 1,
                            control: 0,
                        },
                        table: leaf,
                    },
                    TextFlowBlock::Paragraph {
                        owner: 2,
                        paragraph: Box::new(paragraph("after")),
                    },
                ],
            }],
        }],
        0.0,
        SplitPolicy::WithinCells,
        &styles,
        96.0,
    )
    .unwrap();
    let mut geometry = pages(22.0);
    geometry.body.width = 230.0;
    let mut session = TablePreviewSession::new(&parent, geometry, 10).unwrap();
    let page = session.next_page().unwrap().unwrap();
    assert_eq!(page.fragment.geometry().reserved_height(), 18.0);
    let mut lines = Vec::new();
    collect(&page.tree.root, &mut lines);
    assert_eq!(
        lines.iter().map(|n| text(n)).collect::<Vec<_>>(),
        ["before"]
    );
    for _ in 0..2 {
        assert!(matches!(
            session.next_page(),
            Err(TablePreviewError::DoesNotFit {
                page_index: 1,
                required_height: 43.0,
                ..
            })
        ));
        assert_eq!(session.emitted_pages(), 1);
    }
    // New viewport = new session from the same immutable plan, not a mid-step
    // width/engine switch. All content is still available exactly once.
    geometry.body.height = 100.0;
    let mut restarted = TablePreviewSession::new(&parent, geometry, 2).unwrap();
    let page = restarted.next_page().unwrap().unwrap();
    assert_eq!(page.fragment.geometry().reserved_height(), 79.0); //18+43+18
    let mut lines = Vec::new();
    collect(&page.tree.root, &mut lines);
    assert_eq!(
        lines.iter().map(|n| text(n)).collect::<Vec<_>>(),
        ["before", "alpha", "beta", "after"]
    );
    assert!(restarted.next_page().unwrap().is_none());
}
