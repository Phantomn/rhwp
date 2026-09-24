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
fn unsupported_table_and_complex_edges_are_explicit_even_when_geometry_fits() {
    let mut t = grid(1);
    t.repeat_header = false;
    t.cells[0].is_header = false;
    t.page_break = TablePageBreak::RowBreak;
    let mut table_border = t.clone();
    table_border.border_fill_id = 1;
    assert!(direct(&doc(table_border), 96.)
        .err()
        .unwrap()
        .to_string()
        .contains("Unsupported"));
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
