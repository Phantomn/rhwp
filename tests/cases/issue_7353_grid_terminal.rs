//! The declared table edge, corroborated by a complete row, closes short rows.
//! Cell measurement and final placement consume the same resolved terminal cell.
use rhwp::{
    model::{
        control::Control,
        document::Document,
        paragraph::Paragraph,
        table::{Cell, Table},
    },
    renderer::table_v2::*,
};

struct WidthLine;
impl CellParagraphComposer for WidthLine {
    fn compose(&self, _: &Paragraph, width: f64) -> Result<Vec<ParagraphItem>, GeometryError> {
        Ok(vec![ParagraphItem::Lines {
            height: 10.0,
            advance: 10.0,
            lines: vec![(
                0,
                Rect {
                    x: 0.0,
                    y: 0.0,
                    width,
                    height: 10.0,
                },
            )],
        }])
    }
}
fn cell(row: u16, col: u16, width: u32) -> Cell {
    Cell {
        row,
        col,
        row_span: 1,
        col_span: 1,
        width,
        paragraphs: vec![Paragraph::default()],
        ..Default::default()
    }
}
fn sample() -> Table {
    let mut t = Table {
        row_count: 2,
        col_count: 2,
        cells: vec![
            cell(0, 0, 6000),
            cell(0, 1, 3995),
            cell(1, 0, 6000),
            cell(1, 1, 4000),
        ],
        ..Default::default()
    };
    t.common.width = 10000;
    t
}
#[test]
fn corroborated_terminal_edge_is_shared_by_content_and_placement() {
    for reverse in [false, true] {
        let mut t = sample();
        if reverse {
            t.cells.reverse();
        }
        let original = t.clone();
        for scale in [1.0 / 100.0, 1.0 / 75.0, 1.0 / 50.0] {
            let plan = TableContentPlan::from_ir_contents(&t, scale, &WidthLine).unwrap();
            let FragmentFit::Placed(f) = plan
                .start()
                .fit(PageArea {
                    bounds: Rect {
                        x: 17.0,
                        y: 23.0,
                        width: 300.0,
                        height: 100.0,
                    },
                })
                .unwrap()
            else {
                panic!()
            };
            for c in &f.placement().cells {
                let expected = if c.column == 0 { 6000.0 } else { 4000.0 } * scale;
                assert!((c.bounds.width - expected).abs() < 1e-9);
                assert!((c.lines[0].bounds.width - expected).abs() < 1e-9);
                if c.column == 1 {
                    assert!((c.bounds.x + c.bounds.width - (17.0 + 10000.0 * scale)).abs() < 1e-9);
                }
            }
        }
        assert_eq!(
            t.cells.iter().map(|c| c.width).collect::<Vec<_>>(),
            original.cells.iter().map(|c| c.width).collect::<Vec<_>>()
        );
    }
}
#[test]
fn uncorroborated_overwide_or_disputed_internal_edges_remain_errors() {
    for mode in 0..4 {
        let mut t = sample();
        match mode {
            0 => t.common.width = 0,
            1 => t.common.width = 10001,
            2 => t.common.width = 9995,
            _ => {
                t.cells[0].width += 1;
                t.cells[1].width -= 1;
            }
        }
        assert!(
            TableContentPlan::from_ir_contents(&t, 1.0 / 75.0, &WidthLine).is_err(),
            "mode {mode}"
        );
    }
}
fn original_table(d: &Document) -> &Table {
    d.sections[0].paragraphs[10]
        .controls
        .iter()
        .find_map(|c| {
            if let Control::Table(t) = c {
                Some(t.as_ref())
            } else {
                None
            }
        })
        .unwrap()
}

#[test]
fn actual_saved_document_reaches_paint_without_rounding_away_the_terminal_space() {
    let input = include_bytes!("../fixtures/issue7353/grid-terminal/grid-saved.hwp");
    let mut session = DocumentV2Session::from_bytes(
        input,
        r#"{"dpi":96,"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}"#,
    )
    .unwrap();
    let page: serde_json::Value =
        serde_json::from_str(&session.next_page_json().unwrap().unwrap()).unwrap();
    fn collect<'a>(n: &'a serde_json::Value, out: &mut Vec<&'a serde_json::Value>) {
        if n["node_type"].get("TableCell").is_some() && n["bbox"]["y"].as_f64().unwrap() > 650.0 {
            out.push(n);
        }
        if let Some(children) = n["children"].as_array() {
            for c in children {
                collect(c, out);
            }
        }
    }
    let mut cells = vec![];
    collect(&page["render_tree"]["root"], &mut cells);
    assert_eq!(cells.len(), 25);
    // Independent Hancom PDF clip edges in points. Source row0 columns
    // 0/1/2/5/6/8 and common right: no rescaling of internal grid boundaries.
    for (column, x) in [
        (0, 58.049),
        (1, 164.911),
        (2, 226.918),
        (5, 337.618),
        (6, 377.797),
        (8, 429.489),
    ] {
        let c = cells
            .iter()
            .find(|c| {
                c["node_type"]["TableCell"]["row"] == 0
                    && c["node_type"]["TableCell"]["col"] == column
            })
            .unwrap();
        assert!((c["bbox"]["x"].as_f64().unwrap() - x * 4.0 / 3.0).abs() < 1.0);
    }
    for c in &cells {
        let cell = &c["node_type"]["TableCell"];
        if cell["col"].as_u64().unwrap() + cell["col_span"].as_u64().unwrap() == 11 {
            let right = c["bbox"]["x"].as_f64().unwrap() + c["bbox"]["width"].as_f64().unwrap();
            assert!(
                (right - 537.311 * 4.0 / 3.0).abs() < 1.0,
                "PDF right {right}"
            );
        }
    }
    let last = cells.last().unwrap();
    let bottom = last["bbox"]["y"].as_f64().unwrap() + last["bbox"]["height"].as_f64().unwrap();
    assert!(
        (bottom - (841.0 - 117.232) * 4.0 / 3.0).abs() < 1.0,
        "PDF bottom {bottom}"
    );
    assert!(session.next_page_json().unwrap().is_none());
}
#[test]
fn actual_hwp_hwpx_and_hancom_resave_accept_the_same_terminal_geometry() {
    for bytes in [
        include_bytes!("../../samples/86712_regulatory_analysis.hwp").as_slice(),
        include_bytes!("../../samples/issue1891/86712_regulatory_analysis.hwpx").as_slice(),
        include_bytes!("../fixtures/issue7353/grid-terminal/grid-saved.hwp").as_slice(),
    ] {
        let d = rhwp::parse_document(bytes).unwrap();
        let t = original_table(&d);
        let plan = TableContentPlan::from_ir_contents(t, 1.0 / 75.0, &WidthLine).unwrap();
        let FragmentFit::Placed(f) = plan
            .start()
            .fit(PageArea {
                bounds: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 700.0,
                    height: 1000.0,
                },
            })
            .unwrap()
        else {
            panic!()
        };
        for c in &f.placement().cells {
            let source = t
                .cells
                .iter()
                .find(|s| s.row as usize == c.row && s.col as usize == c.column)
                .unwrap();
            if source.col + source.col_span == t.col_count {
                assert!((c.bounds.x + c.bounds.width - 47958.0 / 75.0).abs() < 1e-9);
            }
        }
    }
}
