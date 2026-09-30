//! Shared section-host contracts. Normal Hancom saves and independent PDFs are
//! in issue7353_stored_anchor_review; mutations below are synthetic boundaries.
use rhwp::{
    model::{control::Control, document::Document},
    renderer::{
        render_tree::{RenderNode, RenderNodeType},
        table_v2::{CellEndPolicy, HostedSectionSession},
    },
};

fn load(name: &str) -> Document {
    rhwp::parse_document(
        &std::fs::read(format!(
            "{}/tests/fixtures/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    )
    .unwrap()
}
fn pages(d: &Document) -> Vec<RenderNode> {
    let s = HostedSectionSession::from_document(d, 0, 96., CellEndPolicy::OmitFinalParagraphGap)
        .unwrap();
    (0..s.pagination().pages.len())
        .map(|i| {
            let a = s.render_page_json(i).unwrap();
            assert_eq!(a, s.render_page_json(i).unwrap());
            s.render_page(i).unwrap().root
        })
        .collect()
}
fn all(n: &RenderNode) -> Vec<&RenderNode> {
    std::iter::once(n)
        .chain(n.children.iter().flat_map(all))
        .collect()
}
fn tables(n: &RenderNode) -> Vec<&RenderNode> {
    all(n)
        .into_iter()
        .filter(|n| matches!(n.node_type, RenderNodeType::Table(_)))
        .collect()
}
fn lines(n: &RenderNode) -> Vec<&RenderNode> {
    all(n)
        .into_iter()
        .filter(|n| matches!(n.node_type, RenderNodeType::TextLine(_)))
        .collect()
}
fn text(n: &RenderNode) -> String {
    all(n)
        .into_iter()
        .filter_map(|n| {
            if let RenderNodeType::TextRun(r) = &n.node_type {
                Some(r.text.as_str())
            } else {
                None
            }
        })
        .collect()
}
fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-8, "{actual} != {expected}");
}
const BASE: &str = "issue7353_stored_anchor_review/anchor-review-saved.hwp";
const DEFER: &str = "issue7353_stored_anchor_review/variants/defer-saved.hwp";

#[test]
fn normal_prefix_keeps_zero_width_and_positive_width_anchor_owners() {
    let d = load("issue7353/anchor-offset/prefix90-saved.hwp");
    let p = pages(&d);
    assert_eq!(p.len(), 17);
    let last = tables(&p[16])[0];
    // Independent same-HWP PDF17 and source coordinates, not a new golden.
    near(last.bbox.x, (5669. + 1238. + 141.) / 75.);
    near(last.bbox.y, (5670. + 4640. + 784. + 141.) / 75.);
    near(last.bbox.width, 46207. / 75.);
    near(last.bbox.height, 58612. / 75.);
    let owner = lines(&p[16])
        .into_iter()
        .find(|n| match &n.node_type {
            RenderNodeType::TextLine(l) => l.para_index == Some(89),
            _ => false,
        })
        .unwrap();
    near(owner.bbox.y, (5670. + 4640.) / 75.);
    near(owner.bbox.height, 1000. / 75.);
}

#[test]
fn normal_stored_anchor_preserves_offsets_rows_and_after() {
    for (name, top, offset) in [
        (BASE, 283., 2835.),
        (
            "issue7353_stored_anchor_review/variants/top0-saved.hwp",
            0.,
            2835.,
        ),
        (
            "issue7353_stored_anchor_review/variants/top2mm-saved.hwp",
            567.,
            2835.,
        ),
        (
            "issue7353_stored_anchor_review/variants/offset20mm-saved.hwp",
            283.,
            5670.,
        ),
    ] {
        let d = load(name);
        let before = format!("{:?}", d.sections);
        let p = pages(&d);
        assert_eq!(p.len(), 2);
        near(tables(&p[0])[0].bbox.y, (5669. + offset + top) / 75.);
        near(tables(&p[1])[0].bbox.y, (5669. + top) / 75.);
        let labels = p.iter().map(text).collect::<String>();
        for i in 1..=24 {
            assert_eq!(labels.matches(&format!("자료 {i:02} :")).count(), 1);
        }
        let last = tables(&p[1])[0];
        let after = lines(&p[1])
            .into_iter()
            .find(|n| text(n).contains("표 종료 후 본문"))
            .unwrap();
        assert!(after.bbox.y >= last.bbox.y + last.bbox.height);
        assert_eq!(format!("{:?}", d.sections), before);
    }
}

#[test]
fn normal_atomic_deferral_keeps_following_prose_on_source_page() {
    let p = pages(&load(DEFER));
    assert_eq!(p.len(), 2);
    assert!(tables(&p[0]).is_empty());
    assert_eq!(text(&p[0]), "표 시작 위치 확인표 종료 후 본문입니다.");
    near(lines(&p[0])[0].bbox.y, 5669. / 75.);
    near(lines(&p[0])[1].bbox.y, (5669. + 1760.) / 75.);
    near(tables(&p[1])[0].bbox.y, (5669. + 283.) / 75.);
    near(tables(&p[1])[0].bbox.height, 6978. / 75.);
    assert!(!text(&p[1]).contains("표 종료 후 본문"));
}

#[test]
fn normal_long_line_gap_does_not_shift_object_origin() {
    let p = pages(&load("issue7353_anchor_gap_review/anchor-saved.hwp"));
    assert_eq!(p.len(), 2);
    near(tables(&p[0])[0].bbox.y, (5669. + 2835. + 283.) / 75.);
    near(tables(&p[0])[0].bbox.height, 4652. / 75.);
    near(lines(&p[1])[0].bbox.y, 5669. / 75.);
    assert!(tables(&p[1]).is_empty());
}

#[test]
fn synthetic_pending_table_survives_story_end_without_blank_pages() {
    let mut d = load(DEFER);
    d.sections[0].paragraphs.truncate(1);
    let p = pages(&d);
    assert_eq!(p.len(), 2);
    assert_eq!(text(&p[0]), "표 시작 위치 확인");
    assert_eq!(tables(&p[1]).len(), 1);
    near(tables(&p[1])[0].bbox.y, (5669. + 283.) / 75.);
}

#[test]
fn synthetic_atomic_child_fitting_without_margin_is_not_committed() {
    let mut d = load(DEFER);
    // Full page body fits the 6978HU child, but not top283+child+bottom567.
    let page = &mut d.sections[0].section_def.page_def;
    page.height = page.margin_top + page.margin_bottom + 6978;
    assert!(matches!(
        HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::OmitFinalParagraphGap),
        Err(rhwp::renderer::table_v2::HostedTableError::NoProgress)
    ));
}

#[test]
fn synthetic_multiple_reservations_drain_in_order_before_explicit_boundary() {
    let mut d = load(DEFER);
    let mut second = d.sections[0].paragraphs[0].clone();
    second.controls.retain(|c| matches!(c, Control::Table(_)));
    second.column_type = rhwp::model::paragraph::ColumnBreakType::None;
    d.sections[0].paragraphs.insert(1, second);
    d.sections[0].paragraphs[2].column_type = rhwp::model::paragraph::ColumnBreakType::Page;
    let p = pages(&d);
    // Explicit break is AFTER both preceding control reservations. It must
    // not be spent merely moving those controls off the source page.
    assert_eq!(p.len(), 3);
    assert!(tables(&p[0]).is_empty());
    let t = tables(&p[1]);
    assert_eq!(t.len(), 2);
    near(t[0].bbox.y, (5669. + 283.) / 75.);
    near(t[1].bbox.y, (5669. + 283. + 6978. + 567. + 283.) / 75.);
    assert!(!text(&p[1]).contains("표 종료 후 본문"));
    assert!(tables(&p[2]).is_empty());
    let after = lines(&p[2])
        .into_iter()
        .find(|n| text(n).contains("표 종료 후 본문"))
        .unwrap();
    near(after.bbox.y, 5669. / 75.);
}

#[test]
fn synthetic_overlapping_anchor_remains_rejected() {
    let mut d = load(BASE);
    for c in &mut d.sections[0].paragraphs[0].controls {
        if let Control::Table(t) = c {
            t.common.vertical_offset = 0;
        }
    }
    assert!(
        HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::OmitFinalParagraphGap)
            .is_err()
    );
}
