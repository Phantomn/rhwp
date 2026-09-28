//! Normally saved source/PDF: a local single-column declaration owns a floating
//! child at control1. Geometry mutations below are separate boundary contracts.
use rhwp::{
    model::{control::Control, document::Document, paragraph::Paragraph},
    renderer::table_v2::*,
};
use serde_json::Value;
use std::sync::Arc;

const INPUT: &[u8] =
    include_bytes!("../fixtures/issue7353/local-column-anchor/prefix163-saved.hwp");
const OPTIONS: &str = r#"{"dpi":96,"max_pages":100,"cell_end_policy":"omit_final_paragraph_gap"}"#;
fn render(bytes: &[u8]) -> Vec<Value> {
    let mut session = DocumentV2Session::from_bytes(bytes, OPTIONS).unwrap();
    let mut out = vec![];
    while let Some(page) = session.next_page_json().unwrap() {
        out.push(serde_json::from_str(&page).unwrap());
    }
    assert!(session.next_page_json().unwrap().is_none());
    out
}
fn nodes<'a>(n: &'a Value, kind: &str) -> Vec<&'a Value> {
    let mut out = vec![];
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
fn b(n: &Value, key: &str) -> f64 {
    n["bbox"][key].as_f64().unwrap()
}
fn near(a: f64, expected: f64) {
    assert!((a - expected).abs() < 1e-8, "{a} != {expected}");
}
fn host(d: &mut Document) -> &mut Paragraph {
    let Control::Table(t) = &mut d.sections[0].paragraphs[161].controls[0] else {
        panic!()
    };
    &mut t.cells[13].paragraphs[0]
}

#[test]
fn normal_saved_nested_cut_preserves_both_fragments_label_and_following_blank() {
    let pages = render(INPUT);
    assert_eq!(pages.len(), 26);
    let mut child_text = String::new();
    for (pi, pdf_top, pdf_bottom) in [(24, 660.837, 73.839), (25, 782.864, 750.619)] {
        let root = &pages[pi]["render_tree"]["root"];
        let parent = nodes(root, "Table")
            .into_iter()
            .find(|n| n["node_type"]["Table"]["para_index"] == 161)
            .unwrap();
        let children = nodes(parent, "Table");
        assert_eq!(
            children.len(),
            2,
            "page{} child must remain in its parent",
            pi + 1
        );
        let child = children[1];
        child_text.push_str(&text(child));
        // Independent PDF stroke endpoints in pt, paper height841pt. The
        // printer uses .119869 rather than .12 for vertical coordinates;
        // tolerance is that measured scale difference plus one print pixel.
        for (actual, raw) in [
            (b(parent, "y"), (841. - pdf_top) * 4. / 3.),
            (
                b(parent, "y") + b(parent, "height"),
                (841. - pdf_bottom) * 4. / 3.,
            ),
        ] {
            assert!(
                (actual - raw).abs() <= raw * (1. - 0.119869 / 0.12) + 0.2,
                "page{}: {actual} vs PDF{raw}",
                pi + 1
            );
        }
        let label_cell = parent["children"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| {
                n["node_type"]["TableCell"]["row"] == 6 && n["node_type"]["TableCell"]["col"] == 0
            })
            .unwrap();
        let right = parent["children"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| {
                n["node_type"]["TableCell"]["row"] == 6 && n["node_type"]["TableCell"]["col"] == 1
            })
            .unwrap();
        let labels = nodes(label_cell, "TextLine");
        if pi == 24 {
            assert_eq!(labels.len(), 1);
            assert_eq!(text(labels[0]), "근거설명");
            near(
                b(labels[0], "y") + b(labels[0], "height") / 2.,
                b(label_cell, "y") + b(label_cell, "height") / 2.,
            );
            let blank = &right["children"][0];
            assert!(text(blank).is_empty());
            near(b(blank, "height"), 1500. / 75.);
            near(b(blank, "y"), b(right, "y") + 223. / 75.);
            near(b(child, "y"), b(blank, "y"));
            assert!(text(child).ends_with("지정 전까지 "));
            assert!(!text(child).contains("약 1년간"));
            near(
                b(parent, "y") + b(parent, "height"),
                b(child, "y") + b(child, "height") + 223. / 75.,
            );
        } else {
            assert!(labels.is_empty(), "consumed label must not replay");
            assert_eq!(
                text(child),
                "각 주민대표단별로 약 1년간 운영하는 것으로 가정"
            );
            near(b(child, "y"), b(right, "y") + 223. / 75.);
            let lines = nodes(child, "TextLine");
            near(b(lines[0], "y"), b(child, "y") + 141. / 75.);
            // PDF26 glyph baseline600 * .119869pt. Source padding independently
            // establishes the exact origin; the PDF checks the observed output.
            let baseline = b(lines[0], "y")
                + lines[0]["node_type"]["TextLine"]["baseline"]
                    .as_f64()
                    .unwrap();
            assert!((baseline - 600. * 0.119869 * 4. / 3.).abs() < 0.3);
            let following = right["children"].as_array().unwrap().last().unwrap();
            assert!(following["node_type"].get("TextLine").is_some());
            assert!(text(following).is_empty());
            near(b(following, "height"), 1300. / 75.);
            near(b(following, "y"), b(child, "y") + b(child, "height"));
        }
        for line in nodes(child, "TextLine") {
            assert!(b(line, "y") >= b(child, "y") - 1e-8);
            assert!(b(line, "y") + b(line, "height") <= b(child, "y") + b(child, "height") + 1e-8);
        }
    }
    let mut d = rhwp::parse_document(INPUT).unwrap();
    let Control::Table(child) = &host(&mut d).controls[1] else {
        panic!()
    };
    let expected: String = child.cells[0]
        .paragraphs
        .iter()
        .map(|p| p.text.as_str())
        .collect();
    assert_eq!(child_text, expected, "every source unit preserved once");
}

#[test]
fn local_column_keeps_control_slot_and_para_reference_geometry() {
    let original = render(INPUT);
    let mut d = rhwp::parse_document(INPUT).unwrap();
    assert!(matches!(host(&mut d).controls[0], Control::ColumnDef(_)));
    let Control::Table(t) = &mut host(&mut d).controls[1] else {
        panic!()
    };
    assert_eq!(t.common.horz_rel_to, rhwp::model::shape::HorzRelTo::Column);
    t.common.horz_rel_to = rhwp::model::shape::HorzRelTo::Para;
    let encoded = rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap();
    let mut roundtrip = rhwp::parse_document(&encoded).unwrap();
    let Control::Table(t) = &host(&mut roundtrip).controls[1] else {
        panic!()
    };
    assert_eq!(t.common.horz_rel_to, rhwp::model::shape::HorzRelTo::Para);
    let equivalent = render(&encoded);
    for (a, b) in original.iter().zip(equivalent) {
        assert_eq!(a["render_tree"], b["render_tree"]);
    }
}

#[test]
fn column_reference_requires_the_actual_initial_single_lane() {
    for case in 0..5 {
        let mut d = rhwp::parse_document(INPUT).unwrap();
        let p = host(&mut d);
        let Control::ColumnDef(cd) = &mut p.controls[0] else {
            panic!()
        };
        match case {
            0 => cd.column_count = 2,
            1 => cd.spacing = 100,
            2 => cd.separator_type = 1,
            3 => {
                p.controls.remove(0);
            }
            _ => {
                let Control::Table(t) = &mut d.sections[0].paragraphs[161].controls[0] else {
                    panic!()
                };
                t.cells[13].paragraphs.swap(0, 1);
            }
        }
        // Exercise typed IR directly: this is a deliberate mutation, not a
        // normally saved multi-column document. HWP raw control records can
        // preserve the original declaration during serialization.
        let Control::Table(t) = &d.sections[0].paragraphs[161].controls[0] else {
            panic!()
        };
        let styles = rhwp::renderer::style_resolver::resolve_styles(&d.doc_info, 96.);
        assert!(
            PreparedTextTable::prepare_with_end_policy(
                t,
                &styles,
                96.,
                &d.bin_data_content,
                CellEndPolicy::OmitFinalParagraphGap
            )
            .is_err(),
            "case{case}"
        );
    }
}

fn line(paragraph: usize) -> FlowBlock {
    FlowBlock::Lines {
        height: 20.,
        advance: 20.,
        lines: vec![LineBox {
            owner: LineOwner { paragraph, line: 0 },
            bounds: Rect {
                x: 0.,
                y: 0.,
                width: 80.,
                height: 20.,
            },
        }],
    }
}
fn wrapper(child: TableContentPlan, anchored: bool, minimum: f64) -> TableContentPlan {
    let plan = Arc::new(child);
    let owner = ControlOwner {
        paragraph: 0,
        control: 1,
    };
    let block = if anchored {
        FlowBlock::AnchoredTable {
            owner,
            host: Some(LineBox {
                owner: LineOwner {
                    paragraph: 0,
                    line: 0,
                },
                bounds: Rect {
                    x: 0.,
                    y: 0.,
                    width: 0.,
                    height: 10.,
                },
            }),
            host_advance: 12.,
            offset_x: 0.,
            offset_y: 0.,
            available_width: 100.,
            top: 0.,
            bottom: 0.,
            plan,
        }
    } else {
        FlowBlock::Table {
            owner,
            offset_x: 0.,
            restart_top: 0.,
            plan,
        }
    };
    TableContentPlan::from_flow_rows(
        vec![100.],
        vec![FlowRowInput {
            cells: vec![FlowCellInput {
                padding: Insets {
                    top: 3.,
                    bottom: 3.,
                    ..Default::default()
                },
                minimum_height: minimum,
                width: 100.,
                blocks: vec![block],
            }],
        }],
        0.,
        SplitPolicy::BetweenRows,
    )
    .unwrap()
}
fn child(policy: SplitPolicy) -> TableContentPlan {
    TableContentPlan::from_flow_rows(
        vec![100.],
        vec![FlowRowInput {
            cells: vec![FlowCellInput {
                padding: Insets::default(),
                minimum_height: 0.,
                width: 100.,
                blocks: vec![line(10), FlowBlock::StoredFrameStart, line(11)],
            }],
        }],
        0.,
        policy,
    )
    .unwrap()
}
fn area(height: f64) -> PageArea {
    PageArea {
        bounds: Rect {
            x: 10.,
            y: 20.,
            width: 100.,
            height,
        },
    }
}
fn owners(t: &TablePlacement) -> Vec<usize> {
    let mut out = vec![];
    for c in &t.cells {
        out.extend(
            c.lines
                .iter()
                .filter(|l| l.bounds.width > 0.)
                .map(|l| l.owner.paragraph),
        );
        for t in &c.tables {
            out.extend(owners(&t.placement));
        }
    }
    out
}

#[test]
fn descendant_cut_reserves_ancestor_insets_and_retries_without_consuming_units() {
    for anchored in [false, true] {
        for deep in [false, true] {
            let mut p = child(SplitPolicy::BetweenRows);
            if deep {
                p = wrapper(p, anchored, 0.);
            }
            let cursor = wrapper(p, anchored, 60.).start();
            // First unit20 plus each ancestor's two3px insets. A deficit cannot
            // consume the zero-width host, an inset, or a nested continuation.
            let required = if deep { 32. } else { 26. };
            assert!(matches!(
                cursor.fit(area(required - 1.)).unwrap(),
                FragmentFit::DoesNotFit { .. }
            ));
            let FragmentFit::Placed(first) = cursor.fit(area(50.)).unwrap() else {
                panic!()
            };
            assert_eq!(owners(first.placement()), [10]);
            assert_eq!(first.placement().cells[0].tables[0].owner.control, 1);
            assert_eq!(first.reserved_height(), 50.);
            assert!(!first.continuation().is_complete());
            let FragmentFit::Placed(second) = first.continuation().fit(area(50.)).unwrap() else {
                panic!()
            };
            assert_eq!(owners(second.placement()), [11]);
            assert_eq!(second.reserved_height(), required);
            assert!(second.continuation().is_complete());
            // An intact query ignores saved pagination, preserving both lines.
            let FragmentFit::Placed(intact) = cursor.fit(area(100.)).unwrap() else {
                panic!()
            };
            assert_eq!(owners(intact.placement()), [10, 11]);
            assert_eq!(intact.reserved_height(), 60.);
        }
    }
}

#[test]
fn never_split_child_does_not_advertise_a_descendant_cut() {
    for anchored in [false, true] {
        let cursor = wrapper(child(SplitPolicy::Never), anchored, 60.).start();
        assert!(matches!(
            cursor.fit(area(50.)).unwrap(),
            FragmentFit::DoesNotFit {
                required_height: 60.,
                ..
            }
        ));
        let FragmentFit::Placed(whole) = cursor.fit(area(60.)).unwrap() else {
            panic!()
        };
        assert_eq!(owners(whole.placement()), [10, 11]);
        assert!(whole.continuation().is_complete());
    }
}

#[test]
fn complete_companion_aligns_in_first_fragment_and_short_budget_cannot_consume_it() {
    use rhwp::model::table::VerticalAlign;
    for alignment in [VerticalAlign::Center, VerticalAlign::Bottom] {
        let mut d = rhwp::parse_document(INPUT).unwrap();
        let Control::Table(t) = &mut d.sections[0].paragraphs[161].controls[0] else {
            panic!()
        };
        t.cells[12].vertical_align = alignment;
        let styles = rhwp::renderer::style_resolver::resolve_styles(&d.doc_info, 96.);
        let prepared = PreparedTextTable::prepare_with_end_policy(
            t,
            &styles,
            96.,
            &d.bin_data_content,
            CellEndPolicy::OmitFinalParagraphGap,
        )
        .unwrap();
        let bounds = |height| PageArea {
            bounds: Rect {
                x: 0.,
                y: 0.,
                width: 700.,
                height,
            },
        };
        // Independent saved rows0..5 occupy19836HU. Remaining10px cannot
        // accept the label's1300HU line plus two223HU insets.
        let cursor = prepared.start();
        let TextFragmentFit::Placed(short) = cursor.fit(bounds(19836. / 75. + 10.)).unwrap() else {
            panic!()
        };
        assert!(short.geometry().placement().cells.iter().all(|c| c.row < 6));
        // Retry the unaccepted row, now with enough room for the label, but
        // not the entire child. A physical cut before the stored boundary is
        // allowed; the intact companion must be placed once in that fragment.
        let TextFragmentFit::Placed(first) = short.continuation().fit(bounds(200.)).unwrap() else {
            panic!()
        };
        let label = &first.geometry().placement().cells[0];
        assert_eq!(label.row, 6);
        assert_eq!(label.lines.len(), 1);
        let line = label.lines[0].bounds;
        let expected = match alignment {
            VerticalAlign::Center => label.bounds.y + (label.bounds.height - line.height) / 2.,
            VerticalAlign::Bottom => {
                label.bounds.y + label.bounds.height - 223. / 75. - line.height
            }
            _ => unreachable!(),
        };
        near(line.y, expected);
        assert!(line.y >= label.bounds.y);
        assert!(line.y + line.height <= label.bounds.y + label.bounds.height);
        let TextFragmentFit::Placed(second) = first.continuation().fit(bounds(1000.)).unwrap()
        else {
            panic!()
        };
        assert!(second.geometry().placement().cells[0].lines.is_empty());
    }
}
