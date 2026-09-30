//! TAC tail advance must match the backend's fractional character replay.
//! Original stored paragraphs supply line partitions; scoped page is a contract,
//! not a claim about the original document's page numbers.
use rhwp::{
    model::{document::Section, page::PageDef},
    renderer::{
        layout::{EmbeddedTextMeasurer, TextMeasurer},
        render_tree::{RenderNode, RenderNodeType},
        table_v2::{CellEndPolicy, HostedSectionSession},
    },
};

fn walk(n: &RenderNode) -> Vec<&RenderNode> {
    std::iter::once(n)
        .chain(n.children.iter().flat_map(walk))
        .collect()
}

fn check(pi: usize, dpi: f64) {
    let mut d =
        rhwp::parse_document(include_bytes!("../../samples/21_언어_기출_편집가능본.hwp")).unwrap();
    let p = d.sections[0].paragraphs[pi].clone();
    let expected_rows = p.line_segs.len();
    let expected_text = p.text.clone();
    let mut section = Section::default();
    section.section_def.page_def = PageDef {
        width: 31888,
        height: 92000,
        ..Default::default()
    };
    section.paragraphs = vec![p];
    d.sections = vec![section];
    let s = HostedSectionSession::from_document(&d, 0, dpi, CellEndPolicy::default()).unwrap();
    let mut text = String::new();
    let mut rows = 0;
    let mut shapes = 0;
    for page in 0..s.pagination().pages.len() {
        let tree = s.render_page(page).unwrap();
        for row in walk(&tree.root).into_iter().filter(|n| {
            matches!(n.node_type, RenderNodeType::TextLine(_)) && n.bbox.width > 300. * dpi / 96.
        }) {
            rows += 1;
            for n in &row.children {
                if let RenderNodeType::TextRun(r) = &n.node_type {
                    text.push_str(&r.text);
                    // Independent consumer: backend replay must terminate at
                    // the same advance as the emitted run's placement bbox.
                    let positions = EmbeddedTextMeasurer.compute_char_positions(&r.text, &r.style);
                    let trimmed = r.text.trim_end_matches(' ');
                    let replay = *positions.last().unwrap();
                    if trimmed == r.text {
                        assert!(
                            (n.bbox.width - replay).abs() < 1e-7,
                            "{pi}/{dpi}: {:?}: bbox={} replay={replay}",
                            r.text,
                            n.bbox.width
                        );
                    }
                    // Soft-wrap trailing spaces retain caret advances but are
                    // not visible ink; inspect the final visible character.
                    if !trimmed.is_empty() {
                        assert!(
                            n.bbox.x + positions[trimmed.chars().count()]
                                <= row.bbox.x + row.bbox.width + 1e-7
                        );
                    }
                }
            }
            shapes += walk(row)
                .iter()
                .filter(|n| matches!(n.node_type, RenderNodeType::Rectangle(_)))
                .count();
        }
    }
    assert_eq!(rows, expected_rows);
    assert_eq!(text, expected_text);
    assert_eq!(shapes, 1);
}

#[test]
fn kim_wonhaeng_tail_96dpi() {
    check(212, 96.);
}
#[test]
fn kim_wonhaeng_tail_192dpi() {
    check(212, 192.);
}
#[test]
fn lefort_tail_96dpi() {
    check(145, 96.);
}
#[test]
fn lefort_tail_192dpi() {
    check(145, 192.);
}
