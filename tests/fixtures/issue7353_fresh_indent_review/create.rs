//! Authored input with no LineSeg. Hancom independently composes and prints it.
use rhwp::model::{control::Control, style::Alignment};
fn main() {
    let mut d = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue7353_indent_review/indent-input.hwpx").unwrap(),
    )
    .unwrap();
    let host = &mut d.sections[0].paragraphs[0];
    host.text = "REFLOW INDENTATION: explicit line breaks".into();
    host.char_count = host.text.len() as u32 + 8;
    host.char_offsets = (8..host.char_count).collect();
    host.line_segs.clear();
    let table = host
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
    // Fresh body anchoring currently admits a zero-offset paragraph-relative
    // object only. Author that case; do not patch any Hancom saved line cache.
    table.common.vertical_offset = 0;
    table.common.horizontal_offset = 0;
    table.common.margin = Default::default();
    table.outer_margin_left = 0;
    table.outer_margin_right = 0;
    table.outer_margin_top = 0;
    table.outer_margin_bottom = 0;
    // Keep a declared cell minimum to separate horizontal indentation from the
    // known terminal-spacing and font-dependent automatic row-height questions.
    table.common.height = 28000;
    table.cells[0].height = 28000;
    for (p, text) in table.cells[0].paragraphs.iter_mut().zip([
        "FIRST LEFT\nSecond line left\nThird line left",
        "HANGING LEFT\nSecond line inset\nThird line inset",
        "FIRST CENTER\nSecond line centered\nThird line centered",
        "HANGING RIGHT\nSecond line right\nThird line right",
    ]) {
        p.text = text.into();
        p.char_count = text.len() as u32;
        p.char_offsets = (0..p.char_count).collect();
        p.line_segs.clear();
    }
    // An editor-authored empty paragraph is a real line, not zero space.
    let mut blank = table.cells[0].paragraphs[0].clone();
    let mut style = d.doc_info.para_shapes[blank.para_shape_id as usize].clone();
    style.indent = 3000;
    style.alignment = Alignment::Left;
    style.spacing_after = 0;
    blank.para_shape_id = d.doc_info.para_shapes.len() as u16;
    d.doc_info.para_shapes.push(style);
    blank.text.clear();
    blank.char_count = 0;
    blank.char_offsets.clear();
    table.cells[0].paragraphs.insert(1, blank);
    let after = &mut d.sections[0].paragraphs[1];
    after.text = "AFTER REFLOW TABLE".into();
    after.char_count = after.text.len() as u32;
    after.char_offsets = (0..after.char_count).collect();
    after.line_segs.clear();
    std::fs::write(
        "tests/fixtures/issue7353_fresh_indent_review/fresh-input.hwpx",
        rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
