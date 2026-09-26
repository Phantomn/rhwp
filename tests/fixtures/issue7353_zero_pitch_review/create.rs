//! Extract original #6923 title/units/blank/following-source-note. The large
//! table is deliberately omitted: its separate paint-metric rejection is not
//! covered by this paragraph-spacing comparison. Hancom regenerates all rows.
fn main() {
    let out = "output/7353/r19/stored-body-rows";
    let original = rhwp::parse_document(&std::fs::read(
        "tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp"
    ).unwrap()).unwrap();
    for (name, spacing) in [("zero", 0), ("normal", 100)] {
        let mut d = original.clone();
        d.sections[0].paragraphs = [21, 22, 23, 25].into_iter()
            .map(|i| original.sections[0].paragraphs[i].clone()).collect();
        let id = d.sections[0].paragraphs[1].para_shape_id as usize;
        d.doc_info.para_shapes[id].line_spacing = spacing;
        d.doc_info.para_shapes[id].line_spacing_v2 = spacing as u32;
        for p in &mut d.sections[0].paragraphs {
            p.line_segs.clear();
        }
        std::fs::write(format!("{out}/{name}-input.hwpx"),
            rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap()).unwrap();
    }
}
