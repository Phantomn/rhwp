use rhwp::model::{control::Control, paragraph::Paragraph, Padding};
fn main() {
    let mut d = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp")
            .unwrap(),
    )
    .unwrap();
    let Control::Table(parent) = &d.sections[0].paragraphs[5].controls[0] else {
        panic!()
    };
    let Control::Table(child) = &parent.cells[0].paragraphs[26].controls[0] else {
        panic!()
    };
    let mut t = child.clone();
    t.common.treat_as_char = true;
    t.common.horizontal_offset = 0;
    t.common.vertical_offset = 0;
    t.common.margin = Padding::default();
    t.outer_margin_left = 0;
    t.outer_margin_right = 0;
    t.outer_margin_top = 0;
    t.outer_margin_bottom = 0;
    let mut host = d.sections[0].paragraphs[0].clone();
    host.controls.retain(|c| !matches!(c, Control::Table(_)));
    host.controls.push(Control::Table(t));
    host.line_segs.clear();
    host.text.clear();
    host.char_offsets.clear();
    host.char_count = 9;
    let mut shape = d.doc_info.para_shapes[host.para_shape_id as usize].clone();
    shape.line_spacing = 100;
    shape.line_spacing_v2 = 100;
    shape.line_spacing_type = rhwp::model::style::LineSpacingType::Percent;
    shape.indent = 0;
    shape.margin_left = 0;
    shape.margin_right = 0;
    host.para_shape_id = d.doc_info.para_shapes.len() as u16;
    d.doc_info.para_shapes.push(shape);
    let mut after = Paragraph::default();
    after.text = "AFTER TABLE".into();
    after.char_count = 12;
    after.char_offsets = (0..11).collect();
    after.char_shapes = host.char_shapes.clone();
    after.para_shape_id = host.para_shape_id;
    d.sections.truncate(1);
    d.sections[0].paragraphs = vec![host, after];
    std::fs::write(
        "output/7353/r19/rowspan-height/final/rowspan-input.hwpx",
        rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
