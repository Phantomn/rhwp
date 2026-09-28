//! Fresh synthetic Document IR, not a Hancom saved-layout oracle.
//! TopAndBottom at paragraph top excludes the full host width; host lines follow
//! the child, unlike TAC, side-wrap or a positive-offset anchor.
use rhwp::{
    model::{
        control::Control,
        document::{Document, Section},
        paragraph::{CharShapeRef, LineSeg, Paragraph},
        shape::{CommonObjAttr, HorzRelTo, TextWrap, VertRelTo},
        style::{CharShape, LineSpacingType, ParaShape},
        table::{Cell, Table, TablePageBreak},
        Padding,
    },
    renderer::{
        render_tree::{PageRenderTree, RenderNode, RenderNodeType},
        style_resolver::resolve_styles,
        svg::SvgRenderer,
        table_v2::*,
    },
};

const SELECT: TableSelection = TableSelection {
    section: 0,
    paragraph: 0,
    control: 0,
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

fn table(width: u32, padding: Padding, paragraphs: Vec<Paragraph>) -> Table {
    Table {
        row_count: 1,
        col_count: 1,
        page_break: TablePageBreak::RowBreak, // HWPX CELL: split at complete lines.
        common: CommonObjAttr {
            text_wrap: TextWrap::TopAndBottom,
            vert_rel_to: VertRelTo::Para,
            horz_rel_to: HorzRelTo::Para,
            ..Default::default()
        },
        padding,
        cells: vec![Cell {
            width,
            row_span: 1,
            col_span: 1,
            paragraphs,
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn leaf() -> Table {
    // 96 DPI: child width150, padding2/3/2/3; height2+18+18+3 =41.
    table(
        11250,
        Padding {
            left: 150,
            right: 225,
            top: 150,
            bottom: 225,
        },
        vec![paragraph("alpha"), paragraph("beta")],
    )
}

fn host(text: &str, child: Table) -> Paragraph {
    let mut p = paragraph(text);
    p.controls.push(Control::Table(Box::new(child)));
    p.char_count += 8;
    for offset in &mut p.char_offsets {
        *offset += 8;
    }
    p
}

fn wrapper(host_text: &str) -> Table {
    // 3 + before18 + child41 + host18 + after18 +4 =102.
    table(
        15900,
        Padding {
            left: 375,
            right: 525,
            top: 225,
            bottom: 300,
        },
        vec![
            paragraph("before"),
            host(host_text, leaf()),
            paragraph("after"),
        ],
    )
}

fn document(root: Table) -> Document {
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
        paragraphs: vec![host("", root)],
        ..Default::default()
    }];
    d
}

fn pages(height: f64) -> TablePreviewPages {
    TablePreviewPages {
        width: 600.0,
        height: 500.0,
        body: Rect {
            x: 20.0,
            y: 30.0,
            width: 550.0,
            height,
        },
        first_y: 30.0,
    }
}

fn session(d: &Document, height: f64) -> TablePreviewSession {
    TablePreviewSession::from_document(d, SELECT, 96.0, pages(height), 100).unwrap()
}

fn root_mut(d: &mut Document) -> &mut Table {
    let Control::Table(t) = &mut d.sections[0].paragraphs[0].controls[0] else {
        panic!()
    };
    t
}

fn child_mut(d: &mut Document) -> &mut Table {
    let Control::Table(t) = &mut root_mut(d).cells[0].paragraphs[1].controls[0] else {
        panic!()
    };
    t
}

fn text(line: &RenderNode) -> String {
    line.children
        .iter()
        .filter_map(|n| match &n.node_type {
            RenderNodeType::TextRun(r) => Some(r.text.as_str()),
            _ => None,
        })
        .collect()
}

fn collect<'a>(
    n: &'a RenderNode,
    lines: &mut Vec<&'a RenderNode>,
    tables: &mut Vec<&'a RenderNode>,
) {
    match n.node_type {
        RenderNodeType::TextLine(_) => lines.push(n),
        RenderNodeType::Table(_) => tables.push(n),
        _ => {}
    }
    for c in &n.children {
        collect(c, lines, tables);
    }
}

fn rendered_lines(page: &PageRenderTree) -> Vec<(String, f64, f64, f64)> {
    let mut lines = Vec::new();
    collect(&page.root, &mut lines, &mut Vec::new());
    lines
        .iter()
        .map(|n| (text(n), n.bbox.x, n.bbox.y, n.bbox.height))
        .collect()
}

#[test]
fn document_nested_ir_uses_one_content_plan_for_split_and_actual_paint() {
    let d = document(wrapper("host"));
    let before = format!("{d:?}");
    let mut s = session(&d, 43.0);
    let expected = [
        (41.0, vec![("before", 25.0, 33.0), ("alpha", 27.0, 53.0)]),
        (39.0, vec![("beta", 27.0, 30.0), ("host", 25.0, 51.0)]),
        (22.0, vec![("after", 25.0, 30.0)]),
    ];
    for (i, (height, lines)) in expected.into_iter().enumerate() {
        let p = s.next_page().unwrap().unwrap();
        assert_eq!(p.page_index as usize, i);
        assert_eq!(p.fragment.geometry().reserved_height(), height);
        assert_eq!(
            rendered_lines(&p.tree),
            lines
                .into_iter()
                .map(|(t, x, y)| (t.into(), x, y, 12.0))
                .collect::<Vec<_>>()
        );
        let mut tables = Vec::new();
        collect(&p.tree.root, &mut Vec::new(), &mut tables);
        assert_eq!(tables[0].bbox.height, height);
        if i < 2 {
            assert_eq!(
                (tables[1].bbox.y, tables[1].bbox.height),
                if i == 0 { (51.0, 20.0) } else { (30.0, 21.0) }
            );
        } else {
            assert_eq!(tables.len(), 1);
        }
        if let Ok(dir) = std::env::var("ISSUE7353_PREVIEW_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            let mut svg = SvgRenderer::new();
            svg.render_tree(&p.tree);
            std::fs::write(format!("{dir}/ir-nested-{i}.svg"), svg.output()).unwrap();
        }
    }
    assert!(s.next_page().unwrap().is_none());
    assert_eq!(format!("{d:?}"), before);
}

#[test]
fn empty_anchor_host_keeps_its_line_and_paragraph_after_spacing() {
    let mut d = document(wrapper(""));
    d.doc_info.para_shapes.push(ParaShape {
        line_spacing_type: LineSpacingType::Fixed,
        line_spacing: 2700,
        spacing_after: 750,
        ..Default::default()
    }); //5px
    root_mut(&mut d).cells[0].paragraphs[1].para_shape_id = 1;
    let p = session(&d, 200.0).next_page().unwrap().unwrap();
    assert_eq!(p.fragment.geometry().reserved_height(), 107.0);
    let lines = rendered_lines(&p.tree);
    assert_eq!(
        lines.iter().map(|l| l.0.as_str()).collect::<Vec<_>>(),
        ["before", "alpha", "beta", "", "after"]
    );
    assert_eq!(lines[3], ("".into(), 25.0, 92.0, 12.0));
    assert_eq!(lines[4].2, 115.0); //92 + empty18 + after5
}

#[test]
fn recursive_ir_paint_owners_are_local_to_each_table_and_cell() {
    let mut root = wrapper("outer-host");
    let inner = table(
        11250,
        Padding::default(),
        vec![host(
            "inner-host",
            table(9000, Padding::default(), vec![paragraph("deep")]),
        )],
    );
    root.cells[0].paragraphs[1] = host("outer-host", inner);
    let p = session(&document(root), 300.0)
        .next_page()
        .unwrap()
        .unwrap();
    let lines = rendered_lines(&p.tree);
    assert_eq!(
        lines.iter().map(|l| l.0.as_str()).collect::<Vec<_>>(),
        ["before", "deep", "inner-host", "outer-host", "after"]
    );
    assert_eq!(
        lines.iter().map(|l| l.2).collect::<Vec<_>>(),
        [33.0, 51.0, 69.0, 87.0, 105.0]
    );
    assert_eq!(p.fragment.geometry().reserved_height(), 97.0); //3+5*18+4

    let mut root = wrapper("left");
    let mut second = root.cells[0].clone();
    second.col = 1;
    second.paragraphs[1].text = "right".into();
    second.paragraphs[1].char_offsets = (8..13).collect();
    second.paragraphs[1].char_count = 13;
    root.col_count = 2;
    root.cells.insert(0, second); // storage order differs from grid order
    let p = session(&document(root), 200.0)
        .next_page()
        .unwrap()
        .unwrap();
    let lines = rendered_lines(&p.tree);
    assert_eq!(lines[3].0, "left");
    assert_eq!(lines[8].0, "right");
    assert_eq!(lines[8].1 - lines[3].1, 212.0);
    assert_eq!(p.fragment.geometry().reserved_height(), 102.0);
}

#[test]
fn ir_host_preserves_utf16_style_offsets_and_explicit_newlines() {
    let mut d = document(wrapper("A\nB"));
    d.doc_info.char_shapes.push(CharShape {
        base_size: 675,
        ..Default::default()
    }); //9px
    root_mut(&mut d).cells[0].paragraphs[1].char_shapes = vec![
        CharShapeRef {
            start_pos: 0,
            char_shape_id: 0,
        },
        CharShapeRef {
            start_pos: 10,
            char_shape_id: 1,
        },
    ];
    let p = session(&d, 200.0).next_page().unwrap().unwrap();
    let mut lines = Vec::new();
    collect(&p.tree.root, &mut lines, &mut Vec::new());
    assert_eq!(
        lines.iter().map(|n| text(n)).collect::<Vec<_>>(),
        ["before", "alpha", "beta", "A", "B", "after"]
    );
    let RenderNodeType::TextRun(run) = &lines[4].children[0].node_type else {
        panic!()
    };
    assert_eq!(run.style.font_size, 9.0);
    assert_eq!(p.fragment.geometry().reserved_height(), 120.0); // additional explicit line18
}

#[test]
fn ir_atomic_child_moves_without_losing_host_or_following_paragraph() {
    let mut d = document(wrapper("host"));
    child_mut(&mut d).page_break = TablePageBreak::None;
    let mut s = session(&d, 60.0);
    let first = s.next_page().unwrap().unwrap();
    assert_eq!(first.fragment.geometry().reserved_height(), 21.0);
    assert_eq!(
        rendered_lines(&first.tree)
            .iter()
            .map(|l| l.0.as_str())
            .collect::<Vec<_>>(),
        ["before"]
    );
    let second = s.next_page().unwrap().unwrap();
    assert_eq!(second.fragment.geometry().reserved_height(), 59.0); //child41+host18
    assert_eq!(
        rendered_lines(&second.tree)
            .iter()
            .map(|l| l.0.as_str())
            .collect::<Vec<_>>(),
        ["alpha", "beta", "host"]
    );
    let third = s.next_page().unwrap().unwrap();
    assert_eq!(third.fragment.geometry().reserved_height(), 22.0);
    assert_eq!(rendered_lines(&third.tree)[0].0, "after");
    assert!(s.next_page().unwrap().is_none());
}

#[test]
fn unsupported_anchor_forms_do_not_get_stacked_as_top_and_bottom() {
    for mode in 0..8 {
        let mut d = document(wrapper("host"));
        let child = child_mut(&mut d);
        match mode {
            0 => child.common.treat_as_char = true,
            1 => child.common.text_wrap = TextWrap::Square,
            2 => child.common.vertical_offset = 750,
            3 => child.common.horizontal_offset = 750,
            4 => child.common.vert_rel_to = VertRelTo::Page,
            5 => child.common.margin.bottom = 75,
            6 => child.border_fill_id = 1,
            _ => child.cells[0].paragraphs[0]
                .line_segs
                .push(LineSeg::default()),
        }
        assert!(
            matches!(
                TablePreviewSession::from_document(&d, SELECT, 96.0, pages(200.0), 10),
                Err(TablePreviewError::Geometry(GeometryError::Unsupported(_)))
            ),
            "mode {mode}"
        );
    }
    let mut d = document(wrapper("host"));
    root_mut(&mut d).cells[0].paragraphs[1]
        .controls
        .push(Control::Table(Box::new(leaf())));
    assert!(matches!(
        TablePreviewSession::from_document(&d, SELECT, 96.0, pages(200.0), 10),
        Err(TablePreviewError::Geometry(GeometryError::Unsupported(
            "multiple anchored cell controls"
        )))
    ));
    for mode in 0..4 {
        let mut d = document(wrapper("host"));
        let mut style = d.doc_info.para_shapes[0].clone();
        match mode {
            0 => style.margin_left = 150,
            1 => style.margin_right = 150,
            2 => style.indent = 150,
            _ => style.spacing_before = 150,
        }
        d.doc_info.para_shapes.push(style);
        root_mut(&mut d).cells[0].paragraphs[1].para_shape_id = 1;
        assert!(matches!(
            TablePreviewSession::from_document(&d, SELECT, 96.0, pages(200.0), 10),
            Err(TablePreviewError::Geometry(GeometryError::Unsupported(
                "anchored host paragraph insets"
            )))
        ));
    }
}

#[test]
fn fresh_nested_ir_budget_sweep_keeps_every_line_and_physical_extent() {
    let d = document(wrapper("host"));
    for budget in [
        18.0, 20.0, 22.0, 30.0, 41.0, 43.0, 60.0, 100.0, 102.0, 200.0,
    ] {
        let mut s = session(&d, budget);
        let mut all = Vec::new();
        let mut height = 0.0;
        while let Some(p) = s.next_page().unwrap() {
            let h = p.fragment.geometry().reserved_height();
            assert!(h <= budget);
            height += h;
            for (text, _, y, line_h) in rendered_lines(&p.tree) {
                assert!(y >= 30.0 && y + line_h <= 30.0 + h);
                all.push(text);
            }
        }
        assert_eq!(all, ["before", "alpha", "beta", "host", "after"]);
        assert_eq!(height, 102.0);
    }
}

#[test]
fn unsupported_deep_descendant_is_not_partially_published_or_mutated() {
    let mut d = document(wrapper("host"));
    child_mut(&mut d).cells[0].paragraphs[0]
        .controls
        .push(Control::Table(Box::new(leaf())));
    // Third level is a stored host, not fresh IR. It must not be silently rebuilt.
    child_mut(&mut d).cells[0].paragraphs[0]
        .line_segs
        .push(LineSeg {
            vertical_pos: 123,
            ..Default::default()
        });
    let before = format!("{d:?}");
    let styles = resolve_styles(&d.doc_info, 96.0);
    assert!(PreparedTextTable::prepare(root_mut(&mut d), &styles, 96.0).is_err());
    assert_eq!(format!("{d:?}"), before);
}

#[test]
fn serialized_hwpx_without_stored_rows_reopens_through_the_same_v2_boundary() {
    let d = document(wrapper("host"));
    let bytes = rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap();
    let reopened = rhwp::parser::hwpx::parse_hwpx(&bytes).unwrap();
    // Parsing may add section controls. Select by type, not a guessed index.
    let ci = reopened.sections[0].paragraphs[0]
        .controls
        .iter()
        .position(|c| matches!(c, Control::Table(_)))
        .unwrap();
    let mut actual = TablePreviewSession::from_document(
        &reopened,
        TableSelection {
            control: ci,
            ..SELECT
        },
        96.0,
        pages(43.0),
        100,
    )
    .unwrap();
    let mut expected = session(&d, 43.0);
    for height in [41.0, 39.0, 22.0] {
        let e = expected.next_page().unwrap().unwrap();
        let a = actual.next_page().unwrap().unwrap();
        assert_eq!(a.fragment.geometry().reserved_height(), height);
        assert_eq!(rendered_lines(&a.tree), rendered_lines(&e.tree));
        assert_eq!(
            a.fragment.geometry().placement(),
            e.fragment.geometry().placement()
        );
    }
    assert!(actual.next_page().unwrap().is_none());
    if let Ok(dir) = std::env::var("ISSUE7353_PREVIEW_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/fresh-nested.hwpx"), bytes).unwrap();
    }
}
