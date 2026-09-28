//! Saved by Hancom, not manually edited LineSeg caches. Independent first
//! fragment heights and PDF provenance: fixtures/issue7353/stored-frame-end.
use rhwp::renderer::table_v2::*;
use serde_json::Value;

fn tables<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
    if node["node_type"].get("Table").is_some() {
        out.push(node);
    }
    if let Some(children) = node["children"].as_array() {
        for child in children {
            tables(child, out);
        }
    }
}

fn runs<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
    if node["node_type"].get("TextRun").is_some() {
        out.push(node);
    }
    if let Some(children) = node["children"].as_array() {
        for child in children {
            runs(child, out);
        }
    }
}

#[test]
fn saved_hancom_controls_end_at_occupied_extent_not_page_budget() {
    // Normal Hancom save common.height, corroborated by the paired printed PDF.
    // All six failed against the previous full-page-budget frame-end behavior.
    for (name, expected, count) in [
        ("base2", 899.76, 2),
        ("blank2", 899.76, 2),
        ("blankend3", 899.76, 2),
        ("padding2", 889.2133333333334, 2),
        ("outer2", 881.0933333333334, 2),
        ("gap3", 888.56, 3),
    ] {
        let path = format!(
            "{}/tests/fixtures/issue7353/stored-frame-end/{name}-saved.hwp",
            env!("CARGO_MANIFEST_DIR")
        );
        let mut session = DocumentV2Session::from_bytes(
            &std::fs::read(path).unwrap(),
            r#"{"dpi":96,"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}"#,
        )
        .unwrap();
        let mut pages = Vec::new();
        while let Some(page) = session.next_page_json().unwrap() {
            pages.push(serde_json::from_str::<Value>(&page).unwrap());
        }
        assert_eq!(pages.len(), count, "{name}: no padding-only page");
        let mut first = Vec::new();
        tables(&pages[0]["render_tree"]["root"], &mut first);
        let actual = first[0]["bbox"]["height"].as_f64().unwrap();
        assert!(
            (actual - expected).abs() < 1e-8,
            "{name}: {actual} != {expected}"
        );
        let first_count = match name {
            "gap3" => 30,
            "padding2" | "outer2" => 47,
            _ => 48,
        };
        let mut owners = Vec::new();
        let mut after = Vec::new();
        let mut last_bottom = 0.0;
        for (page_index, page) in pages.iter().enumerate() {
            let mut frame = Vec::new();
            tables(&page["render_tree"]["root"], &mut frame);
            assert_eq!(frame.len(), 1);
            let cell = &frame[0]["children"][0];
            let lines = cell["children"].as_array().unwrap();
            if page_index == 0 {
                assert_eq!(lines.len(), first_count, "{name}: first cut");
            }
            let cell_bottom =
                cell["bbox"]["y"].as_f64().unwrap() + cell["bbox"]["height"].as_f64().unwrap();
            last_bottom = cell_bottom;
            for line in lines {
                assert!(line["node_type"].get("TextLine").is_some());
                assert!((line["bbox"]["height"].as_f64().unwrap() - 1400.0 / 75.0).abs() < 1e-8);
                assert!(line["bbox"]["y"].as_f64().unwrap() + 1400.0 / 75.0 <= cell_bottom + 1e-8);
                let mut text = Vec::new();
                runs(line, &mut text);
                owners.push(
                    text.iter()
                        .map(|r| r["node_type"]["TextRun"]["text"].as_str().unwrap())
                        .collect::<String>(),
                );
            }
            let mut text = Vec::new();
            runs(&page["render_tree"]["root"], &mut text);
            after.extend(
                text.into_iter()
                    .filter(|r| r["node_type"]["TextRun"]["text"] == "AFTER TABLE")
                    .map(|r| (page_index, r["bbox"]["y"].as_f64().unwrap())),
            );
        }
        assert_eq!(
            owners,
            (1..=65)
                .map(|i| {
                    if (name == "blank2" && i == 41) || (name == "blankend3" && i == 48) {
                        String::new()
                    } else {
                        format!("LINE {i:02} - saved frame control")
                    }
                })
                .collect::<Vec<_>>(),
            "{name}: every line, including blanks, exactly once"
        );
        assert_eq!(after.len(), 1, "{name}: trailing paragraph preserved");
        assert_eq!(after[0].0, count - 1);
        assert!(
            after[0].1 + 1e-8 >= last_bottom,
            "{name}: trailing paragraph outside table"
        );
    }
}

fn plan(policy: SplitPolicy, advance: f64) -> TableContentPlan {
    padded_plan(policy, advance, 0.0)
}

fn padded_plan(policy: SplitPolicy, advance: f64, top: f64) -> TableContentPlan {
    let line = |paragraph, height, advance| FlowBlock::Lines {
        height,
        advance,
        lines: vec![LineBox {
            owner: LineOwner { paragraph, line: 0 },
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                width: 80.0,
                height,
            },
        }],
    };
    TableContentPlan::from_flow_rows(
        vec![100.0],
        vec![FlowRowInput {
            cells: vec![FlowCellInput {
                width: 100.0,
                padding: Insets {
                    top,
                    bottom: 5.0,
                    ..Insets::default()
                },
                minimum_height: 0.0,
                blocks: vec![
                    line(0, 20.0, advance),
                    FlowBlock::StoredFrameTail {
                        spaces: vec![10.0, 3.0],
                        paragraph_after: 3.0,
                    },
                    FlowBlock::StoredFrameStart,
                    line(1, 10.0, 10.0),
                ],
            }],
        }],
        0.0,
        policy,
    )
    .unwrap()
}

#[test]
fn first_frame_padding_cannot_commit_without_its_first_unit() {
    let start = padded_plan(SplitPolicy::WithinCells, 20.0, 5.0).start();
    for height in [4.0, 5.0, 25.0, 32.0] {
        let FragmentFit::DoesNotFit {
            required_height, ..
        } = start.fit_in_page(area(height), 100.0).unwrap()
        else {
            panic!("padding-only frame at budget {height}")
        };
        assert_eq!(required_height, 33.0);
    }
    let FragmentFit::Placed(first) = start.fit_in_page(area(33.0), 100.0).unwrap() else {
        panic!()
    };
    assert_eq!(first.reserved_height(), 33.0);
    assert_eq!(first.placement().cells[0].lines[0].bounds.y, 35.0);
}

fn area(height: f64) -> PageArea {
    PageArea {
        bounds: Rect {
            x: 7.0,
            y: 30.0,
            width: 100.0,
            height,
        },
    }
}

#[test]
fn final_unit_reserves_after_and_bottom_before_acceptance() {
    for advance in [15.0, 20.0] {
        let start = plan(SplitPolicy::WithinCells, advance).start();
        for height in [4.0, 20.0, 27.0] {
            let FragmentFit::DoesNotFit {
                required_height, ..
            } = start.fit_in_page(area(height), 100.0).unwrap()
            else {
                panic!("accepted line without its physical end: {height}")
            };
            assert_eq!(required_height, 28.0);
        }
        for height in [28.0, 100.0] {
            let FragmentFit::Placed(first) = start.fit_in_page(area(height), 100.0).unwrap() else {
                panic!()
            };
            assert_eq!(first.reserved_height(), 28.0);
            assert_eq!(first.placement().cells[0].lines.len(), 1);
            let FragmentFit::Placed(last) =
                first.continuation().fit_in_page(area(15.0), 100.0).unwrap()
            else {
                panic!()
            };
            assert_eq!(last.placement().cells[0].lines[0].owner.paragraph, 1);
            assert_eq!(last.placement().cells[0].lines[0].bounds.y, 30.0);
            assert!(last.continuation().is_complete());
        }
    }
}

#[test]
fn intact_queries_preserve_tail_and_do_not_repeat_frame_insets() {
    for policy in [SplitPolicy::Never, SplitPolicy::BetweenRows] {
        let FragmentFit::Placed(fragment) = plan(policy, 20.0)
            .start()
            .fit_in_page(area(100.0), 100.0)
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(fragment.reserved_height(), 48.0);
        assert_eq!(fragment.placement().cells[0].lines[1].bounds.y, 63.0);
        assert!(fragment.continuation().is_complete());
    }
}
