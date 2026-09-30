//! #7008 p17, unchanged original and Hancom HWPX: first row lh1128/th1156,
//! baseline935, gap752; next vpos57218 - first55310 =1908 =1156+752HU.
//! Final text boxes and split budgets must consume the same composed extent.
//! Positive cases deliberately remove character borders from resolved styles
//! only, isolating geometry from the next unsupported non-text paint boundary.
//! They are contract adapters, not original-document visual fidelity evidence.
use rhwp::{
    model::{
        document::Document,
        paragraph::Paragraph,
        table::{Cell, Table, TablePageBreak},
    },
    renderer::{
        render_tree::{PageRenderTree, RenderNode, RenderNodeType},
        style_resolver::{resolve_styles, ResolvedStyleSet},
        table_v2::*,
    },
};
fn source() -> Document {
    rhwp::parse_document(include_bytes!("../../samples/21_언어_기출_편집가능본.hwp")).unwrap()
}
fn geometry_styles(d: &Document, dpi: f64) -> ResolvedStyleSet {
    let mut styles = resolve_styles(&d.doc_info, dpi);
    for style in &mut styles.char_styles {
        style.border_fill_id = 0;
    }
    styles
}
fn table(p: Paragraph) -> Table {
    Table {
        row_count: 1,
        col_count: 1,
        page_break: TablePageBreak::RowBreak,
        cells: vec![Cell {
            width: 31888,
            row_span: 1,
            col_span: 1,
            apply_inner_margin: true,
            paragraphs: vec![p],
            ..Default::default()
        }],
        ..Default::default()
    }
}
fn nodes(n: &RenderNode) -> Vec<&RenderNode> {
    std::iter::once(n)
        .chain(n.children.iter().flat_map(nodes))
        .collect()
}
fn area(height: f64, dpi: f64) -> PageArea {
    PageArea {
        bounds: Rect {
            x: 0.,
            y: 0.,
            width: 31888. * dpi / 7200.,
            height: height * dpi / 7200.,
        },
    }
}
#[test]
fn stored_unequal_heights_keep_saved_baselines_and_row_membership() {
    let d = source();
    let p = &d.sections[0].paragraphs[17];
    let snapshot = format!("{p:?}");
    for dpi in [96., 192.] {
        let plan =
            PreparedTextTable::prepare(&table(p.clone()), &geometry_styles(&d, dpi), dpi).unwrap();
        let TextFragmentFit::Placed(fragment) = plan.start().fit(area(10000., dpi)).unwrap() else {
            panic!("missing lines")
        };
        let mut tree = PageRenderTree::new(0, 1000., 1000.);
        fragment.append_to(&mut tree).unwrap();
        let all = nodes(&tree.root);
        let lines: Vec<_> = all
            .iter()
            .filter(|n| matches!(n.node_type, RenderNodeType::TextLine(_)))
            .collect();
        assert_eq!(lines.len(), 2);
        for (n, hu) in lines.iter().zip([1156., 1100.]) {
            assert!((n.bbox.height - hu * dpi / 7200.).abs() < 1e-7);
            if let RenderNodeType::TextLine(l) = &n.node_type {
                assert!((l.baseline - 935. * dpi / 7200.).abs() < 1e-7);
            }
        }
        assert!((lines[1].bbox.y - lines[0].bbox.y - 1908. * dpi / 7200.).abs() < 1e-7);
        let painted: String = all
            .iter()
            .filter_map(|n| {
                if let RenderNodeType::TextRun(r) = &n.node_type {
                    Some(r.text.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(painted, p.text);
    }
    assert_eq!(format!("{p:?}"), snapshot);
}
#[test]
fn fit_reserves_the_text_extent_not_the_smaller_nominal_height() {
    let d = source();
    for dpi in [96., 192.] {
        let plan = PreparedTextTable::prepare(
            &table(d.sections[0].paragraphs[17].clone()),
            &geometry_styles(&d, dpi),
            dpi,
        )
        .unwrap();
        assert!(matches!(
            plan.start().fit(area(1128., dpi)).unwrap(),
            TextFragmentFit::DoesNotFit { .. }
        ));
        assert!(matches!(
            plan.start().fit(area(1155., dpi)).unwrap(),
            TextFragmentFit::DoesNotFit { .. }
        ));
        let TextFragmentFit::Placed(fragment) = plan.start().fit(area(1156., dpi)).unwrap() else {
            panic!("exact text extent should fit")
        };
        let mut tree = PageRenderTree::new(0, 1000., 1000.);
        fragment.append_to(&mut tree).unwrap();
        assert_eq!(
            nodes(&tree.root)
                .iter()
                .filter(|n| matches!(n.node_type, RenderNodeType::TextLine(_)))
                .count(),
            1
        );
        let TextFragmentFit::Placed(tail) = fragment.continuation().fit(area(10000., dpi)).unwrap()
        else {
            panic!("missing continuation")
        };
        tail.append_to(&mut tree).unwrap();
        assert!(matches!(
            tail.continuation().fit(area(10000., dpi)).unwrap(),
            TextFragmentFit::Complete
        ));
        let painted: String = nodes(&tree.root)
            .iter()
            .filter_map(|n| match &n.node_type {
                RenderNodeType::TextRun(r) => Some(r.text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(painted, d.sections[0].paragraphs[17].text);
    }
}
#[test]
fn synthetic_last_row_extent_owns_the_end_and_fits_without_truncation() {
    // Geometry-only counterexample: the LAST row, not the first, has an
    // expanded text box. This is not claimed to be a Hancom-generated sample.
    let d = source();
    let mut p = d.sections[0].paragraphs[17].clone();
    p.line_segs[1].text_height = 1156;
    for dpi in [96., 192.] {
        let plan =
            PreparedTextTable::prepare(&table(p.clone()), &geometry_styles(&d, dpi), dpi).unwrap();
        let TextFragmentFit::Placed(fragment) = plan.start().fit(area(10000., dpi)).unwrap() else {
            panic!("missing expanded last row")
        };
        let mut tree = PageRenderTree::new(0, 1000., 1000.);
        fragment.append_to(&mut tree).unwrap();
        let lines: Vec<_> = nodes(&tree.root)
            .into_iter()
            .filter(|n| matches!(n.node_type, RenderNodeType::TextLine(_)))
            .collect();
        assert_eq!(lines.len(), 2);
        assert!((lines[1].bbox.height - 1156. * dpi / 7200.).abs() < 1e-7);
        assert!(matches!(
            fragment.continuation().fit(area(10000., dpi)).unwrap(),
            TextFragmentFit::Complete
        ));
    }
}
#[test]
fn invalid_text_heights_and_baselines_remain_rejected() {
    let d = source();
    for (height, baseline) in [(0, 935), (-1, 935), (1156, 1157)] {
        let mut p = d.sections[0].paragraphs[17].clone();
        p.line_segs[0].text_height = height;
        p.line_segs[0].baseline_distance = baseline;
        assert!(PreparedTextTable::prepare(&table(p), &geometry_styles(&d, 96.), 96.).is_err());
    }
}
#[test]
fn unchanged_original_preserves_character_borders_with_their_line() {
    let d = source();
    let plan = PreparedTextTable::prepare(
        &table(d.sections[0].paragraphs[17].clone()),
        &resolve_styles(&d.doc_info, 96.),
        96.,
    )
    .unwrap();
    let mut bounds = area(1156., 96.);
    bounds.bounds.x = 31.;
    bounds.bounds.y = 53.;
    let TextFragmentFit::Placed(fragment) = plan.start().fit(bounds).unwrap() else {
        panic!("bordered row missing")
    };
    let mut tree = PageRenderTree::new(0, 1000., 1000.);
    fragment.append_to(&mut tree).unwrap();
    let all = nodes(&tree.root);
    let row = all
        .iter()
        .find(|n| matches!(n.node_type, RenderNodeType::TextLine(_)))
        .unwrap();
    let edges: Vec<_> = row
        .children
        .iter()
        .filter_map(|n| match &n.node_type {
            RenderNodeType::Line(l) => Some(l),
            _ => None,
        })
        .collect();
    // Hancom's normal-save PDF joins charPr48/49/48 into ONE rectangle:
    // an underline change does not end the identical character border.
    // Paths use the SAME translated row
    // top/bottom, not the previous page's origin. Source stored height1156HU.
    assert_eq!(edges.len(), 4);
    assert!((row.bbox.y - 53.).abs() < 1e-7);
    for node in &row.children {
        if let RenderNodeType::Line(edge) = &node.node_type {
            if edge.y1 == edge.y2 {
                // Butt caps do not extend the horizontal path at either end.
                assert!((node.bbox.x - edge.x1).abs() < 1e-7);
                assert!((node.bbox.width - (edge.x2 - edge.x1)).abs() < 1e-7);
            }
        }
    }
    for edge in edges {
        assert!(edge.x1 >= row.bbox.x && edge.x2 <= row.bbox.x + row.bbox.width + 1e-7);
        for y in [edge.y1, edge.y2] {
            assert!((y - 53.).abs() < 1e-7 || (y - 53. - 1156. / 75.).abs() < 1e-7);
        }
    }
}

#[test]
fn unsupported_character_border_effects_are_not_silently_dropped() {
    let mut d = source();
    let p = d.sections[0].paragraphs[17].clone();
    let id = d.doc_info.char_shapes[48].border_fill_id;
    assert!(id > 0);
    let mut styles = resolve_styles(&d.doc_info, 96.);
    styles.border_styles[id as usize - 1].fill_color = Some(0xff0000);
    assert!(matches!(
        PreparedTextTable::prepare(&table(p), &styles, 96.),
        Err(GeometryError::Unsupported("V2 character border geometry"))
    ));
    d.doc_info.border_fills[id as usize - 1].three_d = true;
    d.sections[0].paragraphs = vec![d.sections[0].paragraphs[17].clone()];
    let page = d.sections[0].section_def.page_def.clone();
    d.sections[0].section_def = Default::default();
    d.sections[0].section_def.page_def = page;
    let err = HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default())
        .err()
        .unwrap();
    assert!(
        format!("{err:?}").contains("V2 character border effect"),
        "{err:?}"
    );
}

#[test]
fn border_connection_uses_adjacent_style_ownership_not_only_coordinates_or_ids() {
    let d = source();
    let p = d.sections[0].paragraphs[17].clone();
    let original = resolve_styles(&d.doc_info, 96.);
    for (case, expected) in [
        ("equal_pen_new_id", 4),
        ("different_pen", 12),
        ("unbordered", 8),
    ] {
        let mut styles = original.clone();
        let id = styles.char_styles[49].border_fill_id;
        let mut border = styles.border_styles[id as usize - 1].clone();
        if case == "different_pen" {
            for edge in &mut border.borders {
                edge.color ^= 0x00ff00;
            }
        }
        styles.border_styles.push(border);
        styles.char_styles[49].border_fill_id = if case == "unbordered" {
            0
        } else {
            styles.border_styles.len() as u16
        };
        let plan = PreparedTextTable::prepare(&table(p.clone()), &styles, 96.).unwrap();
        let TextFragmentFit::Placed(fragment) = plan.start().fit(area(1156., 96.)).unwrap() else {
            panic!("bordered line missing: {case}")
        };
        let mut tree = PageRenderTree::new(0, 1000., 1000.);
        fragment.append_to(&mut tree).unwrap();
        let count = nodes(&tree.root)
            .iter()
            .filter(|n| matches!(n.node_type, RenderNodeType::Line(_)))
            .count();
        assert_eq!(count, expected, "{case}");
        let text: String = nodes(&tree.root)
            .iter()
            .filter_map(|n| match &n.node_type {
                RenderNodeType::TextRun(run) => Some(run.text.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            text.contains("프로세스에 내재된 업무 관련 규정"),
            "{case}: {text}"
        );
    }
}
