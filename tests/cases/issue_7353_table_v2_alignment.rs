//! Fresh synthetic HWPX: geometric alignment invariants, not Hancom fidelity.
//! Parent width200 minus left10/right30 =160px; child width100 leaves60px.
//! Left/center/right offsets are therefore0/30/60, independently of pagination.
use rhwp::model::{
    control::Control,
    document::{Document, Section},
    paragraph::{CharShapeRef, Paragraph},
    shape::{CommonObjAttr, HorzAlign, HorzRelTo, TextWrap, VertRelTo},
    style::{BorderFill, BorderLine, BorderLineType, CharShape, LineSpacingType, ParaShape},
    table::{Cell, Table, TablePageBreak},
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
fn table(width: u32, paragraphs: Vec<Paragraph>) -> Table {
    Table {
        row_count: 1,
        col_count: 1,
        page_break: TablePageBreak::CellBreak,
        common: CommonObjAttr {
            text_wrap: TextWrap::TopAndBottom,
            vert_rel_to: VertRelTo::Para,
            horz_rel_to: HorzRelTo::Para,
            ..Default::default()
        },
        cells: vec![Cell {
            width,
            row_span: 1,
            col_span: 1,
            border_fill_id: 1,
            paragraphs,
            ..Default::default()
        }],
        ..Default::default()
    }
}
fn wrapper(child: Table) -> Table {
    let mut root = table(15000, vec![host("host", child), para("after")]);
    root.padding = Padding {
        left: 750,
        right: 2250,
        ..Default::default()
    };
    root
}
fn aligned(align: HorzAlign) -> Table {
    let mut child = table(7500, vec![para("A"), para("B"), para("C")]);
    child.common.horz_align = align;
    wrapper(child)
}
fn source(root: Table) -> (Vec<u8>, Value) {
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
    let control = parsed.sections[0].paragraphs[0]
        .controls
        .iter()
        .position(|c| matches!(c, Control::Table(_)))
        .unwrap();
    (
        bytes,
        json!({"selection":{"section":0,"paragraph":0,"control":control},"dpi":96,
        "pages":{"width":400,"height":400,"body":{"x":20,"y":30,"width":300,"height":36},"first_y":30},"max_pages":10}),
    )
}
fn run(name: &str, root: Table) -> Vec<Value> {
    let (bytes, options) = source(root);
    let mut s = TablePreviewExportSession::from_bytes(&bytes, &options.to_string()).unwrap();
    let mut pages = Vec::new();
    while let Some(p) = s.next_page_json().unwrap() {
        pages.push(serde_json::from_str(&p).unwrap());
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
fn check(pages: &[Value], child_x: f64) {
    assert_eq!(pages.len(), 3);
    let expected = [
        vec![("A", child_x, 30.0), ("B", child_x, 48.0)],
        vec![("C", child_x, 30.0), ("host", 30.0, 48.0)],
        vec![("after", 30.0, 30.0)],
    ];
    for (i, p) in pages.iter().enumerate() {
        let root = &p["render_tree"]["root"];
        let lines = collect(root, "TextLine");
        let actual: Vec<_> = lines
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
        assert_eq!(actual, expected[i]);
        let cells = collect(root, "TableCell");
        assert_eq!(cells.len(), if i < 2 { 2 } else { 1 });
        assert_eq!(
            cells[0]["bbox"],
            json!({"x":20.0,"y":30.0,"width":200.0,"height":if i==2 {18.0} else {36.0}})
        );
        if i < 2 {
            assert_eq!(
                cells[1]["bbox"],
                json!({"x":child_x,"y":30.0,"width":100.0,"height":if i==0 {36.0} else {18.0}})
            );
        }
        // Paint consumes the same accepted child/parent boxes; no post-fit translation.
        let box_edges = |x: f64, w: f64, h: f64| {
            vec![
                [x, 30.0, x, 30.0 + h],
                [x + w, 30.0, x + w, 30.0 + h],
                [x, 30.0, x + w, 30.0],
                [x, 30.0 + h, x + w, 30.0 + h],
            ]
        };
        let mut edges = if i < 2 {
            box_edges(child_x, 100.0, if i == 0 { 36.0 } else { 18.0 })
        } else {
            vec![]
        };
        edges.extend(box_edges(20.0, 200.0, if i == 2 { 18.0 } else { 36.0 }));
        assert_eq!(
            collect(root, "Line")
                .iter()
                .map(|n| ["x1", "y1", "x2", "y2"]
                    .map(|k| n["node_type"]["Line"][k].as_f64().unwrap()))
                .collect::<Vec<_>>(),
            edges
        );
    }
}
#[test]
fn left_aligned_child_preserves_existing_origin_and_continuation() {
    check(&run("align-left", aligned(HorzAlign::Left)), 30.0);
}
#[test]
fn centered_child_uses_padded_parent_width_on_every_fragment() {
    check(&run("align-center", aligned(HorzAlign::Center)), 60.0);
}
#[test]
fn right_aligned_child_uses_padded_parent_width_on_every_fragment() {
    check(&run("align-right", aligned(HorzAlign::Right)), 90.0);
}

#[test]
fn individual_cell_padding_overrides_table_padding_for_alignment() {
    let mut root = aligned(HorzAlign::Center);
    root.padding.left = 4500; // Ignored: effective margins come from the cell.
    root.cells[0].apply_inner_margin = true;
    root.cells[0].padding = Padding {
        left: 750,
        right: 2250,
        ..Default::default()
    };
    check(&run("align-cell-padding", root), 60.0);
}

#[test]
fn nested_alignment_accumulates_each_local_origin_once() {
    let mut leaf = table(3000, vec![para("A"), para("B"), para("C")]); //40px
    leaf.common.horz_align = HorzAlign::Center;
    let mut middle = table(7500, vec![host("middle", leaf)]); //100px
    middle.common.horz_align = HorzAlign::Right;
    middle.padding = Padding {
        left: 750,
        right: 750,
        ..Default::default()
    }; //80px inner
    let pages = run("align-deep", wrapper(middle));
    let expected = [
        vec![("A", 120.0, 30.0), ("B", 120.0, 48.0)],
        vec![("C", 120.0, 30.0), ("middle", 100.0, 48.0)],
        vec![("host", 30.0, 30.0), ("after", 30.0, 48.0)],
    ];
    assert_eq!(pages.len(), 3);
    for (i, p) in pages.iter().enumerate() {
        let root = &p["render_tree"]["root"];
        let lines = collect(root, "TextLine");
        assert_eq!(
            lines
                .iter()
                .map(|n| {
                    let runs = collect(n, "TextRun");
                    (
                        runs[0]["node_type"]["TextRun"]["text"].as_str().unwrap(),
                        n["bbox"]["x"].as_f64().unwrap(),
                        n["bbox"]["y"].as_f64().unwrap(),
                    )
                })
                .collect::<Vec<_>>(),
            expected[i]
        );
        let cells = collect(root, "TableCell");
        assert_eq!(cells.len(), if i < 2 { 3 } else { 1 });
        for (j, (x, w, h)) in [
            (20.0, 200.0, 36.0),
            (90.0, 100.0, 36.0),
            (120.0, 40.0, if i == 0 { 36.0 } else { 18.0 }),
        ]
        .iter()
        .take(cells.len())
        .enumerate()
        {
            assert_eq!(
                cells[j]["bbox"],
                json!({"x":x,"y":30.0,"width":w,"height":h})
            );
        }
    }
}

#[test]
fn right_aligned_repeated_child_header_keeps_body_cursor_and_host() {
    let mut child = table(7500, vec![para("title")]);
    child.common.horz_align = HorzAlign::Right;
    child.repeat_header = true;
    child.row_count = 3;
    child.cells[0].is_header = true;
    for (row, label) in [(1, "A"), (2, "B")] {
        let mut cell = table(7500, vec![para(label)]).cells.remove(0);
        cell.row = row;
        child.cells.push(cell);
    }
    let pages = run("align-header", wrapper(child));
    assert_eq!(pages.len(), 3);
    for (i, p) in pages.iter().enumerate() {
        let root = &p["render_tree"]["root"];
        let labels = collect(root, "TextRun");
        assert_eq!(
            labels
                .iter()
                .map(|n| n["node_type"]["TextRun"]["text"].as_str().unwrap())
                .collect::<Vec<_>>(),
            [
                vec!["title", "A"],
                vec!["title", "B"],
                vec!["host", "after"]
            ][i]
        );
        let lines = collect(root, "TextLine");
        assert_eq!(
            lines
                .iter()
                .map(|n| n["bbox"]["x"].as_f64().unwrap())
                .collect::<Vec<_>>(),
            vec![if i < 2 { 90.0 } else { 30.0 }; 2]
        );
        assert_eq!(
            lines
                .iter()
                .map(|n| n["bbox"]["y"].as_f64().unwrap())
                .collect::<Vec<_>>(),
            [30.0, 48.0]
        );
        let cells = collect(root, "TableCell");
        assert_eq!(cells.len(), if i < 2 { 3 } else { 1 });
        assert_eq!(
            cells[0]["bbox"],
            json!({"x":20.0,"y":30.0,"width":200.0,"height":36.0})
        );
        for (row, c) in cells.iter().skip(1).enumerate() {
            assert_eq!(
                c["bbox"],
                json!({"x":90.0,"y":30.0+18.0*row as f64,"width":100.0,"height":18.0})
            );
        }
    }
}

#[test]
fn unresolved_anchor_and_oversized_child_are_still_rejected() {
    for case in 0..8 {
        let mut child = table(7500, vec![para("A")]);
        child.common.horz_align = HorzAlign::Center;
        match case {
            0 => child.common.horz_align = HorzAlign::Inside,
            1 => child.common.horz_align = HorzAlign::Outside,
            2 => child.common.horizontal_offset = 1,
            3 => child.common.vertical_offset = 1,
            4 => child.common.horz_rel_to = HorzRelTo::Page,
            5 => child.common.treat_as_char = true,
            6 => child.common.text_wrap = TextWrap::Square,
            7 => child.cells[0].width = 12075, //161px does not fit160px, no clamp.
            _ => unreachable!(),
        }
        let (bytes, options) = source(wrapper(child));
        let error = TablePreviewExportSession::from_bytes(&bytes, &options.to_string())
            .err()
            .expect("must reject");
        let text = format!("{error:?}");
        assert!(
            text.contains(if case == 7 {
                "ContentWidth"
            } else {
                "Unsupported"
            }),
            "case{case}: {text}"
        );
    }
}

#[test]
fn caller_composed_offsets_are_validated_before_pagination() {
    use rhwp::renderer::table_v2::*;
    use std::sync::Arc;
    for offset_x in [-1.0, f64::NAN, f64::INFINITY, 61.0] {
        let plan = TableContentPlan::from_flow_rows(
            vec![100.0],
            vec![FlowRowInput {
                cells: vec![FlowCellInput {
                    padding: Insets::default(),
                    minimum_height: 18.0,
                    width: 100.0,
                    blocks: vec![],
                }],
            }],
            0.0,
            SplitPolicy::WithinCells,
        )
        .unwrap();
        let result = TableContentPlan::from_flow_rows(
            vec![160.0],
            vec![FlowRowInput {
                cells: vec![FlowCellInput {
                    padding: Insets::default(),
                    minimum_height: 0.0,
                    width: 160.0,
                    blocks: vec![FlowBlock::Table {
                        owner: ControlOwner {
                            paragraph: 0,
                            control: 0,
                        },
                        offset_x,
                        plan: Arc::new(plan),
                    }],
                }],
            }],
            0.0,
            SplitPolicy::WithinCells,
        );
        assert!(result.is_err(), "invalid offset {offset_x} accepted");
    }
}
