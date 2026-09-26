//! Author a no-LineSeg input, then save in Hancom before using it as evidence.
use rhwp::model::{control::Control, style::DiagonalLine, table::TableZone};
fn main() {
    let mut d = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue7353_zone_review/zone-lines-saved.hwp").unwrap(),
    )
    .unwrap();
    let plain = d
        .doc_info
        .border_fills
        .iter()
        .find(|b| {
            b.attr == 0
                && b.fill.fill_type == rhwp::model::style::FillType::None
                && b.borders
                    .iter()
                    .all(|e| e.line_type == rhwp::model::style::BorderLineType::Solid)
        })
        .unwrap()
        .clone();
    let base = d.doc_info.border_fills.len() as u16 + 1;
    for (attr, pen, color) in [
        (0, 0, 0),
        (8, 1, 0x0000ff),
        (64, 1, 0xff0000),
        (72, 1, 0x008000),
        (8, 0, 0),
        (0, 1, 0),
    ] {
        let mut b = plain.clone();
        b.raw_data = None;
        b.attr = attr;
        b.diagonal = DiagonalLine {
            color,
            diagonal_type: pen,
            width: 7,
        };
        d.doc_info.border_fills.push(b);
    }
    let host = &mut d.sections[0].paragraphs[0];
    host.text = "DIAGONAL: red / blue / green / none".into();
    host.char_count = host.text.len() as u32 + 8;
    host.char_offsets = (8..host.char_count).collect();
    host.line_segs.clear();
    let t = host
        .controls
        .iter_mut()
        .find_map(|c| match c {
            Control::Table(t) => Some(t),
            _ => None,
        })
        .unwrap();
    for c in &mut t.cells {
        c.border_fill_id = if c.col == 0 && c.row < 5 {
            base + c.row + 1
        } else if c.row == 23 {
            base + 1
        } else {
            base
        };
        for p in &mut c.paragraphs {
            p.line_segs.clear();
        }
    }
    t.zones = vec![TableZone {
        start_row: 17,
        start_col: 0,
        end_row: 20,
        end_col: 1,
        border_fill_id: base + 1,
    }];
    let after = &mut d.sections[0].paragraphs[1];
    after.text = "AFTER DIAGONAL TABLE".into();
    after.char_count = after.text.len() as u32;
    after.char_offsets = (0..after.char_count).collect();
    after.line_segs.clear();
    std::fs::create_dir_all("output/7353/r19/diagonal").unwrap();
    std::fs::write(
        "output/7353/r19/diagonal/diagonal-input.hwpx",
        rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
    let t = d.sections[0].paragraphs[0]
        .controls
        .iter_mut()
        .find_map(|c| match c {
            Control::Table(t) => Some(t),
            _ => None,
        })
        .unwrap();
    t.row_count = 1;
    t.col_count = 1;
    t.row_sizes = vec![1];
    t.cell_grid.clear();
    t.zones.clear();
    t.page_break = rhwp::model::table::TablePageBreak::CellBreak;
    t.common.height = 2326;
    t.cells.truncate(1);
    t.cells[0].width = 32000;
    let proto = t.cells[0].paragraphs[0].clone();
    t.cells[0].paragraphs = (1..=40)
        .map(|i| {
            let mut p = proto.clone();
            p.text = format!("CELL LINE {i:02}");
            p.char_count = p.text.len() as u32;
            p.char_offsets = (0..p.char_count).collect();
            p.line_segs.clear();
            p
        })
        .collect();
    std::fs::write(
        "output/7353/r19/diagonal/diagonal-cell-input.hwpx",
        rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
