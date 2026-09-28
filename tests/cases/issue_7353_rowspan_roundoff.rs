//! A normally saved #6923 table has1482HU occupied rows (1200+141+141),
//! even where the source row minimum is1382HU. The three-row minimum4446HU
//! must not demand redistribution for the f64 sum's one-bit residual.
use rhwp::{
    model::control::Control,
    renderer::{style_resolver::resolve_styles, table_v2::*},
};

#[test]
fn stored_inline_rowspan_table_fits_its_complete_box_at_any_page_origin() {
    let doc = rhwp::parse_document(include_bytes!(
        "../fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp"
    ))
    .unwrap();
    let Control::Table(parent) = &doc.sections[0].paragraphs[5].controls[0] else {
        panic!()
    };
    let Control::Table(table) = &parent.cells[0].paragraphs[4].controls[0] else {
        panic!()
    };
    assert_eq!(table.common.height, 33372);
    let styles = resolve_styles(&doc.doc_info, 96.0);
    let prepared = PreparedTextTable::prepare_with_end_policy(
        table,
        &styles,
        96.0,
        &[],
        CellEndPolicy::OmitFinalParagraphGap,
    )
    .unwrap();
    // Original declared height and Hancom p1's intact four-row nested table.
    // Translation must not drop the last row. One HU short must still not fit
    // the complete table; this is not a tolerance enlargement.
    for y in [0.0, 98.26666666666668, 488.6933333333333, 1000.0] {
        for deficit in [0.0, 1.0 / 75.0] {
            let TextFragmentFit::Placed(part) = prepared
                .start()
                .fit(PageArea {
                    bounds: Rect {
                        x: 80.0,
                        y,
                        width: table.common.width as f64 / 75.0,
                        height: 33372.0 / 75.0 - deficit,
                    },
                })
                .unwrap()
            else {
                panic!("first connected group fits");
            };
            assert_eq!(
                matches!(
                    part.continuation()
                        .fit(PageArea {
                            bounds: Rect {
                                x: 80.0,
                                y,
                                width: 800.0,
                                height: 1000.0
                            }
                        })
                        .unwrap(),
                    TextFragmentFit::Complete
                ),
                deficit == 0.0
            );
            if deficit == 0.0 {
                let cells = &part.geometry().placement().cells;
                assert_eq!(cells.len(), table.cells.len());
                assert!(cells.iter().any(|cell| cell.row == 3));
                let mut tree = rhwp::renderer::render_tree::PageRenderTree::new(0, 900.0, 2000.0);
                part.append_to(&mut tree).unwrap();
                assert!((tree.root.children[0].bbox.height - 33372.0 / 75.0).abs() < 1e-12);
            }
        }
    }
}

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
    // This boundary probe explicitly requests whole-cell cuts. The unchanged
    // normally saved document is separately exercised by stored_cell_frames
    // and fragment_minimum_band; its raw2 allows internal line cuts.
    let mut whole_cell = table.as_ref().clone();
    whole_cell.page_break = rhwp::model::table::TablePageBreak::CellBreak;
    let prepared = prepare(&whole_cell).unwrap();
    let area = |h| PageArea {
        bounds: Rect {
            x: 20.0,
            y: 30.0,
            width: 800.0,
            height: h,
        },
    };
    // A physical1HU shortage must not admit the third1482HU whole row. TABLE
    // may now accept the first two rows inside the span, but never shrink them.
    let TextFragmentFit::Placed(short) = prepared.start().fit(area(4445.0 / 75.0)).unwrap() else {
        panic!("two logical rows fit")
    };
    assert_eq!(short.geometry().rows(), 0..2);
    // The unconsumed1481HU is a physical closing band, not a third row.
    assert!((short.geometry().reserved_height() - 4445.0 / 75.0).abs() < 1e-12);
    assert!(matches!(
        short.continuation().fit(area(1000.0)).unwrap(),
        TextFragmentFit::Placed(_)
    ));
    let mut cursor = prepared.start();
    let mut owners = Vec::new();
    let mut heights = Vec::new();
    let mut row_ranges = Vec::new();
    for _ in 0..10 {
        match cursor.fit(area(4446.0 / 75.0)).unwrap() {
            TextFragmentFit::Complete => break,
            TextFragmentFit::Placed(part) => {
                let geometry = part.geometry();
                let placement = geometry.placement();
                assert!(geometry.reserved_height() <= 4446.0 / 75.0);
                heights.push(geometry.reserved_height());
                row_ranges.push(geometry.rows());
                for c in &placement.cells {
                    assert!(c.bounds.y + c.bounds.height <= 30.0 + 4446.0 / 75.0 + 1e-12);
                    if c.row == 0 && c.column == 0 {
                        assert!((c.bounds.height - 4446.0 / 75.0).abs() < 1e-12);
                    }
                    if c.row < 3 && c.row_span == 1 {
                        assert!((c.bounds.height - 1482.0 / 75.0).abs() < 1e-12);
                    }
                    if c.visible_rows.start == c.row {
                        owners.push((c.row, c.column));
                    } else {
                        assert!(c.lines.is_empty() && c.tables.is_empty());
                    }
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
    assert_eq!(row_ranges, [0..3, 3..5, 5..8]);
    // Source r3 minimum1765HU, r4 occupied1482HU. The next1482HU row
    // does not fit the4446HU frame. The crossing r4 span closes its second
    // fragment with1199HU blank physical space; that is not consumed r5.
    assert_eq!(
        table.cells.iter().find(|c| c.row == 3).unwrap().height,
        1765
    );
    let closing_band_hu = 4446.0 - (1765.0 + 1482.0);
    assert!((heights.iter().sum::<f64>() - (12139.0 + closing_band_hu) / 75.0).abs() < 1e-12);
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
