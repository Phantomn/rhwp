//! A blank picture frame followed by a visible picture in one nested-cell row.
//! Hancom normal-save recalculates line metrics; source is not the masked original.
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
        &std::fs::read("tests/fixtures/issue7353_picture_space_review/picture-saved.hwp").unwrap(),
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
    let pic = child.cells[0].paragraphs[0]
        .controls
        .iter_mut()
        .find_map(|c| {
            if let Control::Picture(pic) = c {
                Some(pic)
            } else {
                None
            }
        })
        .unwrap();
    pic.image_attr.bin_data_id = 0;
    // Group-local source offsets do not place an ungrouped empty TAC frame.
    pic.shape_attr.offset_x = 3745;
    pic.shape_attr.offset_y = -1509;
    clear(&mut d.sections[0].paragraphs);
    std::fs::write(
        "tests/fixtures/issue7353_missing_picture_review/picture-input.hwpx",
        rhwp::serializer::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
