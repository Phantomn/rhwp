//! A normal saved zero-width host preserves its own line and a distinct child offset.
use rhwp::renderer::table_v2::*;
use serde_json::Value;
use std::sync::Arc;
const INPUT: &[u8] = include_bytes!("../fixtures/issue7353/anchor-offset/prefix90-saved.hwp");
fn collect<'a>(n: &'a Value, kind: &str, out: &mut Vec<&'a Value>) {
    if n["node_type"].get(kind).is_some() {
        out.push(n);
    }
    for c in n["children"].as_array().unwrap() {
        collect(c, kind, out);
    }
}
fn near(v: &Value, expected: f64) {
    assert!(
        (v.as_f64().unwrap() - expected).abs() < 1e-7,
        "{v} != {expected}"
    );
}
#[test]
fn normal_saved_offset_preserves_host_and_table_final_geometry() {
    let source = rhwp::parse_document(INPUT).unwrap();
    assert_eq!(source.sections[0].section_def.page_def.margin_left, 5669);
    let mut s = DocumentV2Session::from_bytes(
        INPUT,
        r#"{"dpi":96,"max_pages":100,"cell_end_policy":"omit_final_paragraph_gap"}"#,
    )
    .unwrap();
    let mut pages = vec![];
    while let Some(p) = s.next_page_json().unwrap() {
        pages.push(serde_json::from_str::<Value>(&p).unwrap());
    }
    assert_eq!(pages.len(), 17); // independent PDF from this exact saved HWP
    assert!(s.next_page_json().unwrap().is_none());
    let root = &pages[16]["render_tree"]["root"];
    let mut lines = vec![];
    collect(root, "TextLine", &mut lines);
    let hosts: Vec<_> = lines
        .iter()
        .filter(|n| {
            n["node_type"]["TextLine"]["para_index"] == 89
                && n["node_type"]["TextLine"]["section_index"] == 0
        })
        .collect();
    assert_eq!(hosts.len(), 1);
    near(&hosts[0]["bbox"]["y"], 75.6 + 4640. / 75.);
    near(&hosts[0]["bbox"]["height"], 1000. / 75.);
    near(&hosts[0]["bbox"]["width"], 0.);
    let mut tables = vec![];
    collect(root, "Table", &mut tables);
    let t = tables
        .into_iter()
        .find(|t| t["node_type"]["Table"]["para_index"] == 89)
        .unwrap();
    near(&t["bbox"]["y"], 75.6 + (4640. + 784. + 141.) / 75.);
    near(&t["bbox"]["x"], (5669. + 1238. + 141.) / 75.);
    near(&t["bbox"]["width"], 46207. / 75.);
    near(&t["bbox"]["height"], 58612. / 75.);
    let mut cells = vec![];
    collect(t, "TableCell", &mut cells);
    assert_eq!(cells.len(), 18);
}
fn line(p: usize) -> FlowBlock {
    FlowBlock::Lines {
        height: 20.,
        advance: 20.,
        lines: vec![LineBox {
            owner: LineOwner {
                paragraph: p,
                line: 0,
            },
            bounds: Rect {
                x: 0.,
                y: 0.,
                width: 80.,
                height: 20.,
            },
        }],
    }
}
fn plan(offset: f64, policy: SplitPolicy) -> TableContentPlan {
    let row = |blocks| FlowRowInput {
        cells: vec![FlowCellInput {
            width: 100.,
            padding: Insets::default(),
            minimum_height: 0.,
            blocks,
        }],
    };
    let child = TableContentPlan::from_flow_rows(
        vec![100.],
        if policy == SplitPolicy::BetweenRows {
            vec![row(vec![line(1)]), row(vec![line(2)])]
        } else {
            vec![row(vec![line(1), line(2)])]
        },
        0.,
        policy,
    )
    .unwrap();
    TableContentPlan::from_flow_rows(
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
                                x: 0.,
                                y: 0.,
                                width: 0.,
                                height: 10.,
                            },
                        }),
                        host_advance: 16.,
                        offset_x: 0.,
                        offset_y: offset,
                        available_width: 100.,
                        top: 3.,
                        bottom: 2.,
                        plan: Arc::new(child),
                    },
                    line(3),
                ],
            }],
        }],
        0.,
        SplitPolicy::WithinCells,
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
#[test]
fn first_offset_is_transactional_continuation_keeps_only_margin_and_following_line() {
    let cursor = plan(7., SplitPolicy::WithinCells).start();
    // Margin+line fits but source displacement does not. No host or band is consumed.
    assert!(matches!(
        cursor.fit(area(29.)).unwrap(),
        FragmentFit::DoesNotFit { .. }
    ));
    let FragmentFit::Placed(first) = cursor.fit(area(30.)).unwrap() else {
        panic!()
    };
    assert_eq!(first.reserved_height(), 30.);
    assert_eq!(first.placement().cells[0].lines.len(), 1); // host exactly once
    assert_eq!(first.placement().cells[0].lines[0].bounds.y, 20.);
    assert_eq!(
        first.placement().cells[0].tables[0].placement.cells[0].lines[0]
            .bounds
            .y,
        30.
    );
    let next = first.continuation();
    assert!(matches!(
        next.fit(area(22.)).unwrap(),
        FragmentFit::DoesNotFit { .. }
    ));
    let FragmentFit::Placed(last) = next.fit(area(45.)).unwrap() else {
        panic!()
    };
    assert_eq!(
        last.placement().cells[0].tables[0].placement.cells[0].lines[0]
            .bounds
            .y,
        23.
    );
    assert_eq!(
        last.placement().cells[0].tables[0].placement.cells[0].lines[0]
            .owner
            .paragraph,
        2
    );
    assert_eq!(last.placement().cells[0].lines.len(), 1);
    assert_eq!(last.placement().cells[0].lines[0].owner.paragraph, 3);
    assert_eq!(last.placement().cells[0].lines[0].bounds.y, 45.); // margin3+line20+bottom2
    assert_eq!(last.reserved_height(), 45.);
    assert!(last.continuation().is_complete());
    assert!(matches!(
        last.continuation().fit(area(100.)).unwrap(),
        FragmentFit::Complete
    ));
}
#[test]
fn intact_and_atomic_retry_use_the_same_offset_envelope() {
    for offset in [0., 7., 15.] {
        let cursor = plan(offset, SplitPolicy::Never).start();
        assert!(matches!(
            cursor.fit(area(offset + 42.)).unwrap(),
            FragmentFit::DoesNotFit { .. }
        ));
        let FragmentFit::Placed(f) = cursor.fit(area(100.)).unwrap() else {
            panic!()
        };
        assert!(f.continuation().is_complete());
        assert_eq!(f.reserved_height(), offset + 65.);
        assert_eq!(
            f.placement().cells[0].tables[0].placement.bounds.y,
            23. + offset
        );
        assert_eq!(f.placement().cells[0].lines[1].bounds.y, 65. + offset);
    }
}

#[test]
fn rowbreak_reserves_bottom_margin_with_cut_and_does_not_repeat_offset() {
    let cursor = plan(7., SplitPolicy::BetweenRows).start();
    assert!(matches!(
        cursor.fit(area(31.)).unwrap(),
        FragmentFit::DoesNotFit { .. }
    ));
    let FragmentFit::Placed(first) = cursor.fit(area(32.)).unwrap() else {
        panic!()
    };
    assert_eq!(first.reserved_height(), 32.); // offset7 + top3 + row20 + bottom2
    let child = &first.placement().cells[0].tables[0].placement;
    assert_eq!(child.bounds.y, 30.);
    assert_eq!(child.cells[0].lines[0].owner.paragraph, 1);
    let FragmentFit::Placed(last) = first.continuation().fit(area(45.)).unwrap() else {
        panic!()
    };
    let child = &last.placement().cells[0].tables[0].placement;
    assert_eq!(child.bounds.y, 23.);
    assert_eq!(child.cells[0].lines[0].owner.paragraph, 2);
    assert_eq!(last.placement().cells[0].lines[0].owner.paragraph, 3);
    assert_eq!(last.placement().cells[0].lines[0].bounds.y, 45.);
    assert_eq!(last.reserved_height(), 45.);
    assert!(last.continuation().is_complete());
}

#[test]
fn cell_adapter_carries_offset_into_final_nested_placement_without_moving_host() {
    use rhwp::{
        model::control::Control,
        renderer::{render_tree::PageRenderTree, style_resolver::resolve_styles},
    };
    // Synthetic property variants exercise the cell adapter, not additional
    // independently generated Hancom evidence. Source line partitions stay intact.
    let d = rhwp::parse_document(include_bytes!(
        "../fixtures/issue7353_cell_anchor_review/anchor-saved.hwp"
    ))
    .unwrap();
    let Control::Table(source) = &d.sections[0].paragraphs[0].controls[2] else {
        panic!()
    };
    let styles = resolve_styles(&d.doc_info, 96.);
    let mut geometry = vec![];
    for offset in [0, 750, u32::MAX] {
        let mut t = source.clone();
        t.cells[0].paragraphs.truncate(1);
        t.cells[0].height = 0;
        let Control::Table(child) = &mut t.cells[0].paragraphs[0].controls[0] else {
            panic!()
        };
        child.common.vertical_offset = offset;
        let prepared = PreparedTextTable::prepare_with_end_policy(
            &t,
            &styles,
            96.,
            &[],
            CellEndPolicy::OmitFinalParagraphGap,
        );
        if offset == u32::MAX {
            assert!(prepared.is_err());
            continue;
        }
        let TextFragmentFit::Placed(part) = prepared
            .unwrap()
            .start()
            .fit(PageArea {
                bounds: Rect {
                    x: 20.,
                    y: 30.,
                    width: 800.,
                    height: 1000.,
                },
            })
            .unwrap()
        else {
            panic!()
        };
        let mut page = PageRenderTree::new(0, 800., 1000.);
        part.append_to(&mut page).unwrap();
        let root = serde_json::to_value(page).unwrap();
        let mut tables = vec![];
        collect(&root["root"], "Table", &mut tables);
        let mut lines = vec![];
        collect(&root["root"], "TextLine", &mut lines);
        let host = lines
            .into_iter()
            .find(|l| l["bbox"]["width"] == 0.)
            .unwrap();
        geometry.push((
            tables[1]["bbox"]["y"].as_f64().unwrap(),
            host["bbox"]["y"].as_f64().unwrap(),
            part.geometry().reserved_height(),
        ));
    }
    assert!((geometry[1].0 - geometry[0].0 - 10.).abs() < 1e-8);
    assert_eq!(geometry[0].1, geometry[1].1);
    assert!((geometry[1].2 - geometry[0].2 - 10.).abs() < 1e-8);
}
