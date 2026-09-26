use rhwp::model::{control::Control, paragraph::Paragraph};
fn fresh(p: &mut Paragraph) {
    p.line_segs.clear();
    for c in &mut p.controls {
        if let Control::Table(t) = c {
            for cell in &mut t.cells {
                for p in &mut cell.paragraphs {
                    fresh(p);
                }
            }
        }
    }
}
fn main() {
    let original = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp")
            .unwrap(),
    )
    .unwrap();
    let Control::Table(outer) = &original.sections[0].paragraphs[5].controls[0] else {
        panic!()
    };
    let Control::Table(anchor) = &outer.cells[0].paragraphs[26].controls[0] else {
        panic!()
    };
    let mut d = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue7353_tac_space_review/space-saved.hwp").unwrap(),
    )
    .unwrap();
    let head = &mut d.sections[0].paragraphs[0];
    let Control::Table(parent) = head
        .controls
        .iter_mut()
        .find(|c| matches!(c, Control::Table(_)))
        .unwrap()
    else {
        panic!()
    };
    let host = &mut parent.cells[0].paragraphs[0];
    host.text.clear();
    host.char_offsets.clear();
    host.char_count = 9;
    let Control::Table(child) = &mut host.controls[0] else {
        panic!()
    };
    child.common = anchor.common.clone();
    child.common.height = 5000;
    child.outer_margin_left = 141;
    child.outer_margin_right = 141;
    child.outer_margin_top = 141;
    child.outer_margin_bottom = 141;
    child.cells[0].width = 44957;
    child.cells[0].paragraphs[0].text = "POSITIONED CHILD".into();
    child.cells[0].paragraphs[0].char_offsets = (0..16).collect();
    child.cells[0].paragraphs[0].char_count = 17;
    let mut after = child.cells[0].paragraphs[0].clone();
    after.text = "AFTER ANCHOR".into();
    after.char_offsets = (0..12).collect();
    after.char_count = 13;
    let mut blank = after.clone();
    blank.text.clear();
    blank.char_offsets.clear();
    blank.char_count = 1;
    parent.cells[0].paragraphs.push(blank);
    parent.cells[0].paragraphs.push(after);
    for p in &mut d.sections[0].paragraphs {
        fresh(p);
    }
    std::fs::write(
        "output/7353/r19/next-anchor/anchor-input.hwpx",
        rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
