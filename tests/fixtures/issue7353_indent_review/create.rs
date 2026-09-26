//! Author paragraphs without saved rows; Hancom must save the qualification input.
use rhwp::model::{control::Control, style::Alignment, table::TablePageBreak};
fn main() {
    let mut d = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue7353_stored_anchor_review/anchor-review-saved.hwp")
            .unwrap(),
    )
    .unwrap();
    let proto = match &d.sections[0].paragraphs[0]
        .controls
        .iter()
        .find(|c| matches!(c, Control::Table(_)))
        .unwrap()
    {
        Control::Table(t) => t.cells[0].paragraphs[0].clone(),
        _ => unreachable!(),
    };
    let base = d.doc_info.para_shapes[proto.para_shape_id as usize].clone();
    let mut paragraphs = Vec::new();
    for (label, indent, alignment) in [
        ("FIRST LEFT", 3000, Alignment::Left),
        ("HANGING LEFT", -3000, Alignment::Left),
        ("FIRST CENTER", 3000, Alignment::Center),
        ("HANGING RIGHT", -3000, Alignment::Right),
    ] {
        let mut s = base.clone();
        s.raw_data = None;
        s.indent = indent;
        s.margin_left = 1000;
        s.margin_right = 1000;
        s.alignment = alignment;
        s.spacing_before = 0;
        // The final paragraph has no after-gap. --terminal-gap preserves the
        // separate known cell-terminal-spacing diagnostic instead.
        s.spacing_after =
            if label == "HANGING RIGHT" && !std::env::args().any(|a| a == "--terminal-gap") {
                0
            } else {
                800
            };
        s.line_spacing = 160;
        let id = d.doc_info.para_shapes.len() as u16;
        d.doc_info.para_shapes.push(s);
        let mut p = proto.clone();
        p.para_shape_id = id;
        p.text=format!("{label}: This saved paragraph keeps its original line breaks. Indentation changes the usable line box once, while the next line and the following paragraph retain their own positions.");
        p.char_count = p.text.len() as u32;
        p.char_offsets = (0..p.char_count).collect();
        p.line_segs.clear();
        paragraphs.push(p);
    }
    let host = &mut d.sections[0].paragraphs[0];
    host.text = "SAVED INDENTATION: first line and hanging".into();
    host.char_count = host.text.len() as u32 + 8;
    host.char_offsets = (8..host.char_count).collect();
    host.line_segs.clear();
    let t = host
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
    t.row_count = 1;
    t.col_count = 1;
    t.row_sizes = vec![1];
    t.cell_grid.clear();
    t.zones.clear();
    t.page_break = TablePageBreak::None;
    t.common.height = 28000;
    t.cells.truncate(1);
    t.cells[0].width = 32000;
    t.cells[0].height = 28000;
    t.cells[0].paragraphs = paragraphs;
    let after = &mut d.sections[0].paragraphs[1];
    after.text = "AFTER INDENTED TABLE".into();
    after.char_count = after.text.len() as u32;
    after.char_offsets = (0..after.char_count).collect();
    after.line_segs.clear();
    let out = "tests/fixtures/issue7353_indent_review/indent-input.hwpx";
    std::fs::write(out, rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap()).unwrap();
}
