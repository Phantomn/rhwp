//! Independent normal-save specimen; does not replace the original #2470.
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
        &std::fs::read("tests/fixtures/issue7353_column_picture_review/picture-saved.hwp").unwrap(),
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
    let mut second = p
        .controls
        .iter()
        .find(|c| matches!(c, Control::Picture(_)))
        .unwrap()
        .clone();
    let Control::Picture(pic) = &mut second else {
        panic!()
    };
    pic.common.width = 4000;
    pic.common.height = 4000;
    pic.shape_attr.current_width = 4000;
    pic.shape_attr.current_height = 4000;
    // Both source pictures retain the same crop/resource. Only declared size
    // differs; one explicit tab places the second picture after the first.
    p.text = "\t".into();
    p.char_offsets = vec![p.controls.len() as u32 * 8];
    p.tab_extended = vec![[1500, 0, 0x100, 32, 32, 32, 9]];
    p.controls.push(second);
    p.char_count = p.controls.len() as u32 * 8 + 9;
    clear(&mut d.sections[0].paragraphs);
    std::fs::write(
        "tests/fixtures/issue7353_picture_space_review/picture-input.hwpx",
        rhwp::serializer::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
