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
    let Control::Table(child) = &parent.cells[0].paragraphs[37].controls[0] else {
        panic!()
    };
    let mut t = child.clone();
    // Isolate the tab paragraph, not the unrelated merged-cell/zone surface.
    let mut first = t.cells[0].clone();
    first.row = 0;
    first.col = 0;
    first.row_span = 1;
    first.col_span = 1;
    first.height = 3000;
    let mut second = first.clone();
    second.row = 1;
    let p = &mut second.paragraphs[0];
    p.text = "LEFT\tRIGHT".into();
    p.char_offsets = vec![0, 1, 2, 3, 4, 12, 13, 14, 15, 16];
    p.char_count = 18;
    p.line_segs.clear();
    t.row_count = 2;
    t.col_count = 1;
    t.common.width = first.width;
    t.common.height = 6000;
    t.row_sizes = vec![1, 1];
    t.cell_grid.clear();
    t.cells = vec![first, second];
    t.zones.clear();
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
        "output/7353/r19/tabs/tab-input.hwpx",
        rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
