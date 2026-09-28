//! Normal Hancom saved Square table: six left lanes followed by full-width text.
use rhwp::{
    model::{control::Control, document::Document, table::Table},
    renderer::table_v2::*,
};
use serde_json::Value;
const INPUT: &[u8] = include_bytes!("../fixtures/issue7353_body_wrap_review/wrap-saved.hwp");
fn doc() -> Document {
    rhwp::parse_document(INPUT).unwrap()
}
fn table(d: &mut Document) -> &mut Table {
    match &mut d.sections[0].paragraphs[1].controls[1] {
        Control::Table(t) => t,
        _ => panic!(),
    }
}
fn open(bytes: &[u8], dpi: f64) -> Result<DocumentV2Session, DocumentV2Error> {
    DocumentV2Session::from_bytes(
        bytes,
        &format!(r#"{{"dpi":{dpi},"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}}"#),
    )
}
fn encode(d: &Document) -> Vec<u8> {
    rhwp::serializer::serialize_hwpx(d).unwrap()
}
fn nodes<'a>(n: &'a Value, kind: &str, out: &mut Vec<&'a Value>) {
    if n["node_type"].get(kind).is_some() {
        out.push(n);
    }
    for c in n["children"].as_array().unwrap() {
        nodes(c, kind, out);
    }
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}
fn number(n: &Value, k: &str) -> f64 {
    n["bbox"][k].as_f64().unwrap()
}

#[test]
fn stored_square_keeps_host_six_side_rows_full_width_tail_and_footer() {
    let d = doc();
    let hwpx = encode(&d);
    for data in [INPUT, hwpx.as_slice()] {
        for dpi in [96., 192.] {
            let mut s = open(data, dpi).unwrap();
            let p: Value = serde_json::from_str(&s.next_page_json().unwrap().unwrap()).unwrap();
            assert!(s.next_page_json().unwrap().is_none());
            let mut lines = vec![];
            let mut tables = vec![];
            nodes(&p["render_tree"]["root"], "TextLine", &mut lines);
            nodes(&p["render_tree"]["root"], "Table", &mut tables);
            assert_eq!(tables.len(), 2);
            let side = tables
                .iter()
                .find(|n| n["node_type"]["Table"]["para_index"] == 1)
                .unwrap();
            let scale = dpi / 7200.;
            // Independent source HU metrics confirmed by the same saved HWP's PDF.
            near(number(side, "x"), (5669. + 26319. + 1417.) * scale);
            near(number(side, "y"), (5668. + 4696. + 834. + 138.) * scale);
            near(number(side, "width"), 21022. * scale);
            near(number(side, "height"), 13241. * scale);
            let body: Vec<_> = lines
                .iter()
                .filter(|n| n["node_type"]["TextLine"]["para_index"] == 2)
                .collect();
            assert_eq!(body.len(), 7);
            for (i, line) in body.iter().enumerate() {
                near(
                    number(line, "y"),
                    (5668. + 5896. + i as f64 * 2252.) * scale,
                );
                near(number(line, "x"), 5669. * scale);
                near(
                    number(line, "width"),
                    if i < 6 {
                        26319. * scale
                    } else {
                        48188. * scale
                    },
                );
            }
            let host = lines
                .iter()
                .find(|n| n["node_type"]["TextLine"]["para_index"] == 1)
                .unwrap();
            near(number(host, "y"), (5668. + 4696.) * scale);
            near(number(host, "height"), 800. * scale);
            let after = lines
                .iter()
                .find(|n| n["node_type"]["TextLine"]["para_index"] == 3)
                .unwrap();
            near(number(after, "y"), (5668. + 21660.) * scale);
            near(number(after, "height"), 400. * scale);
            let mut runs = vec![];
            nodes(&p["render_tree"]["root"], "TextRun", &mut runs);
            let text: String = runs
                .iter()
                .filter(|n| n["node_type"]["TextRun"]["para_index"] == 2)
                .map(|n| n["node_type"]["TextRun"]["text"].as_str().unwrap())
                .collect();
            assert_eq!(text, d.sections[0].paragraphs[2].text);
            assert!(runs
                .iter()
                .any(|n| n["node_type"]["TextRun"]["text"] == "- 1 -"));
        }
    }
}

#[test]
fn stale_full_width_or_intruding_anchor_is_rejected_before_page_commit() {
    for widen in [true, false] {
        let mut d = doc();
        if widen {
            d.sections[0].paragraphs[2].line_segs[0].segment_width = 48188;
        } else {
            table(&mut d).common.horizontal_offset -= 100;
        }
        let mut s = open(&encode(&d), 96.).unwrap();
        for _ in 0..2 {
            assert!(s
                .next_page_json()
                .unwrap_err()
                .to_string()
                .contains("side-wrap intersects flow"));
            assert_eq!(s.emitted_pages(), 0);
        }
    }
}

#[test]
fn unqualified_fresh_wrapping_or_outside_paper_stays_explicit() {
    for outside in [false, true] {
        let mut d = doc();
        if outside {
            table(&mut d).common.horizontal_offset = 59528;
        } else {
            d.sections[0].paragraphs[1].line_segs.clear();
        }
        assert!(open(&encode(&d), 96.).is_err());
    }
}

#[test]
fn side_object_cannot_be_partially_committed_or_deferred_away_from_saved_lanes() {
    let mut d = doc();
    d.sections[0].section_def.page_def.height = 24000;
    let def = d.sections[0].section_def.clone();
    for c in &mut d.sections[0].paragraphs[0].controls {
        if let Control::SectionDef(value) = c {
            **value = def.clone();
        }
    }
    let mut s = open(&encode(&d), 96.).unwrap();
    for _ in 0..2 {
        assert!(s
            .next_page_json()
            .unwrap_err()
            .to_string()
            .contains("side-wrap across pages"));
        assert_eq!(s.emitted_pages(), 0);
    }
}
