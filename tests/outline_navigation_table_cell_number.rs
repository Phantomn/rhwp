//! 개요 탐색의 번호·제목·수준 질의 계약.
//!
//! #7445로 이관한 두 합성 입력의 저장 의미만 확인한다.
//! 기존 함수 이름은 유지하지만 페이지·SVG 대조는 제거했으며,
//! 이 계약의 통과를 최종 렌더링 승인으로 사용하지 않는다.

use rhwp::wasm_api::HwpDocument;

const FIXTURE: &str = "mydocs/pr/assets/issue7445/outline_navigation_table_cell_number.hwpx";
const DEMO_FIXTURE: &str = "mydocs/pr/assets/issue7445/outline_navigation_panel_demo.hwpx";

fn outline_entries(doc: &HwpDocument) -> Vec<(String, String, u64)> {
    let json: serde_json::Value =
        serde_json::from_str(&doc.get_outline_navigation().unwrap()).expect("개요 탐색 JSON");
    json["outline"]
        .as_array()
        .expect("outline 배열")
        .iter()
        .map(|item| {
            (
                item["number"].as_str().unwrap_or_default().to_owned(),
                item["title"].as_str().unwrap_or_default().to_owned(),
                item["level"].as_u64().unwrap_or_default(),
            )
        })
        .collect()
}

#[test]
fn outline_numbers_match_rendered_numbers_across_table_cell_number() {
    let bytes = std::fs::read(FIXTURE).unwrap();
    let doc = HwpDocument::from_bytes(&bytes).unwrap();

    let entries = outline_entries(&doc);

    // 개요 속성이 붙은 문단만 — 표 셀 문단과 `1. 일반 본문`(텍스트만 번호) 은 빠진다.
    assert_eq!(
        entries.len(),
        3,
        "개요 문단 3개만 나와야 한다 (실제: {entries:?})"
    );
    assert_eq!(
        entries,
        vec![
            ("1.".to_owned(), "개요".to_owned(), 1),
            ("가.".to_owned(), "목적".to_owned(), 2),
            // 표 셀의 NUMBER 문단이 카운터를 2 로 밀어낸 뒤라 3. 이다.
            ("3.".to_owned(), "요구사항".to_owned(), 1),
        ],
    );
}

/// #7445로 이관한 데모의 개요 번호·제목·수준 질의 계약만 유지한다.
///
/// 기존 함수 이름은 유지하지만 쪽 번호·SVG 대조는 피델리티 재구축 전까지 제외한다.
#[test]
fn panel_demo_outline_matches_rendered_document() {
    let bytes = std::fs::read(DEMO_FIXTURE).unwrap();
    let doc = HwpDocument::from_bytes(&bytes).unwrap();

    let entries = outline_entries(&doc);
    let shape: Vec<(&str, &str, u64)> = entries
        .iter()
        .map(|(number, title, level)| (number.as_str(), title.as_str(), *level))
        .collect();

    assert_eq!(
        shape,
        vec![
            ("1.", "총칙", 1),
            ("가.", "목적", 2),
            ("1)", "배경", 3),
            ("2)", "적용 범위", 3),
            ("나.", "용어 정의", 2),
            ("2.", "본문 규정", 1),
            ("가.", "요구사항", 2),
            ("1)", "기능 요구", 3),
            ("2)", "비기능 요구", 3),
            ("나.", "제약 조건", 2),
            ("3.", "표가 낀 구간", 1),
            ("가.", "표 뒤 하위 개요", 2),
            // 표 셀의 NUMBER 문단이 4 를 가져갔으므로 다음 최상위 개요는 5. 다.
            ("5.", "부칙", 1),
            ("가.", "시행일", 2),
            ("나.", "경과 조치", 2),
        ],
    );
}
