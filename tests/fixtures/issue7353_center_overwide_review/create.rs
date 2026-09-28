use rhwp::model::{control::Control, style::Alignment};

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
    let mut style = d.doc_info.para_shapes[carrier.para_shape_id as usize].clone();
    style.alignment = Alignment::Center;
    carrier.para_shape_id = d.doc_info.para_shapes.len() as u16;
    d.doc_info.para_shapes.push(style);
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
    // Wider than the content lane, still inside the physical parent cell.
    child.common.width = 31500;
    child.common.margin.left = 100;
    child.common.margin.right = 100;
    child.outer_margin_left = 100;
    child.outer_margin_right = 100;
    child.cells[0].width = 31500;
    child.cell_grid.clear();
    for p in &mut child.cells[0].paragraphs {
        p.line_segs.clear();
    }
    std::fs::write(
        "output/7353/r19/center-overwide/contained-input.hwpx",
        rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
