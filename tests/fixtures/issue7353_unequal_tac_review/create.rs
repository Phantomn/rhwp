// Controlled review derived from an existing normal Hancom fixture. Original
// #6601 remains untouched. Retain valid fonts/styles/page metadata; alter only
// the nested carrier's two source table boxes and labels before Hancom saves it.
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
    carrier.controls.clear();
    carrier.text.clear();
    carrier.char_offsets.clear();
    carrier.char_count = 17;
    carrier.line_segs.clear();
    for (height, label) in [(5000, "SHORT TABLE"), (8000, "TALL TABLE")] {
        let mut t = original.clone();
        t.common.width = 10000;
        t.common.height = height;
        t.row_sizes = vec![height as i16];
        t.cell_grid.clear();
        t.cells[0].width = 10000;
        t.cells[0].height = height;
        t.outer_margin_left = 200;
        t.outer_margin_right = 200;
        t.outer_margin_top = 200;
        t.outer_margin_bottom = 200;
        t.common.margin.left = 200;
        t.common.margin.right = 200;
        t.common.margin.top = 200;
        t.common.margin.bottom = 200;
        let p = &mut t.cells[0].paragraphs[0];
        p.text = label.into();
        p.char_count = label.len() as u32 + 1;
        p.char_offsets = (0..label.len() as u32).collect();
        p.line_segs.clear();
        carrier.controls.push(Control::Table(t));
    }
    std::fs::write(
        "output/7353/r19/unequal-tac/review-input.hwpx",
        rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
