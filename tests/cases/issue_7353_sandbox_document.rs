//! Whole, unmodified HWPX. Independent Windows Hancom PDF (job
//! 2e11ab6c-00a7-41dd-9a4a-c3b8ec05c17c, 11.0.0.9136, no preprocessing)
//! has 17 pages. See stage19 document-review for PDF hash and direct sweep.
//! These are supported-output contracts, not a new pagination bug-fix claim.
use rhwp::renderer::table_v2::DocumentV2Session;
use serde_json::Value;

fn pages(dpi: u32) -> Vec<Value> {
    let input = include_bytes!("../../samples/issue4090/156492236_규제샌드박스_min.hwpx");
    let mut session = DocumentV2Session::from_bytes(
        input,
        &format!(r#"{{"dpi":{dpi},"max_pages":17,"cell_end_policy":"omit_final_paragraph_gap"}}"#),
    )
    .unwrap();
    let mut pages = vec![];
    while let Some(json) = session.next_page_json().unwrap() {
        pages.push(serde_json::from_str(&json).unwrap());
    }
    assert_eq!(pages.len(), 17);
    assert_eq!(session.emitted_pages(), 17);
    assert!(session.next_page_json().unwrap().is_none());
    pages
}

fn nodes<'a>(n: &'a Value, kind: &str) -> Vec<&'a Value> {
    let mut out = vec![];
    if n["node_type"].get(kind).is_some() {
        out.push(n);
    }
    for child in n["children"].as_array().unwrap() {
        out.extend(nodes(child, kind));
    }
    out
}

fn compact(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

fn text(n: &Value) -> String {
    nodes(n, "TextRun")
        .iter()
        .map(|v| v["node_type"]["TextRun"]["text"].as_str().unwrap())
        .collect()
}

#[test]
fn original_document_preserves_page_boundary_sentences_and_terminal_content() {
    let pages = pages(96);
    let strings: Vec<_> = pages
        .iter()
        .map(|p| compact(&text(&p["render_tree"]["root"])))
        .collect();
    // Observed Hancom page endings/starts, including mostly empty continuation
    // pages. Do not discard a page just because it has only one body line.
    for (preceding, end, start) in [
        (
            1,
            "새로운 일자리를 창출하였습니다.",
            "- 또한, 비수도권의 14개 시도",
        ),
        (4, "동 사업이", "확대될 경우 공유경제의 바람직한 모델"),
        (6, "’21.12월까지", "1,100만건(15.7조원)에 달하는 조회 건수"),
        (
            14,
            "1,104명이 사용하",
            "는 등 장애인의 편의 향상에 크게 기여",
        ),
    ] {
        let footer = compact(&format!("- {} -", preceding + 1));
        assert!(strings[preceding].ends_with(&(compact(end) + &footer)));
        assert!(strings[preceding + 1].starts_with(&compact(start)));
        let joined = strings.join("");
        assert_eq!(joined.matches(&compact(start)).count(), 1);
    }
    assert!(strings[16].contains(&compact("친환경 소비도 확대될 것으로 기대되고 있습니다.")));
    for (i, s) in strings.iter().enumerate() {
        let footer = compact(&format!("- {} -", i + 1));
        assert!(s.ends_with(&footer));
        assert_eq!(s.matches(&footer).count(), 1);
    }
}

#[test]
fn original_page_five_final_table_frames_match_independent_pdf() {
    // PDF path centers in points, top-down from841pt. The two title frames:
    // x58.049..542.468, y58.017..100.570 and482.832..546.003.
    // The PDF text matrix is(.119935,.119869)pt per600dpi device unit,
    // not(.12,.12). Apply that independently observed printer scaling to
    // geometric points before comparison. Two device dots(.24pt) allow
    // independent edge rounding, NOT a pagination tolerance.
    for dpi in [96, 192] {
        let pages = pages(dpi);
        let titles: Vec<_> = nodes(&pages[4]["render_tree"]["root"], "Table")
            .into_iter()
            .filter(|n| !text(n).is_empty())
            .collect();
        assert_eq!(titles.len(), 2);
        for (table, (top, bottom)) in titles.iter().zip([(58.017, 100.570), (482.832, 546.003)]) {
            let b = &table["bbox"];
            let pt = |k| {
                let printer = if k == "x" || k == "width" {
                    0.119935 / 0.12
                } else {
                    0.119869 / 0.12
                };
                b[k].as_f64().unwrap() * 72.0 / f64::from(dpi) * printer
            };
            for (actual, expected) in [
                (pt("x"), 58.049),
                (pt("x") + pt("width"), 542.468),
                (pt("y"), top),
                (pt("y") + pt("height"), bottom),
            ] {
                assert!(
                    (actual - expected).abs() < 0.24,
                    "{actual} versus {expected}"
                );
            }
        }
    }
}
