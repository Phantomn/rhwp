//! Author objects without cached lines; Hancom must save/reflow the specimen.
use rhwp::model::{
    control::Control,
    page::PageDef,
    paragraph::{CharShapeRef, Paragraph},
    shape::*,
    style::{Alignment, ParaShape, ShapeBorderLine},
    Point,
};
fn p(text: &str, id: u32) -> Paragraph {
    Paragraph {
        text: text.into(),
        char_count: text.encode_utf16().count() as u32 + 1,
        char_shapes: vec![CharShapeRef {
            start_pos: 0,
            char_shape_id: id,
        }],
        ..Default::default()
    }
}
fn line(h: u32, top: i16, bottom: i16) -> Control {
    Control::Shape(Box::new(ShapeObject::Line(LineShape {
        common: CommonObjAttr {
            width: 18000,
            height: h,
            treat_as_char: true,
            margin: rhwp::model::Padding {
                top,
                bottom,
                ..Default::default()
            },
            ..Default::default()
        },
        drawing: DrawingObjAttr {
            shape_attr: ShapeComponentAttr {
                original_width: 18000,
                original_height: h,
                current_width: 18000,
                current_height: h,
                render_sx: 1.,
                render_sy: 1.,
                ..Default::default()
            },
            border_line: ShapeBorderLine {
                width: 30,
                attr: 1,
                ..Default::default()
            },
            ..Default::default()
        },
        start: Point { x: 0, y: 0 },
        end: Point {
            x: 18000,
            y: h as i32,
        },
        ..Default::default()
    })))
}
fn main() {
    let mut d = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue7353_host_absolute_review/saved.hwp").unwrap(),
    )
    .unwrap();
    d.doc_info.para_shapes = vec![ParaShape {
        alignment: Alignment::Left,
        line_spacing: 160,
        ..Default::default()
    }];
    let mut cs = d.doc_info.char_shapes[0].clone();
    cs.base_size = 1100;
    let mut large = cs.clone();
    large.base_size = 2000;
    d.doc_info.char_shapes = vec![cs, large];
    let s = &mut d.sections[0];
    s.section_def = Default::default();
    s.section_def.page_def = PageDef {
        width: 36000,
        height: 56000,
        margin_left: 2400,
        margin_right: 3600,
        margin_top: 4000,
        margin_bottom: 4000,
        margin_header: 0,
        margin_footer: 0,
        ..Default::default()
    };
    s.paragraphs = vec![p("SMALL INLINE OBJECT / 11PT", 0)];
    for (label, h, top, bottom, id) in [
        ("THIN 4", 4, 0, 0, 0),
        ("SMALL 600", 600, 0, 0, 0),
        ("TALL 2400", 2400, 0, 0, 0),
        ("SMALL + TOP 900", 600, 900, 0, 0),
        ("SMALL + BOTTOM 900", 600, 0, 900, 0),
        ("THIN / 20PT", 4, 0, 0, 1),
        ("SMALL + TOP 200", 600, 200, 0, 0),
        ("SMALL + BOTTOM 200", 600, 0, 200, 0),
    ] {
        s.paragraphs.push(p(label, 0));
        let mut q = p("", id);
        q.controls.push(line(h, top, bottom));
        q.char_count = 9;
        s.paragraphs.push(q);
        s.paragraphs.push(p("AFTER", 0));
    }
    std::fs::write(
        "tests/fixtures/issue7353_small_inline_review/input.hwpx",
        rhwp::serializer::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
