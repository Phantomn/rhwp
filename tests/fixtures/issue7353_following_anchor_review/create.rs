use rhwp::model::{control::Control, paragraph::Paragraph, Padding};
fn plain(mut p: Paragraph, text: &str) -> Paragraph {
    p.text = text.into();
    p.char_offsets = (0..text.chars().count() as u32).collect();
    p.char_count = text.encode_utf16().count() as u32 + 1;
    p.controls.clear();
    p.ctrl_data_records.clear();
    p.line_segs.clear();
    p
}
fn main() {
    let mut d = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp")
            .unwrap(),
    )
    .unwrap();
    let Control::Table(source) = &d.sections[0].paragraphs[5].controls[0] else {
        panic!()
    };
    let mut parent = source.clone();
    let cell = &source.cells[0];
    let mut host = cell.paragraphs[37].clone();
    host.line_segs.clear();
    let Control::Table(child) = &mut host.controls[0] else {
        panic!()
    };
    child.row_count = 1;
    child.col_count = 1;
    child.cells.truncate(1);
    child.zones.clear();
    child.common.height = 5000;
    child.cells[0].row_span = 1;
    child.cells[0].col_span = 1;
    child.cells[0].width = child.common.width;
    child.cells[0].height = 5000;
    child.cells[0].paragraphs = vec![plain(cell.paragraphs[38].clone(), "POSITIONED CHILD")];
    parent.common.treat_as_char = true;
    parent.common.height = 0;
    parent.common.horizontal_offset = 0;
    parent.common.vertical_offset = 0;
    parent.common.margin = Padding::default();
    parent.outer_margin_left = 0;
    parent.outer_margin_right = 0;
    parent.outer_margin_top = 0;
    parent.outer_margin_bottom = 0;
    parent.cells[0].height = 0;
    parent.cells[0].paragraphs = vec![
        plain(cell.paragraphs[38].clone(), "BEFORE ANCHOR"),
        host,
        plain(cell.paragraphs[38].clone(), ""),
        plain(cell.paragraphs[38].clone(), "AFTER HOST"),
    ];
    let mut top = d.sections[0].paragraphs[0].clone();
    top.text.clear();
    top.char_offsets.clear();
    top.char_count = 9;
    top.line_segs.clear();
    top.controls.retain(|c| !matches!(c, Control::Table(_)));
    top.controls.push(Control::Table(parent));
    let mut style = d.doc_info.para_shapes[top.para_shape_id as usize].clone();
    style.line_spacing = 100;
    style.line_spacing_v2 = 100;
    style.indent = 0;
    top.para_shape_id = d.doc_info.para_shapes.len() as u16;
    d.doc_info.para_shapes.push(style);
    let mut after = plain(cell.paragraphs[38].clone(), "AFTER CELL");
    after.para_shape_id = top.para_shape_id;
    d.sections.truncate(1);
    d.sections[0].paragraphs = vec![top, after];
    std::fs::write(
        "output/7353/r19/next-anchor2/anchor-input.hwpx",
        rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
    let Control::Table(parent) = d.sections[0].paragraphs[0]
        .controls
        .iter_mut()
        .find(|c| matches!(c, Control::Table(_)))
        .unwrap()
    else {
        unreachable!()
    };
    let p = &mut parent.cells[0].paragraphs[1];
    p.text = "HOST TEXT".into();
    p.char_offsets = (8..17).collect();
    p.char_count = 18;
    std::fs::write(
        "output/7353/r19/next-anchor2/visible-input.hwpx",
        rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
