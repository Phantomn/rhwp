//! Normal Hancom saves + independent PDF bounds; see the fixture README.
//! CENTER aligns margin-inclusive boxes. Test actual child bounds, occupied
//! parent height, following cell/body text and termination, not helper output.
use rhwp::{model::control::Control, renderer::table_v2::DocumentV2Session};
use serde_json::Value;

const CENTER: &[u8] = include_bytes!("../fixtures/issue7353_center_tac_review/center-saved.hwp");
const ASYMMETRIC: &[u8] =
    include_bytes!("../fixtures/issue7353_center_tac_review/center-asymmetric-saved.hwp");

fn nodes<'a>(v: &'a Value, kind: &str, out: &mut Vec<&'a Value>) {
    if v["node_type"][kind].is_object() {
        out.push(v);
    }
    if let Some(children) = v["children"].as_array() {
        for c in children {
            nodes(c, kind, out);
        }
    }
}
fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-8, "{actual} != {expected}");
}
fn check(input: &[u8], asymmetric: bool) {
    for dpi in [96., 192.] {
        let options = format!(
            r#"{{"dpi":{dpi},"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}}"#
        );
        let mut s = DocumentV2Session::from_bytes(input, &options).unwrap();
        let page: Value = serde_json::from_str(&s.next_page_json().unwrap().unwrap()).unwrap();
        assert!(s.next_page_json().unwrap().is_none());
        assert_eq!(s.emitted_pages(), 1);
        let mut tables = Vec::new();
        nodes(&page["render_tree"]["root"], "Table", &mut tables);
        assert_eq!(tables.len(), 3);
        let scale = dpi / 7200.;
        let (small_y, tall_y, after_y) = if asymmetric {
            (12830., 10930., 20190.)
        } else {
            (12530., 11030., 19890.)
        };
        for (t, y, height) in [
            (tables[0], 8787., 18000.),
            (tables[1], small_y, 5000.),
            (tables[2], tall_y, 8000.),
        ] {
            near(t["bbox"]["y"].as_f64().unwrap(), y * scale);
            near(t["bbox"]["height"].as_f64().unwrap(), height * scale);
        }
        // Independent PDF clips, page-top pt; normal print-axis quantization.
        // Incorrectly centering bare boxes fails the asymmetric small-table y.
        let clips = if asymmetric {
            [
                [97.559, 128.211, 99.956, 49.893],
                [201.470, 109.141, 99.836, 79.997],
            ]
        } else {
            [
                [97.559, 125.213, 99.956, 49.893],
                [201.470, 110.221, 99.836, 79.877],
            ]
        };
        for (table, expected) in tables[1..].iter().zip(clips) {
            for (field, pt) in ["x", "y", "width", "height"].into_iter().zip(expected) {
                let print_scale = if matches!(field, "x" | "width") {
                    0.119851 / 0.12
                } else {
                    0.119935 / 0.12
                };
                let actual = table["bbox"][field].as_f64().unwrap() * 72. / dpi * print_scale;
                assert!((actual - pt).abs() < 0.25, "{field}: {actual} vs {pt}");
            }
        }
        let mut runs = Vec::new();
        nodes(&page["render_tree"]["root"], "TextRun", &mut runs);
        for (text, y) in [
            ("SHORT TABLE", small_y + 283.),
            ("TALL TABLE", tall_y + 283.),
            ("CELL AFTER", after_y),
            ("AFTER PARENT TABLE", 27354.),
        ] {
            let found: Vec<_> = runs
                .iter()
                .filter(|r| r["node_type"]["TextRun"]["text"] == text)
                .collect();
            assert_eq!(found.len(), 1, "{text}");
            near(found[0]["bbox"]["y"].as_f64().unwrap(), y * scale);
        }
    }
}
#[test]
fn hancom_center_tac_retains_common_outer_center_and_following_content() {
    check(CENTER, false);
}
#[test]
fn hancom_asymmetric_tac_margins_are_part_of_the_centered_box() {
    check(ASYMMETRIC, true);
}
#[test]
fn center_does_not_admit_stale_baseline_cache_or_unimplemented_fresh_alignment() {
    for fresh in [false, true] {
        let mut d = rhwp::parse_document(CENTER).unwrap();
        let parent = d.sections[0].paragraphs[0]
            .controls
            .iter_mut()
            .find_map(|c| match c {
                Control::Table(t) => Some(t),
                _ => None,
            })
            .unwrap();
        let p = &mut parent.cells[0].paragraphs[1];
        if fresh {
            p.line_segs.clear();
        } else {
            p.line_segs[0].baseline_distance = 7000;
        }
        let bytes = rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap();
        let err = DocumentV2Session::from_bytes(&bytes, r#"{"dpi":96,"max_pages":10}"#)
            .err()
            .unwrap();
        assert!(
            format!("{err:?}").contains(if fresh {
                "TAC paragraph vertical alignment"
            } else {
                "stored TAC center reference mismatch"
            }),
            "{err:?}"
        );
    }
}
