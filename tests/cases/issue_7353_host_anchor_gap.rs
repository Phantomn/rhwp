//! Stored host lines, paragraph anchors and exclusion clearance are distinct.
use rhwp::{model::control::Control, renderer::table_v2::DocumentV2Session};
use serde_json::Value;

const SAVED: &[u8] = include_bytes!("../fixtures/issue7353/host-insets/prefix35-saved.hwp");

fn nodes<'a>(node: &'a Value, kind: &str, out: &mut Vec<&'a Value>) {
    if node["node_type"].get(kind).is_some() {
        out.push(node);
    }
    for c in node["children"].as_array().unwrap() {
        nodes(c, kind, out);
    }
}
fn render(bytes: &[u8], dpi: f64) -> Vec<Value> {
    let mut s = DocumentV2Session::from_bytes(
        bytes,
        &format!(r#"{{"dpi":{dpi},"max_pages":100,"cell_end_policy":"omit_final_paragraph_gap"}}"#),
    )
    .unwrap();
    let mut pages = Vec::new();
    while let Some(p) = s.next_page_json().unwrap() {
        pages.push(serde_json::from_str(&p).unwrap());
    }
    assert!(s.next_page_json().unwrap().is_none());
    pages
}
fn near(v: &Value, expected: f64) {
    assert!(
        (v.as_f64().unwrap() - expected).abs() < 1e-8,
        "{v} != {expected}"
    );
}

#[test]
fn normal_saved_empty_host_keeps_line_and_paragraph_relative_child_origin() {
    let doc = rhwp::parse_document(SAVED).unwrap();
    let p = &doc.sections[0].paragraphs[34];
    assert!(p.text.is_empty());
    assert_eq!(
        doc.doc_info.para_shapes[p.para_shape_id as usize].indent,
        -2500
    );
    assert_eq!(p.line_segs[0].line_height, 400);
    assert_eq!(p.line_segs[0].line_spacing, 200);
    for dpi in [96., 192.] {
        let pages = render(SAVED, dpi);
        assert_eq!(pages.len(), 11);
        let root = &pages[10]["render_tree"]["root"];
        let mut lines = Vec::new();
        nodes(root, "TextLine", &mut lines);
        let owned = |pi| {
            *lines
                .iter()
                .find(|n| n["node_type"]["TextLine"]["para_index"] == pi)
                .unwrap()
        };
        let scale = dpi / 7200.;
        let def = &doc.sections[0].section_def.page_def;
        let y = f64::from(def.margin_top + def.margin_header) * scale;
        // Real first blank1000+gap500, title1500+gap752, host400+gap200.
        near(&owned(32)["bbox"]["y"], y);
        near(&owned(33)["bbox"]["y"], y + 1500. * scale);
        near(&owned(34)["bbox"]["y"], y + 3752. * scale);
        near(&owned(34)["bbox"]["height"], 400. * scale);
        let mut tables = Vec::new();
        nodes(root, "Table", &mut tables);
        assert_eq!(tables.len(), 2);
        // Independent source origin: host + offset448 + top margin141.
        // This is11HU inside the trailing200HU gap, never inside the400HU line.
        let x = f64::from(def.margin_left) * scale;
        near(&tables[0]["bbox"]["x"], x + (41. + 141.) * scale);
        near(&tables[0]["bbox"]["y"], y + (3752. + 448. + 141.) * scale);
        near(&tables[0]["bbox"]["height"], 9062. * scale);
        near(
            &tables[1]["bbox"]["y"],
            y + (3752. + 448. + 141. + 141. + 141.) * scale,
        );
        near(&tables[1]["bbox"]["width"], 47622. * scale);
        // Parent47867; child left141 + width47622 fits. Right clearance141
        // would exceed by37HU, but no child border/content exceeds the parent.
        let child_right = tables[1]["bbox"]["x"].as_f64().unwrap() + 47622. * scale;
        let parent_right = tables[0]["bbox"]["x"].as_f64().unwrap() + 47867. * scale;
        assert!(child_right < parent_right && child_right + 141. * scale > parent_right);
        let mut runs = Vec::new();
        nodes(tables[1], "TextRun", &mut runs);
        let text = runs
            .iter()
            .map(|n| n["node_type"]["TextRun"]["text"].as_str().unwrap())
            .collect::<String>();
        for label in [
            "구분",
            "장점",
            "단점",
            "규제대안1",
            "규제대안2",
            "설립 부담",
        ] {
            assert!(text.contains(label), "{text}");
        }
    }
}

#[test]
fn exclusion_clearance_is_not_ink_but_actual_child_overflow_is_rejected() {
    // Explicit mutations, not additional normally saved specimens.
    let baseline = render(SAVED, 96.);
    let mut d = rhwp::parse_document(SAVED).unwrap();
    let Control::Table(parent) = &mut d.sections[0].paragraphs[34].controls[0] else {
        unreachable!()
    };
    let Control::Table(child) = &mut parent.cells[0].paragraphs[0].controls[0] else {
        unreachable!()
    };
    child.common.margin.right = 10000;
    child.outer_margin_right = 10000;
    let bytes = rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap();
    let enlarged = render(&bytes, 96.);
    assert_eq!(baseline[10]["render_tree"], enlarged[10]["render_tree"]);
    let Control::Table(parent) = &mut d.sections[0].paragraphs[34].controls[0] else {
        unreachable!()
    };
    let Control::Table(child) = &mut parent.cells[0].paragraphs[0].controls[0] else {
        unreachable!()
    };
    child.common.horizontal_offset = 105; //141+105+47622=47868 >47867 by1HU
    let bytes = rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap();
    let error = DocumentV2Session::from_bytes(
        &bytes,
        r#"{"dpi":96,"max_pages":100,"cell_end_policy":"omit_final_paragraph_gap"}"#,
    )
    .err()
    .unwrap();
    assert!(format!("{error:?}").contains("ContentWidth"), "{error:?}");
}

#[test]
fn saved_anchor_cannot_occupy_the_real_blank_line_box() {
    let mut d = rhwp::parse_document(SAVED).unwrap();
    let Control::Table(t) = &mut d.sections[0].paragraphs[34].controls[0] else {
        unreachable!()
    };
    t.common.vertical_offset = 258; //258+141=399HU < real400HU line box
    let bytes = rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap();
    let error = DocumentV2Session::from_bytes(
        &bytes,
        r#"{"dpi":96,"max_pages":100,"cell_end_policy":"omit_final_paragraph_gap"}"#,
    )
    .err()
    .unwrap();
    assert!(
        format!("{error:?}").contains("stored body anchor intersects host flow"),
        "{error:?}"
    );
}

#[test]
fn following_host_exclusion_uses_the_same_physical_width_contract() {
    let saved = include_bytes!("../fixtures/issue7353_following_anchor_review/visible-saved.hwp");
    let baseline = render(saved, 96.);
    let mut d = rhwp::parse_document(saved).unwrap();
    let mut changed = 0;
    for para in &mut d.sections[0].paragraphs {
        for ctrl in &mut para.controls {
            if let Control::Table(parent) = ctrl {
                for cell in &mut parent.cells {
                    for host in &mut cell.paragraphs {
                        for ctrl in &mut host.controls {
                            if let Control::Table(child) = ctrl {
                                assert!(!host.text.is_empty());
                                assert!(!child.common.treat_as_char);
                                child.common.margin.right = 20000;
                                child.outer_margin_right = 20000;
                                changed += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(changed, 1);
    let mutated = rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap();
    let pages = render(&mutated, 96.);
    assert_eq!(pages.len(), baseline.len());
    for (a, b) in baseline.iter().zip(pages.iter()) {
        assert_eq!(a["render_tree"], b["render_tree"]);
    }
}
