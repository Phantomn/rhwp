//! HWP range kind 1 is HWPX hp:t/@charStyleIDRef, not another paint layer.
//! Independent observation: Hancom's unchanged #7008 HWP -> HWPX conversion,
//! job 5287c9a0-3d66-4e3b-98b4-d51bfd35c131. Header styles 7/1/20 are
//! CHAR styles referencing charPr 12/9/21, also present on the actual runs.
//! The source HWP and its stored rows are never rewritten for these tests.
use rhwp::{
    model::{
        document::Document,
        paragraph::Paragraph,
        table::{Cell, Table},
    },
    renderer::{
        render_tree::{PageRenderTree, RenderNode, RenderNodeType},
        style_resolver::resolve_styles,
        table_v2::*,
    },
};
fn source() -> Document {
    rhwp::parse_document(include_bytes!("../../samples/21_언어_기출_편집가능본.hwp")).unwrap()
}
fn table(p: Paragraph) -> Table {
    Table {
        row_count: 1,
        col_count: 1,
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
fn render(d: &Document, p: Paragraph, dpi: f64) -> Result<PageRenderTree, GeometryError> {
    let prepared = PreparedTextTable::prepare(&table(p), &resolve_styles(&d.doc_info, dpi), dpi)?;
    let fit = prepared.start().fit(PageArea {
        bounds: Rect {
            x: 20.,
            y: 30.,
            width: 31888. * dpi / 7200.,
            height: 1000.,
        },
    })?;
    let TextFragmentFit::Placed(fragment) = fit else {
        panic!("missing line")
    };
    let mut tree = PageRenderTree::new(0, 1000., 1000.);
    fragment.append_to(&mut tree)?;
    Ok(tree)
}
#[test]
fn original_named_styles_keep_stored_line_and_resolved_run_styles() {
    let d = source();
    for (pi, height, ids) in [
        (5, 1100., vec![12, 8]),
        (14, 1300., vec![9, 10]),
        (26, 1300., vec![9, 56, 32, 21]),
    ] {
        let p = &d.sections[0].paragraphs[pi];
        let before = format!("{p:?}");
        assert!(p.range_tags.iter().all(|r| r.tag >> 24 == 1));
        for dpi in [96., 192.] {
            let tree = render(&d, p.clone(), dpi).unwrap();
            let all = nodes(&tree.root);
            let lines: Vec<_> = all
                .iter()
                .filter(|n| matches!(n.node_type, RenderNodeType::TextLine(_)))
                .collect();
            assert_eq!(lines.len(), 1);
            assert!((lines[0].bbox.y - 30.).abs() < 1e-7);
            assert!((lines[0].bbox.height - height * dpi / 7200.).abs() < 1e-7);
            let runs: Vec<_> = all
                .iter()
                .filter_map(|n| {
                    if let RenderNodeType::TextRun(r) = &n.node_type {
                        Some(r)
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(
                runs.iter().map(|r| r.text.as_str()).collect::<String>(),
                p.text
            );
            for id in &ids {
                assert!(runs.iter().any(|r| r.char_shape_id == Some(*id)));
            }
            // Metamorphic negative control: the editor style association adds no
            // extra paint. Stored geometry above is the independent line contract.
            let mut plain = p.clone();
            plain.range_tags.clear();
            assert_eq!(
                serde_json::to_value(&tree).unwrap(),
                serde_json::to_value(render(&d, plain, dpi).unwrap()).unwrap()
            );
        }
        assert_eq!(format!("{p:?}"), before);
    }
}
#[test]
fn fresh_named_style_association_keeps_same_composition() {
    let d = source();
    let mut p = d.sections[0].paragraphs[5].clone();
    p.line_segs.clear();
    let tagged = render(&d, p.clone(), 96.).unwrap();
    p.range_tags.clear();
    assert_eq!(
        serde_json::to_value(tagged).unwrap(),
        serde_json::to_value(render(&d, p, 96.).unwrap()).unwrap()
    );
}
#[test]
fn paint_bearing_unknown_and_invalid_ranges_remain_rejected() {
    let d = source();
    for kind in [0, 2, 3, 255] {
        let mut p = d.sections[0].paragraphs[5].clone();
        p.range_tags[0].tag = (kind << 24) | 7;
        assert!(render(&d, p, 96.).is_err(), "kind {kind}");
    }
    for (start, end) in [(2, 1), (0, 10000)] {
        let mut p = d.sections[0].paragraphs[5].clone();
        p.range_tags[0].start = start;
        p.range_tags[0].end = end;
        assert!(render(&d, p, 96.).is_err());
    }
}
