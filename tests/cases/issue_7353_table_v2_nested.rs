//! Synthetic ownership and physical-space contracts. No Hancom fidelity claim.
use rhwp::model::{
    control::Control,
    paragraph::Paragraph,
    shape::{CommonObjAttr, HorzRelTo, TextWrap, VertRelTo},
    table::{Cell, Table, TablePageBreak},
    Padding,
};
use rhwp::renderer::table_v2::*;
use std::sync::Arc;

fn lines(paragraph: usize, height: f64) -> FlowBlock {
    FlowBlock::Lines {
        height,
        advance: height,
        lines: vec![LineBox {
            owner: LineOwner { paragraph, line: 0 },
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                width: 40.0,
                height,
            },
        }],
    }
}

fn child(policy: SplitPolicy) -> TableContentPlan {
    TableContentPlan::from_flow_rows(
        vec![60.0],
        vec![
            FlowRowInput {
                cells: vec![FlowCellInput {
                    padding: Insets::default(),
                    minimum_height: 0.0,
                    width: 60.0,
                    blocks: vec![lines(0, 20.0)],
                }],
            },
            FlowRowInput {
                cells: vec![FlowCellInput {
                    padding: Insets::default(),
                    minimum_height: 0.0,
                    width: 60.0,
                    blocks: vec![lines(1, 30.0)],
                }],
            },
        ],
        0.0,
        policy,
    )
    .unwrap()
}

fn wrapper(child: TableContentPlan) -> TableContentPlan {
    TableContentPlan::from_flow_rows(
        vec![100.0],
        vec![FlowRowInput {
            cells: vec![FlowCellInput {
                padding: Insets {
                    left: 5.0,
                    right: 15.0,
                    top: 3.0,
                    bottom: 7.0,
                },
                minimum_height: 0.0,
                width: 80.0,
                blocks: vec![
                    lines(0, 10.0),
                    FlowBlock::Table {
                        offset_x: 0.0,
                        restart_top: 0.0,
                        owner: ControlOwner {
                            paragraph: 1,
                            control: 0,
                        },
                        plan: Arc::new(child),
                    },
                    lines(2, 5.0),
                ],
            }],
        }],
        0.0,
        SplitPolicy::WithinCells,
    )
    .unwrap()
}

fn fit(cursor: &TableCursor, budget: f64) -> TableFragmentPlan {
    match cursor
        .fit(PageArea {
            bounds: Rect {
                x: 10.0,
                y: 20.0,
                width: 100.0,
                height: budget,
            },
        })
        .unwrap()
    {
        FragmentFit::Placed(f) => f,
        other => panic!("expected fit, got {other:?}"),
    }
}

#[test]
fn child_cut_is_retained_and_following_paragraph_stays_after_child() {
    let cursor = wrapper(child(SplitPolicy::BetweenRows)).start();
    // 3 top + 10 pre + first child row 20; second child row does not fit.
    let first = fit(&cursor, 40.0);
    assert_eq!(first.reserved_height(), 33.0);
    let cell = &first.placement().cells[0];
    assert_eq!(
        cell.lines
            .iter()
            .map(|l| l.owner.paragraph)
            .collect::<Vec<_>>(),
        vec![0]
    );
    assert_eq!(
        cell.tables[0].owner,
        ControlOwner {
            paragraph: 1,
            control: 0
        }
    );
    assert_eq!(cell.tables[0].placement.bounds.y, 33.0);
    assert_eq!(cell.tables[0].placement.bounds.height, 20.0);
    assert_eq!(cell.tables[0].placement.cells[0].row, 0);
    let second = fit(&first.continuation(), 42.0);
    let cell = &second.placement().cells[0];
    assert_eq!(cell.tables[0].placement.cells[0].row, 1);
    assert_eq!(cell.tables[0].placement.bounds.y, 20.0);
    assert_eq!(cell.tables[0].placement.bounds.height, 30.0);
    assert_eq!(cell.lines.len(), 1);
    assert_eq!(cell.lines[0].owner.paragraph, 2);
    assert_eq!(cell.lines[0].bounds.y, 50.0);
    assert_eq!(second.reserved_height(), 42.0); // 30 child + 5 post + 7 bottom
    assert!(second.continuation().is_complete());
    assert_eq!(fit(&cursor, 40.0).placement(), first.placement()); // no query mutation
}

#[test]
fn atomic_child_moves_intact_instead_of_becoming_a_prefix() {
    let first = fit(&wrapper(child(SplitPolicy::Never)).start(), 40.0);
    assert_eq!(first.reserved_height(), 13.0);
    assert!(first.placement().cells[0].tables.is_empty());
    let second = fit(&first.continuation(), 62.0);
    assert_eq!(
        second.placement().cells[0].tables[0].placement.cells.len(),
        2
    );
    assert_eq!(second.reserved_height(), 62.0);
    assert!(second.continuation().is_complete());
}

#[test]
fn nested_three_levels_keep_their_own_origins_and_cuts() {
    let middle = wrapper(child(SplitPolicy::BetweenRows));
    let root = TableContentPlan::from_flow_rows(
        vec![100.0],
        vec![FlowRowInput {
            cells: vec![FlowCellInput {
                width: 100.0,
                padding: Insets {
                    top: 1.0,
                    bottom: 1.0,
                    ..Insets::default()
                },
                minimum_height: 0.0,
                blocks: vec![FlowBlock::Table {
                    offset_x: 0.0,
                    restart_top: 0.0,
                    owner: ControlOwner {
                        paragraph: 0,
                        control: 0,
                    },
                    plan: Arc::new(middle),
                }],
            }],
        }],
        0.0,
        SplitPolicy::WithinCells,
    )
    .unwrap()
    .start();
    let first = fit(&root, 35.0);
    assert_eq!(first.reserved_height(), 34.0);
    let middle = &first.placement().cells[0].tables[0].placement;
    assert_eq!(middle.bounds.y, 21.0);
    let inner = &middle.cells[0].tables[0].placement;
    assert_eq!(inner.bounds.y, 34.0);
    assert_eq!(inner.bounds.height, 20.0);
    let second = fit(&first.continuation(), 43.0);
    assert_eq!(second.reserved_height(), 43.0);
    assert_eq!(
        second.placement().cells[0].tables[0].placement.cells[0].tables[0]
            .placement
            .cells[0]
            .row,
        1
    );
    assert!(second.continuation().is_complete());
}

#[test]
fn consumed_content_does_not_discard_remaining_padding_or_minimum_band() {
    let mut cursor = TableContentPlan::from_flow_rows(
        vec![100.0],
        vec![FlowRowInput {
            cells: vec![FlowCellInput {
                width: 100.0,
                padding: Insets {
                    top: 3.0,
                    bottom: 7.0,
                    ..Insets::default()
                },
                minimum_height: 35.0,
                blocks: vec![lines(0, 10.0)],
            }],
        }],
        0.0,
        SplitPolicy::WithinCells,
    )
    .unwrap()
    .start();
    let mut total = 0.0;
    let mut owners = Vec::new();
    for expected in [15.0, 15.0, 5.0] {
        let fragment = fit(&cursor, 15.0);
        assert_eq!(fragment.reserved_height(), expected);
        total += fragment.reserved_height();
        owners.extend(fragment.placement().cells[0].lines.iter().map(|l| l.owner));
        cursor = fragment.continuation();
    }
    assert_eq!(total, 35.0);
    assert_eq!(
        owners,
        vec![LineOwner {
            paragraph: 0,
            line: 0
        }]
    );
    assert!(cursor.is_complete());
}

#[test]
fn restarted_child_margin_is_budgeted_transactionally_and_painted_once() {
    // Independent arithmetic: child rows20/30; restart margin6. No padding.
    // A30px budget fits the row alone but cannot accept margin+row36px.
    let plan = TableContentPlan::from_flow_rows(
        vec![60.0],
        vec![FlowRowInput {
            cells: vec![FlowCellInput {
                padding: Insets::default(),
                minimum_height: 0.0,
                width: 60.0,
                blocks: vec![
                    FlowBlock::Table {
                        owner: ControlOwner {
                            paragraph: 0,
                            control: 0,
                        },
                        offset_x: 0.0,
                        restart_top: 6.0,
                        plan: Arc::new(child(SplitPolicy::BetweenRows)),
                    },
                    lines(2, 5.0),
                ],
            }],
        }],
        0.0,
        SplitPolicy::WithinCells,
    )
    .unwrap();
    let first = fit(&plan.start(), 20.0);
    assert_eq!(first.reserved_height(), 20.0);
    let cursor = first.continuation();
    for _ in 0..2 {
        assert!(matches!(
            cursor
                .fit(PageArea {
                    bounds: Rect {
                        x: 10.0,
                        y: 20.0,
                        width: 100.0,
                        height: 30.0,
                    }
                })
                .unwrap(),
            FragmentFit::DoesNotFit {
                required_height: 36.0,
                ..
            }
        ));
    }
    let second = fit(&cursor, 41.0);
    assert_eq!(second.reserved_height(), 41.0);
    let cell = &second.placement().cells[0];
    assert_eq!(cell.tables.len(), 1);
    assert_eq!(cell.tables[0].placement.bounds.y, 26.0);
    assert_eq!(cell.tables[0].placement.bounds.height, 30.0);
    assert_eq!(cell.lines[0].bounds.y, 56.0);
    assert!(second.continuation().is_complete());
    assert_eq!(fit(&cursor, 41.0).placement(), second.placement());
}

#[test]
fn zero_budget_never_advances_a_nonzero_line_and_oversize_is_reported() {
    let cursor = child(SplitPolicy::WithinCells).start();
    for budget in [0.0, 19.0] {
        assert!(matches!(
            cursor
                .fit(PageArea {
                    bounds: Rect {
                        x: 0.0,
                        y: 0.0,
                        width: 100.0,
                        height: budget
                    }
                })
                .unwrap(),
            FragmentFit::DoesNotFit {
                required_height: 20.0,
                ..
            }
        ));
    }
    assert_eq!(fit(&cursor, 20.0).placement().cells[0].lines.len(), 1);
}

struct SyntheticComposer;

#[test]
fn fractional_page_budget_does_not_split_an_atomic_nested_table() {
    let inner = TableContentPlan::from_flow_rows(
        vec![60.0],
        vec![FlowRowInput {
            cells: vec![FlowCellInput {
                width: 60.0,
                padding: Insets::default(),
                minimum_height: 0.0,
                blocks: vec![lines(0, 0.2)],
            }],
        }],
        0.0,
        SplitPolicy::Never,
    )
    .unwrap();
    let cursor = TableContentPlan::from_flow_rows(
        vec![100.0],
        vec![FlowRowInput {
            cells: vec![FlowCellInput {
                width: 100.0,
                padding: Insets::default(),
                minimum_height: 0.0,
                blocks: vec![
                    lines(0, 1.0),
                    FlowBlock::Table {
                        offset_x: 0.0,
                        restart_top: 0.0,
                        owner: ControlOwner {
                            paragraph: 1,
                            control: 0,
                        },
                        plan: Arc::new(inner),
                    },
                ],
            }],
        }],
        0.0,
        SplitPolicy::Never,
    )
    .unwrap()
    .start();
    let result = fit(&cursor, 1.2);
    assert!(result.continuation().is_complete());
    assert_eq!(result.placement().cells[0].tables.len(), 1);
    let child = &result.placement().cells[0].tables[0].placement.bounds;
    assert_eq!(child.y + child.height, 21.2);
}

#[test]
fn parallel_cells_resume_independently_without_replaying_short_cell_lines() {
    let cursor = TableContentPlan::from_flow_rows(
        vec![100.0, 100.0],
        vec![FlowRowInput {
            cells: vec![
                FlowCellInput {
                    width: 100.0,
                    padding: Insets::default(),
                    minimum_height: 0.0,
                    blocks: vec![FlowBlock::Table {
                        offset_x: 0.0,
                        restart_top: 0.0,
                        owner: ControlOwner {
                            paragraph: 0,
                            control: 0,
                        },
                        plan: Arc::new(child(SplitPolicy::BetweenRows)),
                    }],
                },
                FlowCellInput {
                    width: 100.0,
                    padding: Insets::default(),
                    minimum_height: 0.0,
                    blocks: vec![lines(0, 15.0), lines(1, 20.0)],
                },
            ],
        }],
        0.0,
        SplitPolicy::WithinCells,
    )
    .unwrap()
    .start();
    let query = |cursor: &TableCursor, height| match cursor
        .fit(PageArea {
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height,
            },
        })
        .unwrap()
    {
        FragmentFit::Placed(f) => f,
        other => panic!("{other:?}"),
    };
    let first = query(&cursor, 25.0);
    assert_eq!(first.reserved_height(), 20.0);
    assert_eq!(first.placement().cells[1].lines[0].owner.paragraph, 0);
    let second = query(&first.continuation(), 30.0);
    assert_eq!(second.reserved_height(), 30.0);
    assert_eq!(
        second.placement().cells[0].tables[0].placement.cells[0].row,
        1
    );
    let right = &second.placement().cells[1];
    assert_eq!(right.lines.len(), 1);
    assert_eq!(right.lines[0].owner.paragraph, 1);
    assert_eq!(right.lines[0].bounds.x, 100.0);
    assert_eq!(right.lines[0].bounds.y, 0.0);
    assert_eq!(right.bounds.height, 30.0);
    assert!(second.continuation().is_complete());
}

impl CellParagraphComposer for SyntheticComposer {
    fn compose(&self, p: &Paragraph, width: f64) -> Result<Vec<ParagraphItem>, GeometryError> {
        let mut items = vec![ParagraphItem::Lines {
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
        }];
        items.extend((0..p.controls.len()).map(ParagraphItem::TableControl));
        Ok(items)
    }
}

fn ir_leaf() -> Table {
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
            row_span: 1,
            col_span: 1,
            width: 60,
            paragraphs: vec![Paragraph::new_empty()],
            ..Default::default()
        }],
        ..Default::default()
    }
}

#[test]
fn paragraph_end_distinguishes_occupied_end_and_next_origin() {
    let end = ParagraphEnd::new(12.0, 12.0, vec![6.0, 2.0]).unwrap();
    assert_eq!(end.occupied_end(), 12.0);
    assert_eq!(end.next_origin(), 20.0);
    // Negative TAC spacing has already advanced the row pen by 30, although
    // its physical box extends to36. It must not be collapsed to the pen.
    let end = ParagraphEnd::new(36.0, 30.0, vec![2.0]).unwrap();
    assert_eq!(end.occupied_end(), 36.0);
    assert_eq!(end.next_origin(), 32.0);
    for tail in [vec![-1.0], vec![f64::NAN], vec![f64::INFINITY]] {
        assert!(ParagraphEnd::new(12.0, 12.0, tail).is_err());
    }
    assert!(ParagraphEnd::new(12.0, f64::MAX, vec![f64::MAX]).is_err());
}

#[test]
fn terminal_policy_does_not_guess_spacing_from_external_composer_tail() {
    struct Composer;
    impl CellParagraphComposer for Composer {
        fn compose(&self, p: &Paragraph, width: f64) -> Result<Vec<ParagraphItem>, GeometryError> {
            let mut items = SyntheticComposer.compose(p, width)?;
            items.push(ParagraphItem::End(ParagraphEnd::new(
                10.0,
                10.0,
                vec![6.0, 2.0],
            )?));
            Ok(items)
        }
    }
    for policy in [
        CellEndPolicy::PreserveAdvance,
        CellEndPolicy::OmitFinalLineGap,
        CellEndPolicy::OmitFinalParagraphGap,
    ] {
        let cursor =
            TableContentPlan::from_ir_contents_with_end_policy(&ir_leaf(), 1.0, &Composer, policy)
                .unwrap()
                .start();
        let fragment = fit(&cursor, 18.0);
        assert_eq!(fragment.reserved_height(), 18.0);
        assert_eq!(fragment.placement().cells[0].lines.len(), 1);
        assert_eq!(fragment.placement().cells[0].lines[0].bounds.height, 10.0);
        assert!(fragment.continuation().is_complete());
    }
}

#[test]
fn paragraph_end_bands_and_external_space_keep_blank_line_ownership() {
    struct Composer(f64);
    impl CellParagraphComposer for Composer {
        fn compose(&self, p: &Paragraph, width: f64) -> Result<Vec<ParagraphItem>, GeometryError> {
            let mut items = SyntheticComposer.compose(p, width)?;
            if p.text.is_empty() {
                items.push(ParagraphItem::End(ParagraphEnd::new(
                    10.0,
                    10.0,
                    vec![6.0, 2.0],
                )?));
                items.push(ParagraphItem::Space(self.0));
            } else {
                items.push(ParagraphItem::End(ParagraphEnd::new(10.0, 10.0, vec![])?));
            }
            Ok(items)
        }
    }
    for extra in [0.0, 2.0, 7.0] {
        let mut t = ir_leaf();
        t.cells[0].paragraphs.push(Paragraph {
            text: "after".into(),
            ..Paragraph::new_empty()
        });
        let cursor = TableContentPlan::from_ir_contents(&t, 1.0, &Composer(extra))
            .unwrap()
            .start();
        let fragment = fit(&cursor, 28.0 + extra);
        let lines = &fragment.placement().cells[0].lines;
        assert_eq!(lines.len(), 2);
        assert_eq!(
            lines[0].owner,
            LineOwner {
                paragraph: 0,
                line: 0
            }
        );
        assert_eq!(
            lines[1].owner,
            LineOwner {
                paragraph: 1,
                line: 0
            }
        );
        assert_eq!(lines[0].bounds.height, 10.0);
        assert_eq!(lines[1].bounds.y - lines[0].bounds.y, 18.0 + extra);
        assert_eq!(fragment.reserved_height(), 28.0 + extra);
        assert!(fragment.continuation().is_complete());
    }
}

#[test]
fn ir_adapter_uses_table_padding_and_real_paragraph_control_ownership() {
    let mut root = ir_leaf();
    root.cells[0].width = 100;
    root.padding = Padding {
        left: 5,
        right: 15,
        top: 3,
        bottom: 7,
    };
    root.cells[0].padding = Padding {
        left: 40,
        right: 40,
        top: 40,
        bottom: 40,
    }; // inactive
    root.cells[0].paragraphs.push(Paragraph {
        controls: vec![Control::Table(Box::new(ir_leaf()))],
        ..Paragraph::new_empty()
    });
    let cursor = TableContentPlan::from_ir_contents(&root, 1.0, &SyntheticComposer)
        .unwrap()
        .start();
    let fragment = fit(&cursor, 40.0);
    let cell = &fragment.placement().cells[0];
    assert_eq!(fragment.reserved_height(), 40.0); // 3 + 10 + 10 + child10 + 7
    assert_eq!(cell.lines[0].bounds.width, 80.0);
    assert_eq!(cell.lines[0].bounds.x, 15.0);
    assert_eq!(
        cell.tables[0].owner,
        ControlOwner {
            paragraph: 1,
            control: 0
        }
    );
    assert_eq!(cell.tables[0].placement.bounds.y, 43.0);
    assert!(fragment.continuation().is_complete());
    root.cells[0].apply_inner_margin = true;
    assert!(TableContentPlan::from_ir_contents(&root, 1.0, &SyntheticComposer).is_err());
    // child cannot fit inner 20
}

#[test]
fn unsupported_ir_and_missing_child_slots_do_not_silently_fallback() {
    struct MissingSlot;
    impl CellParagraphComposer for MissingSlot {
        fn compose(&self, _: &Paragraph, _: f64) -> Result<Vec<ParagraphItem>, GeometryError> {
            Ok(vec![])
        }
    }
    let mut root = ir_leaf();
    root.cells[0].width = 100;
    root.cells[0].paragraphs[0]
        .controls
        .push(Control::Table(Box::new(ir_leaf())));
    assert!(TableContentPlan::from_ir_contents(&root, 1.0, &MissingSlot).is_err());
    let Control::Table(child) = &mut root.cells[0].paragraphs[0].controls[0] else {
        unreachable!()
    };
    child.common.treat_as_char = true;
    assert!(TableContentPlan::from_ir_contents(&root, 1.0, &SyntheticComposer).is_err());
    root.cells[0].row_span = 2;
    assert!(TableContentPlan::from_ir_contents(&root, 1.0, &SyntheticComposer).is_err());
}

/// #5301's table-vs-cell margin contract at V2's actual geometry boundary.
/// The 36572 HU cell has inactive 510 HU side margins; table side margins are 0.
/// This is synthetic IR, not the original document's glyph/RenderTree test.
#[test]
fn ir_zero_table_horizontal_margin_keeps_full_nested_cell_width() {
    let mut child = ir_leaf();
    child.cells[0].width = 36572;
    child.cells[0].padding = Padding {
        left: 510,
        right: 510,
        top: 141,
        bottom: 141,
    };
    child.padding = Padding {
        left: 0,
        right: 0,
        top: 141,
        bottom: 141,
    };
    let mut root = ir_leaf();
    root.cells[0].width = 40000;
    root.cells[0].paragraphs[0]
        .controls
        .push(Control::Table(Box::new(child)));
    let plan = TableContentPlan::from_ir_contents(&root, 1.0, &SyntheticComposer).unwrap();
    let FragmentFit::Placed(fragment) = plan
        .start()
        .fit(PageArea {
            bounds: Rect {
                x: 50.0,
                y: 60.0,
                width: 40000.0,
                height: 302.0,
            },
        })
        .unwrap()
    else {
        panic!("20 line units + 282 padding units fit exactly")
    };
    let nested = &fragment.placement().cells[0].tables[0].placement;
    let line = &nested.cells[0].lines[0].bounds;
    assert_eq!(line.x, 50.0);
    assert_eq!(line.width, 36572.0);
    assert_eq!(line.y, 211.0); // page60 + host10 + child top141
    assert_eq!(nested.bounds.height, 292.0); // top141 + line10 + bottom141
    assert_eq!(fragment.reserved_height(), 302.0);
    assert!(fragment.continuation().is_complete());
}

/// #5301/#2070: explicitly selected zero margins are values, not missing fields.
#[test]
fn ir_explicit_zero_cell_padding_does_not_fall_back_to_table() {
    let mut table = ir_leaf();
    table.padding = Padding {
        left: 5,
        right: 15,
        top: 3,
        bottom: 7,
    };
    table.cells[0].apply_inner_margin = true;
    table.cells[0].padding = Padding::default();
    let plan = TableContentPlan::from_ir_contents(&table, 1.0, &SyntheticComposer).unwrap();
    let fragment = fit(&plan.start(), 10.0);
    assert_eq!(
        fragment.placement().cells[0].lines[0].bounds,
        Rect {
            x: 10.0,
            y: 20.0,
            width: 60.0,
            height: 10.0,
        }
    );
    assert_eq!(fragment.reserved_height(), 10.0);
    assert!(fragment.continuation().is_complete());
}

/// Legacy's malformed-input recovery (#6358) is not silently adopted as a V2 rule.
#[test]
fn ir_selected_negative_padding_is_rejected_without_legacy_repair() {
    let mut table = ir_leaf();
    table.padding = Padding {
        left: 5,
        right: 5,
        top: 3,
        bottom: 7,
    };
    table.cells[0].padding.left = -1;
    table.cells[0].apply_inner_margin = true;
    assert!(matches!(
        TableContentPlan::from_ir_contents(&table, 1.0, &SyntheticComposer),
        Err(GeometryError::InvalidNumber("resolved IR padding"))
    ));
    // The same unused field must not invalidate a valid table-default selection.
    table.cells[0].apply_inner_margin = false;
    let plan = TableContentPlan::from_ir_contents(&table, 1.0, &SyntheticComposer).unwrap();
    let fragment = fit(&plan.start(), 20.0);
    assert_eq!(fragment.placement().cells[0].lines[0].bounds.x, 15.0);
    assert_eq!(fragment.placement().cells[0].lines[0].bounds.width, 50.0);
    assert_eq!(fragment.reserved_height(), 20.0);
    assert!(fragment.continuation().is_complete());
}
