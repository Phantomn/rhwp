//! Cross-paragraph field ends preserve the authored empty closing line.
use rhwp::renderer::table_v2::DocumentV2Session;
use serde_json::Value;
use std::io::{Cursor, Read, Write};
const INPUT: &[u8] =
    include_bytes!("../../samples/issue6601/36331407_side_by_side_tac_tables.hwpx");
const OPTIONS: &str = r#"{"dpi":96,"max_pages":100,"cell_end_policy":"omit_final_paragraph_gap"}"#;

#[test]
fn fresh_body_gap_preserves_blank_lines_and_separate_paragraph_after() {
    use rhwp::model::{
        document::{Document, Section},
        page::PageDef,
        paragraph::{CharShapeRef, Paragraph},
        style::{CharShape, LineSpacingType, ParaShape},
    };
    //9pt =12px line; fixed2700 half-HU =18px pitch. Distinguish the
    //6px following gap from a real12px blank and explicit4px paragraph-after.
    for (budget, after, pages_expected, next_y) in [
        (14, 0, 2, 30.),
        (30, 0, 1, 48.),
        (36, 0, 1, 48.),
        (18, 600, 2, 34.),
    ] {
        let mut d = Document::default();
        d.doc_info.char_shapes.push(CharShape {
            base_size: 900,
            ..Default::default()
        });
        d.doc_info.para_shapes.push(ParaShape {
            line_spacing_type: LineSpacingType::Fixed,
            line_spacing: 2700,
            spacing_after: after,
            ..Default::default()
        });
        d.doc_info.para_shapes.push(ParaShape {
            line_spacing_type: LineSpacingType::Fixed,
            line_spacing: 2700,
            ..Default::default()
        });
        let mut section = Section::default();
        section.section_def.page_def = PageDef {
            width: 30000,
            height: 2250 + budget * 75,
            margin_left: 1500,
            margin_right: 1500,
            margin_top: 1500,
            margin_header: 750,
            margin_bottom: 0,
            margin_footer: 0,
            ..Default::default()
        };
        section.paragraphs = ["", "NEXT"]
            .into_iter()
            .enumerate()
            .map(|(index, text)| Paragraph {
                para_shape_id: index as u16,
                text: text.into(),
                char_count: text.len() as u32,
                char_offsets: (0..text.len() as u32).collect(),
                char_shapes: vec![CharShapeRef {
                    start_pos: 0,
                    char_shape_id: 0,
                }],
                ..Default::default()
            })
            .collect();
        d.sections.push(section);
        let encoded = rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap();
        let mut s = DocumentV2Session::from_bytes(&encoded, OPTIONS).unwrap();
        let mut pages = Vec::new();
        while let Some(raw) = s.next_page_json().unwrap() {
            pages.push(serde_json::from_str::<Value>(&raw).unwrap());
        }
        assert_eq!(pages.len(), pages_expected, "{budget}/{after}");
        let mut ls = Vec::new();
        for page in &pages {
            lines(&page["render_tree"]["root"], &mut ls);
        }
        assert_eq!(ls.len(), 2);
        assert!((ls[0]["bbox"]["y"].as_f64().unwrap() - 30.).abs() < 1e-7);
        assert!((ls[0]["bbox"]["height"].as_f64().unwrap() - 12.).abs() < 1e-7);
        assert!(
            (ls[1]["bbox"]["y"].as_f64().unwrap() - next_y).abs() < 1e-7,
            "{budget}/{after}"
        );
        assert!(s.next_page_json().unwrap().is_none());
    }
}
fn modified(edit: impl Fn(&str) -> String) -> Vec<u8> {
    let mut src = zip::ZipArchive::new(Cursor::new(INPUT)).unwrap();
    let mut dst = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..src.len() {
        let mut f = src.by_index(i).unwrap();
        let mut data = Vec::new();
        f.read_to_end(&mut data).unwrap();
        if f.name() == "Contents/section0.xml" {
            data = edit(std::str::from_utf8(&data).unwrap()).into_bytes();
        }
        dst.start_file(f.name(), zip::write::SimpleFileOptions::default())
            .unwrap();
        dst.write_all(&data).unwrap();
    }
    dst.finish().unwrap().into_inner()
}
fn lines<'a>(n: &'a Value, out: &mut Vec<&'a Value>) {
    if n["node_type"].get("TextLine").is_some() {
        out.push(n);
    }
    for c in n["children"].as_array().unwrap() {
        lines(c, out);
    }
}
#[test]
fn body_page_start_does_not_inherit_previous_paragraph_line_gap() {
    // Original Hancom PDF page4: first baseline111.398px at96dpi;
    // PageDef body top7088HU + saved baseline1275HU =111.5067px.
    // The authored blank p31 stays on page3; its1200HU following gap
    // is not an authored blank line at the top of page4.
    let mut session = DocumentV2Session::from_bytes(INPUT, OPTIONS).unwrap();
    let mut pages = Vec::new();
    while let Some(raw) = session.next_page_json().unwrap() {
        pages.push(serde_json::from_str::<Value>(&raw).unwrap());
    }
    assert_eq!(pages.len(), 4);
    let mut previous = Vec::new();
    lines(&pages[2]["render_tree"]["root"], &mut previous);
    let blank = previous
        .iter()
        .find(|n| n["node_type"]["TextLine"]["para_index"] == 31)
        .unwrap();
    assert!((blank["bbox"]["height"].as_f64().unwrap() - 20.0).abs() < 1e-7);
    let mut next = Vec::new();
    lines(&pages[3]["render_tree"]["root"], &mut next);
    assert_eq!(next[0]["node_type"]["TextLine"]["para_index"], 32);
    assert!((next[0]["bbox"]["y"].as_f64().unwrap() - 7088.0 / 75.0).abs() < 1e-7);
    assert!(
        (next[1]["bbox"]["y"].as_f64().unwrap() - next[0]["bbox"]["y"].as_f64().unwrap() - 36.0)
            .abs()
            < 1e-7
    );
    assert!(session.next_page_json().unwrap().is_none());
}
#[test]
fn body_field_end_preserves_saved_empty_line_and_next_paragraph() {
    // Synthetic successor makes the closing blank's advance observable. It
    // does not alter the closing paragraph's original source slots or metrics.
    let input = modified(|s| {
        s.replace("</hs:sec>",r#"<hp:p id="1" paraPrIDRef="25" styleIDRef="0" pageBreak="0" columnBreak="0" merged="0"><hp:run charPrIDRef="41"><hp:t>AFTER FIELD</hp:t></hp:run><hp:linesegarray><hp:lineseg textpos="0" vertpos="34940" vertsize="1600" textheight="1600" baseline="1360" spacing="1280" horzpos="0" horzsize="51024" flags="393216"/></hp:linesegarray></hp:p></hs:sec>"#)
    });
    let mut s = DocumentV2Session::from_bytes(&input, OPTIONS).unwrap();
    let mut pages = Vec::new();
    while let Some(raw) = s.next_page_json().unwrap() {
        pages.push(serde_json::from_str::<Value>(&raw).unwrap());
    }
    let mut ls = Vec::new();
    for p in &pages {
        lines(&p["render_tree"]["root"], &mut ls);
    }
    let last = ls.last().unwrap();
    let closing = ls[ls.len() - 2];
    assert_eq!(
        last["children"][0]["node_type"]["TextRun"]["text"],
        "AFTER FIELD"
    );
    assert!((closing["bbox"]["height"].as_f64().unwrap() - 1600.0 / 75.0).abs() < 1e-7);
    assert!(
        (last["bbox"]["y"].as_f64().unwrap()
            - closing["bbox"]["y"].as_f64().unwrap()
            - 2880.0 / 75.0)
            .abs()
            < 1e-7
    );
    assert!(closing["children"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["node_type"]["TextRun"]["text"] == ""));
    assert!(s.next_page_json().unwrap().is_none());
}
#[test]
fn body_field_end_rejects_unmatched_duplicate_and_text_after_end() {
    for case in ["id", "duplicate", "text", "row"] {
        let input = modified(|s| {
            match case{
            "id"=>s.replace("beginIDRef=\"1561678090\"","beginIDRef=\"1561678091\""),
            "duplicate"=>s.replace("<hp:fieldEnd beginIDRef=\"1561678090\" fieldid=\"627272811\"/>","<hp:fieldEnd beginIDRef=\"1561678090\" fieldid=\"627272811\"/><hp:fieldEnd beginIDRef=\"1561678090\" fieldid=\"627272811\"/>"),
            "text"=>s.replace("<hp:run charPrIDRef=\"41\"><hp:t/>","<hp:run charPrIDRef=\"41\"><hp:t>UNSUPPORTED</hp:t>"),
            "row"=>s.replace("vertpos=\"32060\" vertsize=\"1600\"","vertpos=\"32060\" vertsize=\"0\""),
            _=>unreachable!(),
        }
        });
        assert!(
            DocumentV2Session::from_bytes(&input, OPTIONS).is_err(),
            "{case}"
        );
    }
}

#[test]
fn multiline_body_field_tail_keeps_rows_and_successor() {
    // Synthetic saved HWPX boundary: one explicit LF with two known rows.
    // The separately tested normal Hancom save supplies HWP evidence.
    let bytes = modified(|s| {
        let start = s.rfind("<hp:p ").unwrap();
        let tail = s[start..].replace(
            "<hp:ctrl><hp:fieldEnd",
            "<hp:t>ONE<hp:lineBreak/>TWO</hp:t><hp:ctrl><hp:fieldEnd",
        );
        let row = r#"<hp:lineseg textpos="0" vertpos="32060" vertsize="1600" textheight="1600" baseline="1360" spacing="1280" horzpos="0" horzsize="51024" flags="393216"/>"#;
        let second = row
            .replace("textpos=\"0\"", "textpos=\"4\"")
            .replace("32060", "34940");
        let after_row = row.replace("32060", "37820");
        let after = format!(
            r#"<hp:p id="1" paraPrIDRef="25" styleIDRef="0" pageBreak="0" columnBreak="0" merged="0"><hp:run charPrIDRef="21"><hp:t>AFTER</hp:t></hp:run><hp:linesegarray>{after_row}</hp:linesegarray></hp:p></hs:sec>"#
        );
        format!(
            "{}{}",
            &s[..start],
            tail.replace(row, &format!("{row}{second}"))
                .replace("</hs:sec>", &after)
        )
    });
    {
        let mut session = DocumentV2Session::from_bytes(&bytes, OPTIONS).unwrap();
        let mut pages = Vec::new();
        while let Some(raw) = session.next_page_json().unwrap() {
            pages.push(serde_json::from_str::<Value>(&raw).unwrap());
        }
        assert_eq!(pages.len(), 4);
        let mut ls = Vec::new();
        lines(&pages[3]["render_tree"]["root"], &mut ls);
        assert_eq!(ls.len(), 5);
        for (i, text) in ["ONE", "TWO", "AFTER"].into_iter().enumerate() {
            let line = ls[ls.len() - 3 + i];
            let actual: String = line["children"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|n| n["node_type"]["TextRun"]["text"].as_str())
                .collect();
            assert_eq!(actual, text);
            assert!((line["bbox"]["height"].as_f64().unwrap() - 1600. / 75.).abs() < 1e-7);
            assert!(
                (line["bbox"]["y"].as_f64().unwrap() - (7088. + 5400. + i as f64 * 2880.) / 75.)
                    .abs()
                    < 1e-7
            );
        }
        assert!(session.next_page_json().unwrap().is_none());
    }
}

#[test]
fn trailing_body_field_end_preserves_literal_text_and_saved_line_geometry() {
    // Source-only structural markers must not change the literal paragraph's
    // occupied line or the successor origin. Include a UTF-16 surrogate pair
    // so scalar end indices cannot be mistaken for source-stream offsets.
    for text in ["붙임 문서 1부. 끝.", "  ", "A😀B"] {
        let input = modified(|s| {
            s.replace(
                "<hp:ctrl><hp:fieldEnd beginIDRef=\"1561678090\"",
                &format!("<hp:t>{text}</hp:t><hp:ctrl><hp:fieldEnd beginIDRef=\"1561678090\""),
            )
        });
        let parsed = rhwp::parse_document(&input).unwrap();
        let tail = parsed.sections[0].paragraphs.last().unwrap();
        assert_eq!(tail.orphan_field_ends[0].char_idx, text.chars().count());
        assert_eq!(tail.char_count as usize, text.encode_utf16().count() + 9);
        let mut session = DocumentV2Session::from_bytes(&input, OPTIONS).unwrap();
        let mut pages = Vec::new();
        while let Some(raw) = session.next_page_json().unwrap() {
            pages.push(serde_json::from_str::<Value>(&raw).unwrap());
        }
        assert_eq!(pages.len(), 4);
        let mut ls = Vec::new();
        lines(&pages[3]["render_tree"]["root"], &mut ls);
        let closing = ls.last().unwrap();
        let actual: String = closing["children"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|n| n["node_type"]["TextRun"]["text"].as_str())
            .collect();
        assert_eq!(actual, text);
        assert!((closing["bbox"]["height"].as_f64().unwrap() - 1600. / 75.).abs() < 1e-7);
        // Two earlier 1500+1200 HU attachment lines precede the closing line.
        assert!((closing["bbox"]["y"].as_f64().unwrap() - (7088. + 5400.) / 75.).abs() < 1e-7);
        assert!(session.next_page_json().unwrap().is_none());
    }
}

#[test]
fn nested_body_end_ids_close_in_reverse_order_without_extra_lines() {
    for (crossed, literal) in [(false, ""), (true, ""), (false, "END"), (true, "END")] {
        let input = modified(|s| {
            let start = s.find("<hp:fieldBegin id=\"1561678090\"").unwrap();
            let end =
                start + s[start..].find("</hp:fieldBegin>").unwrap() + "</hp:fieldBegin>".len();
            let outer = &s[start..end];
            let inner = outer
                .replace("1561678090", "1561678091")
                .replace("627272811", "627272812");
            let text = s.replacen(outer, &format!("{outer}{inner}"), 1);
            let outer_end = "<hp:fieldEnd beginIDRef=\"1561678090\" fieldid=\"627272811\"/>";
            let inner_end = "<hp:fieldEnd beginIDRef=\"1561678091\" fieldid=\"627272812\"/>";
            let closed = text.replace(
                outer_end,
                &if crossed {
                    format!("{outer_end}{inner_end}")
                } else {
                    format!("{inner_end}{outer_end}")
                },
            );
            closed.replace(
                "<hp:ctrl><hp:fieldEnd",
                &format!("<hp:t>{literal}</hp:t><hp:ctrl><hp:fieldEnd"),
            )
        });
        let mut result = DocumentV2Session::from_bytes(&input, OPTIONS);
        if crossed {
            assert!(result.is_err());
            continue;
        }
        let s = result.as_mut().unwrap();
        let mut last = None;
        let mut count = 0;
        while let Some(raw) = s.next_page_json().unwrap() {
            last = Some(serde_json::from_str::<Value>(&raw).unwrap());
            count += 1;
        }
        assert_eq!(count, 4);
        let last = last.unwrap();
        let mut ls = Vec::new();
        lines(&last["render_tree"]["root"], &mut ls);
        // The source paragraph still has exactly one 1600HU line, even though
        // two structural ends occupy 16 source units instead of eight.
        assert!(
            (ls.last().unwrap()["bbox"]["height"].as_f64().unwrap() - 1600.0 / 75.0).abs() < 1e-7
        );
        let actual: String = ls.last().unwrap()["children"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|n| n["node_type"]["TextRun"]["text"].as_str())
            .collect();
        assert_eq!(actual, literal);
    }
}

#[test]
fn hancom_saved_trailing_end_keeps_text_successor_and_termination() {
    let input = include_bytes!("../fixtures/issue7353_body_field_tail_review/tail-saved.hwp");
    let source = rhwp::parse_document(input).unwrap();
    let closing = &source.sections[0].paragraphs[2];
    assert_eq!(closing.text, "붙임 문서 1부. 끝.");
    assert_eq!(closing.orphan_field_ends.len(), 1);
    assert_eq!(
        closing.orphan_field_ends[0].char_idx,
        closing.text.chars().count()
    );
    // Normal Hancom stored metrics and original PageDef, not V2 measurements:
    // body top7088 HU, row1300 HU, following gap780 HU ->2080 HU pitch.
    assert_eq!(closing.line_segs[0].line_height, 1300);
    assert_eq!(closing.line_segs[0].line_spacing, 780);
    let mut session = DocumentV2Session::from_bytes(input, OPTIONS).unwrap();
    let mut pages = Vec::new();
    while let Some(raw) = session.next_page_json().unwrap() {
        pages.push(serde_json::from_str::<Value>(&raw).unwrap());
    }
    assert_eq!(pages.len(), 2);
    let mut ls = Vec::new();
    lines(&pages[1]["render_tree"]["root"], &mut ls);
    assert_eq!(ls.len(), 2);
    for (i, expected) in ["붙임 문서 1부. 끝.", "누름틀 종료 뒤 문단"]
        .into_iter()
        .enumerate()
    {
        let line = ls[i];
        let actual: String = line["children"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|n| n["node_type"]["TextRun"]["text"].as_str())
            .collect();
        assert_eq!(actual, expected);
        assert!(
            (line["bbox"]["y"].as_f64().unwrap() - (7088. + i as f64 * 2080.) / 75.).abs() < 1e-7
        );
        assert!((line["bbox"]["height"].as_f64().unwrap() - 1300. / 75.).abs() < 1e-7);
    }
    assert!(session.next_page_json().unwrap().is_none());
}
