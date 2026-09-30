// Controlled input, with no handwritten LineSeg. Hancom must save this input
// and print that same saved HWP before it serves as an independent oracle.
use rhwp::model::{
    control::Control,
    page::{ColumnDef, ColumnType, PageDef},
    paragraph::{CharShapeRef, Paragraph},
    shape::{CommonObjAttr, HorzAlign, HorzRelTo, TextWrap, VertAlign, VertRelTo},
    style::{Alignment, BorderFill, BorderLine, BorderLineType, LineSpacingType, ParaShape},
    table::{Cell, Table, TablePageBreak},
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

fn table(label: &str, bottom: bool, border: u16) -> Control {
    Control::Table(Box::new(Table {
        common: CommonObjAttr {
            width: 9000,
            height: 2400,
            treat_as_char: false,
            text_wrap: TextWrap::TopAndBottom,
            vert_rel_to: VertRelTo::Paper,
            vert_align: if bottom {
                VertAlign::Bottom
            } else {
                VertAlign::Top
            },
            vertical_offset: 2400,
            horz_rel_to: if bottom {
                HorzRelTo::Column
            } else {
                HorzRelTo::Paper
            },
            horz_align: if bottom {
                HorzAlign::Right
            } else {
                HorzAlign::Center
            },
            ..Default::default()
        },
        row_count: 1,
        col_count: 1,
        page_break: TablePageBreak::None,
        cells: vec![Cell {
            width: 9000,
            height: 2400,
            row_span: 1,
            col_span: 1,
            paragraphs: vec![paragraph(label)],
            border_fill_id: border,
            ..Default::default()
        }],
        ..Default::default()
    }))
}

fn main() {
    let mut doc = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue7353_picture_space_review/picture-saved.hwp").unwrap(),
    )
    .unwrap();
    doc.doc_info.para_shapes = vec![ParaShape {
        alignment: Alignment::Left,
        line_spacing_type: LineSpacingType::Fixed,
        line_spacing: 3600,
        ..Default::default()
    }];
    doc.doc_info.char_shapes[0].base_size = 1200;
    doc.doc_info.border_fills.push(BorderFill {
        borders: [BorderLine {
            line_type: BorderLineType::Solid,
            width: 2,
            color: 0,
        }; 4],
        ..Default::default()
    });
    let table_border = doc.doc_info.border_fills.len() as u16;
    let sec = &mut doc.sections[0];
    sec.section_def = Default::default();
    sec.section_def.page_def = PageDef {
        width: 36000,
        height: 40000,
        margin_left: 2400,
        margin_right: 3600,
        margin_top: 8000,
        margin_bottom: 18000,
        margin_header: 0,
        margin_footer: 0,
        ..Default::default()
    };
    let mut first = paragraph("BEFORE");
    first.controls = vec![
        Control::ColumnDef(ColumnDef {
            column_type: ColumnType::Normal,
            column_count: 2,
            same_width: true,
            spacing: 1200,
            ..Default::default()
        }),
        table("PAPER CENTER", false, table_border),
    ];
    let mut footer = paragraph("FOOTER OWNER");
    footer
        .controls
        .push(table("COLUMN RIGHT", true, table_border));
    sec.paragraphs = vec![
        first,
        paragraph("LINE 01\nLINE 02\n\nLINE 04\nLINE 05\nLINE 06\nLINE 07\nLINE 08\nLINE 09"),
        footer,
        paragraph("LINE 11\nLINE 12\nLINE 13\nLINE 14\nLINE 15\nLINE 16\nLINE 17\nLINE 18"),
        paragraph("AFTER"),
    ];
    std::fs::write(
        "tests/fixtures/issue7353_host_absolute_review/input.hwpx",
        rhwp::serializer::serialize_hwpx(&doc).unwrap(),
    )
    .unwrap();
}
