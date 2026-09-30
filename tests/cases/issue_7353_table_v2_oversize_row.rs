//! Explicit fresh-page capacity, independent line-unit conservation contracts.
//! Synthetic inputs do not constitute Hancom visual evidence.
use rhwp::renderer::table_v2::*;

fn plan(policy: SplitPolicy, heights: &[f64], minimum: f64) -> TableContentPlan {
    TableContentPlan::from_flow_rows(
        vec![100.0],
        vec![FlowRowInput {
            cells: vec![FlowCellInput {
                width: 100.0,
                padding: Insets::default(),
                minimum_height: minimum,
                blocks: heights
                    .iter()
                    .enumerate()
                    .map(|(paragraph, &height)| FlowBlock::Lines {
                        height,
                        advance: height,
                        lines: vec![LineBox {
                            owner: LineOwner { paragraph, line: 0 },
                            bounds: Rect {
                                x: 0.0,
                                y: 0.0,
                                width: 80.0,
                                height,
                            },
                        }],
                    })
                    .collect(),
            }],
        }],
        0.0,
        policy,
    )
    .unwrap()
}

fn area(height: f64) -> PageArea {
    PageArea {
        bounds: Rect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height,
        },
    }
}

#[test]
fn normal_row_defers_instead_of_using_the_current_remainder_as_page_capacity() {
    let p = plan(SplitPolicy::BetweenRows, &[20.0, 20.0], 0.0).start();
    assert!(matches!(
        p.fit_in_page(area(25.0), 50.0).unwrap(),
        FragmentFit::DoesNotFit {
            required_height: 40.0,
            ..
        }
    ));
    let FragmentFit::Placed(f) = p.fit_in_page(area(50.0), 50.0).unwrap() else {
        panic!("fresh page fits row");
    };
    assert_eq!(f.reserved_height(), 40.0);
    assert!(f.continuation().is_complete());
}

#[test]
fn oversize_row_preserves_every_line_and_final_physical_band_once() {
    let p = plan(SplitPolicy::BetweenRows, &[20.0, 20.0, 20.0, 20.0], 110.0);
    let mut cursor = p.start();
    let mut owners = Vec::new();
    let mut reserved = Vec::new();
    for budget in [25.0, 50.0, 50.0] {
        let FragmentFit::Placed(f) = cursor.fit_in_page(area(budget), 50.0).unwrap() else {
            panic!("fragment fits");
        };
        for cell in &f.placement().cells {
            for line in &cell.lines {
                assert!(line.bounds.y >= cell.bounds.y);
                assert!(line.bounds.y + line.bounds.height <= cell.bounds.y + cell.bounds.height);
                owners.push(line.owner.paragraph);
            }
        }
        assert!(f.reserved_height() <= budget);
        reserved.push(f.reserved_height());
        cursor = f.continuation();
    }
    assert_eq!(owners, vec![0, 1, 2, 3]);
    // The 110px minimum is physical occupancy, not a tail postponed until
    // all four lines finish. With caller budgets25/50/50 it consumes25/50/35.
    // Normal saved counterparts: issue7353/minimum-band and stored-frame-end.
    // Line cuts stay1/2/1; the remaining physical band is accounted once.
    assert_eq!(reserved, vec![25.0, 50.0, 35.0]);
    assert!(cursor.is_complete());
    assert!(matches!(
        cursor.fit_in_page(area(50.0), 50.0).unwrap(),
        FragmentFit::Complete
    ));
}

#[test]
fn unknown_capacity_and_never_split_remain_strict() {
    let p = plan(SplitPolicy::BetweenRows, &[20.0; 4], 0.0);
    assert!(matches!(
        p.start().fit(area(50.0)).unwrap(),
        FragmentFit::DoesNotFit {
            required_height: 80.0,
            ..
        }
    ));
    let p = plan(SplitPolicy::Never, &[20.0; 4], 0.0);
    assert!(matches!(
        p.start().fit_in_page(area(50.0), 50.0).unwrap(),
        FragmentFit::DoesNotFit {
            required_height: 80.0,
            ..
        }
    ));
}
