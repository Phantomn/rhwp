//! Product routing contracts, not new typesetting rules. The saved portrait
//! fixture has an independent Hancom PDF and documented line origins. The
//! two-section variant below is a synthetic ownership/numbering contract.
use rhwp::{
    document_core::{DocumentCore, TypesettingEngine},
    model::control::{Control, PageNumberPos},
    renderer::{
        render_tree::{RenderNode, RenderNodeType},
        table_v2::{CellEndPolicy, HostedSectionSession},
    },
};

const SAVED: &[u8] = include_bytes!("../fixtures/issue7353_body_frame_review/portrait-saved.hwp");

fn open(data: &[u8]) -> DocumentCore {
    DocumentCore::from_bytes_with_engine(data, TypesettingEngine::V2).unwrap()
}

fn text(node: &RenderNode) -> String {
    let mut out = match &node.node_type {
        RenderNodeType::TextRun(run) => run.display_text.as_ref().unwrap_or(&run.text).clone(),
        _ => String::new(),
    };
    for child in &node.children {
        out.push_str(&text(child));
    }
    out
}

fn lines<'a>(node: &'a RenderNode, out: &mut Vec<&'a RenderNode>) {
    if matches!(node.node_type, RenderNodeType::TextLine(_)) {
        out.push(node);
    }
    for child in &node.children {
        lines(child, out);
    }
}

#[test]
fn product_v2_saved_body_uses_same_geometry_for_svg_and_canvas() {
    let core = open(SAVED);
    assert_eq!(core.typesetting_engine(), TypesettingEngine::V2);
    assert_eq!(core.page_count(), 2);
    let parsed = rhwp::parse_document(SAVED).unwrap();
    assert_eq!(
        format!("{:?}", core.document().sections),
        format!("{:?}", parsed.sections),
        "V2 must not synthesize/erase saved lines through Legacy load fixups"
    );
    let preview =
        HostedSectionSession::from_document(&parsed, 0, 96., CellEndPolicy::OmitFinalParagraphGap)
            .unwrap();
    let mut evidence = Vec::new();
    for (page, expected) in [
        (0, ["BEFORE", "ALPHA", "BRAVO", "", "CHARLIE"].as_slice()),
        (1, ["DELTA", "ECHO", "AFTER"].as_slice()),
    ] {
        let tree = core.build_page_render_tree(page).unwrap();
        assert_eq!(
            serde_json::to_value(&tree).unwrap(),
            serde_json::to_value(preview.render_page(page as usize).unwrap()).unwrap()
        );
        let mut actual = Vec::new();
        lines(&tree.root, &mut actual);
        assert_eq!(actual.len(), expected.len());
        for (i, (line, label)) in actual.iter().zip(expected).enumerate() {
            assert_eq!(text(line), *label);
            assert!((line.bbox.y - (20. + i as f64 * 24.)).abs() < 1e-7);
            assert!((line.bbox.height - 16.).abs() < 1e-7);
        }
        let canvas = core
            .build_canvas_page_layer_tree_with_profile(page, rhwp::paint::RenderProfile::Screen)
            .unwrap();
        let portable = core.build_page_layer_tree(page).unwrap();
        assert_eq!(canvas.to_json(), portable.to_json());
        evidence.push(serde_json::json!({ "tree": serde_json::from_str::<serde_json::Value>(&tree.root.to_json()).unwrap(), "layer": serde_json::from_str::<serde_json::Value>(&canvas.to_json()).unwrap(), "svg": core.render_page_svg_native(page).unwrap() }));
    }
    // Optional browser evidence from the exact same tested Native product API.
    if let Some(path) = std::env::var_os("RHWP_V2_PRODUCT_EVIDENCE") {
        let path = std::path::PathBuf::from(path);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(
            path.join("native.json"),
            serde_json::to_vec_pretty(&evidence).unwrap(),
        )
        .unwrap();
        for (i, page) in evidence.iter().enumerate() {
            std::fs::write(
                path.join(format!("native-{}.svg", i + 1)),
                page["svg"].as_str().unwrap(),
            )
            .unwrap();
        }
    }
}

#[test]
fn product_v2_multisection_preserves_global_address_paper_and_source_ownership() {
    let mut core = open(SAVED);
    let mut second = core.document().sections[0].clone();
    // Same saved lanes remain valid in a wider page; no synthetic LineSeg data.
    second.section_def.page_def.width += 7500;
    second.section_def.page_num = 0;
    for p in &mut second.paragraphs {
        for c in &mut p.controls {
            if let Control::SectionDef(def) = c {
                *def = Box::new(second.section_def.clone());
            }
        }
    }
    core.document_mut().sections.push(second);
    core.repaginate_if_needed();
    core.ensure_typesetting_ready().unwrap();
    assert_eq!(core.page_count(), 4);
    let first_width = core.build_page_render_tree(0).unwrap().root.bbox.width;
    for index in 0..4 {
        let info: serde_json::Value =
            serde_json::from_str(&core.get_page_info_native(index).unwrap()).unwrap();
        assert_eq!(info["pageIndex"], index);
        assert_eq!(info["pageNumber"], index + 1);
        assert_eq!(info["sectionIndex"], index / 2);
        let tree = core.build_page_render_tree(index).unwrap();
        let RenderNodeType::Page(page) = &tree.root.node_type else {
            panic!()
        };
        assert_eq!(page.page_index, index);
        assert_eq!(page.section_index, (index / 2) as usize);
        assert!((page.width - first_width - if index >= 2 { 100. } else { 0. }).abs() < 1e-7);
        let mut body = Vec::new();
        lines(&tree.root, &mut body);
        for node in body {
            let RenderNodeType::TextLine(line) = &node.node_type else {
                unreachable!()
            };
            assert_eq!(line.section_index, Some((index / 2) as usize));
            assert!(line.para_index.is_some());
        }
    }
    assert!(core.build_page_render_tree(4).is_err());
}

#[test]
fn product_v2_rejects_unsupported_without_fallback_or_stale_cache() {
    let mut core = open(SAVED);
    core.render_page_svg_native(0).unwrap(); // warm tree + layer caches
    core.document_mut().sections[0].section_def.line_grid = 1;
    assert_eq!(core.page_count(), 0);
    assert!(core.render_page_svg_native(0).is_err());
    core.repaginate_if_needed();
    let reason = core.ensure_typesetting_ready().unwrap_err().to_string();
    assert!(
        reason.contains("V2 section 0") && reason.contains("grid"),
        "{reason}"
    );
    assert_eq!(core.page_count(), 0);
    assert!(core.build_page_layer_tree(0).is_err());
    assert_eq!(core.typesetting_engine(), TypesettingEngine::V2);
    let bytes = rhwp::serializer::serialize_hwpx(core.document()).unwrap();
    assert!(DocumentCore::from_bytes_with_engine(&bytes, TypesettingEngine::V2).is_err());
    let legacy = DocumentCore::from_bytes_with_engine(&bytes, TypesettingEngine::Legacy).unwrap();
    assert_eq!(legacy.typesetting_engine(), TypesettingEngine::Legacy);
    assert!(legacy.page_count() > 0);
}

#[test]
fn product_v2_rejects_unbound_cross_section_story_with_source_address() {
    let mut core = open(SAVED);
    let mut second = core.document().sections[0].clone();
    second.paragraphs[0]
        .controls
        .push(Control::PageNumberPos(PageNumberPos::default()));
    core.document_mut().sections.push(second);
    core.repaginate_if_needed();
    let reason = core.ensure_typesetting_ready().unwrap_err().to_string();
    assert!(
        reason.contains("section 1")
            && reason.contains("paragraph 0")
            && reason.contains("inheritance"),
        "{reason}"
    );
    assert_eq!(core.page_count(), 0, "no partially published first section");
}

#[test]
fn product_v2_split_table_preserves_fragments_and_after_origin() {
    let bytes = include_bytes!("../fixtures/issue7353_host_owner_review/split-saved.hwp");
    let core = open(bytes);
    assert_eq!(core.page_count(), 3);
    let mut evidence = Vec::new();
    let mut contents = String::new();
    for page in 0..3 {
        let tree = core.build_page_render_tree(page).unwrap();
        let actual = text(&tree.root);
        if page == 0 {
            assert_eq!(actual, "BEFORE");
        }
        if page == 1 {
            assert!(actual.contains("ROW 19"));
            assert!(!actual.contains("ROW 20"));
        }
        if page == 2 {
            assert!(actual.starts_with("ROW 20"));
            let mut body = Vec::new();
            lines(&tree.root, &mut body);
            let after = body.iter().find(|line| text(line) == "AFTER").unwrap();
            // Saved AFTER vpos8175HU + margin1500HU; independent PDF bottom
            // 96.759pt and AFTER baseline104.442pt are in fixture README.
            assert!((after.bbox.y - 129.).abs() < 1e-7);
        }
        contents.push_str(&actual);
        let canvas = core
            .build_canvas_page_layer_tree_with_profile(page, rhwp::paint::RenderProfile::Screen)
            .unwrap();
        evidence.push(serde_json::json!({ "tree": serde_json::from_str::<serde_json::Value>(&tree.root.to_json()).unwrap(), "layer": serde_json::from_str::<serde_json::Value>(&canvas.to_json()).unwrap(), "svg": core.render_page_svg_native(page).unwrap() }));
    }
    for row in 1..=25 {
        assert_eq!(contents.matches(&format!("ROW {row:02}")).count(), 1);
    }
    assert_eq!(contents.matches("AFTER").count(), 1);
    if let Some(path) = std::env::var_os("RHWP_V2_PRODUCT_EVIDENCE") {
        let path = std::path::PathBuf::from(path).join("table");
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(
            path.join("native.json"),
            serde_json::to_vec_pretty(&evidence).unwrap(),
        )
        .unwrap();
        for (i, page) in evidence.iter().enumerate() {
            std::fs::write(
                path.join(format!("native-{}.svg", i + 1)),
                page["svg"].as_str().unwrap(),
            )
            .unwrap();
        }
    }
}

#[test]
fn product_v2_cell_page_fields_are_numbered_before_fragment_paint() {
    use rhwp::model::{
        paragraph::{CharShapeRef, Paragraph},
        table::{Cell, Table},
    };
    let source =
        rhwp::parse_document(include_bytes!("../../samples/21_언어_기출_편집가능본.hwp")).unwrap();
    let Control::Table(source_table) = &source.sections[0].paragraphs[206].controls[0] else {
        panic!()
    };
    let mut field = source_table.cells[0].paragraphs[0].clone();
    let mut pstyle = source.doc_info.para_shapes[field.para_shape_id as usize].clone();
    let mut cstyle =
        source.doc_info.char_shapes[field.char_shapes[0].char_shape_id as usize].clone();
    pstyle.border_fill_id = 0;
    pstyle.tab_def_id = 0;
    pstyle.raw_data = None;
    cstyle.border_fill_id = 0;
    cstyle.raw_data = None;
    let mut core = open(SAVED);
    let doc = core.document_mut();
    field.para_shape_id = doc.doc_info.para_shapes.len() as u16;
    field.char_shapes[0].char_shape_id = doc.doc_info.char_shapes.len() as u32;
    doc.doc_info.para_shapes.push(pstyle);
    doc.doc_info.char_shapes.push(cstyle);
    let Control::AutoNumber(number) = &mut field.controls[0] else {
        panic!()
    };
    number.assigned_number = 77; // must not paint the stored cached display
    let mut table = Table {
        row_count: 1,
        col_count: 1,
        cells: vec![Cell {
            width: 10000,
            row_span: 1,
            col_span: 1,
            paragraphs: vec![field],
            ..Default::default()
        }],
        ..Default::default()
    };
    table.common.width = 10000;
    table.common.text_wrap = rhwp::model::shape::TextWrap::TopAndBottom;
    table.common.vert_rel_to = rhwp::model::shape::VertRelTo::Para;
    table.common.horz_rel_to = rhwp::model::shape::HorzRelTo::Column;
    let owner = Paragraph {
        controls: vec![Control::Table(Box::new(table))],
        char_shapes: vec![CharShapeRef {
            start_pos: 0,
            char_shape_id: 0,
        }],
        ..Default::default()
    };
    doc.sections[0].paragraphs = vec![owner];
    doc.sections[0].section_def.page_num = 0;
    doc.sections.push(doc.sections[0].clone());
    core.repaginate_if_needed();
    core.ensure_typesetting_ready().unwrap();
    assert_eq!(core.page_count(), 2);
    assert_eq!(
        text(&core.build_page_render_tree(0).unwrap().root).trim(),
        "1"
    );
    assert_eq!(
        text(&core.build_page_render_tree(1).unwrap().root).trim(),
        "2"
    );
    // Explicit restart does not change the global page address.
    core.document_mut().sections[1].section_def.page_num = 1;
    core.repaginate_if_needed();
    let restarted = core.build_page_render_tree(1).unwrap();
    assert_eq!(text(&restarted.root).trim(), "1");
    let RenderNodeType::Page(page) = restarted.root.node_type else {
        panic!()
    };
    assert_eq!(page.page_index, 1);
}
