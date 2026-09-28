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
fn isolated_thin_thick_edges_use_table_pens_and_keep_cell_geometry() {
    // Normal Hancom borders-2020.pdf: 0.7mm pens 4/9 at 600dpi,
    // centres -6/+4 from the cell edge. Thin is above/left on BOTH sides.
    for dpi in [96., 192.] {
        for side in 0..4 {
            let mut d = doc(grid(1));
            for edge in &mut d.doc_info.border_fills[0].borders {
                edge.line_type = BorderLineType::None;
            }
            let plain = direct(&d, dpi).unwrap().next_page().unwrap().unwrap();
            let plain = serde_json::to_value(&plain.tree).unwrap();
            d.doc_info.border_fills[0].borders[side] = BorderLine {
                line_type: BorderLineType::ThinThickDouble,
                width: 9,
                color: 0x332211,
            };
            let page = direct(&d, dpi).unwrap().next_page().unwrap().unwrap();
            let tree = serde_json::to_value(&page.tree).unwrap();
            let cells = collect(&tree["root"], "TableCell");
            assert_eq!(
                cells[0]["bbox"],
                collect(&plain["root"], "TableCell")[0]["bbox"]
            );
            assert_eq!(
                collect(&tree["root"], "TextLine"),
                collect(&plain["root"], "TextLine")
            );
            let b = &cells[0]["bbox"];
            let x = b["x"].as_f64().unwrap();
            let y = b["y"].as_f64().unwrap();
            let right = x + b["width"].as_f64().unwrap();
            let bottom = y + b["height"].as_f64().unwrap();
            let lines = collect(&tree["root"], "Line");
            assert_eq!(lines.len(), 2);
            for (line, (pen, offset)) in lines.iter().zip([(4., -6.), (9., 4.)]) {
                near(
                    line["node_type"]["Line"]["style"]["width"]
                        .as_f64()
                        .unwrap(),
                    pen * dpi / 600.,
                );
                let at = [x, right, y, bottom][side] + offset * dpi / 600.;
                let expected = if side < 2 {
                    [at, y, at, bottom]
                } else {
                    [x, at, right, at]
                };
                for (actual, expected) in coords(line).into_iter().zip(expected) {
                    near(actual, expected);
                }
            }
        }
    }
}

#[test]
fn thin_thick_catalog_matches_independent_pdf_strokes() {
    // catalog-2020.pdf trace, widths/offsets in 600dpi units. The input was
    // normally saved by Hancom; these are not values queried from our helper.
    let catalog = [
        (0., 2., 0.),
        (1., 3., -2.),
        (1., 3., -2.),
        (1., 3., -2.),
        (1., 4., -3.),
        (1., 5., -3.),
        (2., 5., -3.),
        (3., 6., -5.),
        (3., 8., -6.),
        (4., 9., -6.),
        (6., 12., -9.),
        (8., 19., -13.),
        (11., 25., -18.),
        (17., 37., -27.),
        (23., 49., -36.),
        (29., 60., -45.),
    ];
    for dpi in [96., 192.] {
        for (width, (thin, thick, offset)) in catalog.into_iter().enumerate() {
            let mut d = doc(grid(1));
            for e in &mut d.doc_info.border_fills[0].borders {
                e.line_type = BorderLineType::None;
            }
            d.doc_info.border_fills[0].borders[3] = BorderLine {
                line_type: BorderLineType::ThinThickDouble,
                width: width as u8,
                color: 0,
            };
            let p = direct(&d, dpi).unwrap().next_page().unwrap().unwrap();
            let tree = serde_json::to_value(&p.tree).unwrap();
            let b = &collect(&tree["root"], "TableCell")[0]["bbox"];
            let bottom = b["y"].as_f64().unwrap() + b["height"].as_f64().unwrap();
            let lines = collect(&tree["root"], "Line");
            assert_eq!(lines.len(), if width == 0 { 1 } else { 2 });
            let expected = [(thin, offset), (thick, thin)]
                .into_iter()
                .filter(|(w, _)| *w > 0.);
            for (line, (pen, offset)) in lines.iter().zip(expected) {
                near(
                    line["node_type"]["Line"]["style"]["width"]
                        .as_f64()
                        .unwrap(),
                    pen * dpi / 600.,
                );
                near(coords(line)[1], bottom + offset * dpi / 600.);
                near(
                    line["bbox"]["y"].as_f64().unwrap(),
                    bottom + (offset - pen / 2.) * dpi / 600.,
                );
                near(line["bbox"]["height"].as_f64().unwrap(), pen * dpi / 600.);
            }
        }
    }
}

#[test]
fn thin_thick_fragment_edges_preserve_headers_nested_content_and_following_text() {
    for nested in [false, true] {
        let t = if nested {
            let mut parent = grid(1);
            parent.repeat_header = false;
            parent.page_break = TablePageBreak::CellBreak;
            parent.cells[0].is_header = false;
            parent.cells[0].border_fill_id = 0;
            parent.cells[0].paragraphs = vec![host("host", grid(3)), para("after")];
            parent
        } else {
            grid(3)
        };
        let mut d = doc(t);
        for e in &mut d.doc_info.border_fills[0].borders {
            e.line_type = BorderLineType::None;
        }
        d.doc_info.border_fills[0].borders[3].line_type = BorderLineType::Solid;
        let baseline = pages("thin-thick-solid-control", &d);
        d.doc_info.border_fills[0].borders[3].line_type = BorderLineType::ThinThickDouble;
        let actual = pages("thin-thick-fragments", &d);
        assert!(actual.len() > 1, "exercise actual continuation");
        assert_eq!(actual.len(), baseline.len());
        for (a, b) in actual.iter().zip(&baseline) {
            for kind in ["Table", "TableCell", "TextLine", "TextRun"] {
                let boxes = |p: &Value| {
                    collect(root(p), kind)
                        .iter()
                        .map(|n| n["bbox"].clone())
                        .collect::<Vec<_>>()
                };
                assert_eq!(boxes(a), boxes(b), "{kind}");
            }
            let text = |p: &Value| {
                collect(root(p), "TextRun")
                    .iter()
                    .map(|n| n["node_type"]["TextRun"]["text"].clone())
                    .collect::<Vec<_>>()
            };
            assert_eq!(text(a), text(b));
            let solid = collect(root(b), "Line");
            let strokes = collect(root(a), "Line");
            assert_eq!(strokes.len(), solid.len() * 2);
            for (pair, s) in strokes.chunks_exact(2).zip(solid) {
                let c = coords(s);
                for (line, (width, offset)) in pair.iter().zip([(3., -5.), (6., 3.)]) {
                    let actual = coords(line);
                    near(actual[0], c[0]);
                    near(actual[2], c[2]);
                    near(actual[1], c[1] + offset * 96. / 600.);
                    near(actual[3], c[3] + offset * 96. / 600.);
                    near(
                        line["node_type"]["Line"]["style"]["width"]
                            .as_f64()
                            .unwrap(),
                        width * 96. / 600.,
                    );
                }
            }
        }
    }
}

#[test]
fn thin_thick_junctions_and_zones_remain_explicit_and_retryable() {
    for crossing in [false, true] {
        let mut d = doc(grid(if crossing { 2 } else { 1 }));
        let e = &mut d.doc_info.border_fills[0].borders;
        for line in &mut *e {
            line.line_type = BorderLineType::None;
        }
        e[3].line_type = BorderLineType::ThinThickDouble;
        e[0].line_type = BorderLineType::Solid;
        let mut s = direct(&d, 96.).unwrap();
        for _ in 0..2 {
            assert!(s
                .next_page()
                .err()
                .unwrap()
                .to_string()
                .contains("thin-thick border junction"));
            assert_eq!(s.emitted_pages(), 0);
        }
    }
    let mut t = grid(1);
    t.zones.push(rhwp::model::table::TableZone {
        start_row: 0,
        end_row: 0,
        start_col: 0,
        end_col: 0,
        border_fill_id: 1,
    });
    let mut d = doc(t);
    for e in &mut d.doc_info.border_fills[0].borders {
        e.line_type = BorderLineType::None;
    }
    d.doc_info.border_fills[0].borders[3].line_type = BorderLineType::ThinThickDouble;
    assert!(direct(&d, 96.)
        .err()
        .unwrap()
        .to_string()
        .contains("double zone perimeter"));
}

#[test]
fn normal_saved_thin_thick_borders_preserve_four_directions_and_following_content() {
    let data = include_bytes!("../fixtures/issue7353_thin_thick_review/borders-saved.hwp");
    let parsed = rhwp::parse_document(data).unwrap();
    let hwpx = rhwp::serializer::serialize_hwpx(&parsed).unwrap();
    for input in [data.as_slice(), hwpx.as_slice()] {
        let mut s = rhwp::renderer::table_v2::DocumentV2Session::from_bytes(
            input,
            r#"{"dpi":96,"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}"#,
        )
        .unwrap();
        let p: Value = serde_json::from_str(&s.next_page_json().unwrap().unwrap()).unwrap();
        let cells = collect(root(&p), "TableCell");
        assert_eq!(cells.len(), 9);
        // Normal PDF: parent + eight children, five bottom widths then
        // top/left/right width9. No reciprocal flip on bottom/right edges.
        let specs = [
            (3, vec![(2., 0.)]),
            (3, vec![(1., -2.), (3., 1.)]),
            (3, vec![(3., -5.), (6., 3.)]),
            (3, vec![(4., -6.), (9., 4.)]),
            (3, vec![(8., -13.), (19., 8.)]),
            (2, vec![(4., -6.), (9., 4.)]),
            (0, vec![(4., -6.), (9., 4.)]),
            (1, vec![(4., -6.), (9., 4.)]),
        ];
        let tables = collect(root(&p), "Table");
        for ((cell, table), (side, pens)) in cells[1..].iter().zip(&tables[1..]).zip(specs) {
            let b = &cell["bbox"];
            let x = b["x"].as_f64().unwrap();
            let y = b["y"].as_f64().unwrap();
            let w = b["width"].as_f64().unwrap();
            let h = b["height"].as_f64().unwrap();
            let strokes = collect(table, "Line");
            assert_eq!(strokes.len(), pens.len());
            for (line, (width, offset)) in strokes.iter().zip(pens) {
                let c = coords(line);
                near(
                    line["node_type"]["Line"]["style"]["width"]
                        .as_f64()
                        .unwrap(),
                    width * 96. / 600.,
                );
                let at = [x, x + w, y, y + h][side] + offset * 96. / 600.;
                near(if side < 2 { c[0] } else { c[1] }, at);
            }
        }
        let text = collect(root(&p), "TextRun");
        assert!(text
            .iter()
            .any(|n| n["node_type"]["TextRun"]["text"] == "CELL AFTER"));
        let after = text
            .iter()
            .find(|n| n["node_type"]["TextRun"]["text"] == "AFTER PARENT TABLE")
            .unwrap();
        let parent = &cells[0]["bbox"];
        assert!(
            after["bbox"]["y"].as_f64().unwrap()
                >= parent["y"].as_f64().unwrap() + parent["height"].as_f64().unwrap()
        );
        assert!(s.next_page_json().unwrap().is_none());
    }
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
fn normal_saved_mixed_inner_junctions_stop_at_solid_ink_and_preserve_content() {
    // Independently normal-saved Hancom references: 0.5mm double pens are
    // 0.36pt each with 1.08pt centre distance. Solid 0.12/0.5mm = 0.36/1.44pt.
    // Their T/cross junctions have uninterrupted solids and no exposed double
    // ink inside that solid band. See the fixture README for vector evidence.
    for (name, horizontal, solid_width) in [
        ("vertical-inner", false, 0.48),
        ("horizontal-inner", true, 1.92),
    ] {
        let data = std::fs::read(format!(
            "{}/tests/fixtures/issue7353/mixed-borders/{name}-saved.hwp",
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
        let b = &child["bbox"];
        near(b["width"].as_f64().unwrap(), 30000. / 75.);
        near(b["height"].as_f64().unwrap(), 8000. / 75.);
        let x = b["x"].as_f64().unwrap();
        let y = b["y"].as_f64().unwrap();
        // Physical frame from the independent PDF, not a pre-change V2 golden.
        assert!((x - 68.123 * 4. / 3.).abs() < 0.5);
        assert!((y - (841. - 765.962) * 4. / 3.).abs() < 0.5);
        let (along, across, step, length) = if horizontal {
            (x, y + 4000. / 75., 15000. / 75., 8000. / 75.)
        } else {
            (y, x + 15000. / 75., 4000. / 75., 30000. / 75.)
        };
        let lines = collect(child, "Line");
        assert_eq!(lines.len(), 9, "four pens and five distinct solid edges");
        let double: Vec<_> = lines
            .iter()
            .filter(|n| {
                let c = coords(n);
                if horizontal {
                    c[1] == c[3] && (c[1] - across).abs() < 1.
                } else {
                    c[0] == c[2] && (c[0] - across).abs() < 1.
                }
            })
            .collect();
        assert_eq!(double.len(), 4);
        for half in 0..2 {
            for offset in [-0.72, 0.72] {
                let expected = [
                    along + f64::from(half) * step + solid_width / 2.,
                    along + f64::from(half + 1) * step - solid_width / 2.,
                ];
                assert!(double.iter().any(|n| {
                    let c = coords(n);
                    let (a, b, at) = if horizontal {
                        (c[0], c[2], c[1])
                    } else {
                        (c[1], c[3], c[0])
                    };
                    (a - expected[0]).abs() < 1e-9
                        && (b - expected[1]).abs() < 1e-9
                        && (at - across - offset).abs() < 1e-9
                        && (n["node_type"]["Line"]["style"]["width"].as_f64().unwrap() - 0.48).abs()
                            < 1e-9
                }));
            }
        }
        // Each intersecting solid stays whole across the double's white gap.
        for i in 0..3 {
            assert!(lines.iter().any(|n| {
                let c = coords(n);
                let (a, b, at) = if horizontal {
                    (c[1], c[3], c[0])
                } else {
                    (c[0], c[2], c[1])
                };
                (at - along - f64::from(i) * step).abs() < 1e-9
                    && (a - (across - length / 2.)).abs() < 1e-9
                    && (b - (across + length / 2.)).abs() < 1e-9
            }));
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
        let last = collect(root(&p), "TextLine").pop().unwrap();
        assert!(
            last["bbox"]["y"].as_f64().unwrap()
                > tables[0]["bbox"]["y"].as_f64().unwrap()
                    + tables[0]["bbox"]["height"].as_f64().unwrap()
        );
    }
}

#[test]
fn mixed_inner_repeated_header_keeps_fragments_and_color_conflicts_reject() {
    let mut d = doc(grid(3));
    let control = pages("mixed-header-control", &d);
    for side in [1, 0] {
        let mut b = border();
        b.borders[side].line_type = BorderLineType::Double;
        d.doc_info.border_fills.push(b);
    }
    let Control::Table(t) = &mut d.sections[0].paragraphs[0].controls[0] else {
        panic!()
    };
    for cell in &mut t.cells {
        if cell.row > 0 {
            cell.border_fill_id = 2 + cell.col;
        }
    }
    let mixed = pages("mixed-header", &d);
    assert_eq!(mixed.len(), control.len());
    assert!(mixed.len() > 1);
    for (a, b) in control.iter().zip(&mixed) {
        for kind in ["Table", "TableCell", "TextLine", "TextRun"] {
            let geometry = |p: &Value| {
                collect(root(p), kind)
                    .into_iter()
                    .map(|n| {
                        let mut node = n["node_type"].clone();
                        if let Some(cell) = node.get_mut("TableCell") {
                            cell.as_object_mut().unwrap().remove("border_fill_id");
                        }
                        (n["bbox"].clone(), node)
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(geometry(a), geometry(b));
        }
    }
    // Both incident solids agree with each other but not with the double's
    // color: this must NOT enter the new same-color internal-junction rule.
    for b in &mut d.doc_info.border_fills {
        for e in &mut b.borders {
            if e.line_type == BorderLineType::Solid {
                e.color = 0x998877;
            }
        }
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
            line_type: BorderLineType::DashDot,
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
fn dash_fragment_edges_preserve_headers_nested_content_and_geometry() {
    patterned_fragment_edges_preserve_geometry(BorderLineType::Dash, "dash-fragments", 3.2);
}

#[test]
fn dot_fragment_edges_preserve_headers_nested_content_and_geometry() {
    // Width index1: four printer units at600dpi, independently observed in
    // issue7353_inline_bounds_review/dot-2020.pdf (0.64px at96dpi).
    patterned_fragment_edges_preserve_geometry(BorderLineType::Dot, "dot-fragments", 0.64);
}

fn patterned_fragment_edges_preserve_geometry(kind: BorderLineType, label: &str, max_len: f64) {
    for nested in [false, true] {
        let t = if nested {
            let mut parent = grid(1);
            parent.repeat_header = false;
            parent.page_break = TablePageBreak::CellBreak;
            parent.cells[0].is_header = false;
            parent.cells[0].border_fill_id = 0;
            parent.cells[0].paragraphs = vec![host("host", grid(3)), para("after")];
            parent
        } else {
            grid(3)
        };
        let mut d = doc(t);
        let baseline = pages(&format!("{label}-solid-control"), &d);
        for e in &mut d.doc_info.border_fills[0].borders {
            e.line_type = kind;
            e.width = 1;
        }
        let actual = pages(label, &d);
        assert_eq!(actual.len(), baseline.len());
        for (a, b) in actual.iter().zip(&baseline) {
            for kind in ["Table", "TableCell", "TextLine", "TextRun"] {
                let boxes = |p: &Value| {
                    collect(root(p), kind)
                        .iter()
                        .map(|n| n["bbox"].clone())
                        .collect::<Vec<_>>()
                };
                assert_eq!(boxes(a), boxes(b), "{kind}");
            }
            let text = |p: &Value| {
                collect(root(p), "TextRun")
                    .iter()
                    .map(|n| n["node_type"]["TextRun"]["text"].clone())
                    .collect::<Vec<_>>()
            };
            assert_eq!(text(a), text(b));
            let strokes = collect(root(a), "Line");
            let mut keys = std::collections::BTreeSet::new();
            for n in strokes {
                let c = coords(n);
                assert!(keys.insert(c.map(f64::to_bits)), "duplicate stroke");
                let len = (c[2] - c[0]).abs() + (c[3] - c[1]).abs();
                assert!(len > 0. && len <= max_len + 1e-9);
                near(
                    n["node_type"]["Line"]["style"]["width"].as_f64().unwrap(),
                    0.48,
                );
            }
        }
    }
}

#[test]
fn dash_shared_conflicts_and_zone_admission_remain_explicit() {
    for (other_type, other_width, other_color) in [
        (BorderLineType::Dash, 7, 0x332211),
        (BorderLineType::Solid, 1, 0x332211),
        (BorderLineType::Dash, 1, 0),
    ] {
        let mut d = doc(one_row());
        for e in &mut d.doc_info.border_fills[0].borders {
            e.line_type = BorderLineType::Dash;
            e.width = 1;
        }
        let mut other = d.doc_info.border_fills[0].clone();
        other.borders[0] = BorderLine {
            line_type: other_type,
            width: other_width,
            color: other_color,
        };
        d.doc_info.border_fills.push(other);
        let Control::Table(t) = &mut d.sections[0].paragraphs[0].controls[0] else {
            unreachable!()
        };
        t.cells[1].border_fill_id = 2;
        let mut s = direct(&d, 96.).unwrap();
        let before = s.emitted_pages();
        assert!(s.next_page().is_err());
        assert_eq!(s.emitted_pages(), before);
    }
    let mut d = doc(grid(2));
    for e in &mut d.doc_info.border_fills[0].borders {
        e.line_type = BorderLineType::Dash;
    }
    let Control::Table(t) = &mut d.sections[0].paragraphs[0].controls[0] else {
        unreachable!()
    };
    t.zones.push(rhwp::model::table::TableZone {
        start_row: 0,
        end_row: 1,
        start_col: 0,
        end_col: 1,
        border_fill_id: 1,
    });
    assert!(direct(&d, 96.)
        .err()
        .unwrap()
        .to_string()
        .contains("dash zone"));
}

#[test]
fn all_dash_pens_use_independent_catalog_lengths_at_two_dpis() {
    // Independent PDF pen-catalog-2020.pdf, horizontal+vertical trace, in
    // 600dpi pen units; these are not values read back from the implementation.
    let catalog = [
        (17., 9.),
        (20., 12.),
        (26., 14.),
        (34., 21.),
        (43., 27.),
        (52., 29.),
        (69., 42.),
        (86., 51.),
        (104., 62.),
        (121., 72.),
        (173., 105.),
        (259., 156.),
        (347., 206.),
        (520., 311.),
        (693., 417.),
        (866., 519.),
    ];
    for dpi in [96., 192.] {
        for (width, (on, off)) in catalog.into_iter().enumerate() {
            let mut t = grid(1);
            t.repeat_header = false;
            t.cells[0].is_header = false;
            let mut d = doc(t);
            for e in &mut d.doc_info.border_fills[0].borders {
                e.line_type = BorderLineType::Dash;
                e.width = width as u8;
            }
            let page = direct(&d, dpi).unwrap().next_page().unwrap().unwrap();
            let tree = serde_json::to_value(&page.tree).unwrap();
            let cell = collect(&tree["root"], "TableCell")[0];
            let x = cell["bbox"]["x"].as_f64().unwrap();
            let y = cell["bbox"]["y"].as_f64().unwrap();
            let w = cell["bbox"]["width"].as_f64().unwrap();
            let h = cell["bbox"]["height"].as_f64().unwrap();
            let lines = collect(&tree["root"], "Line");
            for horizontal in [true, false] {
                let segments: Vec<_> = lines
                    .iter()
                    .map(|n| coords(n))
                    .filter(|c| {
                        if horizontal {
                            c[1] == y && c[3] == y
                        } else {
                            c[0] == x && c[2] == x
                        }
                    })
                    .collect();
                let length = if horizontal { w } else { h };
                let start = if horizontal { x } else { y };
                let step = (on + off) * dpi / 600.;
                assert_eq!(segments.len(), (length / step).ceil() as usize);
                for (i, c) in segments.iter().enumerate() {
                    let a = if horizontal { c[0] } else { c[1] };
                    let b = if horizontal { c[2] } else { c[3] };
                    near(a, start + i as f64 * step);
                    near(b, (a + on * dpi / 600.).min(start + length));
                }
            }
        }
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
