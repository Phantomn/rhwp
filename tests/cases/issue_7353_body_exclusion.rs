//! A saved zero-width declaration line shares a TopAndBottom table's origin.
//! Independent source: normally saved first ten paragraphs of the regulatory cover.
use rhwp::{
    model::{control::Control, document::Document},
    renderer::table_v2::*,
};
use serde_json::Value;
const INPUT: &[u8] = include_bytes!("../fixtures/issue7353/body-exclusion/contents-saved.hwp");
fn document() -> Document {
    rhwp::parse_document(INPUT).unwrap()
}
fn session(d: &Document) -> Result<DocumentV2Session, DocumentV2Error> {
    DocumentV2Session::from_bytes(
        &rhwp::serializer::hwpx::serialize_hwpx(d).unwrap(),
        r#"{"dpi":96,"max_pages":30,"cell_end_policy":"omit_final_paragraph_gap"}"#,
    )
}
fn pages(d: &Document) -> Vec<Value> {
    drain(session(d).unwrap())
}
fn drain(mut s: DocumentV2Session) -> Vec<Value> {
    let mut out = vec![];
    while let Some(p) = s.next_page_json().unwrap() {
        out.push(serde_json::from_str(&p).unwrap());
    }
    assert!(s.next_page_json().unwrap().is_none());
    out
}
fn collect<'a>(n: &'a Value, kind: &str, out: &mut Vec<&'a Value>) {
    if n["node_type"].get(kind).is_some() {
        out.push(n);
    }
    if let Some(c) = n["children"].as_array() {
        for n in c {
            collect(n, kind, out);
        }
    }
}
fn nodes<'a>(p: &'a Value, kind: &str) -> Vec<&'a Value> {
    let mut v = vec![];
    collect(&p["render_tree"]["root"], kind, &mut v);
    v
}
fn text(n: &Value) -> String {
    let mut v = vec![];
    collect(n, "TextRun", &mut v);
    v.iter()
        .map(|n| n["node_type"]["TextRun"]["text"].as_str().unwrap())
        .collect()
}
fn number(n: &Value, key: &str) -> f64 {
    n["bbox"][key].as_f64().unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}
fn body_line(p: &Value, pi: usize) -> &Value {
    nodes(p, "TextLine")
        .into_iter()
        .find(|n| {
            n["node_type"]["TextLine"]["para_index"] == pi
                && n["node_type"]["TextLine"]["section_index"] == 0
        })
        .unwrap()
}

#[test]
fn saved_excluded_body_line_and_following_blank_use_one_table_envelope() {
    let d = document();
    let before = serde_json::to_value(&d.sections[0].paragraphs).unwrap();
    let p = drain(
        DocumentV2Session::from_bytes(
            INPUT,
            r#"{"dpi":96,"max_pages":30,"cell_end_policy":"omit_final_paragraph_gap"}"#,
        )
        .unwrap(),
    );
    assert_eq!(p.len(), 1);
    let origin = number(body_line(&p[0], 1), "y") - 4131.0 / 75.0;
    let host = body_line(&p[0], 8);
    let after = body_line(&p[0], 9);
    close(number(host, "width"), 0.0);
    close(number(host, "height"), 1600.0 / 75.0);
    close(number(host, "y"), origin + 25995.0 / 75.0);
    close(number(after, "y"), origin + 42719.0 / 75.0);
    close(number(after, "height"), 1600.0 / 75.0);
    let cell = nodes(&p[0], "TableCell")
        .into_iter()
        .find(|c| text(c).starts_with("1.주민대표단"))
        .unwrap();
    close(number(cell, "y"), number(host, "y") + 141.0 / 75.0);
    close(number(cell, "height"), 16442.0 / 75.0);
    let mut lines = vec![];
    collect(cell, "TextLine", &mut lines);
    assert_eq!(lines.len(), 8);
    for (i, n) in lines.iter().enumerate() {
        close(
            number(n, "y"),
            number(cell, "y") + (141.0 + i as f64 * 2080.0) / 75.0,
        );
    }
    // Independent PDF: cell clip (84.315,317.772)..(509.246,481.993)pt;
    // glyph origins(746,2776/2949/3123), text transform(.119935,.119869).
    for (axis, expected) in [
        ("x", 84.315),
        ("y", 841.0 - 523.228),
        ("width", 509.246 - 84.315),
        ("height", 523.228 - 359.007),
    ] {
        assert!(
            (number(cell, axis) - expected * 96.0 / 72.0).abs() < 1.0,
            "PDF {axis}"
        );
    }
    for (line, y) in lines.iter().zip([2776.0, 2949.0, 3123.0]) {
        let mut runs = vec![];
        collect(line, "TextRun", &mut runs);
        let run = runs[0];
        assert!((number(run, "x") - 746.0 * 0.119935 * 96.0 / 72.0).abs() < 1.0);
        let baseline = number(run, "y") + run["node_type"]["TextRun"]["baseline"].as_f64().unwrap();
        assert!((baseline - y * 0.119869 * 96.0 / 72.0).abs() < 1.0);
    }
    assert_eq!(
        before,
        serde_json::to_value(&d.sections[0].paragraphs).unwrap()
    );
}

#[test]
fn independently_saved_following_marker_preserves_the_authored_blank() {
    let input = include_bytes!("../fixtures/issue7353/body-exclusion/marked-saved.hwp");
    let d = rhwp::parse_document(input).unwrap();
    assert!(d.sections[0].paragraphs[9].text.is_empty());
    let p = drain(
        DocumentV2Session::from_bytes(
            input,
            r#"{"dpi":96,"max_pages":30,"cell_end_policy":"omit_final_paragraph_gap"}"#,
        )
        .unwrap(),
    );
    assert_eq!(p.len(), 1);
    assert_eq!(
        text(body_line(&p[0], 10)),
        "목차 표와 빈 문단 다음 위치 확인"
    );
    close(
        number(body_line(&p[0], 10), "y") - number(body_line(&p[0], 9), "y"),
        1920.0 / 75.0,
    );
    close(
        number(body_line(&p[0], 9), "y") - number(body_line(&p[0], 8), "y"),
        16724.0 / 75.0,
    );
    // Independent marked PDF baseline4306, same text transform. The source
    // paragraph is CENTER: fallback glyph widths change its left edge, not
    // its centered placement or the vertical advance under test here.
    let mut runs = vec![];
    collect(body_line(&p[0], 10), "TextRun", &mut runs);
    let run = runs[0];
    let line = body_line(&p[0], 10);
    close(
        number(run, "x") + number(run, "width") / 2.0,
        number(line, "x") + number(line, "width") / 2.0,
    );
    let baseline = number(run, "y") + run["node_type"]["TextRun"]["baseline"].as_f64().unwrap();
    assert!((baseline - 4306.0 * 0.119869 * 96.0 / 72.0).abs() < 1.0);
}

#[test]
fn original_hwp_and_hwpx_first_ten_paragraphs_keep_excluded_host() {
    for file in [
        "samples/86712_regulatory_analysis.hwp",
        "samples/issue1891/86712_regulatory_analysis.hwpx",
    ] {
        let mut d = rhwp::parse_document(
            &std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(file)).unwrap(),
        )
        .unwrap();
        d.sections.truncate(1);
        d.sections[0].paragraphs.truncate(10);
        let p = pages(&d);
        assert_eq!(p.len(), 1);
        close(
            number(body_line(&p[0], 9), "y") - number(body_line(&p[0], 8), "y"),
            16724.0 / 75.0,
        );
    }
}

#[test]
fn body_exclusion_continuation_consumes_host_once_and_keeps_child_blanks() {
    for height in [8000, 14000] {
        let mut d = document();
        let def = &mut d.sections[0].section_def;
        def.page_def.height = def.page_def.margin_top
            + def.page_def.margin_header
            + def.page_def.margin_bottom
            + def.page_def.margin_footer
            + height;
        let def = d.sections[0].section_def.clone();
        for c in &mut d.sections[0].paragraphs[0].controls {
            if let Control::SectionDef(v) = c {
                **v = def.clone();
            }
        }
        let Control::Table(t) = &mut d.sections[0].paragraphs[8].controls[0] else {
            panic!()
        };
        t.page_break = rhwp::model::table::TablePageBreak::CellBreak;
        t.cells[0].vertical_align = rhwp::model::table::VerticalAlign::Top;
        let p = pages(&d);
        assert!(p.len() > 1);
        let body_top = (d.sections[0].section_def.page_def.margin_top
            + d.sections[0].section_def.page_def.margin_header) as f64
            / 75.0;
        let mut host = 0;
        let mut after = 0;
        let mut child_lines = vec![];
        for p in &p {
            for line in nodes(p, "TextLine") {
                if line["node_type"]["TextLine"]["section_index"] == 0 {
                    if line["node_type"]["TextLine"]["para_index"] == 8 {
                        host += 1;
                    }
                    if line["node_type"]["TextLine"]["para_index"] == 9 {
                        after += 1;
                    }
                }
            }
            for cell in nodes(p, "TableCell") {
                // The contents table has the independent declared width42519HU.
                if (number(cell, "width") - 42519.0 / 75.0).abs() < 1e-7 {
                    assert!(number(cell, "y") >= body_top - 1e-7);
                    assert!(
                        number(cell, "y") + number(cell, "height")
                            <= body_top + height as f64 / 75.0 + 1e-7
                    );
                    collect(cell, "TextLine", &mut child_lines);
                }
            }
        }
        assert_eq!((host, after), (1, 1));
        assert_eq!(
            child_lines.iter().map(|n| text(n)).collect::<Vec<_>>(),
            [
                "1.주민대표단 요건·구성절차·운영방법·동의요건 등",
                "2.예비사업시행자 지정신청 요건 등",
                "3.특별정비구역 제안 요건 등",
                "",
                "",
                "",
                "",
                ""
            ]
        );
    }
}

#[test]
fn zero_width_without_the_exclusion_contract_is_not_accepted() {
    for mutation in 1..6 {
        let mut d = document();
        let p = &mut d.sections[0].paragraphs[8];
        match mutation {
            1 => {
                p.text = " ".into();
                p.char_offsets = vec![8];
                p.char_count = 10;
            }
            2 => {
                p.line_segs[0].segment_width = 1;
            }
            3 => {
                let Control::Table(t) = &mut p.controls[0] else {
                    panic!()
                };
                t.common.text_wrap = rhwp::model::shape::TextWrap::Square;
            }
            4 => {
                let Control::Table(t) = &mut p.controls[0] else {
                    panic!()
                };
                t.common.vertical_offset = (-100i32) as u32;
            }
            _ => {
                p.line_segs[0].line_height = 0;
            }
        }
        assert!(session(&d).is_err(), "mutation {mutation}");
    }
}

const WIDE: &[u8] = include_bytes!("../fixtures/issue7353/body-width/wide-saved.hwp");

#[test]
fn body_float_lane_does_not_enable_overflow_inside_a_cell() {
    use std::sync::Arc;
    let child = Arc::new(
        TableContentPlan::new(
            vec![90.0],
            vec![RowInput {
                cells: vec![CellInput {
                    padding: Insets::default(),
                    minimum_height: 10.0,
                    content: ComposedCell {
                        width: 90.0,
                        height: 10.0,
                        lines: vec![],
                    },
                }],
            }],
            0.0,
            SplitPolicy::Never,
        )
        .unwrap(),
    );
    for (offset, lane, valid) in [
        (10.0, 90.0, true),
        (20.0, 90.0, false),
        (0.0, 101.0, false),
        (0.0, 89.0, false),
    ] {
        let plan = TableContentPlan::from_flow_rows(
            vec![100.0],
            vec![FlowRowInput {
                cells: vec![FlowCellInput {
                    width: 100.0,
                    padding: Insets::default(),
                    minimum_height: 0.0,
                    blocks: vec![FlowBlock::AnchoredTable {
                        owner: ControlOwner {
                            paragraph: 0,
                            control: 0,
                        },
                        host: None,
                        host_advance: 0.0,
                        offset_x: offset,
                        offset_y: 0.0,
                        available_width: lane,
                        top: 0.0,
                        bottom: 0.0,
                        plan: child.clone(),
                    }],
                }],
            }],
            0.0,
            SplitPolicy::Never,
        );
        if !valid {
            assert!(matches!(plan, Err(GeometryError::ContentBounds { .. })));
            continue;
        }
        let FragmentFit::Placed(fragment) = plan
            .unwrap()
            .start()
            .fit(PageArea {
                bounds: Rect {
                    x: 5.0,
                    y: 3.0,
                    width: 100.0,
                    height: 10.0,
                },
            })
            .unwrap()
        else {
            panic!()
        };
        let table = &fragment.placement().cells[0].tables[0];
        close(table.placement.bounds.x, 15.0);
        close(table.placement.bounds.width, 90.0);
    }
}

#[test]
fn saved_absolute_body_float_keeps_width_beyond_text_lane_and_following_origin() {
    let d = rhwp::parse_document(WIDE).unwrap();
    let p = drain(
        DocumentV2Session::from_bytes(
            WIDE,
            r#"{"dpi":96,"max_pages":30,"cell_end_policy":"omit_final_paragraph_gap"}"#,
        )
        .unwrap(),
    );
    assert_eq!(p.len(), 1);
    let tables = nodes(&p[0], "Table");
    assert_eq!(tables.len(), 1);
    let t = tables[0];
    close(number(t, "x"), (5669.0 + 141.0) / 75.0);
    close(number(t, "width"), 49204.0 / 75.0);
    let pd = &d.sections[0].section_def.page_def;
    assert!(number(t, "x") + number(t, "width") > (pd.width - pd.margin_right) as f64 / 75.0);
    // Independent normal Hancom PDF path, not a V2-derived golden.
    for (key, pt) in [
        ("x", 58.049),
        ("y", 841.0 - 758.89),
        ("width", 549.784 - 58.049),
        ("height", 758.89 - 625.955),
    ] {
        assert!((number(t, key) - pt * 4.0 / 3.0).abs() < 0.6, "PDF {key}");
    }
    let after = body_line(&p[0], 2);
    close(number(after, "x"), 5669.0 / 75.0);
    close(
        number(after, "width"),
        d.sections[0].paragraphs[2].line_segs[0].segment_width as f64 / 75.0,
    );
    assert!(number(after, "width") < number(t, "width"));
    assert_eq!(text(after), "AFTER TABLE / ORIGINAL BODY WIDTH");
    let baseline =
        number(after, "y") + after["node_type"]["TextLine"]["baseline"].as_f64().unwrap();
    assert!((baseline - 1912.0 * 0.119869 * 4.0 / 3.0).abs() < 0.6);
    assert!(number(after, "y") >= number(t, "y") + number(t, "height"));
}

fn sync_section(d: &mut Document) {
    let def = d.sections[0].section_def.clone();
    for p in &mut d.sections[0].paragraphs {
        for c in &mut p.controls {
            if let Control::SectionDef(value) = c {
                **value = def.clone();
            }
        }
    }
}

#[test]
fn absolute_body_float_checks_paper_and_outer_margin_without_widening_text() {
    for deficit in [0, 1, 141] {
        let mut d = rhwp::parse_document(WIDE).unwrap();
        let pd = &mut d.sections[0].section_def.page_def;
        // Keep the authored text lane and all stored LineSeg widths unchanged;
        // change only the physical right paper margin. Exact fit then 1HU short.
        let width = 5669 + 141 + 49204 + 141 - deficit;
        pd.margin_right -= pd.width - width;
        pd.width = width;
        sync_section(&mut d);
        let result = session(&d);
        if deficit == 0 {
            let p = drain(result.unwrap());
            close(number(nodes(&p[0], "Table")[0], "width"), 49204.0 / 75.0);
        } else {
            assert!(
                matches!(
                    result,
                    Err(DocumentV2Error::Paragraph {
                        reason: GeometryError::Unsupported("stored body anchor outside paper"),
                        ..
                    })
                ),
                "deficit {deficit}"
            );
        }
    }
}

#[test]
fn wide_body_float_continuation_keeps_width_content_and_following_paragraph() {
    // Source has a four-row merged cell, atomic in RowBreak. Split that cell
    // structurally for this RowBreak continuation contract, retaining its text
    // only in the first cell. This is a mutation contract, not Hancom evidence.
    let mut source = rhwp::parse_document(WIDE).unwrap();
    let Control::Table(t) = &mut source.sections[0].paragraphs[1].controls[0] else {
        panic!()
    };
    let first = t
        .cells
        .iter_mut()
        .find(|c| c.row == 0 && c.col == 0)
        .unwrap();
    first.row_span = 1;
    first.height = 0;
    for p in &mut first.paragraphs {
        p.line_segs.clear();
    }
    let mut blank = first.clone();
    blank.paragraphs.truncate(1);
    let p = &mut blank.paragraphs[0];
    p.text.clear();
    p.char_count = 1;
    p.char_offsets.clear();
    p.char_shapes.truncate(1);
    p.char_shapes[0].start_pos = 0;
    for row in 1..4 {
        blank.row = row;
        t.cells.push(blank.clone());
    }
    t.cell_grid.clear();
    // Source-owned cell strings, not an intact V2 rendering used as a golden.
    let mut source_cells = t.cells.iter().collect::<Vec<_>>();
    source_cells.sort_by_key(|cell| (cell.row, cell.col));
    let expected = source_cells
        .iter()
        .map(|cell| {
            cell.paragraphs
                .iter()
                .map(|p| p.text.as_str())
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    let intact = pages(&source);
    assert_eq!(
        nodes(&intact[0], "TableCell")
            .into_iter()
            .map(text)
            .collect::<Vec<_>>(),
        expected
    );
    for height in [8000, 14000] {
        let mut d = source.clone();
        let pd = &mut d.sections[0].section_def.page_def;
        pd.height = pd.margin_top + pd.margin_header + pd.margin_bottom + pd.margin_footer + height;
        sync_section(&mut d);
        let p = pages(&d);
        assert!(p.len() > 1);
        let mut content = std::collections::BTreeMap::new();
        let mut host = 0;
        let mut after = 0;
        for p in &p {
            for t in nodes(p, "Table") {
                close(number(t, "x"), (5669.0 + 141.0) / 75.0);
                close(number(t, "width"), 49204.0 / 75.0);
                assert!(
                    number(t, "y") + number(t, "height") <= (5670 + height) as f64 / 75.0 + 1e-7
                );
            }
            for c in nodes(p, "TableCell") {
                let cell = &c["node_type"]["TableCell"];
                let key = (cell["row"].as_u64().unwrap(), cell["col"].as_u64().unwrap());
                content
                    .entry(key)
                    .or_insert_with(String::new)
                    .push_str(&text(c));
            }
            for line in nodes(p, "TextLine") {
                if line["node_type"]["TextLine"]["section_index"] == 0 {
                    match line["node_type"]["TextLine"]["para_index"].as_u64() {
                        Some(1) => host += 1,
                        Some(2) => {
                            after += 1;
                            close(number(line, "x"), 5669.0 / 75.0);
                        }
                        _ => {}
                    }
                }
            }
        }
        assert_eq!((host, after), (1, 1));
        assert_eq!(content.into_values().collect::<Vec<_>>(), expected);
    }
}

#[test]
fn wide_body_float_atomic_deferral_preserves_host_table_and_following_text() {
    let mut d = rhwp::parse_document(WIDE).unwrap();
    // Atomic deferral is a Never contract, not a consequence of rowspan.
    // The independent prefix14 PDF cuts through RowBreak spanning cells.
    for p in &mut d.sections[0].paragraphs {
        for c in &mut p.controls {
            if let Control::Table(t) = c {
                t.page_break = rhwp::model::table::TablePageBreak::None;
            }
        }
    }
    let pd = &mut d.sections[0].section_def.page_def;
    pd.height = pd.margin_top + pd.margin_header + pd.margin_bottom + pd.margin_footer + 14000;
    sync_section(&mut d);
    let p = pages(&d);
    // 32px title advance +177.55px table cannot fit186.67px; the intact
    // four-row merged table does fit a fresh page. Its following20px line
    // cannot fit after it. No geometry or content may be shrunk to avoid this.
    assert_eq!(p.len(), 3);
    assert!(nodes(&p[0], "Table").is_empty());
    assert_eq!(text(body_line(&p[0], 0)), "< 규제 개요 >");
    let t = nodes(&p[1], "Table");
    assert_eq!(t.len(), 1);
    close(number(t[0], "x"), (5669.0 + 141.0) / 75.0);
    close(number(t[0], "width"), 49204.0 / 75.0);
    close(number(t[0], "y"), (5670.0 + 141.0) / 75.0);
    assert_eq!(nodes(&p[1], "TableCell").len(), 11);
    assert_eq!(text(body_line(&p[1], 1)), "");
    assert!(nodes(&p[2], "Table").is_empty());
    assert_eq!(
        text(body_line(&p[2], 2)),
        "AFTER TABLE / ORIGINAL BODY WIDTH"
    );
}
