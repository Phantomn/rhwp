//! Actual TypesetEngine -> PageContent -> LayoutEngine contracts. Fresh IR:
//! 9pt text occupies12px; fixed2700HU advances18px; padding225/300HU=3/4px.
//! 480px paper,20px side margins,16px gutter -> two212px columns. Body40px.
//! Fresh IR assertions are synthetic contracts. Stored-text tests separately use
//! the normal Hancom save and matching PDF in issue7353_body_frame_review.
use rhwp::{
    model::{
        control::Control,
        document::{Document, Section},
        page::{ColumnDef, PageDef},
        paragraph::{CharShapeRef, Paragraph},
        shape::{CommonObjAttr, HorzRelTo, TextWrap, VertRelTo},
        style::{CharShape, LineSpacingType, ParaShape},
        table::{Cell, Table, TablePageBreak},
        Padding,
    },
    renderer::{
        pagination::PageItem,
        render_tree::{RenderNode, RenderNodeType},
        table_v2::*,
    },
};

fn paragraph(text: &str) -> Paragraph {
    Paragraph {
        text: text.into(),
        char_count: text.len() as u32,
        char_offsets: (0..text.len() as u32).collect(),
        char_shapes: vec![CharShapeRef {
            start_pos: 0,
            char_shape_id: 0,
        }],
        ..Default::default()
    }
}

#[test]
fn hosted_fresh_tac_preserves_oversize_budget_and_following_paragraph() {
    // Synthetic HWP contracts, not the separately converted HWPX/PDF fixtures.
    for (name, bytes, count) in [
        (
            "first",
            include_bytes!("../fixtures/issue7353_oversize_tac_review/first.hwp").as_slice(),
            2,
        ),
        (
            "grown",
            include_bytes!("../fixtures/issue7353_oversize_tac_review/grown.hwp").as_slice(),
            2,
        ),
        (
            "preceded",
            include_bytes!("../fixtures/issue7353_oversize_tac_review/preceded.hwp").as_slice(),
            3,
        ),
    ] {
        let mut doc = rhwp::parse_document(bytes).unwrap();
        // This budget-only contract selects the currently supported normal
        // column host; the HWPX/PDF fixtures are not changed or reused as oracle.
        for p in &mut doc.sections[0].paragraphs {
            for c in &mut p.controls {
                if let Control::ColumnDef(c) = c {
                    *c = ColumnDef {
                        column_count: 1,
                        same_width: true,
                        ..Default::default()
                    };
                }
            }
        }
        let session =
            HostedSectionSession::from_document(&doc, 0, 96., CellEndPolicy::OmitFinalParagraphGap)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(session.pagination().pages.len(), count, "{name}");
        let mut all_tables = 0;
        for index in 0..count {
            all_tables += table_nodes(&session.render_page(index).unwrap().root).len();
        }
        assert_eq!(all_tables, 1, "{name}: intact table once");
        let last = session.render_page(count - 1).unwrap();
        assert!(text(&last.root).contains("AFTER TABLES"));
        assert!(table_nodes(&last.root).is_empty());
    }
}

#[test]
fn hosted_normal_saved_sales_tac_keeps_cells_and_after_source() {
    let bytes = include_bytes!("../fixtures/issue7353_tac_overflow_review/sales-saved.hwp");
    let doc = rhwp::parse_document(bytes).unwrap();
    let snapshot = format!("{:?}", doc.sections[0].paragraphs);
    let s = HostedSectionSession::from_document(&doc, 0, 96., CellEndPolicy::OmitFinalParagraphGap)
        .unwrap();
    assert_eq!(s.pagination().pages.len(), 1);
    let tree = s.render_page(0).unwrap();
    assert_eq!(table_nodes(&tree.root).len(), 1);
    assert!(text(&tree.root).contains("100.0"));
    let tail = lines(&tree.root)
        .into_iter()
        .find(|n| text(n).contains("자료출처"))
        .unwrap();
    let t = table_nodes(&tree.root)[0];
    assert!(tail.bbox.y >= t.bbox.y + t.bbox.height);
    // Independent same-HWP PDF path bounds at72dpi, including stroke extent.
    // Subpixel print quantization/stroke differences are not layout tolerances.
    for (actual, pt) in [
        (t.bbox.x, 57.809),
        (t.bbox.y, 106.563),
        (t.bbox.x + t.bbox.width, 538.870),
        (t.bbox.y + t.bbox.height, 323.406),
    ] {
        assert!((actual - pt * 96. / 72.).abs() < 0.6);
    }
    assert_eq!(format!("{:?}", doc.sections[0].paragraphs), snapshot);
}

#[test]
fn inline_host_rejects_unbound_page_story_and_mixed_floating_control() {
    let bytes = include_bytes!("../fixtures/issue7353_tac_overflow_review/sales-saved.hwp");
    let mut d = rhwp::parse_document(bytes).unwrap();
    let Control::Table(t) = &mut d.sections[0].paragraphs[2].controls[0] else {
        panic!()
    };
    t.cells[0].paragraphs[0]
        .controls
        .push(Control::PageNumberPos(Default::default()));
    assert!(matches!(
        HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()),
        Err(HostedTableError::UnsupportedHost(
            "inline table page-number story requires host timeline"
        ))
    ));
    let mut d = rhwp::parse_document(bytes).unwrap();
    let Control::Table(mut floating) = d.sections[0].paragraphs[2].controls[0].clone() else {
        panic!()
    };
    floating.common.treat_as_char = false;
    d.sections[0].paragraphs[2]
        .controls
        .push(Control::Table(floating));
    assert!(matches!(
        HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()),
        Err(HostedTableError::UnsupportedHost(_))
    ));
}

#[test]
fn inline_host_does_not_erase_unresolved_paragraph_border_effects() {
    let bytes = include_bytes!("../fixtures/issue7353_tac_overflow_review/sales-saved.hwp");
    let mut d = rhwp::parse_document(bytes).unwrap();
    let p = &mut d.sections[0].paragraphs[2];
    let mut shape = d.doc_info.para_shapes[p.para_shape_id as usize].clone();
    let mut border = rhwp::model::style::BorderFill::default();
    border.three_d = true;
    d.doc_info.border_fills.push(border);
    shape.border_fill_id = d.doc_info.border_fills.len() as u16;
    p.para_shape_id = d.doc_info.para_shapes.len() as u16;
    d.doc_info.para_shapes.push(shape);
    assert!(HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()).is_err());
}

#[test]
fn hosted_fresh_inline_group_preserves_rows_margins_and_source_order() {
    for mode in ["same", "narrow", "break", "space"] {
        let mut d = document();
        d.sections[0].section_def.page_def.width = if mode == "narrow" { 10500 } else { 25500 };
        d.sections[0].section_def.page_def.height = 9750; //body70px
        d.sections[0].paragraphs[0].controls = vec![Control::ColumnDef(ColumnDef {
            column_count: 1,
            same_width: true,
            ..Default::default()
        })];
        let mut style = d.doc_info.para_shapes[0].clone();
        style.line_spacing_type = LineSpacingType::Percent;
        style.line_spacing = 100;
        d.doc_info.para_shapes.push(style);
        let p = &mut d.sections[0].paragraphs[1];
        let Control::Table(t) = &mut p.controls[0] else {
            panic!()
        };
        t.common.treat_as_char = true;
        t.common.width = 6000;
        t.common.height = 1; //stale declaration must not replace prepared box
        t.outer_margin_left = 150;
        t.outer_margin_right = 150;
        t.outer_margin_top = 150;
        t.outer_margin_bottom = 150;
        t.common.margin = Padding {
            left: 150,
            right: 150,
            top: 150,
            bottom: 150,
        };
        t.padding = Padding::default();
        t.cells[0].width = 6000;
        t.cells[0].height = 2700;
        t.cells[0].paragraphs = vec![paragraph("LEFT")];
        let mut second = t.clone();
        second.cells[0].paragraphs = vec![paragraph("RIGHT")];
        p.controls.push(Control::Table(second));
        p.para_shape_id = 1;
        p.char_count = 17;
        if mode == "break" || mode == "space" {
            p.text = if mode == "break" { "\n" } else { " " }.into();
            p.char_offsets = vec![8];
            p.char_count = 18;
        }
        let s = HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()).unwrap();
        let split = mode == "break" || mode == "narrow";
        assert_eq!(
            s.pagination().pages.len(),
            if split { 2 } else { 1 },
            "{mode}"
        );
        let first = s.render_page(0).unwrap();
        let last = s.render_page(s.pagination().pages.len() - 1).unwrap();
        let a = table_nodes(&first.root)[0];
        let b = if split {
            table_nodes(&last.root)[0]
        } else {
            table_nodes(&first.root)[1]
        };
        close(a.bbox.x, 22.);
        close(a.bbox.y, 50.);
        close(a.bbox.height, 36.);
        close(b.bbox.y, if split { 32. } else { 50. });
        close(b.bbox.height, 36.);
        if mode != "space" {
            close(b.bbox.x, if split { 22. } else { 106. });
        } else {
            assert!(b.bbox.x > 106.);
        }
        let tail = lines(&last.root)
            .into_iter()
            .find(|n| text(n) == "suffix")
            .unwrap();
        close(tail.bbox.y, if split { 70. } else { 88. });
        let all = (0..s.pagination().pages.len())
            .map(|i| text(&s.render_page(i).unwrap().root))
            .collect::<String>();
        for label in ["prefix", "LEFT", "RIGHT", "suffix"] {
            assert_eq!(all.matches(label).count(), 1);
        }
    }
}
fn document() -> Document {
    let mut doc = Document::default();
    doc.doc_info.char_shapes.push(CharShape {
        base_size: 900,
        ..Default::default()
    });
    doc.doc_info.para_shapes.push(ParaShape {
        line_spacing_type: LineSpacingType::Fixed,
        line_spacing: 2700,
        ..Default::default()
    });
    let table = Table {
        common: CommonObjAttr {
            width: 15900,
            text_wrap: TextWrap::TopAndBottom,
            vert_rel_to: VertRelTo::Para,
            horz_rel_to: HorzRelTo::Column,
            ..Default::default()
        },
        row_count: 1,
        col_count: 1,
        page_break: TablePageBreak::CellBreak,
        padding: Padding {
            left: 375,
            right: 525,
            top: 225,
            bottom: 300,
        },
        cells: vec![Cell {
            width: 15900,
            row_span: 1,
            col_span: 1,
            paragraphs: ["alpha", "beta", "gamma"].map(paragraph).to_vec(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut prefix = paragraph("prefix");
    prefix.controls.push(Control::ColumnDef(ColumnDef {
        column_count: 2,
        spacing: 1200,
        same_width: true,
        ..Default::default()
    }));
    let mut section = Section {
        paragraphs: vec![
            prefix,
            Paragraph {
                controls: vec![Control::Table(Box::new(table))],
                // The empty carrier owns a real9pt line, including its
                // explicit character shape just like a parsed HWP paragraph.
                ..paragraph("")
            },
            paragraph("suffix"),
        ],
        ..Default::default()
    };
    section.section_def.page_def = PageDef {
        width: 36000,
        height: 7500,
        margin_left: 1500,
        margin_right: 1500,
        margin_top: 2250,
        margin_bottom: 2250,
        ..Default::default()
    };
    doc.sections.push(section);
    doc
}

// Synthetic host geometry, not hand-authored LineSeg evidence for admission.
// Exercise the already-established saved-owner rule at the section transport:
// prefix18px; owner12+6px; offset(5,7), margins(2,4,3,2); child lines18px.
fn positioned_document(body_height: u32, split: bool) -> Document {
    let mut d = occluded_owner_budget_document(body_height);
    let Control::ColumnDef(c) = &mut d.sections[0].paragraphs[0].controls[0] else { panic!() };
    c.column_count = 1;
    c.spacing = 0;
    let p = &mut d.sections[0].paragraphs[1];
    p.line_segs[0].line_height = 900;
    p.line_segs[0].text_height = 900;
    p.line_segs[0].baseline_distance = 765;
    p.line_segs[0].line_spacing = 450;
    let Control::Table(t) = &mut p.controls[0] else { panic!() };
    t.common.flow_with_text = true;
    t.common.horizontal_offset = 375;
    t.common.vertical_offset = 525;
    t.common.margin = Padding { left: 150, right: 300, top: 225, bottom: 150 };
    t.outer_margin_left = 150;
    t.outer_margin_right = 300;
    t.outer_margin_top = 225;
    t.outer_margin_bottom = 150;
    if split { t.cells[0].paragraphs = ["alpha", "beta", "gamma"].map(paragraph).to_vec(); }
    d
}

#[test]
fn positioned_owner_reserves_offset_and_margin_once_before_following_text() {
    let d = positioned_document(60, false);
    let before = format!("{:?}", d.sections[0].paragraphs);
    let s = session(&d);
    assert_eq!(s.pagination().pages.len(), 1);
    let tree = s.render_page(0).unwrap();
    let t = table_nodes(&tree.root)[0];
    close(t.bbox.x, 20. + 5. + 2.);
    close(t.bbox.y, 30. + 18. + 7. + 3.);
    close(t.bbox.height, 18.);
    let rows = lines(&tree.root);
    let owner = rows.iter().find(|n| text(n).is_empty()).unwrap();
    close(owner.bbox.y, 48.);
    close(owner.bbox.height, 12.);
    let suffix = rows.iter().find(|n| text(n) == "suffix").unwrap();
    close(suffix.bbox.y, 78.); //table bottom76 + physical bottom margin2
    assert_eq!(text(&tree.root), "prefixalphasuffix");
    assert_eq!(format!("{:?}", d.sections[0].paragraphs), before);
}

#[test]
fn positioned_first_fragment_failure_and_continuation_preserve_owner_and_units() {
    // 21px left after prefix: offset10 + first line12 + bottom2 cannot fit.
    let d = positioned_document(39, true);
    let s = session(&d);
    assert_eq!(text(&s.render_page(0).unwrap().root), "prefix");
    let mut all = String::new();
    let mut fragments = Vec::new();
    let mut owners = 0;
    for pi in 0..s.pagination().pages.len() {
        let tree = s.render_page(pi).unwrap();
        all.push_str(&text(&tree.root));
        owners += lines(&tree.root).iter().filter(|n| text(n).is_empty()).count();
        for t in table_nodes(&tree.root) {
            close(t.bbox.x, 27.);
            assert!(t.bbox.y + t.bbox.height + 2. <= 69.);
            fragments.push(t.bbox.y);
        }
    }
    // Two18px units + top3 + bottom2 exceed39; each continuation owns one unit.
    assert_eq!(fragments, vec![40., 33., 33.]);
    assert_eq!(owners, 1);
    for label in ["prefix", "alpha", "beta", "gamma", "suffix"] {
        assert_eq!(all.matches(label).count(), 1);
    }
    let last = s.render_page(s.pagination().pages.len()-1).unwrap();
    assert!(text(&last.root).ends_with("suffix"));
}

#[test]
fn positioned_host_rejects_unsupported_anchor_and_unshared_margin_records() {
    for mode in ["mismatch", "negative", "paper", "columns", "square"] {
        let mut d = positioned_document(60, false);
        let Control::Table(t) = &mut d.sections[0].paragraphs[1].controls[0] else { panic!() };
        match mode {
            "mismatch" => t.outer_margin_top = 0,
            "negative" => t.common.vertical_offset = u32::MAX,
            "paper" => t.common.horizontal_offset = 33000,
            "square" => t.common.text_wrap = TextWrap::Square,
            "columns" => {
                let Control::ColumnDef(c) = &mut d.sections[0].paragraphs[0].controls[0] else { panic!() };
                c.column_count = 2;
            }
            _ => unreachable!(),
        }
        assert!(HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()).is_err(), "{mode}");
    }
}
fn session(doc: &Document) -> HostedSectionSession {
    HostedSectionSession::from_document(doc, 0, 96., CellEndPolicy::default()).unwrap()
}

fn document_with_page_fields() -> Document {
    let source =
        rhwp::parse_document(include_bytes!("../../samples/21_언어_기출_편집가능본.hwp")).unwrap();
    let Control::Table(t) = &source.sections[0].paragraphs[206].controls[0] else {
        panic!()
    };
    let mut doc = document();
    let mut field = t.cells[0].paragraphs[0].clone();
    let mut para_style = source.doc_info.para_shapes[field.para_shape_id as usize].clone();
    para_style.border_fill_id = 0;
    para_style.tab_def_id = 0;
    para_style.raw_data = None;
    let mut char_style =
        source.doc_info.char_shapes[field.char_shapes[0].char_shape_id as usize].clone();
    char_style.border_fill_id = 0;
    char_style.raw_data = None;
    field.para_shape_id = doc.doc_info.para_shapes.len() as u16;
    field.char_shapes[0].char_shape_id = doc.doc_info.char_shapes.len() as u32;
    doc.doc_info.para_shapes.push(para_style);
    doc.doc_info.char_shapes.push(char_style);
    let Control::AutoNumber(n) = &mut field.controls[0] else {
        panic!()
    };
    n.assigned_number = 77;
    let Control::Table(t) = &mut doc.sections[0].paragraphs[1].controls[0] else {
        panic!()
    };
    t.cells[0].paragraphs = vec![field; 12];
    doc
}

fn field_values(n: &RenderNode) -> Vec<String> {
    let mut out = Vec::new();
    if let RenderNodeType::TextRun(t) = &n.node_type {
        if let Some(v) = &t.display_text {
            out.push(v.clone());
        }
    }
    for c in &n.children {
        out.extend(field_values(c));
    }
    out
}

#[test]
fn cell_page_fields_follow_final_host_pages_not_columns_or_saved_numbers() {
    let doc = document_with_page_fields();
    let s = session(&doc);
    assert!(s.pagination().pages.len() > 1);
    let mut count = 0;
    for (index, p) in s.pagination().pages.iter().enumerate() {
        let tree = s.render_page(index).unwrap();
        for v in field_values(&tree.root) {
            assert_eq!(v, p.page_number.to_string());
            count += 1;
        }
    }
    assert_eq!(count, 12);
}

#[test]
fn standalone_page_fields_keep_section_start_number() {
    let mut doc = document_with_page_fields();
    doc.sections[0].paragraphs[0].controls.clear();
    doc.sections[0].section_def.page_num = 7;
    let Control::Table(t) = &mut doc.sections[0].paragraphs[1].controls[0] else {
        panic!()
    };
    t.common.horz_rel_to = HorzRelTo::Para;
    let bytes = rhwp::serializer::cfb_writer::serialize_hwp(&doc).unwrap();
    let mut s = DocumentV2Session::from_bytes(&bytes, r#"{"dpi":96,"max_pages":30}"#).unwrap();
    let mut count = 0;
    while let Some(raw) = s.next_page_json().unwrap() {
        let p: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let expected = (7 + p["page_index"].as_u64().unwrap()).to_string();
        let mut nodes = vec![&p["render_tree"]["root"]];
        while let Some(n) = nodes.pop() {
            if let Some(v) = n["node_type"]["TextRun"]["display_text"].as_str() {
                assert_eq!(v, expected);
                count += 1;
            }
            if let Some(children) = n["children"].as_array() {
                nodes.extend(children);
            }
        }
    }
    assert_eq!(count, 12);
}
fn text(n: &RenderNode) -> String {
    let mut value = match &n.node_type {
        RenderNodeType::TextRun(run) => run.text.clone(),
        _ => String::new(),
    };
    for child in &n.children {
        value.push_str(&text(child));
    }
    value
}
fn table_nodes(n: &RenderNode) -> Vec<&RenderNode> {
    let mut result = vec![];
    if matches!(n.node_type, RenderNodeType::Table(_)) {
        result.push(n);
    }
    for child in &n.children {
        result.extend(table_nodes(child));
    }
    result
}
fn ids(n: &RenderNode, seen: &mut std::collections::HashSet<u32>) {
    assert!(seen.insert(n.id), "duplicate render ID {}", n.id);
    for c in &n.children {
        ids(c, seen);
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}

#[test]
fn normal_saved_host_reuses_partition_blank_and_page_cut_for_final_paint() {
    // Untouched Hancom save + matching PDF, not manually assigned LineSeg.
    // Fixture README independently fixes1200HU lines/600HU gaps and page2 DELTA.
    let d = rhwp::parse_document(include_bytes!(
        "../fixtures/issue7353_body_frame_review/portrait-saved.hwp"
    ))
    .unwrap();
    for dpi in [96., 192.] {
        let s = HostedSectionSession::from_document(&d, 0, dpi, CellEndPolicy::default()).unwrap();
        assert_eq!(s.pagination().pages.len(), 2);
        for (page, expected) in [
            vec!["BEFORE", "ALPHA", "BRAVO", "", "CHARLIE"],
            vec!["DELTA", "ECHO", "AFTER"],
        ]
        .into_iter()
        .enumerate()
        {
            let tree = s.render_page(page).unwrap();
            let rows = lines(&tree.root);
            assert_eq!(rows.len(), expected.len());
            for (i, (line, value)) in rows.iter().zip(expected).enumerate() {
                assert_eq!(text(line), value);
                close(line.bbox.y, (1500. + i as f64 * 1800.) * dpi / 7200.);
                close(line.bbox.height, 1200. * dpi / 7200.);
                let RenderNodeType::TextLine(owner) = &line.node_type else {
                    unreachable!()
                };
                let expected_para = match value {
                    "BEFORE" => 0,
                    "AFTER" => 2,
                    _ => 1,
                };
                assert_eq!(owner.section_index, Some(0));
                assert_eq!(owner.para_index, Some(expected_para));
            }
            ids(&tree.root, &mut Default::default());
            assert_eq!(
                serde_json::to_value(&tree).unwrap(),
                serde_json::to_value(s.render_page(page).unwrap()).unwrap()
            );
            let envelope: serde_json::Value =
                serde_json::from_str(&s.render_page_json(page).unwrap()).unwrap();
            assert_eq!(envelope["engine"], "table_v2_hosted_section");
            assert_eq!(envelope["page_index"], page);
            // Compare both sides after the same JSON number round-trip; bbox
            // expectations above independently check the original f64 values.
            let wire_tree: serde_json::Value =
                serde_json::from_str(&serde_json::to_string(&tree).unwrap()).unwrap();
            assert_eq!(envelope["render_tree"], wire_tree);
        }
        assert!(matches!(
            s.render_page_json(2),
            Err(HostedTableError::WrongDestination)
        ));
    }
}

#[test]
fn stored_host_rejects_unqualified_nonzero_frame_reset() {
    let mut d = rhwp::parse_document(include_bytes!(
        "../fixtures/issue7353_body_frame_review/portrait-saved.hwp"
    ))
    .unwrap();
    d.sections[0].paragraphs[1].line_segs[4].vertical_pos = 100;
    assert!(matches!(
        HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()),
        Err(HostedTableError::Geometry(GeometryError::Unsupported(
            "unqualified stored body frame reset"
        )))
    ));
}

fn lines(n: &RenderNode) -> Vec<&RenderNode> {
    let mut result = vec![];
    if matches!(n.node_type, RenderNodeType::TextLine(_)) {
        result.push(n);
    }
    for child in &n.children {
        result.extend(lines(child));
    }
    result
}

#[test]
fn existing_host_transitions_reserve_and_paint_same_fragments_with_following_text() {
    let doc = document();
    let session = session(&doc);
    let pages = &session.pagination().pages;
    assert_eq!(pages.len(), 2, "no terminal blank page");
    let mut packets = vec![];
    for page in pages {
        for col in &page.column_contents {
            for item in &col.items {
                match item {
                    PageItem::HostedTable { fragment, .. } => packets.push(fragment.occupied()),
                    PageItem::HostedParagraph { .. } => {}
                    _ => panic!("V2 must not emit Legacy table cuts: {item:?}"),
                }
            }
        }
    }
    assert_eq!(packets.len(), 2);
    for (r, expected) in packets
        .iter()
        .zip([(20., 48., 212., 21.), (248., 30., 212., 40.)])
    {
        close(r.x, expected.0);
        close(r.y, expected.1);
        close(r.width, expected.2);
        close(r.height, expected.3);
    }
    close(pages[0].column_contents[0].used_height, 39.);
    close(pages[0].column_contents[1].used_height, 40.);
    let first = session.render_page(0).unwrap();
    assert_eq!(text(&first.root), "prefixalphabetagamma");
    let painted = table_nodes(&first.root);
    assert_eq!(painted.len(), 2);
    for (node, reserved) in painted.iter().zip(&packets) {
        close(node.bbox.x, reserved.x);
        close(node.bbox.y, reserved.y);
        close(node.bbox.width, reserved.width);
        close(node.bbox.height, reserved.height);
    }
    let second = session.render_page(1).unwrap();
    assert_eq!(text(&second.root), "suffix");
    close(pages[1].column_contents[0].used_height, 36.);
    let final_lines = lines(&second.root);
    assert_eq!(
        final_lines.len(),
        2,
        "the float's empty owner line must survive"
    );
    close(final_lines[0].bbox.y, 30.);
    close(final_lines[1].bbox.y, 48.);
    for tree in [&first, &second] {
        ids(&tree.root, &mut Default::default());
    }
    assert_eq!(
        serde_json::to_value(&first).unwrap(),
        serde_json::to_value(session.render_page(0).unwrap()).unwrap()
    );
}

#[test]
fn unsupported_host_does_not_silently_choose_legacy() {
    let mut d = document();
    let Control::Table(t) = &mut d.sections[0].paragraphs[1].controls[0] else {
        unreachable!()
    };
    t.common.text_wrap = TextWrap::Square;
    assert!(matches!(
        HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()),
        Err(HostedTableError::UnsupportedHost(_))
    ));
    let mut d = document();
    d.doc_info.para_shapes[0].attr1 |= 1 << 17;
    assert!(matches!(
        HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()),
        Err(HostedTableError::UnsupportedHost(_))
    ));
}

#[test]
fn empty_equal_columns_non_fit_terminates_without_blank_page_loop() {
    let mut d = document();
    d.sections[0].section_def.page_def.height = 4800; //4px body cannot hold12px line +3px padding
    d.sections[0].paragraphs.remove(0);
    // default single column has sufficient width, but insufficient height.
    assert!(matches!(
        HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()),
        Err(HostedTableError::NoProgress)
    ));
}

#[test]
fn edit_requires_new_snapshot_and_does_not_mutate_committed_paint() {
    let mut d = document();
    let before = session(&d);
    let Control::Table(t) = &mut d.sections[0].paragraphs[1].controls[0] else {
        unreachable!()
    };
    t.cells[0].paragraphs[0] = paragraph("edited");
    let after = session(&d);
    assert_eq!(
        text(&before.render_page(0).unwrap().root),
        "prefixalphabetagamma"
    );
    assert_eq!(
        text(&after.render_page(0).unwrap().root),
        "prefixeditedbetagamma"
    );
}

#[test]
fn following_empty_paragraph_occupies_its_line_after_committed_table() {
    let mut d = document();
    d.sections[0].section_def.page_def.height = 15000; //140px body
    d.sections[0].paragraphs.insert(2, paragraph(""));
    let s = session(&d);
    assert_eq!(s.pagination().pages.len(), 1);
    let tree = s.render_page(0).unwrap();
    let t = table_nodes(&tree.root);
    assert_eq!(t.len(), 1);
    // prefix18 + table(3+3*18+4)=61 + owner line18 + intentional empty18.
    // may pull the suffix up into the blank line or recompute the table height.
    close(t[0].bbox.y, 48.);
    close(t[0].bbox.height, 61.);
    let rows = lines(&tree.root);
    let suffix = rows.iter().find(|n| text(n) == "suffix").unwrap();
    close(suffix.bbox.y, 145.);
    assert_eq!(rows.iter().filter(|n| text(n).is_empty()).count(), 2);
    close(
        rows.iter().find(|n| text(n).is_empty()).unwrap().bbox.y,
        109.,
    );
}

#[test]
fn right_to_left_host_columns_keep_packet_coordinates_and_order() {
    let mut d = document();
    let Control::ColumnDef(c) = &mut d.sections[0].paragraphs[0].controls[0] else {
        unreachable!()
    };
    c.direction = rhwp::model::page::ColumnDirection::RightToLeft;
    let s = session(&d);
    let tree = s.render_page(0).unwrap();
    let t = table_nodes(&tree.root);
    assert_eq!(t.len(), 2);
    close(t[0].bbox.x, 248.);
    close(t[1].bbox.x, 20.);
    assert_eq!(text(&tree.root), "prefixalphabetagamma");
}

#[test]
fn section_driver_honors_explicit_page_break_before_v2_table() {
    let mut d = document();
    d.sections[0].section_def.page_def.height = 15000; //140px body
    d.sections[0].paragraphs[1].column_type = rhwp::model::paragraph::ColumnBreakType::Page;
    let s = session(&d);
    assert_eq!(s.pagination().pages.len(), 2);
    assert_eq!(text(&s.render_page(0).unwrap().root), "prefix");
    let page = s.render_page(1).unwrap();
    assert_eq!(text(&page.root), "alphabetagammasuffix");
    let tables = table_nodes(&page.root);
    assert_eq!(tables.len(), 1);
    close(tables[0].bbox.y, 30.);
    close(tables[0].bbox.height, 61.); //3px padding + three18px lines +4px
    let rows = lines(&page.root);
    close(
        rows.iter().find(|n| text(n).is_empty()).unwrap().bbox.y,
        91.,
    );
    close(
        rows.iter().find(|n| text(n) == "suffix").unwrap().bbox.y,
        109.,
    );
}

#[test]
fn section_driver_honors_explicit_column_break_before_v2_table() {
    let mut d = document();
    d.sections[0].section_def.page_def.height = 15000;
    d.sections[0].paragraphs[1].column_type = rhwp::model::paragraph::ColumnBreakType::Column;
    let s = session(&d);
    assert_eq!(s.pagination().pages.len(), 1);
    let page = s.render_page(0).unwrap();
    let t = table_nodes(&page.root);
    assert_eq!(t.len(), 1);
    close(t[0].bbox.x, 248.);
    close(t[0].bbox.y, 30.);
    close(t[0].bbox.height, 61.);
    let rows = lines(&page.root);
    let suffix = rows.iter().find(|n| text(n) == "suffix").unwrap();
    close(suffix.bbox.x, 248.);
    close(suffix.bbox.y, 109.);
    assert_eq!(text(&page.root), "prefixalphabetagammasuffix");
}

#[test]
fn section_driver_applies_style_page_break_after_v2_continuation() {
    let mut d = document();
    let mut style = d.doc_info.para_shapes[0].clone();
    style.attr1 |= 1 << 19; // page-break-before (paragraph shape)
    d.sections[0].paragraphs[2].para_shape_id = d.doc_info.para_shapes.len() as u16;
    d.doc_info.para_shapes.push(style);
    let s = session(&d);
    // Table splits across both columns. Its empty owner line is on page2;
    // the suffix's explicit paragraph-style boundary must start page3.
    assert_eq!(s.pagination().pages.len(), 3);
    assert_eq!(
        text(&s.render_page(0).unwrap().root),
        "prefixalphabetagamma"
    );
    let owner = s.render_page(1).unwrap();
    assert_eq!(lines(&owner.root).len(), 1);
    close(lines(&owner.root)[0].bbox.y, 30.);
    let last = s.render_page(2).unwrap();
    assert_eq!(text(&last.root), "suffix");
    close(lines(&last.root)[0].bbox.y, 30.);
}

#[test]
fn section_driver_keeps_page_fields_after_explicit_boundary() {
    let mut d = document_with_page_fields();
    d.sections[0].paragraphs[1].column_type = rhwp::model::paragraph::ColumnBreakType::Page;
    let s = session(&d);
    assert_eq!(text(&s.render_page(0).unwrap().root), "prefix");
    let mut count = 0;
    for (index, page) in s.pagination().pages.iter().enumerate() {
        for value in field_values(&s.render_page(index).unwrap().root) {
            assert!(index > 0);
            assert_eq!(value, page.page_number.to_string());
            count += 1;
        }
    }
    assert_eq!(count, 12);
}

#[test]
fn section_driver_does_not_absorb_a_table_only_terminal_paragraph() {
    let mut d = document();
    d.sections[0].paragraphs.remove(2);
    d.sections[0].paragraphs.remove(0);
    let s = session(&d);
    let all = (0..s.pagination().pages.len())
        .map(|i| s.render_page(i).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        all.iter().map(|p| text(&p.root)).collect::<String>(),
        "alphabetagamma"
    );
    assert_eq!(
        all.iter()
            .flat_map(|p| lines(&p.root))
            .filter(|n| text(n).is_empty())
            .count(),
        1
    );
    assert_eq!(all.iter().flat_map(|p| table_nodes(&p.root)).count(), 2);
}

#[test]
fn section_driver_rejects_legacy_absorption_instead_of_losing_v2_line() {
    let mut d = document();
    d.sections[0].paragraphs = vec![paragraph("prefix"), paragraph("")];
    d.sections[0].paragraphs[1].column_type = rhwp::model::paragraph::ColumnBreakType::Column;
    // Legacy intentionally absorbs a terminal empty single-column break.
    // This V2 line has not been admitted under that rule: fail closed rather
    // than claiming the full source was placed while a composition remains.
    assert!(matches!(
        HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()),
        Err(HostedTableError::UnsupportedHost(
            "section boundary absorbed a V2 paragraph"
        ))
    ));
}

#[test]
fn saved_zero_width_owner_shares_table_origin_and_preserves_following_paragraph() {
    let bytes = include_bytes!("../fixtures/issue7353_host_owner_review/split-saved.hwp");
    let d = rhwp::parse_document(bytes).unwrap();
    let before = format!("{:?}", d.sections[0].paragraphs);
    let row = &d.sections[0].paragraphs[1].line_segs[0];
    assert_eq!(
        (row.segment_width, row.line_height, row.line_spacing),
        (0, 900, 450)
    );
    let s = HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::OmitFinalParagraphGap)
        .unwrap();
    assert_eq!(s.pagination().pages.len(), 3);
    assert_eq!(text(&s.render_page(0).unwrap().root), "BEFORE");
    let first = s.render_page(1).unwrap();
    let last = s.render_page(2).unwrap();
    assert_eq!(table_nodes(&first.root).len(), 1);
    assert_eq!(table_nodes(&last.root).len(), 1);
    let all = format!("{}{}", text(&first.root), text(&last.root));
    for i in 1..=25 {
        assert_eq!(all.matches(&format!("ROW {i:02}")).count(), 1);
    }
    assert!(text(&first.root).contains("ROW 19"));
    assert!(!text(&first.root).contains("ROW 20"));
    assert!(text(&last.root).contains("ROW 20"));
    let owners: Vec<_> = lines(&first.root)
        .into_iter()
        .filter(|n| text(n).is_empty())
        .collect();
    assert_eq!(owners.len(), 1);
    close(owners[0].bbox.width, 0.);
    close(owners[0].bbox.y, 20.);
    close(owners[0].bbox.height, 12.); //900HU, preserved at the table anchor
    assert!(!lines(&last.root).iter().any(|n| text(n).is_empty()));
    // Independent PDF p3: table bottom96.759pt, AFTER baseline104.442pt.
    // Saved AFTER line vpos8175HU + top margin1500HU gives129px at96dpi.
    let after = lines(&last.root)
        .into_iter()
        .find(|n| text(n) == "AFTER")
        .unwrap();
    close(after.bbox.y, 129.);
    let t = table_nodes(&last.root)[0];
    assert!((t.bbox.y + t.bbox.height - 96.759 * 96. / 72.).abs() < 0.2);
    close(t.bbox.y + t.bbox.height, after.bbox.y);
    assert_eq!(format!("{:?}", d.sections[0].paragraphs), before);
}

// Synthetic budget boundaries of the independently observed zero-width owner:
// prefix18px, table18px, owner30px. Table-only fit must not consume the owner.
fn occluded_owner_budget_document(body_height: u32) -> Document {
    let mut d = document();
    d.sections[0].section_def.page_def.height = body_height * 75 + 4500;
    let p = &mut d.sections[0].paragraphs[1];
    p.line_segs = vec![rhwp::model::paragraph::LineSeg {
        line_height: 2250,
        text_height: 2250,
        baseline_distance: 1800,
        tag: rhwp::model::paragraph::LineSeg::TAG_SINGLE_SEGMENT_LINE,
        ..Default::default()
    }];
    let Control::Table(t) = &mut p.controls[0] else {
        panic!("table")
    };
    t.padding = Padding::default();
    t.cells[0].paragraphs.truncate(1);
    d
}

#[test]
fn joint_owner_budget_moves_line_and_table_together_without_consuming_failed_fit() {
    let d = occluded_owner_budget_document(40);
    let s = session(&d);
    assert_eq!(s.pagination().pages.len(), 2);
    let first = s.render_page(0).unwrap();
    let owner = lines(&first.root)
        .into_iter()
        .find(|n| text(n).is_empty())
        .unwrap();
    close(owner.bbox.x, 248.);
    close(owner.bbox.y, 30.);
    close(owner.bbox.height, 30.);
    let t = table_nodes(&first.root)[0];
    close(t.bbox.x, 248.);
    close(t.bbox.y, 30.);
    close(t.bbox.height, 18.);
    assert_eq!(text(&first.root), "prefixalpha");
    let last = s.render_page(1).unwrap();
    assert_eq!(text(&last.root), "suffix");
    close(lines(&last.root)[0].bbox.y, 30.);
    assert!(!lines(&last.root).iter().any(|n| text(n).is_empty()));
}

#[test]
fn exact_joint_owner_budget_fits_without_inventing_a_second_line() {
    let s = session(&occluded_owner_budget_document(48));
    assert_eq!(s.pagination().pages.len(), 1);
    let tree = s.render_page(0).unwrap();
    let t = table_nodes(&tree.root)[0];
    close(t.bbox.x, 20.);
    close(t.bbox.y, 48.);
    let rows = lines(&tree.root);
    let owner = rows.iter().find(|n| text(n).is_empty()).unwrap();
    close(owner.bbox.y, t.bbox.y);
    assert_eq!(rows.iter().filter(|n| text(n).is_empty()).count(), 1);
    let suffix = rows.iter().find(|n| text(n) == "suffix").unwrap();
    close(suffix.bbox.x, 248.);
    close(suffix.bbox.y, 30.);
}

#[test]
fn failed_table_query_does_not_leave_its_fitting_owner_on_the_previous_column() {
    let mut d = occluded_owner_budget_document(100);
    let Control::Table(t) = &mut d.sections[0].paragraphs[1].controls[0] else {
        panic!("table")
    };
    t.common.height = 7125;
    t.cells[0].height = 7125; //95px: owner30 fits the82px remainder, table does not
    let s = session(&d);
    assert_eq!(s.pagination().pages.len(), 2);
    let tree = s.render_page(0).unwrap();
    let owner = lines(&tree.root)
        .into_iter()
        .find(|n| text(n).is_empty())
        .unwrap();
    let t = table_nodes(&tree.root)[0];
    close(owner.bbox.x, 248.);
    close(owner.bbox.y, 30.);
    close(t.bbox.x, owner.bbox.x);
    close(t.bbox.y, owner.bbox.y);
    close(t.bbox.height, 95.);
    assert_eq!(
        lines(&tree.root)
            .into_iter()
            .filter(|n| text(n).is_empty())
            .count(),
        1
    );
    assert_eq!(text(&tree.root), "prefixalpha");
    let next = s.render_page(1).unwrap();
    assert_eq!(text(&next.root), "suffix");
    assert_eq!(lines(&next.root).len(), 1);
}

#[test]
fn zero_width_without_qualifying_table_is_not_an_occluded_owner() {
    let mut d = occluded_owner_budget_document(40);
    d.sections[0].paragraphs[1].controls.clear();
    assert!(matches!(
        HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()),
        Err(HostedTableError::Geometry(_))
    ));
}

#[test]
fn authored_blank_after_occluded_owner_is_still_a_separate_line() {
    let mut d = occluded_owner_budget_document(100);
    d.sections[0].paragraphs.insert(2, paragraph(""));
    let s = session(&d);
    assert_eq!(s.pagination().pages.len(), 1);
    let page = s.render_page(0).unwrap();
    let rows = lines(&page.root);
    let blanks: Vec<_> = rows.iter().filter(|n| text(n).is_empty()).collect();
    assert_eq!(blanks.len(), 2);
    close(blanks[0].bbox.width, 0.);
    close(blanks[0].bbox.y, 48.);
    close(blanks[1].bbox.y, 78.);
    close(blanks[1].bbox.height, 12.);
    close(
        rows.iter().find(|n| text(n) == "suffix").unwrap().bbox.y,
        96.,
    );
}

#[test]
fn tac_and_invalid_line_metrics_cannot_use_occluded_owner_admission() {
    let mut d = occluded_owner_budget_document(40);
    let Control::Table(t) = &mut d.sections[0].paragraphs[1].controls[0] else {
        panic!("table")
    };
    t.common.treat_as_char = true;
    assert!(HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()).is_err());
    let mut d = occluded_owner_budget_document(40);
    d.sections[0].paragraphs[1].line_segs[0].baseline_distance = 2251;
    assert!(matches!(
        HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()),
        Err(HostedTableError::Geometry(GeometryError::Unsupported(
            "occluded table owner line"
        )))
    ));
}
