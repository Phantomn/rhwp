//! Body border contracts through real hosted pagination and final paint.
//! Source p6 of #7008 is an intentional empty line: height1100, advance1044HU.
//! Hancom HWPX retains it; the corresponding PDF opens the connected box there.
//! In-memory synthetic frame below is NOT a serialized Hancom visual fixture.
use rhwp::{
    model::{
        document::{Document, Section},
        page::PageDef,
    },
    renderer::{
        render_tree::{RenderNode, RenderNodeType},
        table_v2::*,
    },
};
fn document() -> Document {
    let mut d =
        rhwp::parse_document(include_bytes!("../../samples/21_언어_기출_편집가능본.hwp")).unwrap();
    let p = d.sections[0].paragraphs[6].clone();
    let mut q = d.sections[0].paragraphs[5].clone();
    q.para_shape_id = p.para_shape_id;
    // Synthetic second line after the original empty row. Keep its saved style
    // and text; normalize only the frame-local vertical positions in both rows.
    let mut p = p;
    p.line_segs[0].vertical_pos = 0;
    q.line_segs[0].vertical_pos = 1044;
    q.line_segs[0].column_start = p.line_segs[0].column_start;
    q.line_segs[0].segment_width = p.line_segs[0].segment_width;
    q.line_segs[0].tag = p.line_segs[0].tag;
    let mut s = Section::default();
    s.section_def.page_def = PageDef {
        width: 31888,
        height: 20000,
        ..Default::default()
    };
    s.paragraphs = vec![p, q];
    d.sections = vec![s];
    d
}
fn nodes(n: &RenderNode) -> Vec<&RenderNode> {
    std::iter::once(n)
        .chain(n.children.iter().flat_map(nodes))
        .collect()
}
fn horizontal(n: &RenderNode) -> Vec<f64> {
    nodes(n)
        .into_iter()
        .filter_map(|n| {
            if let RenderNodeType::Line(l) = &n.node_type {
                (l.y1 == l.y2).then_some(l.y1)
            } else {
                None
            }
        })
        .collect()
}
#[test]
fn connected_empty_line_retains_height_advance_and_only_outer_horizontal_edges() {
    let d = document();
    for dpi in [96., 192.] {
        let s =
            HostedSectionSession::from_document(&d, 0, dpi, CellEndPolicy::OmitFinalParagraphGap)
                .unwrap();
        let tree = s.render_page(0).unwrap();
        let lines: Vec<_> = nodes(&tree.root)
            .into_iter()
            .filter(|n| matches!(n.node_type, RenderNodeType::TextLine(_)))
            .collect();
        assert_eq!(lines.len(), 2);
        assert!((lines[0].bbox.height - 1100. * dpi / 7200.).abs() < 1e-7);
        assert!((lines[1].bbox.y - lines[0].bbox.y - 1044. * dpi / 7200.).abs() < 1e-7);
        let h = horizontal(&tree.root);
        assert_eq!(h.len(), 2, "shared edge must be absent");
        assert!((h[0] - lines[0].bbox.y).abs() < 1e-7);
        assert!(h[1] >= lines[1].bbox.y + lines[1].bbox.height);
    }
}
#[test]
fn nonconnecting_paragraphs_close_their_own_edges() {
    let mut d = document();
    let id = d.sections[0].paragraphs[0].para_shape_id as usize;
    d.doc_info.para_shapes[id].attr1 &= !(1 << 28);
    let s = HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()).unwrap();
    assert_eq!(horizontal(&s.render_page(0).unwrap().root).len(), 4);
}

#[test]
fn matching_top_edge_does_not_merge_different_side_pens() {
    let mut d = document();
    let mut style =
        d.doc_info.para_shapes[d.sections[0].paragraphs[1].para_shape_id as usize].clone();
    let mut border = d.doc_info.border_fills[style.border_fill_id as usize - 1].clone();
    border.borders[0].color = 0x0000ff;
    d.doc_info.border_fills.push(border);
    style.border_fill_id = d.doc_info.border_fills.len() as u16;
    d.sections[0].paragraphs[1].para_shape_id = d.doc_info.para_shapes.len() as u16;
    d.doc_info.para_shapes.push(style);
    let s = HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()).unwrap();
    assert_eq!(horizontal(&s.render_page(0).unwrap().root).len(), 4);
}
#[test]
fn column_cut_does_not_close_the_connected_group() {
    let mut d = document();
    d.sections[0].section_def.page_def.height = 1500;
    let s = HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()).unwrap();
    assert_eq!(s.pagination().pages.len(), 2);
    assert_eq!(horizontal(&s.render_page(0).unwrap().root).len(), 1);
    assert_eq!(horizontal(&s.render_page(1).unwrap().root).len(), 1);
}

#[test]
fn multiline_source_paragraph_preserves_all_rows_across_fragments() {
    let source =
        rhwp::parse_document(include_bytes!("../../samples/21_언어_기출_편집가능본.hwp")).unwrap();
    let mut d = document();
    d.sections[0].paragraphs[1] = source.sections[0].paragraphs[7].clone();
    d.sections[0].section_def.page_def.height = 4000;
    let s = HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()).unwrap();
    let mut horizontal_count = 0;
    let mut row_count = 0;
    let mut actual = String::new();
    for i in 0..s.pagination().pages.len() {
        let tree = s.render_page(i).unwrap();
        horizontal_count += horizontal(&tree.root).len();
        for n in nodes(&tree.root) {
            match &n.node_type {
                RenderNodeType::TextLine(_) => row_count += 1,
                RenderNodeType::TextRun(r) => actual.push_str(&r.text),
                _ => {}
            }
        }
    }
    assert_eq!(row_count, 11);
    assert_eq!(actual, source.sections[0].paragraphs[7].text);
    assert_eq!(horizontal_count, 2);
}
#[test]
fn unsupported_source_effects_and_insets_are_not_silently_dropped() {
    for effect in [true, false] {
        let mut d = document();
        let id = d.sections[0].paragraphs[0].para_shape_id as usize;
        if effect {
            let b = d.doc_info.para_shapes[id].border_fill_id as usize - 1;
            d.doc_info.border_fills[b].three_d = true;
        } else {
            d.doc_info.para_shapes[id].border_spacing[0] = 100;
        }
        assert!(HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()).is_err());
    }
}

fn mixed_document(height: u32) -> Document {
    let mut d = document();
    let source =
        rhwp::parse_document(include_bytes!("../../samples/21_언어_기출_편집가능본.hwp")).unwrap();
    d.sections[0].paragraphs = vec![source.sections[0].paragraphs[145].clone()];
    d.sections[0].section_def.page_def.height = height;
    d
}

#[test]
fn mixed_tac_textbox_border_consumes_the_saved_line_envelope() {
    // TAC tail advances now use the same fractional width as the line estimate.
    for dpi in [96., 192.] {
        let d = mixed_document(40000);
        let session =
            HostedSectionSession::from_document(&d, 0, dpi, CellEndPolicy::default()).unwrap();
        let tree = session.render_page(0).unwrap();
        let all = nodes(&tree.root);
        let rows: Vec<_> = all
            .iter()
            .filter(|n| {
                matches!(n.node_type, RenderNodeType::TextLine(_))
                    && n.bbox.width > 300. * dpi / 96.
            })
            .collect();
        assert_eq!(rows.len(), 13);
        for (row, saved) in rows.iter().zip(&d.sections[0].paragraphs[0].line_segs) {
            assert!(
                (row.bbox.y - rows[0].bbox.y - f64::from(saved.vertical_pos - 2860) * dpi / 7200.)
                    .abs()
                    < 1e-7
            );
            assert!((row.bbox.height - f64::from(saved.line_height) * dpi / 7200.).abs() < 1e-7);
        }
        let rects: Vec<_> = all
            .iter()
            .filter(|n| matches!(n.node_type, RenderNodeType::Rectangle(_)))
            .collect();
        assert_eq!(rects.len(), 1);
        assert!((rects[0].bbox.height - 1417. * dpi / 7200.).abs() < 1e-7);
        assert!((rects[0].bbox.y - rows[2].bbox.y).abs() < 1e-7);
        let edges = horizontal(&tree.root);
        assert_eq!(edges.len(), 2);
        assert!(edges[0] <= rows[0].bbox.y);
        assert!(edges[1] >= rows.last().unwrap().bbox.y + rows.last().unwrap().bbox.height);
    }
}

#[test]
fn mixed_shape_high_dpi_border_does_not_change_saved_line_ownership() {
    for bordered in [true, false] {
        let mut d = mixed_document(40000);
        if !bordered {
            let id = d.sections[0].paragraphs[0].para_shape_id as usize;
            d.doc_info.para_shapes[id].border_fill_id = 0;
        }
        let session =
            HostedSectionSession::from_document(&d, 0, 192., CellEndPolicy::default()).unwrap();
        let tree = session.render_page(0).unwrap();
        let all = nodes(&tree.root);
        let rows: Vec<_> = all
            .iter()
            .filter(|n| matches!(n.node_type, RenderNodeType::TextLine(_)) && n.bbox.width > 600.)
            .collect();
        assert_eq!(rows.len(), 13);
        assert_eq!(
            all.iter()
                .filter(|n| matches!(n.node_type, RenderNodeType::Rectangle(_)))
                .count(),
            1
        );
        for (i, row) in rows.iter().enumerate() {
            assert!(
                (row.bbox.y
                    - rows[0].bbox.y
                    - f64::from(d.sections[0].paragraphs[0].line_segs[i].vertical_pos - 2860)
                        * 192.
                        / 7200.)
                    .abs()
                    < 1e-7
            );
        }
    }
}

#[test]
fn mixed_shape_border_keeps_unsupported_effect_and_inset_gates() {
    for effect in [true, false] {
        let mut d = mixed_document(40000);
        let id = d.sections[0].paragraphs[0].para_shape_id as usize;
        if effect {
            let b = d.doc_info.para_shapes[id].border_fill_id as usize - 1;
            d.doc_info.border_fills[b].three_d = true;
        } else {
            d.doc_info.para_shapes[id].border_spacing[0] = 100;
        }
        assert!(HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()).is_err());
    }
}

#[test]
fn mixed_tac_textbox_survives_fragment_cuts_once_with_open_border_sides() {
    let d = mixed_document(5000);
    let session =
        HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()).unwrap();
    let mut body = String::new();
    let mut rectangles = 0;
    let mut horizontal_count = 0;
    let mut row_count = 0;
    assert!(session.pagination().pages.len() > 1);
    for i in 0..session.pagination().pages.len() {
        let tree = session.render_page(i).unwrap();
        horizontal_count += horizontal(&tree.root).len();
        for n in nodes(&tree.root) {
            if matches!(n.node_type, RenderNodeType::Rectangle(_)) {
                rectangles += 1;
            }
            if matches!(n.node_type, RenderNodeType::TextLine(_)) && n.bbox.width > 300. {
                row_count += 1;
                for child in &n.children {
                    if let RenderNodeType::TextRun(r) = &child.node_type {
                        body.push_str(&r.text);
                    }
                }
            }
        }
    }
    assert_eq!(rectangles, 1);
    assert_eq!(horizontal_count, 2);
    assert_eq!(row_count, 13);
    assert_eq!(body, d.sections[0].paragraphs[0].text);
}
