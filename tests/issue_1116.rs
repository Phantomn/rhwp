//! Issue #1116: HWP3→HWP5 sample16 목차 leader 및 p3 문단 vpos 정합 가드.

use std::fs;
use std::path::Path;

fn render_svg(rel_path: &str, page_idx: u32) -> String {
    let repo_root = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(repo_root).join(rel_path);
    let bytes = fs::read(&path).unwrap_or_else(|e| panic!("read {rel_path}: {e}"));
    let doc = rhwp::wasm_api::HwpDocument::from_bytes(&bytes)
        .unwrap_or_else(|e| panic!("parse {rel_path}: {e:?}"));
    doc.render_page_svg_native(page_idx)
        .unwrap_or_else(|e| panic!("render {rel_path} page {page_idx}: {e:?}"))
}

fn load_doc(rel_path: &str) -> rhwp::wasm_api::HwpDocument {
    let repo_root = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(repo_root).join(rel_path);
    let bytes = fs::read(&path).unwrap_or_else(|e| panic!("read {rel_path}: {e}"));
    rhwp::wasm_api::HwpDocument::from_bytes(&bytes)
        .unwrap_or_else(|e| panic!("parse {rel_path}: {e:?}"))
}

fn extract_text_positions(svg: &str, wanted: &str) -> Vec<(f64, f64)> {
    let mut positions = Vec::new();
    let mut search_from = 0;
    while let Some(rel) = svg[search_from..].find("<text ") {
        let tag_start = search_from + rel;
        search_from = tag_start + 6;
        let Some(close_rel) = svg[tag_start..].find('>') else {
            break;
        };
        let attrs = &svg[tag_start..tag_start + close_rel];
        let content_start = tag_start + close_rel + 1;
        let Some(end_rel) = svg[content_start..].find("</text>") else {
            break;
        };
        let text = &svg[content_start..content_start + end_rel];
        if text == wanted {
            if let (Some(x), Some(y)) = (attr_f64(attrs, "x"), attr_f64(attrs, "y")) {
                positions.push((x, y));
            }
        }
    }
    positions
}

fn toc_page_number_right_edges(svg: &str) -> Vec<f64> {
    let mut rows: std::collections::BTreeMap<i32, Vec<(f64, f64, String)>> =
        std::collections::BTreeMap::new();
    let mut search_from = 0;
    while let Some(rel) = svg[search_from..].find("<text ") {
        let tag_start = search_from + rel;
        search_from = tag_start + 6;
        let Some(close_rel) = svg[tag_start..].find('>') else {
            break;
        };
        let attrs = &svg[tag_start..tag_start + close_rel];
        let content_start = tag_start + close_rel + 1;
        let Some(end_rel) = svg[content_start..].find("</text>") else {
            break;
        };
        let text = &svg[content_start..content_start + end_rel];
        if text.chars().all(|ch| ch.is_ascii_digit()) {
            let Some(x) = attr_f64(attrs, "x") else {
                continue;
            };
            let Some(y) = attr_f64(attrs, "y") else {
                continue;
            };
            let text_length = attr_f64(attrs, "textLength").unwrap_or(0.0);
            if x > 600.0 {
                rows.entry((y * 10.0).round() as i32).or_default().push((
                    x,
                    x + text_length,
                    text.to_string(),
                ));
            }
        }
    }

    rows.into_values()
        .filter_map(|mut digits| {
            digits.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            digits.last().map(|(_, right, _)| *right)
        })
        .collect()
}

fn extract_text_attrs(svg: &str, wanted: &str) -> Vec<String> {
    let mut attrs_list = Vec::new();
    let mut search_from = 0;
    while let Some(rel) = svg[search_from..].find("<text ") {
        let tag_start = search_from + rel;
        search_from = tag_start + 6;
        let Some(close_rel) = svg[tag_start..].find('>') else {
            break;
        };
        let attrs = &svg[tag_start..tag_start + close_rel];
        let content_start = tag_start + close_rel + 1;
        let Some(end_rel) = svg[content_start..].find("</text>") else {
            break;
        };
        let text = &svg[content_start..content_start + end_rel];
        if text == wanted {
            attrs_list.push(attrs.to_string());
        }
    }
    attrs_list
}

fn attr_f64(attrs: &str, name: &str) -> Option<f64> {
    let needle = format!("{name}=\"");
    let start = attrs.find(&needle)? + needle.len();
    let end = attrs[start..].find('"')?;
    attrs[start..start + end].parse().ok()
}

#[test]
fn sample16_hwp3_toc_page_numbers_share_right_edge() {
    let svg = render_svg("samples/hwp3-sample16.hwp", 1);
    let edges = toc_page_number_right_edges(&svg);
    assert!(
        edges.len() >= 25,
        "원본 HWP3 목차 페이지 번호를 충분히 찾아야 함: {edges:?}"
    );
    let min = edges.iter().copied().fold(f64::INFINITY, f64::min);
    let max = edges.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    assert!(
        max - min <= 1.5,
        "원본 HWP3 목차 페이지 번호 오른쪽 끝이 한컴처럼 같은 기준선에 정렬되어야 함: min={min:.2}, max={max:.2}, edges={edges:?}"
    );
}

fn assert_page3_latin_poppy_resolves_to_palatino(rel_path: &str) {
    let svg = render_svg(rel_path, 2);
    let latin_c_attrs = extract_text_attrs(&svg, "C");
    assert!(
        latin_c_attrs.iter().any(|attrs| {
            // SVG attribute serialization must XML-escape the CSS family-name
            // quotes. Assert the semantic family list rather than a single
            // serializer spelling (plain `'` vs `&apos;`).
            attrs
                .replace("&apos;", "'")
                .contains("font-family=\"'Palatino Linotype',")
        }),
        "{rel_path} p3 Latin glyphs must resolve HCI Poppy to Palatino Linotype: {latin_c_attrs:?}"
    );
    assert!(
        latin_c_attrs.iter().all(|attrs| {
            !attrs
                .replace("&apos;", "'")
                .contains("font-family=\"'HCI Poppy',")
        }),
        "{rel_path} p3 Latin glyphs must not fall back through unresolved HCI Poppy: {latin_c_attrs:?}"
    );
}

#[test]
fn sample16_hwp5_2022_page3_latin_font_matches_legacy_hancom_mapping() {
    assert_page3_latin_poppy_resolves_to_palatino("samples/hwp3-sample16-hwp5-2022.hwp");
}

#[test]
fn sample16_hwp3_page3_latin_font_matches_legacy_hancom_mapping() {
    assert_page3_latin_poppy_resolves_to_palatino("samples/hwp3-sample16.hwp");
}

#[test]
fn sample16_hwp3_page3_heading_positions_follow_hancom_grid() {
    let svg = render_svg("samples/hwp3-sample16.hwp", 2);
    let twos = extract_text_positions(&svg, "2");
    let threes = extract_text_positions(&svg, "3");

    let heading2 = twos.iter().find(|(_, y)| (*y - 337.2).abs() < 2.0).copied();
    assert!(
        heading2.is_some(),
        "HWP3 원본 p3 `2. 추진방향`도 한컴 3mm 격자 y≈337.2를 따라야 함: {twos:?}"
    );

    let heading3 = threes
        .iter()
        .find(|(_, y)| (*y - 749.2).abs() < 2.0)
        .copied();
    assert!(
        heading3.is_some(),
        "HWP3 원본 p3 `3. 주요 추진내용`도 한컴 3mm 격자 y≈749.2를 따라야 함: {threes:?}"
    );
}

#[test]
fn sample16_hwp5_2022_page3_bcp_tail_paragraph_folds_orphan_lineseg() {
    let doc = load_doc("samples/hwp3-sample16-hwp5-2022.hwp");
    let dump = doc.dump_page_items(Some(2));
    let p83 = dump
        .lines()
        .find(|line| line.contains("FullParagraph  pi=83"))
        .unwrap_or_else(|| panic!("2022 p3 pi=83 dump line not found:\n{dump}"));
    let summary = dump
        .lines()
        .find(|line| line.contains("단 0 (items=19"))
        .unwrap_or_else(|| panic!("2022 p3 단 요약을 찾을 수 없음:\n{dump}"));

    // dump-pages 는 #4628 이후 HeightMeasurer 합(31.5)이 아니라 프로덕션
    // format_paragraph total(sb=1.9 + lines=27.7 + sa=0 → 29.6)을 말한다.
    // 접힘 계약은 그대로: lh/ls 한 줄, 꼬리 LINE_SEG 를 별도 시각 줄로 세지 않는다.
    assert!(
        p83.contains("h=29.6")
            && p83.contains("lines=27.7")
            && p83.contains("lh=17.3")
            && p83.contains("ls=10.4"),
        "2022 p83 BCP 문단의 마지막 LINE_SEG 꼬리는 한컴오피스처럼 앞 줄에 접혀야 함: {p83}"
    );
    assert!(
        summary.contains("used=874.5px")
            && summary.contains("hwp_used≈841.6px")
            && summary.contains("diff=+32.9px"),
        "2022 p3 단 요약은 p83 꼬리 LINE_SEG를 별도 시각 줄로 세지 않아야 함: {summary}"
    );
}

#[test]
fn sample16_hwp5_2022_page3_bcp_tail_glyph_stays_on_hancom_line() {
    let svg = render_svg("samples/hwp3-sample16-hwp5-2022.hwp", 2);
    let tail_glyphs = extract_text_positions(&svg, "립");

    let folded_tail = tail_glyphs
        .iter()
        .find(|(x, y)| *x > 620.0 && (*y - 881.35).abs() < 1.0)
        .copied();
    assert!(
        folded_tail.is_some(),
        "2022 p83 BCP `수립`의 `립`은 한컴오피스처럼 p83 본문 줄 y≈881.35에 있어야 함: {tail_glyphs:?}"
    );

    let orphan_tail = tail_glyphs
        .iter()
        .find(|(x, y)| (*x - 126.7).abs() < 2.0 && (*y - 909.1).abs() < 2.0)
        .copied();
    assert!(
        orphan_tail.is_none(),
        "2022 p83 BCP `립`이 다음 줄 머리에 단독 배치되면 p84 이하가 한컴오피스보다 내려감: {tail_glyphs:?}"
    );
}
