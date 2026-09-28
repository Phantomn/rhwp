//! [Issue #7203] 자리차지 표의 윗변이 경로마다 다른 원점을 쓴다.
//!
//! ## 갈려 있던 두 산식
//!
//! 같은 이름의 게이트가 `layout.rs`(렌더)와 `typeset.rs`(조판)에 각각 있었고 값이 달랐다.
//!
//! ```text
//!   layout.rs   need = 높이 + 아래여백 − 위여백 ,  윗변 = vpos − 위여백
//!   typeset.rs  need = 높이 + 위여백 + 아래여백 ,  윗변 = vpos
//! ```
//!
//! 대칭 여백(283/283)이면 `need` 가 `2 × 위여백` = 566 HU 갈린다. `hwpctl_API_v2.4.hwp` 의
//! 어긋난 자리차지 표는 **예외 없이** 사다리 간격이 `높이 + 66 HU` 였다 — 렌더는 저장
//! 앵커를 수용하고 조판은 500 HU 모자라 거부하는 구간이다. 그래서 흐름이 표를 host 줄
//! 높이(1000 HU = 13.33px)만큼 위에 놓아 `−13.33 + 3.77 = −9.56px` 가 남았다.
//!
//! ## 기대값의 출처 — 구현과 독립
//!
//! 저장소 안 정본 `pdf/hwpctl_API_v2.4-hwp-2020.pdf` 의 가로 괘선과 원본 저장 사다리를
//! 대조하면 한 규칙이 나온다(9건 전부 잔차 +0.26 .. +0.74px).
//!
//! ```text
//!   정본 윗변 = 본문 상단 + (앵커 vpos − 위 바깥여백)
//! ```
//!
//! 이 검사는 그 규칙을 **문서 자신의 저장값**(`rhwp dump` 의 `ls[0] vpos` 와
//! `[outer_margin] top`)으로 세우고 실제 배치와 대조한다. rhwp 가 계산한 좌표를 다시
//! 인용하지 않는다.
//!
//! ## 수정
//!
//! `renderer::stored_float_anchor` 가 판정·원점의 정본이고 조판·렌더가 같은 값을 쓴다.
//! 종전의 "본문 하단 절반의 앵커만" 위치 게이트는 걷었다 — 정본이 상단 절반에서도 같은
//! 규칙을 쓰기 때문이다. 대신 **산식이 성립하는 조건**으로 좁혔다: 문단 기준
//! `vertical_offset` 이 걸린 개체는 윗변이 `앵커 + 오프셋` 이라 흐름 배치에 맡긴다.

#![cfg(not(target_arch = "wasm32"))]

use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};
use rhwp::wasm_api::HwpDocument;

const HWPUNIT_PER_PX: f64 = 7200.0 / 96.0;

fn page_root(sample: &str, page: u32) -> RenderNode {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(sample);
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("fixture 를 읽을 수 없다 ({}): {error}", path.display()));
    let document = HwpDocument::from_bytes(&bytes).expect("문서 로드");
    document
        .build_page_render_tree(page)
        .unwrap_or_else(|error| panic!("쪽 idx {page} render tree: {error:?}"))
        .root
}

fn find<'a>(
    node: &'a RenderNode,
    want: &mut impl FnMut(&RenderNode) -> bool,
) -> Vec<&'a RenderNode> {
    let mut out = Vec::new();
    fn walk<'a>(
        node: &'a RenderNode,
        want: &mut impl FnMut(&RenderNode) -> bool,
        out: &mut Vec<&'a RenderNode>,
    ) {
        if want(node) {
            out.push(node);
        }
        for child in &node.children {
            walk(child, want, out);
        }
    }
    walk(node, want, &mut out);
    out
}

fn table_top(root: &RenderNode, para_index: usize) -> f64 {
    let tables = find(root, &mut |node| match &node.node_type {
        RenderNodeType::Table(table) => table.para_index == Some(para_index),
        _ => false,
    });
    assert!(!tables.is_empty(), "문단 {para_index} 의 표가 이 쪽에 없다");
    // 한 문단이 여러 조각을 낼 수 있다 — 가장 위 조각이 윗변이다.
    tables
        .iter()
        .map(|node| node.bbox.y)
        .fold(f64::INFINITY, f64::min)
}

/// 저장 사다리 pi186→187은2432HU = 앞 개체 높이1300 + 위/아래여백566씩이다.
/// 뒤 표의 가시 윗변은 자기 앵커에 위여백141HU를 한 번 더한 위치다.
/// 두 표의 가시 상자 간격을 원시 앵커 간격과 혼동하지 않고 독립 PDF 괘선도 대조한다.
#[test]
fn stored_anchor_after_a_taller_float_consumes_its_span_once() {
    let root = page_root("samples/issue6111/56345_regulatory_impact_analysis.hwp", 10);
    let following_top = table_top(&root, 187);
    let advance = following_top - table_top(&root, 186);
    let stored_advance = (26992.0 - 24560.0) / HWPUNIT_PER_PX;
    // 원본 dump의 뒤 표 바깥 위여백141HU는 앞 개체가 닫는2432HU 밖에 있다.
    let following_outer_top = 141.0 / HWPUNIT_PER_PX;
    let visible_advance = stored_advance + following_outer_top;
    assert!(
        (advance - visible_advance).abs() <= 0.2,
        "저장 간격과 뒤 표 위여백 {visible_advance:.2}px 대신 {advance:.2}px를 소비했다"
    );
    // 독립 정본 PDF11쪽의 가로 괘선327.721008pt를96DPI로 변환한 좌표다.
    let pdf_following_top = 436.961_344;
    assert!(
        (following_top - pdf_following_top).abs() <= 1.5,
        "뒤 표 윗변이 독립 PDF와 다르다: 실제{following_top:.2}px, 기준{pdf_following_top:.6}px"
    );
}
