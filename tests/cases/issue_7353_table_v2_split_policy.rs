//! Synthetic policy/placement contracts; normal Hancom witness pairs are in
//! tests/fixtures/issue7353/child-frame-tail. These do not claim full-document fidelity.
//! Independent semantics: Hancom help table(table).htm and PageBreak API 2/1/0.
use rhwp::model::{
    paragraph::Paragraph,
    table::{Cell, Table, TablePageBreak},
};
use rhwp::renderer::table_v2::*;

struct TwoLines;
impl CellParagraphComposer for TwoLines {
    fn compose(&self, _: &Paragraph, width: f64) -> Result<Vec<ParagraphItem>, GeometryError> {
        Ok((0..2)
            .map(|line| ParagraphItem::Lines {
                height: 10.,
                advance: 10.,
                lines: vec![(
                    line,
                    Rect {
                        x: 0.,
                        y: 0.,
                        width,
                        height: 10.,
                    },
                )],
            })
            .collect())
    }
}
fn area(height: f64) -> PageArea {
    PageArea {
        bounds: Rect {
            x: 3.,
            y: 7.,
            width: 100.,
            height,
        },
    }
}
fn cursor(policy: TablePageBreak) -> TableCursor {
    let table = Table {
        row_count: 1,
        col_count: 1,
        page_break: policy,
        cells: vec![Cell {
            row_span: 1,
            col_span: 1,
            width: 100,
            paragraphs: vec![Paragraph::default()],
            ..Default::default()
        }],
        ..Default::default()
    };
    TableContentPlan::from_ir_contents(&table, 1., &TwoLines)
        .unwrap()
        .start()
}
#[test]
fn bit2_line_split_preserves_line_owners_and_final_coordinates() {
    let c = cursor(TablePageBreak::RowBreak); // parser maps raw2 to this name
    let FragmentFit::Placed(first) = c.fit(area(10.)).unwrap() else {
        panic!("Hancom 나눔 must accept the first complete line");
    };
    assert_eq!(first.placement().cells[0].lines.len(), 1);
    assert_eq!(first.placement().cells[0].lines[0].owner.line, 0);
    assert_eq!(first.placement().cells[0].lines[0].bounds.y, 7.);
    let FragmentFit::Placed(last) = first.continuation().fit(area(10.)).unwrap() else {
        panic!()
    };
    assert_eq!(last.placement().cells[0].lines.len(), 1);
    assert_eq!(last.placement().cells[0].lines[0].owner.line, 1);
    assert_eq!(last.placement().cells[0].lines[0].bounds.y, 7.);
    assert!(last.continuation().is_complete());
}
#[test]
fn bit1_whole_cell_defers_without_consuming_a_line() {
    let c = cursor(TablePageBreak::CellBreak); // parser maps raw1 to this name
    assert!(
        matches!(c.fit(area(10.)).unwrap(), FragmentFit::DoesNotFit { .. }),
        "Hancom 셀 단위로 나눔 must defer the whole cell"
    );
    let FragmentFit::Placed(all) = c.fit(area(20.)).unwrap() else {
        panic!()
    };
    assert_eq!(all.placement().cells[0].lines.len(), 2);
    assert_eq!(all.placement().cells[0].lines[1].bounds.y, 17.);
    assert!(all.continuation().is_complete());
}
#[test]
fn bit0_unsplit_control_still_defers() {
    assert!(matches!(
        cursor(TablePageBreak::None).fit(area(10.)).unwrap(),
        FragmentFit::DoesNotFit { .. }
    ));
}
