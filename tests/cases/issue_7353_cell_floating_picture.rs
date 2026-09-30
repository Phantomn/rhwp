//! A positioned cell picture and its empty host have separate physical boxes.
use rhwp::{
    model::{control::Control, document::Document, paragraph::Paragraph, shape::TextWrap},
    renderer::table_v2::{DocumentV2Error, DocumentV2Session},
};
use serde_json::Value;

const ORIGINAL: &[u8] =
    include_bytes!("../fixtures/issue7353_cell_picture_review/picture-saved.hwp");
const VISIBLE: &[u8] =
    include_bytes!("../fixtures/issue7353_cell_picture_review/visible-saved.hwp");
fn host(d: &mut Document) -> &mut Paragraph {
    let table = d.sections[0].paragraphs[1]
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
    &mut table.cells[0].paragraphs[0]
}
fn encode(d: &Document) -> Vec<u8> {
    rhwp::serializer::serialize_hwpx(d).unwrap()
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
fn collect<'a>(page: &'a Value, kind: &str) -> Vec<&'a Value> {
    let mut result = vec![];
    nodes(&page["render_tree"]["root"], kind, &mut result);
    result
}
fn num(n: &Value, key: &str) -> f64 {
    n["bbox"][key].as_f64().unwrap()
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}
fn page(bytes: &[u8], dpi: f64) -> Value {
    let mut s = open(bytes, dpi).unwrap();
    let result = serde_json::from_str(&s.next_page_json().unwrap().unwrap()).unwrap();
    assert!(s.next_page_json().unwrap().is_none());
    assert!(s.next_page_json().unwrap().is_none());
    result
}

#[test]
fn normal_saved_picture_preserves_host_cell_alignment_and_source_prose() {
    for input in [ORIGINAL, VISIBLE] {
        let d = rhwp::parse_document(input).unwrap();
        let hwpx = encode(&d);
        for bytes in [input, hwpx.as_slice()] {
            for dpi in [96., 192.] {
                let p = page(bytes, dpi);
                let u = dpi / 7200.;
                let tables = collect(&p, "Table");
                assert_eq!(tables.len(), 2);
                let t = tables[1];
                for (key, hu) in [
                    ("x", 35818.),
                    ("y", 11355.),
                    ("width", 18475.),
                    ("height", 14377.),
                ] {
                    near(num(t, key), hu * u);
                }
                let pictures = collect(&p, "Image");
                assert_eq!(pictures.len(), 1);
                let pic = pictures[0];
                // Source image bottom=263+13562=13825HU. Center in14377HU cell:
                // 276HU alignment offset; image starts at263+276=539HU.
                for (key, hu) in [
                    ("x", 36095.),
                    ("y", 11894.),
                    ("width", 17764.),
                    ("height", 13562.),
                ] {
                    near(num(pic, key), hu * u);
                }
                // Independent Hancom PDF draw-image rectangle in points. Its printer
                // rounding/scaling is not exact HU conversion; tolerance is0.25pt.
                for (key, pt) in [
                    ("x", 360.766),
                    ("y", 118.79001),
                    ("width", 177.505),
                    ("height", 135.452),
                ] {
                    assert!((num(pic, key) * 72. / dpi - pt).abs() < 0.25);
                }
                let empty: Vec<_> = collect(&p, "TextLine")
                    .into_iter()
                    .filter(|l| num(l, "width") == 0.)
                    .collect();
                assert_eq!(empty.len(), 1);
                near(num(empty[0], "y"), (11355. + 276.) * u);
                near(num(empty[0], "height"), 1200. * u);
                let lines: Vec<_> = collect(&p, "TextLine")
                    .into_iter()
                    .filter(|n| n["node_type"]["TextLine"]["para_index"] == 2)
                    .collect();
                assert_eq!(lines.len(), 7);
                let prose: String = collect(&p, "TextRun")
                    .into_iter()
                    .filter(|n| n["node_type"]["TextRun"]["para_index"] == 2)
                    .map(|n| n["node_type"]["TextRun"]["text"].as_str().unwrap())
                    .collect();
                assert_eq!(prose, d.sections[0].paragraphs[2].text.replace('\n', ""));
            }
        }
    }
}

#[test]
fn image_outer_margins_participate_in_occupied_envelope_not_host_line_height() {
    let mut d = rhwp::parse_document(VISIBLE).unwrap();
    let Control::Picture(pic) = &mut host(&mut d).controls[0] else {
        panic!()
    };
    pic.common.margin.top = 100;
    pic.common.margin.bottom = 200;
    let p = page(&encode(&d), 96.);
    let images = collect(&p, "Image");
    // Envelope grows300HU: center shifts-150HU, image additionally moves+100HU.
    near(num(images[0], "y"), (11894. - 50.) / 75.);
    near(num(images[0], "height"), 13562. / 75.);
    let lines = collect(&p, "TextLine");
    let empty = lines.iter().find(|l| num(l, "width") == 0.).unwrap();
    near(num(empty, "y"), (11355. + 126.) / 75.);
    near(num(empty, "height"), 1200. / 75.);
}

#[test]
fn host_line_still_occupies_space_when_picture_is_shorter() {
    let mut d = rhwp::parse_document(VISIBLE).unwrap();
    let Control::Picture(pic) = &mut host(&mut d).controls[0] else {
        panic!()
    };
    pic.common.height = 100;
    pic.common.vertical_offset = 0;
    let p = page(&encode(&d), 96.);
    // Real host occupies1200+720HU even though the picture occupies only100HU.
    let origin = (11355. + (14377. - 1920.) / 2.) / 75.;
    near(num(collect(&p, "Image")[0], "y"), origin);
    let lines = collect(&p, "TextLine");
    let empty = lines.iter().find(|l| num(l, "width") == 0.).unwrap();
    near(num(empty, "y"), origin);
    near(num(empty, "height"), 1200. / 75.);
}

#[test]
fn unqualified_wrapping_reference_and_width_are_not_silently_treated_as_tac() {
    use rhwp::model::shape::VertRelTo;
    for variant in 0..5 {
        let mut d = rhwp::parse_document(VISIBLE).unwrap();
        let Control::Picture(pic) = &mut host(&mut d).controls[0] else {
            panic!()
        };
        match variant {
            0 => pic.common.text_wrap = TextWrap::Square,
            1 => pic.common.vert_rel_to = VertRelTo::Page,
            2 => pic.common.horizontal_offset = 20000,
            3 => pic.common.vertical_offset = u32::MAX,
            _ => pic.common.allow_overlap = true,
        }
        assert!(open(&encode(&d), 96.)
            .err()
            .unwrap()
            .to_string()
            .contains("stored cell picture exclusion"));
    }
}

#[test]
fn mixed_text_or_multiple_host_lines_require_real_composition() {
    for multiline in [false, true] {
        let mut d = rhwp::parse_document(VISIBLE).unwrap();
        let h = host(&mut d);
        if multiline {
            h.line_segs.push(h.line_segs[0].clone());
        } else {
            h.text = "HOST".into();
        }
        assert!(open(&encode(&d), 96.).is_err());
    }
}

#[test]
fn fit_reserves_whole_picture_envelope_before_emitting_host_or_image() {
    use rhwp::renderer::{
        render_tree::PageRenderTree,
        style_resolver::resolve_styles,
        table_v2::{CellEndPolicy, PageArea, PreparedTextTable, Rect, TextFragmentFit},
    };
    let d = rhwp::parse_document(VISIBLE).unwrap();
    let Control::Table(t) = d.sections[0].paragraphs[1]
        .controls
        .iter()
        .find(|c| matches!(c, Control::Table(_)))
        .unwrap()
    else {
        panic!()
    };
    let mut t = t.clone();
    // Synthetic fit boundary: remove only the declared empty tail of the cell.
    // The independent picture envelope is263+13562HU, not the1200HU host.
    t.cells[0].height = 0;
    t.common.height = 0;
    let s = resolve_styles(&d.doc_info, 96.);
    let prepared = PreparedTextTable::prepare_with_end_policy(
        &t,
        &s,
        96.,
        &d.bin_data_content,
        CellEndPolicy::OmitFinalParagraphGap,
    )
    .unwrap();
    let area = |height| PageArea {
        bounds: Rect {
            x: 20.,
            y: 30.,
            width: 500.,
            height,
        },
    };
    let cursor = prepared.start();
    assert!(matches!(
        cursor.fit(area(13824. / 75.)).unwrap(),
        TextFragmentFit::DoesNotFit { .. }
    ));
    let TextFragmentFit::Placed(fragment) = cursor.fit(area(13825. / 75.)).unwrap() else {
        panic!()
    };
    near(fragment.geometry().reserved_height(), 13825. / 75.);
    let mut tree = PageRenderTree::new(0, 600., 600.);
    fragment.append_to(&mut tree).unwrap();
    let p = serde_json::json!({"render_tree":tree});
    let images = collect(&p, "Image");
    assert_eq!(images.len(), 1);
    near(num(images[0], "y"), 30. + 263. / 75.);
    near(
        num(images[0], "y") + num(images[0], "height"),
        30. + 13825. / 75.,
    );
    assert_eq!(collect(&p, "TextLine").len(), 1);
    assert!(matches!(
        fragment.continuation().fit(area(500.)).unwrap(),
        TextFragmentFit::Complete
    ));

    // A following authored blank paragraph must start below the entire image,
    // not below the shorter zero-width host; it must survive continuation.
    let mut after = t.cells[0].paragraphs[0].clone();
    after.controls.clear();
    after.char_count = 1;
    after.line_segs[0].segment_width = 18475;
    after.line_segs[0].vertical_pos = 13825;
    t.cells[0].paragraphs.push(after);
    let prepared = PreparedTextTable::prepare_with_end_policy(
        &t,
        &s,
        96.,
        &d.bin_data_content,
        CellEndPolicy::OmitFinalParagraphGap,
    )
    .unwrap();
    let TextFragmentFit::Placed(first) = prepared.start().fit(area(13825. / 75.)).unwrap() else {
        panic!()
    };
    let TextFragmentFit::Placed(second) = first.continuation().fit(area(1200. / 75.)).unwrap()
    else {
        panic!()
    };
    let mut tree = PageRenderTree::new(1, 600., 600.);
    second.append_to(&mut tree).unwrap();
    let p = serde_json::json!({"render_tree":tree});
    assert!(collect(&p, "Image").is_empty());
    let lines = collect(&p, "TextLine");
    assert_eq!(lines.len(), 1);
    near(num(lines[0], "y"), 30.);
    near(num(lines[0], "height"), 1200. / 75.);
    assert!(matches!(
        second.continuation().fit(area(500.)).unwrap(),
        TextFragmentFit::Complete
    ));
}
