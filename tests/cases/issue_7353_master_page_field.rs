//! A normally saved master textbox field uses final page identity, not cache.
use rhwp::{
    model::{control::Control, document::Document, shape::ShapeObject},
    renderer::{
        render_tree::{RenderNode, RenderNodeType},
        table_v2::*,
    },
};
fn source() -> Document {
    rhwp::parse_document(include_bytes!(
        "../fixtures/issue7353_host_master_field_review/saved.hwp"
    ))
    .unwrap()
}
fn session(d: &Document, dpi: f64) -> HostedSectionSession {
    HostedSectionSession::from_document(d, 0, dpi, CellEndPolicy::OmitFinalParagraphGap).unwrap()
}
fn nodes(n: &RenderNode) -> Vec<&RenderNode> {
    std::iter::once(n)
        .chain(n.children.iter().flat_map(nodes))
        .collect()
}
fn field(d: &mut Document) -> &mut rhwp::model::paragraph::Paragraph {
    let Control::Shape(s) =
        &mut d.sections[0].section_def.master_pages[0].paragraphs[0].controls[1]
    else {
        panic!()
    };
    let ShapeObject::Rectangle(r) = s.as_mut() else {
        panic!()
    };
    &mut r.drawing.text_box.as_mut().unwrap().paragraphs[0]
}
#[test]
fn saved_fields_follow_final_pages_and_preserve_centered_reserved_line() {
    let d = source();
    let before = format!("{d:?}");
    let roundtrip = rhwp::parse_document(&rhwp::serializer::serialize_hwpx(&d).unwrap()).unwrap();
    for d in [&d, &roundtrip] {
        for dpi in [96., 192.] {
            let scale = dpi / 7200.;
            let s = session(d, dpi);
            assert_eq!(s.pagination().pages.len(), 3);
            // Exercise repeated and out-of-order rendering without state drift.
            for i in [2, 0, 1, 2, 1] {
                let tree = s.render_page(i).unwrap();
                let fields: Vec<_> = nodes(&tree.root).into_iter().filter(|n| matches!(&n.node_type, RenderNodeType::TextRun(r) if r.display_text.is_some())).collect();
                assert_eq!(fields.len(), usize::from(i != 0));
                if i == 0 {
                    continue;
                }
                let RenderNodeType::TextRun(r) = &fields[0].node_type else {
                    panic!()
                };
                assert_eq!(
                    r.display_text.as_deref(),
                    Some((i + 1).to_string().as_str())
                );
                let master = tree
                    .root
                    .children
                    .iter()
                    .find(|n| matches!(n.node_type, RenderNodeType::MasterPage))
                    .unwrap();
                let rect = master
                    .children
                    .iter()
                    .find(|n| {
                        matches!(n.node_type, RenderNodeType::Rectangle(_))
                            && (n.bbox.y - 31000. * scale).abs() < 1e-7
                    })
                    .unwrap();
                let line = &rect.children[0];
                let Control::Shape(sh) =
                    &d.sections[0].section_def.master_pages[i - 1].paragraphs[0].controls[1]
                else {
                    panic!()
                };
                let ShapeObject::Rectangle(source_rect) = sh.as_ref() else {
                    panic!()
                };
                let textbox = source_rect.drawing.text_box.as_ref().unwrap();
                // Independent center invariant:3000HU minus source padding,
                // with a1200HU font row (not a height read from rendered nodes).
                let top = f64::from(textbox.margin_top);
                let bottom = f64::from(textbox.margin_bottom);
                assert!(
                    (line.bbox.y - (31000. + top + (3000. - top - bottom - 1200.) / 2.) * scale)
                        .abs()
                        < 1e-7
                );
                assert!((line.bbox.height - 1200. * scale).abs() < 1e-7);
                assert!(fields[0].bbox.x >= line.bbox.x - 1e-7);
                assert!(
                    fields[0].bbox.x + fields[0].bbox.width <= line.bbox.x + line.bbox.width + 1e-7
                );
            }
        }
    }
    assert_eq!(format!("{d:?}"), before);
}
#[test]
fn cached_number_is_not_used_and_decorations_do_not_move_body() {
    let mut d = source();
    let Control::AutoNumber(n) = &mut field(&mut d).controls[0] else {
        panic!()
    };
    n.number = 91;
    n.assigned_number = 92;
    let s = session(&d, 96.);
    let mut bare = d.clone();
    bare.sections[0].section_def.master_pages.clear();
    let b = session(&bare, 96.);
    for i in 0..3 {
        let mut tree = s.render_page(i).unwrap();
        for n in nodes(&tree.root) {
            if let RenderNodeType::TextRun(r) = &n.node_type {
                if let Some(display) = &r.display_text {
                    assert_eq!(display, &(i + 1).to_string());
                }
            }
        }
        tree.root
            .children
            .retain(|n| !matches!(n.node_type, RenderNodeType::MasterPage));
        let other = b.render_page(i).unwrap();
        // IDs are allocation details; actual body node type and boxes agree.
        let values = |n: &RenderNode| {
            nodes(n)
                .iter()
                .map(|v| {
                    (
                        format!("{:?}", v.node_type),
                        [v.bbox.x, v.bbox.y, v.bbox.width, v.bbox.height],
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(values(&tree.root), values(&other.root));
    }
}
#[test]
fn terminal_style_is_not_an_interior_field_style_change() {
    let mut d = source();
    let mut terminal_style = d.doc_info.char_shapes[0].clone();
    terminal_style.text_color = 0x0000ff00;
    terminal_style.bold = true;
    let terminal_id = d.doc_info.char_shapes.len() as u32;
    d.doc_info.char_shapes.push(terminal_style);
    let p = field(&mut d);
    let mut end = p.char_shapes[0].clone();
    end.start_pos = p.char_count - 1;
    end.char_shape_id = terminal_id;
    p.char_shapes.push(end);
    let tree = session(&d, 96.).render_page(1).unwrap();
    let run = nodes(&tree.root)
        .into_iter()
        .find_map(|n| match &n.node_type {
            RenderNodeType::TextRun(r) if r.display_text.is_some() => Some(r),
            _ => None,
        })
        .unwrap();
    assert_eq!(run.style.color, d.doc_info.char_shapes[0].text_color);
    assert!(!run.style.bold);
    field(&mut d).char_shapes.last_mut().unwrap().start_pos = 1;
    assert!(
        HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::OmitFinalParagraphGap)
            .is_err()
    );
}
#[test]
fn unsupported_number_formats_remain_rejected() {
    let mut d = source();
    let Control::AutoNumber(n) = &mut field(&mut d).controls[0] else {
        panic!()
    };
    n.format = 1;
    assert!(
        HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::OmitFinalParagraphGap)
            .is_err()
    );
}

#[test]
fn textbox_reflow_does_not_relax_table_cell_stored_row_contract() {
    let d =
        rhwp::parse_document(include_bytes!("../../samples/21_언어_기출_편집가능본.hwp")).unwrap();
    let Control::Table(t) = &d.sections[0].paragraphs[206].controls[0] else {
        panic!()
    };
    let mut t = t.clone();
    t.cells[0].paragraphs[0].line_segs.clear();
    assert!(PreparedTextTable::prepare_with_end_policy(
        &t,
        &rhwp::renderer::style_resolver::resolve_styles(&d.doc_info, 96.),
        96.,
        &d.bin_data_content,
        CellEndPolicy::OmitFinalParagraphGap,
    )
    .is_err());
}

#[test]
fn repeated_master_displays_two_digit_final_page_number() {
    // Synthetic longer body: reuse intact plain rows, not another PDF oracle.
    let mut d = source();
    let body = d.sections[0].paragraphs[6].clone();
    assert!(body.controls.is_empty());
    d.sections[0]
        .paragraphs
        .extend(std::iter::repeat_n(body, 12));
    let s = session(&d, 96.);
    assert!(s.pagination().pages.len() >= 11);
    for i in [9, 10, 1, 9] {
        let tree = s.render_page(i).unwrap();
        let displays: Vec<_> = nodes(&tree.root)
            .into_iter()
            .filter_map(|n| match &n.node_type {
                RenderNodeType::TextRun(r) => r.display_text.as_deref(),
                _ => None,
            })
            .collect();
        assert_eq!(displays, vec![(i + 1).to_string()]);
    }
}
