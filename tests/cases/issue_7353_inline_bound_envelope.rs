//! Binding/fit contract. Decimal row sums and source-scaled slots describe the
//! same geometry; reservation must use the measured child without hiding debt.
use rhwp::{
    model::{
        control::Control,
        paragraph::Paragraph,
        table::{Cell, Table, TablePageBreak},
    },
    renderer::table_v2::*,
};

struct Composed {
    slot: f64,
    offset: f64,
}
impl CellParagraphComposer for Composed {
    fn compose(&self, p: &Paragraph, width: f64) -> Result<Vec<ParagraphItem>, GeometryError> {
        if !p.controls.is_empty() {
            Ok(vec![ParagraphItem::InlineTables {
                height: self.offset + self.slot,
                advance: self.offset + self.slot,
                tables: vec![(
                    0,
                    Rect {
                        x: 0.,
                        y: self.offset,
                        width,
                        height: self.slot,
                    },
                )],
                lines: vec![],
            }])
        } else {
            let height = if p.text == "A" { 0.1 } else { 0.2 };
            Ok(vec![ParagraphItem::Lines {
                height,
                advance: height,
                lines: vec![(
                    0,
                    Rect {
                        x: 0.,
                        y: 0.,
                        width,
                        height,
                    },
                )],
            }])
        }
    }
}
fn t(paragraphs: Vec<Paragraph>) -> Table {
    Table {
        row_count: 1,
        col_count: 1,
        page_break: TablePageBreak::RowBreak,
        cells: vec![Cell {
            width: 100,
            row_span: 1,
            col_span: 1,
            paragraphs,
            ..Default::default()
        }],
        ..Default::default()
    }
}
fn input() -> Table {
    let mut child = t(["A", "B"]
        .map(|s| Paragraph {
            text: s.into(),
            ..Default::default()
        })
        .to_vec());
    child.common.treat_as_char = true;
    t(vec![
        Paragraph {
            controls: vec![Control::Table(Box::new(child))],
            ..Default::default()
        },
        Paragraph {
            text: "AFTER".into(),
            ..Default::default()
        },
    ])
}
fn area(height: f64) -> PageArea {
    PageArea {
        bounds: Rect {
            x: 0.,
            y: 0.,
            width: 100.,
            height,
        },
    }
}

#[test]
fn bound_child_reserves_its_exact_bottom_and_preserves_next_fragment() {
    for offset in [0., 0.1] {
        let plan =
            TableContentPlan::from_ir_contents(&input(), 1., &Composed { slot: 0.3, offset })
                .unwrap();
        let cursor = plan.start();
        let actual = offset + (0.1_f64 + 0.2);
        // Independent arithmetic expectation, not a measured-plan getter.
        let short = f64::from_bits(actual.to_bits() - 1);
        assert!(matches!(
            cursor.fit(area(short)).unwrap(),
            FragmentFit::DoesNotFit { .. }
        ));
        let FragmentFit::Placed(first) = cursor.fit(area(actual)).unwrap() else {
            panic!()
        };
        let cell = &first.placement().cells[0];
        assert_eq!(cell.tables.len(), 1);
        assert_eq!(first.placement().bounds.height, actual);
        let child = &cell.tables[0].placement;
        assert_eq!(child.bounds.y, offset);
        assert_eq!(child.bounds.height, 0.1_f64 + 0.2);
        let lines = &child.cells[0].lines;
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].bounds.y, offset);
        assert_eq!(lines[1].bounds.y, offset + 0.1);
        let FragmentFit::Placed(last) = first.continuation().fit(area(1.)).unwrap() else {
            panic!()
        };
        assert_eq!(last.placement().cells[0].lines.len(), 1);
        assert_eq!(last.placement().cells[0].lines[0].owner.paragraph, 1);
        assert!(matches!(
            last.continuation().fit(area(1.)).unwrap(),
            FragmentFit::Complete
        ));
    }
}

#[test]
fn real_slot_mismatch_is_not_promoted_to_a_larger_envelope() {
    for slot in [0.29, 0.31] {
        assert!(matches!(
            TableContentPlan::from_ir_contents(&input(), 1., &Composed { slot, offset: 0. }),
            Err(GeometryError::Unsupported(
                "TAC content changed stored occupied box"
            ))
        ));
    }
}
