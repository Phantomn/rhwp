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
    // Indent needs its own source-tag/context contract; never ignore it.
    for indent in [-10.0, 10.0] {
        s.para_styles[0].indent = indent;
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
        let prepared = match PreparedTextTable::prepare(&t, &styles, 96.0) {
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
    }
    eprintln!("original stored paragraphs admitted={admitted}, unsupported={reasons:?}");
    assert_eq!(
        admitted, 6,
        "actual source admission must not silently disappear"
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
