//! Saved unbreakable TAC images keep physical width independently of their line lane.
use rhwp::{
    model::{control::Control, document::Document, paragraph::Paragraph, style::Alignment},
    renderer::table_v2::DocumentV2Session,
};
use serde_json::Value;
const INPUT: &[u8] =
    include_bytes!("../fixtures/issue7353_overwide_picture_review/visible-saved.hwp");
fn doc() -> Document {
    rhwp::parse_document(INPUT).unwrap()
}
fn host(d: &mut Document) -> &mut Paragraph {
    let t = d.sections[0].paragraphs[1]
        .controls
        .iter_mut()
        .find_map(|c| {
            if let Control::Table(t) = c {
                Some(t)
            } else {
                None
            }
        })
        .unwrap();
    &mut t.cells[0].paragraphs[0]
}
fn bytes(d: &Document) -> Vec<u8> {
    rhwp::serializer::serialize_hwpx(d).unwrap()
}
fn page(input: &[u8], dpi: f64) -> Value {
    let mut s = DocumentV2Session::from_bytes(
        input,
        &format!(r#"{{"dpi":{dpi},"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}}"#),
    )
    .unwrap();
    let p = serde_json::from_str(&s.next_page_json().unwrap().unwrap()).unwrap();
    assert!(s.next_page_json().unwrap().is_none());
    p
}
fn nodes<'a>(n: &'a Value, kind: &str, out: &mut Vec<&'a Value>) {
    if n["node_type"].get(kind).is_some() {
        out.push(n);
    }
    for c in n["children"].as_array().unwrap() {
        nodes(c, kind, out);
    }
}
fn collect<'a>(p: &'a Value, kind: &str) -> Vec<&'a Value> {
    let mut v = vec![];
    nodes(&p["render_tree"]["root"], kind, &mut v);
    v
}
fn val(n: &Value, key: &str) -> f64 {
    n["bbox"][key].as_f64().unwrap()
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}
#[test]
fn hancom_saved_overwide_image_keeps_full_frame_without_negative_centering() {
    let d = doc();
    let hwpx = bytes(&d);
    for input in [INPUT, hwpx.as_slice()] {
        for dpi in [96., 192.] {
            let p = page(input, dpi);
            let u = dpi / 7200.;
            let pics = collect(&p, "Image");
            assert_eq!(pics.len(), 1);
            let pic = pics[0];
            // Source page origin + host vpos + outer table offsets/margins.
            for (key, hu) in [
                ("x", 34252.),
                ("y", 11936.),
                ("width", 19686.),
                ("height", 9461.),
            ] {
                near(val(pic, key), hu * u);
            }
            // Independent Hancom PDF image transform, printer tolerance0.25pt.
            for (key, pt) in [
                ("x", 342.296),
                ("y", 119.14902),
                ("width", 196.814),
                ("height", 94.457),
            ] {
                assert!((val(pic, key) * 72. / dpi - pt).abs() < 0.25);
            }
            let lines = collect(&p, "TextLine");
            let line = lines
                .iter()
                .find(|n| {
                    n["children"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|c| c["node_type"].get("Image").is_some())
                })
                .unwrap();
            near(val(line, "width"), 19604. * u);
            near(val(line, "x"), val(pic, "x"));
            let tables = collect(&p, "Table");
            assert_eq!(tables.len(), 2);
            near(val(tables[1], "width"), 19607. * u);
            near(val(tables[1], "height"), 9461. * u);
            near(
                val(pic, "x") + val(pic, "width") - val(tables[1], "x") - val(tables[1], "width"),
                79. * u,
            );
            for pi in [2, 3] {
                let body_lines: Vec<_> = collect(&p, "TextLine")
                    .into_iter()
                    .filter(|n| n["node_type"]["TextLine"]["para_index"] == pi)
                    .collect();
                assert_eq!(
                    body_lines.len(),
                    d.sections[0].paragraphs[pi].line_segs.len()
                );
                for (line, source) in body_lines
                    .iter()
                    .zip(&d.sections[0].paragraphs[pi].line_segs)
                {
                    near(val(line, "y"), (5668. + f64::from(source.vertical_pos)) * u);
                    near(val(line, "width"), f64::from(source.segment_width) * u);
                }
                let text: String = collect(&p, "TextRun")
                    .into_iter()
                    .filter(|n| n["node_type"]["TextRun"]["para_index"] == pi)
                    .map(|n| n["node_type"]["TextRun"]["text"].as_str().unwrap())
                    .collect();
                assert_eq!(text, d.sections[0].paragraphs[pi].text.replace('\n', ""));
            }
        }
    }
}
#[test]
fn alignment_uses_spare_lane_width_only_when_it_exists() {
    for align in [Alignment::Left, Alignment::Center, Alignment::Right] {
        for width in [19603, 19604, 19605, 19686] {
            let mut d = doc();
            let ps = host(&mut d).para_shape_id as usize;
            d.doc_info.para_shapes[ps].alignment = align;
            let Control::Picture(pic) = &mut host(&mut d).controls[0] else {
                panic!()
            };
            pic.common.width = width;
            let p = page(&bytes(&d), 96.);
            let pics = collect(&p, "Image");
            let free = (19604. - f64::from(width)).max(0.);
            let offset = match align {
                Alignment::Center => free / 2.,
                Alignment::Right => free,
                _ => 0.,
            };
            near(val(pics[0], "x"), (34252. + offset) / 75.);
            near(val(pics[0], "width"), f64::from(width) / 75.);
        }
    }
}
#[test]
fn multiple_overwide_objects_do_not_inherit_single_object_permission() {
    let mut d = doc();
    let h = host(&mut d);
    h.controls.push(h.controls[0].clone());
    h.char_count = 17;
    let err = DocumentV2Session::from_bytes(
        &bytes(&d),
        r#"{"dpi":96,"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}"#,
    )
    .err()
    .unwrap();
    assert!(
        err.to_string().contains("TAC row exceeds stored width"),
        "{err}"
    );
}

#[test]
fn leading_space_does_not_inherit_single_unbreakable_object_permission() {
    let mut d = doc();
    let h = host(&mut d);
    h.text = " ".into();
    h.char_offsets = vec![0];
    h.char_count = 10;
    let err = DocumentV2Session::from_bytes(
        &bytes(&d),
        r#"{"dpi":96,"max_pages":10,"cell_end_policy":"omit_final_paragraph_gap"}"#,
    )
    .err()
    .unwrap();
    assert!(
        err.to_string().contains("TAC row exceeds stored width"),
        "{err}"
    );
}
#[test]
fn overwide_image_vertical_fit_preserves_atomic_height_and_following_blank() {
    use rhwp::renderer::{
        render_tree::PageRenderTree, style_resolver::resolve_styles, table_v2::*,
    };
    let d = doc();
    let Control::Table(t) = d.sections[0].paragraphs[1]
        .controls
        .iter()
        .find(|c| matches!(c, Control::Table(_)))
        .unwrap()
    else {
        panic!()
    };
    let mut t = t.clone();
    t.common.height = 0;
    t.cells[0].height = 0;
    let mut after = t.cells[0].paragraphs[0].clone();
    after.controls.clear();
    after.char_count = 1;
    let r = &mut after.line_segs[0];
    r.vertical_pos = 10181;
    r.line_height = 1200;
    r.text_height = 1200;
    r.baseline_distance = 1020;
    r.line_spacing = 720;
    t.cells[0].paragraphs.push(after);
    let styles = resolve_styles(&d.doc_info, 96.);
    let prepared = PreparedTextTable::prepare_with_end_policy(
        &t,
        &styles,
        96.,
        &d.bin_data_content,
        CellEndPolicy::OmitFinalParagraphGap,
    )
    .unwrap();
    let area = |height| PageArea {
        bounds: Rect {
            x: 20.,
            y: 30.,
            width: 400.,
            height,
        },
    };
    assert!(matches!(
        prepared.start().fit(area(9460. / 75.)).unwrap(),
        TextFragmentFit::DoesNotFit { .. }
    ));
    let TextFragmentFit::Placed(first) = prepared.start().fit(area(10181. / 75.)).unwrap() else {
        panic!()
    };
    near(first.geometry().reserved_height(), 10181. / 75.);
    let mut tree = PageRenderTree::new(0, 600., 600.);
    first.append_to(&mut tree).unwrap();
    let p = serde_json::json!({"render_tree":tree});
    let pics = collect(&p, "Image");
    assert_eq!(pics.len(), 1);
    near(val(pics[0], "width"), 19686. / 75.);
    near(val(pics[0], "height"), 9461. / 75.);
    let TextFragmentFit::Placed(last) = first.continuation().fit(area(1200. / 75.)).unwrap() else {
        panic!()
    };
    let mut tree = PageRenderTree::new(1, 600., 600.);
    last.append_to(&mut tree).unwrap();
    let p = serde_json::json!({"render_tree":tree});
    assert!(collect(&p, "Image").is_empty());
    let lines = collect(&p, "TextLine");
    assert_eq!(lines.len(), 1);
    near(val(lines[0], "y"), 30.);
    assert!(matches!(
        last.continuation().fit(area(500.)).unwrap(),
        TextFragmentFit::Complete
    ));
}
