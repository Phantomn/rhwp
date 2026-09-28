//! Synthetic fresh IR: a repeating prefix is an atomic physical reservation,
//! not a second consumption of body lines. No Hancom fidelity claim.
use rhwp::{
    model::{
        control::Control,
        document::{Document, Section},
        paragraph::{CharShapeRef, Paragraph},
        shape::{CommonObjAttr, HorzRelTo, TextWrap, VertRelTo},
        style::{CharShape, LineSpacingType, ParaShape},
        table::{Cell, Table, TablePageBreak},
    },
    renderer::{
        render_tree::{RenderNode, RenderNodeType},
        svg::SvgRenderer,
        table_v2::*,
    },
};

fn para(text: &str) -> Paragraph {
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

fn table(rows: &[&[&str]], headers: usize) -> Table {
    Table {
        row_count: rows.len() as u16,
        col_count: 1,
        repeat_header: headers > 0,
        page_break: TablePageBreak::RowBreak,
        common: CommonObjAttr {
            text_wrap: TextWrap::TopAndBottom,
            vert_rel_to: VertRelTo::Para,
            horz_rel_to: HorzRelTo::Para,
            ..Default::default()
        },
        cells: rows
            .iter()
            .enumerate()
            .map(|(row, lines)| Cell {
                row: row as u16,
                row_span: 1,
                col_span: 1,
                width: 15000, //200px
                is_header: row < headers,
                paragraphs: lines.iter().map(|s| para(s)).collect(),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}

fn host(text: &str, child: Table) -> Paragraph {
    let mut p = para(text);
    p.controls.push(Control::Table(Box::new(child)));
    p.char_count += 8;
    p.char_offsets.iter_mut().for_each(|o| *o += 8);
    p
}

fn doc(root: Table) -> Document {
    let mut d = Document::default();
    d.doc_info.char_shapes.push(CharShape {
        base_size: 900,
        ..Default::default()
    });
    d.doc_info.para_shapes.push(ParaShape {
        line_spacing_type: LineSpacingType::Fixed,
        line_spacing: 2700, //18px physical pitch; glyph box12px
        ..Default::default()
    });
    d.sections = vec![Section {
        paragraphs: vec![host("", root)],
        ..Default::default()
    }];
    d
}

fn session(d: &Document, budget: f64) -> Result<TablePreviewSession, TablePreviewError> {
    TablePreviewSession::from_document(
        d,
        TableSelection {
            section: 0,
            paragraph: 0,
            control: 0,
        },
        96.0,
        TablePreviewPages {
            width: 400.0,
            height: 400.0,
            body: Rect {
                x: 20.0,
                y: 30.0,
                width: 300.0,
                height: budget,
            },
            first_y: 30.0,
        },
        100,
    )
}

fn lines(node: &RenderNode) -> Vec<(String, f64)> {
    if matches!(node.node_type, RenderNodeType::TextLine(_)) {
        let text = node
            .children
            .iter()
            .filter_map(|c| match &c.node_type {
                RenderNodeType::TextRun(r) => Some(r.text.as_str()),
                _ => None,
            })
            .collect();
        vec![(text, node.bbox.y)]
    } else {
        node.children.iter().flat_map(lines).collect()
    }
}

fn assert_contained(n: &RenderNode, top: f64, bottom: f64) {
    if matches!(
        n.node_type,
        RenderNodeType::Table(_) | RenderNodeType::TableCell(_) | RenderNodeType::TextLine(_)
    ) {
        assert!(
            n.bbox.y >= top && n.bbox.y + n.bbox.height <= bottom,
            "{:?}",
            n.bbox
        );
        for c in &n.children {
            assert_contained(c, n.bbox.y, n.bbox.y + n.bbox.height);
        }
    } else {
        for c in &n.children {
            assert_contained(c, top, bottom);
        }
    }
}

#[test]
fn repeated_prefix_reserves_actual_height_and_reuses_exact_paint() {
    let d = doc(table(&[&["heading"], &["alpha", "beta", "gamma"]], 1));
    let source = format!("{d:?}");
    let mut s = session(&d, 36.0).unwrap();
    for (i, body) in ["alpha", "beta", "gamma"].into_iter().enumerate() {
        let page = s.next_page().unwrap().unwrap();
        assert_eq!(page.page_index as usize, i);
        assert_eq!(page.fragment.geometry().reserved_height(), 36.0);
        assert_eq!(
            lines(&page.tree.root),
            [("heading".into(), 30.0), (body.into(), 48.0)]
        );
        assert_eq!(
            page.fragment.geometry().rows(),
            if i == 0 { 0..2 } else { 1..2 }
        );
        assert_contained(&page.tree.root, 30.0, 66.0);
        if let Ok(dir) = std::env::var("ISSUE7353_PREVIEW_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            let mut svg = SvgRenderer::new();
            svg.render_tree(&page.tree);
            std::fs::write(format!("{dir}/header-{i}.svg"), svg.output()).unwrap();
        }
    }
    assert!(s.next_page().unwrap().is_none());
    assert_eq!(format!("{d:?}"), source);
}

#[test]
fn header_only_fit_is_nonfit_and_retry_does_not_consume_body() {
    let d = doc(table(&[&["H"], &["body"]], 1));
    let mut s = session(&d, 29.0).unwrap(); //18 header + only11: body glyph12 cannot fit
    for _ in 0..2 {
        let e = s
            .next_page()
            .err()
            .expect("header alone must not commit a page");
        assert!(
            matches!(
                e,
                TablePreviewError::DoesNotFit {
                    page_index: 0,
                    required_height: 30.0,
                    ..
                }
            ),
            "{e:?}"
        );
    }
    // A new independent session with enough budget retains both lines.
    let mut enough = session(&d, 36.0).unwrap();
    assert_eq!(
        lines(&enough.next_page().unwrap().unwrap().tree.root),
        [("H".into(), 30.0), ("body".into(), 48.0)]
    );
    assert!(enough.next_page().unwrap().is_none());
}

#[test]
fn multirow_prefix_is_atomic_and_between_rows_budget_includes_it() {
    let mut t = table(&[&["H1"], &["H2"], &["A"], &["B"]], 2);
    t.page_break = TablePageBreak::CellBreak;
    let d = doc(t);
    let mut s = session(&d, 53.0).unwrap();
    assert!(matches!(
        s.next_page(),
        Err(TablePreviewError::DoesNotFit {
            required_height: 54.0,
            ..
        })
    ));
    let mut s = session(&d, 54.0).unwrap();
    for body in ["A", "B"] {
        let page = s.next_page().unwrap().unwrap();
        assert_eq!(page.fragment.geometry().reserved_height(), 54.0);
        assert_eq!(
            lines(&page.tree.root),
            [
                ("H1".into(), 30.0),
                ("H2".into(), 48.0),
                (body.into(), 66.0)
            ]
        );
    }
    assert!(s.next_page().unwrap().is_none());
}

#[test]
fn nested_header_reservation_advances_parent_and_preserves_following_text() {
    let child = table(&[&["H"], &["A", "B", "C"]], 1);
    let mut outer = table(&[&[]], 0);
    outer.cells[0].paragraphs = vec![para("before"), host("host", child), para("after")];
    let d = doc(outer);
    let mut s = session(&d, 54.0).unwrap();
    let expected = [
        vec![("before", 30.0), ("H", 48.0), ("A", 66.0)],
        vec![("H", 30.0), ("B", 48.0), ("C", 66.0)],
        vec![("host", 30.0), ("after", 48.0)],
    ];
    for (i, e) in expected.into_iter().enumerate() {
        let p = s.next_page().unwrap().unwrap();
        assert_eq!(
            p.fragment.geometry().reserved_height(),
            if i < 2 { 54.0 } else { 36.0 }
        );
        assert_eq!(
            lines(&p.tree.root),
            e.into_iter()
                .map(|(s, y)| (s.into(), y))
                .collect::<Vec<_>>()
        );
        assert_contained(&p.tree.root, 30.0, 84.0);
    }
    assert!(s.next_page().unwrap().is_none());
}

#[test]
fn disabled_repeat_and_unsplit_policy_remain_normal_content() {
    for mode in 0..3 {
        let mut t = table(&[&["H"], &["A", "B"]], 1);
        if mode == 0 {
            t.repeat_header = false;
        }
        if mode == 1 {
            t.page_break = TablePageBreak::None;
        }
        if mode == 2 {
            t.cells[0].is_header = false;
        }
        let mut s = session(&doc(t), 100.0).unwrap();
        let p = s.next_page().unwrap().unwrap();
        assert_eq!(p.fragment.geometry().reserved_height(), 54.0);
        assert_eq!(
            lines(&p.tree.root),
            [("H".into(), 30.0), ("A".into(), 48.0), ("B".into(), 66.0)]
        );
        assert!(s.next_page().unwrap().is_none());
    }
}

#[test]
fn header_content_including_nested_table_is_replayed_as_one_atomic_prefix() {
    let mut t = table(&[&[], &["A", "B"]], 1);
    t.cells[0].paragraphs = vec![host("title-host", table(&[&["deep"]], 0))];
    let mut s = session(&doc(t), 54.0).unwrap();
    for body in ["A", "B"] {
        let p = s.next_page().unwrap().unwrap();
        assert_eq!(
            lines(&p.tree.root),
            [
                ("deep".into(), 30.0),
                ("title-host".into(), 48.0),
                (body.into(), 66.0)
            ]
        );
        assert_contained(&p.tree.root, 30.0, 84.0);
    }
    assert!(s.next_page().unwrap().is_none());
}

#[test]
fn all_header_table_terminates_and_oversized_prefix_never_partially_paints() {
    let t = table(&[&["H1"], &["H2"]], 2);
    let mut s = session(&doc(t.clone()), 35.0).unwrap();
    assert!(matches!(
        s.next_page(),
        Err(TablePreviewError::DoesNotFit {
            required_height: 36.0,
            ..
        })
    ));
    let mut s = session(&doc(t), 36.0).unwrap();
    assert_eq!(lines(&s.next_page().unwrap().unwrap().tree.root).len(), 2);
    assert!(s.next_page().unwrap().is_none());
}

#[test]
fn header_padding_and_minimum_band_are_reserved_on_every_fragment() {
    let mut t = table(&[&["H"], &["A", "B"]], 1);
    t.cells[0].apply_inner_margin = true;
    t.cells[0].padding.top = 150; //2px
    t.cells[0].padding.bottom = 225; //3px
    t.cells[0].height = 2250; //30px minimum > 2+18+3
    t.cells[1].apply_inner_margin = true;
    t.cells[1].padding.top = 75; //1px
    t.cells[1].padding.bottom = 300; //4px
    let mut s = session(&doc(t), 53.0).unwrap();
    let a = s.next_page().unwrap().unwrap();
    assert_eq!(a.fragment.geometry().reserved_height(), 49.0); //30+1+18
    assert_eq!(
        lines(&a.tree.root),
        [("H".into(), 32.0), ("A".into(), 61.0)]
    );
    let b = s.next_page().unwrap().unwrap();
    assert_eq!(b.fragment.geometry().reserved_height(), 52.0); //30+18+4
    assert_eq!(
        lines(&b.tree.root),
        [("H".into(), 32.0), ("B".into(), 60.0)]
    );
    assert_contained(&a.tree.root, 30.0, 79.0);
    assert_contained(&b.tree.root, 30.0, 82.0);
    assert!(s.next_page().unwrap().is_none());
}

#[test]
fn parent_and_child_prefixes_share_the_same_physical_budget() {
    let mut root = table(&[&["parent-H"], &[]], 1);
    root.cells[1].paragraphs = vec![
        para("before"),
        host("host", table(&[&["child-H"], &["A", "B", "C"]], 1)),
        para("after"),
    ];
    let mut s = session(&doc(root), 72.0).unwrap();
    let expected = [
        vec!["parent-H", "before", "child-H", "A"],
        vec!["parent-H", "child-H", "B", "C"],
        vec!["parent-H", "host", "after"],
    ];
    for (i, expected) in expected.into_iter().enumerate() {
        let p = s.next_page().unwrap().unwrap();
        assert_eq!(
            p.fragment.geometry().reserved_height(),
            if i < 2 { 72.0 } else { 54.0 }
        );
        assert_eq!(
            lines(&p.tree.root),
            expected
                .into_iter()
                .enumerate()
                .map(|(j, t)| (t.into(), 30.0 + j as f64 * 18.0))
                .collect::<Vec<_>>()
        );
        assert_contained(&p.tree.root, 30.0, 102.0);
        if let Ok(dir) = std::env::var("ISSUE7353_PREVIEW_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            let mut svg = SvgRenderer::new();
            svg.render_tree(&p.tree);
            std::fs::write(format!("{dir}/nested-header-{i}.svg"), svg.output()).unwrap();
        }
    }
    assert!(s.next_page().unwrap().is_none());
}

#[test]
fn first_partial_page_defers_header_and_body_together() {
    let d = doc(table(&[&["H"], &["A", "B"]], 1));
    let mut s = TablePreviewSession::from_document(
        &d,
        TableSelection {
            section: 0,
            paragraph: 0,
            control: 0,
        },
        96.0,
        TablePreviewPages {
            width: 400.0,
            height: 400.0,
            body: Rect {
                x: 20.0,
                y: 30.0,
                width: 300.0,
                height: 36.0,
            },
            first_y: 42.0,
        },
        100,
    )
    .unwrap();
    for (i, body) in ["A", "B"].into_iter().enumerate() {
        let p = s.next_page().unwrap().unwrap();
        assert_eq!(p.page_index as usize, i + 1);
        assert_eq!(
            lines(&p.tree.root),
            [("H".into(), 30.0), (body.into(), 48.0)]
        );
    }
    assert!(s.next_page().unwrap().is_none());
}

#[test]
fn repeat_budget_sweep_preserves_body_and_reports_real_physical_overhead() {
    let d = doc(table(&[&["H"], &["A", "B", "C", "D"]], 1));
    for budget in [
        30.0, 31.0, 35.0, 36.0, 47.0, 48.0, 53.0, 54.0, 71.0, 72.0, 90.0, 200.0,
    ] {
        let mut s = session(&d, budget).unwrap();
        let mut body = Vec::new();
        let mut physical = 0.0;
        let mut fragments = 0;
        while let Some(p) = s.next_page().unwrap() {
            fragments += 1;
            assert!(fragments < 20);
            let h = p.fragment.geometry().reserved_height();
            assert!(h <= budget);
            physical += h;
            let l = lines(&p.tree.root);
            assert_eq!(l[0], ("H".into(), 30.0));
            body.extend(l.into_iter().skip(1).map(|l| l.0));
            assert_contained(&p.tree.root, 30.0, 30.0 + h);
        }
        assert_eq!(body, ["A", "B", "C", "D"]);
        // 4 body pitches once, one full header pitch per accepted fragment.
        assert_eq!(physical, 72.0 + 18.0 * fragments as f64);
    }
}

#[test]
fn ambiguous_header_markers_are_not_silently_generalized() {
    let mut scattered = table(&[&["A"], &["H"]], 0);
    scattered.repeat_header = true;
    scattered.cells[1].is_header = true;
    assert!(matches!(
        session(&doc(scattered), 100.0),
        Err(TablePreviewError::Geometry(GeometryError::Unsupported(
            "partial or non-leading header rows"
        )))
    ));
    let mut partial = table(&[&["H"], &["A"]], 1);
    partial.col_count = 2;
    for mut cell in partial.cells.clone() {
        cell.col = 1;
        cell.is_header = false;
        partial.cells.push(cell);
    }
    assert!(matches!(
        session(&doc(partial), 100.0),
        Err(TablePreviewError::Geometry(GeometryError::Unsupported(
            "partial or non-leading header rows"
        )))
    ));
}

#[test]
fn complete_header_rows_cover_short_sibling_cells_without_replaying_body() {
    let mut t = table(&[&["HL"], &["A"]], 1);
    t.col_count = 2;
    for mut cell in t.cells.clone() {
        cell.width = 7500; //100px each column
        cell.col = 1;
        cell.paragraphs = if cell.row == 0 {
            vec![para("HR")]
        } else {
            vec![para("B"), para("C")]
        };
        t.cells.push(cell);
    }
    for c in &mut t.cells {
        c.width = 7500;
    }
    t.cells.reverse(); // Repetition must follow grid ownership, not storage order.
    let mut s = session(&doc(t), 36.0).unwrap();
    for (i, expected) in [vec!["HL", "HR", "A", "B"], vec!["HL", "HR", "C"]]
        .into_iter()
        .enumerate()
    {
        let p = s.next_page().unwrap().unwrap();
        let l = lines(&p.tree.root);
        assert_eq!(l.iter().map(|l| l.0.as_str()).collect::<Vec<_>>(), expected);
        assert_eq!(
            l.iter().map(|l| l.1).collect::<Vec<_>>(),
            if i == 0 {
                vec![30.0, 30.0, 48.0, 48.0]
            } else {
                vec![30.0, 30.0, 48.0]
            }
        );
        let cells = &p.fragment.geometry().placement().cells;
        assert_eq!(cells.len(), 4);
        for c in cells {
            assert_eq!(c.bounds.height, 18.0);
        }
        assert_contained(&p.tree.root, 30.0, 66.0);
    }
    assert!(s.next_page().unwrap().is_none());
}

#[test]
fn hwpx_roundtrip_keeps_repeat_flags_and_fragment_geometry() {
    let d = doc(table(&[&["H"], &["A", "B"]], 1));
    let bytes = rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap();
    let reopened = rhwp::parser::hwpx::parse_hwpx(&bytes).unwrap();
    let control = reopened.sections[0].paragraphs[0]
        .controls
        .iter()
        .position(|c| matches!(c, Control::Table(_)))
        .unwrap();
    let mut s = TablePreviewSession::from_document(
        &reopened,
        TableSelection {
            section: 0,
            paragraph: 0,
            control,
        },
        96.0,
        TablePreviewPages {
            width: 400.0,
            height: 400.0,
            body: Rect {
                x: 20.0,
                y: 30.0,
                width: 300.0,
                height: 36.0,
            },
            first_y: 30.0,
        },
        100,
    )
    .unwrap();
    let mut expected = session(&d, 36.0).unwrap();
    for _ in 0..2 {
        let a = s.next_page().unwrap().unwrap();
        let e = expected.next_page().unwrap().unwrap();
        assert_eq!(
            a.fragment.geometry().placement(),
            e.fragment.geometry().placement()
        );
        assert_eq!(lines(&a.tree.root), lines(&e.tree.root));
    }
    assert!(s.next_page().unwrap().is_none());
    if let Ok(dir) = std::env::var("ISSUE7353_PREVIEW_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/fresh-headers.hwpx"), bytes).unwrap();
    }
}
