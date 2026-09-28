//! Original saved HWP + independent corresponding Hancom PDF p2. Synthetic
//! geometry below tests cut ownership, not additional Hancom fidelity claims.
use rhwp::renderer::table_v2::*;
use serde_json::Value;

fn collect<'a>(n: &'a Value, kind: &str, out: &mut Vec<&'a Value>) {
    if n["node_type"].get(kind).is_some() {
        out.push(n);
    }
    if let Some(children) = n["children"].as_array() {
        for child in children {
            collect(child, kind, out);
        }
    }
}

#[test]
fn original_blank_line_belongs_to_second_page_before_title() {
    let bytes =
        include_bytes!("../fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp");
    let mut s = DocumentV2Session::from_bytes(
        bytes,
        r#"{"dpi":96,"max_pages":100,"cell_end_policy":"omit_final_paragraph_gap"}"#,
    )
    .unwrap();
    let mut occurrences = Vec::new();
    let mut title = None;
    let mut page = 0;
    while let Some(raw) = s.next_page_json().unwrap() {
        let v: Value = serde_json::from_str(&raw).unwrap();
        if page < 2 {
            let mut tables = Vec::new();
            collect(&v["render_tree"]["root"], "Table", &mut tables);
            let parent = tables
                .iter()
                .find(|t| t["node_type"]["Table"]["para_index"] == 5)
                .unwrap();
            let cell = parent["children"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["node_type"].get("TableCell").is_some())
                .unwrap();
            let first = &cell["children"][0];
            assert!(first["node_type"].get("TextLine").is_some());
            // The original parent's first line (pi0 on p1, pi6 on p2) starts
            // inside its141HU top inset. PDF p2 title top126.103px minus the
            // frame edge98.292px independently confirms the padded origin:
            // (141 +1500 +316 +141)/75 =27.9733px (PDF raster rounding aside).
            let y = |n: &Value| n["bbox"]["y"].as_f64().unwrap();
            assert!(
                (y(first) - y(cell) - 141.0 / 75.0).abs() < 1e-8,
                "page{}: parent first blank origin must include cell inset; cell={}, line={}",
                page + 1,
                y(cell),
                y(first)
            );
        }
        if page < 4 {
            let mut bodies = Vec::new();
            let mut tables = Vec::new();
            collect(&v["render_tree"]["root"], "Body", &mut bodies);
            collect(&v["render_tree"]["root"], "Table", &mut tables);
            let parent = tables
                .iter()
                .find(|t| t["node_type"]["Table"]["para_index"] == 5)
                .unwrap();
            let bottom = |n: &Value| {
                n["bbox"]["y"].as_f64().unwrap() + n["bbox"]["height"].as_f64().unwrap()
            };
            // Page ownership does not imply filling the whole page body.
            // Exact frame-end geometry is covered by independent saved controls.
            assert!(bottom(parent) <= bottom(bodies[0]) + 1e-8);
        }
        let mut lines = Vec::new();
        collect(&v["render_tree"]["root"], "TextLine", &mut lines);
        for line in lines {
            let mut runs = Vec::new();
            collect(line, "TextRun", &mut runs);
            for run in runs {
                let r = &run["node_type"]["TextRun"];
                if r["para_shape_id"] == 64 && r["char_shape_id"] == 93 && r["text"] == "  " {
                    occurrences.push((
                        page,
                        line["bbox"]["y"].as_f64().unwrap(),
                        line["bbox"]["height"].as_f64().unwrap(),
                    ));
                }
            }
        }
        if page == 1 {
            let mut tables = Vec::new();
            collect(&v["render_tree"]["root"], "Table", &mut tables);
            for t in tables {
                let mut runs = Vec::new();
                collect(t, "TextRun", &mut runs);
                if runs
                    .iter()
                    .any(|r| r["node_type"]["TextRun"]["text"] == "사건의 발단·조사배경")
                {
                    // The last matching table is the title child, not its parent.
                    title = Some(t["bbox"]["y"].as_f64().unwrap());
                }
            }
        }
        page += 1;
    }
    assert_eq!(
        occurrences.len(),
        1,
        "blank owner must not disappear or repeat"
    );
    assert_eq!(
        occurrences[0].0, 1,
        "Hancom PDF p2 starts with source cell paragraph6, not paragraph7"
    );
    assert!((occurrences[0].2 - 1500.0 / 75.0).abs() < 1e-8);
    // Source blank line1500 + spacing316; title top margin141HU.
    assert!((title.unwrap() - occurrences[0].1 - (1500.0 + 316.0 + 141.0) / 75.0).abs() < 1e-8);
}

fn frame_plan(policy: SplitPolicy) -> TableContentPlan {
    padded_frame_plan(policy, 0.0)
}

fn padded_frame_plan(policy: SplitPolicy, top: f64) -> TableContentPlan {
    let line = |paragraph, height| FlowBlock::Lines {
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
    };
    TableContentPlan::from_flow_rows(
        vec![100.0],
        vec![FlowRowInput {
            cells: vec![FlowCellInput {
                width: 100.0,
                padding: Insets {
                    top,
                    ..Insets::default()
                },
                minimum_height: 0.0,
                blocks: vec![
                    line(0, 20.0),
                    FlowBlock::Space(4.0),
                    FlowBlock::StoredFrameStart,
                    line(1, 10.0),
                    FlowBlock::Space(3.0),
                    line(2, 20.0),
                ],
            }],
        }],
        0.0,
        policy,
    )
    .unwrap()
}

#[test]
fn frame_inset_is_reserved_once_and_cannot_commit_without_first_line() {
    let area = |height| PageArea {
        bounds: Rect {
            x: 7.0,
            y: 30.0,
            width: 100.0,
            height,
        },
    };
    let start = padded_frame_plan(SplitPolicy::WithinCells, 5.0).start();
    let FragmentFit::Placed(first) = start.fit_in_page(area(100.0), 100.0).unwrap() else {
        panic!()
    };
    assert_eq!(first.placement().cells[0].lines[0].bounds.y, 35.0);
    let next = first.continuation();
    for height in [0.0, 4.0, 14.0] {
        let FragmentFit::DoesNotFit {
            required_height, ..
        } = next.fit_in_page(area(height), 100.0).unwrap()
        else {
            panic!("inset-only page")
        };
        assert_eq!(required_height, 15.0);
    }
    let FragmentFit::Placed(full) = next.fit_in_page(area(38.0), 100.0).unwrap() else {
        panic!()
    };
    let c = &full.placement().cells[0];
    assert_eq!(c.content_origin.1, 35.0);
    assert_eq!(
        c.lines
            .iter()
            .map(|l| (l.owner.paragraph, l.bounds.y))
            .collect::<Vec<_>>(),
        [(1, 35.0), (2, 48.0)]
    );
    assert!(full.continuation().is_complete());
    assert_eq!(full.reserved_height(), 38.0);
    // A capacity cut inside the frame is not a new saved frame. Do not repeat
    // either the frame inset or its first blank on that continuation.
    let FragmentFit::Placed(part) = next.fit_in_page(area(18.0), 100.0).unwrap() else {
        panic!()
    };
    let FragmentFit::Placed(tail) = part.continuation().fit_in_page(area(20.0), 100.0).unwrap()
    else {
        panic!()
    };
    assert_eq!(tail.placement().cells[0].lines[0].owner.paragraph, 2);
    assert_eq!(tail.placement().cells[0].lines[0].bounds.y, 30.0);
    assert!(tail.continuation().is_complete());
}

#[test]
fn frame_cut_keeps_following_blank_and_retry_is_pure() {
    let area = |height| PageArea {
        bounds: Rect {
            x: 7.0,
            y: 30.0,
            width: 100.0,
            height,
        },
    };
    let start = frame_plan(SplitPolicy::WithinCells).start();
    let FragmentFit::Placed(first) = start.fit_in_page(area(100.0), 100.0).unwrap() else {
        panic!()
    };
    assert_eq!(first.reserved_height(), 24.0);
    assert_eq!(
        first.placement().cells[0]
            .lines
            .iter()
            .map(|l| l.owner.paragraph)
            .collect::<Vec<_>>(),
        [0]
    );
    let cursor = first.continuation();
    assert!(matches!(
        cursor.fit_in_page(area(9.0), 100.0).unwrap(),
        FragmentFit::DoesNotFit { .. }
    ));
    for _ in 0..2 {
        let FragmentFit::Placed(second) = cursor.fit_in_page(area(100.0), 100.0).unwrap() else {
            panic!()
        };
        assert_eq!(second.reserved_height(), 33.0);
        assert_eq!(
            second.placement().cells[0]
                .lines
                .iter()
                .map(|l| (l.owner.paragraph, l.bounds.y))
                .collect::<Vec<_>>(),
            [(1, 30.0), (2, 43.0)]
        );
        assert!(second.continuation().is_complete());
        assert!(matches!(
            second
                .continuation()
                .fit_in_page(area(100.0), 100.0)
                .unwrap(),
            FragmentFit::Complete
        ));
    }
    // Boundary reached exactly at a physical cut must not add an empty page.
    let FragmentFit::Placed(exact) = start.fit_in_page(area(24.0), 100.0).unwrap() else {
        panic!()
    };
    assert_eq!(exact.reserved_height(), 24.0);
    let FragmentFit::Placed(last) = exact
        .continuation()
        .fit_in_page(area(100.0), 100.0)
        .unwrap()
    else {
        panic!()
    };
    assert!(last.continuation().is_complete());
}

#[test]
fn intact_queries_do_not_turn_saved_frames_into_authored_breaks() {
    let area = PageArea {
        bounds: Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        },
    };
    for policy in [SplitPolicy::Never, SplitPolicy::BetweenRows] {
        let FragmentFit::Placed(f) = frame_plan(policy).start().fit_in_page(area, 100.0).unwrap()
        else {
            panic!()
        };
        assert!(f.continuation().is_complete());
        assert_eq!(f.reserved_height(), 57.0);
        assert_eq!(f.placement().cells[0].lines.len(), 3);
    }
}

#[test]
fn stored_origin_reset_is_not_overlapping_line_boxes_or_local_zero_origins() {
    use rhwp::{
        model::{control::Control, table::TablePageBreak},
        renderer::style_resolver::resolve_styles,
    };
    let d = rhwp::parse_document(include_bytes!(
        "../fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp"
    ))
    .unwrap();
    let Control::Table(original) = &d.sections[0].paragraphs[5].controls[0] else {
        panic!()
    };
    let styles = resolve_styles(&d.doc_info, 96.0);
    // Controlled synthetic variants of the source blank's style/metrics.
    // Both line boxes are20px. Positive origin progression2.667px means
    // intentional overlap, NOT the start of another saved page frame.
    for (origins, expected_lines, complete) in [
        ([1000, 1200], 2, true),
        ([0, 0], 2, true),
        ([1000, 0], 1, false),
    ] {
        let mut t = original.clone();
        t.page_break = TablePageBreak::RowBreak;
        t.cells[0].height = 0;
        t.cells[0].paragraphs = vec![original.cells[0].paragraphs[6].clone(); 2];
        for (i, p) in t.cells[0].paragraphs.iter_mut().enumerate() {
            p.line_segs[0].vertical_pos = origins[i];
            p.line_segs[0].line_spacing = if i == 0 { -1300 } else { 0 };
        }
        let p = PreparedTextTable::prepare(&t, &styles, 96.0).unwrap();
        let TextFragmentFit::Placed(f) = p
            .start()
            .fit(PageArea {
                bounds: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 700.0,
                    height: 200.0,
                },
            })
            .unwrap()
        else {
            panic!()
        };
        let lines = &f.geometry().placement().cells[0].lines;
        assert_eq!(lines.len(), expected_lines, "origins {origins:?}");
        assert_eq!(f.geometry().continuation().is_complete(), complete);
        if origins == [1000, 1200] {
            assert!((lines[1].bounds.y - lines[0].bounds.y - 200.0 / 75.0).abs() < 1e-8);
        }
    }
}

#[test]
fn reflow_composer_does_not_inherit_stored_page_membership() {
    use rhwp::model::{control::Control, paragraph::Paragraph, table::TablePageBreak};
    struct Reflow;
    impl CellParagraphComposer for Reflow {
        fn compose(&self, _: &Paragraph, width: f64) -> Result<Vec<ParagraphItem>, GeometryError> {
            Ok(vec![ParagraphItem::Lines {
                height: 20.0,
                advance: 20.0,
                lines: vec![(
                    0,
                    Rect {
                        x: 0.0,
                        y: 0.0,
                        width,
                        height: 20.0,
                    },
                )],
            }])
        }
    }
    let d = rhwp::parse_document(include_bytes!(
        "../fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp"
    ))
    .unwrap();
    let Control::Table(original) = &d.sections[0].paragraphs[5].controls[0] else {
        panic!()
    };
    let mut t = original.clone();
    t.page_break = TablePageBreak::RowBreak;
    t.cells[0].height = 0;
    t.cells[0].paragraphs = vec![original.cells[0].paragraphs[6].clone(); 2];
    t.cells[0].paragraphs[0].line_segs[0].vertical_pos = 1000;
    t.cells[0].paragraphs[1].line_segs[0].vertical_pos = 0;
    let p = TableContentPlan::from_ir_contents(&t, 1.0 / 75.0, &Reflow).unwrap();
    let FragmentFit::Placed(f) = p
        .start()
        .fit_in_page(
            PageArea {
                bounds: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 700.0,
                    height: 200.0,
                },
            },
            200.0,
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(f.continuation().is_complete());
    assert_eq!(f.placement().cells[0].lines.len(), 2);
}
