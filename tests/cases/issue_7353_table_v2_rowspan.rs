//! Intact row groups, not cell-internal rowspan cuts. Synthetic HWPX contracts:
//! 18px line pitch, explicit row minima, shared edges and complete source owners.
//! These are geometry invariants, not a Hancom-output fidelity claim.
use rhwp::model::{
    control::Control,
    document::{Document, Section},
    paragraph::{CharShapeRef, Paragraph},
    shape::{CommonObjAttr, HorzRelTo, TextWrap, VertRelTo},
    style::{BorderFill, BorderLine, BorderLineType, CharShape, LineSpacingType, ParaShape},
    table::{Cell, Table, TablePageBreak, VerticalAlign},
    Padding,
};
use rhwp::renderer::table_v2::TablePreviewExportSession;
use serde_json::{json, Value};

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
fn host(text: &str, table: Table) -> Paragraph {
    let mut p = para(text);
    p.char_count += 8;
    p.char_offsets.iter_mut().for_each(|v| *v += 8);
    p.controls.push(Control::Table(Box::new(table)));
    p
}
fn cell(row: u16, col: u16, rs: u16, cs: u16, height: u32, text: &str) -> Cell {
    Cell {
        row,
        col,
        row_span: rs,
        col_span: cs,
        width: 4500 * u32::from(cs),
        height,
        border_fill_id: 1,
        paragraphs: vec![para(text)],
        ..Default::default()
    }
}
fn table(rows: u16, columns: u16, cells: Vec<Cell>) -> Table {
    Table {
        row_count: rows,
        col_count: columns,
        cells,
        page_break: TablePageBreak::RowBreak,
        common: CommonObjAttr {
            text_wrap: TextWrap::TopAndBottom,
            vert_rel_to: VertRelTo::Para,
            horz_rel_to: HorzRelTo::Para,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn groups() -> Table {
    // The two spans overlap in row2, joining rows1..4 into one108px group.
    table(
        5,
        3,
        vec![
            cell(0, 0, 1, 3, 1350, "prefix"),
            cell(1, 0, 2, 1, 5400, "A"),
            cell(1, 1, 1, 2, 2700, "B"),
            cell(2, 1, 2, 1, 5400, "C"),
            cell(2, 2, 1, 1, 2700, "D"),
            cell(3, 0, 1, 1, 2700, "E"),
            cell(3, 2, 1, 1, 2700, "F"),
            cell(4, 0, 1, 3, 1350, "after"),
        ],
    )
}
fn source(t: Table, budget: f64) -> (Vec<u8>, Value) {
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
    d.doc_info.border_fills.push(BorderFill {
        borders: [BorderLine {
            line_type: BorderLineType::Solid,
            width: 7,
            color: 0x332211,
        }; 4],
        ..Default::default()
    });
    d.sections = vec![Section {
        paragraphs: vec![host("", t)],
        ..Default::default()
    }];
    let bytes = rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap();
    let parsed = rhwp::parse_document(&bytes).unwrap();
    let ci = parsed.sections[0].paragraphs[0]
        .controls
        .iter()
        .position(|c| matches!(c, Control::Table(_)))
        .unwrap();
    (
        bytes,
        json!({"selection":{"section":0,"paragraph":0,"control":ci},"dpi":96,
        "pages":{"width":400,"height":400,"body":{"x":20,"y":30,"width":300,"height":budget},"first_y":30},"max_pages":10}),
    )
}
fn run(name: &str, t: Table, budget: f64) -> Vec<Value> {
    let (bytes, options) = source(t, budget);
    let mut s = TablePreviewExportSession::from_bytes(&bytes, &options.to_string()).unwrap();
    let mut pages = vec![];
    let mut raw_pages = vec![];
    while let Some(page) = s.next_page_json().unwrap() {
        pages.push(serde_json::from_str(&page).unwrap());
        raw_pages.push(page);
        assert!(pages.len() < 10);
    }
    assert_eq!(s.emitted_pages() as usize, pages.len());
    assert!(s.next_page_json().unwrap().is_none());
    if let Ok(dir) = std::env::var("ISSUE7353_EXPORT_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/{name}.hwpx"), bytes).unwrap();
        std::fs::write(format!("{dir}/{name}.options.json"), options.to_string()).unwrap();
        std::fs::write(
            format!("{dir}/{name}.native.json"),
            // Preserve the transport bytes. serde_json's default Value parser
            // can round120.00000000000001 to120 before reserialization; browser
            // parity must compare actual exports, not that second conversion.
            format!("[{}]", raw_pages.join(",")),
        )
        .unwrap();
    }
    pages
}
fn collect<'a>(n: &'a Value, kind: &str) -> Vec<&'a Value> {
    let mut v = if n["node_type"].get(kind).is_some() {
        vec![n]
    } else {
        vec![]
    };
    for child in n["children"].as_array().unwrap() {
        v.extend(collect(child, kind));
    }
    v
}
fn boxes(page: &Value) -> Vec<[f64; 4]> {
    collect(&page["render_tree"]["root"], "TableCell")
        .iter()
        .map(|c| ["x", "y", "width", "height"].map(|k| c["bbox"][k].as_f64().unwrap()))
        .collect()
}
fn texts(page: &Value) -> Vec<&str> {
    collect(&page["render_tree"]["root"], "TextRun")
        .iter()
        .map(|n| n["node_type"]["TextRun"]["text"].as_str().unwrap())
        .collect()
}
fn check_boxes(page: &Value, expected: &[[f64; 4]]) {
    let actual = boxes(page);
    assert_eq!(actual.len(), expected.len());
    for (a, e) in actual.iter().flatten().zip(expected.iter().flatten()) {
        // Unit conversion may represent60px as60.00000000000001; no output rounding.
        assert!(
            (a - e).abs() <= 8.0 * f64::EPSILON * e.abs().max(1.0),
            "{a} != {e}"
        );
    }
}
#[test]
fn connected_spans_defer_together_preserving_owners_and_final_boxes() {
    let mut t = groups();
    t.cells.reverse();
    let pages = run("rowspan-groups", t, 108.0);
    assert_eq!(pages.len(), 3);
    assert_eq!(texts(&pages[0]), ["prefix"]);
    assert_eq!(texts(&pages[1]), ["A", "B", "C", "D", "E", "F"]);
    assert_eq!(texts(&pages[2]), ["after"]);
    for (page, height) in pages.iter().zip([18.0, 108.0, 18.0]) {
        let tables = collect(&page["render_tree"]["root"], "Table");
        assert_eq!(tables.len(), 1);
        assert_eq!(
            tables[0]["bbox"],
            json!({"x":20.0,"y":30.0,"width":180.0,"height":height})
        );
    }
    assert_eq!(boxes(&pages[0]), [[20.0, 30.0, 180.0, 18.0]]);
    check_boxes(
        &pages[1],
        &[
            [20.0, 30.0, 60.0, 72.0],
            [80.0, 30.0, 120.0, 36.0],
            [80.0, 66.0, 60.0, 72.0],
            [140.0, 66.0, 60.0, 36.0],
            [20.0, 102.0, 60.0, 36.0],
            [140.0, 102.0, 60.0, 36.0],
        ],
    );
    assert_eq!(boxes(&pages[2]), [[20.0, 30.0, 180.0, 18.0]]);
    let cells = collect(&pages[1]["render_tree"]["root"], "TableCell");
    assert_eq!(
        cells
            .iter()
            .map(|c| {
                let a = &c["node_type"]["TableCell"];
                (
                    a["row"].as_u64().unwrap(),
                    a["col"].as_u64().unwrap(),
                    a["row_span"].as_u64().unwrap(),
                )
            })
            .collect::<Vec<_>>(),
        [
            (1, 0, 2),
            (1, 1, 1),
            (2, 1, 2),
            (2, 2, 1),
            (3, 0, 1),
            (3, 2, 1)
        ]
    );
    let edges: Vec<_> = collect(&pages[1]["render_tree"]["root"], "Line")
        .iter()
        .map(|n| ["x1", "y1", "x2", "y2"].map(|k| n["node_type"]["Line"][k].as_f64().unwrap()))
        .collect();
    assert_eq!(
        edges,
        vec![
            [20.0, 30.0, 20.0, 138.0],
            [80.0, 30.0, 80.0, 138.0],
            [140.0, 66.0, 140.0, 138.0],
            [200.0, 30.0, 200.0, 138.0],
            [20.0, 30.0, 200.0, 30.0],
            [80.0, 66.0, 200.0, 66.0],
            [20.0, 102.0, 80.0, 102.0],
            [140.0, 102.0, 200.0, 102.0],
            [20.0, 138.0, 200.0, 138.0],
        ]
    );
}
#[test]
fn whole_table_and_repeated_prefix_consume_complete_groups() {
    let mut t = groups();
    t.page_break = TablePageBreak::None;
    let pages = run("rowspan-whole", t, 144.0);
    assert_eq!(pages.len(), 1);
    assert_eq!(
        texts(&pages[0]),
        ["prefix", "A", "B", "C", "D", "E", "F", "after"]
    );
    assert_eq!(
        boxes(&pages[0]).last().unwrap(),
        &[20.0, 156.0, 180.0, 18.0]
    );
    let mut t = groups();
    t.repeat_header = true;
    t.cells[0].is_header = true;
    let pages = run("rowspan-header", t, 126.0);
    assert_eq!(pages.len(), 2);
    assert_eq!(texts(&pages[0]), ["prefix", "A", "B", "C", "D", "E", "F"]);
    assert_eq!(texts(&pages[1]), ["prefix", "after"]);
    assert_eq!(
        boxes(&pages[1]),
        [[20.0, 30.0, 180.0, 18.0], [20.0, 48.0, 180.0, 18.0]]
    );
}
#[test]
fn alignment_uses_entire_span_with_padding_once() {
    let mut t = table(
        2,
        4,
        vec![
            cell(0, 0, 2, 1, 5400, "T"),
            cell(0, 1, 2, 1, 5400, "C"),
            cell(0, 2, 2, 1, 5400, "B"),
            cell(0, 3, 1, 1, 2700, "a"),
            cell(1, 3, 1, 1, 2700, "b"),
        ],
    );
    t.padding = Padding {
        top: 450,
        bottom: 900,
        ..Default::default()
    };
    t.cells[1].vertical_align = VerticalAlign::Center;
    t.cells[2].vertical_align = VerticalAlign::Bottom;
    let pages = run("rowspan-align", t, 72.0);
    assert_eq!(pages.len(), 1);
    assert_eq!(texts(&pages[0]), ["T", "C", "B", "a", "b"]);
    assert_eq!(
        collect(&pages[0]["render_tree"]["root"], "TextLine")
            .iter()
            .map(|n| n["bbox"]["y"].as_f64().unwrap())
            .collect::<Vec<_>>(),
        [36.0, 54.0, 72.0, 36.0, 72.0]
    );
    check_boxes(
        &pages[0],
        &[
            [20.0, 30.0, 60.0, 72.0],
            [80.0, 30.0, 60.0, 72.0],
            [140.0, 30.0, 60.0, 72.0],
            [200.0, 30.0, 60.0, 36.0],
            [200.0, 66.0, 60.0, 36.0],
        ],
    );
}
#[test]
fn nested_groups_preserve_child_origin_host_and_end() {
    let mut outer = table(1, 3, vec![cell(0, 0, 1, 3, 0, "")]);
    outer.page_break = TablePageBreak::CellBreak;
    outer.cells[0].paragraphs = vec![host("host", groups()), para("tail")];
    let pages = run("rowspan-nested", outer, 108.0);
    assert_eq!(pages.len(), 3);
    assert_eq!(texts(&pages[0]), ["prefix"]);
    assert_eq!(texts(&pages[1]), ["A", "B", "C", "D", "E", "F"]);
    assert_eq!(texts(&pages[2]), ["after", "host", "tail"]);
    assert_eq!(
        boxes(&pages[2]),
        [[20.0, 30.0, 180.0, 54.0], [20.0, 30.0, 180.0, 18.0]]
    );
    assert_eq!(
        collect(&pages[2]["render_tree"]["root"], "TextLine")
            .iter()
            .map(|n| n["bbox"]["y"].as_f64().unwrap())
            .collect::<Vec<_>>(),
        [30.0, 48.0, 66.0]
    );
}
#[test]
fn insufficient_group_budget_does_not_commit_header_or_partial_cell() {
    let mut t = groups();
    t.repeat_header = true;
    t.cells[0].is_header = true;
    let (bytes, options) = source(t, 125.0);
    let mut s = TablePreviewExportSession::from_bytes(&bytes, &options.to_string()).unwrap();
    for _ in 0..2 {
        let error = s.next_page_json().unwrap_err().to_string();
        assert!(
            error.contains("DoesNotFit") && error.contains("126"),
            "{error}"
        );
        assert_eq!(s.emitted_pages(), 0);
    }
}

#[test]
fn spanning_header_repeats_without_crossing_into_body_or_losing_border_slots() {
    let mut t = table(
        4,
        2,
        vec![
            cell(0, 0, 2, 1, 5400, "header"),
            cell(0, 1, 1, 1, 2700, "h1"),
            cell(1, 1, 1, 1, 2700, "h2"),
            cell(2, 0, 1, 2, 1350, "body1"),
            cell(3, 0, 1, 2, 1350, "body2"),
        ],
    );
    t.repeat_header = true;
    for c in &mut t.cells {
        c.is_header = c.row < 2;
    }
    let pages = run("rowspan-spanning-header", t, 90.0);
    assert_eq!(pages.len(), 2);
    for (i, p) in pages.iter().enumerate() {
        assert_eq!(
            texts(p),
            ["header", "h1", "h2", if i == 0 { "body1" } else { "body2" }]
        );
        check_boxes(
            p,
            &[
                [20.0, 30.0, 60.0, 72.0],
                [80.0, 30.0, 60.0, 36.0],
                [80.0, 66.0, 60.0, 36.0],
                [20.0, 102.0, 120.0, 18.0],
            ],
        );
        let edges = collect(&p["render_tree"]["root"], "Line");
        assert_eq!(edges.len(), 7);
        assert!(edges.iter().any(
            |n| n["node_type"]["Line"]["y1"] == 120.0 && n["node_type"]["Line"]["y2"] == 120.0
        ));
    }
}

#[test]
fn nested_table_inside_spanning_cell_uses_full_alignment_box_once() {
    let child = table(1, 1, vec![cell(0, 0, 1, 1, 1350, "inner")]);
    let mut t = table(
        2,
        2,
        vec![
            cell(0, 0, 2, 1, 5400, ""),
            cell(0, 1, 1, 1, 2700, "a"),
            cell(1, 1, 1, 1, 2700, "b"),
        ],
    );
    t.cells[0].paragraphs = vec![host("host", child), para("tail")];
    t.cells[0].vertical_align = VerticalAlign::Center;
    let pages = run("rowspan-inner", t, 72.0);
    assert_eq!(pages.len(), 1);
    assert_eq!(texts(&pages[0]), ["inner", "host", "tail", "a", "b"]);
    check_boxes(
        &pages[0],
        &[
            [20.0, 30.0, 60.0, 72.0],
            [20.0, 39.0, 60.0, 18.0],
            [80.0, 30.0, 60.0, 36.0],
            [80.0, 66.0, 60.0, 36.0],
        ],
    );
    assert_eq!(
        collect(&pages[0]["render_tree"]["root"], "TextLine")
            .iter()
            .map(|n| n["bbox"]["y"].as_f64().unwrap())
            .collect::<Vec<_>>(),
        [39.0, 57.0, 75.0, 30.0, 66.0]
    );
}

#[test]
fn failed_later_group_does_not_repeat_already_consumed_prefix() {
    let (bytes, options) = source(groups(), 107.5);
    let mut s = TablePreviewExportSession::from_bytes(&bytes, &options.to_string()).unwrap();
    let first: Value = serde_json::from_str(&s.next_page_json().unwrap().unwrap()).unwrap();
    assert_eq!(texts(&first), ["prefix"]);
    for _ in 0..2 {
        let error = s.next_page_json().unwrap_err().to_string();
        assert!(
            error.contains("DoesNotFit") && error.contains("108"),
            "{error}"
        );
        assert_eq!(s.emitted_pages(), 1);
    }
}

#[test]
fn original_6923_has_observed_span_heights_not_invented_equal_rows() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp");
    let bytes = std::fs::read(path).unwrap();
    let doc = rhwp::parse_document(&bytes).unwrap();
    let t = doc.sections[0].paragraphs[0]
        .controls
        .iter()
        .find_map(|c| {
            if let Control::Table(t) = c {
                Some(t)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!((t.row_count, t.col_count), (5, 4));
    let at = |r, c| t.cells.iter().find(|v| v.row == r && v.col == c).unwrap();
    assert_eq!(
        [
            at(1, 1).height,
            at(2, 1).height,
            at(3, 1).height,
            at(4, 1).height
        ],
        [2282, 3042, 1922, 3274]
    );
    for (r, c, end) in [(1, 0, 4), (1, 3, 3), (3, 3, 5)] {
        let spanning = at(r, c);
        assert_eq!(r + spanning.row_span, end);
        assert_eq!(
            spanning.height,
            (r..end).map(|row| at(row, 1).height).sum::<u32>()
        );
    }
    // Observation of unmodified source geometry only: pictures, section controls
    // and other unresolved source rules still prevent full V2 document admission.
}
#[test]
fn ambiguous_heights_internal_cuts_and_invalid_topology_are_rejected() {
    for kind in 0..7 {
        let mut t = groups();
        let expected = match kind {
            0 => {
                t.page_break = TablePageBreak::CellBreak;
                "rowspan cell-internal cuts"
            }
            1 => {
                t.cells[1].height = 10000;
                "rowspan height needs redistribution"
            }
            2 => {
                t.cells[1].row_span = 0;
                // HWPX parser normalizes zero span to1; the resulting gap must
                // still be rejected, not interpreted as a valid merged grid.
                "incomplete"
            }
            3 => {
                t.cells[1].row_span = 5;
                "cell address or span"
            }
            4 => {
                t.cells[3].col = 0;
                "overlapping"
            }
            5 => {
                t.cells.remove(4);
                "incomplete"
            }
            _ => {
                t.repeat_header = true;
                for c in &mut t.cells {
                    c.is_header = c.row < 2;
                }
                "header boundary crosses rowspan"
            }
        };
        let (bytes, options) = source(t, 300.0);
        let error = TablePreviewExportSession::from_bytes(&bytes, &options.to_string())
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains(expected), "kind={kind}: {error}");
    }
    let t = table(2, 1, vec![cell(0, 0, 2, 1, 5400, "A")]);
    let (bytes, options) = source(t, 300.0);
    assert!(
        TablePreviewExportSession::from_bytes(&bytes, &options.to_string())
            .err()
            .unwrap()
            .to_string()
            .contains("unresolved rowspan row boundaries")
    );
}
