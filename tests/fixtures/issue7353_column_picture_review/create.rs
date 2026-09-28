//! A normally saved control specimen, not a repaired copy of the original.
use rhwp::model::{control::Control, paragraph::Paragraph, style::Alignment};
fn clear_lines(ps: &mut [Paragraph]) {
    for p in ps {
        p.line_segs.clear();
        for c in &mut p.controls {
            if let Control::Table(t) = c {
                for cell in &mut t.cells {
                    clear_lines(&mut cell.paragraphs);
                }
            }
        }
    }
}
fn main() {
    let src = rhwp::parse_document(
        &std::fs::read("samples/issue6601/36331407_side_by_side_tac_tables.hwpx").unwrap(),
    )
    .unwrap();
    let Control::Table(parent) = &src.sections[0].paragraphs[1].controls[0] else {
        panic!()
    };
    let Control::Table(child) = &parent.cells[1].paragraphs[0].controls[1] else {
        panic!()
    };
    let mut picture = child.cells[0].paragraphs[0].clone();
    let mut d = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue7353_tac_noop_review/noop-saved.hwp").unwrap(),
    )
    .unwrap();
    d.bin_data_content = src.bin_data_content.clone();
    d.doc_info.bin_data_list = src.doc_info.bin_data_list.clone();
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
    let carrier = &mut parent.cells[0].paragraphs[1];
    let child = carrier
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
    picture.para_shape_id = d.doc_info.para_shapes.len() as u16;
    let mut style =
        d.doc_info.para_shapes[child.cells[0].paragraphs[0].para_shape_id as usize].clone();
    style.alignment = Alignment::Center;
    d.doc_info.para_shapes.push(style);
    for cs in &mut picture.char_shapes {
        cs.char_shape_id = child.cells[0].paragraphs[0].char_shapes[0].char_shape_id;
    }
    child.common.height = 10000;
    child.cells[0].height = 10000;
    child.cells[0].paragraphs = vec![picture];
    child.cell_grid.clear();
    parent.common.height = 24000;
    parent.cells[0].height = 24000;
    parent.cell_grid.clear();
    clear_lines(&mut d.sections[0].paragraphs);
    std::fs::write(
        "tests/fixtures/issue7353_column_picture_review/picture-input.hwpx",
        rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
