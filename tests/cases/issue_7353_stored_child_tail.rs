//! Normal Hancom whole-row and internal line cuts with physical fragment bands.
//! The child owns the cut; its follower starts after the final row on the same
//! page, not after another parent cut. Alignment consumes the accepted fragment.
use rhwp::renderer::table_v2::DocumentV2Session;
use serde_json::Value;

const INPUT: &[u8] = include_bytes!("../fixtures/issue7353/child-frame-tail/whole-cell-saved.hwp");

fn nodes<'a>(n: &'a Value, kind: &str) -> Vec<&'a Value> {
    let mut out = Vec::new();
    if n["node_type"].get(kind).is_some() {
        out.push(n);
    }
    for c in n["children"].as_array().unwrap() {
        out.extend(nodes(c, kind));
    }
    out
}
fn text(n: &Value) -> String {
    nodes(n, "TextRun")
        .into_iter()
        .map(|n| n["node_type"]["TextRun"]["text"].as_str().unwrap())
        .collect()
}
fn value(n: &Value, k: &str) -> f64 {
    n["bbox"][k].as_f64().unwrap()
}

#[test]
fn original_document_preserves_saved_frame_rows_child_units_and_termination() {
    use rhwp::model::control::Control;
    let input = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/samples/86712_regulatory_analysis.hwp"
    ))
    .unwrap();
    let doc = rhwp::parse_document(&input).unwrap();
    let Control::Table(parent) = &doc.sections[0].paragraphs[172].controls[0] else {
        panic!()
    };
    let p = &parent.cells[77].paragraphs[0];
    // Original file, not the current Hancom re-save: the third saved line
    // restarts at zero. Re-saving moves that reset to the fourth line.
    assert_eq!(
        p.line_segs
            .iter()
            .map(|l| l.vertical_pos)
            .collect::<Vec<_>>(),
        [0, 1560, 0, 1560]
    );
    let chars: Vec<_> = p.text.chars().collect();
    let expected: Vec<String> = (0..p.line_segs.len())
        .map(|i| {
            let start = p
                .char_offsets
                .partition_point(|&v| v < p.line_seg_text_start(i));
            let end = if i + 1 == p.line_segs.len() {
                chars.len()
            } else {
                p.char_offsets
                    .partition_point(|&v| v < p.line_seg_text_start(i + 1))
            };
            chars[start..end].iter().collect()
        })
        .collect();
    let Control::Table(middle) = &parent.cells[80].paragraphs[0].controls[1] else {
        panic!()
    };
    let Control::Table(child) = &middle.cells[0].paragraphs[20].controls[0] else {
        panic!()
    };
    // The original's saved first child extent is only its header row, unlike
    // the independently re-saved document. Do not substitute the latter's PDF.
    assert_eq!(child.common.height, 1463);
    let mut session = DocumentV2Session::from_bytes(
        &input,
        r#"{"dpi":96,"max_pages":100,"cell_end_policy":"omit_final_paragraph_gap"}"#,
    )
    .unwrap();
    let mut pages = Vec::new();
    while let Some(raw) = session.next_page_json().unwrap() {
        pages.push(serde_json::from_str::<Value>(&raw).unwrap());
        assert!(pages.len() < 100, "must finish, not stop at the safety cap");
    }
    assert!(session.next_page_json().unwrap().is_none());
    let mut lines = Vec::new();
    let mut fragments = Vec::new();
    let mut followers = Vec::new();
    for (page, output) in pages.iter().enumerate() {
        let root = &output["render_tree"]["root"];
        let body = nodes(root, "Body")[0];
        for table in nodes(root, "Table")
            .into_iter()
            .filter(|n| n["node_type"]["Table"]["para_index"] == 172)
        {
            for cell in table["children"].as_array().unwrap().iter().filter(|n| {
                n["node_type"]["TableCell"]["row"] == parent.cells[77].row
                    && n["node_type"]["TableCell"]["col"] == parent.cells[77].col
            }) {
                lines.extend(nodes(cell, "TextLine").into_iter().map(|l| (page, text(l))));
            }
            followers.extend(
                nodes(table, "TextLine")
                    .into_iter()
                    .filter(|l| text(l).contains("자료출처"))
                    .map(|l| (page, l)),
            );
            for nested in nodes(table, "Table") {
                assert!(value(nested, "y") >= value(body, "y") - 1e-7);
                assert!(
                    value(nested, "y") + value(nested, "height")
                        <= value(body, "y") + value(body, "height") + 1e-7
                );
                if nested["node_type"]["Table"]["col_count"] == 12 {
                    fragments.push((page, nested));
                }
            }
        }
    }
    let matches: Vec<_> = lines
        .windows(4)
        .filter(|w| w.iter().map(|(_, s)| s).eq(expected.iter()))
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "saved lines must survive exactly once in order"
    );
    let rows = matches[0];
    assert_eq!(rows[0].0, rows[1].0);
    assert_eq!(rows[2].0, rows[3].0);
    assert_eq!(
        rows[2].0,
        rows[0].0 + 1,
        "preserve original frame ownership"
    );
    assert_eq!(fragments.len(), 2);
    assert_eq!(fragments[1].0, fragments[0].0 + 1);
    assert!((value(fragments[0].1, "height") - 1463.0 / 75.0).abs() < 1e-7);
    let cells: Vec<_> = fragments
        .iter()
        .flat_map(|(_, t)| nodes(t, "TableCell"))
        .collect();
    assert_eq!(cells.len(), child.cells.len());
    for source in &child.cells {
        let actual: Vec<_> = cells
            .iter()
            .filter(|n| {
                n["node_type"]["TableCell"]["row"] == source.row
                    && n["node_type"]["TableCell"]["col"] == source.col
            })
            .collect();
        assert_eq!(actual.len(), 1, "no missing or repeated child cell");
        assert_eq!(
            text(actual[0]),
            source
                .paragraphs
                .iter()
                .map(|p| p.text.as_str())
                .collect::<String>()
        );
    }
    assert!(nodes(fragments[0].1, "TableCell")
        .iter()
        .all(|n| n["node_type"]["TableCell"]["row"] == 0));
    assert_eq!(followers.len(), 1);
    assert_eq!(followers[0].0, fragments[1].0);
    assert!(
        value(followers[0].1, "y") >= value(fragments[1].1, "y") + value(fragments[1].1, "height")
    );
}

#[test]
fn saved_child_cut_preserves_rows_follower_and_final_table() {
    check_saved(INPUT, 6330.0);
}

#[test]
fn line_split_permission_preserves_the_same_saved_whole_row_cut() {
    check_saved(
        include_bytes!("../fixtures/issue7353/child-frame-tail/whole-row-saved.hwp"),
        6330.0,
    );
}

fn check_saved(input: &[u8], first_height: f64) {
    use rhwp::model::control::Control;
    let doc = rhwp::parse_document(input).unwrap();
    let Control::Table(parent) = &doc.sections[0].paragraphs[1].controls[0] else {
        panic!()
    };
    let Control::Table(middle) = &parent.cells[80].paragraphs[0].controls[1] else {
        panic!()
    };
    let Control::Table(source) = &middle.cells[0].paragraphs[20].controls[0] else {
        panic!()
    };
    let mut s = DocumentV2Session::from_bytes(
        input,
        r#"{"dpi":96,"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}"#,
    )
    .unwrap();
    let mut pages = Vec::new();
    while let Some(raw) = s.next_page_json().unwrap() {
        pages.push(serde_json::from_str::<Value>(&raw).unwrap());
    }
    // whole-cell-2024.pdf: two whole rows on p2, final row and source on p3.
    assert_eq!(pages.len(), 3);
    assert!(s.next_page_json().unwrap().is_none());
    for (page, expected_rows) in [(1, vec![0, 1]), (2, vec![2])] {
        let root = &pages[page]["render_tree"]["root"];
        let children: Vec<_> = nodes(root, "Table")
            .into_iter()
            .filter(|n| n["node_type"]["Table"]["col_count"] == 12)
            .collect();
        assert_eq!(children.len(), 1);
        let child = children[0];
        let cells = nodes(child, "TableCell");
        assert_eq!(cells.len(), expected_rows.len() * 12);
        for cell in &cells {
            let row = cell["node_type"]["TableCell"]["row"].as_u64().unwrap() as u16;
            let column = cell["node_type"]["TableCell"]["col"].as_u64().unwrap() as u16;
            let original = source
                .cells
                .iter()
                .find(|c| c.row == row && c.col == column)
                .unwrap();
            let expected: String = original
                .paragraphs
                .iter()
                .map(|p| p.text.as_str())
                .collect();
            assert_eq!(
                text(cell),
                expected,
                "row={row} column={column}: exact content once"
            );
            for line in nodes(cell, "TextLine") {
                assert!(value(line, "y") >= value(cell, "y") - 1e-7);
                assert!(
                    value(line, "y") + value(line, "height")
                        <= value(cell, "y") + value(cell, "height") + 1e-7
                );
            }
        }
        let actual: std::collections::BTreeSet<_> = cells
            .iter()
            .map(|c| c["node_type"]["TableCell"]["row"].as_u64().unwrap())
            .collect();
        assert_eq!(actual, expected_rows.into_iter().collect());
        let body = nodes(root, "Body")[0];
        for table in nodes(root, "Table") {
            assert!(
                value(table, "y") + value(table, "height")
                    <= value(body, "y") + value(body, "height") + 1e-7
            );
        }
        if page == 1 {
            // Independent normal saved first-fragment extent, not a page-count
            // target. The second row must center its whole lines in THAT band.
            assert!((value(child, "height") - first_height / 75.0).abs() < 1e-7);
            for cell in cells
                .iter()
                .filter(|c| c["node_type"]["TableCell"]["row"] == 1)
            {
                let lines = nodes(cell, "TextLine");
                if lines.is_empty() {
                    continue;
                }
                let first = lines
                    .iter()
                    .map(|l| value(l, "y"))
                    .fold(f64::INFINITY, f64::min);
                let last = lines
                    .iter()
                    .map(|l| value(l, "y") + value(l, "height"))
                    .fold(f64::NEG_INFINITY, f64::max);
                assert!(
                    (first + last - 2.0 * value(cell, "y") - value(cell, "height")).abs() < 1e-7,
                    "CENTER must use accepted physical height"
                );
            }
            assert!(!text(root).contains("자료출처"));
        } else {
            let following: Vec<_> = nodes(root, "TextLine")
                .into_iter()
                .filter(|n| text(n).contains("자료출처"))
                .collect();
            assert_eq!(following.len(), 1);
            // Independent saved final row: 3x1000HU + 2x300HU gaps + 2x141HU
            // padding =3882HU; then 283HU outer bottom and the following
            // paragraph's independently authored 100HU before spacing.
            assert!((value(child, "height") - 3882.0 / 75.0).abs() < 1e-7);
            assert!(
                (value(following[0], "y") - value(child, "y") - (3882.0 + 283.0 + 100.0) / 75.0)
                    .abs()
                    < 1e-7
            );
            let after: Vec<_> = nodes(root, "Table")
                .into_iter()
                .filter(|n| {
                    n["node_type"]["Table"]["col_count"] == 4
                        && n["node_type"]["Table"]["row_count"] == 5
                })
                .collect();
            assert_eq!(after.len(), 1);
            assert!(
                value(after[0], "y") >= value(following[0], "y") + value(following[0], "height")
            );
        }
    }
}

#[test]
fn shorter_saved_end_row_preserves_content_and_following_frame() {
    let input = include_bytes!("../fixtures/issue7353/child-frame-tail/carrier-saved.hwp");
    // Normal saved first fragment5781HU: first row1565HU and the page-end
    // physical band4216HU. The PDF independently shows the shorter second row.
    // The cell's cached4765HU is not an extra549HU continuation after its text.
    check_saved(input, 5781.0);
}

#[test]
fn contradictory_saved_bands_do_not_clip_content_or_invent_a_cut() {
    use rhwp::model::control::Control;
    let input = include_bytes!("../fixtures/issue7353/child-frame-tail/carrier-saved.hwp");
    let source = rhwp::parse_document(input).unwrap();
    // Serialization is an adversarial-input tool here, not a Hancom oracle.
    // First prove the unchanged round trip still exercises the positive path.
    check_saved(
        &rhwp::serializer::cfb_writer::serialize_hwp(&source).unwrap(),
        5781.0,
    );
    for (first_height, tail_delta) in [(2565, 0), (7000, 0), (5781, 1)] {
        let mut doc = source.clone();
        let Control::Table(parent) = &mut doc.sections[0].paragraphs[1].controls[0] else {
            panic!()
        };
        let Control::Table(middle) = &mut parent.cells[80].paragraphs[0].controls[1] else {
            panic!()
        };
        let Control::Table(child) = &mut middle.cells[0].paragraphs[20].controls[0] else {
            panic!()
        };
        // 2565 leaves1000HU after row0, below row1's real line occupancy.
        // 7000 exceeds the intact rows. Neither is a qualified end-row band.
        child.common.height = first_height;
        // A plausible first band alone cannot establish source ownership.
        // Keep it intact but contradict the following frame's origin as well.
        middle.cells[0].paragraphs[21].line_segs[0].vertical_pos += tail_delta;
        let bytes = rhwp::serializer::cfb_writer::serialize_hwp(&doc).unwrap();
        let error = DocumentV2Session::from_bytes(
            &bytes,
            r#"{"dpi":96,"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}"#,
        )
        .err()
        .expect("contradictory geometry must remain unsupported");
        assert!(
            error
                .to_string()
                .contains("unqualified stored child frame geometry"),
            "{error}"
        );
    }
}

#[test]
fn internal_line_cut_preserves_content_alignment_and_follower() {
    let input = include_bytes!("../fixtures/issue7353/child-frame-tail/line-split-saved.hwp");
    check_internal(input);
}

#[test]
fn resumed_parent_and_child_insets_match_the_independent_page_two_origin() {
    let input = include_bytes!("../fixtures/issue7353/child-frame-tail/line-split-saved.hwp");
    let mut session = DocumentV2Session::from_bytes(
        input,
        r#"{"dpi":96,"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}"#,
    )
    .unwrap();
    let first_page: Value =
        serde_json::from_str(&session.next_page_json().unwrap().unwrap()).unwrap();
    let first_root = &first_page["render_tree"]["root"];
    let first_tables = nodes(first_root, "Table");
    let parent = first_tables
        .iter()
        .find(|n| n["node_type"]["Table"]["row_count"] == 28)
        .unwrap();
    let child = first_tables
        .iter()
        .find(|n| n["node_type"]["Table"]["row_count"] == 1)
        .unwrap();
    // The same source inset contract also closes the preceding physical frame.
    // This checks containment/one inset, not exact PDF border equivalence.
    let parent_end = value(parent, "y") + value(parent, "height");
    let child_end = value(child, "y") + value(child, "height");
    assert!((parent_end - child_end - 223.0 / 75.0).abs() < 1e-7);
    let body = nodes(first_root, "Body")[0];
    assert!(parent_end <= value(body, "y") + value(body, "height") + 1e-7);
    assert!(text(child).contains("편익 수혜자"));
    assert!(!text(child).contains("편익의 종류"));
    let page: Value = serde_json::from_str(&session.next_page_json().unwrap().unwrap()).unwrap();
    let root = &page["render_tree"]["root"];
    let tables = nodes(root, "Table");
    let outer = tables
        .iter()
        .find(|n| n["node_type"]["Table"]["row_count"] == 28)
        .unwrap();
    let inner = tables
        .iter()
        .find(|n| n["node_type"]["Table"]["row_count"] == 1)
        .unwrap();
    // Source table-wide top223HU is active (cell override disabled).
    assert!((value(inner, "y") - value(outer, "y") - 223.0 / 75.0).abs() < 1e-7);
    let lines = nodes(inner, "TextLine");
    let first = lines
        .iter()
        .find(|n| text(n).contains("편익의 종류"))
        .unwrap();
    // Inner top141HU plus authored paragraph-before100HU, not last page's gap.
    assert!((value(first, "y") - value(inner, "y") - 241.0 / 75.0).abs() < 1e-7);
    let run = nodes(first, "TextRun")[0];
    let baseline = value(run, "y") + run["node_type"]["TextRun"]["baseline"].as_f64().unwrap();
    // Independent Hancom PDF glyph transform .12pt * y608, converted to96dpi.
    // 0.2px permits PDF coordinate rounding, not a fitted translation.
    assert!((baseline - 97.28).abs() < 0.2, "actual baseline={baseline}");
}

#[test]
fn equal_positive_origins_require_forward_paragraph_flow_to_start_a_frame() {
    use rhwp::{
        model::control::Control,
        renderer::{style_resolver::resolve_styles, table_v2::*},
    };
    let doc = rhwp::parse_document(include_bytes!(
        "../fixtures/issue7353/child-frame-tail/line-split-saved.hwp"
    ))
    .unwrap();
    let Control::Table(parent) = &doc.sections[0].paragraphs[1].controls[0] else {
        panic!()
    };
    let Control::Table(original) = &parent.cells[80].paragraphs[0].controls[1] else {
        panic!()
    };
    for (origin, spacing, expected) in [(100, 600, 1), (100, -1200, 2), (0, 600, 2), (200, 600, 2)]
    {
        // Synthetic API boundary controls, not independently saved PDF oracles.
        // -1200 cancels the1200HU line advance: intentional overlap cannot
        // prove a frame reset. Zero/local origins likewise are not cuts.
        let mut t = original.clone();
        t.cells[0].height = 0;
        t.cells[0].paragraphs.truncate(2);
        for p in &mut t.cells[0].paragraphs {
            p.line_segs[0].vertical_pos = origin;
        }
        t.cells[0].paragraphs[0].line_segs[0].line_spacing = spacing;
        let p = PreparedTextTable::prepare_with_end_policy(
            &t,
            &resolve_styles(&doc.doc_info, 96.0),
            96.0,
            &[],
            CellEndPolicy::OmitFinalParagraphGap,
        )
        .unwrap();
        let TextFragmentFit::Placed(f) = p
            .start()
            .fit(PageArea {
                bounds: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 700.0,
                    height: 1000.0,
                },
            })
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(
            f.geometry().placement().cells[0].lines.len(),
            expected,
            "origin={origin} spacing={spacing}"
        );
        assert_eq!(f.geometry().continuation().is_complete(), expected == 2);
    }
}

#[test]
fn contradictory_internal_frames_are_rejected_without_hiding_lines() {
    use rhwp::model::{control::Control, table::TablePageBreak};
    let input = include_bytes!("../fixtures/issue7353/child-frame-tail/line-split-saved.hwp");
    let source = rhwp::parse_document(input).unwrap();
    // Adversarial serialization, not an independently authored Hancom oracle.
    check_internal(&rhwp::serializer::cfb_writer::serialize_hwp(&source).unwrap());
    for (first, tail_delta, whole_cells) in [
        (6330 + 1500, 0, false), // smaller than the two source lines plus padding
        (6330 + 4000, 0, false), // larger than the intact cached row
        (9681, 1, false),        // unexplained extra physical tail
        (9681, 0, true),         // source cut contradicts whole-cell policy
    ] {
        let mut doc = source.clone();
        let Control::Table(parent) = &mut doc.sections[0].paragraphs[1].controls[0] else {
            panic!()
        };
        let Control::Table(middle) = &mut parent.cells[80].paragraphs[0].controls[1] else {
            panic!()
        };
        let Control::Table(child) = &mut middle.cells[0].paragraphs[20].controls[0] else {
            panic!()
        };
        child.common.height = first;
        if whole_cells {
            child.page_break = TablePageBreak::CellBreak;
        }
        middle.cells[0].paragraphs[21].line_segs[0].vertical_pos += tail_delta;
        let bytes = rhwp::serializer::cfb_writer::serialize_hwp(&doc).unwrap();
        let error = DocumentV2Session::from_bytes(
            &bytes,
            r#"{"dpi":96,"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}"#,
        )
        .err()
        .expect("contradictory source frames must not silently clip or fall back");
        assert!(
            error.to_string().contains("unqualified stored child"),
            "{error}"
        );
    }
}

#[test]
fn internal_frame_budget_rejection_does_not_consume_its_source_units() {
    use rhwp::{
        model::control::Control,
        renderer::{style_resolver::resolve_styles, table_v2::*},
    };
    let doc = rhwp::parse_document(include_bytes!(
        "../fixtures/issue7353/child-frame-tail/line-split-saved.hwp"
    ))
    .unwrap();
    let Control::Table(parent) = &doc.sections[0].paragraphs[1].controls[0] else {
        panic!()
    };
    let Control::Table(middle) = &parent.cells[80].paragraphs[0].controls[1] else {
        panic!()
    };
    let prepared = PreparedTextTable::prepare_with_end_policy(
        middle,
        &resolve_styles(&doc.doc_info, 96.0),
        96.0,
        &doc.bin_data_content,
        CellEndPolicy::OmitFinalParagraphGap,
    )
    .unwrap();
    let area = |height| PageArea {
        bounds: Rect {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height,
        },
    };
    // Caller-owned geometry stress, NOT a Hancom PDF pagination oracle.
    // This budget leaves the child row's first frame for the next fit.
    let TextFragmentFit::Placed(prefix) = prepared.start().fit(area(900.0)).unwrap() else {
        panic!()
    };
    // The source p0/p1 same-positive-origin transition owns an earlier frame.
    // Resume it, then stop before the target child's internal first fragment.
    let TextFragmentFit::Placed(prefix) = prefix.continuation().fit(area(900.0)).unwrap() else {
        panic!()
    };
    let cursor = prefix.continuation();
    // Saved first row fragment3351 + child outer margins283*2 + the
    // enclosing cell's terminal bottom141 reserved for the descendant cut.
    let required = (3351.0 + 283.0 * 2.0 + 141.0) / 75.0;
    let TextFragmentFit::Placed(reference) = cursor.fit(area(60.0)).unwrap() else {
        panic!()
    };
    for height in [45.0, 50.0, 54.0] {
        // The two lines + padding fit (2582HU), but the physical3351HU band
        // does not. Do not accept the smaller content height and stretch later.
        let TextFragmentFit::DoesNotFit {
            required_height, ..
        } = cursor.fit(area(height)).unwrap()
        else {
            panic!()
        };
        assert!((required_height - required).abs() < 1e-8);
        let TextFragmentFit::Placed(retry) = cursor.fit(area(required + 1e-8)).unwrap() else {
            panic!()
        };
        assert_eq!(
            retry.geometry().placement(),
            reference.geometry().placement()
        );
        assert!((retry.geometry().reserved_height() - required).abs() < 1e-8);
        let child = &retry.geometry().placement().cells[0].tables[0].placement;
        assert_eq!(child.cells.len(), 12);
        assert!(child.cells.iter().all(|c| c.row == 2));
        assert_eq!(child.cells[0].lines.len(), 2);
        assert!(child.cells[1..].iter().all(|c| c.lines.len() == 1));
        let TextFragmentFit::Placed(tail) = retry.continuation().fit(area(1000.0)).unwrap() else {
            panic!()
        };
        let tail_child = &tail.geometry().placement().cells[0].tables[0].placement;
        assert_eq!(tail_child.cells[0].lines.len(), 1);
        assert!(tail_child.cells[1..].iter().all(|c| c.lines.is_empty()));
        assert!(matches!(
            tail.continuation().fit(area(1000.0)).unwrap(),
            TextFragmentFit::Complete
        ));
    }
}

fn check_internal(input: &[u8]) {
    let mut session = DocumentV2Session::from_bytes(
        input,
        r#"{"dpi":96,"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}"#,
    )
    .unwrap();
    let mut pages = Vec::new();
    while let Some(raw) = session.next_page_json().unwrap() {
        pages.push(serde_json::from_str::<Value>(&raw).unwrap());
    }
    assert_eq!(pages.len(), 3);
    assert!(session.next_page_json().unwrap().is_none());
    use rhwp::model::control::Control;
    let doc = rhwp::parse_document(input).unwrap();
    let Control::Table(parent) = &doc.sections[0].paragraphs[1].controls[0] else {
        panic!()
    };
    let Control::Table(middle) = &parent.cells[80].paragraphs[0].controls[1] else {
        panic!()
    };
    let Control::Table(source) = &middle.cells[0].paragraphs[20].controls[0] else {
        panic!()
    };
    let mut contents = std::collections::BTreeMap::<(u16, u16), String>::new();
    for (page, height) in [(1, 9681.0), (2, 1282.0)] {
        let root = &pages[page]["render_tree"]["root"];
        let tables: Vec<_> = nodes(root, "Table")
            .into_iter()
            .filter(|n| n["node_type"]["Table"]["col_count"] == 12)
            .collect();
        assert_eq!(tables.len(), 1);
        let table = tables[0];
        assert!((value(table, "height") - height / 75.0).abs() < 1e-7);
        let cells = nodes(table, "TableCell");
        assert_eq!(cells.len(), if page == 1 { 36 } else { 12 });
        for cell in cells {
            let row = cell["node_type"]["TableCell"]["row"].as_u64().unwrap() as u16;
            let col = cell["node_type"]["TableCell"]["col"].as_u64().unwrap() as u16;
            let content = text(cell);
            contents.entry((row, col)).or_default().push_str(&content);
            if row == 2 {
                let band = if page == 1 { 3351.0 } else { 1282.0 };
                assert!((value(cell, "height") - band / 75.0).abs() < 1e-7);
                if col == 0 {
                    assert_eq!(content, if page == 1 { "년간상승률" } else { "(%)" });
                } else if page == 2 {
                    assert!(content.is_empty(), "numeric companion must not repeat");
                }
            }
            let lines = nodes(cell, "TextLine");
            for line in &lines {
                assert!(value(line, "y") >= value(cell, "y") - 1e-7);
                assert!(
                    value(line, "y") + value(line, "height")
                        <= value(cell, "y") + value(cell, "height") + 1e-7
                );
            }
            if row == 2 && !lines.is_empty() {
                let first = lines
                    .iter()
                    .map(|l| value(l, "y"))
                    .fold(f64::INFINITY, f64::min);
                let last = lines
                    .iter()
                    .map(|l| value(l, "y") + value(l, "height"))
                    .fold(f64::NEG_INFINITY, f64::max);
                assert!(
                    (first + last - 2.0 * value(cell, "y") - value(cell, "height")).abs() < 1e-7,
                    "fragment CENTER"
                );
            }
        }
        let body = nodes(root, "Body")[0];
        for t in nodes(root, "Table") {
            assert!(
                value(t, "y") + value(t, "height")
                    <= value(body, "y") + value(body, "height") + 1e-7
            );
        }
        if page == 1 {
            assert!(!text(root).contains("자료출처"));
        } else {
            let followers: Vec<_> = nodes(root, "TextLine")
                .into_iter()
                .filter(|n| text(n).contains("자료출처"))
                .collect();
            assert_eq!(followers.len(), 1);
            // Final line1000 + twice141HU padding,283HU outer bottom,100HU before.
            assert!(
                (value(followers[0], "y") - value(table, "y") - (1282.0 + 283.0 + 100.0) / 75.0)
                    .abs()
                    < 1e-7
            );
            let after: Vec<_> = nodes(root, "Table")
                .into_iter()
                .filter(|n| {
                    n["node_type"]["Table"]["col_count"] == 4
                        && n["node_type"]["Table"]["row_count"] == 5
                })
                .collect();
            assert_eq!(after.len(), 1);
            assert!(
                value(after[0], "y") >= value(followers[0], "y") + value(followers[0], "height")
            );
        }
    }
    for c in &source.cells {
        assert_eq!(
            contents.remove(&(c.row, c.col)).unwrap(),
            c.paragraphs
                .iter()
                .map(|p| p.text.as_str())
                .collect::<String>()
        );
    }
    assert!(contents.is_empty());
}
