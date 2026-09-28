//! [#5128] HWP5-origin HWPX 표식·프로필과 IR 왕복 계약.
//!
//! #7445로 이관한 공식 배포본의 렌더링·페이지 검사는 제외하고,
//! section·문단·원본 표식 보존만 확인한다. 최종 출력 승인을 뜻하지 않는다.

use std::fs;
use std::path::Path;

use rhwp::document_core::DocumentCore;
use rhwp::model::document::HWP5_ORIGIN_HWPX_MARKER_PATH;
use rhwp::wasm_api::HwpDocument;

const SAMPLE: &str = "mydocs/pr/assets/issue7445/한글문서파일형식_5.0_revision1.3.hwp";
const EXPECTED_SECTIONS: usize = 6;
const EXPECTED_PARAS: usize = 619;

fn sample_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(SAMPLE)
}

fn load_bytes() -> Vec<u8> {
    let path = sample_path();
    fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn spec_revision13_export_hwpx_keeps_origin_marker_and_profile() {
    let data = load_bytes();
    let src = HwpDocument::from_bytes(&data).expect("parse source");
    let bytes = src.export_hwpx_native().expect("export hwpx");
    assert!(
        bytes.len() > 4 && &bytes[0..4] == b"PK\x03\x04",
        "산출물이 ZIP(HWPX) 매직으로 시작해야 한다"
    );

    let ir = rhwp::parse_document(&bytes).expect("parse exported ir");
    assert!(
        ir.hwpx_aux_entry(HWP5_ORIGIN_HWPX_MARKER_PATH).is_some(),
        "HWP5→HWPX 마커가 있어야 한다"
    );
    let profile = ir.layout_profile();
    assert!(
        !profile.native_hwp5_layout() && profile.hwp5_origin_hwpx(),
        "재파싱은 HWP5-origin HWPX 프로필이어야 한다"
    );
    assert!(
        profile.hwp5_stored_pagination_layout(),
        "HWP5-origin HWPX 는 원본과 같은 저장 pagination 계약을 써야 한다"
    );
    assert_eq!(ir.sections.len(), EXPECTED_SECTIONS);
    assert_eq!(
        ir.sections
            .iter()
            .map(|s| s.paragraphs.len())
            .sum::<usize>(),
        EXPECTED_PARAS
    );
}

#[test]
fn spec_revision13_ir_para_count_unchanged() {
    let data = load_bytes();
    let src_ir = rhwp::parse_document(&data).expect("src ir");
    let core = DocumentCore::from_bytes(&data).expect("core");
    let hwpx = core.export_hwpx_native().expect("export");
    let rt_ir = rhwp::parse_document(&hwpx).expect("rt ir");
    assert_eq!(src_ir.sections.len(), rt_ir.sections.len());
    for (i, (a, b)) in src_ir
        .sections
        .iter()
        .zip(rt_ir.sections.iter())
        .enumerate()
    {
        assert_eq!(
            a.paragraphs.len(),
            b.paragraphs.len(),
            "section[{i}] paragraph count"
        );
    }
}
