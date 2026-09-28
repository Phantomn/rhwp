// Independent converter input. Never invent stored LineSegs: Hancom lays out
// the authored cell minima and contents before producing our reference PDF.
use rhwp::model::{control::Control, paragraph::Paragraph};
fn clear(ps: &mut [Paragraph]) {
    for p in ps {
        p.line_segs.clear();
        for c in &mut p.controls {
            if let Control::Table(t) = c {
                for cell in &mut t.cells {
                    clear(&mut cell.paragraphs);
                }
            }
        }
    }
}
fn main() {
    let mut d = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue7353_tac_noop_review/noop-saved.hwp").unwrap(),
    )
    .unwrap();
    let parent = d.sections[0].paragraphs[0]
        .controls
        .iter()
        .find_map(|c| {
            if let Control::Table(t) = c {
                Some(t)
            } else {
                None
            }
        })
        .unwrap();
    let mut before = parent.cells[0].paragraphs[0].clone();
    let mut carrier = parent.cells[0].paragraphs[1].clone();
    let mut after = parent.cells[0].paragraphs.last().unwrap().clone();
    let original = carrier
        .controls
        .iter()
        .find_map(|c| {
            if let Control::Table(t) = c {
                Some(t.clone())
            } else {
                None
            }
        })
        .unwrap();
    for (p, text) in [(&mut before, "BEFORE TABLES"), (&mut after, "AFTER TABLES")] {
        p.text = text.into();
        p.char_count = text.len() as u32 + 1;
        p.char_offsets = (0..text.len() as u32).collect();
        p.controls.clear();
        p.char_shapes.truncate(1);
        p.char_shapes[0].start_pos = 0;
    }
    carrier.controls.clear();
    carrier.text.clear();
    carrier.char_offsets.clear();
    carrier.char_count = 17;
    for (height, text) in [(2700, "SHORT TABLE"), (5400, "TALL TABLE")] {
        let mut t = original.clone();
        t.common.width = 15000;
        t.common.height = 11565;
        t.row_sizes = vec![height as i16];
        t.cell_grid.clear();
        t.cells[0].width = 15000;
        t.cells[0].height = height;
        t.cells[0].paragraphs.truncate(1);
        let p = &mut t.cells[0].paragraphs[0];
        p.text = text.into();
        p.char_count = text.len() as u32 + 1;
        p.char_offsets = (0..text.len() as u32).collect();
        p.controls.clear();
        p.char_shapes.truncate(1);
        p.char_shapes[0].start_pos = 0;
        carrier.controls.push(Control::Table(t));
    }
    d.sections[0].paragraphs = vec![before, carrier, after];
    clear(&mut d.sections[0].paragraphs);
    std::fs::write(
        "tests/fixtures/issue7353_fresh_tac_box_review/carrier.hwpx",
        rhwp::serializer::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
