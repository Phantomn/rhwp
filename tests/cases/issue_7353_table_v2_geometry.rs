//! Synthetic geometry contracts, not evidence of HWP/Hancom fidelity.
//! Expected positions are derived from explicit boxes/padding/page budgets.
use rhwp::renderer::table_v2::{
    CellInput, ComposedCell, FlowBlock, FlowCellInput, FlowRowInput, FragmentFit, GeometryError,
    Insets, LineBox, LineOwner, PageArea, Rect, RowInput, SplitPolicy, TableContentPlan,
    TableCursor, TableFragmentPlan,
};

#[test]
fn exact_fractional_padding_fit_is_independent_of_page_origin() {
    // Source-style units at96dpi:141HU padding +300HU blank line +141HU padding.
    // End coordinates must not turn the same measured band into a partial band.
    let padding = 141.0 * (96.0 / 7200.0);
    let h = padding + 4.0 + padding;
    for y in [0.0, 30.0, 1000.0] {
        let cursor = TableContentPlan::from_flow_rows(
            vec![100.0],
            vec![FlowRowInput {
                cells: vec![FlowCellInput {
                    padding: Insets {
                        top: padding,
                        bottom: padding,
                        ..Default::default()
                    },
                    minimum_height: 0.0,
                    width: 100.0,
                    blocks: vec![FlowBlock::Lines {
                        height: 4.0,
                        advance: 4.0,
                        lines: vec![line(0, 0.0, 4.0)],
                    }],
                }],
            }],
            0.0,
            SplitPolicy::Never,
        )
        .unwrap()
        .start();
        let a = PageArea {
            bounds: Rect {
                x: 10.0,
                y,
                width: 100.0,
                height: h,
            },
        };
        let FragmentFit::Placed(f) = cursor.fit(a).unwrap() else {
            panic!("exact budget")
        };
        assert!(f.continuation().is_complete());
        assert_eq!(f.reserved_height(), h);
        assert_eq!(f.placement().cells[0].lines[0].bounds.y, y + padding);
        assert!(matches!(
            cursor
                .fit(PageArea {
                    bounds: Rect {
                        height: h - 1.0 / 75.0,
                        ..a.bounds
                    }
                })
                .unwrap(),
            FragmentFit::DoesNotFit { .. }
        ));
    }
}

fn area(width: f64, height: f64) -> PageArea {
    PageArea {
        bounds: Rect {
            x: 10.0,
            y: 20.0,
            width,
            height,
        },
    }
}

fn line(index: usize, y: f64, height: f64) -> LineBox {
    LineBox {
        owner: LineOwner {
            paragraph: index,
            line: 0,
        },
        bounds: Rect {
            x: 0.0,
            y,
            width: 80.0,
            height,
        },
    }
}

fn cell(height: f64, lines: Vec<LineBox>) -> CellInput {
    CellInput {
        padding: Insets {
            left: 5.0,
            right: 15.0,
            top: 3.0,
            bottom: 7.0,
        },
        minimum_height: 0.0,
        content: ComposedCell {
            width: 80.0,
            height,
            lines,
        },
    }
}

fn plan(heights: &[f64], spacing: f64, policy: SplitPolicy) -> TableContentPlan {
    TableContentPlan::new(
        vec![100.0],
        heights
            .iter()
            .map(|&h| RowInput {
                cells: vec![cell(h, vec![line(0, 0.0, h)])],
            })
            .collect(),
        spacing,
        policy,
    )
    .unwrap()
}

fn placed(cursor: &TableCursor, height: f64) -> TableFragmentPlan {
    match cursor.fit(area(100.0, height)).unwrap() {
        FragmentFit::Placed(fragment) => fragment,
        other => panic!("expected placed, got {other:?}"),
    }
}

#[test]
fn blank_lines_and_trailing_band_are_reserved_and_placed() {
    // 4-unit blank line + 6-unit inter-paragraph gap + 14-unit line + 8-unit
    // trailing band = 32 content units. Padding makes a 42-unit row.
    let cursor = TableContentPlan::new(
        vec![100.0],
        vec![RowInput {
            cells: vec![cell(32.0, vec![line(0, 0.0, 4.0), line(1, 10.0, 14.0)])],
        }],
        0.0,
        SplitPolicy::BetweenRows,
    )
    .unwrap()
    .start();
    assert!(matches!(
        cursor.fit(area(100.0, 41.0)).unwrap(),
        FragmentFit::DoesNotFit {
            required_height: 42.0,
            ..
        }
    ));
    let f = placed(&cursor, 42.0);
    assert_eq!(f.reserved_height(), 42.0);
    let p = f.placement();
    assert_eq!(
        p.bounds,
        Rect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 42.0
        }
    );
    assert_eq!(p.cells[0].content_origin, (15.0, 23.0));
    assert_eq!(p.cells[0].lines[0].bounds.y, 23.0);
    assert_eq!(p.cells[0].lines[1].bounds.y, 33.0);
    assert_eq!(p.cells[0].lines[1].bounds.height, 14.0);
    assert_eq!(p.cells[0].bounds.height, 42.0);
    assert!(f.continuation().is_complete());
}

#[test]
fn overlap_is_not_a_page_break_and_empty_band_is_not_removed() {
    let cursor = TableContentPlan::new(
        vec![100.0],
        vec![RowInput {
            cells: vec![cell(25.0, vec![line(0, 0.0, 15.0), line(1, 10.0, 15.0)])],
        }],
        0.0,
        SplitPolicy::BetweenRows,
    )
    .unwrap()
    .start();
    let f = placed(&cursor, 35.0);
    assert_eq!(f.placement().cells[0].lines.len(), 2);
    assert_eq!(f.placement().cells[0].lines[1].bounds.y, 33.0);
    assert_eq!(f.reserved_height(), 35.0);
}

#[test]
fn row_boundary_continuations_preserve_every_owner_and_geometry() {
    // Physical rows 20,30,40. Within a fragment the row gap is 2.
    let cursor = plan(&[10.0, 20.0, 30.0], 2.0, SplitPolicy::BetweenRows).start();
    let f = placed(&cursor, 52.0);
    assert_eq!(f.rows(), 0..2);
    assert_eq!(f.reserved_height(), 52.0);
    assert_eq!(f.placement().cells[1].bounds.y, 42.0);
    assert_eq!(f.placement().cells[1].bounds.height, 30.0);
    let next = f.continuation();
    assert!(matches!(
        next.fit(area(100.0, 39.0)).unwrap(),
        FragmentFit::DoesNotFit { .. }
    ));
    let tail = placed(&next, 40.0);
    assert_eq!(tail.rows(), 2..3);
    assert_eq!(tail.placement().cells[0].row, 2);
    assert_eq!(tail.placement().cells[0].bounds.y, 20.0); // no replayed leading gap
    assert_eq!(tail.reserved_height(), 40.0);
    assert!(matches!(
        tail.continuation().fit(area(100.0, 40.0)).unwrap(),
        FragmentFit::Complete
    ));
    // Querying another budget never commits the original cursor.
    assert_eq!(placed(&cursor, 20.0).rows(), 0..1);
}

#[test]
fn atomic_policy_does_not_consume_a_prefix() {
    let cursor = plan(&[10.0, 20.0], 2.0, SplitPolicy::Never).start();
    assert!(matches!(
        cursor.fit(area(100.0, 51.0)).unwrap(),
        FragmentFit::DoesNotFit {
            required_height: 52.0,
            ..
        }
    ));
    assert_eq!(placed(&cursor, 52.0).rows(), 0..2);
}

#[test]
fn shared_row_height_keeps_short_cell_and_next_row_aligned() {
    let mut first = cell(10.0, vec![line(0, 0.0, 10.0)]);
    first.minimum_height = 60.0;
    let cursor = TableContentPlan::new(
        vec![100.0, 100.0],
        vec![RowInput {
            cells: vec![first, cell(30.0, vec![line(0, 0.0, 30.0)])],
        }],
        0.0,
        SplitPolicy::BetweenRows,
    )
    .unwrap()
    .start();
    let f = match cursor.fit(area(200.0, 60.0)).unwrap() {
        FragmentFit::Placed(f) => f,
        other => panic!("{other:?}"),
    };
    assert_eq!(f.reserved_height(), 60.0);
    assert_eq!(f.placement().cells[0].bounds.height, 60.0);
    assert_eq!(f.placement().cells[1].bounds.height, 60.0);
    assert_eq!(f.placement().cells[1].content_origin, (115.0, 23.0));
}

#[test]
fn different_width_requires_recomposition_not_squeezing() {
    let mut c = cell(10.0, vec![]);
    c.content.width = 79.0;
    assert!(matches!(
        TableContentPlan::new(
            vec![100.0],
            vec![RowInput { cells: vec![c] }],
            0.0,
            SplitPolicy::BetweenRows
        ),
        Err(GeometryError::ContentWidth { .. })
    ));
    let cursor = plan(&[10.0], 0.0, SplitPolicy::BetweenRows).start();
    assert!(matches!(
        cursor.fit(area(99.0, 100.0)).unwrap(),
        FragmentFit::DoesNotFit {
            required_width: 100.0,
            ..
        }
    ));
}

#[test]
fn unsupported_splits_and_invalid_content_are_explicit_errors() {
    assert!(matches!(
        TableContentPlan::new(
            vec![100.0],
            vec![RowInput {
                cells: vec![cell(1.0, vec![])]
            }],
            0.0,
            SplitPolicy::WithinCells
        ),
        Err(GeometryError::UnsupportedCellSplit)
    ));
    for height in [f64::NAN, f64::INFINITY, -1.0] {
        assert!(TableContentPlan::new(
            vec![100.0],
            vec![RowInput {
                cells: vec![cell(height, vec![])]
            }],
            0.0,
            SplitPolicy::Never
        )
        .is_err());
    }
    assert!(matches!(
        TableContentPlan::new(
            vec![100.0],
            vec![RowInput {
                cells: vec![cell(1.0, vec![line(0, 0.0, 2.0)])],
            }],
            0.0,
            SplitPolicy::Never
        ),
        Err(GeometryError::ContentBounds { .. })
    ));
    assert!(matches!(
        TableContentPlan::new(
            vec![100.0],
            vec![RowInput {
                cells: vec![cell(1.0, vec![line(0, 0.0, 1.0), line(0, 0.0, 1.0)])],
            }],
            0.0,
            SplitPolicy::Never
        ),
        Err(GeometryError::DuplicateLineOwner { .. })
    ));
}

#[test]
fn recomposition_cannot_rebind_an_existing_continuation() {
    let first = plan(&[10.0, 20.0], 0.0, SplitPolicy::BetweenRows).start();
    let old_tail = placed(&first, 20.0).continuation();
    let new_plan = plan(&[70.0], 0.0, SplitPolicy::BetweenRows).start();
    assert_eq!(placed(&new_plan, 80.0).reserved_height(), 80.0);
    assert_eq!(placed(&old_tail, 30.0).reserved_height(), 30.0);
    assert_eq!(placed(&old_tail, 30.0).rows(), 1..2);
}
