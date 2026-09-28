use rhwp::model::{control::Control, page::ColumnDef, paragraph::Paragraph};
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
        &std::fs::read("tests/fixtures/issue7353_squeeze_review/squeeze-saved.hwp").unwrap(),
    )
    .unwrap();
    let parent = d.sections[0].paragraphs[0]
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
    let child = parent.cells[0].paragraphs[1]
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
    let p = &mut child.cells[0].paragraphs[0];
    assert!(p.controls.is_empty());
    p.controls.push(Control::ColumnDef(ColumnDef {
        column_count: 1,
        same_width: true,
        ..Default::default()
    }));
    p.char_count += 8;
    for pos in &mut p.char_offsets {
        *pos += 8;
    }
    clear(&mut d.sections[0].paragraphs);
    std::fs::write(
        "tests/fixtures/issue7353_column_squeeze_review/squeeze-input.hwpx",
        rhwp::serializer::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
