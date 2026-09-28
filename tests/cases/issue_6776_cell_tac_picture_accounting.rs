//! [#6776] 비-TAC 칸 그림이 칸 유닛 회계에 이중 반영되지 않는 대조군.
//!
//! 양성 문서는 #7445로 보존 이관했으며 시각 미달 렌더링 검사를 제외했다.
//! 이 검사는 다른 비-TAC 문서의 기존 페이지·그림 계약만 유지한다.
#![cfg(not(target_arch = "wasm32"))]

use std::path::Path;

use rhwp::document_core::DocumentCore;

/// 음성 대조 — 칸 안 그림이 **어울림(비-TAC)** 이라 회계에 들어가면 안 된다.
const NEGATIVE: &str = "samples/issue6776/36367506-water-facility-approval.hwpx";

/// 이 문서가 조판하는 A4 세로 종이 높이(px, 96dpi).
const PAPER_HEIGHT_PX: f64 = 1122.5;
fn open(sample: &str) -> DocumentCore {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(sample);
    DocumentCore::from_bytes(&std::fs::read(&path).unwrap_or_else(|e| panic!("read {sample}: {e}")))
        .unwrap_or_else(|e| panic!("open {sample}: {e}"))
}

/// SVG 의 `<image …>` 를 `(y, height)` 로 걷는다.
fn images(svg: &str) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for chunk in svg.split("<image ").skip(1) {
        let head = &chunk[..chunk.find('>').unwrap_or(chunk.len())];
        let attr = |name: &str| -> Option<f64> {
            let key = format!("{name}=\"");
            let s = head.find(&key)? + key.len();
            let e = head[s..].find('"')?;
            head[s..s + e].parse::<f64>().ok()
        };
        if let (Some(y), Some(h)) = (attr("y"), attr("height")) {
            out.push((y, h));
        }
    }
    out
}

fn page_svg(core: &DocumentCore, page: u32) -> String {
    core.render_page_svg_native(page)
        .unwrap_or_else(|e| panic!("{}쪽 svg: {e}", page + 1))
}

#[test]
fn issue_6776_negative_cell_wrapped_picture_is_not_charged() {
    // 이 문서의 칸 그림은 어울림(비-TAC)이라 `para_non_inline_h` 소관이다. 회계에
    // 넣으면 이중 계상돼 3쪽 배분이 무너지고 용지 밖 1건·넘침 1건이 생긴다.
    // engine 2020 정본도 3쪽이고 쪽별 글자 수 [112, 359, 326] 로 같다.
    let core = open(NEGATIVE);
    assert_eq!(
        core.page_count(),
        3,
        "음성 대조: 어울림 그림은 칸 회계에 들어가면 안 된다 — 들어가면 배분이 무너진다"
    );
    let page_count = u32::try_from(core.page_count()).expect("page count fits u32");
    for page in 0..page_count {
        for (y, h) in images(&page_svg(&core, page)) {
            assert!(
                y + h <= PAPER_HEIGHT_PX + 0.5,
                "음성 대조 {}쪽 그림이 종이 밖으로 나가면 안 된다 (y={y:.1} h={h:.1})",
                page + 1
            );
        }
    }
}
