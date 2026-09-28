//! A Square object's host can occupy its left lane at the same vertical height.
use rhwp::{
    model::{control::Control, document::Document, table::Table},
    renderer::table_v2::*,
};
use serde_json::Value;
const INPUT: &[u8] = include_bytes!("../fixtures/issue7353_host_wrap_review/host-saved.hwp");
fn doc() -> Document {
    rhwp::parse_document(INPUT).unwrap()
}
fn encode(d: &Document) -> Vec<u8> {
    rhwp::serializer::serialize_hwpx(d).unwrap()
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
fn nodes<'a>(n: &'a Value, kind: &str, out: &mut Vec<&'a Value>) {
    if n["node_type"].get(kind).is_some() {
        out.push(n);
    }
    for c in n["children"].as_array().unwrap() {
        nodes(c, kind, out);
    }
}
fn num(n: &Value, k: &str) -> f64 {
    n["bbox"][k].as_f64().unwrap()
}
fn visible_host(d: &mut Document) {
    let host = &mut d.sections[0].paragraphs[1];
    host.text = "HOST".into();
    // The normal blank carrier also declares a page number. That separate
    // declaration is not qualified inside visible prose; this lane contract
    // changes text visibility, not page-number declaration support.
    host.controls
        .retain(|c| !matches!(c, Control::PageNumberPos(_)));
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}

#[test]
fn real_saved_host_line_and_side_box_keep_independent_occupied_origins() {
    let d = doc();
    let hwpx = encode(&d);
    for bytes in [INPUT, hwpx.as_slice()] {
        for dpi in [96., 192.] {
            let mut s = open(bytes, dpi).unwrap();
            let page: Value = serde_json::from_str(&s.next_page_json().unwrap().unwrap()).unwrap();
            assert!(s.next_page_json().unwrap().is_none());
            let mut lines = vec![];
            let mut tables = vec![];
            let mut runs = vec![];
            let root = &page["render_tree"]["root"];
            nodes(root, "TextLine", &mut lines);
            nodes(root, "Table", &mut tables);
            nodes(root, "TextRun", &mut runs);
            let host = lines
                .iter()
                .find(|n| n["node_type"]["TextLine"]["para_index"] == 1)
                .unwrap();
            let object = tables
                .iter()
                .find(|n| n["node_type"]["Table"]["para_index"] == 1)
                .unwrap();
            let u = dpi / 7200.;
            // Source HU, independently printed by Hancom: overlap in Y, not in 2D.
            near(num(host, "x"), 5669. * u);
            near(num(host, "y"), (5668. + 6764.) * u);
            near(num(host, "height"), 800. * u);
            near(num(host, "width"), 26319. * u);
            near(num(object, "x"), (5669. + 26319. + 1417.) * u);
            near(num(object, "y"), (5668. + 6764. + 498. + 138.) * u);
            near(num(object, "height"), 13241. * u);
            near(num(object, "width"), 21022. * u);
            assert!(num(object, "y") < num(host, "y") + num(host, "height"));
            assert!(num(host, "x") + num(host, "width") <= num(object, "x") - 1417. * u + 1e-7);
            let body: Vec<_> = lines
                .iter()
                .filter(|n| n["node_type"]["TextLine"]["para_index"] == 2)
                .collect();
            assert_eq!(body.len(), 5);
            for (i, line) in body.iter().enumerate() {
                near(num(line, "y"), (5668. + 7964. + i as f64 * 2252.) * u);
                near(num(line, "width"), 26319. * u);
            }
            for (pi, y, h) in [(3, 19224., 100.), (4, 19376., 1200.)] {
                let line = lines
                    .iter()
                    .find(|n| n["node_type"]["TextLine"]["para_index"] == pi)
                    .unwrap();
                near(num(line, "y"), (5668. + y) * u);
                near(num(line, "height"), h * u);
            }
            for pi in [2, 4] {
                let text: String = runs
                    .iter()
                    .filter(|n| n["node_type"]["TextRun"]["para_index"] == pi)
                    .map(|n| n["node_type"]["TextRun"]["text"].as_str().unwrap())
                    .collect();
                assert_eq!(text, d.sections[0].paragraphs[pi].text);
            }
            assert!(runs
                .iter()
                .any(|n| n["node_type"]["TextRun"]["text"] == "- 1 -"));
        }
    }
}

#[test]
fn actual_host_intersection_is_rejected_even_when_the_host_is_blank() {
    for visible in [false, true] {
        let mut d = doc();
        if visible {
            visible_host(&mut d);
        }
        d.sections[0].paragraphs[1].line_segs[0].segment_width = 48188;
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
fn visible_host_in_the_same_safe_lane_is_not_treated_as_full_width() {
    let mut d = doc();
    visible_host(&mut d);
    let mut s = open(&encode(&d), 96.).unwrap();
    let page: Value = serde_json::from_str(&s.next_page_json().unwrap().unwrap()).unwrap();
    let mut runs = vec![];
    nodes(&page["render_tree"]["root"], "TextRun", &mut runs);
    assert!(runs
        .iter()
        .any(|n| n["node_type"]["TextRun"]["text"] == "HOST"));
    assert!(s.next_page_json().unwrap().is_none());
}

#[test]
fn top_and_bottom_still_excludes_the_full_host_width() {
    let mut d = doc();
    table(&mut d).common.text_wrap = rhwp::model::shape::TextWrap::TopAndBottom;
    assert!(open(&encode(&d), 96.)
        .err()
        .unwrap()
        .to_string()
        .contains("anchor intersects host flow"));
}

fn set_page_height(d: &mut Document, height: u32) {
    d.sections[0].section_def.page_def.height = height;
    let def = d.sections[0].section_def.clone();
    for c in &mut d.sections[0].paragraphs[0].controls {
        if let Control::SectionDef(value) = c {
            **value = def.clone();
        }
    }
}

#[test]
fn budget_uses_the_actual_box_end_not_host_bottom_plus_box_height() {
    // 11336HU paper margins + (6764+498+138+13241+138) occupied body end.
    // One HU either side avoids deciding an exact decimal boundary by rounding.
    for (height, fits) in [(32116, true), (32114, false)] {
        let mut d = doc();
        set_page_height(&mut d, height);
        let mut s = open(&encode(&d), 96.).unwrap();
        if fits {
            assert!(s.next_page_json().unwrap().is_some());
            assert!(s.next_page_json().unwrap().is_none());
        } else {
            for _ in 0..2 {
                assert!(s
                    .next_page_json()
                    .unwrap_err()
                    .to_string()
                    .contains("side-wrap across pages"));
                assert_eq!(s.emitted_pages(), 0);
            }
        }
    }
}

#[test]
fn a_host_split_across_pages_cannot_supply_a_false_paragraph_origin() {
    let mut d = doc();
    let control = d.sections[0].paragraphs[1].controls.remove(1);
    d.sections[0].paragraphs[2].controls.push(control);
    // Positive control: all five host rows on one page share the same paragraph
    // origin, irrespective of the final row's height or visible text.
    let mut intact = open(&encode(&d), 96.).unwrap();
    let page: Value = serde_json::from_str(&intact.next_page_json().unwrap().unwrap()).unwrap();
    let mut tables = vec![];
    nodes(&page["render_tree"]["root"], "Table", &mut tables);
    let object = tables
        .iter()
        .find(|n| n["node_type"]["Table"]["para_index"] == 2)
        .unwrap();
    near(num(object, "y"), (5668. + 7964. + 498. + 138.) / 75.);
    assert!(intact.next_page_json().unwrap().is_none());
    // The five narrow text rows now own the object. This is a contract mutation,
    // not an independently observed Hancom output. Their paragraph must not be
    // anchored relative to only the rows that survived on the second page.
    set_page_height(&mut d, 29336); // 18000HU body, cuts the five-row host.
    let mut s = open(&encode(&d), 96.).unwrap();
    assert!(s.next_page_json().unwrap().is_some());
    assert_eq!(s.emitted_pages(), 1);
    for _ in 0..2 {
        assert!(s
            .next_page_json()
            .unwrap_err()
            .to_string()
            .contains("side-wrap across pages"));
        assert_eq!(s.emitted_pages(), 1);
    }
}
