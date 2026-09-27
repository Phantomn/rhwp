//! 캡션 없는 저장 표의 바깥 위여백은 첫 조각과 새 쪽 조각의 프레임에 속한다.
//! 기대 좌표는 같은 원본의 한컴 PDF에서 관측한 96dpi 괘선이다.

use rhwp::document_core::DocumentCore;
use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};

fn table(node: &RenderNode, owner: usize) -> Option<&RenderNode> {
    if matches!(&node.node_type, RenderNodeType::Table(t) if t.para_index == Some(owner)) {
        return Some(node);
    }
    node.children.iter().find_map(|child| table(child, owner))
}

fn assert_frame(sample: &str) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(sample);
    let core =
        DocumentCore::from_bytes(&std::fs::read(path).expect("기준 원본")).expect("기준 원본 열기");
    assert_eq!(core.page_count(), 3, "기준 PDF의 쪽 소유");
    let pages = core.dump_page_items_json(None);
    let fragments = pages
        .as_array()
        .expect("쪽 목록")
        .iter()
        .flat_map(|page| page["columns"].as_array().expect("단 목록"))
        .flat_map(|column| column["items"].as_array().expect("소유 항목"))
        .filter(|item| item["kind"] == "partialTable" && item["paraIndex"] == 29)
        .collect::<Vec<_>>();
    assert_eq!(fragments.len(), 2, "누락/중복 없이 두 조각");
    for (item, start, end, continuation) in
        [(fragments[0], 0, 5, false), (fragments[1], 5, 8, true)]
    {
        assert_eq!(item["startRow"], start, "기준 PDF의 시작 행 소유");
        assert_eq!(item["endRow"], end, "기준 PDF의 끝 행 소유");
        assert_eq!(item["isContinuation"], continuation);
        assert_eq!(
            item["startCut"],
            serde_json::json!([]),
            "행 내부 유닛 누락 금지"
        );
        assert_eq!(
            item["endCut"],
            serde_json::json!([]),
            "행 내부 유닛 중복 금지"
        );
    }
    let first_page = core.build_page_render_tree(1).expect("첫 분할 조각 쪽");
    let first = table(&first_page.root, 29).expect("2쪽 pi29 표");
    let last_page = core.build_page_render_tree(2).expect("이어받기 쪽");
    let last = table(&last_page.root, 29).expect("3쪽 pi29 표");
    fn painted_borders(node: &RenderNode) -> (f64, f64, f64) {
        let lines = node
            .children
            .iter()
            .filter_map(|child| {
                if let RenderNodeType::Line(line) = &child.node_type {
                    if (line.y1 - line.y2).abs() < 0.01
                        && (line.x2 - line.x1).abs() > node.bbox.width - 1.0
                    {
                        return Some((line.y1, line.style.width));
                    }
                }
                None
            })
            .collect::<Vec<_>>();
        assert!(lines.len() >= 2, "표 외곽 괘선의 실제 paint 좌표");
        let top = lines
            .iter()
            .map(|line| line.0)
            .fold(f64::INFINITY, f64::min);
        let bottom = lines
            .iter()
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .expect("마지막 괘선");
        // 표의 점유 끝은 마지막 괘선의 실제 선 두께까지 포함할 수 있다.
        assert!(
            node.bbox.y + node.bbox.height <= bottom.0 + bottom.1 / 2.0 + 0.05,
            "본문 점유가 실제 외곽선 뒤로 늘어남: {} > {}",
            node.bbox.y + node.bbox.height,
            bottom.0
        );
        (top, bottom.0, bottom.1)
    }
    let (first_top, first_bottom, _) = painted_borders(first);
    let (last_top, last_bottom, _) = painted_borders(last);
    assert!(
        (last_top - 77.356_038).abs() < 0.6,
        "{sample} 이어받기 괘선 상단{last_top}, 독립 PDF77.356038"
    );
    assert!(
        (last_bottom - 296.156_006).abs() < 0.6,
        "{sample} 이어받기 괘선 하단{last_bottom}, 독립 PDF296.156006"
    );
    assert!(
        (first_top - 440.478_678).abs() < 0.6,
        "{sample} 첫 괘선 상단{first_top}, 독립 PDF440.478678"
    );
    // 첫 조각 하단의 기존 잔여 차이는 별도 계측하며 상단 수정으로 일치를 주장하지 않는다.
    assert!(
        (first_bottom - 1_042.540_039).abs() < 1.3,
        "{sample} 첫 괘선 하단{first_bottom}, 독립 PDF1042.540039"
    );
    assert!(
        pages.as_array().expect("쪽 목록")[2]["columns"]
            .as_array()
            .expect("단 목록")
            .iter()
            .flat_map(|column| column["items"].as_array().expect("소유 항목"))
            .any(|item| item["kind"] == "fullParagraph" && item["paraIndex"] == 30),
        "표 뒤 빈 줄의 흐름 소유도 마지막 쪽에 보존"
    );
}

#[test]
fn native_captionless_rowbreak_keeps_independent_frame_origins() {
    assert_frame("samples/issue_1133.hwp");
}

#[test]
fn hwpx_captionless_rowbreak_keeps_independent_frame_origins() {
    assert_frame("samples/hwpx/issue_1133.hwpx");
    assert_terminal_frame_reserved_once();
}

fn assert_terminal_frame_reserved_once() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples/86712_regulatory_analysis.hwp");
    let mut core = DocumentCore::from_bytes(&std::fs::read(path).expect("종료 컷 원본"))
        .expect("종료 컷 문서");
    let original_page = core.build_page_render_tree(27).expect("원본 종료 쪽");
    let original = table(&original_page.root, 172).expect("원본 종료 조각");
    let original_x = original.bbox.x;
    let original_width = original.bbox.width;
    let mut document = core.document().clone();
    let host = &mut document.sections[0].paragraphs[172];
    assert!(host.text.is_empty() && host.controls.len() == 1);
    assert_eq!(host.line_segs.len(), 1, "실제 빈 저장 앵커");
    assert_eq!(host.line_segs[0].segment_width, 0);
    let rhwp::model::control::Control::Table(control) = &mut host.controls[0] else {
        panic!("종료 컷 표");
    };
    assert!(control.caption.is_none());
    // 원본의 가로 기준만 바꾼 제어된 IR이다. 세로 저장 정보는 수정하지 않는다.
    // 빈 호스트의 왼쪽 원점/표 폭은 같고, 가로 기준은 세로 프레임을 중복 예약하지 않는다.
    control.common.horz_rel_to = rhwp::model::shape::HorzRelTo::Column;
    core.set_document(document);
    assert_eq!(
        core.page_count(),
        64,
        "종료 컷과 일반 프레임의 중복 예약 금지"
    );
    let page = core.build_page_render_tree(27).expect("종료 컷 28쪽");
    let frame = table(&page.root, 172).expect("종료 조각");
    let following = table(&page.root, 182).expect("후속 표");
    assert!((frame.bbox.x - original_x).abs() < 0.01, "가로 원점 동등성");
    assert!(
        (frame.bbox.width - original_width).abs() < 0.01,
        "측정 너비 보존"
    );
    // 원본 한컴2024 PDF28쪽의 세로 괘선이다. 변형의 한컴 출력 일치를 주장하지 않는다.
    assert!(
        (frame.bbox.y - 77.514_667).abs() < 0.5,
        "한 프레임의 위여백은 한 번만 paint: {}",
        frame.bbox.y
    );
    assert!(
        (following.bbox.y - 541.488).abs() < 1.0,
        "중복 예약으로 후속 표를 밀면 안 됨: {}",
        following.bbox.y
    );
    let items = core.dump_page_items_json(Some(27));
    let fragment = items[0]["columns"]
        .as_array()
        .expect("단 목록")
        .iter()
        .flat_map(|column| column["items"].as_array().expect("항목"))
        .find(|item| item["paraIndex"] == 172 && item["kind"] == "partialTable")
        .expect("종료 조각 실제 소유");
    assert_eq!(fragment["startRow"], 27);
    assert_eq!(fragment["endRow"], 28);
    assert!(!fragment["startCut"].as_array().expect("시작 컷").is_empty());
    assert_eq!(fragment["endCut"], serde_json::json!([]));
}
