//! [#7486] 새 문서에서 Enter 만 눌러 본문이 1쪽을 넘치면 2쪽이 생겨야 한다.
//!
//! 끝 쪽의 빈 문단만 남은 쪽을 버리는 마무리(`discard_terminal_blank_only_page`)가
//! 편집 흐름이 넘쳐 연 쪽까지 버리면, 새 문단이 어느 쪽에도 속하지 않아 캐럿이
//! 2쪽으로 가지 못한다.
#![cfg(not(target_arch = "wasm32"))]

use rhwp::wasm_api::HwpDocument;

#[test]
fn enter_past_page_end_opens_second_page() {
    let mut doc = HwpDocument::create_empty();
    doc.create_blank_document_native()
        .expect("create blank document");
    assert_eq!(doc.page_count(), 1);

    // 기본 A4 본문은 10pt 빈 줄 40여 개를 담는다. 60번이면 반드시 넘친다.
    const SPLITS: usize = 60;
    for para in 0..SPLITS {
        doc.split_paragraph_native(0, para, 0, None)
            .expect("split paragraph");
    }

    assert_eq!(doc.page_count(), 2);
    doc.get_cursor_rect_native(0, SPLITS, 0)
        .expect("넘친 새 문단은 2쪽에 놓여야 한다");
}
