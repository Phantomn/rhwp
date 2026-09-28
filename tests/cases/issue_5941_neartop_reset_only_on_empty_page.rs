//! #5921 거의 빈 쪽의 near-top 리셋 완화 계약.
//! 큰 실물 문서의 잠정203쪽 검사는 사용자 지시로 #7445에서 피델리티 개선 후 다시 검토한다.

#![cfg(not(target_arch = "wasm32"))]

use rhwp::wasm_api::HwpDocument;

/// `#5921` 의 원 계약은 그대로 — 거의 빈 쪽에서는 완화가 걸려 1쪽이어야 한다.
#[test]
fn issue_5921_empty_page_relaxation_still_applies() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples/task2136/neartop_reset_sb2500.hwpx");
    let bytes =
        std::fs::read(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let doc = HwpDocument::from_bytes(&bytes).expect("parse fixture");
    assert_eq!(
        doc.page_count(),
        1,
        "거의 빈 쪽(채움 2%)에서는 #5921 완화가 그대로 걸려 1쪽이어야 한다"
    );
}
