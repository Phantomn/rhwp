//! Fresh synthetic IR contracts, not a stored-document or Hancom fidelity oracle.
use rhwp::model::{
    paragraph::{CharShapeRef, LineSeg, Paragraph},
    table::{Cell, Table, TablePageBreak},
    Padding,
};
use rhwp::renderer::{
    render_tree::{PageRenderTree, RenderNode, RenderNodeType},
    style_resolver::{ResolvedCharStyle, ResolvedParaStyle, ResolvedStyleSet},
    svg::SvgRenderer,
    table_v2::*,
};

fn styles() -> ResolvedStyleSet {
    let mut s = ResolvedStyleSet::default();
    s.char_styles.push(ResolvedCharStyle {
        font_size: 12.0,
        ..Default::default()
    });
    s.para_styles.push(ResolvedParaStyle {
        line_spacing: 18.0,
        line_spacing_type: rhwp::model::style::LineSpacingType::Fixed,
        ..Default::default()
    });
    s
}

fn para(text: &str) -> Paragraph {
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
        padding: Padding {
            left: 5,
            right: 7,
            top: 3,
            bottom: 4,
        },
        cells: vec![Cell {
            width: 212,
            row_span: 1,
            col_span: 1,
            paragraphs: texts.iter().map(|s| para(s)).collect(),
            ..Default::default()
        }],
        ..Default::default()
    }
}

// dpi=7200 deliberately makes one source HU one layout pixel. Pixel style sizes
// are independent input: fixed line spacing declares an 18px start-to-start interval.
fn area(height: f64) -> PageArea {
    PageArea {
        bounds: Rect {
            x: 20.0,
            y: 30.0,
            width: 212.0,
            height,
        },
    }
}

fn placed(cursor: &TextTableCursor, height: f64) -> TextFragment {
    match cursor.fit(area(height)).unwrap() {
        TextFragmentFit::Placed(f) => f,
        _ => panic!("expected a placed fragment"),
    }
}

fn collect_lines(node: &RenderNode, out: &mut Vec<RenderNode>) {
    if matches!(node.node_type, RenderNodeType::TextLine(_)) {
        out.push(node.clone());
    }
    for child in &node.children {
        collect_lines(child, out);
    }
}

fn render(fragment: &TextFragment) -> (PageRenderTree, Vec<RenderNode>) {
    let mut page = PageRenderTree::new(0, 500.0, 300.0);
    fragment.append_to(&mut page).unwrap();
    let mut lines = Vec::new();
    collect_lines(&page.root, &mut lines);
    (page, lines)
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

#[test]
fn text_payload_and_reserved_fragment_have_identical_final_coordinates() {
    let prepared =
        PreparedTextTable::prepare(&table(&["alpha", "beta"]), &styles(), 7200.0).unwrap();
    let fragment = placed(&prepared.start(), 200.0);
    let (page, lines) = render(&fragment);
    assert_eq!(lines.len(), 2);
    assert_eq!(text(&lines[0]), "alpha");
    assert_eq!(text(&lines[1]), "beta");
    assert_eq!(lines[0].bbox.x, 25.0);
    assert_eq!(lines[0].bbox.y, 33.0);
    assert_eq!(lines[1].bbox.y - lines[0].bbox.y, 18.0);
    assert_eq!(
        fragment.geometry().reserved_height(),
        3.0 + 18.0 * 2.0 + 4.0
    );
    let placements = &fragment.geometry().placement().cells[0].lines;
    for (node, placement) in lines.iter().zip(placements) {
        assert_eq!(node.bbox.x, placement.bounds.x);
        assert_eq!(node.bbox.y, placement.bounds.y);
        assert_eq!(node.bbox.height, placement.bounds.height);
    }
    assert_eq!(
        page.root.children[0].bbox.height,
        fragment.geometry().reserved_height()
    );
    let mut svg = SvgRenderer::new();
    svg.render_tree(&page);
    let xml = roxmltree::Document::parse(svg.output()).unwrap();
    let painted: String = xml
        .descendants()
        .filter(|n| n.has_tag_name("text"))
        .map(|n| {
            n.descendants()
                .filter(|n| n.is_text())
                .filter_map(|n| n.text())
                .collect::<String>()
        })
        .collect();
    assert_eq!(painted, "alphabeta", "{}", svg.output());
}

#[test]
fn blank_line_and_paragraph_spacing_are_physical_content() {
    let mut s = styles();
    s.para_styles[0].spacing_before = 2.0;
    s.para_styles[0].spacing_after = 3.0;
    let prepared = PreparedTextTable::prepare(&table(&["A", "", "B"]), &s, 7200.0).unwrap();
    let f = placed(&prepared.start(), 200.0);
    let (_, lines) = render(&f);
    assert_eq!(lines.len(), 3);
    assert_eq!(text(&lines[1]), "");
    assert_eq!(lines[0].bbox.y, 35.0);
    assert_eq!(lines[1].bbox.y - lines[0].bbox.y, 23.0);
    assert_eq!(lines[2].bbox.y - lines[1].bbox.y, 23.0);
    assert_eq!(f.geometry().reserved_height(), 3.0 + 3.0 * 23.0 + 4.0);
}

#[test]
fn continuations_emit_every_prepared_line_once_without_recomposition() {
    let mut source = table(&["alpha", "beta", "gamma"]);
    let mut s = styles();
    let prepared = PreparedTextTable::prepare(&source, &s, 7200.0).unwrap();
    // Mutating input after preparation cannot change an existing session snapshot.
    source.cells[0].paragraphs.clear();
    s.char_styles[0].font_size = 70.0;
    let initial = prepared.start();
    let a = placed(&initial, 21.0);
    let again = placed(&initial, 21.0);
    assert_eq!(a.geometry().placement(), again.geometry().placement());
    let b = placed(&a.continuation(), 18.0);
    let c = placed(&b.continuation(), 22.0);
    let fragments = [a, b, c];
    let mut all = Vec::new();
    let mut reserved = 0.0;
    for f in &fragments {
        let (_, lines) = render(f);
        assert_eq!(lines.len(), 1);
        all.push(text(&lines[0]));
        reserved += f.geometry().reserved_height();
        for run in &lines[0].children {
            if let RenderNodeType::TextRun(r) = &run.node_type {
                assert_eq!(r.style.font_size, 12.0);
            }
        }
    }
    assert_eq!(all, ["alpha", "beta", "gamma"]);
    assert_eq!(reserved, 3.0 + 3.0 * 18.0 + 4.0);
    assert!(matches!(
        fragments[2].continuation().fit(area(200.0)).unwrap(),
        TextFragmentFit::Complete
    ));
}

#[test]
fn saved_rows_and_objects_are_not_silently_reflowed_or_fallen_back() {
    let mut source = table(&["a"]);
    source.cells[0].paragraphs[0]
        .line_segs
        .push(LineSeg::default());
    assert!(matches!(
        PreparedTextTable::prepare(&source, &styles(), 96.0),
        Err(GeometryError::Unsupported(_))
    ));
    source.cells[0].paragraphs[0].line_segs.clear();
    source.cells[0].paragraphs[0]
        .controls
        .push(rhwp::model::control::Control::Table(Box::new(table(&[
            "child",
        ]))));
    assert!(matches!(
        PreparedTextTable::prepare(&source, &styles(), 96.0),
        Err(GeometryError::Unsupported(_))
    ));
}

#[test]
fn width_and_explicit_breaks_change_prepared_lines_not_fragment_paint() {
    let wide = table(&["가나다라마바사아자차카타파하"]);
    let mut narrow = wide.clone();
    narrow.cells[0].width = 60;
    let s = styles();
    let w = PreparedTextTable::prepare(&wide, &s, 7200.0).unwrap();
    let n = PreparedTextTable::prepare(&narrow, &s, 7200.0).unwrap();
    let (_, wl) = render(&placed(&w.start(), 250.0));
    let (_, nl) = render(&placed(&n.start(), 250.0));
    assert!(nl.len() > wl.len());
    assert_eq!(
        nl.iter().map(text).collect::<String>(),
        wide.cells[0].paragraphs[0].text
    );
    let hard = PreparedTextTable::prepare(&table(&["A\nB"]), &s, 7200.0).unwrap();
    let (_, lines) = render(&placed(&hard.start(), 200.0));
    assert_eq!(lines.len(), 2);
}

#[test]
fn atomic_nonfit_preserves_payload_and_cell_order_is_not_storage_order() {
    let mut source = table(&["left"]);
    source.page_break = TablePageBreak::None;
    source.col_count = 2;
    let mut right = source.cells[0].clone();
    right.col = 1;
    right.paragraphs = vec![para("right")];
    source.cells.insert(0, right);
    let prepared = PreparedTextTable::prepare(&source, &styles(), 7200.0).unwrap();
    let cursor = prepared.start();
    let room = |height| PageArea {
        bounds: Rect {
            width: 424.0,
            height,
            ..area(0.0).bounds
        },
    };
    assert!(matches!(
        cursor.fit(room(10.0)).unwrap(),
        TextFragmentFit::DoesNotFit { .. }
    ));
    let TextFragmentFit::Placed(f) = cursor.fit(room(100.0)).unwrap() else {
        panic!("fit")
    };
    let (mut page, lines) = render(&f);
    assert_eq!(
        lines.iter().map(text).collect::<Vec<_>>(),
        ["left", "right"]
    );
    assert_eq!(lines[1].bbox.x - lines[0].bbox.x, 212.0);
    // A second append receives fresh page-local IDs without modifying either payload.
    f.append_to(&mut page).unwrap();
    fn ids(n: &RenderNode, seen: &mut std::collections::HashSet<u32>) {
        assert!(seen.insert(n.id));
        for child in &n.children {
            ids(child, seen);
        }
    }
    ids(&page.root, &mut std::collections::HashSet::new());
}

#[test]
fn four_point_empty_paragraph_uses_156_percent_pitch_at_normal_dpi() {
    let mut source = table(&["A", "", "B"]);
    source.cells[0].width *= 75;
    source.padding = Padding {
        left: 375,
        right: 525,
        top: 225,
        bottom: 300,
    };
    let mut s = styles();
    s.char_styles[0].font_size = 4.0 * 96.0 / 72.0;
    s.para_styles[0].line_spacing = 156.0;
    s.para_styles[0].line_spacing_type = rhwp::model::style::LineSpacingType::Percent;
    let prepared = PreparedTextTable::prepare(&source, &s, 96.0).unwrap();
    let fragment = placed(&prepared.start(), 200.0);
    let (_, lines) = render(&fragment);
    assert_eq!(lines.len(), 3);
    // Declared 400HU em + 224HU (56%) gap = 624HU = 8.32px at 96dpi.
    let pitch = 624.0 * 96.0 / 7200.0;
    assert!((lines[1].bbox.y - lines[0].bbox.y - pitch).abs() < 1e-9);
    assert!((lines[2].bbox.y - lines[1].bbox.y - pitch).abs() < 1e-9);
    assert!((fragment.geometry().reserved_height() - (3.0 + 3.0 * pitch + 4.0)).abs() < 1e-9);
}
