//! [#7379] 분할 표의 행과 저장 각주 경계를 물리 페이지에 보존한다.
//!
//! 독립 기준은 원본 정책연구 HWPX와 한컴2024 PDF 215쪽이다.
//! 66쪽에는 머리행+본문4행(0..4), 67쪽에는 본문2행(5..6)이 있다.
//! 원본 common.height=11645HU도 첫5행의 저장 높이 합과 같다.
//! 각주77은 저장 vpos [0,1172,0]의 앞 두 줄을66쪽에 두고,
//! Part 482... 및 출처 꼬리는67쪽에 번호 반복 없이 이어야 한다.
//!
//! 원 contributor 변경은 전체 각주 사전 예약을 whole/split 모두에서 제거해
//! 표 존재 검사를 개선했지만 3+3행과 각주77 누락이 남았다. 메인터너는
//! whole 예약을 유지하고 유효 저장 각주 경계를 split queue에 연결한다.
//! 행·각주 소유 개선과 전체 페이지/시각 gate 통과는 별도다. 남은 차이와
//! 실제 전후 실행은 mydocs/pr/archives/pr_7382_review.md에 기록한다.
//!
//! 각주 없는 표 대조군은 수정 전에도 통과하며 결함 검출 증거가 아니다.

#![cfg(not(target_arch = "wasm32"))]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use rhwp::document_core::DocumentCore;
use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};

const SAMPLE: &str =
    "samples/정책연구용역사업 중간진도보고서(살아있는 간장 기증자의 의학적 선별기준 연구).hwpx";
/// 문제의 표를 든 host 문단.
const HOST_PARA: usize = 728;

fn core() -> DocumentCore {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join(SAMPLE);
    let bytes = std::fs::read(path).expect("정식 원본");
    DocumentCore::from_bytes(&bytes).expect("문서 로드")
}

/// 해당 쪽에서 `HOST_PARA` 가 소유한 최상위 표의 `(y, height)` 를 모은다.
fn host_tables(root: &RenderNode) -> Vec<(f64, f64)> {
    fn walk(node: &RenderNode, out: &mut Vec<(f64, f64)>) {
        if let RenderNodeType::Table(table) = &node.node_type {
            if table.para_index == Some(HOST_PARA) {
                out.push((node.bbox.y, node.bbox.height));
            }
        }
        for child in &node.children {
            walk(child, out);
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out
}

fn host_table(root: &RenderNode) -> Option<&RenderNode> {
    table_for_para(root, HOST_PARA)
}

fn table_for_para(root: &RenderNode, para: usize) -> Option<&RenderNode> {
    if matches!(&root.node_type, RenderNodeType::Table(t) if t.para_index == Some(para)) {
        return Some(root);
    }
    root.children
        .iter()
        .find_map(|child| table_for_para(child, para))
}

fn visible_rows(table: &RenderNode) -> BTreeSet<u16> {
    table
        .children
        .iter()
        .filter_map(|n| match &n.node_type {
            RenderNodeType::TableCell(cell) => Some(cell.row),
            _ => None,
        })
        .collect()
}

fn text(node: &RenderNode) -> String {
    let mut result = match &node.node_type {
        RenderNodeType::TextRun(run) => run.text.clone(),
        _ => String::new(),
    };
    for child in &node.children {
        result.push_str(&text(child));
    }
    result
}

fn notes(root: &RenderNode) -> Option<&RenderNode> {
    if matches!(root.node_type, RenderNodeType::FootnoteArea) {
        return Some(root);
    }
    root.children.iter().find_map(notes)
}

fn line_top(root: &RenderNode, needle: &str) -> Option<f64> {
    if matches!(root.node_type, RenderNodeType::TextLine(_)) && text(root).contains(needle) {
        return Some(root.bbox.y);
    }
    root.children
        .iter()
        .find_map(|child| line_top(child, needle))
}

/// PDF의 표 괘선과 캡션/뒤 본문 좌표. 좌표는 PDF pt를96dpi로 환산했고
/// 저장 gap850HU=11.333px가 표 하단과 캡션 사이 거리임을 별도로 확인했다.
#[test]
fn split_table_outer_origin_caption_gap_and_following_body_match_pdf() {
    let core = core();
    let first = core.build_page_render_tree(65).expect("66쪽");
    let next = core.build_page_render_tree(66).expect("67쪽");
    let first_table = host_table(&first.root).expect("66쪽 표");
    let next_table = host_table(&next.root).expect("67쪽 표");
    assert!(
        (first_table.bbox.y - 799.925).abs() <= 1.5,
        "66쪽 표 상단: {}",
        first_table.bbox.y
    );
    assert!(
        (next_table.bbox.y - 86.945).abs() <= 1.5,
        "67쪽 표 상단: {}",
        next_table.bbox.y
    );
    let caption = line_top(&next.root, "표 23.").expect("표23 캡션");
    let body = line_top(&next.root, "○ 42 CFR Part 482").expect("표 뒤 본문");
    assert!((caption - 156.434).abs() <= 1.5, "캡션: {caption}");
    assert!((body - 200.261).abs() <= 1.5, "뒤 본문: {body}");
}

/// IR 여백 변형은 캡션 종료 예산의 알고리즘 반례이며 한컴 출력의 대용이 아니다.
/// 모든 행을 한 번씩 보존하고 캡션과 끝 바깥여백도 각주 lane 밖에 수용해야 한다.
#[test]
fn terminal_caption_margin_budget_preserves_rows_and_footer_space() {
    for (margin, gap) in [(283, 850), (30_000, 850), (30_000, 31_000)] {
        let mut core = core();
        let mut doc = core.document().clone();
        let rhwp::model::control::Control::Table(table) =
            &mut doc.sections[0].paragraphs[HOST_PARA].controls[0]
        else {
            panic!("표23")
        };
        table.outer_margin_bottom = margin;
        table.caption.as_mut().expect("아래 캡션").spacing = gap;
        core.set_document(doc);
        let mut rows = Vec::new();
        let mut caption_count = 0;
        for page in 65..70 {
            let tree = core.build_page_render_tree(page).expect("변형 경계 쪽");
            if let Some(table) = host_table(&tree.root) {
                rows.extend(visible_rows(table));
            }
            if let Some(caption_y) = line_top(&tree.root, "표 23.") {
                caption_count += 1;
                let body = tree
                    .root
                    .children
                    .iter()
                    .find(|node| matches!(node.node_type, RenderNodeType::Body { .. }))
                    .expect("본문 영역");
                let boundary =
                    notes(&tree.root).map_or(body.bbox.y + body.bbox.height, |area| area.bbox.y);
                let occupied_end = caption_y + 1000.0 / 75.0 + f64::from(margin) / 75.0;
                assert!(
                    occupied_end <= boundary + 1.0,
                    "margin={margin}, page={}, caption end={occupied_end}, footer={boundary}",
                    page + 1
                );
            }
        }
        assert_eq!(rows, (0..7).collect::<Vec<_>>(), "행 소유 margin={margin}");
        assert_eq!(caption_count, 1, "캡션 중복/누락 margin={margin}");
    }
}

/// 원본 row4의 [0,1620,3240,0] 저장 줄과 PDF76/77의 prefix/tail 소유.
/// PDF77 그림51 캡션 y=685.034pt를96dpi로 환산했다.
#[test]
fn stored_cell_reset_preserves_prefix_tail_and_following_figure_page() {
    let core = core();
    let first = core.build_page_render_tree(75).expect("76쪽");
    let next = core.build_page_render_tree(76).expect("77쪽");
    let a = table_for_para(&first.root, 866).expect("76쪽 표24");
    let b = table_for_para(&next.root, 866).expect("77쪽 표24");
    assert_eq!(visible_rows(a), (0..5).collect(), "앞3줄의 행4 포함");
    assert_eq!(visible_rows(b), (4..7).collect(), "행4 tail과 나머지 행");
    assert!(text(a).contains("생존 신장 기증자가"));
    assert!(!text(a).contains("투석을 시작하게 된 경우"));
    assert!(text(b).contains("투석을 시작하게 된 경우"));
    assert!(!text(b).contains("생존 신장 기증자가"));
    assert!((b.bbox.y - 86.945).abs() <= 1.5, "표24 tail 상단");
    assert!(
        (b.bbox.y + b.bbox.height - 257.799).abs() <= 1.5,
        "표24 tail 하단"
    );
    let recovery = line_top(&next.root, "기증자의 회복").expect("뒤 본문");
    assert!((recovery - 288.289).abs() <= 1.5, "뒤 본문={recovery}");
    assert!(table_for_para(&next.root, 876).is_some(), "그림51은77쪽");
    let caption = line_top(&next.root, "그림 51.").expect("그림51 캡션");
    assert!(
        (caption - 913.379).abs() <= 1.5,
        "그림51 캡션={caption}, 앞 표={:?}, 회복 줄={:?}, 그림 표={:?}",
        b.bbox,
        line_top(&next.root, "기증자의 회복"),
        table_for_para(&next.root, 876).map(|node| node.bbox)
    );
    for (table, page) in [(a, &first), (b, &next)] {
        let footer = notes(&page.root).expect("기존 각주 영역");
        assert!(table.bbox.y + table.bbox.height <= footer.bbox.y + 0.5);
    }
}

/// PDF78: 기존105/106, PDF79: 표107..111, PDF80: 남은112..124.
/// source marker 행과 실제 footer의 물리 소유를 별도로 확인한다.
#[test]
fn multirow_table_queues_cell_notes_after_the_accepted_body_fragments() {
    let core = core();
    let first = core.build_page_render_tree(77).expect("78쪽");
    let next = core.build_page_render_tree(78).expect("79쪽");
    let following = core.build_page_render_tree(79).expect("80쪽");
    let a = table_for_para(&first.root, 885).expect("표25 시작은78쪽");
    let b = table_for_para(&next.root, 885).expect("표25 끝은79쪽");
    assert_eq!(visible_rows(a), (0..4).collect(), "PDF앞3행+행3 prefix");
    assert_eq!(visible_rows(b), (3..6).collect(), "PDF행3 tail+끝2행");
    assert!(
        table_for_para(&following.root, 885).is_none(),
        "80쪽은뒤본문/남은각주"
    );
    let first_notes = text(notes(&first.root).expect("78쪽기존각주"));
    let next_notes = text(notes(&next.root).expect("79쪽표각주"));
    let last_notes = text(notes(&following.root).expect("80쪽남은각주"));
    for number in 105..=106 {
        assert!(
            first_notes.contains(&format!("{number})")),
            "기존각주{number}"
        );
    }
    for number in 107..=124 {
        let marker = format!("{number})");
        assert!(
            !first_notes.contains(&marker),
            "표각주{number}는후속footer: table={:?}, footer={:?}, caption={:?}, notes={:?}",
            a.bbox,
            notes(&first.root).map(|n| n.bbox),
            line_top(&first.root, "표 25."),
            (107..=124)
                .filter(|n| first_notes.contains(&format!("{n})")))
                .collect::<Vec<_>>()
        );
        let (owner, other) = if number <= 111 {
            (&next_notes, &last_notes)
        } else {
            (&last_notes, &next_notes)
        };
        assert_eq!(owner.matches(&marker).count(), 1, "번호{number}소유/중복");
        assert!(!other.contains(&marker), "번호{number}다른쪽중복");
    }
    assert!(
        text(&following.root).contains("Action Plan"),
        "80쪽뒤본문보존"
    );
    for (table, page) in [(a, &first), (b, &next)] {
        let footer = notes(&page.root).expect("각주영역");
        assert!(
            table.bbox.y + table.bbox.height <= footer.bbox.y + 0.5,
            "표25/각주비충돌: table={:?}, footer={:?}",
            table.bbox,
            footer.bbox
        );
    }
}

/// 빈 각주 몸통으로 footer capacity와 marker 소유를 분리한 합성 IR 계약.
/// 원본 저장 줄/표를 유지하며 본문에 아직 없는 row6의82번은 먼저 등록할 수 없다.
/// 이 변형은 한컴 출력 일치 증거가 아니다.
#[test]
fn queue_capacity_does_not_publish_a_future_row_footnote() {
    let mut core = core();
    let mut doc = core.document().clone();
    let rhwp::model::control::Control::Table(table) =
        &mut doc.sections[0].paragraphs[HOST_PARA].controls[0]
    else {
        panic!("표23")
    };
    for cell in &mut table.cells {
        for para in &mut cell.paragraphs {
            for control in &mut para.controls {
                if let rhwp::model::control::Control::Footnote(note) = control {
                    if note.number != 82 {
                        note.paragraphs.clear();
                    }
                }
            }
        }
    }
    core.set_document(doc);
    let first = core.build_page_render_tree(65).expect("앞 조각");
    let next = core.build_page_render_tree(66).expect("뒤 조각");
    assert!(!visible_rows(host_table(&first.root).expect("앞 표")).contains(&6));
    assert!(visible_rows(host_table(&next.root).expect("뒤 표")).contains(&6));
    let first_notes = text(notes(&first.root).expect("기존각주"));
    let next_notes = text(notes(&next.root).expect("marker82각주"));
    assert!(
        !first_notes.contains("82)"),
        "marker가없는앞조각에82등록: {first_notes}"
    );
    assert_eq!(
        next_notes.matches("82)").count(),
        1,
        "뒤조각82소유: {next_notes}"
    );
}

/// 각주116 하나만 footer에 남겨 용량과 같은 행 안의 marker 소유를 분리한다.
/// 다른 note는 빈 미주로 바꾸되 원래 extended-control 슬롯/저장 줄은 보존한다.
/// 원본 한컴 출력이 아닌 합성 계약이며, 실제 최종 표의 marker와 footer를 대조한다.
#[test]
fn intra_row_cut_does_not_publish_a_later_line_footnote() {
    use rhwp::model::{control::Control, footnote::Endnote};
    fn has_marker(node: &RenderNode, number: u16) -> bool {
        matches!(&node.node_type, RenderNodeType::FootnoteMarker(marker) if marker.number == number)
            || node.children.iter().any(|child| has_marker(child, number))
    }
    let mut core = core();
    let mut doc = core.document().clone();
    let Control::Table(table) = &mut doc.sections[0].paragraphs[885].controls[0] else {
        panic!("표25")
    };
    for cell in &mut table.cells {
        for para in &mut cell.paragraphs {
            for control in &mut para.controls {
                if let Control::Footnote(note) = control {
                    if note.number != 116 {
                        *control = Control::Endnote(Box::new(Endnote {
                            number: note.number,
                            before_decoration_letter: note.before_decoration_letter,
                            after_decoration_letter: note.after_decoration_letter,
                            number_shape: note.number_shape,
                            instance_id: note.instance_id,
                            list_header_property: note.list_header_property,
                            decoration_is_user_char: note.decoration_is_user_char,
                            paragraphs: Vec::new(),
                        }));
                    }
                }
            }
        }
    }
    core.set_document(doc);
    let mut saw_prefix = false;
    let mut marker_pages = 0;
    let mut footer_pages = 0;
    for page in 73..84 {
        let tree = core.build_page_render_tree(page).expect("표25 주변 쪽");
        let marker = table_for_para(&tree.root, 885).is_some_and(|table| has_marker(table, 116));
        let footer = notes(&tree.root).is_some_and(|area| text(area).contains("116)"));
        if let Some(table) = table_for_para(&tree.root, 885) {
            let rows = visible_rows(table);
            if rows.contains(&3) && !rows.contains(&4) {
                assert!(!marker, "분할 행의 뒤쪽 표시가 앞 조각에 중복됨");
                assert!(
                    !footer,
                    "같은 행의 아직 표시되지 않은116 각주가 앞 쪽에 등록됨"
                );
                saw_prefix = true;
            }
        }
        if marker {
            marker_pages += 1;
        }
        if footer {
            assert!(
                marker,
                "단일 작은 각주의 몸통은 marker를 출력한 쪽에 소속되어야 함"
            );
            footer_pages += 1;
        }
    }
    assert!(saw_prefix, "같은 행 안에서 끝난 첫 조각을 실행해야 함");
    assert_eq!(marker_pages, 1, "marker 누락/중복 금지");
    assert_eq!(footer_pages, 1, "footer 누락/중복 금지");
}

/// 한컴 PDF p66은 머리행+본문4행, p67은 나머지2행이다. 원본 개체
/// 11645HU도 첫5행의 저장 높이 합과 같고 각주77은 2줄 뒤 vpos=0으로 재시작한다.
/// 조각 높이 합만으로는 행 소유와 각주 prefix/tail의 보존을 입증할 수 없다.
#[test]
fn saved_rows_and_footnote_reset_keep_their_physical_page_owners() {
    let core = core();
    let first = core.build_page_render_tree(65).expect("66쪽");
    let next = core.build_page_render_tree(66).expect("67쪽");
    let first_table = host_table(&first.root).expect("66쪽 표");
    let next_table = host_table(&next.root).expect("67쪽 표");
    assert_eq!(visible_rows(first_table), (0..5).collect(), "PDF 첫 5행");
    assert_eq!(
        visible_rows(next_table),
        [5, 6].into_iter().collect(),
        "PDF 끝 2행, 중복 없음"
    );
    let first_notes = notes(&first.root).expect("66쪽 각주");
    let next_notes = notes(&next.root).expect("67쪽 각주");
    let a = text(first_notes);
    let b = text(next_notes);
    assert!(
        a.contains("77)") && a.contains("Subchapter G"),
        "77번 앞 두 줄: {a}"
    );
    assert!(!a.contains("Part 482(CONDITIONS"), "77번 tail은 67쪽: {a}");
    assert!(!b.contains("77)"), "tail에서 번호 중복 금지: {b}");
    assert!(
        b.contains("Part 482(CONDITIONS") && b.contains("78)"),
        "77번 tail과 후속 각주: {b}"
    );
    for (table, area) in [(first_table, first_notes), (next_table, next_notes)] {
        assert!(
            table.bbox.y + table.bbox.height <= area.bbox.y,
            "표/각주 충돌 금지"
        );
    }
}

/// 각주를 든 RowBreak 자리차지 표가 쪽 경계에서 쪼개져 앞 조각이 66쪽에 남는다.
#[test]
fn rowbreak_table_with_own_footnotes_splits_at_the_page_boundary() {
    let core = core();

    // 66쪽(0-based 65) — 한/글이 앞 조각을 두는 쪽.
    let tree66 = core.build_page_render_tree(65).expect("66쪽 render tree");
    let on66 = host_tables(&tree66.root);
    assert_eq!(
        on66.len(),
        1,
        "66쪽에 문단 {HOST_PARA} 표 조각이 없다 — 표가 통째로 다음 쪽으로 밀렸다(수정 전 상태). \
         한/글 정본은 이 표의 각주 77번을 66쪽에 둔다. 실제: {on66:?}"
    );

    // 67쪽 — 이어받은 조각.
    let tree67 = core.build_page_render_tree(66).expect("67쪽 render tree");
    let on67 = host_tables(&tree67.root);
    assert_eq!(
        on67.len(),
        1,
        "67쪽에 이어받은 조각이 없다 — 분할이 두 쪽에 걸치지 않았다. 실제: {on67:?}"
    );

    // 앞 조각은 쪽 아래쪽에서 시작하고, 이어받은 조각은 본문 상단에서 시작한다.
    let (first_top, first_h) = on66[0];
    let (next_top, _) = on67[0];
    assert!(
        first_top > 400.0,
        "66쪽 조각이 쪽 위에서 시작한다 ({first_top:.1}) — 앞 본문 뒤에 이어 붙어야 한다"
    );
    assert!(
        next_top < 200.0,
        "67쪽 조각이 본문 상단에서 시작하지 않는다 ({next_top:.1})"
    );
    // 두 조각의 합이 통짜 높이(약 213.5px)를 넘지 않는다 — 같은 행을 두 번 그리지 않는다.
    let (_, next_h) = on67[0];
    assert!(
        first_h + next_h < 260.0,
        "두 조각 높이 합 {:.1}px 이 통짜 표보다 크다 — 행이 중복됐을 수 있다",
        first_h + next_h
    );
}

/// 반례 대조군 — 표 안 각주가 **없는** 같은 형상의 표는 종전대로 두 쪽에 걸쳐 쪼개진다.
///
/// 문단 0.866 은 host·`RowBreak`·`treat_as_char`·`wrap`·`vert`/`horz`·행 수가 위 표와 같고
/// **표 안 각주만 0건**이다. 이 수정이 각주 없는 표의 분할을 건드리지 않았음을 잠근다.
///
/// 쪽 번호는 이 수정으로 앞쪽이 줄면서 밀리므로(수정 전 77/78 · 수정 후 76/77) **절대
/// 번호로 고정하지 않는다.** 그래서 이 시험은 수정 전에도 통과한다 — 결함 검출 증거가
/// 아니라 회귀 잠금이다.
#[test]
fn footnote_free_rowbreak_table_keeps_splitting() {
    const FOOTNOTE_FREE_HOST: usize = 866;
    fn count_on(core: &DocumentCore, page: u32, para: usize) -> usize {
        fn walk(node: &RenderNode, para: usize, out: &mut usize) {
            if let RenderNodeType::Table(table) = &node.node_type {
                if table.para_index == Some(para) {
                    *out += 1;
                }
            }
            for child in &node.children {
                walk(child, para, out);
            }
        }
        let Ok(tree) = core.build_page_render_tree(page) else {
            return 0;
        };
        let mut n = 0;
        walk(&tree.root, para, &mut n);
        n
    }
    let core = core();
    // 70..85쪽 구간에서 이 표가 나타나는 쪽을 모은다(번호 고정 없이).
    let pages: Vec<u32> = (70..85)
        .filter(|&p| count_on(&core, p, FOOTNOTE_FREE_HOST) > 0)
        .collect();
    assert_eq!(
        pages.len(),
        2,
        "표 안 각주가 없는 표(문단 {FOOTNOTE_FREE_HOST})가 두 쪽에 걸쳐 쪼개지지 않는다. 실제 쪽: {pages:?}"
    );
    assert_eq!(
        pages[1],
        pages[0] + 1,
        "두 조각이 연속한 쪽에 있지 않다: {pages:?}"
    );
}
