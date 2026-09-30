//! The Hancom reference puts the inline textbox after "이이의 계승자인 ".
//! Leading section/column declarations occupy source slots, not visual width.
//! Fixture provenance: original samples/21_언어_기출_편집가능본.hwp p212 was
//! isolated via its HWPX conversion with one column and 31888 HU body width.
//! Old LineSeg caches were removed, then Hancom 2020 regenerated the document:
//! HWP job 394f6477-dda5-4815-9199-5957e9723e5d, PDF job
//! dbd8a0cc-a636-4de6-aa6d-f3d5633bb92f (2026-09-30). The sibling
//! reference-2020.pdf is the independent one-page visual reference, not an
//! assertion about original whole-document page positions. Logs and generator:
//! output/7353/r19/mixed-host-border/{make-review.py,tail-width/}.
//! HWP sha256: 78c1b9af3e587e05b2c89c8dfceec18ca34140bd9a9bd568831847e5b6da336a
//! PDF sha256: 85210494b571adfbee79d06300e75414dcb0a485373b0bac83bb53685694cd57
use rhwp::renderer::{
    render_tree::{RenderNode, RenderNodeType},
    table_v2::{CellEndPolicy, HostedSectionSession},
};

fn walk(n: &RenderNode) -> Vec<&RenderNode> {
    std::iter::once(n)
        .chain(n.children.iter().flat_map(walk))
        .collect()
}

fn check(dpi: f64) {
    let d = rhwp::parse_document(include_bytes!(
        "../fixtures/issue7353_host_shape_slots/saved.hwp"
    ))
    .unwrap();
    let para = &d.sections[0].paragraphs[0];
    assert_eq!(para.control_text_positions(), [0, 0, 9]);
    let s = HostedSectionSession::from_document(&d, 0, dpi, CellEndPolicy::OmitFinalParagraphGap)
        .unwrap();
    assert_eq!(s.pagination().pages.len(), 1);
    let tree = s.render_page(0).unwrap();
    let rows: Vec<_> = walk(&tree.root)
        .into_iter()
        .filter(|n| {
            matches!(n.node_type, RenderNodeType::TextLine(_)) && n.bbox.width > 300. * dpi / 96.
        })
        .collect();
    assert_eq!(rows.len(), para.line_segs.len());
    let row = rows[0];
    let shape_index = row
        .children
        .iter()
        .position(|n| matches!(n.node_type, RenderNodeType::Rectangle(_)))
        .unwrap();
    let prefix: String = row.children[..shape_index]
        .iter()
        .filter_map(|n| match &n.node_type {
            RenderNodeType::TextRun(r) => Some(r.text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        prefix, "이이의 계승자인 ",
        "source slot must not become paragraph start"
    );
    let shape = &row.children[shape_index];
    let before = &row.children[shape_index - 1];
    let after = &row.children[shape_index + 1];
    assert!((shape.bbox.x - (before.bbox.x + before.bbox.width)).abs() < 1e-7);
    assert!((after.bbox.x - (shape.bbox.x + shape.bbox.width + 142. * dpi / 7200.)).abs() < 1e-7);
    // Independent saved line origin/height and total text conservation.
    for (row, saved) in rows.iter().zip(&para.line_segs) {
        assert!(
            (row.bbox.y
                - rows[0].bbox.y
                - f64::from(saved.vertical_pos - para.line_segs[0].vertical_pos) * dpi / 7200.)
                .abs()
                < 1e-7
        );
    }
    let text: String = rows
        .iter()
        .flat_map(|n| &n.children)
        .filter_map(|n| match &n.node_type {
            RenderNodeType::TextRun(r) => Some(r.text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(text, para.text);
    assert_eq!(
        walk(&tree.root)
            .iter()
            .filter(|n| matches!(n.node_type, RenderNodeType::Rectangle(_)))
            .count(),
        1
    );
}

#[test]
fn leading_structure_keeps_midline_shape_96dpi() {
    check(96.);
}

#[test]
fn leading_structure_keeps_midline_shape_192dpi() {
    check(192.);
}
