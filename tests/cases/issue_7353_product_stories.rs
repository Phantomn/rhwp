//! U1 product stories. Saved #6864 has an independent Hancom PDF; multi-section
//! variants are explicit synthetic contracts, not claimed Hancom fixtures.
use rhwp::{
    document_core::{DocumentCore, TypesettingEngine},
    model::{
        control::{Control, PageNumberPos},
        header_footer::{Header, HeaderFooterApply},
    },
    renderer::render_tree::{RenderNode, RenderNodeType},
};

const BODY: &[u8] = include_bytes!("../fixtures/issue7353_body_frame_review/portrait-saved.hwp");
fn text(node: &RenderNode) -> String {
    let mut s = match &node.node_type {
        RenderNodeType::TextRun(run) => {
            run.display_text.clone().unwrap_or_else(|| run.text.clone())
        }
        _ => String::new(),
    };
    for child in &node.children {
        s.push_str(&text(child));
    }
    s
}
fn open(source: &rhwp::model::document::Document) -> DocumentCore {
    let bytes = rhwp::serializer::serialize_hwpx(source).unwrap();
    DocumentCore::from_bytes_with_engine(&bytes, TypesettingEngine::V2).unwrap()
}

// Authored IR: no saved layout metrics are fabricated. Explicit leading control
// slots and subsequent UTF-16 text offsets describe the serialized stream.
fn entry(p: &mut rhwp::model::paragraph::Paragraph) {
    let mut offset = p.controls.len() as u32 * 8;
    p.char_offsets = p
        .text
        .chars()
        .map(|c| {
            let start = offset;
            offset += c.len_utf16() as u32;
            start
        })
        .collect();
    p.char_count = offset + 1;
    p.line_segs.clear();
    p.invalidate_layout_inputs();
}

fn numbered_source() -> rhwp::model::document::Document {
    let mut source = rhwp::parse_document(BODY).unwrap();
    source.sections[0].section_def.page_num = 7;
    source.sections[0].paragraphs[0]
        .controls
        .push(Control::PageNumberPos(PageNumberPos {
            position: 5,
            ..Default::default()
        }));
    entry(&mut source.sections[0].paragraphs[0]);
    let mut next = source.sections[0].clone();
    next.section_def.page_num = 0;
    next.paragraphs[0]
        .controls
        .retain(|c| !matches!(c, Control::PageNumberPos(_)));
    entry(&mut next.paragraphs[0]);
    source.sections.push(next);
    source
}

fn header_source() -> rhwp::model::document::Document {
    let mut source = numbered_source();
    let mut paragraph =
        rhwp::model::paragraph::Paragraph::new_empty_like(&source.sections[0].paragraphs[0]);
    paragraph.insert_text_at(0, "U1 HEADER");
    paragraph.line_segs.clear();
    source.sections[0].paragraphs[0]
        .controls
        .push(Control::Header(Box::new(Header {
            apply_to: HeaderFooterApply::Both,
            paragraphs: vec![paragraph],
            ..Default::default()
        })));
    entry(&mut source.sections[0].paragraphs[0]);
    source
}

#[test]
fn u1_saved_header_is_painted_with_source_identity() {
    let bytes = include_bytes!("../../samples/issue6864/header-justify.hwp");
    let core = DocumentCore::from_bytes_with_engine(bytes, TypesettingEngine::V2).unwrap();
    let tree = core.build_page_render_tree(0).unwrap();
    let header = tree
        .root
        .children
        .iter()
        .find(|n| matches!(n.node_type, RenderNodeType::Header))
        .unwrap();
    assert_eq!(text(header), "TEST TEXT");
    assert!(header.header_footer_source.is_some());
    let runs: Vec<_> = header.children.iter().flat_map(|n| &n.children).collect();
    let left = runs.iter().map(|n| n.bbox.x).fold(f64::INFINITY, f64::min);
    let right = runs
        .iter()
        .map(|n| n.bbox.x + n.bbox.width)
        .fold(f64::NEG_INFINITY, f64::max);
    // Corresponding Hancom PDF: TEST starts at x=85.033915pt (pdftotext -bbox).
    assert!((left - 85.033915 * 96. / 72.).abs() < 0.05);
    assert!(
        right > left && right - left < 200.0,
        "Justify final line must not stretch"
    );
}

#[test]
fn u1_normal_hancom_sections_keep_numbering_story_origins_and_body_space() {
    let bytes = include_bytes!("../fixtures/issue7353_product_stories/stories-saved.hwp");
    let core = DocumentCore::from_bytes_with_engine(bytes, TypesettingEngine::V2).unwrap();
    assert_eq!(core.page_count(), 6);
    for (index, number) in [7, 8, 9, 10, 3, 4].into_iter().enumerate() {
        let tree = core.build_page_render_tree(index as u32).unwrap();
        let info: serde_json::Value =
            serde_json::from_str(&core.get_page_info_native(index as u32).unwrap()).unwrap();
        assert_eq!(info["pageNumber"], number);
        let header = tree
            .root
            .children
            .iter()
            .find(|n| matches!(n.node_type, RenderNodeType::Header));
        assert_eq!(header.is_some(), index != 4);
        if let Some(header) = header {
            assert_eq!(text(header), "U1 HEADER");
            assert!((header.children[0].bbox.y - 20.).abs() < 1e-7);
            assert_eq!(header.header_footer_source.as_ref().unwrap().0, 0);
        }
        let footer = tree
            .root
            .children
            .iter()
            .find(|n| matches!(n.node_type, RenderNodeType::Footer))
            .unwrap();
        assert_eq!(text(footer), if number % 2 == 1 { "ODD" } else { "EVEN" });
        assert!((footer.children[0].bbox.y - 160.).abs() < 1e-7);
        // Body origin = (1500HU top + 1500HU header) / 75HU per px.
        let body = tree
            .root
            .children
            .iter()
            .find(|n| matches!(n.node_type, RenderNodeType::Body { .. }))
            .unwrap();
        let first_line = body
            .children
            .iter()
            .flat_map(|n| &n.children)
            .find(|n| matches!(n.node_type, RenderNodeType::TextLine(_)))
            .unwrap();
        assert!((first_line.bbox.y - 40.).abs() < 1e-7);
    }
}

#[test]
fn u1_explicit_number_origin_continues_without_double_addition() {
    let source = numbered_source();
    let core = open(&source);
    assert_eq!(core.page_count(), 4);
    for index in 0..4 {
        let tree = core.build_page_render_tree(index).unwrap();
        let info: serde_json::Value =
            serde_json::from_str(&core.get_page_info_native(index).unwrap()).unwrap();
        assert_eq!(info["pageNumber"], index + 7);
        assert!(text(&tree.root).contains(&(index + 7).to_string()));
    }
}

#[test]
fn u1_plain_header_inherits_and_retains_original_section() {
    let source = header_source();
    let core = open(&source);
    for index in 0..4 {
        let tree = core.build_page_render_tree(index).unwrap();
        let header = tree
            .root
            .children
            .iter()
            .find(|n| matches!(n.node_type, RenderNodeType::Header))
            .unwrap();
        assert_eq!(text(header), "U1 HEADER");
        assert_eq!(header.header_footer_source.as_ref().unwrap().0, 0);
    }
}

#[test]
fn u1_reset_and_first_page_hide_do_not_erase_inheritance() {
    let mut source = header_source();
    source.sections[1].section_def.page_num = 3;
    source.sections[1].section_def.hide_header = true;
    let core = open(&source);
    for (index, number) in [7, 8, 3, 4].into_iter().enumerate() {
        let tree = core.build_page_render_tree(index as u32).unwrap();
        assert!(text(&tree.root).contains(&number.to_string()));
        assert_eq!(text(&tree.root).contains("U1 HEADER"), index != 2);
    }
}

#[test]
fn u1_header_edit_selection_and_reopen_use_v2_geometry() {
    let source = header_source();
    let mut core = open(&source);
    let undo = core.save_snapshot_native();
    let before = core.render_page_svg_native(3).unwrap();
    core.insert_text_in_header_footer_native(0, true, 0, 0, 3, "EDIT ")
        .unwrap();
    for index in 0..4 {
        assert!(text(&core.build_page_render_tree(index).unwrap().root).contains("U1 EDIT HEADER"));
    }
    let rects: serde_json::Value = serde_json::from_str(
        &core
            .get_selection_rects_in_header_footer_native(0, true, 0, 0, 0, 0, 0, 2)
            .unwrap(),
    )
    .unwrap();
    assert!(rects.as_array().is_some_and(|r| !r.is_empty()), "{rects}");
    let reopened = DocumentCore::from_bytes_with_engine(
        &core.export_hwp_native().unwrap(),
        TypesettingEngine::V2,
    )
    .unwrap();
    assert!(text(&reopened.build_page_render_tree(3).unwrap().root).contains("U1 EDIT HEADER"));
    let edited = core.render_page_svg_native(3).unwrap();
    let redo = core.save_snapshot_native();
    core.restore_snapshot_native(undo).unwrap();
    assert_eq!(core.render_page_svg_native(3).unwrap(), before);
    core.restore_snapshot_native(redo).unwrap();
    assert_eq!(core.render_page_svg_native(3).unwrap(), edited);
}

#[test]
fn u1_odd_even_selection_and_inactive_preview_share_composition() {
    let mut source = header_source();
    for (apply, label) in [
        (HeaderFooterApply::Odd, "ODD"),
        (HeaderFooterApply::Even, "EVEN"),
    ] {
        let mut p =
            rhwp::model::paragraph::Paragraph::new_empty_like(&source.sections[0].paragraphs[0]);
        p.insert_text_at(0, label);
        p.line_segs.clear();
        source.sections[0].paragraphs[0]
            .controls
            .push(Control::Header(Box::new(Header {
                apply_to: apply,
                paragraphs: vec![p],
                ..Default::default()
            })));
    }
    entry(&mut source.sections[0].paragraphs[0]);
    let core = open(&source);
    for index in 0..4 {
        let value = text(&core.build_page_render_tree(index).unwrap().root);
        assert!(
            value.contains(if index % 2 == 0 { "ODD" } else { "EVEN" }),
            "{value}"
        );
        assert!(!value.contains("U1 HEADER"));
    }
    // Even target is not painted on printed page 7; the editor still needs its
    // representative projection there, with source ownership and nonempty ink.
    let rects: serde_json::Value = serde_json::from_str(
        &core
            .get_selection_rects_in_header_footer_native(0, true, 1, 0, 0, 0, 0, 4)
            .unwrap(),
    )
    .unwrap();
    assert!(rects.as_array().is_some_and(|r| !r.is_empty()), "{rects}");
}

#[test]
fn u1_late_header_declaration_does_not_activate_on_previous_page() {
    let mut source = rhwp::parse_document(BODY).unwrap();
    let mut p =
        rhwp::model::paragraph::Paragraph::new_empty_like(&source.sections[0].paragraphs[0]);
    p.insert_text_at(0, "LATE");
    p.line_segs.clear();
    source.sections[0].paragraphs[2]
        .controls
        .push(Control::Header(Box::new(Header {
            paragraphs: vec![p],
            ..Default::default()
        })));
    entry(&mut source.sections[0].paragraphs[2]);
    let core = open(&source);
    assert_eq!(core.page_count(), 2);
    assert!(!text(&core.build_page_render_tree(0).unwrap().root).contains("LATE"));
    assert!(text(&core.build_page_render_tree(1).unwrap().root).contains("LATE"));
}

#[test]
fn u1_footer_page_field_uses_resolved_number_not_cached_value() {
    use rhwp::model::{
        control::{AutoNumber, AutoNumberType},
        header_footer::Footer,
    };
    let mut source = header_source();
    let mut field =
        rhwp::model::paragraph::Paragraph::new_empty_like(&source.sections[0].paragraphs[0]);
    field.text = " ".into();
    // The visible blank is the auto-number's placeholder at its control start,
    // not an additional trailing space after the eight-unit control.
    field.char_offsets = vec![0];
    field.char_count = 9;
    field.line_segs.clear();
    field.controls.push(Control::AutoNumber(AutoNumber {
        number_type: AutoNumberType::Page,
        assigned_number: 77,
        ..Default::default()
    }));
    source.sections[0].paragraphs[0]
        .controls
        .push(Control::Footer(Box::new(Footer {
            paragraphs: vec![field],
            ..Default::default()
        })));
    entry(&mut source.sections[0].paragraphs[0]);
    let core = open(&source);
    for index in 0..4 {
        let tree = core.build_page_render_tree(index).unwrap();
        let footer = tree
            .root
            .children
            .iter()
            .find(|n| matches!(n.node_type, RenderNodeType::Footer))
            .unwrap();
        assert_eq!(text(footer), (index + 7).to_string());
        assert_eq!(footer.header_footer_source.as_ref().unwrap().0, 0);
    }
}

#[test]
fn u1_empty_header_paragraph_keeps_its_line_occupancy() {
    let mut source = header_source();
    let Control::Header(h) = source.sections[0].paragraphs[0]
        .controls
        .last_mut()
        .unwrap()
    else {
        panic!()
    };
    let template = h.paragraphs[0].clone();
    h.paragraphs = ["FIRST", "", "LAST"]
        .iter()
        .map(|label| {
            let mut p = rhwp::model::paragraph::Paragraph::new_empty_like(&template);
            p.insert_text_at(0, label);
            p.line_segs.clear();
            p
        })
        .collect();
    let core = open(&source);
    let tree = core.build_page_render_tree(0).unwrap();
    let header = tree
        .root
        .children
        .iter()
        .find(|n| matches!(n.node_type, RenderNodeType::Header))
        .unwrap();
    let lines: Vec<_> = header
        .children
        .iter()
        .filter(|n| matches!(n.node_type, RenderNodeType::TextLine(_)))
        .collect();
    assert_eq!(lines.len(), 3);
    // The independent portrait fixture defines 1200HU glyphs, 1800HU advance.
    // The intentional blank paragraph contributes the same 24px line advance.
    assert!((lines[1].bbox.y - lines[0].bbox.y - 24.).abs() < 1e-7);
    assert!((lines[2].bbox.y - lines[0].bbox.y - 48.).abs() < 1e-7);
    assert!(lines[1].bbox.height > 0.);
}

#[test]
fn u1_emit_authored_fixture_when_requested() {
    if let Ok(dir) = std::env::var("RHWP_U1_OUTPUT") {
        let mut source = header_source();
        // Three small sections: 7/8, continue 9/10, restart 3/4. Header is
        // inherited; the third section hides only its first header. Footer
        // labels expose odd/even selection in the independent Hancom output.
        let mut third = source.sections[1].clone();
        third.section_def.page_num = 3;
        third.section_def.hide_header = true;
        source.sections.push(third);
        for section in &mut source.sections {
            // Keep the original 9000HU body capacity, but give page stories
            // their own visible bands instead of a zero-height header margin.
            section.section_def.page_def.margin_header = 1500;
            section.section_def.page_def.margin_footer = 1500;
            section.section_def.page_def.margin_bottom = 16500;
        }
        for (apply, label) in [
            (HeaderFooterApply::Odd, "ODD"),
            (HeaderFooterApply::Even, "EVEN"),
        ] {
            let mut p = rhwp::model::paragraph::Paragraph::new_empty_like(
                &source.sections[0].paragraphs[0],
            );
            p.insert_text_at(0, label);
            p.line_segs.clear();
            source.sections[0].paragraphs[0]
                .controls
                .push(Control::Footer(Box::new(
                    rhwp::model::header_footer::Footer {
                        apply_to: apply,
                        paragraphs: vec![p],
                        ..Default::default()
                    },
                )));
        }
        entry(&mut source.sections[0].paragraphs[0]);
        std::fs::write(
            std::path::Path::new(&dir).join("stories-input.hwpx"),
            rhwp::serializer::serialize_hwpx(&source).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn u1_product_native_evidence_when_requested() {
    let (Ok(input), Ok(output)) = (
        std::env::var("RHWP_U1_INPUT"),
        std::env::var("RHWP_U1_EVIDENCE"),
    ) else {
        return;
    };
    let bytes = std::fs::read(input).unwrap();
    let core = DocumentCore::from_bytes_with_engine(&bytes, TypesettingEngine::V2).unwrap();
    let path = std::path::Path::new(&output);
    std::fs::create_dir_all(path).unwrap();
    std::fs::write(
        path.join("source-ir.txt"),
        format!("{:#?}", core.document()),
    )
    .unwrap();
    let mut evidence = Vec::new();
    for page in 0..core.page_count() {
        let tree = core.build_page_render_tree(page).unwrap();
        let canvas = core
            .build_canvas_page_layer_tree_with_profile(page, rhwp::paint::RenderProfile::Screen)
            .unwrap();
        let svg = core.render_page_svg_native(page).unwrap();
        std::fs::write(path.join(format!("native-{}.svg", page + 1)), &svg).unwrap();
        evidence.push(serde_json::json!({"tree":serde_json::from_str::<serde_json::Value>(&tree.root.to_json()).unwrap(),"layer":serde_json::from_str::<serde_json::Value>(&canvas.to_json()).unwrap(),"svg":svg}));
    }
    std::fs::write(
        path.join("native.json"),
        serde_json::to_vec_pretty(&evidence).unwrap(),
    )
    .unwrap();
}
