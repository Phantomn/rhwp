//! Normal Hancom save and matching PDF: first-page master hidden, Both on
//! page 2, Odd on page 3; a stored TAC rectangle stays in its owner's line.
use rhwp::{
    model::{control::Control, document::Document},
    renderer::{
        render_tree::{RenderNode, RenderNodeType},
        table_v2::*,
    },
};

fn source() -> Document {
    rhwp::parse_document(include_bytes!(
        "../fixtures/issue7353_host_master_review/saved.hwp"
    ))
    .unwrap()
}
fn session(d: &Document, dpi: f64) -> Result<HostedSectionSession, HostedTableError> {
    HostedSectionSession::from_document(d, 0, dpi, CellEndPolicy::OmitFinalParagraphGap)
}
fn text(n: &RenderNode) -> String {
    let mut out = match &n.node_type {
        RenderNodeType::TextRun(r) => r.text.clone(),
        _ => String::new(),
    };
    for c in &n.children {
        out.push_str(&text(c));
    }
    out
}
fn nodes(n: &RenderNode) -> Vec<&RenderNode> {
    std::iter::once(n)
        .chain(n.children.iter().flat_map(nodes))
        .collect()
}
fn body_lines(n: &RenderNode) -> Vec<&RenderNode> {
    if matches!(
        n.node_type,
        RenderNodeType::MasterPage | RenderNodeType::Table(_)
    ) {
        return vec![];
    }
    if matches!(n.node_type, RenderNodeType::TextLine(_)) {
        return vec![n];
    }
    n.children.iter().flat_map(body_lines).collect()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}

#[test]
fn v2_master_preserves_horizontal_vertical_lines_without_legacy_dispatch() {
    use rhwp::model::shape::{LineShape, ShapeObject};
    // Synthetic variants of the saved master: a horizontal/vertical line has
    // zero extent on one axis, but still owns a visible stroked segment.
    for vertical in [false, true] {
        let mut d = source();
        let c = &mut d.sections[0].section_def.master_pages[0].paragraphs[0].controls[0];
        let Control::Shape(shape) = c else { panic!() };
        let ShapeObject::Rectangle(rect) = shape.as_ref() else {
            panic!()
        };
        let mut line = LineShape {
            common: rect.common.clone(),
            drawing: rect.drawing.clone(),
            ..Default::default()
        };
        line.drawing.text_box = None;
        let a = &mut line.drawing.shape_attr;
        line.common.width = if vertical { 0 } else { 14000 };
        line.common.height = if vertical { 3000 } else { 0 };
        a.original_width = line.common.width;
        a.original_height = line.common.height;
        a.current_width = line.common.width;
        a.current_height = line.common.height;
        line.end.x = line.common.width as i32;
        line.end.y = line.common.height as i32;
        line.drawing.border_line.attr = 2; // dashed, not the inline solid-only gate
        line.drawing.border_line.width = 75;
        **shape = ShapeObject::Line(line);
        let s = session(&d, 96.).unwrap();
        let tree = s.render_page(1).unwrap();
        let master = tree
            .root
            .children
            .iter()
            .find(|n| matches!(n.node_type, RenderNodeType::MasterPage))
            .unwrap();
        let n = master
            .children
            .iter()
            .find(|n| matches!(n.node_type, RenderNodeType::Line(_)))
            .unwrap();
        let RenderNodeType::Line(line) = &n.node_type else {
            panic!()
        };
        close(line.x1, 2400. * 96. / 7200.);
        close(line.y1, 27000. * 96. / 7200.);
        close(
            line.x2 - line.x1,
            if vertical { 0. } else { 14000. * 96. / 7200. },
        );
        close(
            line.y2 - line.y1,
            if vertical { 3000. * 96. / 7200. } else { 0. },
        );
        assert!(line.style.width > 0.);
        assert_eq!(line.section_index, Some(0));
        assert_eq!(line.para_index, Some(0));
        assert_eq!(line.control_index, Some(0));
    }
}

#[test]
fn saved_master_selection_and_body_flow_are_one_host_result() {
    let d = source();
    let snapshot = format!("{d:?}");
    let roundtrip = rhwp::parse_document(&rhwp::serializer::serialize_hwpx(&d).unwrap()).unwrap();
    for d in [&d, &roundtrip] {
        for dpi in [96., 192.] {
            let scale = dpi / 7200.;
            let s = session(d, dpi).unwrap();
            assert_eq!(s.pagination().pages.len(), 3);
            for (i, expected) in [None, Some("BASE MASTER"), Some("ODD MASTER")]
                .into_iter()
                .enumerate()
            {
                let tree = s.render_page(i).unwrap();
                let masters: Vec<_> = tree
                    .root
                    .children
                    .iter()
                    .filter(|n| matches!(n.node_type, RenderNodeType::MasterPage))
                    .collect();
                assert_eq!(masters.len(), usize::from(expected.is_some()));
                if let Some(label) = expected {
                    assert_eq!(text(masters[0]), label);
                    let rect = nodes(masters[0])
                        .into_iter()
                        .find(|n| matches!(n.node_type, RenderNodeType::Rectangle(_)))
                        .unwrap();
                    close(rect.bbox.x, if i == 1 { 2400. } else { 18000. } * scale);
                    close(rect.bbox.y, 27000. * scale);
                    close(rect.bbox.width, 14000. * scale);
                    close(rect.bbox.height, 3000. * scale);
                    let line = rect
                        .children
                        .iter()
                        .find(|n| matches!(n.node_type, RenderNodeType::TextLine(_)))
                        .unwrap();
                    let Control::Shape(sh) =
                        &d.sections[0].section_def.master_pages[i - 1].paragraphs[0].controls[0]
                    else {
                        panic!()
                    };
                    let rhwp::model::shape::ShapeObject::Rectangle(r) = sh.as_ref() else {
                        panic!()
                    };
                    let t = r.drawing.text_box.as_ref().unwrap();
                    // Independent center-alignment invariant: saved 1200HU
                    // text box centered in the source padded 3000HU rectangle.
                    let top = f64::from(t.margin_top);
                    let bottom = f64::from(t.margin_bottom);
                    close(
                        line.bbox.y,
                        (27000. + top + (3000. - top - bottom - 1200.) / 2.) * scale,
                    );
                    close(line.bbox.height, 1200. * scale);
                }
            }
            let tree = s.render_page(1).unwrap();
            let rows = body_lines(&tree.root);
            let row = rows.iter().find(|n| text(n).contains("LEFT")).unwrap();
            assert_eq!(text(row), "LEFT BOX RIGHT");
            close(row.bbox.x, 2400. * scale);
            close(row.bbox.y, 15200. * scale);
            close(row.bbox.height, 1600. * scale);
            let shape = row
                .children
                .iter()
                .find(|n| matches!(n.node_type, RenderNodeType::Rectangle(_)))
                .unwrap();
            close(shape.bbox.y, row.bbox.y);
            close(shape.bbox.height, 1600. * scale);
            close(shape.bbox.width, 4500. * scale);
            let left = row
                .children
                .iter()
                .find(|n| text(n).contains("LEFT"))
                .unwrap();
            let right = row
                .children
                .iter()
                .find(|n| text(n).contains("RIGHT"))
                .unwrap();
            assert!(
                left.bbox.x < shape.bbox.x
                    && shape.bbox.x + shape.bbox.width <= right.bbox.x + 1e-7
            );
            let tail = rows.iter().find(|n| text(n) == "TAIL 01").unwrap();
            close(tail.bbox.y, 17000. * scale);
            let last = s.render_page(2).unwrap();
            let rows = body_lines(&last.root);
            assert_eq!(
                rows.iter().map(|n| text(n)).collect::<Vec<_>>(),
                ["TAIL 12", "TAIL 13", "TAIL 14", "END"]
            );
            close(rows[3].bbox.y, 13400. * scale);
        }
    }
    assert_eq!(format!("{d:?}"), snapshot);
}

#[test]
fn page_decoration_does_not_consume_or_duplicate_body_lines() {
    let d = source();
    let mut bare = d.clone();
    bare.sections[0].section_def.master_pages.clear();
    let a = session(&d, 96.).unwrap();
    let b = session(&bare, 96.).unwrap();
    for i in 0..3 {
        let a = a.render_page(i).unwrap();
        let b = b.render_page(i).unwrap();
        let signature = |n: &RenderNode| {
            body_lines(n)
                .into_iter()
                .map(|n| (text(n), n.bbox.x, n.bbox.y, n.bbox.height))
                .collect::<Vec<_>>()
        };
        assert_eq!(signature(&a.root), signature(&b.root));
    }
    let mut visible = d;
    visible.sections[0].section_def.hide_master_page = false;
    visible.sections[0].section_def.flags &= !4;
    let tree = session(&visible, 96.).unwrap().render_page(0).unwrap();
    assert!(tree
        .root
        .children
        .iter()
        .any(|n| matches!(n.node_type, RenderNodeType::MasterPage) && text(n) == "ODD MASTER"));
}

#[test]
fn unsupported_master_stories_are_not_sent_to_legacy_table_layout() {
    let d = source();
    let mut nested = d.clone();
    let table = d.sections[0]
        .paragraphs
        .iter()
        .flat_map(|p| &p.controls)
        .find(|c| matches!(c, Control::Table(_)))
        .unwrap()
        .clone();
    nested.sections[0].section_def.master_pages[0].paragraphs[0]
        .controls
        .push(table);
    let mut extension = d.clone();
    extension.sections[0].section_def.master_pages[0].is_extension = true;
    let mut vertical = d.clone();
    vertical.sections[0].section_def.master_pages[0].text_direction = 1;
    for d in [nested, extension, vertical] {
        assert!(matches!(
            session(&d, 96.),
            Err(HostedTableError::UnsupportedHost(
                "master-page content requires host admission"
            ))
        ));
    }
}

#[test]
fn unsupported_first_page_flags_remain_explicit() {
    let mut d = source();
    d.sections[0].section_def.hide_empty_line = true;
    assert!(session(&d, 96.).is_ok());
    let mut d = source();
    d.sections[0].section_def.hide_header = true;
    assert!(session(&d, 96.).is_err());
}

#[test]
fn tall_saved_shape_does_not_silently_shrink_to_font_metrics() {
    // Real Hancom save of the initial 2400HU shape + 1800HU fixed pitch.
    // The shared text painter still folds this line to font metrics. Keep it
    // explicitly unsupported instead of pretending this bundle handles it.
    let d = rhwp::parse_document(include_bytes!(
        "../fixtures/issue7353_host_master_review/tall-unsupported.hwp"
    ))
    .unwrap();
    assert!(matches!(
        session(&d, 96.),
        Err(HostedTableError::Geometry(GeometryError::Unsupported(
            "shared text paint changes stored metrics"
        )))
    ));
}

#[test]
fn extracted_selector_keeps_the_existing_document_host_output() {
    let core = rhwp::document_core::DocumentCore::from_bytes(include_bytes!(
        "../fixtures/issue7353_host_master_review/saved.hwp"
    ))
    .unwrap();
    for (page, expected) in [None, Some("BASE MASTER"), Some("ODD MASTER")]
        .into_iter()
        .enumerate()
    {
        let tree = core.build_page_render_tree(page as u32).unwrap();
        let masters: Vec<_> = tree
            .root
            .children
            .iter()
            .filter(|n| matches!(n.node_type, RenderNodeType::MasterPage))
            .collect();
        assert_eq!(masters.len(), usize::from(expected.is_some()));
        if let Some(label) = expected {
            assert_eq!(text(masters[0]), label);
        }
    }
}

#[test]
fn shape_only_line_is_real_occupancy_and_floating_body_shape_is_not_admitted() {
    // Synthetic stream variation, not a second Hancom fidelity claim. The
    // same saved shape/line metrics own a control-only paragraph.
    let mut d = source();
    let p = &mut d.sections[0].paragraphs[5];
    p.text.clear();
    p.char_offsets.clear();
    p.char_count = 9;
    let s = session(&d, 96.).unwrap();
    let tree = s.render_page(1).unwrap();
    let rows = body_lines(&tree.root);
    let shape = rows.iter().find(|n| text(n) == "BOX").unwrap();
    close(shape.bbox.y, 15200. * 96. / 7200.);
    close(shape.bbox.height, 1600. * 96. / 7200.);
    close(
        rows.iter().find(|n| text(n) == "TAIL 01").unwrap().bbox.y,
        17000. * 96. / 7200.,
    );
    let Control::Shape(shape) = &mut d.sections[0].paragraphs[5].controls[0] else {
        panic!()
    };
    shape.common_mut().treat_as_char = false;
    assert!(session(&d, 96.).is_err());
}
