use rhwp::{
    model::control::Control,
    renderer::{style_resolver::resolve_styles, table_v2::*},
};

#[test]
fn original_page_field_tables_are_prepared_without_assigned_number_substitution() {
    let d =
        rhwp::parse_document(include_bytes!("../../samples/21_언어_기출_편집가능본.hwp")).unwrap();
    for pi in [206, 232] {
        let Control::Table(t) = &d.sections[0].paragraphs[pi].controls[0] else {
            panic!()
        };
        PreparedTextTable::prepare_with_end_policy(
            t,
            &resolve_styles(&d.doc_info, 96.),
            96.,
            &d.bin_data_content,
            CellEndPolicy::OmitFinalParagraphGap,
        )
        .unwrap();
    }
}

#[test]
fn oversized_display_rejects_without_mutating_page_or_reserved_fragment() {
    use rhwp::renderer::render_tree::PageRenderTree;
    let d =
        rhwp::parse_document(include_bytes!("../../samples/21_언어_기출_편집가능본.hwp")).unwrap();
    let Control::Table(source) = &d.sections[0].paragraphs[206].controls[0] else {
        panic!()
    };
    // Synthetic boundary, not a modified real-output fixture: narrow the saved
    // 42pt line to 8pt, preserving both paragraph/cell margins independently.
    let mut t = source.clone();
    t.common.width = 1648;
    t.cells[0].width = 1648;
    t.cells[0].paragraphs.truncate(1);
    t.cells[0].paragraphs[0].line_segs[0].segment_width = 800;
    let prepared = PreparedTextTable::prepare_with_end_policy(
        &t,
        &resolve_styles(&d.doc_info, 96.),
        96.,
        &d.bin_data_content,
        CellEndPolicy::OmitFinalParagraphGap,
    )
    .unwrap();
    let TextFragmentFit::Placed(f) = prepared
        .start()
        .fit(PageArea {
            bounds: Rect {
                x: 0.,
                y: 0.,
                width: 100.,
                height: 100.,
            },
        })
        .unwrap()
    else {
        panic!()
    };
    let mut page = PageRenderTree::new(0, 100., 100.);
    let before = serde_json::to_value(&page).unwrap();
    assert!(f
        .append_to_with_page_number(&mut page, Some(65535))
        .is_err());
    assert_eq!(serde_json::to_value(&page).unwrap(), before);
    f.append_to_with_page_number(&mut page, Some(1)).unwrap();
    assert_eq!(display(&page.root), vec!["1"]);
}

#[test]
fn host_number_is_part_of_query_commit_identity() {
    use rhwp::{
        model::page::{ColumnDef, PageDef},
        renderer::{page_layout::PageLayoutInfo, render_tree::PageRenderTree},
    };
    let d =
        rhwp::parse_document(include_bytes!("../../samples/21_언어_기출_편집가능본.hwp")).unwrap();
    let mut s = HostedTableSession::from_document(
        &d,
        TableSelection {
            section: 0,
            paragraph: 206,
            control: 0,
        },
        96.,
        CellEndPolicy::OmitFinalParagraphGap,
    )
    .unwrap();
    let layout =
        PageLayoutInfo::from_page_def_for_page(&PageDef::default(), &ColumnDef::default(), 96., 10);
    let a = layout.body_area;
    let frame = TableHostFrame::new(
        &layout,
        TableHostAddress {
            section: 0,
            page: 9,
            column: None,
            revision: 0,
        },
        Rect {
            x: a.x,
            y: a.y,
            width: a.width,
            height: a.height,
        },
    )
    .unwrap()
    .with_page_number(10)
    .unwrap();
    let HostedTableFit::Placed(proposal) = s.query(frame).unwrap() else {
        panic!()
    };
    assert!(s
        .commit(*proposal, frame.with_page_number(11).unwrap())
        .is_err());
    let HostedTableFit::Placed(proposal) = s.query(frame).unwrap() else {
        panic!()
    };
    let packet = s.commit(*proposal, frame).unwrap();
    let mut page = PageRenderTree::new(9, layout.page_width, layout.page_height);
    packet.append_to(&mut page).unwrap();
    assert_eq!(display(&page.root), vec!["10"]);
}

#[test]
fn unsupported_format_remains_explicitly_rejected() {
    let mut d =
        rhwp::parse_document(include_bytes!("../../samples/21_언어_기출_편집가능본.hwp")).unwrap();
    let Control::Table(t) = &mut d.sections[0].paragraphs[206].controls[0] else {
        panic!()
    };
    let Control::AutoNumber(n) = &mut t.cells[0].paragraphs[0].controls[0] else {
        panic!()
    };
    n.format = 1;
    assert!(PreparedTextTable::prepare_with_end_policy(
        t,
        &resolve_styles(&d.doc_info, 96.),
        96.,
        &d.bin_data_content,
        CellEndPolicy::OmitFinalParagraphGap
    )
    .is_err());
}

fn preview(paragraph: usize, number: Option<u32>) -> TablePreviewSession {
    let d =
        rhwp::parse_document(include_bytes!("../../samples/21_언어_기출_편집가능본.hwp")).unwrap();
    let s = TablePreviewSession::from_document_with_end_policy(
        &d,
        TableSelection {
            section: 0,
            paragraph,
            control: 0,
        },
        96.,
        TablePreviewPages {
            width: 400.,
            height: 600.,
            body: Rect {
                x: 20.,
                y: 30.,
                width: 360.,
                height: 540.,
            },
            first_y: 30.,
        },
        5,
        CellEndPolicy::OmitFinalParagraphGap,
    )
    .unwrap();
    number.map_or(s.clone(), |n| s.with_first_page_number(n).unwrap())
}

fn display(n: &rhwp::renderer::render_tree::RenderNode) -> Vec<String> {
    use rhwp::renderer::render_tree::RenderNodeType;
    let mut result = Vec::new();
    if let RenderNodeType::TextRun(r) = &n.node_type {
        if let Some(s) = &r.display_text {
            result.push(s.clone());
        }
    }
    for c in &n.children {
        result.extend(display(c));
    }
    result
}

#[test]
fn original_fields_use_host_number_not_saved_one_or_two() {
    // Independent Hancom PDF pages 10/11 display 10/11, while source assigns 1/2.
    for (paragraph, number) in [(206, 10), (232, 11)] {
        let mut s = preview(paragraph, Some(number));
        let p = s.next_page().unwrap().unwrap();
        assert_eq!(display(&p.tree.root), vec![number.to_string()]);
        assert!(s.next_page().unwrap().is_none());
    }
}

#[test]
fn absent_context_does_not_commit_and_can_retry() {
    let mut s = preview(206, None);
    assert!(s
        .next_page()
        .err()
        .unwrap()
        .to_string()
        .contains("resolved printed page number"));
    assert_eq!(s.emitted_pages(), 0);
    let mut s = s.with_first_page_number(10).unwrap();
    assert_eq!(
        display(&s.next_page().unwrap().unwrap().tree.root),
        vec!["10"]
    );
}

#[test]
fn changing_printed_digits_preserves_reserved_geometry() {
    let mut bounds = None;
    for number in [9, 10, 11, 99] {
        let p = preview(206, Some(number)).next_page().unwrap().unwrap();
        let b = p.tree.root.children[0].bbox;
        let b = (b.x, b.y, b.width, b.height);
        if let Some(previous) = bounds {
            assert_eq!(b, previous);
        } else {
            bounds = Some(b);
        }
        assert_eq!(display(&p.tree.root), vec![number.to_string()]);
    }
}
