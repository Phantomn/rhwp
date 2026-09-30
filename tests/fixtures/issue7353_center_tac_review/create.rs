// Run at repository root; emits a controlled input for independent Hancom
// save/PDF. --asymmetric changes only the children's top/bottom margins.
use rhwp::model::control::Control;
fn main() {
    let mut d = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue7353_unequal_tac_review/review-saved.hwp").unwrap(),
    )
    .unwrap();
    let parent = d.sections[0].paragraphs[0]
        .controls
        .iter_mut()
        .find_map(|c| match c {
            Control::Table(t) => Some(t),
            _ => None,
        })
        .unwrap();
    let p = &mut parent.cells[0].paragraphs[1];
    let mut style = d.doc_info.para_shapes[p.para_shape_id as usize].clone();
    style.attr1 = (style.attr1 & !(3 << 20)) | (2 << 20);
    style.raw_data = None;
    p.para_shape_id = d.doc_info.para_shapes.len() as u16;
    d.doc_info.para_shapes.push(style);
    p.line_segs.clear();
    if std::env::args().any(|v| v == "--asymmetric") {
        for (c, (top, bottom)) in p.controls.iter_mut().zip([(400, 100), (100, 600)]) {
            if let Control::Table(t) = c {
                t.outer_margin_top = top;
                t.outer_margin_bottom = bottom;
                t.common.margin.top = top;
                t.common.margin.bottom = bottom;
            }
        }
    }
    let name = if std::env::args().any(|v| v == "--asymmetric") {
        "center-asymmetric-input.hwpx"
    } else {
        "center-input.hwpx"
    };
    std::fs::write(
        format!("output/7353/r19/section-scope/{name}"),
        rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
