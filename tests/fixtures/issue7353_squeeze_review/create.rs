use rhwp::model::control::Control;
fn main() {
    let mut d = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue7353_tac_noop_review/noop-saved.hwp").unwrap(),
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
    let carrier = &mut parent.cells[0].paragraphs[1];
    carrier.line_segs.clear();
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
    child.common.width = 10000;
    child.cell_grid.clear();
    let cell = &mut child.cells[0];
    cell.width = 10000;
    cell.line_wrap = 1;
    let p = &mut cell.paragraphs[0];
    p.text = "ONE TWO THREE FOUR FIVE".into();
    p.char_count = p.text.len() as u32 + 1;
    p.char_offsets = (0..p.text.len() as u32).collect();
    p.line_segs.clear();
    std::fs::write(
        "output/7353/r19/squeeze/squeeze-input.hwpx",
        rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
