//! Content cuts and minimum physical height are independent continuation state.
//! Budgets below are caller-owned geometry, not measured from the implementation.
use rhwp::renderer::table_v2::*;
use std::sync::Arc;

fn tables<'a>(node: &'a serde_json::Value, out: &mut Vec<&'a serde_json::Value>) {
    if node["node_type"].get("Table").is_some() {
        out.push(node);
    }
    for child in node["children"].as_array().into_iter().flatten() {
        tables(child, out);
    }
}

#[test]
fn original_saved_frame_bottoms_match_independent_pdf_geometry() {
    let input = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp");
    let mut session = DocumentV2Session::from_bytes(
        &std::fs::read(input).unwrap(),
        r#"{"dpi":96,"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}"#,
    )
    .unwrap();
    // Independent PDF stroke paths at96dpi. Hancom prints84188HU paper at841pt;
    // normalize that known page scale, NOT a fitted transform of renderer output.
    let pdf_bottoms = [1021.922667, 1021.922667, 1016.968, 1010.254667, 832.369333];
    let scale = (84188.0 / 75.0) / (841.0 * 4.0 / 3.0);
    let mut count = 0;
    while let Some(page) = session.next_page_json().unwrap() {
        let page: serde_json::Value = serde_json::from_str(&page).unwrap();
        if count < pdf_bottoms.len() {
            let mut found = Vec::new();
            tables(&page["render_tree"]["root"], &mut found);
            let table = found
                .into_iter()
                .find(|n| n["node_type"]["Table"]["para_index"] == 5)
                .unwrap();
            let bottom =
                table["bbox"]["y"].as_f64().unwrap() + table["bbox"]["height"].as_f64().unwrap();
            let expected = pdf_bottoms[count] * scale;
            assert!(
                (bottom - expected).abs() < 0.25,
                "page {} bottom={bottom}, independent={expected}",
                count + 1
            );
        }
        count += 1;
    }
    assert_eq!(count, 7);
}

#[test]
fn normal_saved_tall_cell_first_fragment_matches_independent_pdf_extent() {
    let input = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/issue7353/minimum-band/cell-tall-saved.hwp");
    let mut session = DocumentV2Session::from_bytes(
        &std::fs::read(input).unwrap(),
        r#"{"dpi":96,"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}"#,
    )
    .unwrap();
    let page: serde_json::Value =
        serde_json::from_str(&session.next_page_json().unwrap().unwrap()).unwrap();
    let mut found = Vec::new();
    tables(&page["render_tree"]["root"], &mut found);
    // Independent PDF stroke edges at96dpi, same0.25px vector tolerance as
    // the original document contract. Normal saved common.height67913HU is
    // supplementary: the100HU compatibility model differs by1HU, so do not
    // claim exact integer reproduction of that stored field.
    let height = found[0]["bbox"]["height"].as_f64().unwrap();
    let scale = (84188.0 / 75.0) / (841.0 * 4.0 / 3.0);
    let expected = (1025.5986666666665 - 121.14799999999998) * scale;
    assert!(
        (height - expected).abs() < 0.25,
        "height={height}, independent={expected}"
    );
}

#[test]
fn saved_hwp_clearance_is_not_assumed_for_hwpx_provenance() {
    let input = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/issue7353/minimum-band/cell-tall-saved.hwp");
    let original = std::fs::read(input).unwrap();
    let doc = rhwp::parse_document(&original).unwrap();
    // Serialization control only, NOT a normal Hancom HWPX visual oracle.
    // Keep the saved rows but give the adapter an actual HWPX container.
    let hwpx = rhwp::serializer::hwpx::serialize_hwpx(&doc).unwrap();
    let mut heights = Vec::new();
    for data in [&original, &hwpx] {
        let mut session = DocumentV2Session::from_bytes(
            data,
            r#"{"dpi":96,"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}"#,
        )
        .unwrap();
        let page: serde_json::Value =
            serde_json::from_str(&session.next_page_json().unwrap().unwrap()).unwrap();
        let mut found = Vec::new();
        tables(&page["render_tree"]["root"], &mut found);
        heights.push(found[0]["bbox"]["height"].as_f64().unwrap());
    }
    assert!((heights[1] - heights[0] - 100.0 / 75.0).abs() < 1e-8);
}

#[test]
fn saved_cell_minimum_occupies_early_fragments_and_preserves_the_total_band() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/issue7353");
    let mut first_heights = Vec::new();
    for name in [
        "stored-frame-end/base2-saved.hwp",
        "minimum-band/cell-tall-saved.hwp",
    ] {
        let mut session = DocumentV2Session::from_bytes(
            &std::fs::read(root.join(name)).unwrap(),
            r#"{"dpi":96,"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}"#,
        )
        .unwrap();
        let mut heights = Vec::new();
        let mut texts = Vec::new();
        while let Some(page) = session.next_page_json().unwrap() {
            let page: serde_json::Value = serde_json::from_str(&page).unwrap();
            let mut found = Vec::new();
            tables(&page["render_tree"]["root"], &mut found);
            assert_eq!(found.len(), 1);
            let cell = &found[0]["children"][0];
            heights.push(cell["bbox"]["height"].as_f64().unwrap());
            for line in cell["children"].as_array().unwrap() {
                assert!(line["node_type"].get("TextLine").is_some());
                texts.push(
                    line["children"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter_map(|n| n["node_type"]["TextRun"]["text"].as_str())
                        .collect::<String>(),
                );
            }
        }
        assert_eq!(
            texts,
            (1..=65)
                .map(|i| format!("LINE {i:02} - saved frame control"))
                .collect::<Vec<_>>()
        );
        if name.starts_with("minimum-band/") {
            assert_eq!(heights.len(), 3);
            // Independent authored cell minimum, in HU at96dpi. This checks
            // preservation/distribution, NOT exact Hancom page-bottom fidelity.
            assert!((heights.iter().sum::<f64>() - 200000.0 / 75.0).abs() < 1e-8);
        } else {
            assert_eq!(heights.len(), 2);
            assert!((heights[0] - 67482.0 / 75.0).abs() < 1e-8);
        }
        first_heights.push(heights[0]);
    }
    // Normal Hancom save has67913HU vs67482HU. Before the fix both V2
    // first fragments were899.76px, hiding the changed physical minimum.
    assert!(first_heights[1] > first_heights[0]);
}

fn plan(minimum: f64, policy: SplitPolicy) -> TableContentPlan {
    let mut blocks = Vec::new();
    for paragraph in 0..3 {
        if paragraph > 0 {
            blocks.push(FlowBlock::StoredFrameStart);
        }
        blocks.push(FlowBlock::Lines {
            height: 20.0,
            advance: 20.0,
            lines: vec![LineBox {
                owner: LineOwner { paragraph, line: 0 },
                bounds: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 80.0,
                    height: 20.0,
                },
            }],
        });
    }
    TableContentPlan::from_flow_rows(
        vec![100.0],
        vec![FlowRowInput {
            cells: vec![FlowCellInput {
                width: 100.0,
                padding: Insets::default(),
                minimum_height: minimum,
                blocks,
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
            y: 100.0,
            width: 100.0,
            height,
        },
    }
}

fn placed(cursor: &TableCursor, height: f64) -> TableFragmentPlan {
    match cursor.fit(area(height)).unwrap() {
        FragmentFit::Placed(p) => p,
        other => panic!("expected fragment: {other:?}"),
    }
}

#[test]
fn minimum_band_is_consumed_with_each_accepted_content_cut() {
    let mut cursor = plan(120.0, SplitPolicy::WithinCells).start();
    // Three occupied 20px lines, one saved cut per frame. A 120px minimum
    // consumes 50 + 50 + 20, NOT 20 + 20 + 50 + an empty 30px page.
    for (paragraph, expected) in [50.0, 50.0, 20.0].into_iter().enumerate() {
        let fit = placed(&cursor, 50.0);
        assert_eq!(fit.reserved_height(), expected);
        let cell = &fit.placement().cells[0];
        assert_eq!(cell.bounds.height, expected);
        assert_eq!(cell.lines.len(), 1);
        assert_eq!(cell.lines[0].owner.paragraph, paragraph);
        assert_eq!(cell.lines[0].bounds.y, 100.0);
        assert!(cell.lines[0].bounds.y + 20.0 <= cell.bounds.y + cell.bounds.height);
        cursor = fit.continuation();
    }
    assert!(cursor.is_complete());
}

#[test]
fn physical_minimum_cannot_create_progress_before_a_blocked_unit() {
    let cursor = plan(120.0, SplitPolicy::WithinCells).start();
    for budget in [0.0, 19.0] {
        assert!(matches!(
            cursor.fit(area(budget)).unwrap(),
            FragmentFit::DoesNotFit {
                required_height: 20.0,
                ..
            }
        ));
    }
    assert_eq!(placed(&cursor, 50.0).reserved_height(), 50.0);
}

#[test]
fn ordinary_capacity_cut_cannot_publish_only_the_inserted_top_padding() {
    let plan = TableContentPlan::from_flow_rows(
        vec![100.0],
        vec![FlowRowInput {
            cells: vec![FlowCellInput {
                width: 100.0,
                padding: Insets {
                    top: 5.0,
                    ..Insets::default()
                },
                minimum_height: 50.0,
                blocks: vec![FlowBlock::Lines {
                    height: 20.0,
                    advance: 20.0,
                    lines: vec![LineBox {
                        owner: LineOwner {
                            paragraph: 0,
                            line: 0,
                        },
                        bounds: Rect {
                            x: 0.0,
                            y: 0.0,
                            width: 80.0,
                            height: 20.0,
                        },
                    }],
                }],
            }],
        }],
        0.0,
        SplitPolicy::WithinCells,
    )
    .unwrap();
    let cursor = plan.start();
    for budget in [5.0, 19.0, 24.0] {
        assert!(matches!(
            cursor.fit(area(budget)).unwrap(),
            FragmentFit::DoesNotFit {
                required_height: 25.0,
                ..
            }
        ));
    }
    let fit = placed(&cursor, 25.0);
    assert_eq!(fit.placement().cells[0].lines[0].bounds.y, 105.0);
    assert_eq!(fit.reserved_height(), 25.0);
    let tail = placed(&fit.continuation(), 50.0);
    assert_eq!(tail.reserved_height(), 25.0);
    assert!(tail.continuation().is_complete());
}

#[test]
fn another_cells_real_progress_can_commit_the_shared_row_fragment() {
    let cells = [(5.0, 40.0), (0.0, 20.0)]
        .into_iter()
        .enumerate()
        .map(|(paragraph, (top, h))| FlowCellInput {
            width: 50.0,
            padding: Insets {
                top,
                ..Insets::default()
            },
            minimum_height: 80.0,
            blocks: vec![FlowBlock::Lines {
                height: h,
                advance: h,
                lines: vec![LineBox {
                    owner: LineOwner { paragraph, line: 0 },
                    bounds: Rect {
                        x: 0.0,
                        y: 0.0,
                        width: 40.0,
                        height: h,
                    },
                }],
            }],
        })
        .collect();
    let plan = TableContentPlan::from_flow_rows(
        vec![50.0, 50.0],
        vec![FlowRowInput { cells }],
        0.0,
        SplitPolicy::WithinCells,
    )
    .unwrap();
    let first = placed(&plan.start(), 25.0);
    assert!(first.placement().cells[0].lines.is_empty());
    assert_eq!(first.placement().cells[1].lines.len(), 1);
    let second = placed(&first.continuation(), 50.0);
    assert_eq!(second.reserved_height(), 50.0);
    assert_eq!(second.placement().cells[0].lines[0].bounds.y, 100.0);
    assert!(second.placement().cells[1].lines.is_empty());
    let tail = placed(&second.continuation(), 50.0);
    assert_eq!(tail.reserved_height(), 5.0);
    assert!(tail.continuation().is_complete());
}

#[test]
fn exhausted_or_small_minimum_does_not_fill_later_pages() {
    for minimum in [0.0, 10.0, 70.0] {
        let expected = if minimum == 70.0 {
            [50.0, 20.0, 20.0]
        } else {
            [20.0; 3]
        };
        let mut cursor = plan(minimum, SplitPolicy::WithinCells).start();
        for height in expected {
            let fit = placed(&cursor, 50.0);
            assert_eq!(fit.reserved_height(), height);
            cursor = fit.continuation();
        }
        assert!(cursor.is_complete());
    }
}

#[test]
fn intact_minimum_still_reserves_whole_height() {
    let cursor = plan(120.0, SplitPolicy::Never).start();
    assert!(matches!(
        cursor.fit(area(119.0)).unwrap(),
        FragmentFit::DoesNotFit { .. }
    ));
    let fit = placed(&cursor, 120.0);
    assert_eq!(fit.reserved_height(), 120.0);
    assert_eq!(fit.placement().cells[0].lines.len(), 3);
    assert!(fit.continuation().is_complete());
}

#[test]
fn actual_physical_tail_survives_after_the_last_content_unit() {
    let mut cursor = plan(180.0, SplitPolicy::WithinCells).start();
    for (index, expected) in [50.0, 50.0, 50.0, 30.0].into_iter().enumerate() {
        let fit = placed(&cursor, 50.0);
        assert_eq!(fit.reserved_height(), expected);
        assert_eq!(fit.placement().cells[0].lines.len(), usize::from(index < 3));
        cursor = fit.continuation();
    }
    assert!(cursor.is_complete());
}

#[test]
fn nested_child_band_is_reserved_by_parent_without_recalculation() {
    let child = Arc::new(plan(120.0, SplitPolicy::WithinCells));
    let parent = TableContentPlan::from_flow_rows(
        vec![100.0],
        vec![FlowRowInput {
            cells: vec![FlowCellInput {
                width: 100.0,
                padding: Insets::default(),
                minimum_height: 0.0,
                blocks: vec![FlowBlock::Table {
                    owner: ControlOwner {
                        paragraph: 0,
                        control: 0,
                    },
                    offset_x: 0.0,
                    restart_top: 0.0,
                    plan: child,
                }],
            }],
        }],
        0.0,
        SplitPolicy::WithinCells,
    )
    .unwrap();
    let mut cursor = parent.start();
    for expected in [50.0, 50.0, 20.0] {
        let fit = placed(&cursor, 50.0);
        assert_eq!(fit.reserved_height(), expected);
        let cell = &fit.placement().cells[0];
        assert_eq!(cell.bounds.height, expected);
        assert_eq!(cell.tables[0].placement.bounds.height, expected);
        cursor = fit.continuation();
    }
    assert!(cursor.is_complete());
}
