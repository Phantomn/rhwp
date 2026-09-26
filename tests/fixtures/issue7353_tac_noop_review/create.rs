//! Author a readable nested TAC control, then let Hancom generate saved rows.
use rhwp::model::{control::Control, style::Alignment, table::TablePageBreak};

fn main() {
    let mut d = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue7353_stored_anchor_review/anchor-review-saved.hwp")
            .unwrap(),
    )
    .unwrap();
    let template = d.sections[0].paragraphs[0]
        .controls
        .iter()
        .find_map(|c| {
            if let Control::Table(t) = c {
                Some(t.as_ref().clone())
            } else {
                None
            }
        })
        .unwrap();
    let mut shape =
        d.doc_info.para_shapes[template.cells[0].paragraphs[0].para_shape_id as usize].clone();
    shape.raw_data = None;
    shape.border_fill_id = 1; // Existing no-paint style; preserve the reference in the save.
    shape.alignment = Alignment::Center;
    shape.indent = 0;
    shape.margin_left = 0;
    shape.margin_right = 0;
    shape.spacing_before = 0;
    shape.spacing_after = 0;
    let style = d.doc_info.para_shapes.len() as u16;
    d.doc_info.para_shapes.push(shape);
    let paragraph = |text: &str| {
        let mut p = template.cells[0].paragraphs[0].clone();
        p.para_shape_id = style;
        p.text = text.into();
        p.char_count = text.len() as u32;
        p.char_offsets = (0..p.char_count).collect();
        p.line_segs.clear();
        p.controls.clear();
        p
    };
    let mut child = template.clone();
    child.cells.truncate(1);
    child.row_count = 1;
    child.col_count = 1;
    child.row_sizes = vec![5000];
    child.cell_grid.clear();
    child.page_break = TablePageBreak::None;
    child.common.treat_as_char = true;
    child.common.width = 24000;
    child.common.height = 5000;
    child.common.horizontal_offset = 0;
    child.common.vertical_offset = 0;
    child.common.margin = Default::default();
    child.outer_margin_left = 0;
    child.outer_margin_right = 0;
    child.outer_margin_top = 0;
    child.outer_margin_bottom = 0;
    child.cells[0].width = 24000;
    child.cells[0].height = 5000;
    child.cells[0].paragraphs = vec![paragraph("INLINE CHILD TABLE")];
    let mut carrier = paragraph("");
    carrier.char_count = 9;
    carrier.controls.push(Control::Table(Box::new(child)));
    let mut outer = template.clone();
    outer.cells.truncate(1);
    outer.row_count = 1;
    outer.col_count = 1;
    outer.row_sizes = vec![18000];
    outer.cell_grid.clear();
    outer.page_break = TablePageBreak::None;
    outer.common.height = 18000;
    outer.cells[0].height = 18000;
    outer.cells[0].paragraphs = vec![paragraph("CELL BEFORE"), carrier, paragraph("CELL AFTER")];
    let host = &mut d.sections[0].paragraphs[0];
    host.text = "NO-PAINT PARAGRAPH STYLE: NESTED INLINE TABLE".into();
    host.char_count = host.text.len() as u32 + 8;
    host.char_offsets = (8..host.char_count).collect();
    host.line_segs.clear();
    *host
        .controls
        .iter_mut()
        .find(|c| matches!(c, Control::Table(_)))
        .unwrap() = Control::Table(Box::new(outer));
    let after = &mut d.sections[0].paragraphs[1];
    after.text = "AFTER PARENT TABLE".into();
    after.char_count = after.text.len() as u32;
    after.char_offsets = (0..after.char_count).collect();
    after.line_segs.clear();
    std::fs::write(
        "tests/fixtures/issue7353_tac_noop_review/noop-input.hwpx",
        rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
