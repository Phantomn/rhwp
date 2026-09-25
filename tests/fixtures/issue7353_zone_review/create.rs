use rhwp::model::{
    control::Control,
    style::{BorderFill, BorderLine, BorderLineType, Fill, FillType, SolidFill},
    table::{TablePageBreak, TableZone},
};
fn main() {
    let tall = std::env::args().any(|a| a == "--tall");
    let rows = if tall { 8 } else { 24 };
    let row_height = if tall { 7000 } else { 2326 };
    let name = if tall { "zone-review" } else { "zone-lines" };
    let input =
        std::fs::read("tests/fixtures/issue7353_stored_anchor_review/anchor-review-saved.hwp")
            .unwrap();
    let mut d = rhwp::parse_document(&input).unwrap();
    let none = BorderLine {
        line_type: BorderLineType::None,
        ..Default::default()
    };
    let solid = |color| BorderLine {
        line_type: BorderLineType::Solid,
        width: 7,
        color,
    };
    let fill = |color| Fill {
        fill_type: FillType::Solid,
        solid: Some(SolidFill {
            background_color: color,
            pattern_type: -1,
            ..Default::default()
        }),
        ..Default::default()
    };
    d.doc_info.border_fills = vec![
        BorderFill {
            borders: [none; 4],
            ..Default::default()
        },
        BorderFill {
            borders: [solid(0); 4],
            ..Default::default()
        },
        BorderFill {
            borders: [solid(0x0000ff); 4],
            fill: fill(0xffeeee),
            ..Default::default()
        },
        BorderFill {
            borders: [solid(0); 4],
            fill: fill(0x00ffff),
            ..Default::default()
        },
    ];
    for p in &mut d.doc_info.para_shapes {
        p.border_fill_id = 1;
    }
    let host = &mut d.sections[0].paragraphs[0];
    host.text = "ZONE: blue area, red perimeter, yellow cell".into();
    host.char_count = host.text.encode_utf16().count() as u32 + 8;
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
    let prototype = t.cells[0].clone();
    t.border_fill_id = 1;
    t.row_count = rows;
    t.col_count = 2;
    t.page_break = TablePageBreak::RowBreak;
    t.common.height = u32::from(rows) * row_height;
    t.cells.clear();
    t.row_sizes = vec![2; usize::from(rows)];
    t.repeat_header = false;
    for row in 0..rows {
        for col in 0..if row == rows - 1 { 1 } else { 2 } {
            let mut c = prototype.clone();
            c.row = row;
            c.col = col;
            c.row_span = 1;
            c.col_span = if row == rows - 1 { 2 } else { 1 };
            c.width = 16000 * u32::from(c.col_span);
            c.height = row_height;
            c.border_fill_id = if row == 2 && col == 0 { 4 } else { 2 };
            let p = &mut c.paragraphs[0];
            p.text = format!("ROW {} / COL {}", row + 1, col + 1);
            p.char_count = p.text.len() as u32;
            p.char_offsets = (0..p.char_count).collect();
            p.line_segs.clear();
            t.cells.push(c);
        }
    }
    t.cell_grid.clear();
    t.zones = vec![TableZone {
        start_row: 1,
        start_col: 0,
        end_row: rows - 1,
        end_col: 0,
        border_fill_id: 3,
    }];
    let after = &mut d.sections[0].paragraphs[1];
    after.text = "AFTER ZONE TABLE".into();
    after.char_count = after.text.len() as u32;
    after.char_offsets = (0..after.char_count).collect();
    after.line_segs.clear();
    std::fs::write(
        format!("output/7353/r19/zones/{name}-input.hwpx"),
        rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
