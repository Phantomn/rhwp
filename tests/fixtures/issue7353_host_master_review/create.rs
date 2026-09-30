//! Controlled fixture authoring; Hancom saves/reflows every row before use.
use rhwp::model::style::ShapeBorderLine;
use rhwp::model::{
    control::Control,
    header_footer::{HeaderFooterApply, MasterPage},
    paragraph::{CharShapeRef, Paragraph},
    shape::*,
    table::VerticalAlign,
};
fn p(text: &str) -> Paragraph {
    Paragraph {
        text: text.into(),
        char_count: text.encode_utf16().count() as u32 + 1,
        char_shapes: vec![CharShapeRef {
            start_pos: 0,
            char_shape_id: 0,
        }],
        ..Default::default()
    }
}
fn shape(label: &str, x: u32, y: u32, w: u32, h: u32, inline: bool) -> Control {
    Control::Shape(Box::new(ShapeObject::Rectangle(RectangleShape {
        common: CommonObjAttr {
            width: w,
            height: h,
            treat_as_char: inline,
            horz_rel_to: if inline {
                HorzRelTo::Para
            } else {
                HorzRelTo::Paper
            },
            vert_rel_to: if inline {
                VertRelTo::Para
            } else {
                VertRelTo::Paper
            },
            horizontal_offset: x,
            vertical_offset: y,
            text_wrap: TextWrap::InFrontOfText,
            ..Default::default()
        },
        drawing: DrawingObjAttr {
            shape_attr: ShapeComponentAttr {
                original_width: w,
                original_height: h,
                current_width: w,
                current_height: h,
                render_sx: 1.,
                render_sy: 1.,
                ..Default::default()
            },
            border_line: ShapeBorderLine {
                color: 0,
                width: 50,
                attr: 1,
                ..Default::default()
            },
            text_box: Some(TextBox {
                max_width: w,
                vertical_align: VerticalAlign::Center,
                list_attr: 32,
                paragraphs: vec![p(label)],
                ..Default::default()
            }),
            ..Default::default()
        },
        x_coords: [0, w as i32, w as i32, 0],
        y_coords: [0, 0, h as i32, h as i32],
        ..Default::default()
    })))
}
fn main() {
    let mut d = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue7353_host_absolute_review/saved.hwp").unwrap(),
    )
    .unwrap();
    // Reuse only font/style resources and table objects. No handwritten rows.
    let s = &mut d.sections[0];
    for p in &mut s.paragraphs {
        p.line_segs.clear();
        p.controls.retain(|c| !matches!(c, Control::SectionDef(_)));
    }
    s.section_def.hide_master_page = true;
    s.section_def.flags = 4;
    s.section_def.master_pages = [
        (HeaderFooterApply::Both, "BASE MASTER", 2400),
        (HeaderFooterApply::Odd, "ODD MASTER", 18000),
    ]
    .into_iter()
    .map(|(apply, label, x)| {
        let mut owner = p("");
        owner
            .controls
            .push(shape(label, x, 27000, 14000, 3000, false));
        MasterPage {
            apply_to: apply,
            text_width: 30000,
            text_height: 14000,
            paragraphs: vec![owner],
            ..Default::default()
        }
    })
    .collect();
    let mut inline = p("LEFT  RIGHT");
    inline.char_count = 20;
    inline.char_offsets = vec![0, 1, 2, 3, 4, 13, 14, 15, 16, 17, 18];
    inline.controls.push(shape("BOX", 0, 0, 4500, 1600, true));
    // New paragraphs, not metadata patches to a claimed saved oracle.
    s.paragraphs.push(inline);
    s.paragraphs.push(p("TAIL 01\nTAIL 02\nTAIL 03\nTAIL 04\nTAIL 05\nTAIL 06\nTAIL 07\nTAIL 08\nTAIL 09\nTAIL 10\nTAIL 11\nTAIL 12\nTAIL 13\nTAIL 14"));
    s.paragraphs.push(p("END"));
    std::fs::write(
        "tests/fixtures/issue7353_host_master_review/input.hwpx",
        rhwp::serializer::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
