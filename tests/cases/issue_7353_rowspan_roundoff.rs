//! A normally saved #6923 table has1482HU occupied rows (1200+141+141),
//! even where the source row minimum is1382HU. The three-row minimum4446HU
//! must not demand redistribution for the f64 sum's one-bit residual.
use rhwp::{
    model::control::Control,
    renderer::{style_resolver::resolve_styles, table_v2::*},
};

#[test]
fn stored_rowspan_minimum_roundoff_preserves_final_boxes_and_real_deficits() {
    let doc = rhwp::parse_document(
        &std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp"
        ))
        .unwrap(),
    )
    .unwrap();
    let Control::Table(parent) = &doc.sections[0].paragraphs[5].controls[0] else {
        panic!()
    };
    let Control::Table(table) = &parent.cells[0].paragraphs[26].controls[0] else {
        panic!()
    };
    let styles = resolve_styles(&doc.doc_info, 96.0);
    let prepare = |t: &rhwp::model::table::Table| {
        PreparedTextTable::prepare_with_end_policy(
            t,
            &styles,
            96.0,
            &[],
            CellEndPolicy::OmitFinalParagraphGap,
        )
    };
    let prepared = prepare(table).unwrap();
    let area = |h| PageArea {
        bounds: Rect {
            x: 20.0,
            y: 30.0,
            width: 800.0,
            height: h,
        },
    };
    // First connected group is4446HU. A physical1HU shortage cannot be hidden
    // by the declared-minimum comparison's arithmetic error allowance.
    assert!(matches!(
        prepared.start().fit(area(4445.0 / 75.0)).unwrap(),
        TextFragmentFit::DoesNotFit { .. }
    ));
    let mut cursor = prepared.start();
    let mut owners = Vec::new();
    let mut heights = Vec::new();
    for _ in 0..10 {
        match cursor.fit(area(4446.0 / 75.0)).unwrap() {
            TextFragmentFit::Complete => break,
            TextFragmentFit::Placed(part) => {
                let geometry = part.geometry();
                let placement = geometry.placement();
                assert!(geometry.reserved_height() <= 4446.0 / 75.0);
                heights.push(geometry.reserved_height());
                for c in &placement.cells {
                    assert!(c.bounds.y + c.bounds.height <= 30.0 + 4446.0 / 75.0 + 1e-12);
                    if c.row == 0 && c.column == 0 {
                        assert!((c.bounds.height - 4446.0 / 75.0).abs() < 1e-12);
                    }
                    if c.row < 3 && c.row_span == 1 {
                        assert!((c.bounds.height - 1482.0 / 75.0).abs() < 1e-12);
                    }
                    owners.push((c.row, c.column));
                }
                let mut page = rhwp::renderer::render_tree::PageRenderTree::new(0, 850.0, 100.0);
                part.append_to(&mut page).unwrap();
                let actual = &page.root.children[0];
                assert_eq!(actual.bbox.height, placement.bounds.height);
                for cell in &placement.cells {
                    let node=actual.children.iter().find(|n|matches!(&n.node_type,
                        rhwp::renderer::render_tree::RenderNodeType::TableCell(v) if usize::from(v.row)==cell.row && usize::from(v.col)==cell.column)).unwrap();
                    assert_eq!(node.bbox.height, cell.bounds.height);
                    assert_eq!(node.bbox.y, cell.bounds.y);
                }
                cursor = part.continuation();
            }
            _ => panic!("unexpected blocked group"),
        }
    }
    assert!(matches!(
        cursor.fit(area(1000.0)).unwrap(),
        TextFragmentFit::Complete
    ));
    owners.sort_unstable();
    let mut expected: Vec<_> = table
        .cells
        .iter()
        .map(|c| (usize::from(c.row), usize::from(c.col)))
        .collect();
    expected.sort_unstable();
    assert_eq!(owners, expected);
    assert!((heights.iter().sum::<f64>() - 12139.0 / 75.0).abs() < 1e-12);
    // Real extra source height remains unsupported; this is not redistribution.
    for delta in [1, 100, 10000] {
        let mut larger = (**table).clone();
        larger.cells[0].height += delta;
        assert!(matches!(
            prepare(&larger),
            Err(GeometryError::Unsupported(
                "rowspan height needs redistribution"
            ))
        ));
    }
}
