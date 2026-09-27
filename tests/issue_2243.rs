//! Issue #2243 — 결재문서의 쪽수와 표·후속 문단의 물리 배치 회귀 검사.
//!
//! 병인: 표 경로의 vpos 앵커 미확립/일괄 말소로 후속 문단 스냅이 드리프트를
//! 역산한 lazy base 에 고착 → 페이지당 +16~22px 팬텀 → razor 경계에서 sliver
//! 쪽(+1~+2). 수정: 표-경로 vpos 앵커 확립 + 저장 사다리 조건부 유지 + 빈 앵커
//! float 표 host 직후 lazy 이중 계상 가드 + 저장-앵커 safety 마진 면제.
//! 기존 쪽수만으로는 컨설팅의 붙임 문단 이동과 세운의 제목/표 순서 반전을
//! 놓쳤다. 기대값은 `pdf/task2243/*-hwpx-2020.pdf`의 쪽수, 표 벡터 경계와
//! 텍스트 기준선이며, PDF 포인트를 96dpi로 변환해 최종 출력과 대조한다.

use std::fs;
use std::path::Path;

use rhwp::document_core::DocumentCore;
use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};

/// (샘플, 한글 실측 쪽수)
const PINS: &[(&str, u32)] = &[
    // 한컴 PDF 5쪽. 표와 붙임의 위치도 아래에서 검사한다.
    ("samples/task2243/36395325_gyeoljae_consulting.hwpx", 5),
    // 빈 앵커 float 표 host 직후 lazy 이중 계상 가드 (4→3쪽)
    ("samples/task2243/36382819_gyeoljae_pm_traffic.hwpx", 3),
    // 지속 결함 복구 (7→5쪽)
    ("samples/task2243/36386907_gyeoljae_sewoon.hwpx", 5),
    // 저장-앵커 safety 마진 면제 razor (2→1쪽, stored 881.3+fit 49.6=930.9≤933.6)
    ("samples/task2243/156631374_taxi_press.hwpx", 1),
];

/// 본문 단의 항목만 모아 같은 문단 번호를 갖는 셀·머리말과 혼동하지 않는다.
fn column_items(root: &RenderNode) -> Vec<&RenderNode> {
    if matches!(root.node_type, RenderNodeType::Column(_)) {
        return root.children.iter().collect();
    }
    if matches!(
        root.node_type,
        RenderNodeType::Page(_) | RenderNodeType::Body { .. }
    ) {
        return root.children.iter().flat_map(column_items).collect();
    }
    Vec::new()
}

fn text_content(node: &RenderNode) -> String {
    if let RenderNodeType::TextRun(run) = &node.node_type {
        return run.text.clone();
    }
    node.children.iter().map(text_content).collect()
}

fn assert_pdf_coordinate(label: &str, actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1.0,
        "{label}: 실제 {actual:.3}px, 한컴 PDF {expected:.3}px"
    );
}

fn assert_table_bounds(
    core: &DocumentCore,
    page: u32,
    para: usize,
    expected: (Option<f64>, f64, f64),
) {
    let tree = core
        .build_page_render_tree(page - 1)
        .expect("본문 렌더 트리");
    let tables: Vec<_> = column_items(&tree.root)
        .into_iter()
        .filter(|node| {
            matches!(&node.node_type, RenderNodeType::Table(table)
            if table.para_index == Some(para) && table.control_index == Some(0)
                && table.cell_context.is_none())
        })
        .collect();
    assert_eq!(tables.len(), 1, "{page}쪽 문단 {para}: 표의 누락·중복");
    let bbox = tables[0].bbox;
    if let Some(x) = expected.0 {
        assert_pdf_coordinate("표 왼쪽", bbox.x, x);
    }
    assert_pdf_coordinate(&format!("{page}쪽 문단 {para} 표 위쪽"), bbox.y, expected.1);
    // 위치와 크기를 각각 PDF 벡터에서 독립적으로 검증한다. 하단 좌표에
    // 두 오차를 합산해 동일한 1px 기준을 적용하면 위치·크기 계약이 혼동된다.
    // 절대 하단의 잔여 차이는 Visual Sweep 증적에 별도로 기록한다.
    assert_pdf_coordinate(
        &format!("{page}쪽 문단 {para} 표 높이"),
        bbox.height,
        expected.2 - expected.1,
    );
    // 검사 대상은 표 조각이 아닌 완전한 표다. 본문 점유 경계도 지켜야 한다.
    fn body_bottom(node: &RenderNode) -> Option<f64> {
        if matches!(node.node_type, RenderNodeType::Body { .. }) {
            Some(node.bbox.y + node.bbox.height)
        } else {
            node.children.iter().find_map(body_bottom)
        }
    }
    assert!(
        bbox.y + bbox.height <= body_bottom(&tree.root).expect("본문 경계") + 0.5,
        "{page}쪽 문단 {para}: 표가 본문 하단을 넘음"
    );
}

fn assert_text_baseline(core: &DocumentCore, page: u32, para: usize, text: &str, expected: f64) {
    let tree = core
        .build_page_render_tree(page - 1)
        .expect("본문 렌더 트리");
    let lines: Vec<_> = column_items(&tree.root)
        .into_iter()
        .filter(|node| {
            matches!(&node.node_type, RenderNodeType::TextLine(line)
            if line.para_index == Some(para))
                && text_content(node).contains(text)
        })
        .collect();
    assert_eq!(
        lines.len(),
        1,
        "{page}쪽 문단 {para}: {text:?}의 누락·중복·잘못된 소유"
    );
    let RenderNodeType::TextLine(line) = &lines[0].node_type else {
        unreachable!()
    };
    assert_pdf_coordinate(text, lines[0].bbox.y + line.baseline, expected);
}

#[test]
fn issue_2243_gyeoljae_sliver_page_pins() {
    let repo_root = env!("CARGO_MANIFEST_DIR");
    for (sample, expected) in PINS {
        let bytes = fs::read(Path::new(repo_root).join(sample))
            .unwrap_or_else(|e| panic!("read {sample}: {e}"));
        let core =
            DocumentCore::from_bytes(&bytes).unwrap_or_else(|e| panic!("parse {sample}: {e:?}"));
        assert_eq!(
            core.page_count(),
            *expected,
            "{sample}: 한컴 PDF 쪽수와 불일치"
        );
        match *sample {
            "samples/task2243/36395325_gyeoljae_consulting.hwpx" => {
                assert_table_bounds(&core, 5, 40, (None, 390.453, 547.081));
                assert_text_baseline(&core, 5, 41, "붙임", 595.200);
                assert_text_baseline(&core, 5, 42, "검토결과", 644.000);
            }
            "samples/task2243/36382819_gyeoljae_pm_traffic.hwpx" => {
                assert_table_bounds(&core, 3, 30, (None, 383.900, 584.800));
                assert_text_baseline(&core, 3, 41, "주민 의견 수렴 참고서", 861.120);
            }
            "samples/task2243/36386907_gyeoljae_sewoon.hwpx" => {
                assert_text_baseline(&core, 3, 35, "심사항목", 589.600);
                assert_table_bounds(&core, 3, 35, (Some(89.392), 609.733, 1022.881));
                assert_text_baseline(&core, 4, 39, "결", 251.040);
                assert_table_bounds(&core, 4, 39, (Some(89.392), 270.584, 1022.881));
                assert_text_baseline(&core, 5, 44, "붙임", 276.320);
            }
            _ => {}
        }
    }
}
