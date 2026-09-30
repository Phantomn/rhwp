// No handwritten LineSeg: save both inputs with Hancom before inspecting them.
use rhwp::model::{
    control::Control,
    page::{ColumnDef, ColumnType, PageDef},
    paragraph::{CharShapeRef, Paragraph},
    style::{Alignment, LineSpacingType, ParaShape},
};

fn paragraph(text: &str) -> Paragraph {
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

fn main() {
    let mut doc = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue7353_host_absolute_review/saved.hwp").unwrap(),
    )
    .unwrap();
    doc.doc_info.para_shapes = vec![ParaShape {
        alignment: Alignment::Left,
        line_spacing_type: LineSpacingType::Fixed,
        line_spacing: 3600,
        ..Default::default()
    }];
    doc.doc_info.char_shapes[0].base_size = 1200;
    let sec = &mut doc.sections[0];
    sec.section_def = Default::default();
    sec.section_def.page_def = PageDef {
        width: 36000,
        height: 40000,
        margin_left: 2400,
        margin_right: 3600,
        margin_top: 8000,
        margin_bottom: 19400,
        margin_header: 0,
        margin_footer: 0,
        ..Default::default()
    };
    sec.paragraphs = vec![
        paragraph("LINE 01\nLINE 02\nLINE 03\nLINE 04\nLINE 05\nLINE 06\nLINE 07"),
        paragraph(""),
        paragraph(""),
        paragraph(""),
        paragraph("AFTER THREE EMPTY PARAGRAPHS"),
        paragraph("END"),
    ];
    for (name, enabled) in [("off", false), ("on", true)] {
        doc.sections[0].section_def.hide_empty_line = enabled;
        doc.sections[0].section_def.flags = if enabled { 1 << 19 } else { 0 };
        std::fs::write(
            format!("tests/fixtures/issue7353_hide_empty_review/{name}.hwpx"),
            rhwp::serializer::serialize_hwpx(&doc).unwrap(),
        )
        .unwrap();
    }
    doc.sections[0].paragraphs[0]
        .controls
        .push(Control::ColumnDef(ColumnDef {
            column_type: ColumnType::Normal,
            column_count: 2,
            same_width: true,
            spacing: 1200,
            ..Default::default()
        }));
    doc.sections[0].paragraphs[4] = paragraph("AFTER\nFILL 02\nFILL 03\nFILL 04\nFILL 05");
    doc.sections[0].paragraphs[5] = paragraph("FILL 06");
    doc.sections[0].paragraphs.extend([
        paragraph(""),
        paragraph(""),
        paragraph(""),
        paragraph("NEXT PAGE"),
    ]);
    for (name, enabled) in [("columns-off", false), ("columns-on", true)] {
        doc.sections[0].section_def.hide_empty_line = enabled;
        doc.sections[0].section_def.flags = if enabled { 1 << 19 } else { 0 };
        std::fs::write(
            format!("tests/fixtures/issue7353_hide_empty_review/{name}.hwpx"),
            rhwp::serializer::serialize_hwpx(&doc).unwrap(),
        )
        .unwrap();
    }
}
