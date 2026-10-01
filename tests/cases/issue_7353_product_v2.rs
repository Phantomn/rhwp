//! Product routing contracts, not new typesetting rules. The saved portrait
//! fixture has an independent Hancom PDF and documented line origins. The
//! two-section variant below is a synthetic ownership/numbering contract.
use rhwp::{
    document_core::{DocumentCore, TypesettingEngine},
    model::control::Control,
    renderer::{
        render_tree::{RenderNode, RenderNodeType},
        table_v2::{CellEndPolicy, HostedSectionSession},
    },
};

const SAVED: &[u8] = include_bytes!("../fixtures/issue7353_body_frame_review/portrait-saved.hwp");

#[test]
fn product_v2_page_and_shape_paint_do_not_dispatch_to_legacy_engine() {
    // Architectural guard accompanies the geometry/edit/continuation contracts
    // below; it does not substitute for their actual output assertions.
    for source in [
        include_str!("../../src/renderer/table_v2/host_page.rs"),
        include_str!("../../src/renderer/table_v2/host_section.rs"),
        include_str!("../../src/renderer/table_v2/host_flow.rs"),
        include_str!("../../src/renderer/table_v2/host_flow_state.rs"),
        include_str!("../../src/renderer/table_v2/host_master.rs"),
        include_str!("../../src/renderer/table_v2/shapes.rs"),
    ] {
        assert!(!source.contains("LayoutEngine::"));
        assert!(!source.contains("layout::table_layout"));
        assert!(!source.contains(".layout_table("));
        assert!(!source.contains("TypesetEngine"));
        assert!(!source.contains("TypesetState"));
    }
}

fn open(data: &[u8]) -> DocumentCore {
    DocumentCore::from_bytes_with_engine(data, TypesettingEngine::V2).unwrap()
}

#[test]
fn product_v2_blank_replacement_and_password_open_keep_engine() {
    let mut core = open(SAVED);
    core.create_blank_document_native().unwrap();
    assert_eq!(core.typesetting_engine(), TypesettingEngine::V2);
    assert_eq!(core.page_count(), 1);
    core.insert_text_native(0, 0, 0, "NEW").unwrap();
    assert!(text(&core.build_page_render_tree(0).unwrap().root).contains("NEW"));
    let encrypted = core
        .prepare_hwp_export_snapshot()
        .serialize_with_password(b"w3-test")
        .unwrap();
    let reopened = DocumentCore::from_bytes_with_password_and_engine(
        &encrypted,
        b"w3-test",
        TypesettingEngine::V2,
    )
    .unwrap();
    assert_eq!(reopened.typesetting_engine(), TypesettingEngine::V2);
    assert!(text(&reopened.build_page_render_tree(0).unwrap().root).contains("NEW"));
    assert!(DocumentCore::from_bytes_with_password_and_engine(
        &encrypted,
        b"wrong",
        TypesettingEngine::V2,
    )
    .is_err());
    assert_eq!(
        DocumentCore::from_bytes(SAVED)
            .unwrap()
            .typesetting_engine(),
        TypesettingEngine::Legacy
    );
}

#[test]
fn product_v2_local_keyboard_edit_publishes_current_generation() {
    let mut core = open(SAVED);
    let before = core.render_page_svg_native(0).unwrap();
    let result: serde_json::Value = serde_json::from_str(
        &core
            .replace_body_text_local_native(0, 0, 0, 0, "E")
            .unwrap(),
    )
    .unwrap();
    assert_eq!(result["documentPaginationPending"], false);
    assert!(text(&core.build_page_render_tree(0).unwrap().root).contains("EBEFORE"));
    core.replace_body_text_local_native(0, 0, 0, 1, "").unwrap();
    assert_eq!(core.render_page_svg_native(0).unwrap(), before);
}

#[test]
fn product_v2_edit_body_snapshot_and_save_keep_engine() {
    let mut core = open(SAVED);
    let before = core.render_page_svg_native(0).unwrap();
    let undo = core.save_snapshot_native();
    core.insert_text_native(0, 0, 0, "EDIT ").unwrap();
    core.ensure_typesetting_ready().unwrap();
    assert!(text(&core.build_page_render_tree(0).unwrap().root).contains("EDIT BEFORE"));
    let edited = core.render_page_svg_native(0).unwrap();
    let redo = core.save_snapshot_native();
    core.restore_snapshot_native(undo).unwrap();
    assert_eq!(core.render_page_svg_native(0).unwrap(), before);
    core.restore_snapshot_native(redo).unwrap();
    assert_eq!(core.render_page_svg_native(0).unwrap(), edited);
    assert_eq!(core.typesetting_engine(), TypesettingEngine::V2);
    let reopened = open(&core.export_hwp_native().unwrap());
    assert!(text(&reopened.build_page_render_tree(0).unwrap().root).contains("EDIT BEFORE"));
}

#[test]
fn product_v2_deferred_cell_edit_hides_old_generation() {
    let mut core = open(include_bytes!(
        "../fixtures/issue7353_host_owner_review/split-saved.hwp"
    ));
    let pi = core.document().sections[0]
        .paragraphs
        .iter()
        .position(|p| p.controls.iter().any(|c| matches!(c, Control::Table(_))))
        .unwrap();
    let ci = core.document().sections[0].paragraphs[pi]
        .controls
        .iter()
        .position(|c| matches!(c, Control::Table(_)))
        .unwrap();
    core.render_page_svg_native(1).unwrap();
    core.insert_text_in_cell_native_deferred_pagination(0, pi, ci, 0, 0, 0, "EDIT ")
        .unwrap();
    assert!(
        core.render_page_svg_native(1).is_err(),
        "unpublished edit must not return old V2 packet"
    );
    core.repaginate_if_needed();
    core.ensure_typesetting_ready().unwrap();
    assert!(text(&core.build_page_render_tree(1).unwrap().root).contains("EDIT ROW"));
}

#[test]
fn product_v2_failed_batch_is_not_reported_as_success() {
    let mut core = open(SAVED);
    let undo = core.save_snapshot_native();
    core.begin_batch_native().unwrap();
    core.document_mut().sections[0].section_def.line_grid = 1;
    assert!(
        core.end_batch_native().is_err(),
        "V2 admission failure must propagate from commit barrier"
    );
    assert_eq!(core.page_count(), 0);
    assert!(core.render_page_svg_native(0).is_err());
    core.restore_snapshot_native(undo).unwrap();
    core.ensure_typesetting_ready().unwrap();
    assert_eq!(core.page_count(), 2);
}

#[test]
fn product_v2_font_environment_and_dpi_invalidate_body_and_table() {
    for data in [
        SAVED,
        include_bytes!("../fixtures/issue7353_host_owner_review/split-saved.hwp").as_slice(),
    ] {
        let mut core = open(data);
        let original = core.render_page_svg_native(0).unwrap();
        let mut replacements = serde_json::Map::new();
        for font in core.document().doc_info.font_faces.iter().flatten() {
            replacements.insert(font.name.clone(), serde_json::json!("Courier New"));
        }
        let environment = rhwp::renderer::font_environment::FontEnvironment::from_json(
            &serde_json::json!({"id":"w2-font-test", "substitutions":replacements}).to_string(),
        )
        .unwrap();
        core.set_font_environment(Some(environment)).unwrap();
        for page in 0..core.page_count() {
            assert!(core
                .render_page_svg_native(page)
                .unwrap()
                .contains("Courier New"));
        }
        core.set_font_environment(None).unwrap();
        assert_eq!(core.render_page_svg_native(0).unwrap(), original);
        let width = core.build_page_render_tree(0).unwrap().root.bbox.width;
        core.set_dpi(144.);
        core.ensure_typesetting_ready().unwrap();
        assert_eq!(
            core.build_page_render_tree(0).unwrap().root.bbox.width,
            width * 1.5
        );
        core.set_dpi(96.);
        assert_eq!(core.render_page_svg_native(0).unwrap(), original);
    }
}

#[test]
fn product_v2_edit_cell_cursor_tracks_fragment() {
    let mut core = open(include_bytes!(
        "../fixtures/issue7353_host_owner_review/split-saved.hwp"
    ));
    let pi = core.document().sections[0]
        .paragraphs
        .iter()
        .position(|p| p.controls.iter().any(|c| matches!(c, Control::Table(_))))
        .unwrap();
    let ci = core.document().sections[0].paragraphs[pi]
        .controls
        .iter()
        .position(|c| matches!(c, Control::Table(_)))
        .unwrap();
    let undo = core.save_snapshot_native();
    let before = core.render_page_svg_native(1).unwrap();
    core.insert_text_in_cell_native(0, pi, ci, 0, 0, 0, "EDIT ")
        .unwrap();
    core.ensure_typesetting_ready().unwrap();
    assert!(text(&core.build_page_render_tree(1).unwrap().root).contains("EDIT ROW"));
    let cursor: serde_json::Value = serde_json::from_str(
        &core
            .get_cursor_rect_in_cell_native(0, pi, ci, 0, 0, 0)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(cursor["pageIndex"], 1, "{cursor}");
    // Saved fixture uses 9pt glyphs at 96dpi, not the 360px fragment box.
    assert_eq!(cursor["height"], 12., "{cursor}");
    assert_eq!(
        cursor["x"], 25.,
        "20px page origin + 5px cell padding: {cursor}"
    );
    let hit = core
        .hit_test_native(
            1,
            cursor["x"].as_f64().unwrap() + 1.,
            cursor["y"].as_f64().unwrap() + 1.,
        )
        .unwrap();
    eprintln!("cell cursor={cursor} hit={hit}");
    let hit: serde_json::Value = serde_json::from_str(&hit).unwrap();
    assert_eq!(hit["cellIndex"], 0, "{hit}");
    core.restore_snapshot_native(undo).unwrap();
    assert_eq!(core.render_page_svg_native(1).unwrap(), before);
}

#[test]
fn product_v2_cell_growth_reflows_and_restores_complete_units() {
    let mut core = open(include_bytes!(
        "../fixtures/issue7353_host_owner_review/split-saved.hwp"
    ));
    let undo = core.save_snapshot_native();
    let before: Vec<_> = (0..core.page_count())
        .map(|p| core.render_page_svg_native(p).unwrap())
        .collect();
    let Control::Table(table) = &core.document().sections[0].paragraphs[1].controls[0] else {
        panic!()
    };
    let end = table.cells[0].paragraphs[0].text.chars().count();
    let added = (0..30)
        .map(|i| format!("\nADDED{i:02}"))
        .collect::<String>();
    core.insert_text_in_cell_native(0, 1, 0, 0, 0, end, &added)
        .unwrap();
    core.ensure_typesetting_ready().unwrap();
    assert!(core.page_count() > before.len() as u32);
    let all = (0..core.page_count())
        .map(|p| text(&core.build_page_render_tree(p).unwrap().root))
        .collect::<String>();
    for i in 0..30 {
        assert_eq!(all.matches(&format!("ADDED{i:02}")).count(), 1);
    }
    // Page ownership comes from the actual painted marker, not a fixed page
    // count or Legacy continuation estimate. 9pt at 96dpi remains 12px.
    let marker_page = (0..core.page_count())
        .find(|p| text(&core.build_page_render_tree(*p).unwrap().root).contains("ADDED29"))
        .unwrap();
    let cursor: serde_json::Value = serde_json::from_str(
        &core
            .get_cursor_rect_in_cell_native(0, 1, 0, 0, 0, end + added.chars().count() - 1)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(cursor["pageIndex"], marker_page);
    assert_eq!(cursor["height"], 12.);
    let redo = core.save_snapshot_native();
    let after: Vec<_> = (0..core.page_count())
        .map(|p| core.render_page_svg_native(p).unwrap())
        .collect();
    core.restore_snapshot_native(undo).unwrap();
    assert_eq!(
        (0..core.page_count())
            .map(|p| core.render_page_svg_native(p).unwrap())
            .collect::<Vec<_>>(),
        before
    );
    core.restore_snapshot_native(redo).unwrap();
    assert_eq!(
        (0..core.page_count())
            .map(|p| core.render_page_svg_native(p).unwrap())
            .collect::<Vec<_>>(),
        after
    );
}

#[test]
fn product_v2_cell_enter_preserves_new_paragraph_query_owner() {
    let mut core = open(include_bytes!(
        "../fixtures/issue7353_host_owner_review/split-saved.hwp"
    ));
    core.insert_text_in_cell_native(0, 1, 0, 0, 0, 0, "EDIT ")
        .unwrap();
    let snapshot = core.save_snapshot_native();
    let before = core.render_page_svg_native(1).unwrap();
    let count = core.get_cell_paragraph_count_native(0, 1, 0, 0).unwrap();
    core.split_paragraph_in_cell_native(0, 1, 0, 0, 0, 5, None)
        .unwrap();
    assert_eq!(
        core.get_cell_paragraph_count_native(0, 1, 0, 0).unwrap(),
        count + 1
    );
    let cursor: serde_json::Value = serde_json::from_str(
        &core
            .get_cursor_rect_in_cell_native(0, 1, 0, 0, 1, 0)
            .unwrap(),
    )
    .unwrap();
    let first: serde_json::Value = serde_json::from_str(
        &core
            .get_cursor_rect_in_cell_native(0, 1, 0, 0, 0, 0)
            .unwrap(),
    )
    .unwrap();
    assert!(cursor["y"].as_f64().unwrap() > first["y"].as_f64().unwrap());
    let hit: serde_json::Value = serde_json::from_str(
        &core
            .hit_test_native(
                cursor["pageIndex"].as_u64().unwrap() as u32,
                cursor["x"].as_f64().unwrap() + 1.,
                cursor["y"].as_f64().unwrap() + 1.,
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(hit["parentParaIndex"], 1);
    assert_eq!(hit["cellIndex"], 0);
    assert_eq!(hit["cellParaIndex"], 1);
    core.restore_snapshot_native(snapshot).unwrap();
    assert_eq!(core.render_page_svg_native(1).unwrap(), before);
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
                **def = second.section_def.clone();
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
fn product_v2_rejects_unbound_cross_section_number_restart_with_source_address() {
    let mut core = open(SAVED);
    let mut second = core.document().sections[0].clone();
    second.paragraphs[0]
        .controls
        // Section start/continue and PageNumberPos are now supported by U1.
        // An in-body NewNumber remains a distinct, unbound timeline control.
        .push(Control::NewNumber(Default::default()));
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
