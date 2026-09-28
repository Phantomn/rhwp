//! Normally saved cell story and independent printed PDF, not edited LineSegs.
//! The table's column/paragraph anchor and its host text have distinct origins.
use rhwp::{
    model::{control::Control, table::Table},
    renderer::table_v2::*,
};
use serde_json::Value;
use std::sync::Arc;
const INPUT: &[u8] = include_bytes!("../fixtures/issue7353/cell-column/cell-column-saved.hwp");
fn nodes<'a>(n: &'a Value, kind: &str) -> Vec<&'a Value> {
    let mut result = vec![];
    if n["node_type"].get(kind).is_some() {
        result.push(n);
    }
    for child in n["children"].as_array().unwrap() {
        result.extend(nodes(child, kind));
    }
    result
}
fn b(n: &Value, k: &str) -> f64 {
    n["bbox"][k].as_f64().unwrap()
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}
fn table(d: &mut rhwp::model::document::Document) -> &mut Table {
    let Control::Table(t) = &mut d.sections[0].paragraphs[1].controls[0] else {
        panic!()
    };
    t
}
fn source_text(t: &Table) -> String {
    let mut s = String::new();
    for c in &t.cells {
        for p in &c.paragraphs {
            s.push_str(&p.text);
            for ctrl in &p.controls {
                if let Control::Table(t) = ctrl {
                    s.push_str(&source_text(t));
                }
            }
        }
    }
    s
}
#[test]
fn saved_column_anchor_uses_cell_lane_and_before_spacing_only_for_host() {
    for dpi in [96., 144.] {
        let scale = dpi / 7200.;
        let mut session = DocumentV2Session::from_bytes(
            INPUT,
            &format!(
                r#"{{"dpi":{dpi},"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}}"#
            ),
        )
        .unwrap();
        let mut pages = vec![];
        while let Some(p) = session.next_page_json().unwrap() {
            pages.push(serde_json::from_str::<Value>(&p).unwrap());
        }
        assert_eq!(pages.len(), 2);
        assert!(session.next_page_json().unwrap().is_none());
        let mut d = rhwp::parse_document(INPUT).unwrap();
        let source = table(&mut d);
        let mut actual = String::new();
        for (pi, paragraph, rows, cols, pdf_top, pdf_bottom) in [
            (0, 20, 3, 12, 189.752, 87.744),
            (1, 23, 5, 4, 761.047, 606.536),
        ] {
            let all = nodes(&pages[pi]["render_tree"]["root"], "Table");
            assert_eq!(all.len(), 2, "parent plus exactly one child per page");
            let parent = all[0];
            let child = all[1];
            assert_eq!(child["node_type"]["Table"]["row_count"], rows);
            assert_eq!(child["node_type"]["Table"]["col_count"], cols);
            let cell = &parent["children"][0];
            let hosts: Vec<_> = cell["children"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|n| n["node_type"].get("TextLine").is_some() && b(n, "width") == 0.)
                .collect();
            assert_eq!(hosts.len(), 1);
            // Source host cs50 and before100 do not change Column/Left or
            // Para/Top table origins. Only the table's own outer margin283 does.
            near(b(hosts[0], "x"), b(cell, "x") + 50. * scale);
            near(b(child, "x"), b(cell, "x") + 283. * scale);
            near(b(child, "y") - b(hosts[0], "y"), (283. - 100.) * scale);
            let p = &source.cells[0].paragraphs[paragraph];
            near(
                b(hosts[0], "y"),
                b(cell, "y") + (141. + f64::from(p.line_segs[0].vertical_pos)) * scale,
            );
            // Independent PDF strokes use paper841pt and printer .119869
            // instead of .12: tolerance accounts only for that measured scale.
            for (actual, pt) in [
                (b(child, "y"), pdf_top),
                (b(child, "y") + b(child, "height"), pdf_bottom),
            ] {
                let expected = (841. - pt) * dpi / 72.;
                assert!(
                    (actual - expected).abs()
                        <= expected * (1. - 0.119869 / 0.12) + 0.2 * dpi / 96.,
                    "page{}: {actual} vs PDF{expected}",
                    pi + 1
                );
            }
            assert!(
                b(child, "y") + b(child, "height") <= b(parent, "y") + b(parent, "height") + 1e-7
            );
            for run in nodes(parent, "TextRun") {
                actual.push_str(run["node_type"]["TextRun"]["text"].as_str().unwrap());
            }
            let last = cell["children"].as_array().unwrap().last().unwrap();
            assert!(
                last["node_type"].get("TextLine").is_some(),
                "following paragraph preserved"
            );
            assert!(b(last, "y") >= b(child, "y") + b(child, "height"));
        }
        assert_eq!(
            actual,
            source_text(source),
            "all source content once, in order"
        );
    }
}

#[test]
fn absent_multicolumn_and_mismatched_text_origin_are_not_column_evidence() {
    for variant in 0..3 {
        let mut d = rhwp::parse_document(INPUT).unwrap();
        let t = table(&mut d);
        if variant == 0 {
            t.cells[0].paragraphs[0].controls.clear();
            t.cells[0].paragraphs[0].column_type = rhwp::model::paragraph::ColumnBreakType::None;
        }
        if variant == 1 {
            let Control::ColumnDef(cd) = &mut t.cells[0].paragraphs[0].controls[0] else {
                panic!()
            };
            cd.column_count = 2;
        }
        if variant == 2 {
            t.cells[0].paragraphs[20].line_segs[0].column_start += 1;
        }
        let styles = rhwp::renderer::style_resolver::resolve_styles(&d.doc_info, 96.);
        let e = PreparedTextTable::prepare(table(&mut d), &styles, 96.)
            .err()
            .unwrap();
        assert!(matches!(e, GeometryError::Unsupported(_)));
    }
}

#[test]
fn shifted_host_envelope_is_transactional_and_following_content_uses_its_end() {
    let child = TableContentPlan::from_flow_rows(
        vec![40.],
        vec![FlowRowInput {
            cells: vec![FlowCellInput {
                width: 40.,
                padding: Insets::default(),
                minimum_height: 5.,
                blocks: vec![],
            }],
        }],
        0.,
        SplitPolicy::Never,
    )
    .unwrap();
    let plan = TableContentPlan::from_flow_rows(
        vec![100.],
        vec![FlowRowInput {
            cells: vec![FlowCellInput {
                width: 100.,
                padding: Insets::default(),
                minimum_height: 0.,
                blocks: vec![
                    FlowBlock::AnchoredTable {
                        owner: ControlOwner {
                            paragraph: 0,
                            control: 0,
                        },
                        host: Some(LineBox {
                            owner: LineOwner {
                                paragraph: 0,
                                line: 0,
                            },
                            bounds: Rect {
                                x: 7.,
                                y: 12.,
                                width: 0.,
                                height: 20.,
                            },
                        }),
                        host_advance: 35.,
                        offset_x: 3.,
                        offset_y: 0.,
                        available_width: 40.,
                        top: 2.,
                        bottom: 2.,
                        plan: Arc::new(child),
                    },
                    FlowBlock::Lines {
                        height: 10.,
                        advance: 10.,
                        lines: vec![LineBox {
                            owner: LineOwner {
                                paragraph: 1,
                                line: 0,
                            },
                            bounds: Rect {
                                x: 0.,
                                y: 0.,
                                width: 90.,
                                height: 10.,
                            },
                        }],
                    },
                ],
            }],
        }],
        0.,
        SplitPolicy::WithinCells,
    )
    .unwrap();
    let cursor = plan.start();
    let area = |h| PageArea {
        bounds: Rect {
            x: 10.,
            y: 20.,
            width: 100.,
            height: h,
        },
    };
    assert!(
        matches!(
            cursor.fit(area(31.)).unwrap(),
            FragmentFit::DoesNotFit { .. }
        ),
        "unshifted20px host fits; actual32px envelope does not"
    );
    let FragmentFit::Placed(first) = cursor.fit(area(32.)).unwrap() else {
        panic!()
    };
    near(first.reserved_height(), 32.);
    near(first.placement().cells[0].lines[0].bounds.x, 17.);
    near(first.placement().cells[0].lines[0].bounds.y, 32.);
    near(first.placement().cells[0].tables[0].placement.bounds.y, 22.);
    let FragmentFit::Placed(last) = first.continuation().fit(area(13.)).unwrap() else {
        panic!()
    };
    assert!(last.placement().cells[0].tables.is_empty());
    assert_eq!(last.placement().cells[0].lines.len(), 1);
    near(last.placement().cells[0].lines[0].bounds.y, 23.);
    assert!(last.continuation().is_complete());
}
