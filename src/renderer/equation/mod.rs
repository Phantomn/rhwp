//! 한컴 수식 스크립트 파싱 및 렌더링
//!
//! 수식 스크립트(버전 6.0)를 토큰화하고 AST로 변환한 뒤 SVG로 렌더링한다.
//! 참조: openhwp/docs/hwpx/appendix-i-formula.md
//!
//! 명령 분기는 [`dispatch`] 가 가족으로 분류하고, [`parser`] 가 같은 핸들러로
//! 소비한다. 동작은 바꾸지 않는다. 모듈 지도는 `README.md`, 매뉴얼은
//! `mydocs/manual/equation_module.md`.

pub mod ast;
#[cfg(target_arch = "wasm32")]
pub mod canvas_render;
pub(crate) mod dispatch;
pub mod layout;
pub(crate) mod legacy_hwpeq;
pub mod parser;
pub mod svg_render;
pub mod symbols;
pub mod tokenizer;

/// 기본 글자보다 낮은 저장 수식 상자는 압축된 표시 프레임이다.
/// 일반 수식 높이에는 줄 여백이 포함될 수 있으므로 기존 글꼴 비율을 유지한다.
/// 압축 프레임에서는 배치 기준선과 모든 출력 backend가 같은 세로 비율을 쓴다.
pub(crate) fn stored_vertical_scale(height: f64, intrinsic_height: f64, font_size: f64) -> f64 {
    if height > 0.0 && height < font_size && intrinsic_height > 0.0 {
        height / intrinsic_height
    } else {
        1.0
    }
}

/// 수식 스크립트와 BaseUnit에서 레이아웃이 소비할 intrinsic HWPUNIT 크기를 계산한다.
pub fn intrinsic_size_hwp(script: &str, font_size: u32) -> (u32, u32) {
    let font_size_px = super::hwpunit_to_px(font_size.max(1) as i32, super::DEFAULT_DPI);
    let tokens = tokenizer::tokenize(script);
    let ast = parser::EqParser::new(tokens).parse();
    let layout = layout::EqLayout::new(font_size_px).layout(&ast);
    (
        super::px_to_hwpunit(layout.width, super::DEFAULT_DPI).max(1) as u32,
        super::px_to_hwpunit(layout.height, super::DEFAULT_DPI).max(1) as u32,
    )
}
