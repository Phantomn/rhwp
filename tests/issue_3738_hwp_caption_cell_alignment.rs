//! Issue #3738 Stage 8: Center 정렬 셀의 Bottom caption 그림은 그림 본체와
//! caption을 하나의 시각 블록으로 정렬해야 한다.
//!
//! 개인정보를 제거한 실제 HWP의 23쪽 그림 21/22는 1×2 표의 Center 셀에
//! Bottom caption(각 5줄)을 둔다. caption을 제외하고 그림만 중앙 정렬하면
//! 그림과 caption이 약 50px 아래로 밀리고 caption이 다음 본문과 겹친다.
//! 동일 원본은 한컴오피스2024 저장본이며 정본 `pdf/정책연구용역사업 …-hwp-2024.pdf`
//! 23쪽 그림21 본체 윗변은113.996002pt = 151.99467px, 캡션 첫 줄은
//! 373.741241pt = 498.32166px(96DPI)다. 종전 검사는2024 기준 x에 과거2020 설명의
//! y=148.3/495.2를 섞어 정본 자체를±3px 범위 밖으로 판정했다. 독립 좌표를 사용하고 공차는 유지한다.

use std::fs;
use std::path::Path;

use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};
use rhwp::wasm_api::HwpDocument;

const SAMPLE: &str =
    "samples/정책연구용역사업 중간진도보고서(살아있는 간장 기증자의 의학적 선별기준 연구).hwp";
const PAGE_23: u32 = 22;
const PDF_IMAGE_TOP_PX: f64 = 151.994_67;
const PDF_CAPTION_TOP_PX: f64 = 498.321_66;

fn find_picture_21_and_caption(
    node: &RenderNode,
    image_y: &mut Option<f64>,
    caption_y: &mut Option<f64>,
) {
    match &node.node_type {
        // [#7063] 그림 x 101.9 → 105.7. 이 문서 정본(`pdf/정책연구용역사업 …-hwp-2024.pdf`)
        // 23쪽의 그림 21 은 x=105.70, caption `그림 21` 은 x=105.76 이다 — 종전 101.9 가
        // 3.8px(283HU) 짧았다. 감싼 자리차지 표가 자기 바깥여백 안으로 들어간 결과다.
        RenderNodeType::Image(_)
            if (node.bbox.x - 105.7).abs() < 2.0 && node.bbox.width > 250.0 =>
        {
            *image_y = Some(node.bbox.y);
        }
        RenderNodeType::TextRun(run)
            if node.bbox.x < 110.0
                && node.bbox.y > 450.0
                && run.text.trim_start().starts_with("그림") =>
        {
            *caption_y = Some(node.bbox.y);
        }
        _ => {}
    }
    for child in &node.children {
        find_picture_21_and_caption(child, image_y, caption_y);
    }
}

#[test]
fn hwp_page23_bottom_caption_is_centered_as_one_visual_block() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(SAMPLE);
    let bytes = fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let doc = HwpDocument::from_bytes(&bytes).expect("parse stage8 HWP evidence fixture");
    let tree = doc
        .build_page_render_tree(PAGE_23)
        .expect("render HWP physical page 23");

    let mut image_y = None;
    let mut caption_y = None;
    find_picture_21_and_caption(&tree.root, &mut image_y, &mut caption_y);
    let image_y = image_y.expect("figure 21 image node");
    let caption_y = caption_y.expect("figure 21 caption text node");

    assert!(
        (image_y - PDF_IMAGE_TOP_PX).abs() <= 3.0,
        "그림21 본체가 캡션을 제외하고 다시 중앙 정렬됨: image_y={image_y:.1}, 한컴2024 PDF={PDF_IMAGE_TOP_PX:.5}px (회귀 전198.4)"
    );
    assert!(
        (caption_y - PDF_CAPTION_TOP_PX).abs() <= 3.0,
        "그림21 캡션 첫 줄이 한컴2024 PDF와 어긋남: caption_y={caption_y:.1}, 정본={PDF_CAPTION_TOP_PX:.5}px (회귀 전544.7)"
    );
}
