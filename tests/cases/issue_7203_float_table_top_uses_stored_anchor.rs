//! #7203: 저장1×1 앵커의 원점과 후속 표 본문 점유 계약.
//! API 문서의 렌더링 회귀는 #7445로 이관하고 독립 규제영향분석서 검사를 유지한다.

use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};
use std::path::Path;

#[test]
fn saved_outer_box_anchor_keeps_following_tables_inside_the_body() {
    // 저장1×1 앵커 사다리는 선언 높이와 위·아래 여백을 합한 만큼 전진한다.
    // 이 값은 흐름 원점이다. 위여백 뒤의 앵커로 해석하면 뒤 표의 예약 높이가
    // 19.5px 늘어나므로 두 원점을 혼동하지 않는다.
    let bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("samples/issue6111/56345_regulatory_impact_analysis.hwp"),
    )
    .unwrap();
    let core = rhwp::document_core::DocumentCore::from_bytes(&bytes).unwrap();
    let tree = core.build_page_render_tree(19).unwrap();
    fn check(node: &RenderNode, body_bottom: Option<f64>, found: &mut bool) {
        let bottom = if matches!(node.node_type, RenderNodeType::Column(_)) {
            Some(node.bbox.y + node.bbox.height)
        } else {
            body_bottom
        };
        if let RenderNodeType::Table(meta) = &node.node_type {
            if let Some(vpos) = match meta.para_index {
                Some(357) => Some(43556),
                Some(358) => Some(45988),
                _ => None,
            } {
                let expected = 75.6 + f64::from(vpos) * 96.0 / 7200.0;
                assert!(
                    (node.bbox.y - expected).abs() < 0.05,
                    "stored flow origin: actual={}, expected={expected}",
                    node.bbox.y
                );
            }
            if meta.para_index == Some(359) {
                *found = true;
                let table_bottom = node.bbox.y + node.bbox.height;
                assert!(
                    table_bottom <= bottom.expect("column") + 0.5,
                    "measured table bottom {table_bottom} exceeds body {bottom:?}"
                );
            }
        }
        for child in &node.children {
            check(child, bottom, found);
        }
    }
    let mut found = false;
    check(&tree.root, None, &mut found);
    assert!(found, "target picture/caption table must remain present");
}
