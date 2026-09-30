//! Original #7008 cell controls, unchanged, exercised through the V2 table
//! adapter. Isolating a cell is a contract fixture, NOT whole-document fidelity.
use rhwp::{
    model::{control::Control, document::Document, shape::ShapeObject, table::Table},
    renderer::{
        render_tree::{PageRenderTree, RenderNode, RenderNodeType},
        style_resolver::resolve_styles,
        table_v2::{CellEndPolicy, PageArea, PreparedTextTable, Rect, TextFragmentFit},
    },
};

fn source() -> Document {
    rhwp::parse_document(include_bytes!("../../samples/21_언어_기출_편집가능본.hwp")).unwrap()
}
fn selected(d: &Document, index: usize) -> Table {
    let Control::Table(t) = &d.sections[0].paragraphs[0].controls[2] else {
        panic!()
    };
    let mut t = t.as_ref().clone();
    let mut cell = t.cells[index].clone();
    cell.row = 0;
    cell.col = 0;
    cell.row_span = 1;
    cell.col_span = 1;
    t.row_count = 1;
    t.col_count = 1;
    t.common.width = cell.width;
    t.common.height = cell.height;
    t.cells = vec![cell];
    t
}
fn prepare(d: &Document, t: &Table, dpi: f64) -> PreparedTextTable {
    PreparedTextTable::prepare_with_end_policy(
        t,
        &resolve_styles(&d.doc_info, dpi),
        dpi,
        &d.bin_data_content,
        CellEndPolicy::OmitFinalParagraphGap,
    )
    .unwrap()
}
fn render(d: &Document, t: &Table, dpi: f64) -> PageRenderTree {
    let p = prepare(d, t, dpi);
    let TextFragmentFit::Placed(f) = p
        .start()
        .fit(PageArea {
            bounds: Rect {
                x: 20.,
                y: 30.,
                width: 1000.,
                height: 1000.,
            },
        })
        .unwrap()
    else {
        panic!()
    };
    let mut page = PageRenderTree::new(0, 1200., 1200.);
    f.append_to(&mut page).unwrap();
    assert!(matches!(
        f.continuation()
            .fit(PageArea {
                bounds: Rect {
                    x: 20.,
                    y: 30.,
                    width: 1000.,
                    height: 1000.
                }
            })
            .unwrap(),
        TextFragmentFit::Complete
    ));
    page
}
fn nodes<'a>(n: &'a RenderNode, predicate: fn(&RenderNodeType) -> bool) -> Vec<&'a RenderNode> {
    let mut found = Vec::new();
    if predicate(&n.node_type) {
        found.push(n);
    }
    for child in &n.children {
        found.extend(nodes(child, predicate));
    }
    found
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}

#[test]
fn original_foucault_inline_textbox_keeps_saved_lines_and_source_width() {
    let d = source();
    let Control::Table(t) = &d.sections[0].paragraphs[167].controls[0] else {
        panic!()
    };
    // Original stored rows at the preview's 96dpi. The existing common text
    // painter rejects this complete table at192dpi; do not bypass that guard.
    for dpi in [96.] {
        let p = prepare(&d, t, dpi);
        let TextFragmentFit::Placed(f) = p
            .start()
            .fit(PageArea {
                bounds: Rect {
                    x: 0.,
                    y: 0.,
                    width: 2000.,
                    height: 4000.,
                },
            })
            .unwrap()
        else {
            panic!()
        };
        let mut tree = PageRenderTree::new(0, 2000., 4000.);
        f.append_to(&mut tree).unwrap();
        let shape = nodes(&tree.root, |n| matches!(n, RenderNodeType::Rectangle(_)))[0];
        near(shape.bbox.width, 2267. * dpi / 7200.);
        near(shape.bbox.height, 1417. * dpi / 7200.);
        // Independent reference: matching PDF page8 vector bounds in points.
        // Body border starts at(98.173,511.217); first grid row is680HU.
        // Textbox interior is(116.513,523.571)..(139.168,537.725).
        // Compare relative to the selected table, not a fitted glyph position.
        assert!((shape.bbox.x - (116.513 - 98.173) * dpi / 72.).abs() < 0.2);
        assert!((shape.bbox.y - (523.571 - 511.217 + 6.8) * dpi / 72.).abs() < 0.2);
        let rows = nodes(&tree.root, |n| matches!(n, RenderNodeType::TextLine(_)));
        let host = rows
            .iter()
            .find(|n| {
                n.children
                    .iter()
                    .any(|c| matches!(&c.node_type, RenderNodeType::TextRun(r) if r.text == "는 "))
            })
            .unwrap();
        near(host.bbox.height, 1417. * dpi / 7200.);
        assert!(shape.bbox.y >= host.bbox.y - 0.02 * dpi / 96.);
        assert!(
            shape.bbox.y + shape.bbox.height <= host.bbox.y + host.bbox.height + 0.02 * dpi / 96.
        );
        // Original common.right=142HU is inline advance, not extra shape width.
        let first = host
            .children
            .iter()
            .find(|c| matches!(&c.node_type, RenderNodeType::TextRun(r) if r.text == "는 "))
            .unwrap();
        near(first.bbox.x - shape.bbox.x, (2267. + 142.) * dpi / 7200.);
        let source_para = &t.cells[5].paragraphs[0];
        let body_rows: Vec<_> = rows
            .iter()
            .filter(|n| n.bbox.y >= host.bbox.y && n.bbox.width > 300. * dpi / 96.)
            .collect();
        assert_eq!(body_rows.len(), source_para.line_segs.len());
        let painted_text: String = body_rows
            .iter()
            .flat_map(|row| row.children.iter())
            .filter_map(|n| {
                if let RenderNodeType::TextRun(run) = &n.node_type {
                    Some(run.text.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(painted_text, source_para.text);
        for (row, stored) in body_rows.iter().zip(&source_para.line_segs) {
            near(
                row.bbox.y - host.bbox.y,
                f64::from(stored.vertical_pos) * dpi / 7200.,
            );
        }
    }
}

#[test]
fn mixed_inline_textbox_rejects_objects_outside_the_saved_row() {
    let d = source();
    let Control::Table(original) = &d.sections[0].paragraphs[167].controls[0] else {
        panic!()
    };
    for bottom_margin in [false, true] {
        let mut table = original.as_ref().clone();
        let Control::Shape(shape) = &mut table.cells[5].paragraphs[0].controls[1] else {
            panic!()
        };
        let ShapeObject::Rectangle(shape) = shape.as_mut() else {
            panic!()
        };
        // Stored row1417HU owns the original1417HU object exactly. Neither
        // increasing the object nor adding exterior padding may be clipped away.
        if bottom_margin {
            shape.common.margin.bottom = 100;
        } else {
            shape.common.height += 100;
        }
        assert!(matches!(
            PreparedTextTable::prepare_with_end_policy(
                &table,
                &resolve_styles(&d.doc_info, 96.),
                96.,
                &d.bin_data_content,
                CellEndPolicy::OmitFinalParagraphGap,
            ),
            Err(rhwp::renderer::table_v2::GeometryError::Unsupported(
                "inline shape outside saved line envelope"
            ))
        ));
    }
}

#[test]
fn cell_mixed_inline_shape_does_not_inherit_host_paragraph_border_admission() {
    let d = source();
    let Control::Table(original) = &d.sections[0].paragraphs[167].controls[0] else {
        panic!()
    };
    let mut table = original.as_ref().clone();
    // Same border style accepted for host p145: the cell path has no host
    // fragment outline owner, so it must not silently accept this decoration.
    table.cells[5].paragraphs[0].para_shape_id = d.sections[0].paragraphs[145].para_shape_id;
    assert!(PreparedTextTable::prepare_with_end_policy(
        &table,
        &resolve_styles(&d.doc_info, 96.),
        96.,
        &d.bin_data_content,
        CellEndPolicy::OmitFinalParagraphGap,
    )
    .is_err());
}

#[test]
fn foreground_para_anchor_precedes_empty_line_spacing_without_removing_it() {
    let d = source();
    for dpi in [96., 192.] {
        let u = dpi / 7200.;
        let mut t = selected(&d, 2);
        t.common.height = 0;
        t.cells[0].height = 0;
        t.cells[0].vertical_align = rhwp::model::table::VerticalAlign::Top;
        // Original Para56:568HU before,2700HU empty line,284HU after.
        // Para/Top offset belongs to the paragraph origin BEFORE that line.
        for offset in [0, 720] {
            shape(&mut t).common.vertical_offset = offset;
            let tree = render(&d, &t, dpi);
            let r = nodes(&tree.root, |n| matches!(n, RenderNodeType::Rectangle(_)))[0];
            let host = nodes(&tree.root, |n| matches!(n, RenderNodeType::TextLine(_)))[0];
            near(host.bbox.y, 30. + (140. + 568.) * u);
            near(host.bbox.height, 2700. * u);
            near(r.bbox.y, 30. + (140. + f64::from(offset)) * u);
            near(
                tree.root.children[0].bbox.height,
                // OmitFinalParagraphGap excludes the final284HU after-gap,
                // not the authored568HU before-gap or2700HU empty line.
                (280. + 568. + 2700.) * u,
            );
        }
    }
}

#[test]
fn original_header_foreground_anchor_matches_independent_pdf() {
    let d = source();
    let Control::Table(t) = &d.sections[0].paragraphs[0].controls[2] else {
        panic!()
    };
    let tree = render(&d, t, 96.);
    let r = nodes(&tree.root, |n| matches!(n, RenderNodeType::Rectangle(_)))[0];
    // Original PDF1:top1043.664pt in bottom-up1190pt page coordinates.
    // Compare in original page coordinates; render() uses a30px table origin.
    let page_y = r.bbox.y - 30. + 9872. / 75.;
    let pdf_y = (1190. - 1043.664) * 96. / 72.;
    assert!((page_y - pdf_y).abs() < 0.16, "{page_y} != PDF {pdf_y}");
    near(tree.root.children[0].bbox.height, 13740. / 75.);
}

#[test]
fn rectangle_round_rate_fifty_is_semicircle_not_quarter_height() {
    let d = source();
    // HWP5 spec table94:0=square,20=rounded,50=semicircle.
    // Original PDF confirms the50 capsule;12 belongs to the TAC counterpart.
    for index in [2, 4] {
        for dpi in [96., 192.] {
            let mut t = selected(&d, index);
            for (rate, fraction) in [(0, 0.), (12, 0.12), (20, 0.2), (50, 0.5)] {
                shape(&mut t).round_rate = rate;
                let tree = render(&d, &t, dpi);
                let r = nodes(&tree.root, |n| matches!(n, RenderNodeType::Rectangle(_)))[0];
                let RenderNodeType::Rectangle(rect) = &r.node_type else {
                    unreachable!()
                };
                near(
                    rect.corner_radius,
                    r.bbox.width.min(r.bbox.height) * fraction,
                );
            }
        }
    }
}

#[test]
fn intact_bottom_cell_preserves_measured_height_at_exact_budget() {
    let d = source();
    let mut t = selected(&d, 4);
    t.page_break = rhwp::model::table::TablePageBreak::None;
    // Original #7008 cell:9160HU frame,3893HU content (3610+283).
    // This isolated plain-row counterpart exercises the same cancellation
    // boundary as the original row-spanning cell, not just its paint output.
    assert_eq!(t.cells[0].height, 9160);
    for align in [
        rhwp::model::table::VerticalAlign::Top,
        rhwp::model::table::VerticalAlign::Center,
        rhwp::model::table::VerticalAlign::Bottom,
    ] {
        t.cells[0].vertical_align = align;
        let prepared = prepare(&d, &t, 96.);
        let h = 9160. * (96. / 7200.);
        let area = |height| PageArea {
            bounds: Rect {
                x: 20.,
                y: 30.,
                width: 1000.,
                height,
            },
        };
        assert!(matches!(
            prepared.start().fit(area(h - 1. / 75.)).unwrap(),
            TextFragmentFit::DoesNotFit { .. }
        ));
        let TextFragmentFit::Placed(f) = prepared.start().fit(area(h)).unwrap() else {
            panic!("exact fit")
        };
        let mut page = PageRenderTree::new(0, 1200., 1200.);
        f.append_to(&mut page).unwrap();
        let rects = nodes(&page.root, |kind| {
            matches!(kind, RenderNodeType::Rectangle(_))
        });
        assert_eq!(rects.len(), 1);
        let offset = match align {
            rhwp::model::table::VerticalAlign::Top => 0.,
            rhwp::model::table::VerticalAlign::Center => (9160. - 3893.) / 2.,
            _ => 9160. - 3893.,
        };
        near(rects[0].bbox.y, 30. + offset * (96. / 7200.));
        near(rects[0].bbox.height, 3610. * (96. / 7200.));
        near(page.root.children[0].bbox.height, h);
        assert!(matches!(
            f.continuation().fit(area(h)).unwrap(),
            TextFragmentFit::Complete
        ));
    }
}
fn shape(t: &mut Table) -> &mut rhwp::model::shape::RectangleShape {
    let Control::Shape(s) = &mut t.cells[0].paragraphs[0].controls[0] else {
        panic!()
    };
    let ShapeObject::Rectangle(r) = s.as_mut() else {
        panic!()
    };
    r
}

#[test]
fn saved_tac_textbox_preserves_declared_object_and_centered_text() {
    let d = source();
    let t = selected(&d, 4);
    for dpi in [96., 192.] {
        let tree = render(&d, &t, dpi);
        let u = dpi / 7200.;
        let rects = nodes(&tree.root, |n| matches!(n, RenderNodeType::Rectangle(_)));
        assert_eq!(rects.len(), 1);
        let r = rects[0];
        near(r.bbox.width, 9743. * u);
        near(r.bbox.height, 3610. * u);
        let RenderNodeType::Rectangle(rect) = &r.node_type else {
            unreachable!()
        };
        near(rect.style.stroke_width, 72. * u);
        let lines = nodes(r, |n| matches!(n, RenderNodeType::TextLine(_)));
        assert_eq!(lines.len(), 1);
        near(lines[0].bbox.y - r.bbox.y, (3610. - 2500.) / 2. * u);
        near(lines[0].bbox.height, 2500. * u);
        let runs = nodes(r, |n| matches!(n, RenderNodeType::TextRun(_)));
        let text: String = runs
            .iter()
            .map(|n| match &n.node_type {
                RenderNodeType::TextRun(r) => r.text.as_str(),
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(text, "홀수형");
    }
}

#[test]
fn foreground_textbox_keeps_real_empty_host_line_and_does_not_push_following_flow() {
    let d = source();
    let mut t = selected(&d, 2);
    // Remove only the isolated contract's declared minimum: otherwise that
    // minimum could mask an erroneous flow-height increase from the decoration.
    t.common.height = 0;
    t.cells[0].height = 0;
    let before = render(&d, &t, 96.);
    let mut taller = t.clone();
    shape(&mut taller).common.height += 2000;
    let after = render(&d, &taller, 96.);
    let table = |p: &PageRenderTree| p.root.children[0].bbox.height;
    near(table(&before), table(&after));
    let r = nodes(&before.root, |n| matches!(n, RenderNodeType::Rectangle(_)))[0];
    near(r.bbox.width, 9070. / 75.);
    near(r.bbox.height, 3117. / 75.);
    let line = nodes(r, |n| matches!(n, RenderNodeType::TextLine(_)))[0];
    near(line.bbox.y - r.bbox.y, 330. / 75.);
    near(line.bbox.height, 2200. / 75.);
    let lines = nodes(&before.root, |n| matches!(n, RenderNodeType::TextLine(_)));
    assert_eq!(lines.len(), 2, "authored empty host plus textbox line");
    near(lines[0].bbox.height, 2700. / 75.);
}

#[test]
fn tac_shape_fit_requires_bottom_margin_and_emits_nothing_on_failed_query() {
    let d = source();
    let mut t = selected(&d, 4);
    t.common.height = 0;
    t.cells[0].height = 0;
    let p = prepare(&d, &t, 96.);
    let area = |hu| PageArea {
        bounds: Rect {
            x: 20.,
            y: 30.,
            width: 1000.,
            height: hu * (96. / 7200.),
        },
    };
    // Source saved row=3893HU, object=3610HU plus283HU bottom margin.
    // Using only object height would accept an incomplete line occupancy.
    assert!(matches!(
        p.start().fit(area(3892.)).unwrap(),
        TextFragmentFit::DoesNotFit { .. }
    ));
    let f = match p.start().fit(area(3893.)).unwrap() {
        TextFragmentFit::Placed(f) => f,
        TextFragmentFit::DoesNotFit {
            required_width,
            required_height,
        } => panic!(
            "requires {required_width}x{required_height}; source style {:?}",
            d.doc_info.para_shapes[t.cells[0].paragraphs[0].para_shape_id as usize]
        ),
        _ => panic!("unexpected complete"),
    };
    near(f.geometry().reserved_height(), 3893. / 75.);
    let mut page = PageRenderTree::new(0, 1200., 1200.);
    f.append_to(&mut page).unwrap();
    let r = nodes(&page.root, |n| matches!(n, RenderNodeType::Rectangle(_)));
    assert_eq!(r.len(), 1);
    near(r[0].bbox.y, 30.);
    near(r[0].bbox.height, 3610. / 75.);
    assert!(matches!(
        f.continuation().fit(area(3893.)).unwrap(),
        TextFragmentFit::Complete
    ));
}

#[test]
fn unsupported_shape_effects_or_text_overflow_are_rejected_not_hidden() {
    let d = source();
    for variant in 0..4 {
        let mut t = selected(&d, 4);
        let r = shape(&mut t);
        match variant {
            0 => r.drawing.shadow_type = 1,
            1 => r.drawing.shape_attr.rotation_angle = 10,
            2 => r.common.height = 500,
            _ => r.x_coords[1] -= 100,
        }
        assert!(PreparedTextTable::prepare_with_resources(
            &t,
            &resolve_styles(&d.doc_info, 96.),
            96.,
            &d.bin_data_content
        )
        .is_err());
    }
}

#[test]
fn multiple_inline_shapes_follow_saved_line_ownership_and_advance() {
    let d = source();
    for split in [false, true] {
        let mut t = selected(&d, 4);
        t.common.width = 24000;
        t.common.height = 0;
        t.cells[0].width = 24000;
        t.cells[0].height = 0;
        t.cells[0].padding.left = 0;
        let para = &mut t.cells[0].paragraphs[0];
        para.char_count = 17;
        para.controls.push(para.controls[0].clone());
        para.line_segs[0].segment_width = 24000;
        if split {
            let mut next = para.line_segs[0].clone();
            next.text_start = 8;
            next.vertical_pos = 3893 + 716;
            para.line_segs.push(next);
        }
        let tree = render(&d, &t, 96.);
        let rects = nodes(&tree.root, |n| matches!(n, RenderNodeType::Rectangle(_)));
        assert_eq!(rects.len(), 2);
        if split {
            near(rects[0].bbox.x, rects[1].bbox.x);
            near(rects[1].bbox.y - rects[0].bbox.y, (3893. + 716.) / 75.);
        } else {
            near(rects[0].bbox.y, rects[1].bbox.y);
            near(rects[1].bbox.x - rects[0].bbox.x, (9743. + 71.) / 75.);
        }
    }
}
