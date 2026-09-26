//! A readable derivative of #6923's affected cell; Hancom owns saved rows.
use rhwp::model::{control::Control, style::Alignment};

fn main() {
    let original = rhwp::parse_document(&std::fs::read(
        "tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp",
    ).unwrap()).unwrap();
    let Control::Table(wrapper) = &original.sections[0].paragraphs[5].controls[0] else { panic!() };
    let Control::Table(child) = &wrapper.cells[0].paragraphs[4].controls[0] else { panic!() };
    let source = &child.cells[6].paragraphs[1];
    let mut d = rhwp::parse_document(&std::fs::read(
        "tests/fixtures/issue7353_tac_noop_review/noop-saved.hwp",
    ).unwrap()).unwrap();
    d.doc_info = original.doc_info.clone();
    let mut style = d.doc_info.para_shapes[source.para_shape_id as usize].clone();
    style.raw_data = None;
    style.indent = 0;
    style.alignment = Alignment::Left;
    let plain = d.doc_info.para_shapes.len() as u16;
    d.doc_info.para_shapes.push(style);
    let host = &mut d.sections[0].paragraphs[0];
    host.para_shape_id = plain;
    host.text.clear();
    host.char_count = 8;
    host.char_offsets.clear();
    host.char_shapes = vec![source.char_shapes[0].clone()];
    host.line_segs.clear();
    let Control::Table(table) = host.controls.iter_mut().find(|c| matches!(c, Control::Table(_))).unwrap() else { panic!() };
    table.common.width = 23741;
    table.common.height = 7500;
    table.common.treat_as_char = true;
    table.common.horizontal_offset = 0;
    table.common.vertical_offset = 0;
    table.common.margin = Default::default();
    table.outer_margin_left = 0;
    table.outer_margin_right = 0;
    table.outer_margin_top = 0;
    table.outer_margin_bottom = 0;
    table.row_sizes = vec![7500];
    table.cell_grid.clear();
    table.cells[0].width = 23741;
    table.cells[0].height = 7500;
    table.cells[0].paragraphs = vec![source.clone()];
    table.cells[0].paragraphs[0].line_segs.clear();
    let after = &mut d.sections[0].paragraphs[1];
    after.para_shape_id = plain;
    after.text = "AFTER CELL: no missing or duplicated text".into();
    after.char_count = after.text.len() as u32;
    after.char_offsets = (0..after.char_count).collect();
    after.char_shapes = vec![source.char_shapes[0].clone()];
    after.line_segs.clear();
    std::fs::write("tests/fixtures/issue7353_justify_review/justify-input.hwpx",
        rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap()).unwrap();
}
