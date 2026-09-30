//! Author an automatic field; Hancom, not this generator, creates saved rows.
use rhwp::model::{
    control::{AutoNumber, AutoNumberType, Control},
    paragraph::{CharShapeRef, Paragraph},
    shape::ShapeObject,
};
fn main() {
    let mut d = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue7353_host_master_review/saved.hwp").unwrap(),
    )
    .unwrap();
    // Authoring input has no cached body rows, just as the parent fixture's
    // generator. Force normal document layout instead of retaining old caches.
    for p in &mut d.sections[0].paragraphs {
        p.line_segs.clear();
        p.controls.retain(|c| !matches!(c, Control::SectionDef(_)));
    }
    for master in &mut d.sections[0].section_def.master_pages {
        let mut field = master.paragraphs[0].controls[0].clone();
        let Control::Shape(s) = &mut field else {
            panic!()
        };
        let ShapeObject::Rectangle(r) = s.as_mut() else {
            panic!()
        };
        r.common.vertical_offset = 31000;
        r.drawing.text_box.as_mut().unwrap().paragraphs = vec![Paragraph {
            text: " ".into(),
            char_count: 9,
            char_offsets: vec![0],
            char_shapes: vec![CharShapeRef {
                start_pos: 0,
                char_shape_id: 0,
            }],
            controls: vec![Control::AutoNumber(AutoNumber {
                number_type: AutoNumberType::Page,
                ..Default::default()
            })],
            ..Default::default()
        }];
        master.paragraphs[0].controls.push(field);
        master.paragraphs[0].char_count += 8;
        master.paragraphs[0].line_segs.clear();
    }
    std::fs::write(
        "tests/fixtures/issue7353_host_master_field_review/input.hwpx",
        rhwp::serializer::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
