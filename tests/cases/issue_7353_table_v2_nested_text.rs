//! Explicit flow contracts, not saved IR anchors or a Hancom-output oracle.
use rhwp::{
    model::{
        paragraph::{CharShapeRef, ColumnBreakType, Paragraph},
        style::LineSpacingType,
    },
    renderer::{
        render_tree::{PageRenderTree, RenderNode, RenderNodeType},
        style_resolver::{ResolvedCharStyle, ResolvedParaStyle, ResolvedStyleSet},
        svg::SvgRenderer,
        table_v2::*,
    },
};

fn styles() -> ResolvedStyleSet {
    let mut s = ResolvedStyleSet::default();
    s.char_styles.push(ResolvedCharStyle {
        font_size: 12.0,
        ..Default::default()
    });
    s.para_styles.push(ResolvedParaStyle {
        line_spacing: 18.0,
        line_spacing_type: LineSpacingType::Fixed,
        ..Default::default()
    });
    s
}

fn paragraph(owner: usize, text: &str) -> TextFlowBlock {
    TextFlowBlock::Paragraph {
        owner,
        paragraph: Box::new(Paragraph {
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
        }),
    }
}

fn child(table: PreparedTextTable) -> TextFlowBlock {
    TextFlowBlock::Table {
        owner: ControlOwner {
            paragraph: 1,
            control: 0,
        },
        table,
    }
}

fn prepare(
    width: f64,
    padding: Insets,
    blocks: Vec<TextFlowBlock>,
    policy: SplitPolicy,
) -> PreparedTextTable {
    PreparedTextTable::from_flow_rows(
        vec![width],
        vec![TextFlowRow {
            cells: vec![TextFlowCell {
                padding,
                minimum_height: 0.0,
                blocks,
            }],
        }],
        0.0,
        policy,
        &styles(),
        7200.0,
    )
    .unwrap()
}

// An explicit physical Space is additive, including the zero identity. This
// contract does not choose whether a terminal line owns its next-line gap.
fn terminal_flow_observation(extra: Option<f64>) -> (f64, Vec<(String, f64, f64)>) {
    let mut blocks = vec![paragraph(0, "last")];
    if let Some(height) = extra {
        blocks.push(TextFlowBlock::Space(height));
    }
    let prepared = prepare(100.0, Insets::default(), blocks, SplitPolicy::Never);
    let fragment = placed(&prepared.start(), area(20.0, 30.0, 100.0));
    let page = render(&fragment);
    let mut lines = Vec::new();
    let mut tables = Vec::new();
    nodes(&page.root, &mut lines, &mut tables);
    (
        fragment.geometry().reserved_height(),
        lines
            .iter()
            .map(|n| (text(n), n.bbox.y, n.bbox.height))
            .collect(),
    )
}

#[test]
fn zero_trailing_space_is_an_identity_for_cell_flow() {
    assert_eq!(
        terminal_flow_observation(None),
        terminal_flow_observation(Some(0.0))
    );
}

#[test]
fn explicit_terminal_space_adds_only_its_own_height() {
    let (height, lines) = terminal_flow_observation(None);
    for extra in [2.0, 7.0] {
        let (with_space, shifted_lines) = terminal_flow_observation(Some(extra));
        assert_eq!(
            shifted_lines, lines,
            "physical tail does not move preceding lines"
        );
        assert_eq!(
            with_space - height,
            extra,
            "tail must not switch paragraph composition"
        );
    }
}

fn leaf(policy: SplitPolicy) -> PreparedTextTable {
    prepare(
        100.0,
        Insets {
            left: 2.0,
            right: 3.0,
            top: 2.0,
            bottom: 3.0,
        },
        vec![paragraph(0, "alpha"), paragraph(1, "beta")],
        policy,
    )
}

fn wrapper(child_table: PreparedTextTable) -> PreparedTextTable {
    prepare(
        120.0,
        Insets {
            left: 5.0,
            right: 7.0,
            top: 3.0,
            bottom: 4.0,
        },
        vec![
            paragraph(0, "before"),
            child(child_table),
            paragraph(2, "after"),
        ],
        SplitPolicy::WithinCells,
    )
}

fn area(x: f64, y: f64, height: f64) -> PageArea {
    PageArea {
        bounds: Rect {
            x,
            y,
            width: 300.0,
            height,
        },
    }
}

fn placed(cursor: &TextTableCursor, area: PageArea) -> TextFragment {
    match cursor.fit(area).unwrap() {
        TextFragmentFit::Placed(f) => f,
        _ => panic!("expected fragment"),
    }
}

fn nodes<'a>(
    node: &'a RenderNode,
    lines: &mut Vec<&'a RenderNode>,
    tables: &mut Vec<&'a RenderNode>,
) {
    match &node.node_type {
        RenderNodeType::TextLine(_) => lines.push(node),
        RenderNodeType::Table(_) => tables.push(node),
        _ => {}
    }
    for child in &node.children {
        nodes(child, lines, tables);
    }
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

fn render(f: &TextFragment) -> PageRenderTree {
    let mut page = PageRenderTree::new(0, 400.0, 180.0);
    f.append_to(&mut page).unwrap();
    page
}

fn svg(page: &PageRenderTree) -> String {
    let mut renderer = SvgRenderer::new();
    renderer.render_tree(page);
    renderer.output().to_string()
}

#[test]
fn nested_paint_uses_child_cuts_and_keeps_following_text_after_child() {
    let start = wrapper(leaf(SplitPolicy::WithinCells)).start();
    // Parent 3 top + before 18 + child (2 top + alpha 18) = 41.
    // Next page: beta 18 + child bottom 3 + after 18 + parent bottom 4 = 43.
    let first = placed(&start, area(20.0, 30.0, 43.0));
    assert_eq!(first.geometry().reserved_height(), 41.0);
    assert_eq!(
        placed(&start, area(20.0, 30.0, 43.0))
            .geometry()
            .placement(),
        first.geometry().placement()
    );
    let second = placed(&first.continuation(), area(40.0, 80.0, 43.0));
    assert_eq!(second.geometry().reserved_height(), 43.0);
    assert!(matches!(
        second.continuation().fit(area(0.0, 0.0, 43.0)).unwrap(),
        TextFragmentFit::Complete
    ));
    for (f, expected, positions, child_y, child_height) in [
        (
            &first,
            ["before", "alpha"],
            [(25.0, 33.0), (27.0, 53.0)],
            51.0,
            20.0,
        ),
        (
            &second,
            ["beta", "after"],
            [(47.0, 80.0), (45.0, 101.0)],
            80.0,
            21.0,
        ),
    ] {
        let page = render(f);
        let (mut lines, mut tables) = (Vec::new(), Vec::new());
        nodes(&page.root, &mut lines, &mut tables);
        assert_eq!(lines.iter().map(|l| text(l)).collect::<Vec<_>>(), expected);
        assert_eq!(tables.len(), 2);
        assert_eq!(tables[0].bbox.height, f.geometry().reserved_height());
        assert_eq!(
            (tables[1].bbox.y, tables[1].bbox.height),
            (child_y, child_height)
        );
        for (line, (x, y)) in lines.iter().zip(positions) {
            assert_eq!((line.bbox.x, line.bbox.y, line.bbox.height), (x, y, 12.0));
            assert!(line.bbox.y + line.bbox.height <= tables[0].bbox.y + tables[0].bbox.height);
        }
        let output = svg(&page);
        let xml = roxmltree::Document::parse(&output).unwrap();
        let texts: String = xml
            .descendants()
            .filter(|n| n.has_tag_name("text"))
            .map(|n| {
                n.descendants()
                    .filter(|n| n.is_text())
                    .filter_map(|n| n.text())
                    .collect::<String>()
            })
            .collect();
        assert_eq!(texts, expected.join(""));
    }
    if let Ok(dir) = std::env::var("ISSUE7353_PREVIEW_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/nested-first.svg"), svg(&render(&first))).unwrap();
        std::fs::write(
            format!("{dir}/nested-continuation.svg"),
            svg(&render(&second)),
        )
        .unwrap();
    }
}

#[test]
fn atomic_child_nonfit_does_not_consume_payload_or_parent_following_text() {
    let start = wrapper(leaf(SplitPolicy::Never)).start();
    let first = placed(&start, area(0.0, 0.0, 60.0));
    assert_eq!(first.geometry().reserved_height(), 21.0);
    assert!(first.geometry().placement().cells[0].tables.is_empty());
    let next = first.continuation();
    assert!(matches!(
        next.fit(area(0.0, 0.0, 40.0)).unwrap(),
        TextFragmentFit::DoesNotFit {
            required_height: 41.0,
            ..
        }
    ));
    let second = placed(&next, area(0.0, 0.0, 63.0));
    let page = render(&second);
    let (mut lines, mut tables) = (Vec::new(), Vec::new());
    nodes(&page.root, &mut lines, &mut tables);
    assert_eq!(
        lines.iter().map(|l| text(l)).collect::<Vec<_>>(),
        ["alpha", "beta", "after"]
    );
    assert_eq!(second.geometry().reserved_height(), 63.0);
    assert_eq!((tables[1].bbox.y, tables[1].bbox.height), (0.0, 41.0));
    assert_eq!(lines[2].bbox.y, 41.0);
}

#[test]
fn three_levels_keep_page_origins_and_physical_tail_without_duplicate_text() {
    let middle = wrapper(leaf(SplitPolicy::WithinCells));
    let outer = prepare(
        140.0,
        Insets {
            left: 4.0,
            top: 1.0,
            bottom: 2.0,
            right: 0.0,
        },
        vec![child(middle)],
        SplitPolicy::WithinCells,
    );
    let first = placed(&outer.start(), area(10.0, 20.0, 42.0));
    let second = placed(&first.continuation(), area(30.0, 40.0, 45.0));
    assert_eq!(
        first.geometry().reserved_height() + second.geometry().reserved_height(),
        87.0
    );
    for (f, coords) in [
        (&first, [(19.0, 24.0), (21.0, 44.0)]),
        (&second, [(41.0, 40.0), (39.0, 61.0)]),
    ] {
        let page = render(f);
        let (mut lines, mut tables) = (Vec::new(), Vec::new());
        nodes(&page.root, &mut lines, &mut tables);
        assert_eq!(tables.len(), 3);
        assert_eq!(
            lines
                .iter()
                .map(|n| (n.bbox.x, n.bbox.y))
                .collect::<Vec<_>>(),
            coords
        );
        assert_eq!(tables[2].bbox.x, tables[1].bbox.x + 5.0);
    }
    assert!(matches!(
        second.continuation().fit(area(0.0, 0.0, 100.0)).unwrap(),
        TextFragmentFit::Complete
    ));
}

#[test]
fn blank_paragraph_and_minimum_physical_band_are_not_lost_after_child() {
    let mut rows = vec![TextFlowRow {
        cells: vec![TextFlowCell {
            padding: Insets::default(),
            minimum_height: 70.0,
            blocks: vec![child(leaf(SplitPolicy::WithinCells)), paragraph(2, "")],
        }],
    }];
    let prepared = PreparedTextTable::from_flow_rows(
        vec![120.0],
        std::mem::take(&mut rows),
        0.0,
        SplitPolicy::WithinCells,
        &styles(),
        7200.0,
    )
    .unwrap();
    let first = placed(&prepared.start(), area(0.0, 0.0, 59.0));
    let page = render(&first);
    let (mut lines, mut tables) = (Vec::new(), Vec::new());
    nodes(&page.root, &mut lines, &mut tables);
    assert_eq!(
        lines.iter().map(|n| text(n)).collect::<Vec<_>>(),
        ["alpha", "beta", ""]
    );
    // 12px glyph line + 6px spacing = the declared fixed 18px advance.
    assert_eq!((lines[2].bbox.y, lines[2].bbox.height), (41.0, 12.0));
    let tail = placed(&first.continuation(), area(0.0, 0.0, 11.0));
    assert_eq!(tail.geometry().reserved_height(), 11.0);
    let page = render(&tail);
    let (mut lines, mut tables) = (Vec::new(), Vec::new());
    nodes(&page.root, &mut lines, &mut tables);
    assert!(lines.is_empty());
    assert_eq!(tables.len(), 1);
    assert!(matches!(
        tail.continuation().fit(area(0.0, 0.0, 100.0)).unwrap(),
        TextFragmentFit::Complete
    ));
}

#[test]
fn sibling_cells_scope_identical_control_owners_and_allocate_unique_ids() {
    let prepared = PreparedTextTable::from_flow_rows(
        vec![120.0, 120.0],
        vec![TextFlowRow {
            cells: ["left", "right"]
                .into_iter()
                .map(|label| TextFlowCell {
                    padding: Insets::default(),
                    minimum_height: 0.0,
                    blocks: vec![child(prepare(
                        100.0,
                        Insets::default(),
                        vec![paragraph(0, label)],
                        SplitPolicy::Never,
                    ))],
                })
                .collect(),
        }],
        0.0,
        SplitPolicy::WithinCells,
        &styles(),
        7200.0,
    )
    .unwrap();
    let f = placed(&prepared.start(), area(10.0, 20.0, 18.0));
    let mut page = render(&f);
    f.append_to(&mut page).unwrap();
    let (mut lines, mut tables) = (Vec::new(), Vec::new());
    nodes(&page.root, &mut lines, &mut tables);
    assert_eq!(
        lines.iter().map(|l| text(l)).collect::<Vec<_>>(),
        ["left", "right", "left", "right"]
    );
    assert_eq!((lines[0].bbox.x, lines[1].bbox.x), (10.0, 130.0));
    fn ids(n: &RenderNode, seen: &mut std::collections::HashSet<u32>) {
        assert!(seen.insert(n.id));
        for child in &n.children {
            ids(child, seen);
        }
    }
    let mut seen = std::collections::HashSet::new();
    for root in &page.root.children {
        ids(root, &mut seen);
    }
}

#[test]
fn invalid_flow_is_rejected_without_legacy_fallback() {
    let run = |blocks, dpi| {
        PreparedTextTable::from_flow_rows(
            vec![120.0],
            vec![TextFlowRow {
                cells: vec![TextFlowCell {
                    padding: Insets::default(),
                    minimum_height: 0.0,
                    blocks,
                }],
            }],
            0.0,
            SplitPolicy::WithinCells,
            &styles(),
            dpi,
        )
    };
    assert!(matches!(
        run(vec![paragraph(0, "a"), paragraph(0, "b")], 7200.0),
        Err(GeometryError::Unsupported("duplicate paragraph owner"))
    ));
    assert!(matches!(
        run(
            vec![
                child(leaf(SplitPolicy::Never)),
                child(leaf(SplitPolicy::Never))
            ],
            7200.0
        ),
        Err(GeometryError::Unsupported("duplicate table owner"))
    ));
    assert!(matches!(
        run(vec![child(leaf(SplitPolicy::Never))], 96.0),
        Err(GeometryError::Unsupported("nested text flow DPI mismatch"))
    ));
    let mut p = paragraph(0, "break");
    if let TextFlowBlock::Paragraph { paragraph, .. } = &mut p {
        paragraph.column_type = ColumnBreakType::Page;
    }
    assert!(run(vec![p], 7200.0).is_err());
    assert!(run(vec![TextFlowBlock::Space(f64::NAN)], 7200.0).is_err());
}

#[test]
fn flat_ir_child_snapshot_can_be_nested_without_recomposing_or_aliasing() {
    use rhwp::model::table::{Cell, Table, TablePageBreak};
    let TextFlowBlock::Paragraph { paragraph, .. } = paragraph(0, "snapshot") else {
        unreachable!()
    };
    let mut source = Table {
        row_count: 1,
        col_count: 1,
        page_break: TablePageBreak::CellBreak,
        cells: vec![Cell {
            width: 100,
            row_span: 1,
            col_span: 1,
            paragraphs: vec![*paragraph],
            ..Default::default()
        }],
        ..Default::default()
    };
    let prepared = PreparedTextTable::prepare(&source, &styles(), 7200.0).unwrap();
    source.cells[0].paragraphs[0].text = "changed".into();
    let outer = wrapper(prepared);
    let f = placed(&outer.start(), area(20.0, 30.0, 61.0));
    // 3 top + before 18 + snapshot 18 + after 18 + 4 bottom.
    assert_eq!(f.geometry().reserved_height(), 61.0);
    let page = render(&f);
    let (mut lines, mut tables) = (Vec::new(), Vec::new());
    nodes(&page.root, &mut lines, &mut tables);
    assert_eq!(
        lines.iter().map(|n| text(n)).collect::<Vec<_>>(),
        ["before", "snapshot", "after"]
    );
    assert_eq!(lines[1].bbox.y, 51.0);
    assert_eq!(tables[1].bbox.height, 18.0);
}

#[test]
fn all_fit_budgets_preserve_exact_text_and_padding_until_complete() {
    let prepared = wrapper(leaf(SplitPolicy::WithinCells));
    for budget in [
        18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 40.0, 41.0, 42.0, 43.0, 43.5, 60.0, 63.0, 83.0, 84.0,
        100.0,
    ] {
        let mut cursor = prepared.start();
        let mut total_height = 0.0;
        let mut content = Vec::new();
        let mut complete = false;
        for page_index in 0..20 {
            let page_area = area(10.0 + page_index as f64, 20.0, budget);
            match cursor.fit(page_area).unwrap() {
                TextFragmentFit::Complete => {
                    complete = true;
                    break;
                }
                TextFragmentFit::DoesNotFit { .. } => panic!("18px line must fit budget {budget}"),
                TextFragmentFit::Placed(f) => {
                    let h = f.geometry().reserved_height();
                    assert!(h > 0.0 && h <= budget);
                    total_height += h;
                    let page = render(&f);
                    let (mut lines, mut tables) = (Vec::new(), Vec::new());
                    nodes(&page.root, &mut lines, &mut tables);
                    for n in &tables {
                        assert!(n.bbox.y >= 20.0 && n.bbox.y + n.bbox.height <= 20.0 + h);
                    }
                    for n in lines {
                        assert!(n.bbox.y >= 20.0 && n.bbox.y + n.bbox.height <= 20.0 + h);
                        content.push(text(n));
                    }
                    cursor = f.continuation();
                }
            }
        }
        assert!(complete, "budget {budget} did not terminate");
        assert_eq!(total_height, 84.0, "budget {budget}");
        assert_eq!(
            content,
            ["before", "alpha", "beta", "after"],
            "budget {budget}"
        );
    }
}

#[test]
fn between_row_child_continuation_uses_the_next_rows_payload_and_padding() {
    let rows = ["row-one", "row-two"]
        .into_iter()
        .map(|label| TextFlowRow {
            cells: vec![TextFlowCell {
                padding: Insets {
                    left: 2.0,
                    right: 3.0,
                    top: 2.0,
                    bottom: 3.0,
                },
                minimum_height: 0.0,
                // Paragraph/line IDs intentionally coincide across rows.
                blocks: vec![paragraph(0, label)],
            }],
        })
        .collect();
    let inner = PreparedTextTable::from_flow_rows(
        vec![100.0],
        rows,
        0.0,
        SplitPolicy::BetweenRows,
        &styles(),
        7200.0,
    )
    .unwrap();
    let outer = wrapper(inner);
    let first = placed(&outer.start(), area(20.0, 30.0, 44.0));
    let second = placed(&first.continuation(), area(20.0, 30.0, 45.0));
    // Each child row is 2 + 18 + 3 = 23; parent is 3 + 18 + 46 + 18 + 4 = 89.
    assert_eq!(first.geometry().reserved_height(), 44.0);
    assert_eq!(second.geometry().reserved_height(), 45.0);
    for (f, expected, child_row, line_y) in [
        (&first, ["before", "row-one"], 0, [33.0, 53.0]),
        (&second, ["row-two", "after"], 1, [32.0, 53.0]),
    ] {
        let page = render(f);
        let (mut lines, mut tables) = (Vec::new(), Vec::new());
        nodes(&page.root, &mut lines, &mut tables);
        assert_eq!(lines.iter().map(|n| text(n)).collect::<Vec<_>>(), expected);
        assert_eq!(lines.iter().map(|n| n.bbox.y).collect::<Vec<_>>(), line_y);
        assert_eq!(tables[1].bbox.height, 23.0);
        let RenderNodeType::TableCell(cell) = &tables[1].children[0].node_type else {
            panic!("missing cell");
        };
        assert_eq!(cell.row, child_row);
    }
    assert!(matches!(
        second.continuation().fit(area(0.0, 0.0, 100.0)).unwrap(),
        TextFragmentFit::Complete
    ));
}
