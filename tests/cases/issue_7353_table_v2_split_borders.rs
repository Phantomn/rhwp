//! Cell cuts use accepted physical fragments, including empty continuations.
//! Closure oracle: task3236 Hancom PDF p1 bottom / p2 top. Coordinates below
//! are synthetic 18px lines and declared padding/minima, not PDF-fidelity claims.
use rhwp::model::{
    control::Control,
    document::{Document, Section},
    paragraph::{CharShapeRef, Paragraph},
    shape::{CommonObjAttr, HorzRelTo, TextWrap, VertRelTo},
    style::{BorderFill, BorderLine, BorderLineType, CharShape, LineSpacingType, ParaShape},
    table::{Cell, Table, TablePageBreak},
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
fn cell(row: u16, col: u16, span: u16, text: &[&str]) -> Cell {
    Cell {
        row,
        col,
        row_span: 1,
        col_span: span,
        width: u32::from(span) * 7500,
        border_fill_id: 1,
        paragraphs: text.iter().map(|s| para(s)).collect(),
        ..Default::default()
    }
}
fn table(cells: Vec<Cell>, rows: u16) -> Table {
    Table {
        row_count: rows,
        col_count: 2,
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
fn doc(table: Table) -> Document {
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
        paragraphs: vec![host("", table)],
        ..Default::default()
    }];
    d
}
fn source(d: &Document) -> (Vec<u8>, Value) {
    let data = rhwp::serializer::hwpx::serialize_hwpx(d).unwrap();
    let parsed = rhwp::parse_document(&data).unwrap();
    let control = parsed.sections[0].paragraphs[0]
        .controls
        .iter()
        .position(|c| matches!(c, Control::Table(_)))
        .unwrap();
    (
        data,
        json!({"selection":{"section":0,"paragraph":0,"control":control},"dpi":96,
        "pages":{"width":400,"height":400,"body":{"x":20,"y":30,"width":300,"height":36},"first_y":30},"max_pages":100}),
    )
}
fn run(name: &str, d: &Document) -> Vec<Value> {
    let (data, config) = source(d);
    let mut session = TablePreviewExportSession::from_bytes(&data, &config.to_string()).unwrap();
    let mut pages = Vec::new();
    while let Some(p) = session.next_page_json().unwrap() {
        pages.push(serde_json::from_str(&p).unwrap());
        assert!(pages.len() < 10, "must terminate after physical remainder");
    }
    assert_eq!(session.emitted_pages() as usize, pages.len());
    assert!(session.next_page_json().unwrap().is_none());
    if let Ok(dir) = std::env::var("ISSUE7353_EXPORT_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/{name}.hwpx"), data).unwrap();
        std::fs::write(format!("{dir}/{name}.options.json"), config.to_string()).unwrap();
        std::fs::write(
            format!("{dir}/{name}.native.json"),
            serde_json::to_vec_pretty(&pages).unwrap(),
        )
        .unwrap();
    }
    pages
}
fn collect<'a>(node: &'a Value, kind: &str) -> Vec<&'a Value> {
    let mut result = if node["node_type"].get(kind).is_some() {
        vec![node]
    } else {
        vec![]
    };
    for c in node["children"].as_array().unwrap() {
        result.extend(collect(c, kind));
    }
    result
}
fn root(page: &Value) -> &Value {
    &page["render_tree"]["root"]
}
fn labels(page: &Value) -> Vec<&str> {
    collect(root(page), "TextRun")
        .iter()
        .map(|n| n["node_type"]["TextRun"]["text"].as_str().unwrap())
        .collect()
}
fn edges(node: &Value) -> Vec<[f64; 4]> {
    collect(node, "Line")
        .iter()
        .map(|n| {
            let l = &n["node_type"]["Line"];
            assert_eq!(l["style"]["color"], 0x332211);
            assert!((l["style"]["width"].as_f64().unwrap() - 1.92).abs() < 1e-9);
            ["x1", "y1", "x2", "y2"].map(|k| l[k].as_f64().unwrap())
        })
        .collect()
}
fn box_edges(x: f64, y: f64, width: f64, height: f64) -> Vec<[f64; 4]> {
    vec![
        [x, y, x, y + height],
        [x + width, y, x + width, y + height],
        [x, y, x + width, y],
        [x, y + height, x + width, y + height],
    ]
}
fn bounds(pages: &[Value]) {
    for p in pages {
        for c in collect(root(p), "TableCell") {
            let b = &c["bbox"];
            assert!(b["y"].as_f64().unwrap() >= 30.);
            assert!(b["y"].as_f64().unwrap() + b["height"].as_f64().unwrap() <= 66.);
        }
    }
}

#[test]
fn split_cell_closes_each_fragment_preserving_empty_line_and_horizontal_padding() {
    let mut t = table(vec![cell(0, 0, 2, &["A", "", "B", "C"])], 1);
    t.padding.left = 750;
    t.padding.right = 750;
    let p = run("cut-lines", &doc(t));
    assert_eq!(p.len(), 2);
    assert_eq!(labels(&p[0]), ["A", ""]);
    assert_eq!(labels(&p[1]), ["B", "C"]);
    for page in &p {
        assert_eq!(edges(root(page)), box_edges(20., 30., 200., 36.));
        let lines = collect(root(page), "TextLine");
        assert_eq!(
            lines
                .iter()
                .map(|n| n["bbox"]["y"].as_f64().unwrap())
                .collect::<Vec<_>>(),
            [30., 48.]
        );
        assert!(lines.iter().all(|n| n["bbox"]["x"] == 30.));
    }
    bounds(&p);
}

#[test]
fn physical_tail_has_edges_after_last_unit_without_repeating_text_or_extra_page() {
    let mut c = cell(0, 0, 2, &["A"]);
    c.height = 6750; // 90px = 36 + 36 + 18; no content in last two fragments.
    let p = run("cut-tail", &doc(table(vec![c], 1)));
    assert_eq!(p.len(), 3);
    for (i, page) in p.iter().enumerate() {
        assert_eq!(labels(page), if i == 0 { vec!["A"] } else { vec![] });
        assert_eq!(
            edges(root(page)),
            box_edges(20., 30., 200., if i < 2 { 36. } else { 18. })
        );
    }
    bounds(&p);
}

#[test]
fn matching_table_outline_follows_content_cuts_and_empty_physical_tail() {
    for tail in [false, true] {
        let mut c = cell(0, 0, 2, if tail { &["A"] } else { &["A", "", "B", "C"] });
        if tail {
            c.height = 6750;
        }
        let mut t = table(vec![c], 1);
        t.border_fill_id = 1;
        if !tail {
            t.padding.left = 750;
            t.padding.right = 750;
        }
        let p = run(
            if tail {
                "outer-cut-tail"
            } else {
                "outer-cut-lines"
            },
            &doc(t),
        );
        assert_eq!(p.len(), if tail { 3 } else { 2 });
        for (i, page) in p.iter().enumerate() {
            assert_eq!(
                labels(page),
                if tail {
                    if i == 0 {
                        vec!["A"]
                    } else {
                        vec![]
                    }
                } else if i == 0 {
                    vec!["A", ""]
                } else {
                    vec!["B", "C"]
                }
            );
            assert_eq!(
                edges(root(page)),
                box_edges(20., 30., 200., if tail && i == 2 { 18. } else { 36. })
            );
        }
        bounds(&p);
    }
}

#[test]
fn matching_parent_and_child_outlines_use_their_own_final_heights() {
    let mut child = table(vec![cell(0, 0, 2, &["A", "B", "C"])], 1);
    child.border_fill_id = 1;
    let mut c = cell(0, 0, 2, &[]);
    c.paragraphs = vec![host("host", child), para("after")];
    let mut parent = table(vec![c], 1);
    parent.border_fill_id = 1;
    let p = run("outer-cut-nested", &doc(parent));
    assert_eq!(p.len(), 3);
    assert_eq!(labels(&p[0]), ["A", "B"]);
    assert_eq!(labels(&p[1]), ["C", "host"]);
    assert_eq!(labels(&p[2]), ["after"]);
    for (i, page) in p.iter().enumerate() {
        let mut expected = if i < 2 {
            box_edges(20., 30., 200., if i == 0 { 36. } else { 18. })
        } else {
            vec![]
        };
        expected.extend(box_edges(20., 30., 200., if i < 2 { 36. } else { 18. }));
        assert_eq!(edges(root(page)), expected);
    }
    bounds(&p);
}

#[test]
fn completed_sibling_keeps_physical_frame_and_shared_edge_only_once() {
    let t = table(
        vec![cell(0, 0, 1, &["L"]), cell(0, 1, 1, &["A", "B", "C", "D"])],
        1,
    );
    let p = run("cut-siblings", &doc(t));
    assert_eq!(p.len(), 2);
    assert_eq!(labels(&p[0]), ["L", "A", "B"]);
    assert_eq!(labels(&p[1]), ["C", "D"]);
    for page in &p {
        assert_eq!(
            edges(root(page)),
            vec![
                [20., 30., 20., 66.],
                [120., 30., 120., 66.],
                [220., 30., 220., 66.],
                [20., 30., 220., 30.],
                [20., 66., 220., 66.]
            ]
        );
        assert_eq!(collect(root(page), "TableCell").len(), 2);
    }
    bounds(&p);
}

#[test]
fn repeated_colspan_header_and_body_cut_use_physical_row_order() {
    let mut header = cell(0, 0, 2, &["title"]);
    header.is_header = true;
    let mut t = table(
        vec![
            header,
            cell(1, 0, 1, &["L"]),
            cell(1, 1, 1, &["A", "B", "C"]),
        ],
        2,
    );
    t.repeat_header = true;
    let p = run("cut-header", &doc(t));
    assert_eq!(p.len(), 3);
    assert_eq!(labels(&p[0]), ["title", "L", "A"]);
    assert_eq!(labels(&p[1]), ["title", "B"]);
    assert_eq!(labels(&p[2]), ["title", "C"]);
    for page in &p {
        assert_eq!(
            edges(root(page)),
            vec![
                [20., 30., 20., 66.],
                [120., 48., 120., 66.],
                [220., 30., 220., 66.],
                [20., 30., 220., 30.],
                [20., 48., 220., 48.],
                [20., 66., 220., 66.]
            ]
        );
    }
    bounds(&p);
}

#[test]
fn nested_cell_cuts_keep_each_frame_and_following_host_content() {
    let child = table(vec![cell(0, 0, 2, &["A", "B", "C"])], 1);
    let mut parent = cell(0, 0, 2, &[]);
    parent.paragraphs = vec![host("host", child), para("after")];
    let p = run("cut-nested", &doc(table(vec![parent], 1)));
    assert_eq!(p.len(), 3);
    assert_eq!(labels(&p[0]), ["A", "B"]);
    assert_eq!(labels(&p[1]), ["C", "host"]);
    assert_eq!(labels(&p[2]), ["after"]);
    for (i, page) in p.iter().enumerate() {
        // Recursive lines are child first, then parent. Child ends halfway through p2;
        // parent's bottom is after the host, not overwritten by child's 18px cut.
        let mut expected = if i < 2 {
            box_edges(20., 30., 200., if i == 0 { 36. } else { 18. })
        } else {
            vec![]
        };
        expected.extend(box_edges(20., 30., 200., if i < 2 { 36. } else { 18. }));
        assert_eq!(edges(root(page)), expected);
    }
    bounds(&p);
}

#[test]
fn vertical_padding_is_consumed_once_and_none_edges_are_not_synthesized_at_cuts() {
    let mut t = table(vec![cell(0, 0, 2, &["A", "B"])], 1);
    t.padding.top = 1350; // 18px top + 36px content + 18px bottom = 72px.
    t.padding.bottom = 1350;
    let mut d = doc(t);
    d.doc_info.border_fills[0].borders[2].line_type = BorderLineType::None;
    let p = run("cut-padding", &d);
    assert_eq!(p.len(), 2);
    for (i, page) in p.iter().enumerate() {
        assert_eq!(labels(page), if i == 0 { vec!["A"] } else { vec!["B"] });
        assert_eq!(
            edges(root(page)),
            vec![
                [20., 30., 20., 66.],
                [220., 30., 220., 66.],
                [20., 66., 220., 66.]
            ]
        );
        assert_eq!(
            collect(root(page), "TextLine")[0]["bbox"]["y"],
            if i == 0 { 48. } else { 30. }
        );
    }
    bounds(&p);
}

#[test]
fn split_conflicting_edges_fail_before_cursor_commit_and_retry_same_fragment() {
    let mut right = cell(0, 1, 1, &["A", "B", "C"]);
    right.border_fill_id = 2;
    let mut d = doc(table(vec![cell(0, 0, 1, &["L"]), right], 1));
    let mut other = d.doc_info.border_fills[0].clone();
    other.borders[0].width = 8;
    d.doc_info.border_fills.push(other);
    let (data, config) = source(&d);
    let mut s = TablePreviewExportSession::from_bytes(&data, &config.to_string()).unwrap();
    for _ in 0..2 {
        assert!(s
            .next_page_json()
            .unwrap_err()
            .to_string()
            .contains("conflicting shared V2 cell borders"));
        assert_eq!(s.emitted_pages(), 0);
    }
}
