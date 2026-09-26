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
    for (margin, gap, reduced_body_hu) in [
        (283, 850, 0),
        (30_000, 850, 0),
        (30_000, 31_000, 0),
        (32_700, 32_700, 1_500),
    ] {
        assert_terminal_caption_budget(margin, gap, reduced_body_hu, false);
    }
}

#[test]
fn terminal_caption_withholds_the_last_rowspan_unit_together() {
    assert_terminal_caption_budget(32_700, 32_700, 0, true);
}

/// 현재 조각의 예산만 부족하며 마지막 행 하나와 캡션은 새 쪽에 들어간다.
/// 이월하면서 그 행과 뒤 본문을 보존해야 한다.
#[test]
fn terminal_caption_last_single_row_defers_without_consuming_it() {
    assert_single_row_caption_deferral(rhwp::model::shape::CaptionDirection::Bottom, 0);
}

#[test]
fn opening_caption_last_single_row_reserves_its_full_object_frame() {
    assert_single_row_caption_deferral(rhwp::model::shape::CaptionDirection::Top, 0);
}

#[test]
fn paragraph_caption_frame_preserves_positive_vertical_offset() {
    assert_single_row_caption_deferral(rhwp::model::shape::CaptionDirection::Bottom, 2_250);
    assert_single_row_caption_deferral(rhwp::model::shape::CaptionDirection::Top, 2_250);
}

fn assert_single_row_caption_deferral(
    direction: rhwp::model::shape::CaptionDirection,
    vertical_offset: u32,
) {
    use rhwp::model::control::Control;
    let mut core = core();
    let mut doc = core.document().clone();
    let Control::Table(table) = &mut doc.sections[0].paragraphs[HOST_PARA].controls[0] else {
        panic!("원표");
    };
    table.cells.retain(|cell| cell.row == 6);
    for cell in &mut table.cells {
        cell.row = 0;
        for paragraph in &mut cell.paragraphs {
            paragraph
                .controls
                .retain(|control| !matches!(control, Control::Footnote(_)));
        }
    }
    table.row_count = 1;
    table.common.vertical_offset = vertical_offset;
    table.common.height = table
        .cells
        .iter()
        .map(|cell| cell.height)
        .max()
        .expect("마지막 행 높이");
    table.outer_margin_bottom = 32_700;
    let caption = table.caption.as_mut().expect("캡션");
    caption.spacing = 32_700;
    caption.direction = direction;
    let outer_top = f64::from(table.outer_margin_top) / 75.0;
    let following_para = doc.sections[0]
        .paragraphs
        .iter()
        .enumerate()
        .skip(HOST_PARA + 1)
        .find(|(_, para)| para.text.contains("42 CFR Part 482"))
        .map(|(index, _)| index)
        .expect("뒤 본문 원본");
    core.set_document(doc);
    let dump = core.dump_page_items_json(None);
    let owners: Vec<_> = dump
        .as_array()
        .expect("쪽")
        .iter()
        .filter(|page| {
            page["columns"].as_array().expect("단").iter().any(|col| {
                col["items"]
                    .as_array()
                    .expect("항목")
                    .iter()
                    .any(|item| item["paraIndex"].as_u64() == Some(HOST_PARA as u64))
            })
        })
        .collect();
    assert_eq!(
        owners.len(),
        1,
        "한 행을 누락/중복/빈 조각 없이 한 쪽에 보존"
    );
    let page = owners[0]["pageIndex"].as_u64().expect("쪽 번호") as u32;
    let tree = core.build_page_render_tree(page).expect("실제 소유 쪽");
    let table = host_table(&tree.root).expect("마지막 행");
    assert_eq!(visible_rows(table), BTreeSet::from([0]));
    let body_top = owners[0]["bodyArea"]["y"].as_f64().expect("본문 상단");
    let opening_caption_height = if matches!(direction, rhwp::model::shape::CaptionDirection::Top) {
        (1_000.0 + 32_700.0) / 75.0
    } else {
        0.0
    };
    let expected_table_top =
        body_top + outer_top + opening_caption_height + f64::from(vertical_offset) / 75.0;
    assert!(
        (table.bbox.y - expected_table_top).abs() <= 1.5,
        "중간 쪽에 과수용하지 않고 새 쪽에서 시작: table={}, body={body_top}",
        table.bbox.y
    );
    let caption_y = line_top(&tree.root, "표 23.").expect("최종 캡션");
    let body_bottom = body_top + owners[0]["bodyArea"]["height"].as_f64().expect("본문 높이");
    let occupied_end = if matches!(direction, rhwp::model::shape::CaptionDirection::Top) {
        assert!(
            caption_y >= body_top - 0.5 && caption_y < table.bbox.y,
            "위 캡션은 본문 안에서 표보다 앞에 배치: {caption_y}/{}",
            table.bbox.y
        );
        table.bbox.y + table.bbox.height + 32_700.0 / 75.0
    } else {
        caption_y + (1_000.0 + 32_700.0) / 75.0
    };
    assert!(
        occupied_end <= body_bottom + 1.0,
        "캡션/표/바깥 여백 전체 수용: {occupied_end}/{body_bottom}"
    );
    let following_owner = dump
        .as_array()
        .expect("전체 쪽")
        .iter()
        .find(|candidate| {
            candidate["columns"]
                .as_array()
                .expect("단")
                .iter()
                .any(|column| {
                    column["items"]
                        .as_array()
                        .expect("항목")
                        .iter()
                        .any(|item| item["paraIndex"].as_u64() == Some(following_para as u64))
                })
        })
        .expect("뒤 본문 소유 쪽");
    let following_page = following_owner["pageIndex"].as_u64().expect("뒤 쪽") as u32;
    assert!(
        following_page >= page,
        "뒤 본문은 캡션보다 앞 쪽으로 가지 않음"
    );
    let following_tree = core
        .build_page_render_tree(following_page)
        .expect("뒤 본문 출력");
    let following_y = line_top(&following_tree.root, "○ 42 CFR Part 482").expect("뒤 본문 보존");
    if following_page == page {
        assert!(
            following_y >= occupied_end - 1.5,
            "표/캡션 종료 뒤 본문 비충돌: {following_y}/{occupied_end}"
        );
    }
}

fn assert_terminal_caption_budget(
    margin: i16,
    gap: i16,
    reduced_body_hu: u32,
    protect_terminal_rows: bool,
) {
    let mut core = core();
    let mut doc = core.document().clone();
    let rhwp::model::control::Control::Table(table) =
        &mut doc.sections[0].paragraphs[HOST_PARA].controls[0]
    else {
        panic!("표23")
    };
    if protect_terminal_rows {
        // 이 반례는 새 쪽에 들어가는 통째 캡션 유닛만 분리한다.
        // 원표의 각주 여섯 개와 키운 캡션은 한 새 쪽에 함께 들어가지 않으므로
        // 각주 소유는 별도의 실제 원본 검사로 확인한다.
        for cell in &mut table.cells {
            for paragraph in &mut cell.paragraphs {
                paragraph.controls.retain(|control| {
                    !matches!(control, rhwp::model::control::Control::Footnote(_))
                });
            }
        }
        // 마지막 행 병합 셀은 두 행의 텍스트를 모두 포함하는 분할 유닛 하나다.
        let lower = table
            .cells
            .iter()
            .position(|cell| cell.row == 6 && cell.col == 0)
            .expect("끝행 첫 셀");
        let tail = table.cells.remove(lower);
        let upper = table
            .cells
            .iter_mut()
            .find(|cell| cell.row == 5 && cell.col == 0)
            .expect("끝행 보호 블록 시작");
        upper.row_span = 2;
        upper.height += tail.height;
        upper.paragraphs.extend(tail.paragraphs);
    }
    table.outer_margin_bottom = margin;
    table.caption.as_mut().expect("아래 캡션").spacing = gap;
    doc.sections[0].section_def.page_def.margin_bottom += reduced_body_hu;
    core.set_document(doc);
    let mut rows = Vec::new();
    let mut caption_count = 0;
    // 물리 예산을 줄이면 앞 본문 문단도 이동할 수 있다.
    // 표의 실제 소유 쪽을 찾는다. 합성 입력에는 기준 PDF 페이지 번호가 없다.
    let owners: Vec<u32> = core
        .dump_page_items_json(None)
        .as_array()
        .expect("물리 페이지")
        .iter()
        .filter(|page| {
            page["columns"].as_array().expect("단").iter().any(|col| {
                col["items"]
                    .as_array()
                    .expect("항목")
                    .iter()
                    .any(|item| item["paraIndex"].as_u64() == Some(HOST_PARA as u64))
            })
        })
        .map(|page| page["pageIndex"].as_u64().expect("페이지 번호") as u32)
        .collect();
    for page in owners {
        let tree = core.build_page_render_tree(page).expect("변형 경계 쪽");
        if let Some(table) = host_table(&tree.root) {
            let owned_rows = visible_rows(table);
            if protect_terminal_rows && (owned_rows.contains(&5) || owned_rows.contains(&6)) {
                assert!(
                    owned_rows.contains(&5) && owned_rows.contains(&6),
                    "캡션 예산 때문에 마지막 rowspan 소유 유닛을 절단하지 않음: {owned_rows:?}"
                );
            }
            rows.extend(owned_rows.iter().copied());
            if reduced_body_hu > 0 && !owned_rows.contains(&6) {
                let dump = core.dump_page_items_json(Some(page));
                let column = &dump[0]["columns"][0];
                if column["itemCount"].as_u64() == Some(1) {
                    let body_y = dump[0]["bodyArea"]["y"].as_f64().expect("본문 원점");
                    let reserved = column["usedHeight"].as_f64().expect("예약 높이");
                    let painted_end = table.bbox.y + table.bbox.height - body_y;
                    // 종료 전 조각은 수용한 행만 소유한다.
                    // 캡션과 종료 아래 여백은 아직 소비하지 않았다.
                    assert!((reserved - painted_end).abs() <= 0.5,
                        "중간 조각 내용/물리 공간: page={}, rows={owned_rows:?}, reserved={reserved}, painted_end={painted_end}", page + 1);
                }
            }
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

/// 실제 한컴 PDF78 표25 첫 조각의 마지막 가로선970.937px.
/// 유닛 컷의 내용 소유가 맞아도 빈 밴드/셀 배치를 잃으면 물리 끝점이 다르다.
#[test]
fn first_large_table_fragment_preserves_pdf_physical_bottom() {
    let core = core();
    let tree = core.build_page_render_tree(77).expect("78쪽");
    let table = table_for_para(&tree.root, 885).expect("표25 첫 조각");
    assert!(
        (table.bbox.y - 527.104).abs() <= 1.5,
        "표25 위={}",
        table.bbox.y
    );
    let bottom = table.bbox.y + table.bbox.height;
    assert!(
        (bottom - 970.937).abs() <= 1.5,
        "표25 첫 끝={bottom}, 독립PDF970.937"
    );
}

/// 수용 예산보다 큰 수동 선언은 실제 저장 컷을 늘리는 근거가 아니다.
/// 합성 거부 대조군이며 한컴 출력 일치의 대용으로 쓰지 않는다.
#[test]
fn declared_opening_frame_cannot_spend_unavailable_body_space() {
    let mut core = core();
    let mut doc = core.document().clone();
    let rhwp::model::control::Control::Table(table) =
        &mut doc.sections[0].paragraphs[885].controls[0]
    else {
        panic!("표25")
    };
    table.common.height = 75_000; // 1000px: 한 쪽의 본문 높이보다 큰 거부 입력.
    core.set_document(doc);
    let tree = core.build_page_render_tree(77).expect("78쪽");
    let table = table_for_para(&tree.root, 885).expect("내용 앞 조각");
    let area = notes(&tree.root).expect("기존 각주 lane");
    assert!(
        table.bbox.y + table.bbox.height <= area.bbox.y + 0.5,
        "선언 상자의 강제 수용 금지: 표={:?}, 각주={:?}",
        table.bbox,
        area.bbox
    );
    assert!(
        table.bbox.height < 1000.0,
        "무효 선언을 실제 조각 높이로 수용하면 안 됨"
    );
}

/// 유한한 저장 첫 조각 안에서는 실제 소비한 셀 내용으로 원래 Center를 지킨다.
/// 내용 높이는 원본 세 문단의8개 저장 줄(900+272HU)에서 독립적으로 계산한다.
#[test]
fn saved_opening_frame_aligns_the_consumed_cell_prefix() {
    use rhwp::model::{control::Control, table::VerticalAlign};
    let core = core();
    let tree = core.build_page_render_tree(77).expect("78쪽");
    let node = table_for_para(&tree.root, 885).expect("첫 조각");
    let cell_node = node
        .children
        .iter()
        .find(|n| matches!(&n.node_type, RenderNodeType::TableCell(c) if c.row == 3 && c.col == 2))
        .expect("분할 셀");
    let Control::Table(table) = &core.document().sections[0].paragraphs[885].controls[0] else {
        panic!("표25")
    };
    let cell = table
        .cells
        .iter()
        .find(|c| c.row == 3 && c.col == 2)
        .expect("원본 셀");
    assert_eq!(cell.vertical_align, VerticalAlign::Center);
    let content: f64 = cell
        .paragraphs
        .iter()
        .take(3)
        .flat_map(|p| &p.line_segs)
        .map(|l| f64::from(l.line_height + l.line_spacing) / 75.0)
        .sum();
    let pad = cell.effective_padding(&table.padding);
    let top = f64::from(pad.top) / 75.0;
    let bottom = f64::from(pad.bottom) / 75.0;
    let expected = cell_node.bbox.y + top + (cell_node.bbox.height - top - bottom - content) / 2.0;
    let actual = line_top(cell_node, "[공통]").expect("첫 prefix 줄");
    assert!((actual - expected).abs() <= 0.5,
        "분할 뒤 내용의 높이가 아니라 소비한8줄로 Center: 실제{actual}, 기대{expected}, 원줄{content}");
    assert!((actual - 839.52).abs() <= 1.5, "독립 PDF 첫 prefix 위치");
}

/// 개수가 적다는 이유로 각주 영역을 표 아래 남은 공간보다 크게 수용하지 않는다.
/// 원본 marker/저장 줄을 유지한 합성 IR이며 한컴 출력의 대용은 아니다.
#[test]
fn small_note_queue_reserves_the_actual_painted_footnote_area() {
    use rhwp::model::{control::Control, footnote::Endnote};
    let mut core = core();
    let mut doc = core.document().clone();
    let Control::Table(table) = &mut doc.sections[0].paragraphs[885].controls[0] else {
        panic!("표25")
    };
    for cell in &mut table.cells {
        for para in &mut cell.paragraphs {
            for control in &mut para.controls {
                if let Control::Footnote(note) = control {
                    if ![107, 108].contains(&note.number) {
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
    let mut fragments = 0;
    let mut published = [0usize; 2];
    let mut following_lines = 0;
    fn check_following_lines(node: &RenderNode, top: f64, checked: &mut usize) {
        if let RenderNodeType::TextLine(line) = &node.node_type {
            if line.para_index.is_some_and(|pi| (886..=889).contains(&pi)) {
                *checked += 1;
                assert!(
                    node.bbox.y + node.bbox.height <= top + 0.5,
                    "terminal뒤본문도같은각주예약소비: line={:?}, 각주위={top}",
                    node.bbox
                );
            }
        }
        for child in &node.children {
            check_following_lines(child, top, checked);
        }
    }
    for page in 73..84 {
        let tree = core.build_page_render_tree(page).expect("표 주변");
        if let Some(area) = notes(&tree.root) {
            if let Some(body_node) = tree
                .root
                .children
                .iter()
                .find(|node| matches!(node.node_type, RenderNodeType::Body { .. }))
            {
                check_following_lines(body_node, area.bbox.y, &mut following_lines);
            }
            let body = text(area);
            for (index, number) in [107, 108].iter().enumerate() {
                if body.contains(&format!("{number})")) {
                    published[index] += 1;
                }
            }
            if let Some(table) = table_for_para(&tree.root, 885) {
                fragments += 1;
                assert!(
                    table.bbox.y + table.bbox.height <= area.bbox.y + 0.5,
                    "각주 개수로 물리 충돌을 숨기지 않음: page={}, 표끝={}, 각주위={}",
                    page + 1,
                    table.bbox.y + table.bbox.height,
                    area.bbox.y
                );
            }
        }
    }
    assert!(fragments > 0, "표/각주 공동 소유 쪽을 실제 실행");
    assert!(following_lines > 0, "각주 공동 소유 후속 본문을 실제 실행");
    assert_eq!(published, [1, 1], "각주 몸통 누락/중복 금지");
}

/// 원본 마지막 저장 줄43311+1000+padding282=첫 프레임44593HU.
/// PDF174는그림66/미치지않음.224)까지,175는나머지표·223..231각주를소유한다.
#[test]
fn single_cell_saved_frame_preserves_picture_and_note_page_owners() {
    let core = core();
    let first = core.build_page_render_tree(173).expect("174쪽");
    let next = core.build_page_render_tree(174).expect("175쪽");
    let following = core.build_page_render_tree(175).expect("176쪽");
    let a = table_for_para(&first.root, 1822).expect("174쪽첫조각");
    let b = table_for_para(&next.root, 1822).expect("175쪽꼬리");
    fn has_image(node: &RenderNode) -> bool {
        matches!(node.node_type, RenderNodeType::Image(_)) || node.children.iter().any(has_image)
    }
    assert!(has_image(a), "그림66은첫저장프레임174쪽소유");
    assert!(
        text(a).contains("미치지 않음"),
        "224번표시를가진첫프레임꼬리문장보존"
    );
    assert!(!text(b).contains("미치지 않음"), "첫문장꼬리중복금지");
    assert!(
        text(b).contains("연령은 주요한 이식 합병증"),
        "175쪽마지막셀문단보존"
    );
    assert!(
        table_for_para(&following.root, 1822).is_none(),
        "176쪽불필요한세번째조각금지"
    );
    let area = notes(&next.root).expect("175쪽각주");
    let body = text(area);
    for number in 223..=231 {
        assert_eq!(
            body.matches(&format!("{number})")).count(),
            1,
            "175쪽각주{number}소유"
        );
    }
    assert!(
        text(&next.root).contains("기증자 비만도"),
        "표뒤본문175쪽소유"
    );
    assert!(
        b.bbox.y + b.bbox.height <= area.bbox.y + 0.5,
        "표/각주비충돌"
    );
}

/// 같은 Hancom PDF174/175의 외곽선과 뒤 제목 좌표; 페이지 소유만으로 대체하지 않는다.
#[test]
fn single_cell_saved_frame_border_and_following_heading_match_pdf() {
    let core = core();
    let first = core.build_page_render_tree(173).expect("174쪽");
    let next = core.build_page_render_tree(174).expect("175쪽");
    let a = table_for_para(&first.root, 1822).expect("첫프레임");
    let b = table_for_para(&next.root, 1822).expect("마지막프레임");
    for (label, actual, expected) in [
        ("첫위", a.bbox.y, 433.127),
        ("첫끝", a.bbox.y + a.bbox.height, 1027.036),
        ("꼬리위", b.bbox.y, 86.945),
        ("꼬리끝", b.bbox.y + b.bbox.height, 583.361),
        (
            "뒤제목",
            line_top(&next.root, "기증자 비만도").expect("뒤제목줄"),
            641.061,
        ),
    ] {
        assert!(
            (actual - expected).abs() <= 1.5,
            "{label}: 실제{actual}, 독립PDF{expected}"
        );
    }
}

/// 실제 HWPX note234는 저장 두 줄의 vpos0/0을 보존한다. 독립 PDF176에는
/// 번호와 첫 줄,177에는 번호/구분선 없는 꼬리와 뒤 각주235가 있다.
#[test]
fn unqueued_cell_note_preserves_repeated_page_top_prefix_and_tail() {
    let core = core();
    let first = core.build_page_render_tree(175).expect("176쪽");
    let next = core.build_page_render_tree(176).expect("177쪽");
    let prefix = text(notes(&first.root).expect("176쪽 각주"));
    let suffix = text(notes(&next.root).expect("177쪽 각주"));
    assert!(prefix.contains("234)"), "234 번호는 표시 소유 쪽");
    assert!(
        !prefix.contains("severely steatotic"),
        "234 꼬리는 다음 물리 쪽"
    );
    assert!(suffix.contains("severely steatotic"), "234 꼬리 누락 금지");
    assert!(!suffix.contains("234)"), "이월 꼬리 번호 중복 금지");
    assert!(suffix.contains("235)"), "다음 정상 각주 보존");
    for (label, actual, expected) in [
        (
            "176 첫 각주",
            line_top(&first.root, "Dare AJ").expect("232 첫 줄"),
            949.169,
        ),
        (
            "177 이월 꼬리",
            line_top(&next.root, "severely steatotic").expect("234 꼬리"),
            996.209,
        ),
    ] {
        assert!(
            (actual - expected).abs() <= 1.5,
            "{label}: 실제{actual}, 독립PDF{expected}"
        );
    }
}

/// 수동 합성 0/0은 원본 저장 경계의 증거가 아니다. 물리 분할을 강제하지 않는다.
#[test]
fn synthetic_cell_note_zero_positions_do_not_force_a_physical_tail() {
    use rhwp::model::{control::Control, paragraph::LineSeg};
    let mut core = core();
    let mut doc = core.document().clone();
    let Control::Table(table) = &mut doc.sections[0].paragraphs[1832].controls[0] else {
        panic!("원표");
    };
    let mut changed = false;
    for cell in &mut table.cells {
        for p in &mut cell.paragraphs {
            for c in &mut p.controls {
                if let Control::Footnote(note) = c {
                    if note.number == 234 {
                        for para in &mut note.paragraphs {
                            for line in &mut para.line_segs {
                                line.tag |= LineSeg::TAG_IMPLEMENTATION_PROPERTY;
                            }
                        }
                        changed = true;
                    }
                }
            }
        }
    }
    assert!(changed, "반례의 원본 각주를 실제 변경");
    core.set_document(doc);
    let first = core.build_page_render_tree(175).expect("표시 쪽");
    let prefix = text(notes(&first.root).expect("각주"));
    assert!(prefix.contains("234)"));
    assert!(
        prefix.contains("severely steatotic"),
        "합성 위치로 실제 저장 경계를 발명하지 않음"
    );
}

/// PDF176/177의 단일 셀 통째 표는 다행 RowBreak와 같은 바깥 상자를 쓴다.
/// 괘선의 실제 상·하단을 검사하며 행 개수를 좌표 규칙의 근거로 삼지 않는다.
#[test]
fn whole_single_cell_rowbreak_border_includes_saved_outer_top() {
    let core = core();
    for (page, para, top, bottom) in [(175, 1832, 486.508, 903.012), (176, 1843, 539.729, 689.965)]
    {
        let tree = core.build_page_render_tree(page).expect("원본 영향 쪽");
        let table = table_for_para(&tree.root, para).expect("통째 단일 셀 표");
        assert!(
            (table.bbox.y - top).abs() <= 1.5,
            "p{} top: 실제{}, 독립PDF{}",
            page + 1,
            table.bbox.y,
            top
        );
        assert!(
            (table.bbox.y + table.bbox.height - bottom).abs() <= 1.5,
            "p{} bottom: 실제{}, 독립PDF{}",
            page + 1,
            table.bbox.y + table.bbox.height,
            bottom
        );
    }
}

/// 원본 guide 뒤 vpos50822 = 표높이50256 + 바깥여백283+283HU.
/// 독립 PDF182의 그림67과 뒤 본문,183의 그림68을 같은 페이지/좌표에서 검사한다.
#[test]
fn deferred_float_closed_source_frame_preserves_figure_and_following_body() {
    let core = core();
    let first = core.build_page_render_tree(181).expect("182쪽");
    let next = core.build_page_render_tree(182).expect("183쪽");
    let table = table_for_para(&first.root, 1904).expect("그림67 표");
    assert!(
        (table.bbox.y - 86.945).abs() <= 1.5,
        "그림67 첫 원점: {}",
        table.bbox.y
    );
    for (label, needle, expected) in [
        ("그림67 캡션", "그림 67.", 741.701),
        ("첫 뒤 본문", "매독 전파 사례", 787.461),
        ("다음 뒤 본문", "기생충 질환:", 894.021),
    ] {
        let actual = line_top(&first.root, needle).expect(label);
        assert!(
            (actual - expected).abs() <= 1.5,
            "{label}: 실제{actual}, 독립PDF{expected}"
        );
    }
    assert_eq!(
        text(&first.root).matches("기생충 질환:").count(),
        1,
        "원본 본문 누락/중복 금지"
    );
    assert!(
        !text(&next.root).contains("기생충 질환:"),
        "뒤 본문을 새 쪽에 다시 방출하지 않음"
    );
    assert!(table_for_para(&next.root, 1914).is_some(), "그림68은183쪽");
    assert_eq!(core.page_count(), 215, "독립 원본 PDF의 전체215쪽");
}

/// 같은 guide 좌표만으로 개체 소유를 증명할 수 없다. 종료 사다리 등식이
/// 깨진 합성 입력에서는 새 쪽 원점을 발명하지 않고 원래 앵커 오프셋을 유지한다.
#[test]
fn guide_overlap_without_closed_source_frame_keeps_anchor_offset() {
    let mut core = core();
    let mut doc = core.document().clone();
    assert_eq!(
        doc.sections[0].paragraphs[1910].line_segs[0].vertical_pos,
        50822
    );
    doc.sections[0].paragraphs[1910].line_segs[0].vertical_pos += 1000;
    core.set_document(doc);
    let page = core.build_page_render_tree(181).expect("합성 반례 쪽");
    let table = table_for_para(&page.root, 1904).expect("원표");
    // 본문 상단83.1733 + 원래 문단 오프셋3022/75 =123.4667px.
    assert!(
        (table.bbox.y - 123.4667).abs() <= 0.1,
        "guide 겹침만으로 새 원점을 수용하지 않음: {}",
        table.bbox.y
    );
}

/// 원본 note240의 두 번째0은 PDF178/179의 실제 물리 각주 경계다.
#[test]
fn body_note_repeated_page_top_survives_hwpx_parser() {
    use rhwp::model::control::Control;
    let core = core();
    let Control::Footnote(note) = &core.document().sections[0].paragraphs[1865].controls[0] else {
        panic!("원본 본문 각주");
    };
    assert_eq!(note.number, 240);
    assert_eq!(
        note.paragraphs[0]
            .line_segs
            .iter()
            .map(|s| s.vertical_pos)
            .collect::<Vec<_>>(),
        vec![0, 0, 1172]
    );
}

#[test]
fn body_note_repeated_page_top_keeps_prefix_tail_and_following_notes() {
    let core = core();
    let first = core.build_page_render_tree(177).expect("178쪽");
    let next = core.build_page_render_tree(178).expect("179쪽");
    let prefix = text(notes(&first.root).expect("178 각주"));
    let suffix = text(notes(&next.root).expect("179 각주"));
    assert!(prefix.contains("240)"));
    assert!(!prefix.contains("HTLV-1"), "꼬리를 앞쪽에서 소비하지 않음");
    assert!(
        suffix.contains("HTLV-1") && suffix.contains("jikeisurgery.jp"),
        "꼬리 두 줄 보존"
    );
    assert!(!suffix.contains("240)"), "번호 중복 금지");
    assert!(
        suffix.contains("241)") && suffix.contains("242)"),
        "뒤 정상 각주 보존"
    );
    for (label, actual, expected) in [
        (
            "앞 번호 줄",
            line_top(&first.root, "B형 간염 과거력").expect("앞줄"),
            1027.569,
        ),
        (
            "꼬리 첫 줄",
            line_top(&next.root, "HTLV-1").expect("꼬리"),
            964.689,
        ),
        (
            "꼬리 출처",
            line_top(&next.root, "jikeisurgery.jp").expect("출처"),
            980.369,
        ),
    ] {
        assert!(
            (actual - expected).abs() <= 1.5,
            "{label}: 실제{actual}, 독립PDF{expected}"
        );
    }
}

/// 합성 위치나 저장 줄이 없는 입력으로 물리 owner를 발명하지 않는다.
#[test]
fn body_note_invalid_saved_reset_stays_atomic() {
    use rhwp::model::{control::Control, paragraph::LineSeg};
    for synthetic in [true, false] {
        let mut core = core();
        let mut doc = core.document().clone();
        let Control::Footnote(note) = &mut doc.sections[0].paragraphs[1865].controls[0] else {
            panic!("원본 본문 각주");
        };
        if synthetic {
            for line in &mut note.paragraphs[0].line_segs {
                line.tag |= LineSeg::TAG_IMPLEMENTATION_PROPERTY;
            }
        } else {
            note.paragraphs[0].line_segs.clear();
        }
        core.set_document(doc);
        let first = core.build_page_render_tree(177).expect("앞쪽");
        let prefix = text(notes(&first.root).expect("각주"));
        assert!(
            prefix.contains("240)")
                && prefix.contains("HTLV-1")
                && prefix.contains("jikeisurgery.jp"),
            "유효하지 않은 저장 줄은 각주를 분할하지 않음: {prefix}"
        );
    }
}

/// 한컴 PDF31의 첫 두 줄은 앞쪽 본문407의 stored reset 뒤 꼬리다.
#[test]
fn body_first_note_reservation_preserves_saved_reset_tail_owner() {
    let core = core();
    let first = core.build_page_render_tree(29).expect("30쪽");
    let next = core.build_page_render_tree(30).expect("31쪽");
    assert!(
        !text(&first.root).contains("문제가 나타남. 조직학적"),
        "reset 뒤 두 줄을 앞쪽에서 소비하지 않음"
    );
    for (needle, expected) in [
        ("문제가 나타남. 조직학적", 83.141),
        ("대한 추적 관찰이 필요함", 109.861),
    ] {
        let actual = line_top(&next.root, needle).expect("독립PDF의 다음 본문 소유");
        assert!(
            (actual - expected).abs() <= 1.5,
            "{needle}: {actual} vs PDF{expected}"
        );
    }
    assert!(text(notes(&first.root).expect("30각주")).contains("29)"));
    assert!(!text(notes(&next.root).expect("31각주")).contains("29)"));
}

/// 본문421의 마지막 줄은 PDF32에서 차트35보다 앞에 점유한다.
#[test]
fn body_two_line_note_preserves_reset_tail_before_following_picture() {
    let core = core();
    let first = core.build_page_render_tree(30).expect("31쪽");
    let next = core.build_page_render_tree(31).expect("32쪽");
    let needle = "와 같이 점차 감소하는 추세임";
    assert!(
        !text(&first.root).contains(needle),
        "본문 꼬리를 앞쪽에서 소비하지 않음"
    );
    let actual = line_top(&next.root, needle).expect("차트 앞 본문 꼬리");
    assert!((actual - 83.141).abs() <= 1.5, "꼬리{actual} vs PDF83.141");
    assert!(text(notes(&first.root).expect("앞 각주")).contains("30)"));
    let tail = text(notes(&next.root).expect("꼬리각주"));
    assert!(tail.contains("Transplantationszentren") && !tail.contains("30)"));
}

/// 합성 되감김은 원본 저장 경계의 대용이 아니며 이 physical route를 켜지 않는다.
#[test]
fn synthetic_body_reset_does_not_create_a_saved_footnote_boundary() {
    use rhwp::model::paragraph::LineSeg;
    let mut core = core();
    let mut doc = core.document().clone();
    for line in &mut doc.sections[0].paragraphs[407].line_segs {
        line.tag |= LineSeg::TAG_IMPLEMENTATION_PROPERTY;
    }
    core.set_document(doc);
    let first = core.build_page_render_tree(29).expect("변형 앞쪽");
    assert!(
        text(&first.root).contains("문제가 나타남. 조직학적"),
        "합성 되감김을 각주 소유 증거로 쓰지 않음"
    );
}

/// 번호 없는 이월 꼬리에도 한컴의 해당 물리 쪽 각주 구분선은 남는다.
fn assert_continued_body_note_separator(core: DocumentCore) {
    for (page, expected_y) in [(30, 1018.725), (31, 1018.725)] {
        let tree = core.build_page_render_tree(page).expect("실제 각주 쪽");
        let area = notes(&tree.root).expect("각주 영역");
        let lines: Vec<_> = area
            .children
            .iter()
            .filter_map(|n| match &n.node_type {
                RenderNodeType::Line(line) => Some(line),
                _ => None,
            })
            .collect();
        assert_eq!(lines.len(), 1, "물리{}쪽 구분선 누락·중복 금지", page + 1);
        let line = lines[0];
        for (actual, expected) in [
            (line.x1, 94.509),
            (line.x2, 283.528),
            (line.y1, expected_y),
            (line.y2, expected_y),
        ] {
            assert!(
                (actual - expected).abs() <= 1.5,
                "구분선 좌표{actual} vs 독립PDF{expected}"
            );
        }
        let note = text(area);
        if page == 30 {
            assert!(note.contains("30)"));
        } else {
            assert!(
                !note.contains("30)") && note.contains("Transplantationszentren"),
                "번호 없는 꼬리 보존"
            );
            let y = line_top(area, "Transplantationszentren").expect("꼬리 줄");
            assert!((y - 1027.569).abs() <= 1.5, "꼬리 위치{y}");
        }
    }
    // 꼬리와 정상 각주가 함께 있어도 페이지당 구분선을 한 번만 칠한다.
    let tree = core
        .build_page_render_tree(178)
        .expect("각주240 꼬리와241/242");
    let area = notes(&tree.root).expect("뒤 각주 영역");
    assert_eq!(
        area.children
            .iter()
            .filter(|n| matches!(n.node_type, RenderNodeType::Line(_)))
            .count(),
        1
    );
}

#[test]
fn hwpx_continued_body_note_keeps_page_separator_without_number_repeat() {
    assert_continued_body_note_separator(core());
}

#[test]
fn hwp_continued_body_note_keeps_page_separator_without_number_repeat() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(SAMPLE.replace(".hwpx", ".hwp"));
    let bytes = std::fs::read(path).expect("같은 보고서 원본HWP");
    assert_continued_body_note_separator(DocumentCore::from_bytes(&bytes).expect("HWP 로드"));
}

/// 뒤 본문 다음의 쪽 경계로 그 본문 앞 빈 줄의 점유를 지우지 않는다.
/// 동일 입력 한컴 PDF32의 세 줄 좌표와 저장53340→55340HU가 독립 근거다.
#[test]
fn empty_line_before_visible_body_keeps_its_height_before_later_page_reset() {
    let core = core();
    let tree = core.build_page_render_tree(31).expect("32쪽 실제 본문");
    for (needle, expected) in [
        ("생존 간 기증을 기증자 이식대상자 관계", 821.061),
        ("연관계(형제자매", 847.621),
        ("기증(배우자", 874.341),
    ] {
        let actual = line_top(&tree.root, needle).expect("빈 줄 뒤 본문 보존");
        assert!(
            (actual - expected).abs() <= 1.5,
            "{needle}: 실제{actual}, 독립PDF{expected}"
        );
    }
    assert_eq!(core.page_count(), 215, "원본 쪽 수 보존");
}

/// 동일 저장 표를 담은 직접 HWPX도 원본 한컴의 온전한 행 점유로 분할한다.
#[test]
fn stored_rewinding_table_preserves_whole_row_footprint_and_caption() {
    let core = core();
    let first = core.build_page_render_tree(105).expect("106쪽 표");
    let next = core.build_page_render_tree(106).expect("107쪽 이어받기");
    let first_table = table_for_para(&first.root, 1136).expect("표29 앞 조각");
    let next_table = table_for_para(&next.root, 1136).expect("표29 뒤 조각");
    assert_eq!(visible_rows(first_table), BTreeSet::from([0, 1, 2]));
    assert_eq!(visible_rows(next_table), BTreeSet::from([3, 4, 5, 6, 7]));
    for (label, actual, expected) in [
        ("앞 표 상단", first_table.bbox.y, 670.947),
        (
            "앞 표 하단",
            first_table.bbox.y + first_table.bbox.height,
            986.121,
        ),
        (
            "뒤 표 하단",
            next_table.bbox.y + next_table.bbox.height,
            517.833,
        ),
        (
            "끝 캡션",
            line_top(&next.root, "표 29.").expect("캡션"),
            529.714,
        ),
        (
            "뒤 본문",
            line_top(&next.root, "O 미성년자").expect("뒤 본문"),
            592.901,
        ),
    ] {
        assert!(
            (actual - expected).abs() <= 1.5,
            "{label}: 실제{actual}, 독립PDF{expected}"
        );
    }
}

/// 표의 실제 끝점을 예약해야 뒤 본문1144의 물리 경계를 같은 쪽에서 소비한다.
#[test]
fn stored_rewinding_table_keeps_following_body_tail_before_figure() {
    let core = core();
    let first = core.build_page_render_tree(106).expect("107쪽 본문");
    let next = core.build_page_render_tree(107).expect("108쪽 꼬리와 그림");
    let needle = "적으로 적합하다는 결정은 주치의가";
    assert!(
        !text(&first.root).contains(needle),
        "본문 꼬리 조기 소비 금지"
    );
    let tail = line_top(&next.root, needle).expect("그림 앞 원본 꼬리");
    assert!((tail - 83.141).abs() <= 1.5, "꼬리{tail} vs PDF83.141");
    let title = line_top(&next.root, "1) 장기 기증 관련 결정 순서").expect("그림 제목");
    assert!((title - 189.861).abs() <= 1.5, "제목{title} vs PDF189.861");
}

/// 셀 선언 높이는 최소값이며 실제 온전한 행 안 여백을 줄이는 근거가 아니다.
/// 원본510HU 위 여백과 같은 입력 한컴 PDF의 실제 글줄 위치를 검사한다.
#[test]
fn whole_row_cell_uses_its_allocated_height_for_saved_inner_margin() {
    let core = core();
    for (page, row, needle, expected) in [
        (105, 0, "법적 자격이 있는 관계 속에", 678.514),
        (106, 3, "어떠한 기존 관계 없는 지정", 91.794),
    ] {
        let tree = core.build_page_render_tree(page).expect("실제 표 조각");
        let table = table_for_para(&tree.root, 1136).expect("표29");
        let cell = table.children.iter().find(|node| {
            matches!(&node.node_type, RenderNodeType::TableCell(cell) if cell.row == row && cell.col == 0)
        }).expect("온전한 첫 열 셀");
        let y = line_top(cell, needle).expect("저장 첫 글줄");
        assert!(
            (y - expected).abs() <= 1.5,
            "{}쪽 글줄{y} vs 독립PDF{expected}",
            page + 1
        );
        assert!(
            (y - cell.bbox.y - 510.0 / 75.0).abs() <= 0.01,
            "실제 셀 상자는 저장 위 여백6.8px를 소유: cell{}, line{y}",
            cell.bbox.y
        );
    }
}

/// 같은 원본의 한컴 PDF44/45는 앞쪽 본문의 저장 vpos=0 꼬리로 시작한다.
/// 시작 줄의 쪽 비율 대신 실제 저장 앵커와 흐름의 일치로 소유를 확인한다.
#[test]
fn anchored_body_reset_preserves_page_top_tail_below_fill_threshold() {
    let core = core();
    for (page, tail) in [(43, "(47.7%)이었음."), (44, "되었으며, <표 20>과 같음.")] {
        let before = core.build_page_render_tree(page - 1).expect("앞 조각 쪽");
        let next = core.build_page_render_tree(page).expect("꼬리 소유 쪽");
        let y = line_top(&next.root, tail).expect("독립 PDF의 첫 꼬리 글줄");
        assert!(
            (y - 83.141).abs() <= 1.5,
            "{}쪽 첫 글줄{y} vs PDF83.141",
            page + 1
        );
        assert!(
            !text(&before.root).contains(tail),
            "꼬리를 앞쪽에서 중복 소비하면 안 됨"
        );
    }
    let next = core.build_page_render_tree(44).expect("45쪽 뒤 표");
    let table = table_for_para(&next.root, 518).expect("뒤 표21");
    assert!(
        (table.bbox.y - 193.388).abs() <= 1.5,
        "뒤 표 원점{} vs PDF193.388",
        table.bbox.y
    );
    assert_eq!(core.page_count(), 215, "원본 한컴 PDF215쪽");
}

/// 저장 앵커가 현재 흐름과 다르거나 합성 줄이면 낮은 시작 높이의
/// reset만으로 새 물리 쪽 소유를 만들지 않는다.
#[test]
fn invalid_body_anchor_does_not_promote_low_fill_reset_to_page_owner() {
    use rhwp::model::paragraph::LineSeg;
    for synthetic in [true, false] {
        let mut core = core();
        let mut doc = core.document().clone();
        let paragraph = &mut doc.sections[0].paragraphs[512];
        if synthetic {
            for line in &mut paragraph.line_segs {
                line.tag |= LineSeg::TAG_IMPLEMENTATION_PROPERTY;
            }
        } else {
            paragraph.line_segs[0].vertical_pos -= 7500;
        }
        core.set_document(doc);
        let before = core
            .build_page_render_tree(42)
            .expect("유효하지 않은 앵커 대조군");
        assert!(
            text(&before.root).contains("(47.7%)이었음."),
            "증거가 없는 reset으로 꼬리를 새 쪽에 넘기면 안 됨"
        );
    }
}

/// 실제 텍스트 편집이 재구성한 줄은 저장 쪽 경계의 근거로 재사용하지 않는다.
/// 이 대조군은 편집 후 출력의 한컴 일치가 아니라 재조판 소유 계약을 확인한다.
#[test]
fn edited_body_reflow_does_not_reuse_original_saved_reset_owner() {
    use rhwp::model::paragraph::LineSeg;
    let mut core = core();
    let original = &core.document().sections[0].paragraphs[512].line_segs;
    assert_eq!(original[3].vertical_pos, 0, "원본의 실제 저장 reset");
    assert!(original
        .iter()
        .all(|line| line.tag & LineSeg::TAG_IMPLEMENTATION_PROPERTY == 0));
    core.insert_text_native(0, 512, 0, " ")
        .expect("실제 본문 편집 명령");
    let edited = &core.document().sections[0].paragraphs[512].line_segs;
    assert!(!edited.is_empty(), "실제 재조판 줄 생성");
    assert!(
        edited
            .windows(2)
            .all(|pair| pair[1].vertical_pos > pair[0].vertical_pos),
        "실제 재조판은 원본의 내부0 reset을 유지하지 않고 연속 줄 위치를 생성"
    );
    let before = core.build_page_render_tree(42).expect("재조판 앞쪽");
    let next = core.build_page_render_tree(43).expect("재조판 다음 쪽");
    let tail = "(47.7%)이었음.";
    assert!(
        text(&before.root).contains(tail),
        "원본 저장 컷 대신 실제 재조판 용량으로 소비"
    );
    assert!(!text(&next.root).contains(tail), "재조판 내용 중복 없음");
}

/// 빈 문단의 두 자리차지 표는 각 개체의 바깥 여백을 보존한다.
/// 독립 기대값은 동일 원본 한컴 PDF44쪽의 괘선과 캡션 상단이다.
#[test]
fn empty_host_sibling_tables_preserve_outer_margin_and_caption_origins() {
    fn collect<'a>(node: &'a RenderNode, out: &mut Vec<&'a RenderNode>) {
        if matches!(&node.node_type, RenderNodeType::Table(t) if t.para_index == Some(515)) {
            out.push(node);
        }
        for child in &node.children {
            collect(child, out);
        }
    }
    let core = core();
    let tree = core.build_page_render_tree(43).expect("44쪽 두 표");
    let mut tables = Vec::new();
    collect(&tree.root, &mut tables);
    assert_eq!(tables.len(), 2, "형제 표의 누락·중복 금지");
    tables.sort_by(|a, b| a.bbox.y.total_cmp(&b.bbox.y));
    for (table, top, bottom) in [
        (tables[0], 327.961344, 536.533325),
        (tables[1], 568.657349, 860.658691),
    ] {
        assert!(
            (table.bbox.y - top).abs() <= 1.5,
            "표 상단{} vs 독립 PDF{top}",
            table.bbox.y
        );
        assert!(
            ((table.bbox.y + table.bbox.height) - bottom).abs() <= 1.5,
            "표 하단{} vs 독립 PDF{bottom}",
            (table.bbox.y + table.bbox.height)
        );
    }
    for (needle, expected) in [("표 19. 일본의", 548.261027), ("표 20. 일본 생존", 872.901)]
    {
        let y = line_top(&tree.root, needle).expect("표 캡션");
        assert!(
            (y - expected).abs() <= 1.5,
            "캡션{needle} 상단{y} vs 독립 PDF{expected}"
        );
    }
    let body_y = line_top(&tree.root, "간 기증자의 수술 후 주요 합병증").expect("뒤 본문");
    assert!((body_y - 919.301025).abs() <= 1.5, "뒤 본문{body_y}");
    assert_eq!(core.page_count(), 215);
}

/// 한컴 PDF121의 표시와 각주159/160은 같은 쪽에 있고122에는161만 있다.
/// 문단1297의 앞7줄·뒤3줄 소유와 각주의 등록 시점을 구분한다.
fn assert_stored_body_multi_note_owner(core: DocumentCore) {
    let first = core.build_page_render_tree(120).expect("표시와 각주121쪽");
    let next = core.build_page_render_tree(121).expect("본문 꼬리122쪽");
    let area = notes(&first.root).expect("121쪽 각주");
    let following = notes(&next.root).expect("122쪽 각주");
    for (needle, expected) in [("159)", 1011.728597), ("160)", 1027.568604)] {
        let y = line_top(area, needle).expect("독립 PDF의 표시 쪽 각주");
        assert!(
            (y - expected).abs() <= 1.5,
            "각주{needle} 원점{y} vs 독립 PDF{expected}"
        );
    }
    assert!(
        !text(following).contains("160)"),
        "각주160 꼬리 쪽 중복·잘못된 소유 금지"
    );
    assert!(text(following).contains("161)"), "후행 정상 각주 보존");
    let y = line_top(&first.root, "Royal Decree 2070/1999").expect("실제 표시 첫 줄");
    assert!((y - 803.012614).abs() <= 1.5, "각주 표시의 본문 원점{y}");
    let y = line_top(&next.root, "야 함이 조건으로 추가됨.(Article 11)").expect("실제 뒤 본문");
    assert!((y - 136.421071).abs() <= 1.5, "다음 쪽 본문{y}");
    assert_eq!(core.page_count(), 215);
}

/// 합성 위치는 표시가 앞쪽에 있더라도 저장 쪽 경계로 승격하지 않는다.
#[test]
fn synthetic_body_tail_does_not_enable_saved_multi_note_routing() {
    use rhwp::model::paragraph::LineSeg;
    let mut core = core();
    let mut doc = core.document().clone();
    for line in &mut doc.sections[0].paragraphs[1297].line_segs {
        line.tag |= LineSeg::TAG_IMPLEMENTATION_PROPERTY;
    }
    core.set_document(doc);
    let first = core.build_page_render_tree(120).expect("합성 앞쪽");
    let next = core.build_page_render_tree(121).expect("합성 꼬리 쪽");
    assert!(!text(notes(&first.root).expect("기존 각주")).contains("160)"));
    assert!(text(notes(&next.root).expect("일반 각주 소유")).contains("160)"));
}

/// 소급 등록할 공간이 없는 큰 각주는 완료된 표시 쪽 본문을 침범하지 않는다.
/// 수동 대조군이며 한컴의 원본 출력 일치 증거로 사용하지 않는다.
#[test]
fn oversized_body_note_does_not_intrude_into_completed_marker_page() {
    use rhwp::model::{control::Control, paragraph::LineSeg};
    let mut core = core();
    let mut doc = core.document().clone();
    let Control::Footnote(note) = &mut doc.sections[0].paragraphs[1297].controls[0] else {
        panic!("원본 각주160");
    };
    let original = note.paragraphs[0].clone();
    for _ in 0..7 {
        let mut added = original.clone();
        added.controls.clear();
        for line in &mut added.line_segs {
            line.tag |= LineSeg::TAG_IMPLEMENTATION_PROPERTY;
        }
        note.paragraphs.push(added);
    }
    core.set_document(doc);
    let first = core.build_page_render_tree(120).expect("완료된 표시 쪽");
    let area = notes(&first.root).expect("기존 각주159 영역");
    assert!(
        !text(area).contains("160)"),
        "큰 각주를 완료 쪽에 강제 예약하면 안 됨"
    );
    let next = core.build_page_render_tree(121).expect("후행 소유 쪽");
    assert!(text(notes(&next.root).expect("큰 각주 영역")).contains("160)"));
}

/// 동일 원본 HWPX의 표시 쪽 각주와 뒤 본문을 함께 확인한다.
#[test]
fn stored_body_marker_keeps_whole_note_with_prior_notes_on_its_page() {
    assert_stored_body_multi_note_owner(core());
}

/// Native HWP의 기존 소유 계약은 독립 HWP 기준 PDF121/122와 같다.
#[test]
fn native_hwp_multi_note_owner_remains_on_original_marker_page() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(SAMPLE.replace(".hwpx", ".hwp"));
    let bytes = std::fs::read(path).expect("동일 보고서 원본 HWP");
    assert_stored_body_multi_note_owner(DocumentCore::from_bytes(&bytes).expect("HWP 로드"));
}

/// 음수 줄간격으로 흐름은 그대로여도 앞의 큰 줄 상자가 각주 영역을 점유한다.
/// 수동 메트릭 반례이며 한컴 원본 출력 일치 증거로 사용하지 않는다.
#[test]
fn taller_body_line_box_prevents_completed_page_note_intrusion() {
    let mut core = core();
    let mut doc = core.document().clone();
    let first = &mut doc.sections[0].paragraphs[1297].line_segs[0];
    // 원래 다음 줄까지의 2000HU 전진을 보존하되 첫 줄 상자는 15000HU다.
    first.line_height = 15_000;
    first.line_spacing = -13_000;
    core.set_document(doc);
    let first = core
        .build_page_render_tree(120)
        .expect("큰 줄 상자의 표시 쪽");
    let next = core
        .build_page_render_tree(121)
        .expect("각주를 수용할 꼬리 쪽");
    assert!(
        !text(notes(&first.root).expect("기존 각주159")).contains("160)"),
        "전진량이 작아도 큰 본문 줄 상자에 각주160을 소급 예약하면 안 됨"
    );
    assert!(text(notes(&next.root).expect("꼬리 쪽 각주")).contains("160)"));
}

/// Native 원본 PDF90의 캡션과 표 괘선은 서로 다른 물리 원점을 가진다.
#[test]
fn native_pre_emitted_caption_and_table_share_saved_paragraph_reference() {
    assert_saved_caption_table_reference(0, false);
}

/// 문단 기준 양수 오프셋을 바꾸면 표만 이동하며 캡션 소유는 그대로다.
/// 수동 오프셋 변형은 원본 PDF의 대용이 아닌 좌표 계약 대조군이다.
#[test]
fn native_pre_emitted_caption_preserves_positive_table_offset_change() {
    assert_saved_caption_table_reference(500, false);
}

fn assert_saved_caption_table_reference(extra_offset: u32, hwpx: bool) {
    use rhwp::model::control::Control;
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(if hwpx {
        SAMPLE.to_owned()
    } else {
        SAMPLE.replace(".hwpx", ".hwp")
    });
    let bytes = std::fs::read(path).expect("원본 HWP");
    let mut core = DocumentCore::from_bytes(&bytes).expect("HWP 로드");
    let mut doc = core.document().clone();
    let Control::Table(table) = &mut doc.sections[0].paragraphs[962].controls[0] else {
        panic!("원본 표27");
    };
    table.common.vertical_offset += extra_offset;
    core.set_document(doc);
    let first = core.build_page_render_tree(89).expect("캡션과 첫 조각");
    let next = core.build_page_render_tree(90).expect("이어받는 끝 행");
    let table = table_for_para(&first.root, 962).expect("첫 표 조각");
    let expected_top = 718.094727 + extra_offset as f64 * 96.0 / 7200.0;
    assert!(
        (table.bbox.y - expected_top).abs() <= 1.5,
        "문단 기준 표 상단: {} vs {expected_top}",
        table.bbox.y
    );
    if extra_offset == 0 {
        assert!(
            (table.bbox.y + table.bbox.height - 995.710693).abs() <= 1.5,
            "첫 조각 하단: {}",
            table.bbox.y + table.bbox.height
        );
    }
    let area = notes(&first.root).expect("첫 쪽 기존 각주 영역");
    assert!(
        table.bbox.y + table.bbox.height <= area.bbox.y + 0.5,
        "표가 실제 각주 영역을 침범하면 안 됨"
    );
    let caption = line_top(&first.root, "표 27.").expect("첫 쪽 캡션");
    assert!((caption - 696.421061).abs() <= 1.5, "캡션 원점: {caption}");
    assert!(
        caption + 13.333333 <= table.bbox.y + 0.5,
        "캡션 줄 상자가 표 괘선과 겹치면 안 됨"
    );
    assert!(visible_rows(table).contains(&5), "첫 쪽 관계 행 소유");
    let tail = table_for_para(&next.root, 962).expect("이어받기 표");
    assert_eq!(
        visible_rows(tail),
        BTreeSet::from([6]),
        "끝 행만 한 번 소비"
    );
    assert!(line_top(&next.root, "표 27.").is_none(), "캡션 중복 없음");
}

/// 독립 HWP 기준 PDF90은 문단957의 저장 꼬리 여덟 줄과 뒤 문단을 보존한다.
#[test]
fn native_stored_bullet_tail_preserves_saved_rows_and_following_paragraph() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(SAMPLE.replace(".hwpx", ".hwp"));
    let bytes = std::fs::read(path).expect("원본 HWP");
    let core = DocumentCore::from_bytes(&bytes).expect("HWP 로드");
    let para = &core.document().sections[0].paragraphs[957];
    assert_eq!(para.line_segs.len(), 9, "원본 저장 줄");
    assert!(para
        .line_segs
        .iter()
        .all(|line| line.column_start == 496 && line.segment_width == 44856));
    let page = core.build_page_render_tree(89).expect("Native 90쪽");
    fn collect<'a>(node: &'a RenderNode, pi: usize, lines: &mut Vec<&'a RenderNode>) {
        if matches!(&node.node_type, RenderNodeType::TextLine(line) if line.para_index == Some(pi))
        {
            lines.push(node);
        }
        for child in &node.children {
            collect(child, pi, lines);
        }
    }
    let mut lines = Vec::new();
    collect(&page.root, 957, &mut lines);
    assert_eq!(lines.len(), 8, "저장 줄1..9의 꼬리 소유");
    assert!(text(lines[0]).starts_with("Human Rights and Biomedicine)"));
    assert_eq!(text(lines[7]).trim(), "부재함.");
    assert!(
        (lines[7].bbox.y - 269.861).abs() <= 1.5,
        "꼬리 줄 원점: {}",
        lines[7].bbox.y
    );
    let mut following = Vec::new();
    collect(&page.root, 958, &mut following);
    assert_eq!(following.len(), 4, "후행 문단 저장 네 줄");
    assert!(
        (following[0].bbox.y - 296.421).abs() <= 1.5,
        "후행 본문 원점: {}",
        following[0].bbox.y
    );
}

/// HWPX 기준 PDF90도 관계 행까지 실제 각주 영역 앞에 보존한다.
#[test]
fn hwpx_stored_caption_table_keeps_relationship_row_above_actual_footnote_area() {
    assert_saved_caption_table_reference(0, true);
}

/// 수동 폭 변형과 합성 저장 줄은 원본 목록 분할의 수용 증거가 아니다.
#[test]
fn stored_bullet_origin_does_not_admit_wrong_width_or_synthetic_rows() {
    use rhwp::model::paragraph::LineSeg;
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(SAMPLE.replace(".hwpx", ".hwp"));
    let bytes = std::fs::read(path).expect("원본 HWP");
    for synthetic in [false, true] {
        let mut core = DocumentCore::from_bytes(&bytes).expect("HWP 로드");
        let mut doc = core.document().clone();
        for line in &mut doc.sections[0].paragraphs[957].line_segs {
            if synthetic {
                line.tag |= LineSeg::TAG_IMPLEMENTATION_PROPERTY;
            } else {
                line.segment_width -= 1;
            }
        }
        core.set_document(doc);
        let page = core
            .build_page_render_tree(89)
            .expect("원본 분할을 수용하지 않는 쪽");
        fn count(node: &RenderNode) -> usize {
            usize::from(
                matches!(&node.node_type, RenderNodeType::TextLine(line) if line.para_index == Some(957)),
            ) + node.children.iter().map(count).sum::<usize>()
        }
        assert_ne!(
            count(&page.root),
            8,
            "원본 여덟 저장 줄을 그대로 수용하면 안 됨: synthetic={synthetic}"
        );
        assert!(
            text(&page.root).contains("부재함."),
            "재조판이 꼬리 내용을 잃으면 안 됨"
        );
    }
}

fn assert_caption_note_owner(hwpx: bool, page: u32, number: u16, expected_y: f64) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(if hwpx {
        SAMPLE.to_owned()
    } else {
        SAMPLE.replace(".hwpx", ".hwp")
    });
    let bytes = std::fs::read(path).expect("정식 원본");
    let core = DocumentCore::from_bytes(&bytes).expect("문서 로드");
    assert_eq!(core.page_count(), 215, "독립 기준 PDF 쪽 수");
    let needle = format!("{number})");
    for index in [page - 2, page - 1, page] {
        let tree = core
            .build_page_render_tree(index)
            .expect("각주 소유 인접 쪽");
        fn note_line<'a>(node: &'a RenderNode, needle: &str) -> Option<&'a RenderNode> {
            if matches!(&node.node_type, RenderNodeType::TextLine(_)) && text(node).contains(needle)
            {
                return Some(node);
            }
            node.children
                .iter()
                .find_map(|child| note_line(child, needle))
        }
        let found = notes(&tree.root).and_then(|area| note_line(area, &needle));
        if index == page - 1 {
            let line = found.expect("분할 표 캡션 각주가 끝 조각 쪽에 있어야 함");
            // PDF는 가시 글자의 상단이고 렌더 트리는 줄 상자다. 동일 값으로 비교하지 않는다.
            // 기준 글자 상단이 실제 해당 각주 줄 상자에 속하는지와 인접 쪽 소유를 확인한다.
            assert!(
                line.bbox.y - 0.5 <= expected_y
                    && expected_y <= line.bbox.y + line.bbox.height + 0.5,
                "각주{number} 기준 글자 상단 {expected_y}는 실제 줄 상자 {}..{}에 속해야 함",
                line.bbox.y,
                line.bbox.y + line.bbox.height
            );
        } else {
            assert!(found.is_none(), "각주{number} 인접 쪽 중복/잘못된 소유");
        }
    }
}

/// 원본 HWPX PDF87의 표 캡션 각주138은 끝 조각 아래에 있다.
#[test]
fn hwpx_caption_note_138_is_preserved_on_terminal_table_page() {
    assert_caption_note_owner(true, 87, 138, 1027.556885);
}

/// 원본 HWPX PDF91의 캡션 각주142는 다른 본문 각주보다 먼저 보인다.
#[test]
fn hwpx_caption_note_142_is_preserved_before_following_body_notes() {
    assert_caption_note_owner(true, 91, 142, 920.368571);
}

/// 원본 HWPX PDF95의 URL 각주147도 끝 조각 소유를 따른다.
#[test]
fn hwpx_caption_note_147_is_preserved_on_terminal_table_page() {
    assert_caption_note_owner(true, 95, 147, 1011.888590);
}

/// Native 원본의 기존 번호 캡션 각주 소유·좌표는 같은 독립 출력에 부합한다.
#[test]
fn native_caption_notes_keep_existing_terminal_table_page_owners() {
    for (page, number, y) in [
        (87, 138, 1027.556885),
        (91, 142, 920.368571),
        (95, 147, 1011.888590),
    ] {
        assert_caption_note_owner(false, page, number, y);
    }
}
fn original_picture_wrapper_core(hwpx: bool) -> DocumentCore {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(if hwpx {
        SAMPLE.to_owned()
    } else {
        SAMPLE.replace(".hwpx", ".hwp")
    });
    DocumentCore::from_bytes(&std::fs::read(path).expect("정식 원본")).expect("문서 로드")
}

fn find_picture(root: &RenderNode) -> Option<&RenderNode> {
    if matches!(root.node_type, RenderNodeType::Image(_)) {
        Some(root)
    } else {
        root.children.iter().find_map(find_picture)
    }
}

fn assert_empty_opening_picture_fragment(hwpx: bool) {
    let core = original_picture_wrapper_core(hwpx);
    assert_eq!(core.page_count(), 215, "독립 기준 쪽 수");
    let page = core.build_page_render_tree(11).expect("12쪽");
    let table = table_for_para(&page.root, 250).expect("그림 앞의 빈 첫 물리 조각");
    // 두 셀의 괘선만 표시한 독립 한컴 PDF의 상단/하단이다.
    assert!(
        (table.bbox.y - 820.062663).abs() <= 1.5,
        "빈 조각 상단 {}",
        table.bbox.y
    );
    assert!(
        (table.bbox.y + table.bbox.height - 1002.263997).abs() <= 1.5,
        "빈 조각 하단 {}",
        table.bbox.y + table.bbox.height
    );
    assert!(
        find_picture(table).is_none(),
        "그림 유닛은 첫 빈 조각에서 소비하지 않음"
    );
    assert!(
        !text(table).contains("그림 8."),
        "캡션은 이어받는 쪽의 소유"
    );
    let area = notes(&page.root).expect("기존 각주7");
    assert!(text(area).contains("7)"));
    assert!(
        !text(area).contains("8)"),
        "이어받는 캡션 각주를 앞쪽에 등록하지 않음"
    );
}

fn assert_picture_fragment_remainder(hwpx: bool) {
    let core = original_picture_wrapper_core(hwpx);
    assert_eq!(core.page_count(), 215, "독립 기준 쪽 수");
    let page = core.build_page_render_tree(12).expect("13쪽");
    let table = table_for_para(&page.root, 250).expect("그림과 캡션의 이어받기 조각");
    let image = find_picture(table).expect("그림8");
    // 원본 PDF의 그림 외곽과 괘선 대조군의 동일 배치다.
    assert!(
        (image.bbox.y - 88.702637).abs() <= 1.5,
        "그림 상단 {}",
        image.bbox.y
    );
    assert!(
        (image.bbox.y + image.bbox.height - 325.403971).abs() <= 1.5,
        "그림 하단 {}",
        image.bbox.y + image.bbox.height
    );
    assert!(
        (table.bbox.y + table.bbox.height - 344.422689).abs() <= 1.5,
        "캡션 포함 표 하단 {}",
        table.bbox.y + table.bbox.height
    );
    assert!(
        text(table).contains("그림 8. 장기매매 유형"),
        "이어받는 캡션 보존"
    );
    let caption_top = line_top(table, "그림 8.").expect("그림8 캡션 줄");
    // 가시 글자 상단329.541056px는 저장 논리 줄의 내부에 있다.
    assert!(
        caption_top <= 329.541056 + 0.5 && 329.541056 <= caption_top + 1000.0 / 75.0 + 0.5,
        "독립 캡션 글자 상단과 실제 줄의 포함 관계 {caption_top}"
    );
    let next = line_top(&page.root, "2. 미국").expect("그림 뒤 본문");
    // 저장 줄 vpos23902HU와 본문 원점6239HU로 정한 논리 줄 상단이다.
    let expected = (6239.0 + 23902.0) * 96.0 / 7200.0;
    assert!(
        (next - expected).abs() <= 1.5,
        "뒤 본문 상단 {next}, 독립 저장 줄 {expected}"
    );
    let area = notes(&page.root).expect("13쪽 각주");
    for marker in ["8)", "9)", "10)"] {
        assert!(text(area).contains(marker), "각주 {marker}의 물리 쪽 소유");
    }
    assert!(!text(area).contains("7)"), "앞쪽 각주 중복 없음");
}

#[test]
fn hwpx_picture_wrapper_preserves_empty_opening_physical_fragment() {
    assert_empty_opening_picture_fragment(true);
}

#[test]
fn native_picture_wrapper_preserves_empty_opening_physical_fragment() {
    assert_empty_opening_picture_fragment(false);
}

#[test]
fn hwpx_picture_wrapper_continuation_consumes_remaining_physical_height() {
    assert_picture_fragment_remainder(true);
}

#[test]
fn native_picture_wrapper_continuation_consumes_remaining_physical_height() {
    assert_picture_fragment_remainder(false);
}

fn assert_interior_control_picture_table_anchor(hwpx: bool) {
    let core = original_picture_wrapper_core(hwpx);
    let host = &core.document().sections[0].paragraphs[246];
    assert_eq!(
        host.control_text_positions(),
        [207],
        "실제 원문 컨트롤 위치"
    );
    assert_eq!(host.line_segs[3].text_start, 171, "컨트롤 소유 줄 시작");
    assert_eq!(host.line_segs[4].text_start, 230, "다음 줄 시작");
    let tree = core.build_page_render_tree(11).expect("12쪽 그림7");
    let table = table_for_para(&tree.root, 246).expect("그림7 표");
    let image = find_picture(table).expect("그림7");
    // 원본의 원시 컨트롤207은 저장 줄171..230에 속한다. 줄vpos12000HU와
    // 개체offset3618HU·위여백283HU가 독립 PDF의 표/그림 상단을 결정한다.
    let top = (6239.0 + 12000.0 + 3618.0 + 283.0) / 75.0;
    assert!(
        (table.bbox.y - top).abs() <= 1.5,
        "컨트롤 소유 줄의 표 상단 {} vs {top}",
        table.bbox.y
    );
    assert!(
        (image.bbox.y - 296.794667).abs() <= 1.5,
        "독립 PDF 그림7 상단 {}",
        image.bbox.y
    );
    let next = line_top(&tree.root, "질적으로 매매가 이루어짐").or_else(|| {
        fn para_line(node: &RenderNode) -> Option<f64> {
            if matches!(&node.node_type, RenderNodeType::TextLine(line) if line.para_index == Some(247)) {
                return Some(node.bbox.y);
            }
            node.children.iter().find_map(para_line)
        }
        para_line(&tree.root)
    }).expect("그림7 뒤 문단247");
    assert!(
        (next - (6239.0 + 34718.0) / 75.0).abs() <= 1.5,
        "다음 문단 상단 {next}"
    );
}

#[test]
fn hwpx_picture_table_uses_interior_control_stored_line_anchor() {
    assert_interior_control_picture_table_anchor(true);
}

#[test]
fn native_picture_table_uses_interior_control_stored_line_anchor() {
    assert_interior_control_picture_table_anchor(false);
}

fn assert_square_sibling_table_outer_frames(hwpx: bool) {
    fn collect<'a>(node: &'a RenderNode, out: &mut Vec<&'a RenderNode>) {
        if matches!(&node.node_type, RenderNodeType::Table(t) if t.para_index == Some(259)) {
            out.push(node);
        }
        for child in &node.children {
            collect(child, out);
        }
    }
    let core = original_picture_wrapper_core(hwpx);
    assert_eq!(core.page_count(), 215, "원본 독립 PDF 쪽 수");
    let page = core.build_page_render_tree(12).expect("13쪽");
    let mut tables = Vec::new();
    collect(&page.root, &mut tables);
    assert_eq!(tables.len(), 2, "두 어울림 형제 표의 같은 쪽 소유");
    // 네 셀의 NONE 괘선만 SOLID로 바꾼 한컴 PDF의 실제 바깥 프레임이다.
    // 원본과 대조군은 215쪽 전체 텍스트·그림 bbox가 정확히 같다.
    for (table, x, y) in [
        (tables[0], 98.346670, 625.395996),
        (tables[1], 410.338664, 620.921346),
    ] {
        assert!(
            (table.bbox.x - x).abs() < 0.8,
            "바깥 왼쪽 여백 소유: {} != {x}",
            table.bbox.x
        );
        assert!(
            (table.bbox.y - y).abs() < 0.8,
            "바깥 위여백과 호스트 오프셋 소유: {} != {y}",
            table.bbox.y
        );
    }
    // 원본 HU 프레임:본문6239 + 호스트40102 + 개체오프셋/위여백
    // + 첫행(common.height-캡션행1282) + 캡션 셀 위패딩141이다.
    // PDF 가시 glyph 상단897.861/880.101과 논리 줄 원점을 동일값으로 비교하지 않는다.
    for (table, caption, logical_top) in [
        (
            tables[0],
            "표 2. OPTN",
            (6239.0 + 40102.0 + 334.0 + 283.0 + 21531.0 - 1282.0 + 141.0) / 75.0,
        ),
        (
            tables[1],
            "그림 9. OPTN",
            (6239.0 + 40102.0 + 283.0 + 20525.0 - 1282.0 + 141.0) / 75.0,
        ),
    ] {
        let y = line_top(table, caption).expect("원래 캡션 소유");
        assert!(
            (y - logical_top).abs() < 0.1,
            "독립 저장 HU 캡션 논리 줄: {y}/{logical_top}"
        );
        assert_eq!(text(table).matches(caption).count(), 1, "캡션 무중복");
    }
}

#[test]
fn hwpx_square_sibling_tables_consume_same_outer_frames_in_budget_and_paint() {
    assert_square_sibling_table_outer_frames(true);
}

#[test]
fn native_square_sibling_tables_consume_same_outer_frames_in_budget_and_paint() {
    assert_square_sibling_table_outer_frames(false);
}

fn square_sibling_frames(root: &RenderNode) -> Vec<(f64, f64)> {
    fn collect(node: &RenderNode, frames: &mut Vec<(f64, f64)>) {
        if matches!(&node.node_type, RenderNodeType::Table(t) if t.para_index == Some(259)) {
            frames.push((node.bbox.x, node.bbox.y));
        }
        for child in &node.children {
            collect(child, frames);
        }
    }
    let mut frames = Vec::new();
    collect(root, &mut frames);
    frames
}

#[test]
fn square_sibling_negative_offset_zero_control_preserves_original_frames() {
    use rhwp::model::control::Control;
    let mut core = core();
    let before = core.build_page_render_tree(12).expect("원본 쪽");
    let mut doc = core.document().clone();
    let Control::Table(table) = &mut doc.sections[0].paragraphs[259].controls[1] else {
        panic!("원본 둘째 표");
    };
    table.common.vertical_offset = 0;
    core.set_document(doc);
    assert_eq!(core.page_count(), 215);
    let after = core
        .build_page_render_tree(12)
        .expect("단일 속성 대조군 쪽");
    // 한컴 대조군은 전체215쪽 좌표가 원본과 같다. 수정 전에도 통과하는 정상 대조다.
    assert_eq!(
        square_sibling_frames(&before.root),
        square_sibling_frames(&after.root)
    );
}

#[test]
fn square_sibling_synthetic_host_does_not_admit_original_outer_frame() {
    use rhwp::model::paragraph::LineSeg;
    let mut core = core();
    let mut doc = core.document().clone();
    doc.sections[0].paragraphs[259].line_segs[0].tag |= LineSeg::TAG_IMPLEMENTATION_PROPERTY;
    core.set_document(doc);
    let page = core
        .build_page_render_tree(12)
        .expect("합성 저장 줄 대조군");
    let frames = square_sibling_frames(&page.root);
    assert_eq!(frames.len(), 2, "폴백에서도 형제 내용 소유 보존");
    assert!(
        (frames[0].0 - 94.49333333333334).abs() < 0.001,
        "합성 좌표를 유효 저장 프레임으로 승격하지 않음"
    );
    assert!(text(&page.root).contains("표 2. OPTN"));
    assert!(text(&page.root).contains("그림 9. OPTN"));
}
