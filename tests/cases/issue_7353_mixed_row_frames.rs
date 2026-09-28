//! Independent row/line occupancy contracts plus a normally saved Hancom input.
//! Synthetic cuts below are explicit composer output, not fabricated LineSegs.
use rhwp::{
    model::{
        paragraph::Paragraph,
        table::{Cell, Table, TablePageBreak, VerticalAlign},
    },
    renderer::table_v2::*,
};
use serde_json::Value;

struct Composer;
impl CellParagraphComposer for Composer {
    fn stored_frame_starts(&self, p: &[Paragraph]) -> Result<Vec<(usize, usize)>, GeometryError> {
        Ok(if p[0].text == "cut" {
            vec![(0, 1)]
        } else {
            vec![]
        })
    }
    fn compose(&self, p: &Paragraph, width: f64) -> Result<Vec<ParagraphItem>, GeometryError> {
        let (count, height) = match p.text.as_str() {
            "cut" => (2, 20.),
            "row" => (1, 20.),
            _ => (1, 10.),
        };
        Ok((0..count)
            .map(|line| ParagraphItem::Lines {
                height,
                advance: height,
                lines: vec![(
                    line,
                    Rect {
                        x: 0.,
                        y: 0.,
                        width,
                        height,
                    },
                )],
            })
            .collect())
    }
}
fn cell(row: u16, col: u16, span: u16, text: &str) -> Cell {
    Cell {
        row,
        col,
        row_span: span,
        col_span: 1,
        width: 100,
        paragraphs: vec![Paragraph {
            text: text.into(),
            ..Default::default()
        }],
        ..Default::default()
    }
}
fn source() -> Table {
    let mut label = cell(0, 0, 2, "label");
    label.vertical_align = VerticalAlign::Center;
    Table {
        row_count: 3,
        col_count: 2,
        page_break: TablePageBreak::RowBreak,
        cells: vec![
            label,
            cell(0, 1, 1, "row"),
            cell(1, 1, 1, "cut"),
            cell(2, 0, 1, "after"),
            cell(2, 1, 1, "after"),
        ],
        ..Default::default()
    }
}
fn area(height: f64) -> PageArea {
    PageArea {
        bounds: Rect {
            x: 7.,
            y: 11.,
            width: 200.,
            height,
        },
    }
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
fn placed(f: FragmentFit) -> TableFragmentPlan {
    let FragmentFit::Placed(p) = f else {
        panic!("expected fragment: {f:?}")
    };
    p
}

#[test]
fn incoming_span_and_nonspanning_saved_cut_share_the_accepted_fragment() {
    let start = TableContentPlan::from_ir_contents(&source(), 1., &Composer)
        .unwrap()
        .start();
    let first = placed(start.fit_in_page(area(45.), 100.).unwrap());
    near(first.reserved_height(), 40.);
    let cells = &first.placement().cells;
    let label = cells.iter().find(|c| c.row == 0 && c.column == 0).unwrap();
    near(label.bounds.height, 40.);
    near(label.lines[0].bounds.y, 11. + (40. - 10.) / 2.);
    let cut = cells.iter().find(|c| c.row == 1 && c.column == 1).unwrap();
    assert_eq!(cut.lines.len(), 1);
    assert_eq!(cut.lines[0].owner.line, 0);
    near(cut.bounds.y, 31.);
    near(cut.bounds.height, 20.);
    let last = placed(first.continuation().fit_in_page(area(100.), 100.).unwrap());
    near(last.reserved_height(), 30.);
    let continued = last.placement().cells.iter().find(|c| c.row == 0).unwrap();
    assert!(continued.lines.is_empty());
    near(continued.bounds.height, 20.);
    let tail = last.placement().cells.iter().find(|c| c.row == 1).unwrap();
    assert_eq!(tail.lines.len(), 1);
    assert_eq!(tail.lines[0].owner.line, 1);
    let after = last.placement().cells.iter().find(|c| c.row == 2).unwrap();
    near(after.bounds.y, 31.);
    assert!(last.continuation().is_complete());
    // Pure query: asking again must not have consumed the first cut or title.
    let retry = placed(start.fit_in_page(area(45.), 100.).unwrap());
    assert_eq!(
        format!("{:?}", first.placement()),
        format!("{:?}", retry.placement())
    );
}

#[test]
fn insufficient_line_budget_rolls_back_and_never_split_remains_atomic() {
    let mut t = source();
    let start = TableContentPlan::from_ir_contents(&t, 1., &Composer)
        .unwrap()
        .start();
    let first = placed(start.fit_in_page(area(39.), 100.).unwrap());
    assert!(first.placement().cells.iter().all(|c| c.row == 0));
    near(first.reserved_height(), 39.);
    let retry = placed(first.continuation().fit_in_page(area(100.), 100.).unwrap());
    let cut = retry.placement().cells.iter().find(|c| c.row == 1).unwrap();
    assert_eq!(
        cut.lines.iter().map(|l| l.owner.line).collect::<Vec<_>>(),
        vec![0]
    );
    // The composer's explicit cut still owns the second line even when the
    // retried page has spare room. No line was consumed by the failed fit.
    assert!(!retry.continuation().is_complete());
    let last = placed(retry.continuation().fit_in_page(area(100.), 100.).unwrap());
    let tail = last.placement().cells.iter().find(|c| c.row == 1).unwrap();
    assert_eq!(
        tail.lines.iter().map(|l| l.owner.line).collect::<Vec<_>>(),
        vec![1]
    );
    assert!(last.continuation().is_complete());
    t.page_break = TablePageBreak::None;
    let start = TableContentPlan::from_ir_contents(&t, 1., &Composer)
        .unwrap()
        .start();
    assert!(matches!(
        start.fit_in_page(area(45.), 100.).unwrap(),
        FragmentFit::DoesNotFit { .. }
    ));
    let all = placed(start.fit_in_page(area(100.), 100.).unwrap());
    near(all.reserved_height(), 70.);
    assert!(all.continuation().is_complete());
}

#[test]
fn multiple_incoming_spans_end_at_their_own_rows_not_each_others_envelope() {
    for span in [2, 3] {
        let mut t = source();
        t.col_count = 3;
        t.cells[0].row_span = span;
        if span == 3 {
            t.cells.remove(3);
        }
        t.cells.push(cell(0, 2, 2, "other label"));
        t.cells.push(cell(2, 2, 1, "after"));
        let a = |height| PageArea {
            bounds: Rect {
                width: 300.,
                ..area(height).bounds
            },
        };
        let start = TableContentPlan::from_ir_contents(&t, 1., &Composer)
            .unwrap()
            .start();
        let first = placed(start.fit_in_page(a(45.), 100.).unwrap());
        near(first.reserved_height(), 40.);
        for col in [0, 2] {
            let c = first
                .placement()
                .cells
                .iter()
                .find(|c| c.row == 0 && c.column == col)
                .unwrap();
            near(c.bounds.height, 40.);
            assert_eq!(c.lines.len(), 1);
        }
        let last = placed(first.continuation().fit_in_page(a(100.), 100.).unwrap());
        for (col, height) in [(0, if span == 3 { 30. } else { 20. }), (2, 20.)] {
            let c = last
                .placement()
                .cells
                .iter()
                .find(|c| c.row == 0 && c.column == col)
                .unwrap();
            near(c.bounds.height, height);
            assert!(c.lines.is_empty());
        }
        assert!(last.continuation().is_complete());
    }
}

#[test]
fn following_new_span_uses_the_remaining_page_without_losing_its_content() {
    let mut t = source();
    t.row_count = 5;
    t.cells.extend([
        cell(3, 0, 2, "new label"),
        cell(3, 1, 1, "after"),
        cell(4, 1, 1, "after"),
    ]);
    let start = TableContentPlan::from_ir_contents(&t, 1., &Composer)
        .unwrap()
        .start();
    let first = placed(start.fit_in_page(area(45.), 100.).unwrap());
    let last = placed(first.continuation().fit_in_page(area(100.), 100.).unwrap());
    near(last.reserved_height(), 50.);
    let label = last
        .placement()
        .cells
        .iter()
        .find(|c| c.row == 3 && c.column == 0)
        .unwrap();
    near(label.bounds.y, 41.);
    near(label.bounds.height, 20.);
    assert_eq!(label.lines.len(), 1);
    assert!(last.continuation().is_complete());
}

#[test]
fn a_live_span_crossing_a_later_span_start_has_one_physical_cell_per_page() {
    let mut t = source();
    t.row_count = 5;
    t.col_count = 3;
    t.cells[0].row_span = 5;
    t.cells.remove(3);
    t.cells.extend([
        cell(0, 2, 1, "row"),
        cell(1, 2, 1, "other"),
        cell(2, 2, 1, "after"),
        cell(3, 1, 2, "new label"),
        cell(3, 2, 1, "after"),
        cell(4, 2, 1, "after"),
    ]);
    let a = |height| PageArea {
        bounds: Rect {
            width: 300.,
            ..area(height).bounds
        },
    };
    let start = TableContentPlan::from_ir_contents(&t, 1., &Composer)
        .unwrap()
        .start();
    let first = placed(start.fit_in_page(a(45.), 100.).unwrap());
    let last = placed(first.continuation().fit_in_page(a(100.), 100.).unwrap());
    near(last.reserved_height(), 50.);
    let live: Vec<_> = last
        .placement()
        .cells
        .iter()
        .filter(|c| c.row == 0 && c.column == 0)
        .collect();
    assert_eq!(
        live.len(),
        1,
        "one spanning owner, not duplicate borders inside one page"
    );
    near(live[0].bounds.height, 50.);
    assert!(live[0].lines.is_empty());
    assert!(last.continuation().is_complete());
}

fn nodes<'a>(n: &'a Value, kind: &str) -> Vec<&'a Value> {
    let mut v = vec![];
    if n["node_type"].get(kind).is_some() {
        v.push(n);
    }
    for child in n["children"].as_array().unwrap() {
        v.extend(nodes(child, kind));
    }
    v
}
fn b(n: &Value, k: &str) -> f64 {
    n["bbox"][k].as_f64().unwrap()
}
fn text(n: &Value) -> String {
    nodes(n, "TextRun")
        .iter()
        .map(|n| n["node_type"]["TextRun"]["text"].as_str().unwrap())
        .collect()
}
fn source_text(t: &Table) -> String {
    let mut result = String::new();
    for c in &t.cells {
        for p in &c.paragraphs {
            result.push_str(&p.text);
            for control in &p.controls {
                if let rhwp::model::control::Control::Table(t) = control {
                    result.push_str(&source_text(t));
                }
            }
        }
    }
    result
}
#[test]
fn normal_saved_parent_with_spans_preserves_nested_frame_cuts_and_terminates() {
    let bytes = include_bytes!("../fixtures/issue7353/mixed-row-frames/prefix174-saved.hwp");
    let mut s = DocumentV2Session::from_bytes(
        bytes,
        r#"{"dpi":96,"max_pages":40,"cell_end_policy":"omit_final_paragraph_gap"}"#,
    )
    .unwrap();
    let mut pages = vec![];
    while let Some(raw) = s.next_page_json().unwrap() {
        pages.push(serde_json::from_str::<Value>(&raw).unwrap());
    }
    assert_eq!(pages.len(), 28, "independent same-input Hancom PDF");
    assert!(s.next_page_json().unwrap().is_none());
    let parent = |page: usize| {
        nodes(&pages[page]["render_tree"]["root"], "Table")
            .into_iter()
            .find(|t| t["node_type"]["Table"]["para_index"] == 172)
            .unwrap()
    };
    let at = |page, row, col| {
        parent(page)["children"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| {
                c["node_type"]["TableCell"]["row"] == row
                    && c["node_type"]["TableCell"]["col"] == col
            })
            .unwrap()
    };
    let doc = rhwp::parse_document(bytes).unwrap();
    let rhwp::model::control::Control::Table(source) = &doc.sections[0].paragraphs[172].controls[0]
    else {
        panic!()
    };
    let formula = &source.cells[77].paragraphs[0];
    assert_eq!(nodes(at(25, 26, 2), "TextLine").len(), 3);
    assert_eq!(nodes(at(26, 26, 2), "TextLine").len(), 1);
    assert_eq!(text(at(26, 26, 2)).trim(), "이자율");
    assert_eq!(text(at(25, 26, 2)) + &text(at(26, 26, 2)), formula.text);
    // Independent PDF p26 lower horizontal stroke: y72.281pt, paper841pt.
    // The printer's vertical scale .119869 vs .12 explains at most this residual.
    let expected = (841. - 72.281) * 4. / 3.;
    assert!(
        (b(parent(25), "y") + b(parent(25), "height") - expected).abs()
            <= expected * (1. - 0.119869 / 0.12) + 0.3
    );
    let rhwp::model::control::Control::Table(inner) = &source.cells[80].paragraphs[0].controls[1]
    else {
        panic!()
    };
    let mut inner_text = String::new();
    for (page, child_rows, child_cols) in [(26, 3, 12), (27, 5, 4)] {
        let tables = nodes(&pages[page]["render_tree"]["root"], "Table");
        let parent = tables
            .iter()
            .find(|t| t["node_type"]["Table"]["para_index"] == 172)
            .unwrap();
        let child = tables
            .iter()
            .find(|t| {
                t["node_type"]["Table"]["row_count"] == child_rows
                    && t["node_type"]["Table"]["col_count"] == child_cols
            })
            .unwrap();
        let carrier = tables
            .iter()
            .find(|t| {
                t["node_type"]["Table"]["row_count"] == 1
                    && t["node_type"]["Table"]["col_count"] == 1
            })
            .unwrap();
        inner_text.push_str(&text(carrier));
        assert!(b(child, "y") >= b(parent, "y"));
        assert!(b(child, "y") + b(child, "height") <= b(parent, "y") + b(parent, "height") + 1e-7);
        let body = nodes(&pages[page]["render_tree"]["root"], "Body")[0];
        assert!(b(parent, "y") + b(parent, "height") <= b(body, "y") + b(body, "height") + 1e-7);
    }
    assert_eq!(
        inner_text,
        source_text(inner),
        "every nested source unit once in story order"
    );
    assert_eq!(text(at(26, 27, 0)), "근거설명");
    assert!(
        text(at(27, 27, 0)).is_empty(),
        "companion label must not repeat"
    );
    assert!(
        nodes(at(26, 16, 0), "TextLine").is_empty(),
        "incoming spanning label already consumed"
    );
    let after = nodes(&pages[27]["render_tree"]["root"], "TextLine")
        .into_iter()
        .find(|n| n["node_type"]["TextLine"]["para_index"] == 173)
        .unwrap();
    near(
        b(after, "y"),
        b(parent(27), "y") + b(parent(27), "height") + f64::from(source.common.margin.bottom) / 75.,
    );
}
