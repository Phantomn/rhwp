//! Normal-save ClickHere starts without local ends: replay source text, not a
//! guessed field value or the command's Direction prompt.
use rhwp::{model::control::Control, renderer::table_v2::*};
use serde_json::Value;
const INPUT: &[u8] = include_bytes!("../fixtures/issue7353/field-boundary/prefix44-saved.hwp");
const OPTIONS: &str = r#"{"dpi":96,"max_pages":100,"cell_end_policy":"omit_final_paragraph_gap"}"#;
fn nodes<'a>(n: &'a Value, kind: &str, out: &mut Vec<&'a Value>) {
    if n["node_type"].get(kind).is_some() {
        out.push(n);
    }
    for c in n["children"].as_array().unwrap() {
        nodes(c, kind, out);
    }
}
fn text(n: &Value) -> String {
    let mut runs = vec![];
    nodes(n, "TextRun", &mut runs);
    runs.iter()
        .map(|r| r["node_type"]["TextRun"]["text"].as_str().unwrap())
        .collect()
}
#[test]
fn normal_saved_open_markers_preserve_literal_lines_and_termination() {
    let d = rhwp::parse_document(INPUT).unwrap();
    let Control::Table(t) = &d.sections[0].paragraphs[43].controls[0] else {
        panic!()
    };
    let p = &t.cells[0].paragraphs[0];
    assert_eq!(p.controls.len(), 2);
    assert!(p.field_ranges.is_empty());
    assert_eq!(p.char_offsets[0], 16);
    let mut s = DocumentV2Session::from_bytes(INPUT, OPTIONS).unwrap();
    let mut pages = vec![];
    while let Some(p) = s.next_page_json().unwrap() {
        pages.push(serde_json::from_str::<Value>(&p).unwrap());
    }
    assert_eq!(pages.len(), 12); // independent Hancom PDF
    assert!(s.next_page_json().unwrap().is_none());
    let roots: Vec<_> = pages.iter().map(|p| &p["render_tree"]["root"]).collect();
    let all = roots
        .iter()
        .flat_map(|r| {
            let mut tables = vec![];
            nodes(r, "Table", &mut tables);
            tables
                .into_iter()
                .filter(|t| t["node_type"]["Table"]["para_index"] == 43)
                .map(text)
        })
        .collect::<String>();
    assert!(!all.contains("선택하게 된 상세 근거 제시"));
    // Every literal character in the affected cell survives, exactly once.
    let expected = t.cells[0]
        .paragraphs
        .iter()
        .map(|p| p.text.as_str())
        .collect::<String>();
    assert_eq!(all, expected);
    // Saved first five lines remain the source partitions, not font reflow.
    let mut lines = vec![];
    nodes(roots[10], "TextLine", &mut lines);
    let start = lines
        .iter()
        .position(|l| text(l).starts_with("ㅇ 「노후계획도시정비법」은"))
        .unwrap();
    let chars: Vec<_> = p.text.chars().collect();
    for (i, row) in p.line_segs.iter().enumerate() {
        let lo = p
            .char_offsets
            .partition_point(|o| *o < p.line_seg_text_start(i));
        let hi = if i + 1 < p.line_segs.len() {
            p.char_offsets
                .partition_point(|o| *o < p.line_seg_text_start(i + 1))
        } else {
            chars.len()
        };
        assert_eq!(
            text(lines[start + i]),
            chars[lo..hi].iter().collect::<String>()
        );
        assert!(
            (lines[start + i]["bbox"]["height"].as_f64().unwrap()
                - f64::from(row.line_height) / 75.)
                .abs()
                < 1e-8
        );
    }
    // Source page11 host vpos34883; top margin141; cell inset141. The
    // independent PDF starts this same paragraph on11, not a fresh page12.
    let near = |v: &Value, expected: f64| {
        assert!(
            (v.as_f64().unwrap() - expected).abs() < 1e-8,
            "{v} != {expected}"
        )
    };
    let body_top = 75.6;
    near(
        &lines[start]["bbox"]["y"],
        body_top + (34883. + 141. + 141.) / 75.,
    );
    let mut fragments = vec![];
    for root in &roots {
        let mut tables = vec![];
        nodes(root, "Table", &mut tables);
        fragments.extend(
            tables
                .into_iter()
                .filter(|t| t["node_type"]["Table"]["para_index"] == 43),
        );
    }
    assert_eq!(fragments.len(), 2);
    // The saved reset occurs inside paragraph8 after its first line, not at
    // the end of a paragraph or by overlapping line-box bottoms.
    let cut = &t.cells[0].paragraphs[8].line_segs[0];
    assert_eq!(t.cells[0].paragraphs[8].line_segs[1].vertical_pos, 0);
    near(
        &fragments[0]["bbox"]["height"],
        f64::from(cut.vertical_pos + cut.line_height + 282) / 75.,
    );
    let mut continuation = vec![];
    nodes(fragments[1], "TextLine", &mut continuation);
    assert_eq!(continuation.len(), 9);
    assert!(text(continuation[0]).starts_with("전까지의 과도기적 임시단체"));
    near(
        &continuation[0]["bbox"]["y"],
        body_top + (141. + 141.) / 75.,
    );
    let last = t.cells[0]
        .paragraphs
        .last()
        .unwrap()
        .line_segs
        .last()
        .unwrap();
    near(
        &fragments[1]["bbox"]["height"],
        f64::from(last.vertical_pos + last.line_height + 282) / 75.,
    );
}

#[test]
fn open_marker_admission_rejects_unknown_slots_guides_and_edited_partitions() {
    for case in [
        "offset", "count", "dirty", "guide", "residue", "formula", "end",
    ] {
        let mut d = rhwp::parse_document(INPUT).unwrap();
        let Control::Table(t) = &mut d.sections[0].paragraphs[43].controls[0] else {
            panic!()
        };
        let p = &mut t.cells[0].paragraphs[0];
        match case {
            "offset" => p.char_offsets[0] -= 8,
            "count" => p.char_count += 8,
            "dirty" => p.stored_text_partition_dirty = true,
            "end" => p.orphan_field_ends.push(Default::default()),
            "guide" => {
                let Control::Field(f) = &mut p.controls[1] else {
                    panic!()
                };
                f.command = format!(
                    "Clickhere:set:80:Direction:wstring:{}:{} HelpState:wstring:0:  ",
                    p.text.chars().count(),
                    p.text
                );
            }
            "residue" => {
                let Control::Field(f) = &mut p.controls[1] else {
                    panic!()
                };
                f.command.clear();
            }
            "formula" => {
                let Control::Field(f) = &mut p.controls[0] else {
                    panic!()
                };
                f.field_type = rhwp::model::control::FieldType::Formula;
            }
            _ => unreachable!(),
        }
        // Direct IR path preserves deliberately invalid input (no serializer repair).
        let result = TablePreviewSession::from_document_with_end_policy(
            &d,
            TableSelection {
                section: 0,
                paragraph: 43,
                control: 0,
            },
            96.,
            TablePreviewPages {
                width: 794.,
                height: 1123.,
                body: Rect {
                    x: 20.,
                    y: 30.,
                    width: 750.,
                    height: 1000.,
                },
                first_y: 30.,
            },
            100,
            CellEndPolicy::OmitFinalParagraphGap,
        );
        assert!(
            matches!(
                result,
                Err(TablePreviewError::Geometry(GeometryError::Unsupported(
                    "unqualified stored field result"
                )))
            ),
            "{case}"
        );
    }
}

#[test]
fn rowbreak_resumes_proven_saved_frame_without_reclassifying_fresh_rows() {
    fn plan(policy: SplitPolicy, saved: bool) -> TableContentPlan {
        let line = |paragraph, height| FlowBlock::Lines {
            height,
            advance: height,
            lines: vec![LineBox {
                owner: LineOwner { paragraph, line: 0 },
                bounds: Rect {
                    x: 0.,
                    y: 0.,
                    width: 80.,
                    height,
                },
            }],
        };
        let mut blocks = vec![line(0, 20.)];
        if saved {
            blocks.push(FlowBlock::StoredFrameStart);
        }
        blocks.push(line(1, 30.));
        TableContentPlan::from_flow_rows(
            vec![100.],
            vec![FlowRowInput {
                cells: vec![FlowCellInput {
                    width: 100.,
                    padding: Insets {
                        top: 5.,
                        bottom: 3.,
                        ..Insets::default()
                    },
                    minimum_height: 0.,
                    blocks,
                }],
            }],
            0.,
            policy,
        )
        .unwrap()
    }
    let area = |height| PageArea {
        bounds: Rect {
            x: 10.,
            y: 20.,
            width: 100.,
            height,
        },
    };
    for policy in [SplitPolicy::Never, SplitPolicy::BetweenRows] {
        let FragmentFit::Placed(intact) = plan(policy, true)
            .start()
            .fit_in_page(area(100.), 100.)
            .unwrap()
        else {
            panic!()
        };
        assert!(intact.continuation().is_complete());
        assert_eq!(intact.reserved_height(), 58.);
    }
    for (policy, saved) in [
        (SplitPolicy::Never, true),
        (SplitPolicy::BetweenRows, false),
    ] {
        assert!(matches!(
            plan(policy, saved)
                .start()
                .fit_in_page(area(28.), 100.)
                .unwrap(),
            FragmentFit::DoesNotFit { .. }
        ));
    }
    let start = plan(SplitPolicy::BetweenRows, true).start();
    // The source-frame inset+line+bottom is indivisible: padding alone is not progress.
    assert!(matches!(
        start.fit_in_page(area(27.), 100.).unwrap(),
        FragmentFit::DoesNotFit { .. }
    ));
    let FragmentFit::Placed(first) = start.fit_in_page(area(28.), 100.).unwrap() else {
        panic!()
    };
    assert_eq!(first.reserved_height(), 28.);
    assert_eq!(first.placement().cells[0].lines.len(), 1);
    assert_eq!(first.placement().cells[0].lines[0].bounds.y, 25.);
    let next = first.continuation();
    assert!(matches!(
        next.fit_in_page(area(34.), 100.).unwrap(),
        FragmentFit::DoesNotFit { .. }
    ));
    let FragmentFit::Placed(last) = next.fit_in_page(area(38.), 100.).unwrap() else {
        panic!()
    };
    assert_eq!(last.reserved_height(), 38.);
    assert_eq!(last.placement().cells[0].lines.len(), 1);
    assert_eq!(last.placement().cells[0].lines[0].owner.paragraph, 1);
    assert_eq!(last.placement().cells[0].lines[0].bounds.y, 25.);
    assert!(last.continuation().is_complete());
    assert!(matches!(
        last.continuation().fit_in_page(area(100.), 100.).unwrap(),
        FragmentFit::Complete
    ));
}
