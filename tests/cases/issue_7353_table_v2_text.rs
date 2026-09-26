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

#[test]
fn blank_stored_row_consumes_once_when_following_inline_table_defers() {
    use rhwp::{model::control::Control, renderer::style_resolver::resolve_styles};
    let d = rhwp::parse_document(include_bytes!(
        "../fixtures/issue7353_tac_blank_row_review/space-row-saved.hwp"
    ))
    .unwrap();
    let Control::Table(t) = d.sections[0].paragraphs[0]
        .controls
        .iter()
        .find(|c| matches!(c, Control::Table(_)))
        .unwrap()
    else {
        panic!()
    };
    let mut t = t.clone();
    // Synthetic budget boundary using the independent saved row boxes.
    t.page_break = TablePageBreak::CellBreak;
    let s = resolve_styles(&d.doc_info, 96.0);
    let p = PreparedTextTable::prepare_with_end_policy(
        &t,
        &s,
        96.0,
        &d.bin_data_content,
        CellEndPolicy::OmitFinalParagraphGap,
    )
    .unwrap();
    let bounds = |height| PageArea {
        bounds: Rect {
            x: 20.0,
            y: 30.0,
            width: 700.0,
            height,
        },
    };
    let cursor = p.start();
    // Existing cell flow may consume top padding alone. It must not consume
    // the no-ink line when its complete occupied box is one HU too tall.
    let TextFragmentFit::Placed(padding) =
        cursor.fit(bounds((283.0 + 1400.0 - 1.0) / 75.0)).unwrap()
    else {
        panic!()
    };
    assert!(render(&padding).1.is_empty());
    let TextFragmentFit::Placed(retry) = padding.continuation().fit(bounds(400.0)).unwrap() else {
        panic!()
    };
    let (_, retry_lines) = render(&retry);
    assert_eq!(
        retry_lines.iter().map(text).collect::<Vec<_>>(),
        [" ".repeat(60), "TABLE ON SECOND LINE".into()]
    );
    let TextFragmentFit::Placed(first) = cursor
        .fit(bounds((283.0 + 2116.0 + 14847.0 - 1.0) / 75.0))
        .unwrap()
    else {
        panic!()
    };
    let (tree, lines) = render(&first);
    assert_eq!(lines.len(), 1);
    assert_eq!(text(&lines[0]), " ".repeat(60));
    assert!((lines[0].bbox.y - (30.0 + 283.0 / 75.0)).abs() < 1e-9);
    let mut first_tables = Vec::new();
    visit(&tree.root, &mut first_tables);
    assert_eq!(first_tables.len(), 1, "the child must defer intact");
    let next = first.continuation();
    let TextFragmentFit::Placed(second) = next.fit(bounds(400.0)).unwrap() else {
        panic!()
    };
    let (tree, lines) = render(&second);
    assert_eq!(
        lines.iter().map(text).collect::<Vec<_>>(),
        ["TABLE ON SECOND LINE"]
    );
    let mut tables = Vec::new();
    fn visit<'a>(n: &'a RenderNode, out: &mut Vec<&'a RenderNode>) {
        if matches!(n.node_type, RenderNodeType::Table(_)) {
            out.push(n);
        }
        for child in &n.children {
            visit(child, out);
        }
    }
    visit(&tree.root, &mut tables);
    assert_eq!(tables.len(), 2);
    assert!(
        (tables[1].bbox.y - 30.0).abs() < 1e-9,
        "space row must not repeat"
    );
    assert!(matches!(
        second.continuation().fit(bounds(400.0)).unwrap(),
        TextFragmentFit::Complete
    ));
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

#[test]
fn saved_minimum_cell_lane_preserves_blank_lines_cuts_and_physical_edges() {
    // Independent normal-save oracle: issue7353_narrow_cell_review has a1303HU
    // cell,510HU left/right pads and1440HU stored text width (not283HU).
    for own_padding in [false, true] {
        let mut t = table(&["A", "", "B"]);
        t.cells[0].width = 1303;
        t.padding = Padding {
            left: 510,
            right: 510,
            top: 75,
            bottom: 150,
        };
        t.cells[0].apply_inner_margin = own_padding;
        t.cells[0].padding = t.padding;
        if own_padding {
            t.padding = Padding::default();
        }
        for p in &mut t.cells[0].paragraphs {
            p.line_segs = vec![LineSeg {
                text_start: 0,
                vertical_pos: 0,
                line_height: 900,
                text_height: 900,
                baseline_distance: 765,
                line_spacing: 450,
                column_start: 0,
                segment_width: 1440,
                tag: LineSeg::TAG_SINGLE_SEGMENT_LINE,
            }];
        }
        let before = serde_json::to_value(&t).unwrap();
        let prepared = PreparedTextTable::prepare_with_end_policy(
            &t,
            &styles(),
            96.0,
            &[],
            CellEndPolicy::OmitFinalParagraphGap,
        )
        .unwrap();
        let page = |height| PageArea {
            bounds: Rect {
                x: 20.0,
                y: 30.0,
                width: 1303.0 / 75.0,
                height,
            },
        };
        let cursor = prepared.start();
        let TextFragmentFit::Placed(whole) = cursor.fit(page(100.0)).unwrap() else {
            panic!()
        };
        let (_, lines) = render(&whole);
        assert_eq!(lines.iter().map(text).collect::<Vec<_>>(), ["A", "", "B"]);
        for (i, l) in lines.iter().enumerate() {
            assert!((l.bbox.x - 26.8).abs() < 1e-9);
            assert!((l.bbox.width - 19.2).abs() < 1e-9);
            assert!((l.bbox.y - (31.0 + i as f64 * 18.0)).abs() < 1e-9);
            assert_eq!(l.bbox.height, 12.0);
        }
        assert!((whole.geometry().reserved_height() - 51.0).abs() < 1e-9);
        let mut cursor = prepared.start();
        let mut labels = Vec::new();
        for index in 0..3 {
            let TextFragmentFit::Placed(part) = cursor.fit(page(20.0)).unwrap() else {
                panic!()
            };
            let (tree, ls) = render(&part);
            assert_eq!(ls.len(), 1, "one full line fits per page");
            labels.push(text(&ls[0]));
            assert!((ls[0].bbox.x - 26.8).abs() < 1e-9);
            assert!((ls[0].bbox.y - (30.0 + if index == 0 { 1.0 } else { 0.0 })).abs() < 1e-9);
            let table = &tree.root.children[0];
            assert!((table.bbox.width - 1303.0 / 75.0).abs() < 1e-9);
            assert!((table.children[0].bbox.width - 1303.0 / 75.0).abs() < 1e-9);
            cursor = part.continuation();
        }
        assert_eq!(labels, ["A", "", "B"]);
        assert!(matches!(
            cursor.fit(page(20.0)).unwrap(),
            TextFragmentFit::Complete
        ));
        assert_eq!(serde_json::to_value(&t).unwrap(), before);
        // The observed minimum is a rule, not permission for arbitrary width.
        for width in [1439, 1441, 1600] {
            let mut bad = t.clone();
            for p in &mut bad.cells[0].paragraphs {
                p.line_segs[0].segment_width = width;
            }
            assert!(PreparedTextTable::prepare(&bad, &styles(), 96.0).is_err());
        }
        let mut mixed = t.clone();
        mixed.cells[0].paragraphs[1].line_segs.clear();
        assert!(PreparedTextTable::prepare(&mixed, &styles(), 96.0).is_err());
    }
}

#[test]
fn negative_stored_gap_keeps_line_boxes_and_continuation_ownership() {
    // Independent synthetic HU geometry: two 12-high rows start 10HU apart.
    // The second row cannot fit a 21HU budget although its advance ends at 20.
    let mut t = table(&["AB", "C"]);
    t.padding = Padding::default();
    let p = &mut t.cells[0].paragraphs[0];
    p.line_segs = (0..2)
        .map(|i| LineSeg {
            text_start: i,
            vertical_pos: i as i32 * 10,
            line_height: 12,
            text_height: 12,
            baseline_distance: 10,
            line_spacing: -2,
            segment_width: 212,
            tag: LineSeg::TAG_SINGLE_SEGMENT_LINE,
            ..Default::default()
        })
        .collect();
    let prepared = PreparedTextTable::prepare(&t, &styles(), 7200.0).unwrap();
    let first = placed(&prepared.start(), 21.0);
    let (_, first_lines) = render(&first);
    assert_eq!(first_lines.len(), 1);
    assert_eq!(text(&first_lines[0]), "A");
    assert_eq!(first_lines[0].bbox.height, 12.0);
    let second = placed(&first.continuation(), 40.0);
    let (_, second_lines) = render(&second);
    assert_eq!(
        second_lines.iter().map(text).collect::<Vec<_>>(),
        ["B", "C"]
    );
    assert_eq!(second_lines[0].bbox.y, 30.0);
    assert_eq!(second_lines[1].bbox.y, 40.0);
    assert!(matches!(
        second.continuation().fit(area(40.0)).unwrap(),
        TextFragmentFit::Complete
    ));
    let whole = placed(&prepared.start(), 40.0);
    let (_, lines) = render(&whole);
    assert_eq!(
        lines.iter().map(|n| n.bbox.y).collect::<Vec<_>>(),
        [30.0, 40.0, 50.0]
    );
    assert!(lines.iter().all(|n| n.bbox.height == 12.0));

    let mut invalid = t.clone();
    invalid.cells[0].paragraphs[0].line_segs[0].line_spacing = -13;
    assert!(PreparedTextTable::prepare(&invalid, &styles(), 7200.0).is_err());
    invalid = t.clone();
    invalid.cells[0].paragraphs[0].line_segs[1].vertical_pos = 9;
    assert!(PreparedTextTable::prepare(&invalid, &styles(), 7200.0).is_err());
}

#[test]
fn stored_zero_pitch_rows_preserve_source_partitions_at_same_origin() {
    let mut t = table(&["AB"]);
    t.padding = Padding::default();
    t.cells[0].paragraphs[0].line_segs = (0..2)
        .map(|i| LineSeg {
            text_start: i,
            line_height: 12,
            text_height: 12,
            baseline_distance: 10,
            line_spacing: -12,
            segment_width: 212,
            tag: LineSeg::TAG_SINGLE_SEGMENT_LINE,
            ..Default::default()
        })
        .collect();
    let prepared = PreparedTextTable::prepare(&t, &styles(), 7200.0).unwrap();
    let f = placed(&prepared.start(), 12.0);
    let (_, lines) = render(&f);
    assert_eq!(lines.iter().map(text).collect::<Vec<_>>(), ["A", "B"]);
    assert!(lines
        .iter()
        .all(|l| l.bbox.y == 30.0 && l.bbox.height == 12.0));
    assert_eq!(f.geometry().reserved_height(), 12.0);
    assert!(matches!(
        f.continuation().fit(area(12.0)).unwrap(),
        TextFragmentFit::Complete
    ));
}

#[test]
fn zero_pitch_cell_lines_keep_occupied_height_and_distinct_owners() {
    let mut t = table(&["A", "", "B"]);
    t.padding = Padding::default();
    for p in &mut t.cells[0].paragraphs {
        p.line_segs = vec![LineSeg {
            line_height: 12,
            text_height: 12,
            baseline_distance: 10,
            line_spacing: -12,
            segment_width: 212,
            tag: LineSeg::TAG_SINGLE_SEGMENT_LINE,
            ..Default::default()
        }];
    }
    for policy in [
        CellEndPolicy::PreserveAdvance,
        CellEndPolicy::OmitFinalLineGap,
        CellEndPolicy::OmitFinalParagraphGap,
    ] {
        let prepared =
            PreparedTextTable::prepare_with_end_policy(&t, &styles(), 7200.0, &[], policy).unwrap();
        assert!(matches!(
            prepared.start().fit(area(11.0)).unwrap(),
            TextFragmentFit::DoesNotFit { .. }
        ));
        let f = placed(&prepared.start(), 12.0);
        let (tree, lines) = render(&f);
        assert_eq!(lines.iter().map(text).collect::<Vec<_>>(), ["A", "", "B"]);
        assert!(lines
            .iter()
            .all(|l| l.bbox.y == 30.0 && l.bbox.height == 12.0));
        assert_eq!(tree.root.children[0].bbox.height, 12.0);
        assert_eq!(f.geometry().reserved_height(), 12.0);
        assert!(matches!(
            f.continuation().fit(area(12.0)).unwrap(),
            TextFragmentFit::Complete
        ));
    }
}

#[test]
fn fresh_negative_gap_preserves_blank_lines_and_terminal_after_spacing() {
    let mut s = styles();
    // 16HU at75% gives an exact -4HU gap, independent of HU quantization.
    s.char_styles[0].font_size = 16.0;
    s.para_styles[0].line_spacing_type = rhwp::model::style::LineSpacingType::Percent;
    s.para_styles[0].line_spacing = 75.0;
    s.para_styles[0].spacing_after = 3.0;
    let mut t = table(&["A", "", "B"]);
    t.padding = Padding::default();
    for policy in [
        CellEndPolicy::PreserveAdvance,
        CellEndPolicy::OmitFinalLineGap,
        CellEndPolicy::OmitFinalParagraphGap,
    ] {
        let prepared =
            PreparedTextTable::prepare_with_end_policy(&t, &s, 7200.0, &[], policy).unwrap();
        let fragment = placed(&prepared.start(), 60.0);
        let (_, lines) = render(&fragment);
        assert_eq!(lines.len(), 3);
        // 16HU glyph box, advance12HU + authored after3HU.
        for (n, y) in lines.iter().zip([30.0, 45.0, 60.0]) {
            assert!((n.bbox.y - y).abs() < 1e-7);
            assert_eq!(n.bbox.height, 16.0);
        }
        let expected = match policy {
            CellEndPolicy::PreserveAdvance | CellEndPolicy::OmitFinalParagraphGap => 46.0,
            CellEndPolicy::OmitFinalLineGap => 49.0,
        };
        assert!((fragment.geometry().reserved_height() - expected).abs() < 1e-7);
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
fn fresh_indent_preserves_breaks_blank_lines_and_fragment_coordinates() {
    use rhwp::model::style::Alignment;
    for indent in [0.0, 20.0, -20.0] {
        for alignment in [Alignment::Left, Alignment::Center, Alignment::Right] {
            let mut s = styles();
            let style = &mut s.para_styles[0];
            style.margin_left = 10.0;
            style.margin_right = 20.0;
            style.indent = indent;
            style.alignment = alignment;
            let t = table(&["A\n\nB", "", "after"]);
            let prepared = PreparedTextTable::prepare(&t, &s, 7200.0).unwrap();
            let whole = placed(&prepared.start(), 120.0);
            let (_, lines) = render(&whole);
            assert_eq!(
                lines.iter().map(text).collect::<Vec<_>>(),
                ["A", "", "B", "", "after"]
            );
            // Table x20 + cell left5 + paragraph left10. The three physical
            // lines of A/newline/newline/B belong to one paragraph; the blank
            // and after paragraphs each restart their own first-line state.
            for (i, line) in lines.iter().enumerate() {
                let first = i == 0 || i >= 3;
                let inset = if (indent > 0.0 && first) || (indent < 0.0 && !first) {
                    20.0
                } else {
                    0.0
                };
                assert_eq!(line.bbox.x, 35.0 + inset);
                assert_eq!(line.bbox.width, 170.0 - inset);
                assert_eq!(line.bbox.y, 33.0 + i as f64 * 18.0);
                assert_eq!(line.bbox.height, 12.0);
                if let Some(run) = line.children.first().filter(|_| !text(line).is_empty()) {
                    let expected = match alignment {
                        Alignment::Center => line.bbox.x + (line.bbox.width - run.bbox.width) / 2.0,
                        Alignment::Right => line.bbox.x + line.bbox.width - run.bbox.width,
                        _ => line.bbox.x,
                    };
                    assert!((run.bbox.x - expected).abs() < 1e-7);
                }
            }
            assert_eq!(whole.geometry().reserved_height(), 97.0);
            // First fragment owns only A. Continuation must not reindent its
            // explicit blank/B lines as a new paragraph, nor discard the blank.
            let first = placed(&prepared.start(), 21.0);
            assert_eq!(render(&first).1.iter().map(text).collect::<Vec<_>>(), ["A"]);
            let rest = placed(&first.continuation(), 90.0);
            let (_, continued) = render(&rest);
            assert_eq!(
                continued.iter().map(text).collect::<Vec<_>>(),
                ["", "B", "", "after"]
            );
            for (i, line) in continued.iter().enumerate() {
                assert_eq!(line.bbox.x, lines[i + 1].bbox.x);
                assert_eq!(line.bbox.width, lines[i + 1].bbox.width);
                assert_eq!(line.bbox.y, 30.0 + i as f64 * 18.0);
            }
            assert!(matches!(
                rest.continuation().fit(area(90.0)).unwrap(),
                TextFragmentFit::Complete
            ));
            let nested = PreparedTextTable::from_flow_rows(
                vec![232.0],
                vec![TextFlowRow {
                    cells: vec![TextFlowCell {
                        padding: Insets {
                            left: 10.0,
                            right: 10.0,
                            ..Default::default()
                        },
                        minimum_height: 0.0,
                        blocks: vec![TextFlowBlock::Table {
                            owner: ControlOwner {
                                paragraph: 0,
                                control: 0,
                            },
                            table: prepared,
                        }],
                    }],
                }],
                0.0,
                SplitPolicy::WithinCells,
                &s,
                7200.0,
            )
            .unwrap();
            let mut room = area(120.0);
            room.bounds.width = 232.0;
            let TextFragmentFit::Placed(f) = nested.start().fit(room).unwrap() else {
                panic!("nested fit")
            };
            let (_, nested_lines) = render(&f);
            assert_eq!(nested_lines.len(), lines.len());
            for (actual, expected) in nested_lines.iter().zip(&lines) {
                assert_eq!(text(actual), text(expected));
                assert_eq!(actual.bbox.x, expected.bbox.x + 10.0);
                assert_eq!(actual.bbox.y, expected.bbox.y);
                assert_eq!(actual.bbox.width, expected.bbox.width);
            }
        }
    }
}

#[test]
fn fresh_indent_wrapping_uses_available_width_and_rejects_empty_interval() {
    let text_source = "가나다라마바사아자차카타파하";
    for indent in [24.0, -24.0] {
        let mut s = styles();
        s.para_styles[0].alignment = rhwp::model::style::Alignment::Left;
        s.para_styles[0].indent = indent;
        let mut t = table(&[text_source]);
        t.cells[0].width = 72; // 60px inner box, 12px Hangul em; inset two ems.
        let prepared = PreparedTextTable::prepare(&t, &s, 7200.0).unwrap();
        let (_, lines) = render(&placed(&prepared.start(), 200.0));
        let expected = if indent > 0.0 {
            vec!["가나다", "라마바사아", "자차카타파", "하"]
        } else {
            vec!["가나다라마", "바사아", "자차카", "타파하"]
        };
        assert_eq!(lines.iter().map(text).collect::<Vec<_>>(), expected);
        assert_eq!(lines.iter().map(text).collect::<String>(), text_source);
        for (i, line) in lines.iter().enumerate() {
            let inset = if (indent > 0.0 && i == 0) || (indent < 0.0 && i > 0) {
                24.0
            } else {
                0.0
            };
            assert_eq!(line.bbox.x, 25.0 + inset);
            assert_eq!(line.bbox.width, 60.0 - inset);
        }
    }
    for indent in [200.0, -200.0, f64::NAN, f64::INFINITY] {
        let mut s = styles();
        s.para_styles[0].indent = indent;
        assert!(PreparedTextTable::prepare(&table(&["A\nB"]), &s, 7200.0).is_err());
    }
}

#[test]
fn terminal_policy_preserves_blank_line_and_external_space_in_both_adapters() {
    for policy in [
        CellEndPolicy::OmitFinalLineGap,
        CellEndPolicy::OmitFinalParagraphGap,
    ] {
        let expected_height = if policy == CellEndPolicy::OmitFinalLineGap {
            41.0
        } else {
            39.0
        };
        for stored in [false, true] {
            let mut t = table(&["before", ""]);
            if stored {
                for p in &mut t.cells[0].paragraphs {
                    p.line_segs = vec![LineSeg {
                        line_height: 12,
                        text_height: 12,
                        baseline_distance: 10,
                        line_spacing: 6,
                        segment_width: 200,
                        tag: LineSeg::TAG_SINGLE_SEGMENT_LINE,
                        ..Default::default()
                    }];
                }
            }
            let mut s = styles();
            s.para_styles[0].spacing_after = 2.0;
            let ir =
                PreparedTextTable::prepare_with_end_policy(&t, &s, 7200.0, &[], policy).unwrap();
            // padding3 + before(12+6+2) + blank12 + optional terminal after2 + padding4.
            // Only the terminal gap changes; the real blank and prior after2 remain.
            let f = placed(&ir.start(), expected_height);
            assert!(matches!(
                f.continuation().fit(area(41.0)).unwrap(),
                TextFragmentFit::Complete
            ));
            let (_, reference) = render(&f);
            assert_eq!(
                reference.iter().map(text).collect::<Vec<_>>(),
                ["before", ""]
            );
            assert_eq!(
                reference.iter().map(|l| l.bbox.y).collect::<Vec<_>>(),
                [33.0, 53.0]
            );
            assert_eq!(f.geometry().reserved_height(), expected_height);
            for extra in [0.0, 2.0, 7.0] {
                let mut blocks: Vec<_> = t.cells[0]
                    .paragraphs
                    .iter()
                    .enumerate()
                    .map(|(owner, p)| TextFlowBlock::Paragraph {
                        owner,
                        paragraph: Box::new(p.clone()),
                    })
                    .collect();
                blocks.push(TextFlowBlock::Space(extra));
                let flow = PreparedTextTable::from_flow_rows_with_end_policy(
                    vec![212.0],
                    vec![TextFlowRow {
                        cells: vec![TextFlowCell {
                            padding: Insets {
                                left: 5.0,
                                right: 7.0,
                                top: 3.0,
                                bottom: 4.0,
                            },
                            minimum_height: 0.0,
                            blocks,
                        }],
                    }],
                    0.0,
                    SplitPolicy::WithinCells,
                    &s,
                    7200.0,
                    policy,
                )
                .unwrap();
                let f = placed(&flow.start(), expected_height + extra);
                let (_, lines) = render(&f);
                let boxes = |nodes: &[RenderNode]| {
                    nodes
                        .iter()
                        .map(|l| (l.bbox.x, l.bbox.y, l.bbox.width, l.bbox.height))
                        .collect::<Vec<_>>()
                };
                assert_eq!(boxes(&lines), boxes(&reference));
                assert_eq!(f.geometry().reserved_height(), expected_height + extra);
                assert!(matches!(
                    f.continuation().fit(area(41.0)).unwrap(),
                    TextFragmentFit::Complete
                ));
            }
            // If only the preceding paragraph and the blank box do not fit, the
            // blank remains an owned line on the next fragment, not discarded.
            let first = placed(&ir.start(), 34.0);
            assert_eq!(
                render(&first).1.iter().map(text).collect::<Vec<_>>(),
                ["before"]
            );
            let next = placed(&first.continuation(), 41.0);
            assert_eq!(render(&next).1.iter().map(text).collect::<Vec<_>>(), [""]);
            assert!(matches!(
                next.continuation().fit(area(41.0)).unwrap(),
                TextFragmentFit::Complete
            ));
        }
    }
}

#[test]
fn terminal_policy_keeps_advance_before_following_explicit_table() {
    let s = styles();
    let child = PreparedTextTable::prepare_with_end_policy(
        &table(&["child"]),
        &s,
        7200.0,
        &[],
        CellEndPolicy::OmitFinalLineGap,
    )
    .unwrap();
    let prepared = PreparedTextTable::from_flow_rows_with_end_policy(
        vec![212.0],
        vec![TextFlowRow {
            cells: vec![TextFlowCell {
                padding: Insets::default(),
                minimum_height: 0.0,
                blocks: vec![
                    TextFlowBlock::Paragraph {
                        owner: 0,
                        paragraph: Box::new(para("before")),
                    },
                    TextFlowBlock::Space(0.0),
                    TextFlowBlock::Table {
                        owner: ControlOwner {
                            paragraph: 1,
                            control: 0,
                        },
                        table: child,
                    },
                    TextFlowBlock::Space(7.0),
                ],
            }],
        }],
        0.0,
        SplitPolicy::WithinCells,
        &s,
        7200.0,
        CellEndPolicy::OmitFinalLineGap,
    )
    .unwrap();
    // before pitch18 + child(top3+line12+bottom4) + explicit7 =44.
    let f = placed(&prepared.start(), 44.0);
    let (_, lines) = render(&f);
    assert_eq!(
        lines.iter().map(text).collect::<Vec<_>>(),
        ["before", "child"]
    );
    assert_eq!(
        lines.iter().map(|l| l.bbox.y).collect::<Vec<_>>(),
        [30.0, 51.0]
    );
    assert_eq!(f.geometry().reserved_height(), 44.0);
    assert!(matches!(
        f.continuation().fit(area(44.0)).unwrap(),
        TextFragmentFit::Complete
    ));
}

#[test]
fn paragraph_end_policy_preserves_explicit_successor_and_minimum_height() {
    let mut s = styles();
    s.para_styles[0].spacing_after = 2.0;
    let policy = CellEndPolicy::OmitFinalParagraphGap;
    let child =
        PreparedTextTable::prepare_with_end_policy(&table(&[""]), &s, 7200.0, &[], policy).unwrap();
    let p = PreparedTextTable::from_flow_rows_with_end_policy(
        vec![212.0],
        vec![TextFlowRow {
            cells: vec![TextFlowCell {
                padding: Insets::default(),
                minimum_height: 0.0,
                blocks: vec![
                    TextFlowBlock::Paragraph {
                        owner: 0,
                        paragraph: Box::new(para("before")),
                    },
                    TextFlowBlock::Space(0.0),
                    TextFlowBlock::Table {
                        owner: ControlOwner {
                            paragraph: 1,
                            control: 0,
                        },
                        table: child,
                    },
                    TextFlowBlock::Space(7.0),
                ],
            }],
        }],
        0.0,
        SplitPolicy::WithinCells,
        &s,
        7200.0,
        policy,
    )
    .unwrap();
    // Nonterminal paragraph18+after2; child pads3+blank12+4; explicit7 =46.
    let f = placed(&p.start(), 46.0);
    let (_, lines) = render(&f);
    assert_eq!(lines.iter().map(text).collect::<Vec<_>>(), ["before", ""]);
    assert_eq!(
        lines.iter().map(|l| l.bbox.y).collect::<Vec<_>>(),
        [30.0, 53.0]
    );
    assert_eq!(f.geometry().reserved_height(), 46.0);
    assert!(matches!(
        f.continuation().fit(area(46.0)).unwrap(),
        TextFragmentFit::Complete
    ));

    let mut t = table(&["before", ""]);
    t.cells[0].height = 60;
    let p = PreparedTextTable::prepare_with_end_policy(&t, &s, 7200.0, &[], policy).unwrap();
    let f = placed(&p.start(), 60.0);
    assert_eq!(f.geometry().reserved_height(), 60.0);
    assert_eq!(
        render(&f).1.iter().map(text).collect::<Vec<_>>(),
        ["before", ""]
    );
    assert!(matches!(
        f.continuation().fit(area(60.0)).unwrap(),
        TextFragmentFit::Complete
    ));
}

// Synthetic fresh IR, not a Hancom-generated fixture. At 7200 dpi the input
// 400/1400 HU em boxes correspond to 4/14 pt in document units. A 156% pitch
// adds 224/784 HU respectively; both gaps are exact multiples of 4 HU.
// Test nonterminal origins, not the unresolved terminal-cell spacing policy.
fn blank_paragraph_fixture(count: usize, font_height: f64) -> (Table, ResolvedStyleSet) {
    let mut texts = vec!["before"];
    texts.extend(std::iter::repeat_n("", count));
    texts.push("after");
    let mut t = table(&texts);
    t.cells[0].width = 20_012;
    let mut s = styles();
    s.char_styles[0].font_size = 1200.0;
    s.para_styles[0].line_spacing = 1800.0;
    s.char_styles.push(ResolvedCharStyle {
        font_size: font_height,
        ..Default::default()
    });
    s.para_styles.push(ResolvedParaStyle {
        line_spacing: 156.0,
        line_spacing_type: rhwp::model::style::LineSpacingType::Percent,
        spacing_before: 200.0,
        spacing_after: 300.0,
        ..Default::default()
    });
    for p in &mut t.cells[0].paragraphs[1..=count] {
        p.char_shapes[0].char_shape_id = 1;
        p.para_shape_id = 1;
    }
    (t, s)
}

fn prepare_blank_flow(t: &Table, s: &ResolvedStyleSet, explicit: bool) -> PreparedTextTable {
    if !explicit {
        return PreparedTextTable::prepare(t, s, 7200.0).unwrap();
    }
    PreparedTextTable::from_flow_rows(
        vec![20_012.0],
        vec![TextFlowRow {
            cells: vec![TextFlowCell {
                padding: Insets {
                    left: 5.0,
                    right: 7.0,
                    top: 3.0,
                    bottom: 4.0,
                },
                minimum_height: 0.0,
                blocks: t.cells[0]
                    .paragraphs
                    .iter()
                    .enumerate()
                    .map(|(owner, p)| TextFlowBlock::Paragraph {
                        owner,
                        paragraph: Box::new(p.clone()),
                    })
                    .collect(),
            }],
        }],
        0.0,
        SplitPolicy::WithinCells,
        s,
        7200.0,
    )
    .unwrap()
}

fn blank_area(height: f64) -> PageArea {
    let mut page = area(height);
    page.bounds.width = 20_012.0;
    page
}

fn placed_blank(cursor: &TextTableCursor, height: f64) -> TextFragment {
    match cursor.fit(blank_area(height)).unwrap() {
        TextFragmentFit::Placed(f) => f,
        _ => panic!("expected blank paragraph flow"),
    }
}

fn blank_lines(fragment: &TextFragment) -> Vec<RenderNode> {
    // The synthetic frame is in source HU, so the paint page must use that
    // same coordinate scale rather than the small pixel page of other tests.
    let mut page = PageRenderTree::new(0, 21_000.0, 21_000.0);
    fragment.append_to(&mut page).unwrap();
    let mut lines = Vec::new();
    collect_lines(&page.root, &mut lines);
    for line in &lines {
        assert!(line.bbox.y >= 30.0);
        assert!(line.bbox.y + line.bbox.height <= 30.0 + fragment.geometry().reserved_height());
    }
    lines
}

#[test]
fn blank_paragraph_font_percent_and_insets_set_following_line_origin() {
    for explicit in [false, true] {
        for (font_height, pitch) in [(400.0, 624.0), (1400.0, 2184.0)] {
            for count in 0..=2 {
                let (t, s) = blank_paragraph_fixture(count, font_height);
                let original = serde_json::to_value(&t).unwrap();
                let prepared = prepare_blank_flow(&t, &s, explicit);
                let f = placed_blank(&prepared.start(), 20_000.0);
                let lines = blank_lines(&f);
                assert_eq!(lines.len(), count + 2);
                assert_eq!(text(&lines[0]), "before");
                assert_eq!(lines[0].bbox.y, 33.0);
                let advance = 200.0 + pitch + 300.0;
                for (i, line) in lines[1..=count].iter().enumerate() {
                    assert_eq!(text(line), "");
                    assert_eq!(line.bbox.height, font_height);
                    assert_eq!(line.bbox.y, 33.0 + 1800.0 + i as f64 * advance + 200.0);
                }
                let last = lines.last().unwrap();
                assert_eq!(text(last), "after");
                assert_eq!(last.bbox.y, 33.0 + 1800.0 + count as f64 * advance);
                assert!(last.bbox.y + last.bbox.height <= 30.0 + f.geometry().reserved_height());
                assert!(matches!(
                    f.continuation().fit(blank_area(20_000.0)).unwrap(),
                    TextFragmentFit::Complete
                ));
                assert_eq!(
                    serde_json::to_value(&t).unwrap(),
                    original,
                    "composition must not rewrite the input IR"
                );
            }
        }
    }
}

#[test]
fn blank_line_that_does_not_fit_is_carried_before_following_text() {
    for explicit in [false, true] {
        let (t, s) = blank_paragraph_fixture(1, 400.0);
        let prepared = prepare_blank_flow(&t, &s, explicit);
        // Top padding + first pitch + blank paragraph-before + one HU less
        // than the blank's 400 HU line box. No part of that line may disappear.
        let first = placed_blank(&prepared.start(), 3.0 + 1800.0 + 200.0 + 399.0);
        let first_lines = blank_lines(&first);
        assert_eq!(first_lines.iter().map(text).collect::<Vec<_>>(), ["before"]);
        let second = placed_blank(&first.continuation(), 20_000.0);
        let lines = blank_lines(&second);
        assert_eq!(lines.iter().map(text).collect::<Vec<_>>(), ["", "after"]);
        assert_eq!(lines[0].bbox.height, 400.0);
        assert_eq!(lines[1].bbox.y - lines[0].bbox.y, 624.0 + 300.0);
        assert!(matches!(
            second.continuation().fit(blank_area(20_000.0)).unwrap(),
            TextFragmentFit::Complete
        ));
    }
}

#[test]
fn stored_partitions_and_continuation_share_final_line_boxes() {
    // Synthetic boundary contract: two saved 12HU boxes at 18HU pitch,
    // 10HU baseline, 200HU content width. This is not Hancom evidence.
    let mut t = table(&["alphabeta"]);
    t.cells[0].paragraphs[0].line_segs = vec![
        LineSeg {
            text_start: 0,
            vertical_pos: 100,
            line_height: 12,
            text_height: 12,
            baseline_distance: 10,
            line_spacing: 6,
            segment_width: 200,
            tag: LineSeg::TAG_SINGLE_SEGMENT_LINE,
            ..Default::default()
        },
        LineSeg {
            text_start: 5,
            vertical_pos: 118,
            line_height: 12,
            text_height: 12,
            baseline_distance: 10,
            line_spacing: 6,
            segment_width: 200,
            tag: LineSeg::TAG_SINGLE_SEGMENT_LINE,
            ..Default::default()
        },
    ];
    let prepared = PreparedTextTable::prepare(&t, &styles(), 7200.0).unwrap();
    let first = placed(&prepared.start(), 21.0);
    let (_, lines) = render(&first);
    assert_eq!(lines.len(), 1);
    assert_eq!(text(&lines[0]), "alpha");
    assert_eq!(lines[0].bbox.y, 33.0);
    assert_eq!(first.geometry().reserved_height(), 21.0);
    let second = placed(&first.continuation(), 22.0);
    let (_, lines) = render(&second);
    assert_eq!(lines.len(), 1);
    assert_eq!(text(&lines[0]), "beta");
    assert_eq!(lines[0].bbox.y, 30.0);
    assert_eq!(second.geometry().reserved_height(), 22.0);
    assert!(matches!(
        second.continuation().fit(area(22.0)).unwrap(),
        TextFragmentFit::Complete
    ));
    assert_eq!(t.cells[0].paragraphs[0].line_segs[0].vertical_pos, 100);

    for mutation in 0..4 {
        let mut invalid = t.clone();
        let p = &mut invalid.cells[0].paragraphs[0];
        match mutation {
            0 => p.line_segs[1].vertical_pos = 110, // overlap is not a page reset
            1 => p.stored_text_partition_dirty = true,
            2 => p.line_segs[1].text_start = 99,
            _ => p.line_segs[0].tag |= LineSeg::TAG_IMPLEMENTATION_PROPERTY,
        }
        assert!(PreparedTextTable::prepare(&invalid, &styles(), 7200.0).is_err());
    }
    let mut resized = t.clone();
    resized.cells[0].width += 1;
    assert!(PreparedTextTable::prepare(&resized, &styles(), 7200.0).is_ok());
    resized.cells[0].width -= 2; // saved interval no longer fits, not a tolerance
    assert!(PreparedTextTable::prepare(&resized, &styles(), 7200.0).is_err());
}

#[test]
fn original_6923_justified_nested_cell_keeps_both_saved_rows() {
    use rhwp::model::control::Control;
    let d = rhwp::parse_document(
        &std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp"
        ))
        .unwrap(),
    )
    .unwrap();
    let Control::Table(outer) = &d.sections[0].paragraphs[5].controls[0] else {
        panic!()
    };
    let Control::Table(inner) = &outer.cells[0].paragraphs[4].controls[0] else {
        panic!()
    };
    let p = &inner.cells[6].paragraphs[1];
    assert!(p.text.starts_with("○ 나머지 약 79.7%"));
    assert_eq!(p.line_segs.len(), 2);
    assert_eq!(p.line_segs[1].text_start, 30);
    assert_eq!(p.line_segs[0].segment_width, 23172);
    let t = Table {
        row_count: 1,
        col_count: 1,
        page_break: TablePageBreak::CellBreak,
        cells: vec![Cell {
            width: 23175,
            row_span: 1,
            col_span: 1,
            paragraphs: vec![p.clone()],
            ..Default::default()
        }],
        ..Default::default()
    };
    let s = rhwp::renderer::style_resolver::resolve_styles(&d.doc_info, 96.0);
    let prepared = PreparedTextTable::prepare(&t, &s, 96.0).unwrap();
    let area = PageArea {
        bounds: Rect {
            x: 0.0,
            y: 0.0,
            width: 309.0,
            height: 14.666666666666666,
        },
    };
    let TextFragmentFit::Placed(first) = prepared.start().fit(area).unwrap() else {
        panic!()
    };
    let (_, lines) = render(&first);
    assert_eq!(lines.len(), 1);
    assert_eq!(text(&lines[0]), "○ 나머지 약 79.7%(291,679천병)에는 병당 ");
    assert!((lines[0].bbox.width - 23172.0 / 75.0).abs() < 1e-7);
    let tail = lines[0].children.last().unwrap();
    let RenderNodeType::TextRun(run) = &tail.node_type else {
        panic!()
    };
    use rhwp::renderer::layout::{EmbeddedTextMeasurer, TextMeasurer};
    let positions = EmbeddedTextMeasurer.compute_char_positions(&run.text, &run.style);
    assert!(
        (tail.bbox.x + positions[run.text.trim_end_matches(' ').chars().count()] - 23172.0 / 75.0)
            .abs()
            < 1e-7
    );
    let TextFragmentFit::Placed(next) = first
        .continuation()
        .fit(PageArea {
            bounds: Rect {
                height: 40.0,
                ..area.bounds
            },
        })
        .unwrap()
    else {
        panic!()
    };
    let (_, next_lines) = render(&next);
    assert_eq!(next_lines.len(), 1);
    assert_eq!(
        text(&next_lines[0]),
        "2.6%∼100%의 암반수가 들어간 것으로 확인"
    );
    // Hanging indent is saved bit20 plus -4128/2HU, not a new wrap guess.
    assert!((next_lines[0].bbox.x - 2064.0 / 75.0).abs() < 1e-7);
    assert!(matches!(
        next.continuation().fit(area).unwrap(),
        TextFragmentFit::Complete
    ));
}

#[test]
fn justified_stored_rows_use_exact_style_owned_trailing_space_width() {
    use rhwp::model::style::Alignment;
    // Alignment invariant, not a rounded-width oracle: the first justified
    // row's last visible advance ends at the stored right edge. The trailing
    // spaces keep their own style and remain in the source row.
    for font_size in [11.3, 14.666666666666666] {
        for split_suffix in [false, true] {
            let mut t = table(&["AA BB   CC", "after"]);
            let mut s = styles();
            s.char_styles[0].font_size = font_size;
            s.char_styles[0].letter_spacing = font_size * 0.01;
            s.para_styles[0].alignment = Alignment::Justify;
            s.char_styles.push(s.char_styles[0].clone());
            s.char_styles[1].font_size = font_size * 0.8;
            let p = &mut t.cells[0].paragraphs[0];
            if split_suffix {
                p.char_shapes.extend([
                    CharShapeRef {
                        start_pos: 6,
                        char_shape_id: 1,
                    },
                    CharShapeRef {
                        start_pos: 8,
                        char_shape_id: 0,
                    },
                ]);
            }
            p.line_segs = [0, 8]
                .into_iter()
                .enumerate()
                .map(|(i, start)| LineSeg {
                    text_start: start,
                    vertical_pos: i as i32 * 24,
                    line_height: 20,
                    text_height: 20,
                    baseline_distance: 17,
                    line_spacing: 4,
                    segment_width: 200,
                    tag: LineSeg::TAG_SINGLE_SEGMENT_LINE,
                    ..Default::default()
                })
                .collect();
            let prepared = PreparedTextTable::prepare(&t, &s, 7200.0).unwrap();
            let (_, lines) = render(&placed(&prepared.start(), 100.0));
            assert_eq!(text(&lines[0]), "AA BB   ");
            assert_eq!(text(&lines[1]), "CC");
            assert_eq!(text(&lines[2]), "after");
            assert_eq!(lines[1].bbox.y - lines[0].bbox.y, 24.0);
            let first = &lines[0].children[0];
            let RenderNodeType::TextRun(run) = &first.node_type else {
                panic!()
            };
            use rhwp::renderer::layout::{EmbeddedTextMeasurer, TextMeasurer};
            let positions = EmbeddedTextMeasurer.compute_char_positions(&run.text, &run.style);
            assert!((first.bbox.x + positions[5] - (lines[0].bbox.x + 200.0)).abs() < 1e-7);
        }
    }
}

#[test]
fn trailing_plain_space_keeps_logical_advance_without_widening_occupied_line() {
    use rhwp::model::style::{Alignment, UnderlineType};
    // Independent alignment contract: RIGHT places visible A at the right
    // edge, while plain trailing spaces keep their logical advances. The
    // stored row remains 200HU wide, 12HU high, with 6HU following space.
    for split_run in [false, true] {
        let mut t = table(&["A        ", "after"]);
        let p = &mut t.cells[0].paragraphs[0];
        p.line_segs = vec![LineSeg {
            line_height: 12,
            text_height: 12,
            baseline_distance: 10,
            line_spacing: 6,
            segment_width: 200,
            tag: LineSeg::TAG_SINGLE_SEGMENT_LINE,
            ..Default::default()
        }];
        let mut s = styles();
        s.para_styles[0].alignment = Alignment::Right;
        s.char_styles.push(s.char_styles[0].clone());
        if split_run {
            // Force a distinct style owner without changing its geometry.
            p.char_shapes.push(CharShapeRef {
                start_pos: 1,
                char_shape_id: 1,
            });
        }
        let prepared = PreparedTextTable::prepare(&t, &s, 7200.0).unwrap();
        let (page, lines) = render(&placed(&prepared.start(), 43.0));
        assert_eq!(lines.len(), 2);
        assert_eq!(text(&lines[0]), "A        ");
        assert_eq!(lines[0].bbox.width, 200.0);
        assert_eq!(lines[0].bbox.height, 12.0);
        assert_eq!(lines[1].bbox.y - lines[0].bbox.y, 18.0);
        let end = lines[0].bbox.x + 200.0;
        let last = &lines[0].children.last().unwrap().bbox;
        assert!(last.x + last.width > end);
        let mut control = t.clone();
        let p = &mut control.cells[0].paragraphs[0];
        p.text = "A".into();
        p.char_count = 1;
        p.char_offsets = vec![0];
        p.char_shapes.truncate(1);
        let prepared = PreparedTextTable::prepare(&control, &s, 7200.0).unwrap();
        let (_, control_lines) = render(&placed(&prepared.start(), 43.0));
        let a = &control_lines[0].children[0].bbox;
        assert!((a.x + a.width - end).abs() < 1e-7);
        assert!((lines[0].children[0].bbox.x - a.x).abs() < 1e-7);
        let mut svg = SvgRenderer::new();
        svg.render_tree(&page);
        assert!(svg.output().contains("A"));
        // A decoration makes those spaces visible, so overflow stays rejected.
        for effect in [0, 1, 2] {
            let mut decorated = s.clone();
            for font in &mut decorated.char_styles {
                match effect {
                    0 => font.underline = UnderlineType::Bottom,
                    1 => font.strikethrough = true,
                    _ => font.shade_color = 0x00ff00,
                }
            }
            assert!(PreparedTextTable::prepare(&t, &decorated, 7200.0).is_err());
        }
        let mut visible_overflow = t.clone();
        visible_overflow.cells[0].paragraphs[0].text = "A".repeat(100);
        visible_overflow.cells[0].paragraphs[0].char_offsets = (0..100).collect();
        visible_overflow.cells[0].paragraphs[0].char_count = 100;
        assert!(PreparedTextTable::prepare(&visible_overflow, &s, 7200.0).is_err());
    }
}

#[test]
fn soft_wrap_decoration_uses_shared_trim_but_preserves_author_spaces() {
    use rhwp::model::style::{Alignment, UnderlineType};
    use rhwp::paint::{LayerBuilder, LayerNode, LayerNodeKind, PaintOp, RenderProfile};
    fn check(node: &LayerNode, count: &mut usize) {
        match &node.kind {
            LayerNodeKind::Group { children, .. } => {
                for child in children {
                    check(child, count)
                }
            }
            LayerNodeKind::ClipRect { child, .. } => check(child, count),
            LayerNodeKind::Leaf { ops } => {
                for op in ops {
                    if let PaintOp::TextDecoration {
                        bbox,
                        run,
                        trim_trailing_spaces,
                        ..
                    } = op
                    {
                        if run.text == "AA BB   " {
                            assert_eq!(*trim_trailing_spaces, 3);
                            use rhwp::renderer::layout::{EmbeddedTextMeasurer, TextMeasurer};
                            let positions =
                                EmbeddedTextMeasurer.compute_char_positions(&run.text, &run.style);
                            // Justification's independently specified right edge.
                            assert!((bbox.x + positions[5] - (20.0 + 5.0 + 200.0)).abs() < 1e-7);
                            *count += 1;
                        }
                    }
                }
            }
        }
    }
    for strike in [false, true] {
        let mut t = table(&["AA BB   CC", "after"]);
        let mut s = styles();
        s.para_styles[0].alignment = Alignment::Justify;
        s.char_styles[0].underline = if strike {
            UnderlineType::None
        } else {
            UnderlineType::Bottom
        };
        s.char_styles[0].strikethrough = strike;
        t.cells[0].paragraphs[0].line_segs = [0, 8]
            .into_iter()
            .enumerate()
            .map(|(i, start)| LineSeg {
                text_start: start,
                vertical_pos: i as i32 * 24,
                line_height: 20,
                text_height: 20,
                baseline_distance: 17,
                line_spacing: 4,
                segment_width: 200,
                tag: LineSeg::TAG_SINGLE_SEGMENT_LINE,
                ..Default::default()
            })
            .collect();
        let prepared = PreparedTextTable::prepare(&t, &s, 7200.).unwrap();
        let (page, lines) = render(&placed(&prepared.start(), 100.));
        assert_eq!(text(&lines[0]), "AA BB   ");
        assert_eq!(text(&lines[1]), "CC");
        assert_eq!(lines[1].bbox.y - lines[0].bbox.y, 24.);
        let layer = LayerBuilder::new(RenderProfile::Screen).build(&page);
        let mut count = 0;
        check(&layer.root, &mut count);
        assert_eq!(count, 1);
        let mut svg = SvgRenderer::new();
        svg.render_tree(&page);
        let xml = roxmltree::Document::parse(svg.output()).unwrap();
        let edge = xml.descendants().find(|n| n.has_tag_name("line")).unwrap();
        assert!((edge.attribute("x2").unwrap().parse::<f64>().unwrap() - 225.0).abs() < 0.001);
        // An authored last-line signature is not a soft-wrap separator.
        let mut end = t.clone();
        let p = &mut end.cells[0].paragraphs[0];
        p.text = "AA BB   ".into();
        p.char_count = 8;
        p.char_offsets = (0..8).collect();
        p.line_segs.truncate(1);
        s.para_styles[0].alignment = Alignment::Right;
        assert!(PreparedTextTable::prepare(&end, &s, 7200.).is_err());
    }
}

#[test]
fn atomic_row_fit_is_translation_invariant_at_exact_saved_height() {
    // Two original saved row minima in HU. Their sum fits exactly; adding a
    // page origin must not change either the cut or the reserved height.
    let height = 14847.0 / 75.0;
    let cursor = TableContentPlan::new(
        vec![100.0],
        [1765.0 / 75.0, 13082.0 / 75.0]
            .into_iter()
            .map(|minimum_height| RowInput {
                cells: vec![CellInput {
                    padding: Insets::default(),
                    minimum_height,
                    content: ComposedCell {
                        width: 100.0,
                        height: 3.0,
                        lines: vec![LineBox {
                            owner: LineOwner {
                                paragraph: 0,
                                line: 0,
                            },
                            bounds: Rect {
                                x: 0.0,
                                y: 0.0,
                                width: 100.0,
                                height: 3.0,
                            },
                        }],
                    },
                }],
            })
            .collect(),
        0.0,
        SplitPolicy::BetweenRows,
    )
    .unwrap()
    .start();
    for y in [0.0, 126.48, 500.0] {
        let area = PageArea {
            bounds: Rect {
                x: 20.0,
                y,
                width: 100.0,
                height,
            },
        };
        let FragmentFit::Placed(part) = cursor.fit(area).unwrap() else {
            panic!("exact height")
        };
        assert!(
            part.continuation().is_complete(),
            "origin {y} changed the cut"
        );
        assert_eq!(part.reserved_height(), height);
        assert_eq!(part.placement().cells.len(), 2);
        assert_eq!(part.placement().cells[1].bounds.y, y + 1765.0 / 75.0);
        let short = PageArea {
            bounds: Rect {
                height: height - 1.0 / 75.0,
                ..area.bounds
            },
        };
        let FragmentFit::Placed(part) = cursor.fit(short).unwrap() else {
            panic!("first row")
        };
        assert!(!part.continuation().is_complete());
        assert_eq!(part.placement().cells.len(), 1);
    }
}

#[test]
fn contained_stored_frames_survive_alignment_and_continuation() {
    use rhwp::model::style::Alignment;
    for alignment in [Alignment::Left, Alignment::Center, Alignment::Right] {
        let mut t = table(&["AB", "after"]);
        t.cells[0].paragraphs[0].line_segs = [(0, 100, 10, 150), (1, 118, 20, 170)]
            .into_iter()
            .map(
                |(text_start, vertical_pos, column_start, segment_width)| LineSeg {
                    text_start,
                    vertical_pos,
                    column_start,
                    segment_width,
                    line_height: 12,
                    text_height: 12,
                    baseline_distance: 10,
                    line_spacing: 6,
                    tag: LineSeg::TAG_SINGLE_SEGMENT_LINE,
                },
            )
            .collect();
        let source = t.clone();
        let mut s = styles();
        s.para_styles[0].alignment = alignment;
        let prepared = PreparedTextTable::prepare(&t, &s, 7200.0).unwrap();
        let first = placed(&prepared.start(), 21.0);
        let (_, a) = render(&first);
        let next = placed(&first.continuation(), 40.0);
        let (_, b) = render(&next);
        assert_eq!(a.len(), 1);
        assert_eq!(b.len(), 2);
        for (line, x, y, w, label) in [
            (&a[0], 35.0, 33.0, 150.0, "A"),
            (&b[0], 45.0, 30.0, 170.0, "B"),
        ] {
            assert_eq!(
                (line.bbox.x, line.bbox.y, line.bbox.width, line.bbox.height),
                (x, y, w, 12.0)
            );
            assert_eq!(text(line), label);
            let run = &line.children[0].bbox;
            let expected = match alignment {
                Alignment::Center => x + (w - run.width) / 2.0,
                Alignment::Right => x + w - run.width,
                _ => x,
            };
            assert!((run.x - expected).abs() < 1e-9);
        }
        assert_eq!(text(&b[1]), "after");
        assert_eq!(b[1].bbox.y, 48.0);
        assert_eq!(next.geometry().reserved_height(), 40.0);
        assert!(matches!(
            next.continuation().fit(area(40.0)).unwrap(),
            TextFragmentFit::Complete
        ));
        assert_eq!(
            t.cells[0].paragraphs[0].line_segs,
            source.cells[0].paragraphs[0].line_segs
        );
        for (x, w) in [(-1, 150), (51, 150), (0, 201), (0, 0)] {
            let mut bad = t.clone();
            bad.cells[0].paragraphs[0].line_segs[0].column_start = x;
            bad.cells[0].paragraphs[0].line_segs[0].segment_width = w;
            assert!(PreparedTextTable::prepare(&bad, &s, 7200.0).is_err());
        }
    }
}

#[test]
fn stored_margins_are_not_applied_twice_and_invalid_frames_stay_rejected() {
    let mut t = table(&["AB", "after"]);
    t.cells[0].paragraphs[0].line_segs = [(0, 100, 10, 170), (1, 118, 20, 160)]
        .into_iter()
        .map(
            |(text_start, vertical_pos, column_start, segment_width)| LineSeg {
                text_start,
                vertical_pos,
                column_start,
                segment_width,
                line_height: 12,
                text_height: 12,
                baseline_distance: 10,
                line_spacing: 6,
                tag: LineSeg::TAG_SINGLE_SEGMENT_LINE,
            },
        )
        .collect();
    let mut s = styles();
    s.para_styles[0].margin_left = 10.0;
    s.para_styles[0].margin_right = 20.0;
    let source = t.clone();
    let prepared = PreparedTextTable::prepare(&t, &s, 7200.0).unwrap();
    let a = placed(&prepared.start(), 21.0);
    let b = placed(&a.continuation(), 40.0);
    let (_, first) = render(&a);
    let (_, rest) = render(&b);
    // Cell origin25 + saved cs10/20; not + paragraph margin again.
    assert_eq!((first[0].bbox.x, first[0].bbox.width), (35.0, 170.0));
    assert_eq!((rest[0].bbox.x, rest[0].bbox.width), (45.0, 160.0));
    assert_eq!((rest[1].bbox.x, rest[1].bbox.width), (35.0, 170.0));
    assert_eq!(
        (text(&first[0]), text(&rest[0]), text(&rest[1])),
        ("A".into(), "B".into(), "after".into())
    );
    assert_eq!(
        (
            a.geometry().reserved_height(),
            b.geometry().reserved_height()
        ),
        (21.0, 40.0)
    );
    assert_eq!(
        t.cells[0].paragraphs[0].line_segs,
        source.cells[0].paragraphs[0].line_segs
    );
    for (x, w) in [(9, 170), (10, 171)] {
        let mut bad = t.clone();
        bad.cells[0].paragraphs[0].line_segs[0].column_start = x;
        bad.cells[0].paragraphs[0].line_segs[0].segment_width = w;
        assert!(PreparedTextTable::prepare(&bad, &s, 7200.0).is_err());
    }
    // Inactive saved flags do not receive a fresh first-line/hanging inset.
    for indent in [-10.0, 10.0] {
        s.para_styles[0].indent = indent;
        let prepared = PreparedTextTable::prepare(&t, &s, 7200.0).unwrap();
        let (_, first) = render(&placed(&prepared.start(), 21.0));
        assert_eq!((first[0].bbox.x, first[0].bbox.width), (35.0, 170.0));
    }
}

#[test]
fn saved_indentation_survives_continuation_without_restarting_first_line_rules() {
    for indent in [10.0, -10.0] {
        let mut t = table(&["AB", "after"]);
        t.cells[0].paragraphs[0].line_segs = (0..2)
            .map(|i| LineSeg {
                text_start: i,
                vertical_pos: 100 + i as i32 * 18,
                column_start: 10,
                segment_width: 170,
                line_height: 12,
                text_height: 12,
                baseline_distance: 10,
                line_spacing: 6,
                tag: LineSeg::TAG_SINGLE_SEGMENT_LINE
                    | if (indent > 0.0) == (i == 0) {
                        LineSeg::TAG_INDENTATION
                    } else {
                        0
                    },
            })
            .collect();
        let original = t.cells[0].paragraphs[0].line_segs.clone();
        let mut s = styles();
        s.para_styles[0].indent = indent;
        s.para_styles[0].margin_left = 10.0;
        s.para_styles[0].margin_right = 20.0;
        let prepared = PreparedTextTable::prepare(&t, &s, 7200.0).unwrap();
        if let TextFragmentFit::Placed(f) = prepared.start().fit(area(14.0)).unwrap() {
            let (_, lines) = render(&f);
            // Only the 3px top padding fits; no content unit is consumed.
            assert_eq!(f.geometry().reserved_height(), 3.0);
            assert!(lines.is_empty());
            let (_, next) = render(&placed(&f.continuation(), 18.0));
            assert_eq!(text(&next[0]), "A");
        }
        let a = placed(&prepared.start(), 21.0);
        let b = placed(&a.continuation(), 40.0);
        let (_, first) = render(&a);
        let (_, rest) = render(&b);
        for (i, line) in [&first[0], &rest[0]].into_iter().enumerate() {
            let shift = if (indent > 0.0) == (i == 0) {
                10.0
            } else {
                0.0
            };
            assert_eq!(
                (line.bbox.x, line.bbox.width),
                (35.0 + shift, 170.0 - shift)
            );
            assert_eq!(line.children[0].bbox.x, line.bbox.x);
        }
        assert_eq!(
            (text(&first[0]), text(&rest[0]), text(&rest[1])),
            ("A".into(), "B".into(), "after".into())
        );
        assert_eq!(rest[1].bbox.y, 48.0);
        assert_eq!(
            (
                a.geometry().reserved_height(),
                b.geometry().reserved_height()
            ),
            (21.0, 40.0)
        );
        assert!(matches!(
            b.continuation().fit(area(40.0)).unwrap(),
            TextFragmentFit::Complete
        ));
        assert_eq!(t.cells[0].paragraphs[0].line_segs, original);
        // The same prepared child is translated into a parent content box.
        // Neither parent padding nor indentation may be added a second time.
        let nested = PreparedTextTable::from_flow_rows(
            vec![232.0],
            vec![TextFlowRow {
                cells: vec![TextFlowCell {
                    padding: Insets {
                        left: 10.0,
                        right: 10.0,
                        ..Default::default()
                    },
                    minimum_height: 0.0,
                    blocks: vec![TextFlowBlock::Table {
                        owner: ControlOwner {
                            paragraph: 0,
                            control: 0,
                        },
                        table: prepared,
                    }],
                }],
            }],
            0.0,
            SplitPolicy::WithinCells,
            &s,
            7200.0,
        )
        .unwrap();
        let mut nested_area = area(100.0);
        nested_area.bounds.width = 232.0;
        let TextFragmentFit::Placed(f) = nested.start().fit(nested_area).unwrap() else {
            panic!("nested child")
        };
        let (_, lines) = render(&f);
        assert_eq!(
            lines.iter().map(text).collect::<Vec<_>>(),
            ["A", "B", "after"]
        );
        for (i, line) in lines.iter().take(2).enumerate() {
            let shift = if (indent > 0.0) == (i == 0) {
                10.0
            } else {
                0.0
            };
            assert_eq!(
                (line.bbox.x, line.bbox.width),
                (45.0 + shift, 170.0 - shift)
            );
            assert_eq!(line.bbox.y, 33.0 + i as f64 * 18.0);
        }
        assert_eq!(lines[2].bbox.y, 69.0);
        for bad in [170.0, 200.0, 0.5, f64::NAN] {
            s.para_styles[0].indent = bad;
            assert!(
                PreparedTextTable::prepare(&t, &s, 7200.0).is_err(),
                "inset {bad}"
            );
        }
        s.para_styles[0].indent = indent;
        t.cells[0].paragraphs[0].stored_text_partition_dirty = true;
        assert!(PreparedTextTable::prepare(&t, &s, 7200.0).is_err());
    }
}

#[test]
fn original_6923_first_empty_cell_keeps_its_stored_line_frame() {
    let source = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp"),
    )
    .unwrap();
    let d = rhwp::parse_document(&source).unwrap();
    let rhwp::model::control::Control::Table(original) = &d.sections[0].paragraphs[0].controls[3]
    else {
        panic!("first table")
    };
    let cell = original.cells[0].clone();
    assert_eq!(cell.width, 47488);
    assert_eq!(original.padding.left + original.padding.right, 282);
    let row = &cell.paragraphs[0].line_segs[0];
    assert_eq!(
        (
            row.column_start,
            row.segment_width,
            row.line_height,
            row.line_spacing
        ),
        (0, 47204, 300, 92)
    );
    let t = Table {
        row_count: 1,
        col_count: 4,
        page_break: TablePageBreak::RowBreak,
        padding: original.padding,
        cells: vec![cell],
        ..Default::default()
    };
    let s = rhwp::renderer::style_resolver::resolve_styles(&d.doc_info, 96.0);
    let prepared = PreparedTextTable::prepare(&t, &s, 96.0).unwrap();
    let TextFragmentFit::Placed(fragment) = prepared
        .start()
        .fit(PageArea {
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                width: 700.0,
                height: 100.0,
            },
        })
        .unwrap()
    else {
        panic!("fit")
    };
    let (_, lines) = render(&fragment);
    assert_eq!(lines.len(), 1);
    assert!(text(&lines[0]).is_empty());
    for (actual, hu) in [
        (lines[0].bbox.x, 141.0),
        (lines[0].bbox.width, 47204.0),
        (lines[0].bbox.height, 300.0),
    ] {
        assert!((actual - hu / 75.0).abs() < 1e-9);
    }
}

#[test]
fn original_6923_stored_paragraphs_keep_source_metrics_in_v2_fragments() {
    // Actual HWP-origin rows, NOT fabricated LineSeg metadata. Isolated text
    // probes do not claim that the full document's tables are admitted.
    let data = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp"),
    )
    .unwrap();
    let document = rhwp::parse_document(&data).unwrap();
    let styles = rhwp::renderer::style_resolver::resolve_styles(&document.doc_info, 96.0);
    fn visit<'a>(ps: &'a [Paragraph], out: &mut Vec<&'a Paragraph>) {
        for p in ps {
            if p.controls.is_empty() && !p.text.is_empty() {
                out.push(p);
            }
            for control in &p.controls {
                if let rhwp::model::control::Control::Table(t) = control {
                    for c in &t.cells {
                        visit(&c.paragraphs, out);
                    }
                }
            }
        }
    }
    let mut paragraphs = Vec::new();
    visit(&document.sections[0].paragraphs, &mut paragraphs);
    let mut admitted = 0;
    let mut previously_unadorned = 0;
    let mut reasons = std::collections::BTreeMap::new();
    for p in paragraphs {
        let source = serde_json::to_value(p).unwrap();
        let width = p.line_segs[0].segment_width;
        if width <= 0 {
            continue;
        }
        let t = Table {
            row_count: 1,
            col_count: 1,
            page_break: TablePageBreak::CellBreak,
            cells: vec![Cell {
                width: width as u32,
                row_span: 1,
                col_span: 1,
                paragraphs: vec![p.clone()],
                ..Default::default()
            }],
            ..Default::default()
        };
        // A no-paint reference must be observationally identical to no
        // decoration, including rejection on unrelated stored-layout limits.
        // This comparison replaces the old support-count cap, not geometry.
        let id = document.doc_info.para_shapes[p.para_shape_id as usize].border_fill_id;
        let unpainted = id == 0
            || document
                .doc_info
                .border_fills
                .get(id as usize - 1)
                .is_some_and(|b| {
                    use rhwp::model::style::{BorderLineType, CenterLine, FillType};
                    b.attr == 0
                        && !b.three_d
                        && b.center_line == CenterLine::None
                        && b.borders
                            .iter()
                            .all(|line| line.line_type == BorderLineType::None)
                        && (b.fill.fill_type == FillType::None
                            || b.fill.fill_type == FillType::Solid
                                && b.fill.solid.is_some_and(|s| {
                                    s.pattern_type <= 0 && s.background_color >> 24 != 0
                                }))
                });
        let reference = unpainted.then(|| {
            let mut reference_styles = styles.clone();
            reference_styles.para_styles[p.para_shape_id as usize].border_fill_id = 0;
            PreparedTextTable::prepare(&t, &reference_styles, 96.0)
        });
        let actual = PreparedTextTable::prepare(&t, &styles, 96.0);
        if let Some(reference) = &reference {
            assert_eq!(
                actual.is_ok(),
                reference.is_ok(),
                "no-paint reference changed admission"
            );
        }
        let prepared = match actual {
            Ok(v) => v,
            Err(e) => {
                *reasons.entry(format!("{e:?}")).or_insert(0usize) += 1;
                continue;
            }
        };
        let a = PageArea {
            bounds: Rect {
                x: 20.0,
                y: 30.0,
                width: 1000.0,
                height: 2000.0,
            },
        };
        let TextFragmentFit::Placed(fragment) = prepared.start().fit(a).unwrap() else {
            panic!("expected text")
        };
        let mut page = PageRenderTree::new(0, 1000.0, 2200.0);
        fragment.append_to(&mut page).unwrap();
        if let Some(Ok(reference)) = reference {
            let TextFragmentFit::Placed(fragment) = reference.start().fit(a).unwrap() else {
                panic!("reference must fit the same area")
            };
            let mut expected = PageRenderTree::new(0, 1000.0, 2200.0);
            fragment.append_to(&mut expected).unwrap();
            assert_eq!(
                serde_json::to_value(&page).unwrap(),
                serde_json::to_value(&expected).unwrap()
            );
        }
        let mut lines = Vec::new();
        collect_lines(&page.root, &mut lines);
        assert_eq!(lines.len(), p.line_segs.len());
        assert_eq!(lines.iter().map(text).collect::<String>(), p.text);
        for (line, row) in lines.iter().zip(&p.line_segs) {
            assert!((line.bbox.height - f64::from(row.line_height) / 75.0).abs() < 1e-9);
            assert!(
                (line.bbox.y
                    - lines[0].bbox.y
                    - f64::from(row.vertical_pos - p.line_segs[0].vertical_pos) / 75.0)
                    .abs()
                    < 1e-9
            );
        }
        assert_eq!(serde_json::to_value(p).unwrap(), source);
        admitted += 1;
        previously_unadorned += usize::from(id == 0);
    }
    eprintln!("original stored paragraphs admitted={admitted}, unsupported={reasons:?}");
    assert_eq!(
        previously_unadorned, 6,
        "actual source admission must not silently disappear"
    );
    assert!(
        admitted > previously_unadorned,
        "non-painting references must be exercised"
    );
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

fn stored_tab_table() -> Table {
    let mut t = table(&["A\tB", "C\tD"]);
    t.cells[0].width = 15000;
    for (i, p) in t.cells[0].paragraphs.iter_mut().enumerate() {
        p.char_offsets = vec![0, 1, 9];
        p.char_count = 11;
        // One8-unit control per tab, two distinct paragraph-owned payloads.
        p.tab_extended = vec![[1500 + i as u16 * 750, 0, 0x100, 32, 32, 32, 9]];
        p.line_segs = vec![LineSeg {
            text_start: 0,
            vertical_pos: 0,
            line_height: 900,
            text_height: 900,
            baseline_distance: 765,
            line_spacing: 450,
            segment_width: 14988,
            tag: LineSeg::TAG_SINGLE_SEGMENT_LINE,
            ..Default::default()
        }];
    }
    t
}

#[test]
fn saved_left_tabs_keep_advance_line_membership_and_split_payload() {
    let t = stored_tab_table();
    let prepared = PreparedTextTable::prepare(&t, &styles(), 96.0).unwrap();
    let (_, lines) = render(&placed(&prepared.start(), 100.0));
    assert_eq!(lines.iter().map(text).collect::<Vec<_>>(), ["A\tB", "C\tD"]);
    for (line, advance) in lines.iter().zip([20.0, 30.0]) {
        let run = line
            .children
            .iter()
            .find_map(|n| match &n.node_type {
                RenderNodeType::TextRun(r) if r.text.contains('\t') => Some(r),
                _ => None,
            })
            .unwrap();
        use rhwp::renderer::layout::{EmbeddedTextMeasurer, TextMeasurer};
        let pos = run
            .layout_positions
            .clone()
            .unwrap_or_else(|| EmbeddedTextMeasurer.compute_char_positions(&run.text, &run.style));
        let tab = run.text.chars().position(|c| c == '\t').unwrap();
        // Independently declared1500/2250HU advances, not glyph width estimates.
        assert!(
            (pos[tab + 1] - pos[tab] - advance).abs() < 1e-9,
            "text={:?} positions={pos:?} tabs={:?}",
            run.text,
            run.style.inline_tabs
        );
    }
    let first = placed(&prepared.start(), 13.0);
    let (_, first_lines) = render(&first);
    assert_eq!(first_lines.iter().map(text).collect::<Vec<_>>(), ["A\tB"]);
    let next = first.continuation();
    let second = placed(&next, 100.0);
    assert!(matches!(
        second.continuation().fit(area(100.0)).unwrap(),
        TextFragmentFit::Complete
    ));
    let (_, second_lines) = render(&second);
    assert_eq!(second_lines.iter().map(text).collect::<Vec<_>>(), ["C\tD"]);
    let payloads = |line: &RenderNode| {
        line.children
            .iter()
            .map(|n| match &n.node_type {
                RenderNodeType::TextRun(r) => r.clone(),
                _ => panic!("text run"),
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(payloads(&lines[1]), payloads(&second_lines[0]));
}

#[test]
fn unqualified_tabs_do_not_silently_use_fallback_stops() {
    for case in 0..9 {
        let mut t = stored_tab_table();
        let p = &mut t.cells[0].paragraphs[0];
        match case {
            0 => p.line_segs.clear(), // fresh composition is a different contract
            1 => {
                p.tab_extended.pop();
            }
            2 => p.tab_extended[0][0] = 0,     // placeholder
            3 => p.tab_extended[0][1] = 1,     // common walker does not consume high word
            4 => p.tab_extended[0][2] = 0x200, // RIGHT
            5 => p.tab_extended[0][2] = 0x101, // leader
            6 => p.char_offsets[2] = 2,        // invalid raw source ownership
            7 => p.line_segs.push(p.line_segs[0].clone()),
            _ => {
                p.text.push('\t');
                p.char_offsets.push(10);
                p.char_count = 19;
                p.tab_extended.push(p.tab_extended[0]);
            }
        }
        assert!(matches!(
            PreparedTextTable::prepare(&t, &styles(), 96.0),
            Err(GeometryError::Unsupported(
                "stored inline LEFT tab contract"
            ))
        ));
    }
    assert!(matches!(
        PreparedTextTable::prepare(&stored_tab_table(), &styles(), 144.0),
        Err(GeometryError::Unsupported(
            "stored inline LEFT tab contract"
        ))
    ));
}

#[test]
fn single_saved_tab_keeps_advance_before_after_and_between_styled_text() {
    use rhwp::renderer::layout::{EmbeddedTextMeasurer, TextMeasurer};
    for value in ["\tA", "A\t", "A\tB", " \t "] {
        let mut t = stored_tab_table();
        t.cells[0].paragraphs.truncate(1);
        let p = &mut t.cells[0].paragraphs[0];
        p.text = value.into();
        let mut offset = 0;
        p.char_offsets = value
            .chars()
            .map(|c| {
                let start = offset;
                offset += if c == '\t' { 8 } else { 1 };
                start
            })
            .collect();
        p.char_count = offset + 1;
        p.char_shapes.push(CharShapeRef {
            start_pos: p.char_offsets[1],
            char_shape_id: 1,
        });
        let mut s = styles();
        let mut alternate = s.char_styles[0].clone();
        alternate.bold = true;
        s.char_styles.push(alternate);
        let prepared = PreparedTextTable::prepare(&t, &s, 96.0).unwrap();
        let (_, lines) = render(&placed(&prepared.start(), 100.0));
        assert_eq!(lines.len(), 1);
        assert_eq!(text(&lines[0]), value);
        let mut tab_count = 0;
        for node in &lines[0].children {
            let RenderNodeType::TextRun(run) = &node.node_type else {
                panic!("run")
            };
            if let Some(tab) = run.text.chars().position(|c| c == '\t') {
                let positions = run.layout_positions.clone().unwrap_or_else(|| {
                    EmbeddedTextMeasurer.compute_char_positions(&run.text, &run.style)
                });
                assert!((positions[tab + 1] - positions[tab] - 20.0).abs() < 1e-9);
                tab_count += 1;
            }
        }
        assert_eq!(tab_count, 1);
    }
}
