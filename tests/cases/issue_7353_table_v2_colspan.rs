//! HWP cell width belongs to the entire col_span, not each covered column.
//! Synthetic fresh IR contracts; not a Hancom saved-layout oracle.
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
fn cell(row: u16, col: u16, span: u16, width: u32, text: &str) -> Cell {
    Cell {
        row,
        col,
        col_span: span,
        row_span: 1,
        width,
        paragraphs: vec![p(text)],
        ..Default::default()
    }
}
fn table(rows: u16, cols: u16, cells: Vec<Cell>) -> Table {
    Table {
        row_count: rows,
        col_count: cols,
        cells,
        page_break: TablePageBreak::CellBreak,
        common: CommonObjAttr {
            text_wrap: TextWrap::TopAndBottom,
            horz_rel_to: HorzRelTo::Para,
            vert_rel_to: VertRelTo::Para,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn merged() -> Table {
    // Logical widths40/60/100px; title spans all200px. Body is100+100px.
    let mut t = table(
        3,
        3,
        vec![
            cell(0, 0, 3, 15000, "title"),
            cell(1, 0, 2, 7500, "left"),
            cell(1, 2, 1, 7500, "right"),
            cell(2, 0, 1, 3000, "A"),
            cell(2, 1, 1, 4500, "B"),
            cell(2, 2, 1, 7500, "C"),
        ],
    );
    t.repeat_header = true;
    t.cells[0].is_header = true;
    t
}
fn host(text: &str, t: Table) -> Paragraph {
    let mut a = p(text);
    a.controls.push(Control::Table(Box::new(t)));
    a.char_count += 8;
    a.char_offsets.iter_mut().for_each(|v| *v += 8);
    a
}
fn doc(t: Table) -> Document {
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
    d.sections = vec![Section {
        paragraphs: vec![host("", t)],
        ..Default::default()
    }];
    d
}
fn session(d: &Document, h: f64) -> Result<TablePreviewSession, TablePreviewError> {
    let control = d.sections[0].paragraphs[0]
        .controls
        .iter()
        .position(|c| matches!(c, Control::Table(_)))
        .unwrap();
    TablePreviewSession::from_document(
        d,
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
                height: h,
            },
            first_y: 30.0,
        },
        100,
    )
}
fn cells(n: &RenderNode) -> Vec<(u16, u16, u16, f64, f64, f64)> {
    let mut out = vec![];
    if let RenderNodeType::TableCell(c) = &n.node_type {
        out.push((c.row, c.col, c.col_span, n.bbox.x, n.bbox.width, n.bbox.y));
    }
    for c in &n.children {
        out.extend(cells(c));
    }
    out
}
fn lines(n: &RenderNode) -> Vec<(String, f64, f64)> {
    if matches!(n.node_type, RenderNodeType::TextLine(_)) {
        let t = n
            .children
            .iter()
            .filter_map(|c| match &c.node_type {
                RenderNodeType::TextRun(r) => Some(r.text.as_str()),
                _ => None,
            })
            .collect();
        vec![(t, n.bbox.x, n.bbox.y)]
    } else {
        n.children.iter().flat_map(lines).collect()
    }
}
fn bounds(n: &RenderNode, left: f64, top: f64, right: f64, bottom: f64) {
    if matches!(
        n.node_type,
        RenderNodeType::Table(_) | RenderNodeType::TableCell(_) | RenderNodeType::TextLine(_)
    ) {
        assert!(
            n.bbox.x >= left
                && n.bbox.y >= top
                && n.bbox.x + n.bbox.width <= right
                && n.bbox.y + n.bbox.height <= bottom,
            "{:?}",
            n.bbox
        );
        for c in &n.children {
            bounds(
                c,
                n.bbox.x,
                n.bbox.y,
                n.bbox.x + n.bbox.width,
                n.bbox.y + n.bbox.height,
            );
        }
    } else {
        for c in &n.children {
            bounds(c, left, top, right, bottom);
        }
    }
}

#[test]
fn spanning_header_and_cells_keep_grid_addresses_through_paint() {
    let mut t = merged();
    t.cells.reverse();
    let d = doc(t);
    let source = format!("{d:?}");
    let mut s = session(&d, 36.0).unwrap();
    let expected = [
        vec![
            (0, 0, 3, 20.0, 200.0, 30.0),
            (1, 0, 2, 20.0, 100.0, 48.0),
            (1, 2, 1, 120.0, 100.0, 48.0),
        ],
        vec![
            (0, 0, 3, 20.0, 200.0, 30.0),
            (2, 0, 1, 20.0, 40.0, 48.0),
            (2, 1, 1, 60.0, 60.0, 48.0),
            (2, 2, 1, 120.0, 100.0, 48.0),
        ],
    ];
    for (i, e) in expected.into_iter().enumerate() {
        let a = s.next_page().unwrap().unwrap();
        assert_eq!(a.fragment.geometry().reserved_height(), 36.0);
        let actual = cells(&a.tree.root);
        assert_eq!(actual.len(), e.len());
        for (a, e) in actual.iter().zip(&e) {
            assert_eq!((a.0, a.1, a.2), (e.0, e.1, e.2));
            for (a, e) in [(a.3, e.3), (a.4, e.4), (a.5, e.5)] {
                // HWP->pixel multiplication may round 60px to 60.00000000000001.
                assert!((a - e).abs() <= f64::EPSILON * 8.0 * e.abs().max(1.0));
            }
        }
        bounds(&a.tree.root, 20.0, 30.0, 220.0, 66.0);
        assert_eq!(
            lines(&a.tree.root)
                .iter()
                .map(|l| l.0.as_str())
                .collect::<Vec<_>>(),
            if i == 0 {
                vec!["title", "left", "right"]
            } else {
                vec!["title", "A", "B", "C"]
            }
        );
        if let Ok(dir) = std::env::var("ISSUE7353_PREVIEW_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            let mut svg = SvgRenderer::new();
            svg.render_tree(&a.tree);
            std::fs::write(format!("{dir}/colspan-{i}.svg"), svg.output()).unwrap();
        }
    }
    assert!(s.next_page().unwrap().is_none());
    assert_eq!(format!("{d:?}"), source);
}

#[test]
fn merged_cell_effective_width_and_nested_continuation_share_geometry() {
    let mut outer = table(1, 2, vec![cell(0, 0, 2, 15900, "")]);
    outer.cells[0].apply_inner_margin = true;
    outer.cells[0].padding.left = 375;
    outer.cells[0].padding.right = 525; //5/7, inner200
    outer.cells[0].paragraphs = vec![host("host", merged()), p("after")];
    let d = doc(outer);
    let mut s = session(&d, 36.0).unwrap();
    for (i, expected) in [
        vec!["title", "left", "right"],
        vec!["title", "A", "B", "C"],
        vec!["host", "after"],
    ]
    .into_iter()
    .enumerate()
    {
        let a = s.next_page().unwrap().unwrap();
        assert_eq!(a.fragment.geometry().reserved_height(), 36.0);
        assert_eq!(
            lines(&a.tree.root)
                .iter()
                .map(|l| l.0.as_str())
                .collect::<Vec<_>>(),
            expected
        );
        let c = cells(&a.tree.root);
        assert_eq!(c[0], (0, 0, 2, 20.0, 212.0, 30.0));
        if i < 2 {
            assert_eq!(c[1], (0, 0, 3, 25.0, 200.0, 30.0));
        }
        bounds(&a.tree.root, 20.0, 30.0, 232.0, 66.0);
        if let Ok(dir) = std::env::var("ISSUE7353_PREVIEW_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            let mut svg = SvgRenderer::new();
            svg.render_tree(&a.tree);
            std::fs::write(format!("{dir}/nested-colspan-{i}.svg"), svg.output()).unwrap();
        }
    }
    assert!(s.next_page().unwrap().is_none());
}

#[test]
fn unknown_internal_boundary_of_fully_merged_columns_is_not_invented() {
    let t = table(
        2,
        4,
        vec![cell(0, 0, 4, 15000, "A"), cell(1, 0, 4, 15000, "B")],
    );
    let a = session(&doc(t), 100.0)
        .unwrap()
        .next_page()
        .unwrap()
        .unwrap();
    assert_eq!(
        cells(&a.tree.root),
        [(0, 0, 4, 20.0, 200.0, 30.0), (1, 0, 4, 20.0, 200.0, 48.0)]
    );
    assert_eq!(a.fragment.geometry().reserved_height(), 36.0);
}

#[test]
fn inconsistent_widths_overlap_holes_zero_span_and_rowspan_are_rejected() {
    for mode in 0..6 {
        let mut t = merged();
        match mode {
            0 => {
                t.cells[3].width += 75;
                t.cells[4].width -= 75;
            } //same total but disputed boundary? col1 only lastrow: no dispute
            1 => t.cells[1].col_span = 3,
            2 => {
                t.cells.remove(2);
            }
            3 => t.cells[0].col_span = 0,
            4 => t.cells[0].row_span = 2,
            _ => t.cells[2].width += 75,
        }
        if mode == 0 {
            t.cells[1].width += 75;
            t.cells[2].width -= 75;
        } //boundary col2 conflicts with last row
        let d = doc(t);
        assert!(session(&d, 100.0).is_err(), "mode {mode}");
    }
}

#[test]
fn merged_row_split_policies_and_budget_sweep_preserve_content_once() {
    for policy in [
        TablePageBreak::None,
        TablePageBreak::RowBreak,
        TablePageBreak::CellBreak,
    ] {
        let mut t = merged();
        t.repeat_header = false;
        t.page_break = policy;
        let d = doc(t);
        for budget in [18.0, 36.0, 54.0, 100.0] {
            let mut s = session(&d, budget).unwrap();
            if policy == TablePageBreak::None && budget < 54.0 {
                assert!(s.next_page().is_err());
                continue;
            }
            let mut all = Vec::new();
            let mut used = 0.0;
            while let Some(a) = s.next_page().unwrap() {
                used += a.fragment.geometry().reserved_height();
                all.extend(lines(&a.tree.root).into_iter().map(|l| l.0));
                bounds(&a.tree.root, 20.0, 30.0, 220.0, 30.0 + budget);
            }
            assert_eq!(all, ["title", "left", "right", "A", "B", "C"]);
            assert_eq!(used, 54.0);
        }
    }
}

#[test]
fn colspan_hwpx_roundtrip_keeps_width_and_owner_contract() {
    let d = doc(merged());
    let bytes = rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap();
    let reopened = rhwp::parser::hwpx::parse_hwpx(&bytes).unwrap();
    let mut a = session(&reopened, 36.0).unwrap();
    let mut e = session(&d, 36.0).unwrap();
    for _ in 0..2 {
        let ap = a.next_page().unwrap().unwrap();
        let ep = e.next_page().unwrap().unwrap();
        assert_eq!(cells(&ap.tree.root), cells(&ep.tree.root));
        assert_eq!(lines(&ap.tree.root), lines(&ep.tree.root));
    }
    assert!(a.next_page().unwrap().is_none());
    if let Ok(dir) = std::env::var("ISSUE7353_PREVIEW_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/fresh-colspan.hwpx"), bytes).unwrap();
    }
}

#[test]
fn composition_receives_full_merged_width_minus_resolved_padding() {
    struct Fill;
    impl CellParagraphComposer for Fill {
        fn compose(&self, _: &Paragraph, width: f64) -> Result<Vec<ParagraphItem>, GeometryError> {
            Ok(vec![ParagraphItem::Lines {
                height: 10.0,
                lines: vec![(
                    0,
                    Rect {
                        x: 0.0,
                        y: 0.0,
                        width,
                        height: 10.0,
                    },
                )],
            }])
        }
    }
    let mut t = merged();
    t.repeat_header = false;
    t.cells[1].apply_inner_margin = true;
    t.cells[1].padding.left = 225;
    t.cells[1].padding.right = 525; //3/7px
    let cursor = TableContentPlan::from_ir_contents(&t, 96.0 / 7200.0, &Fill)
        .unwrap()
        .start();
    let FragmentFit::Placed(f) = cursor
        .fit(PageArea {
            bounds: Rect {
                x: 20.0,
                y: 30.0,
                width: 200.0,
                height: 100.0,
            },
        })
        .unwrap()
    else {
        panic!()
    };
    let merged = &f.placement().cells[1];
    assert_eq!(merged.column, 0);
    assert_eq!(
        merged.bounds,
        Rect {
            x: 20.0,
            y: 40.0,
            width: 100.0,
            height: 10.0
        }
    );
    assert_eq!(
        merged.lines[0].bounds,
        Rect {
            x: 23.0,
            y: 40.0,
            width: 90.0,
            height: 10.0
        }
    );
    assert_eq!(f.reserved_height(), 30.0);
}

#[test]
fn edges_observed_on_different_rows_cannot_cross_or_collapse() {
    for middle in [7500, 9000] {
        //col1 at100/120px, col2 at100px
        let t = table(
            2,
            3,
            vec![
                cell(0, 0, 2, 7500, "A"),
                cell(0, 2, 1, 7500, "B"),
                cell(1, 0, 1, middle, "C"),
                cell(1, 1, 2, 15000 - middle, "D"),
            ],
        );
        assert!(matches!(
            session(&doc(t), 100.0),
            Err(TablePreviewError::Geometry(GeometryError::Unsupported(
                "non-increasing grid boundary"
            )))
        ));
    }
}

#[test]
fn merged_text_wrap_matches_same_physical_unmerged_width_at_multiple_dpi() {
    let text = "Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu. ".repeat(3);
    for dpi in [72.0, 96.0, 144.0] {
        let mut merged = table(1, 3, vec![cell(0, 0, 3, 11250, &text)]);
        merged.cells[0].apply_inner_margin = true;
        merged.cells[0].padding.left = 151;
        merged.cells[0].padding.right = 229;
        let mut plain = merged.clone();
        plain.col_count = 1;
        plain.cells[0].col_span = 1;
        let md = doc(merged);
        let pd = doc(plain);
        let area = PageArea {
            bounds: Rect {
                x: 20.0,
                y: 30.0,
                width: 300.0,
                height: 1000.0,
            },
        };
        let ms = rhwp::renderer::style_resolver::resolve_styles(&md.doc_info, dpi);
        let Control::Table(mt) = &md.sections[0].paragraphs[0].controls[0] else {
            panic!()
        };
        let Control::Table(pt) = &pd.sections[0].paragraphs[0].controls[0] else {
            panic!()
        };
        let mp = PreparedTextTable::prepare(mt, &ms, dpi).unwrap();
        let pp = PreparedTextTable::prepare(pt, &ms, dpi).unwrap();
        let TextFragmentFit::Placed(m) = mp.start().fit(area).unwrap() else {
            panic!()
        };
        let TextFragmentFit::Placed(p) = pp.start().fit(area).unwrap() else {
            panic!()
        };
        assert!(m.geometry().placement().cells[0].lines.len() > 1);
        assert_eq!(
            m.geometry().placement().cells[0].lines,
            p.geometry().placement().cells[0].lines
        );
        assert_eq!(
            m.geometry().reserved_height(),
            p.geometry().reserved_height()
        );
        // Width150px at96DPI less the declared asymmetric padding, not /3.
        assert_eq!(
            m.geometry().placement().cells[0].bounds.width,
            11250.0 * dpi / 7200.0
        );
    }
}

#[test]
fn split_merged_cell_keeps_short_sibling_empty_on_continuation() {
    let mut t = merged();
    t.row_count = 2;
    t.cells.truncate(3);
    t.cells[1].paragraphs = vec![p("A"), p("B"), p("C")];
    let mut s = session(&doc(t), 36.0).unwrap();
    for (i, body) in ["A", "B", "C"].into_iter().enumerate() {
        let a = s.next_page().unwrap().unwrap();
        assert_eq!(a.fragment.geometry().reserved_height(), 36.0);
        let all = lines(&a.tree.root);
        assert_eq!(
            all.iter().map(|l| l.0.as_str()).collect::<Vec<_>>(),
            if i == 0 {
                vec!["title", body, "right"]
            } else {
                vec!["title", body]
            }
        );
        let c = cells(&a.tree.root);
        assert_eq!(c[1], (1, 0, 2, 20.0, 100.0, 48.0));
        assert_eq!(c[2], (1, 2, 1, 120.0, 100.0, 48.0));
        bounds(&a.tree.root, 20.0, 30.0, 220.0, 66.0);
    }
    assert!(s.next_page().unwrap().is_none());
}
