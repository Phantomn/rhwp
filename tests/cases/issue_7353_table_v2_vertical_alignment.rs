//! Synthetic fresh HWPX; oracle is alignment in the padded, intact row box.
//! Fixed18px lines (including empty paragraphs) and6/12px vertical padding.
//! No Hancom-fidelity claim or guessed alignment across an internal cell cut.
use rhwp::model::{
    control::Control,
    document::{Document, Section},
    paragraph::{CharShapeRef, Paragraph},
    shape::{CommonObjAttr, HorzAlign, HorzRelTo, TextWrap, VertRelTo},
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
fn host(text: &str, child: Table) -> Paragraph {
    let mut p = para(text);
    p.char_count += 8;
    p.char_offsets.iter_mut().for_each(|v| *v += 8);
    p.controls.push(Control::Table(Box::new(child)));
    p
}
fn cell(row: u16, col: u16, span: u16, align: VerticalAlign, text: &[&str]) -> Cell {
    Cell {
        row,
        col,
        row_span: 1,
        col_span: span,
        width: 7500 * u32::from(span),
        border_fill_id: 1,
        vertical_align: align,
        paragraphs: text.iter().map(|s| para(s)).collect(),
        ..Default::default()
    }
}
fn table(rows: u16, columns: u16, cells: Vec<Cell>) -> Table {
    Table {
        row_count: rows,
        col_count: columns,
        cells,
        page_break: TablePageBreak::RowBreak,
        padding: Padding {
            top: 450,
            bottom: 900,
            ..Default::default()
        },
        common: CommonObjAttr {
            text_wrap: TextWrap::TopAndBottom,
            vert_rel_to: VertRelTo::Para,
            horz_rel_to: HorzRelTo::Para,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn grid() -> Table {
    let mut t = table(
        2,
        3,
        vec![
            cell(0, 0, 1, VerticalAlign::Top, &["T", ""]),
            cell(0, 1, 1, VerticalAlign::Center, &["", "C"]),
            cell(0, 2, 1, VerticalAlign::Bottom, &["", "B"]),
            cell(1, 0, 3, VerticalAlign::Top, &["after"]),
        ],
    );
    t.cells[0].height = 6750; //90px sets the common row height; siblings have no minimum.
    t
}
fn source(root: Table, budget: f64) -> (Vec<u8>, Value) {
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
        paragraphs: vec![host("", root)],
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
fn run(name: &str, root: Table, budget: f64) -> Vec<Value> {
    let (bytes, options) = source(root, budget);
    let mut s = TablePreviewExportSession::from_bytes(&bytes, &options.to_string()).unwrap();
    let mut pages = Vec::new();
    while let Some(page) = s.next_page_json().unwrap() {
        pages.push(serde_json::from_str(&page).unwrap());
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
            serde_json::to_vec_pretty(&pages).unwrap(),
        )
        .unwrap();
    }
    pages
}
fn collect<'a>(node: &'a Value, kind: &str) -> Vec<&'a Value> {
    let mut found = if node["node_type"].get(kind).is_some() {
        vec![node]
    } else {
        vec![]
    };
    for c in node["children"].as_array().unwrap() {
        found.extend(collect(c, kind));
    }
    found
}
fn check(page: &Value, lines: &[(&str, f64, f64)], boxes: &[[f64; 4]]) {
    let root = &page["render_tree"]["root"];
    let nodes = collect(root, "TextLine");
    let actual: Vec<_> = nodes
        .iter()
        .map(|n| {
            let runs = collect(n, "TextRun");
            (
                runs[0]["node_type"]["TextRun"]["text"].as_str().unwrap(),
                n["bbox"]["x"].as_f64().unwrap(),
                n["bbox"]["y"].as_f64().unwrap(),
            )
        })
        .collect();
    assert_eq!(actual, lines);
    assert_eq!(
        collect(root, "TableCell")
            .iter()
            .map(|n| ["x", "y", "width", "height"].map(|k| n["bbox"][k].as_f64().unwrap()))
            .collect::<Vec<_>>(),
        boxes
    );
}
fn grid_lines() -> Vec<(&'static str, f64, f64)> {
    // Row90 - padding18 - two lines36 =36px slack:0/18/36.
    vec![
        ("T", 20.0, 36.0),
        ("", 20.0, 54.0),
        ("", 120.0, 54.0),
        ("C", 120.0, 72.0),
        ("", 220.0, 72.0),
        ("B", 220.0, 90.0),
    ]
}
#[test]
fn row_break_alignment_uses_common_height_and_counts_empty_lines() {
    let pages = run("valign-rows", grid(), 90.0);
    assert_eq!(pages.len(), 2);
    check(
        &pages[0],
        &grid_lines(),
        &[
            [20.0, 30.0, 100.0, 90.0],
            [120.0, 30.0, 100.0, 90.0],
            [220.0, 30.0, 100.0, 90.0],
        ],
    );
    check(
        &pages[1],
        &[("after", 20.0, 36.0)],
        &[[20.0, 30.0, 300.0, 36.0]],
    );
}
#[test]
fn unsplit_table_uses_the_same_alignment_without_moving_following_row() {
    let mut t = grid();
    t.page_break = TablePageBreak::None;
    let pages = run("valign-whole", t, 126.0);
    assert_eq!(pages.len(), 1);
    let mut lines = grid_lines();
    lines.push(("after", 20.0, 126.0));
    check(
        &pages[0],
        &lines,
        &[
            [20.0, 30.0, 100.0, 90.0],
            [120.0, 30.0, 100.0, 90.0],
            [220.0, 30.0, 100.0, 90.0],
            [20.0, 120.0, 300.0, 36.0],
        ],
    );
}
#[test]
fn repeated_centered_header_reserves_its_full_physical_height() {
    let mut t = table(
        3,
        2,
        vec![
            cell(0, 0, 2, VerticalAlign::Center, &["title"]),
            cell(1, 0, 2, VerticalAlign::Top, &["A"]),
            cell(2, 0, 2, VerticalAlign::Top, &["B"]),
        ],
    );
    t.repeat_header = true;
    t.cells[0].is_header = true;
    t.cells[0].height = 4050; //54px
    let pages = run("valign-header", t, 90.0);
    assert_eq!(pages.len(), 2);
    for (i, label) in ["A", "B"].iter().enumerate() {
        check(
            &pages[i],
            &[("title", 20.0, 45.0), (label, 20.0, 90.0)],
            &[[20.0, 30.0, 200.0, 54.0], [20.0, 84.0, 200.0, 36.0]],
        );
    }
}
#[test]
fn child_row_alignment_survives_parent_cell_continuation() {
    let mut child = table(
        2,
        1,
        vec![
            cell(0, 0, 1, VerticalAlign::Center, &["", "C"]),
            cell(1, 0, 1, VerticalAlign::Bottom, &["D"]),
        ],
    );
    child.cells[0].height = 6750;
    child.cells[1].height = 5400;
    child.common.horz_align = HorzAlign::Right;
    let mut root = table(1, 2, vec![cell(0, 0, 2, VerticalAlign::Top, &[])]);
    root.page_break = TablePageBreak::CellBreak;
    root.padding = Padding {
        left: 750,
        right: 2250,
        ..Default::default()
    };
    root.cells[0].paragraphs = vec![para("before"), host("host", child), para("after")];
    let pages = run("valign-nested", root, 108.0);
    assert_eq!(pages.len(), 2);
    check(
        &pages[0],
        &[("before", 30.0, 30.0), ("", 90.0, 72.0), ("C", 90.0, 90.0)],
        &[[20.0, 30.0, 200.0, 108.0], [90.0, 48.0, 100.0, 90.0]],
    );
    check(
        &pages[1],
        &[
            ("D", 90.0, 72.0),
            ("host", 30.0, 102.0),
            ("after", 30.0, 120.0),
        ],
        &[[20.0, 30.0, 200.0, 108.0], [90.0, 30.0, 100.0, 72.0]],
    );
}
#[test]
fn centered_parent_translates_nested_table_and_host_together() {
    let mut child = table(1, 1, vec![cell(0, 0, 1, VerticalAlign::Top, &["A"])]);
    child.padding = Padding::default(); //18px
    child.common.horz_align = HorzAlign::Right;
    let mut root = table(1, 2, vec![cell(0, 0, 2, VerticalAlign::Center, &[])]);
    root.cells[0].height = 6750;
    root.cells[0].paragraphs = vec![host("host", child), para("after")]; //54px+18padding=72; center slack9.
    let pages = run("valign-parent", root, 90.0);
    assert_eq!(pages.len(), 1);
    check(
        &pages[0],
        &[
            ("A", 120.0, 45.0),
            ("host", 20.0, 63.0),
            ("after", 20.0, 81.0),
        ],
        &[[20.0, 30.0, 200.0, 90.0], [120.0, 45.0, 100.0, 18.0]],
    );
}
#[test]
fn content_expansion_and_individual_padding_do_not_create_negative_slack() {
    for align in [
        VerticalAlign::Top,
        VerticalAlign::Center,
        VerticalAlign::Bottom,
    ] {
        let mut t = table(1, 1, vec![cell(0, 0, 1, align, &["", "A"])]);
        t.cells[0].height = 750; //10px is less than occupied36px+6/12padding.
        t.cells[0].apply_inner_margin = true;
        t.cells[0].padding = t.padding;
        t.padding.top = 2250; //Must not participate; cell override takes precedence.
        let (bytes, options) = source(t, 54.0);
        let mut s = TablePreviewExportSession::from_bytes(&bytes, &options.to_string()).unwrap();
        let p: Value = serde_json::from_str(&s.next_page_json().unwrap().unwrap()).unwrap();
        check(
            &p,
            &[("", 20.0, 36.0), ("A", 20.0, 54.0)],
            &[[20.0, 30.0, 100.0, 54.0]],
        );
        assert!(s.next_page_json().unwrap().is_none());
    }
}
#[test]
fn split_cell_vertical_alignment_is_rejected_including_nested_input() {
    for align in [VerticalAlign::Center, VerticalAlign::Bottom] {
        for nested in [false, true] {
            let mut t = table(1, 1, vec![cell(0, 0, 1, align, &["A"])]);
            t.page_break = TablePageBreak::CellBreak;
            if nested {
                let mut outer = table(1, 2, vec![cell(0, 0, 2, VerticalAlign::Top, &[])]);
                outer.cells[0].paragraphs = vec![host("host", t)];
                t = outer;
            }
            let (bytes, options) = source(t, 90.0);
            let error = TablePreviewExportSession::from_bytes(&bytes, &options.to_string())
                .err()
                .expect("no silent fallback");
            assert!(format!("{error:?}").contains("Unsupported"));
        }
    }
}
#[test]
fn alignment_does_not_reduce_row_budget_or_commit_failed_fit() {
    let (bytes, options) = source(grid(), 89.0);
    let mut s = TablePreviewExportSession::from_bytes(&bytes, &options.to_string()).unwrap();
    for _ in 0..2 {
        let err = s.next_page_json().unwrap_err();
        assert!(format!("{err:?}").contains("DoesNotFit"));
        assert_eq!(s.emitted_pages(), 0);
    }
}
