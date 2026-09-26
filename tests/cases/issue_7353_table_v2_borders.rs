//! Whole-cell edge contracts use final output, not the border helper's result.
use rhwp::{
    model::{
        control::Control,
        document::{Document, Section},
        paragraph::{CharShapeRef, Paragraph},
        shape::{CommonObjAttr, HorzRelTo, TextWrap, VertRelTo},
        style::{
            BorderFill, BorderLine, BorderLineType, CharShape, Fill, FillType, LineSpacingType,
            ParaShape, SolidFill,
        },
        table::{Cell, Table, TablePageBreak},
    },
    renderer::table_v2::{
        Rect, TablePreviewExportSession, TablePreviewPages, TablePreviewSession, TableSelection,
    },
};
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
fn grid(rows: u16) -> Table {
    let mut cells = vec![Cell {
        row_span: 1,
        col_span: 2,
        width: 15000,
        is_header: true,
        border_fill_id: 1,
        paragraphs: vec![para("title")],
        ..Default::default()
    }];
    for row in 1..rows {
        for (col, label) in [(0, "L"), (1, "R")] {
            cells.push(Cell {
                row,
                col,
                row_span: 1,
                col_span: 1,
                width: 7500,
                border_fill_id: 1,
                paragraphs: vec![para(&format!("{label}{row}"))],
                ..Default::default()
            });
        }
    }
    Table {
        row_count: rows,
        col_count: 2,
        repeat_header: true,
        page_break: TablePageBreak::RowBreak,
        common: CommonObjAttr {
            text_wrap: TextWrap::TopAndBottom,
            horz_rel_to: HorzRelTo::Para,
            vert_rel_to: VertRelTo::Para,
            ..Default::default()
        },
        cells,
        ..Default::default()
    }
}
fn border() -> BorderFill {
    BorderFill {
        borders: [BorderLine {
            line_type: BorderLineType::Solid,
            width: 7,
            color: 0x332211,
        }; 4],
        fill: Fill {
            fill_type: FillType::Solid,
            solid: Some(SolidFill {
                background_color: 0xEEFFEE,
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    }
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
    d.doc_info.border_fills.push(border());
    d.sections = vec![Section {
        paragraphs: vec![host("", t)],
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
fn capture(name: &str, data: &[u8], config: &Value, pages: &[Value]) {
    if let Ok(dir) = std::env::var("ISSUE7353_EXPORT_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/{name}.hwpx"), data).unwrap();
        std::fs::write(format!("{dir}/{name}.options.json"), config.to_string()).unwrap();
        std::fs::write(
            format!("{dir}/{name}.native.json"),
            serde_json::to_vec_pretty(pages).unwrap(),
        )
        .unwrap();
    }
}
fn pages(name: &str, d: &Document) -> Vec<Value> {
    let (data, config) = source(d);
    let mut s = TablePreviewExportSession::from_bytes(&data, &config.to_string()).unwrap();
    let mut result = Vec::new();
    while let Some(p) = s.next_page_json().unwrap() {
        result.push(serde_json::from_str(&p).unwrap());
    }
    assert_eq!(s.emitted_pages() as usize, result.len());
    assert!(s.next_page_json().unwrap().is_none());
    capture(name, &data, &config, &result);
    result
}
fn collect<'a>(n: &'a Value, kind: &str) -> Vec<&'a Value> {
    let mut nodes = if n["node_type"].get(kind).is_some() {
        vec![n]
    } else {
        vec![]
    };
    for child in n["children"].as_array().unwrap() {
        nodes.extend(collect(child, kind));
    }
    nodes
}
fn root(p: &Value) -> &Value {
    &p["render_tree"]["root"]
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn coords(n: &Value) -> [f64; 4] {
    let l = &n["node_type"]["Line"];
    ["x1", "y1", "x2", "y2"].map(|k| l[k].as_f64().unwrap())
}
fn verify_grid(p: &Value) {
    let lines = collect(root(p), "Line");
    let expected = [
        [20., 30., 20., 66.],
        [120., 48., 120., 66.],
        [220., 30., 220., 66.],
        [20., 30., 220., 30.],
        [20., 48., 220., 48.],
        [20., 66., 220., 66.],
    ];
    assert_eq!(
        lines.len(),
        expected.len(),
        "shared lines must not be painted twice"
    );
    for (line, expected) in lines.iter().zip(expected) {
        assert_eq!(coords(line), expected);
        near(
            line["node_type"]["Line"]["style"]["width"]
                .as_f64()
                .unwrap(),
            1.92,
        );
        assert_eq!(line["node_type"]["Line"]["style"]["color"], 0x332211);
        // Butt-cap ink is centered on a boundary; half a stroke on each side.
        if expected[0] == expected[2] {
            near(line["bbox"]["x"].as_f64().unwrap(), expected[0] - 0.96);
            near(line["bbox"]["width"].as_f64().unwrap(), 1.92);
        } else {
            near(line["bbox"]["y"].as_f64().unwrap(), expected[1] - 0.96);
            near(line["bbox"]["height"].as_f64().unwrap(), 1.92);
        }
    }
    for table in collect(root(p), "Table") {
        let children = table["children"].as_array().unwrap();
        if let Some(first_edge) = children
            .iter()
            .position(|n| n["node_type"].get("Line").is_some())
        {
            assert!(
                children[first_edge..]
                    .iter()
                    .all(|n| n["node_type"].get("Line").is_some()),
                "cell fill must precede edges"
            );
        }
    }
}

#[test]
fn whole_colspan_grid_unions_shared_edges_after_backgrounds() {
    let result = pages("border-grid", &doc(grid(2)));
    assert_eq!(result.len(), 1);
    verify_grid(&result[0]);
    assert_eq!(
        collect(root(&result[0]), "Table")[0]["bbox"]["height"],
        36.0
    );
}

#[test]
fn matching_table_outline_is_unioned_with_cells_and_repeated_headers() {
    for (rows, name) in [(2, "outer-border-grid"), (3, "outer-border-header")] {
        let mut t = grid(rows);
        t.border_fill_id = 1;
        let result = pages(name, &doc(t));
        assert_eq!(result.len(), usize::from(rows - 1));
        for (i, page) in result.iter().enumerate() {
            verify_grid(page);
            let labels: Vec<_> = collect(root(page), "TextRun")
                .iter()
                .map(|n| n["node_type"]["TextRun"]["text"].as_str().unwrap())
                .collect();
            assert_eq!(
                labels,
                ["title".into(), format!("L{}", i + 1), format!("R{}", i + 1)]
            );
        }
    }
}

#[test]
fn nested_matching_outline_keeps_host_and_following_paragraph() {
    let mut child = grid(3);
    child.border_fill_id = 1;
    let mut parent = grid(1);
    parent.repeat_header = false;
    parent.page_break = TablePageBreak::CellBreak;
    parent.cells[0].is_header = false;
    parent.cells[0].border_fill_id = 0;
    parent.cells[0].paragraphs = vec![host("host", child), para("after")];
    let result = pages("outer-border-nested", &doc(parent));
    assert_eq!(result.len(), 3);
    verify_grid(&result[0]);
    verify_grid(&result[1]);
    assert!(collect(root(&result[2]), "Line").is_empty());
    let labels: Vec<_> = collect(root(&result[2]), "TextRun")
        .iter()
        .map(|n| n["node_type"]["TextRun"]["text"].as_str().unwrap())
        .collect();
    assert_eq!(labels, ["host", "after"]);
}

#[test]
fn table_outline_does_not_override_absent_or_different_cell_edges() {
    for (name, absent) in [("outer-missing", true), ("outer-conflict", false)] {
        let mut t = one_row();
        t.border_fill_id = 1;
        t.cells[0].border_fill_id = 2;
        let mut d = doc(t);
        let mut b = border();
        if absent {
            b.borders[0].line_type = BorderLineType::None;
        } else {
            b.borders[0].width = 8;
        }
        d.doc_info.border_fills.push(b);
        let (data, config) = source(&d);
        capture(name, &data, &config, &[]);
        let result = pages(name, &d);
        let lines = collect(root(&result[0]), "Line");
        let left: Vec<_> = lines
            .iter()
            .filter(|n| {
                let l = &n["node_type"]["Line"];
                l["x1"] == 20.0 && l["x2"] == 20.0
            })
            .collect();
        if absent {
            assert!(left.is_empty());
        } else {
            assert_eq!(left.len(), 1);
            near(
                left[0]["node_type"]["Line"]["style"]["width"]
                    .as_f64()
                    .unwrap(),
                14.0 * 96.0 / 600.0, // width ID8 = 0.6mm -> fourteen 600dpi units
            );
        }
    }
}

#[test]
fn table_only_outline_is_explicitly_unresolved_not_silently_drawn() {
    let mut t = one_row();
    t.border_fill_id = 1;
    for c in &mut t.cells {
        c.border_fill_id = 0;
    }
    let mut s = direct(&doc(t), 96.).unwrap();
    for _ in 0..2 {
        assert!(s
            .next_page()
            .err()
            .unwrap()
            .to_string()
            .contains("V2 table/cell outline disagreement"));
        assert_eq!(s.emitted_pages(), 0);
    }
}

#[test]
fn explicit_all_none_cells_are_not_missing_outline_references() {
    let mut t = one_row();
    t.border_fill_id = 1;
    for c in &mut t.cells {
        c.border_fill_id = 2;
    }
    let mut d = doc(t);
    let mut b = border();
    for e in &mut b.borders {
        e.line_type = BorderLineType::None;
    }
    d.doc_info.border_fills.push(b);
    let result = pages("all-none-cell-ownership", &d);
    assert_eq!(result.len(), 1);
    assert!(collect(root(&result[0]), "Line").is_empty());
    assert_eq!(collect(root(&result[0]), "TextRun").len(), 2);
}

#[test]
fn normal_saved_title_preserves_double_box_open_gap_and_following_body() {
    use rhwp::renderer::table_v2::DocumentV2Session;
    let input = include_bytes!("../fixtures/issue7353_double_review/title-saved.hwp");
    let mut session = DocumentV2Session::from_bytes(
        input,
        r#"{"dpi":96,"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}"#,
    )
    .unwrap();
    let p: Value = serde_json::from_str(&session.next_page_json().unwrap().unwrap()).unwrap();
    let child = collect(root(&p), "Table")
        .into_iter()
        .find(|n| n["node_type"]["Table"]["col_count"] == 3)
        .unwrap();
    // Source widths 2639,1303,17259 HU. PDF confirms no horizontal edge
    // across the middle spacer, while the number's double box stays visible.
    let x = child["bbox"]["x"].as_f64().unwrap();
    let gap_mid = x + (2639.0 + 1303.0 / 2.0) / 75.0;
    let lines = collect(child, "Line");
    assert_eq!(lines.len(), 12); // eight number-box pens, four title edges
    for line in &lines {
        let [x1, y1, x2, y2] = coords(line);
        assert!(
            !(y1 == y2 && x1 < gap_mid && x2 > gap_mid),
            "table outline must not bridge the spacer"
        );
    }
    near(child["bbox"]["width"].as_f64().unwrap(), 21201.0 / 75.0);
    near(child["bbox"]["height"].as_f64().unwrap(), 2414.0 / 75.0);
    let texts = collect(root(&p), "TextRun");
    assert!(texts
        .iter()
        .any(|n| n["node_type"]["TextRun"]["text"] == "1"));
    assert!(texts.last().unwrap()["node_type"]["TextRun"]["text"]
        .as_str()
        .unwrap()
        .starts_with("AFTER CELL"));
    let parent = collect(root(&p), "Table")[0];
    assert!(
        texts.last().unwrap()["bbox"]["y"].as_f64().unwrap()
            >= parent["bbox"]["y"].as_f64().unwrap() + parent["bbox"]["height"].as_f64().unwrap()
    );
    assert!(session.next_page_json().unwrap().is_none());
}

#[test]
fn explicit_none_cell_edge_is_preserved_at_each_physical_cut() {
    let mut t = grid(3);
    t.border_fill_id = 1;
    t.repeat_header = false;
    for c in &mut t.cells {
        c.is_header = false;
        if c.row == 2 {
            c.border_fill_id = 2;
        }
    }
    let mut d = doc(t);
    let mut b = border();
    b.borders[2].line_type = BorderLineType::None;
    d.doc_info.border_fills.push(b);
    // Full source outer top/bottom agree, but the second fragment's top does not.
    let (data, config) = source(&d);
    capture("outer-continuation", &data, &config, &[]);
    let mut s = TablePreviewExportSession::from_bytes(&data, &config.to_string()).unwrap();
    let p: Value = serde_json::from_str(&s.next_page_json().unwrap().unwrap()).unwrap();
    verify_grid(&p);
    let p: Value = serde_json::from_str(&s.next_page_json().unwrap().unwrap()).unwrap();
    assert!(collect(root(&p), "Line").iter().all(|n| {
        let l = &n["node_type"]["Line"];
        !(l["y1"] == 30.0 && l["y2"] == 30.0)
    }));
    assert_eq!(collect(root(&p), "TextRun").len(), 2);
    assert!(s.next_page_json().unwrap().is_none());
}

#[test]
fn none_table_outline_does_not_suppress_explicit_cell_lines() {
    let mut t = one_row();
    t.border_fill_id = 2;
    t.cells[0].border_fill_id = 0;
    let mut d = doc(t);
    // BorderLineType::default() is Solid, so None must be explicit in this input.
    d.doc_info.border_fills.push(BorderFill {
        borders: [BorderLine {
            line_type: BorderLineType::None,
            ..Default::default()
        }; 4],
        ..Default::default()
    });
    let (data, _) = source(&d);
    assert!(
        rhwp::parse_document(&data).unwrap().doc_info.border_fills[1]
            .borders
            .iter()
            .all(|edge| edge.line_type == BorderLineType::None)
    );
    let p = pages("outer-border-one-sided", &d);
    assert_eq!(p.len(), 1);
    assert_eq!(
        collect(root(&p[0]), "Line")
            .iter()
            .map(|n| coords(n))
            .collect::<Vec<_>>(),
        [
            [120., 30., 120., 48.],
            [220., 30., 220., 48.],
            [120., 30., 220., 30.],
            [120., 48., 220., 48.]
        ]
    );
}

#[test]
fn repeated_headers_use_fragment_row_order_and_preserve_body_units() {
    let result = pages("border-header", &doc(grid(3)));
    assert_eq!(result.len(), 2);
    for (i, p) in result.iter().enumerate() {
        verify_grid(p);
        let labels: Vec<_> = collect(root(p), "TextRun")
            .iter()
            .map(|n| n["node_type"]["TextRun"]["text"].as_str().unwrap())
            .collect();
        assert_eq!(
            labels,
            ["title".into(), format!("L{}", i + 1), format!("R{}", i + 1)]
        );
    }
}

#[test]
fn nested_whole_cell_edges_survive_parent_cell_continuations_and_following_text() {
    let mut parent = grid(1);
    parent.repeat_header = false;
    parent.page_break = TablePageBreak::CellBreak;
    parent.cells[0].is_header = false;
    parent.cells[0].border_fill_id = 0;
    parent.cells[0].paragraphs = vec![host("host", grid(3)), para("after")];
    let result = pages("border-nested", &doc(parent));
    assert_eq!(result.len(), 3);
    verify_grid(&result[0]);
    verify_grid(&result[1]);
    assert!(collect(root(&result[2]), "Line").is_empty());
    let labels: Vec<_> = collect(root(&result[2]), "TextRun")
        .iter()
        .map(|n| n["node_type"]["TextRun"]["text"].as_str().unwrap())
        .collect();
    assert_eq!(labels, ["host", "after"]);
}

fn one_row() -> Table {
    let mut t = grid(2);
    t.row_count = 1;
    t.repeat_header = false;
    t.cells.remove(0);
    for c in &mut t.cells {
        c.row = 0;
    }
    t
}

#[test]
fn one_sided_shared_edge_is_preserved_without_an_invented_neighbor_frame() {
    let mut t = one_row();
    t.cells[0].border_fill_id = 0;
    let result = pages("border-one-sided", &doc(t));
    let lines = collect(root(&result[0]), "Line");
    assert_eq!(
        lines.iter().map(|n| coords(n)).collect::<Vec<_>>(),
        [
            [120., 30., 120., 48.],
            [220., 30., 220., 48.],
            [120., 30., 220., 30.],
            [120., 48., 220., 48.]
        ]
    );
}

fn direct(
    d: &Document,
    dpi: f64,
) -> Result<TablePreviewSession, rhwp::renderer::table_v2::TablePreviewError> {
    TablePreviewSession::from_document(
        d,
        TableSelection {
            section: 0,
            paragraph: 0,
            control: 0,
        },
        dpi,
        TablePreviewPages {
            width: 600.,
            height: 600.,
            body: Rect {
                x: 20.,
                y: 30.,
                width: 550.,
                height: 500.,
            },
            first_y: 30.,
        },
        10,
    )
}

#[test]
fn all_sixteen_widths_follow_independent_hancom_600dpi_grid_at_two_dpis() {
    // samples/issue6913/README.md: independent engine PDF stroke measurements.
    let grid_units = [
        2., 3., 4., 5., 6., 7., 9., 12., 14., 17., 24., 35., 47., 71., 95., 118.,
    ];
    for dpi in [96., 192.] {
        for (width, units) in grid_units.into_iter().enumerate() {
            let mut t = grid(1);
            t.repeat_header = false;
            t.cells[0].is_header = false;
            t.page_break = TablePageBreak::None;
            let mut d = doc(t);
            for edge in &mut d.doc_info.border_fills[0].borders {
                edge.width = width as u8;
            }
            let page = direct(&d, dpi).unwrap().next_page().unwrap().unwrap();
            let tree = serde_json::to_value(&page.tree).unwrap();
            let lines = collect(&tree["root"], "Line");
            assert_eq!(lines.len(), 4);
            for line in lines {
                near(
                    line["node_type"]["Line"]["style"]["width"]
                        .as_f64()
                        .unwrap(),
                    units * dpi / 600.,
                );
            }
        }
    }
}

#[test]
fn double_edges_keep_two_thin_strokes_and_open_junction_gaps() {
    // Independent Hancom grid PDFs: width indices 0/3/7/11, per-pen
    // thickness and centre separation. Not the generic shape 30/40/30 rule.
    for (width, pen_units) in [(0, 1.), (3, 1.), (7, 3.), (11, 9.)] {
        for dpi in [96., 192.] {
            let mut d = doc(grid(2));
            for edge in &mut d.doc_info.border_fills[0].borders {
                edge.line_type = BorderLineType::Double;
                edge.width = width;
            }
            let page = direct(&d, dpi).unwrap().next_page().unwrap().unwrap();
            let tree = serde_json::to_value(&page.tree).unwrap();
            let cells = collect(&tree["root"], "TableCell");
            let left = cells[0]["bbox"]["x"].as_f64().unwrap();
            let top = cells[0]["bbox"]["y"].as_f64().unwrap();
            let right = left + cells[0]["bbox"]["width"].as_f64().unwrap();
            let pen = pen_units * dpi / 600.;
            let offset = 1.5 * pen;
            let lines = collect(&tree["root"], "Line");
            assert!(lines.len() >= 12);
            for line in &lines {
                near(
                    line["node_type"]["Line"]["style"]["width"]
                        .as_f64()
                        .unwrap(),
                    pen,
                );
                assert_eq!(line["node_type"]["Line"]["style"]["line_type"], "Single");
            }
            // Outer/inner corner centres are offset by 1.5 pens; outer
            // horizontal reaches the outside ink, inner starts inside it.
            for (sign, start, stop) in [
                (-1., left - offset - pen / 2., right + offset + pen / 2.),
                (1., left + offset - pen / 2., right - offset + pen / 2.),
            ] {
                let horizontal: Vec<_> = lines
                    .iter()
                    .filter(|l| {
                        let c = coords(l);
                        (c[1] - (top + sign * offset)).abs() < 1e-8 && c[1] == c[3]
                    })
                    .collect();
                near(coords(horizontal[0])[0], start);
                near(coords(horizontal.last().unwrap())[2], stop);
            }
            // Shared edge below the colspan meets a T-junction. The two
            // vertical pens stop at the lower horizontal pen, never cross
            // the white space between that boundary's parallel strokes.
            if cells.len() == 3 {
                let seam_x = cells[2]["bbox"]["x"].as_f64().unwrap();
                let seam_y = cells[2]["bbox"]["y"].as_f64().unwrap();
                for sign in [-1., 1.] {
                    let vertical: Vec<_> = lines
                        .iter()
                        .filter(|l| {
                            let c = coords(l);
                            c[0] == c[2] && (c[0] - (seam_x + sign * offset)).abs() < 1e-8
                        })
                        .collect();
                    assert_eq!(vertical.len(), 1, "shared strokes painted once");
                    near(coords(vertical[0])[1], seam_y + offset - pen / 2.);
                }
            }
        }
    }
}

#[test]
fn saved_grid_width_roundoff_does_not_reject_an_exact_hwpunit_lane() {
    let mut d = rhwp::parse_document(include_bytes!(
        "../fixtures/issue7353_double_review/grid-7-saved.hwp"
    ))
    .unwrap();
    // Remove the new paint dependency to isolate the pre-existing width bug.
    // The unchanged saved cell is 15000HU, with left/right padding 510HU.
    for b in &mut d.doc_info.border_fills {
        for e in &mut b.borders {
            if e.line_type == BorderLineType::Double {
                e.line_type = BorderLineType::Solid;
            }
        }
    }
    let bytes = rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap();
    let mut session = rhwp::renderer::table_v2::DocumentV2Session::from_bytes(
        &bytes,
        r#"{"dpi":96,"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}"#,
    )
    .unwrap();
    let p: Value = serde_json::from_str(&session.next_page_json().unwrap().unwrap()).unwrap();
    let cells = collect(root(&p), "TableCell");
    assert_eq!(cells.len(), 5);
    let lines = collect(root(&p), "TextLine");
    assert!(lines
        .iter()
        .any(|l| (l["bbox"]["width"].as_f64().unwrap() - 13980. / 75.).abs() < 1e-12));
    assert!(session.next_page_json().unwrap().is_none());
}

#[test]
fn normal_saved_double_grids_preserve_cross_gaps_and_following_content() {
    for (name, pen) in [
        ("grid-0", 0.16),
        ("grid-3", 0.16),
        ("grid-7", 0.48),
        ("grid-11", 1.44),
    ] {
        let data = std::fs::read(format!(
            "{}/tests/fixtures/issue7353_double_review/{name}-saved.hwp",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let mut s = rhwp::renderer::table_v2::DocumentV2Session::from_bytes(
            &data,
            r#"{"dpi":96,"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}"#,
        )
        .unwrap();
        let p: Value = serde_json::from_str(&s.next_page_json().unwrap().unwrap()).unwrap();
        assert!(s.next_page_json().unwrap().is_none());
        let tables = collect(root(&p), "Table");
        assert_eq!(tables.len(), 2);
        let child = tables[1];
        let cells = collect(child, "TableCell");
        let cross_x = cells[3]["bbox"]["x"].as_f64().unwrap();
        let cross_y = cells[3]["bbox"]["y"].as_f64().unwrap();
        let lines = collect(child, "Line");
        assert_eq!(
            lines.len(),
            24,
            "12 physical half-edges, two pens each; no doubled shared cell ink"
        );
        for line in lines {
            near(
                line["node_type"]["Line"]["style"]["width"]
                    .as_f64()
                    .unwrap(),
                pen,
            );
            let c = coords(line);
            if c[0] == c[2] && (c[0] - cross_x).abs() < 2. * pen {
                assert!(c[3] <= cross_y - pen + 1e-9 || c[1] >= cross_y + pen - 1e-9);
            }
            if c[1] == c[3] && (c[1] - cross_y).abs() < 2. * pen {
                assert!(c[2] <= cross_x - pen + 1e-9 || c[0] >= cross_x + pen - 1e-9);
            }
        }
        let text = collect(root(&p), "TextRun");
        for label in ["R0 C0", "R0 C1", "R1 C0", "R1 C1", "AFTER CELL"] {
            assert_eq!(
                text.iter()
                    .filter(|n| n["node_type"]["TextRun"]["text"]
                        .as_str()
                        .unwrap()
                        .contains(label))
                    .count(),
                1
            );
        }
    }
}

#[test]
fn double_repeated_header_keeps_the_same_fragments_as_solid() {
    let solid = doc(grid(3));
    let before = pages("double-header-control", &solid);
    let mut double = solid;
    for e in &mut double.doc_info.border_fills[0].borders {
        e.line_type = BorderLineType::Double;
    }
    let after = pages("double-header", &double);
    assert_eq!(before.len(), after.len());
    for (a, b) in before.iter().zip(&after) {
        for kind in ["TableCell", "TextLine", "TextRun"] {
            let geometry = |p: &Value| {
                collect(root(p), kind)
                    .into_iter()
                    .map(|n| (n["bbox"].clone(), n["node_type"].clone()))
                    .collect::<Vec<_>>()
            };
            assert_eq!(geometry(a), geometry(b));
        }
    }
}

#[test]
fn double_mixed_junctions_and_zone_perimeters_remain_explicit() {
    for variant in 0..3 {
        let mut t = grid(1);
        t.repeat_header = false;
        t.cells[0].is_header = false;
        let mut d = doc(t);
        for e in &mut d.doc_info.border_fills[0].borders {
            e.line_type = BorderLineType::Double;
        }
        match variant {
            0 => d.doc_info.border_fills[0].borders[0].line_type = BorderLineType::Solid,
            1 => d.doc_info.border_fills[0].borders[0].color = 0x112233,
            _ => d.doc_info.border_fills[0].borders[0].width = 11,
        }
        let mut s = direct(&d, 96.).unwrap();
        for _ in 0..2 {
            assert!(s
                .next_page()
                .err()
                .unwrap()
                .to_string()
                .contains("mixed double-border junction"));
        }
    }
    let mut t = grid(1);
    t.zones.push(rhwp::model::table::TableZone {
        start_row: 0,
        start_col: 0,
        end_row: 0,
        end_col: 0,
        border_fill_id: 1,
    });
    let mut d = doc(t);
    for e in &mut d.doc_info.border_fills[0].borders {
        e.line_type = BorderLineType::Double;
    }
    assert!(direct(&d, 96.)
        .err()
        .unwrap()
        .to_string()
        .contains("double zone perimeter"));
}

#[test]
fn fractional_grid_uses_one_shared_topological_boundary() {
    let mut t = grid(2);
    t.cells[0].width = 15001;
    t.cells[2].width = 7501;
    let page = direct(&doc(t), 101.3)
        .unwrap()
        .next_page()
        .unwrap()
        .unwrap();
    let tree = serde_json::to_value(&page.tree).unwrap();
    let lines = collect(&tree["root"], "Line");
    assert_eq!(lines.len(), 6);
    near(coords(lines[1])[0], 20. + 7500. * 101.3 / 7200.);
    near(coords(lines[4])[2], 20. + 15001. * 101.3 / 7200.);
}

#[test]
fn conflicting_edges_reject_before_commit_and_remain_retryable() {
    let mut t = one_row();
    t.cells[1].border_fill_id = 2;
    let mut d = doc(t);
    let mut other = border();
    other.borders[0].width = 8;
    other.borders[0].color = 0x445566;
    d.doc_info.border_fills.push(other.clone());
    let (data, config) = source(&d);
    capture("border-conflict", &data, &config, &[]);
    let mut s = TablePreviewExportSession::from_bytes(&data, &config.to_string()).unwrap();
    for _ in 0..2 {
        assert!(s
            .next_page_json()
            .unwrap_err()
            .to_string()
            .contains("conflicting shared V2 cell borders"));
        assert_eq!(s.emitted_pages(), 0);
    }

    // The same paint error after an accepted fragment must retain that fragment
    // and retry the failing body row, not consume it or repeat the first page.
    let mut t = grid(3);
    t.cells.last_mut().unwrap().border_fill_id = 2;
    let mut d = doc(t);
    d.doc_info.border_fills.push(other);
    let (data, config) = source(&d);
    let mut s = TablePreviewExportSession::from_bytes(&data, &config.to_string()).unwrap();
    let first: Value = serde_json::from_str(&s.next_page_json().unwrap().unwrap()).unwrap();
    verify_grid(&first);
    for _ in 0..2 {
        assert!(s
            .next_page_json()
            .unwrap_err()
            .to_string()
            .contains("conflicting shared V2 cell borders"));
        assert_eq!(s.emitted_pages(), 1);
    }
}

#[test]
fn same_color_solid_shared_edges_union_width_per_interval() {
    let base = doc(grid(2));
    let baseline = pages("shared-width-control", &base);
    for reverse in [false, true] {
        let mut d = base.clone();
        let mut thick = border();
        thick.borders[2].width = 8;
        d.doc_info.border_fills.push(thick);
        let Control::Table(t) = &mut d.sections[0].paragraphs[0].controls[0] else {
            panic!()
        };
        t.cells[1].border_fill_id = 2;
        if reverse {
            t.cells.reverse();
        }
        let actual = pages("shared-width", &d);
        assert_eq!(actual.len(), 1);
        let lines = collect(root(&actual[0]), "Line");
        let shared: Vec<_> = lines
            .iter()
            .filter(|l| {
                let c = coords(l);
                c[1] == 48. && c[3] == 48.
            })
            .collect();
        assert_eq!(shared.len(), 2);
        assert_eq!(coords(shared[0]), [20., 48., 120., 48.]);
        assert_eq!(coords(shared[1]), [120., 48., 220., 48.]);
        // 0.6/0.5mm round to 14/12 units on Hancom's 600dpi grid: 2.24/1.92px.
        near(
            shared[0]["node_type"]["Line"]["style"]["width"]
                .as_f64()
                .unwrap(),
            2.24,
        );
        near(
            shared[1]["node_type"]["Line"]["style"]["width"]
                .as_f64()
                .unwrap(),
            1.92,
        );
        for kind in ["Table", "TableCell", "TextLine", "TextRun"] {
            let boxes = |p: &Value| {
                collect(root(p), kind)
                    .iter()
                    .map(|n| n["bbox"].clone())
                    .collect::<Vec<_>>()
            };
            assert_eq!(boxes(&actual[0]), boxes(&baseline[0]), "{kind}");
        }
    }
}

#[test]
fn unsupported_complex_edges_are_explicit_even_when_geometry_fits() {
    let mut t = grid(1);
    t.repeat_header = false;
    t.cells[0].is_header = false;
    t.page_break = TablePageBreak::RowBreak;
    for edge in [
        BorderLine {
            line_type: BorderLineType::Dash,
            width: 7,
            color: 0,
        },
        BorderLine {
            line_type: BorderLineType::Solid,
            width: 16,
            color: 0,
        },
        BorderLine {
            line_type: BorderLineType::Solid,
            width: 7,
            color: 0xFF332211,
        },
    ] {
        let mut d = doc(t.clone());
        d.doc_info.border_fills[0].borders[0] = edge;
        assert!(direct(&d, 96.)
            .err()
            .unwrap()
            .to_string()
            .contains("Unsupported"));
        let Control::Table(t) = &mut d.sections[0].paragraphs[0].controls[0] else {
            unreachable!()
        };
        t.border_fill_id = 1;
        t.cells[0].border_fill_id = 0;
        assert!(direct(&d, 96.)
            .err()
            .unwrap()
            .to_string()
            .contains("Unsupported"));
    }
}

#[test]
fn coincident_boundaries_of_a_zero_height_unbordered_row_are_not_double_painted() {
    let mut t = grid(3);
    t.repeat_header = false;
    t.page_break = TablePageBreak::None;
    for c in &mut t.cells {
        c.is_header = false;
        if c.row == 1 {
            c.border_fill_id = 0;
            c.paragraphs.clear();
        }
    }
    let mut s = direct(&doc(t.clone()), 96.).unwrap();
    for _ in 0..2 {
        assert!(s
            .next_page()
            .err()
            .expect("degenerate bordered topology must be rejected")
            .to_string()
            .contains("zero-height row in bordered table"));
        assert_eq!(s.emitted_pages(), 0);
    }
    // Zero physical height itself is valid for a borderless table: do not change
    // its pagination or manufacture a line-height merely to draw the edges.
    for c in &mut t.cells {
        c.border_fill_id = 0;
    }
    let mut s = direct(&doc(t), 96.).unwrap();
    let page = s.next_page().unwrap().unwrap();
    let tree = serde_json::to_value(&page.tree).unwrap();
    assert!(collect(&tree["root"], "Line").is_empty());
    assert_eq!(collect(&tree["root"], "Table")[0]["bbox"]["height"], 36.0);
    assert!(s.next_page().unwrap().is_none());
}
