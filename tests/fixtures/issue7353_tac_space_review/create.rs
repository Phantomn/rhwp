use rhwp::model::control::Control;
fn main() {
    let mut d = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue7353_justify_review/justify-saved.hwp").unwrap(),
    )
    .unwrap();
    let original = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp")
            .unwrap(),
    )
    .unwrap();
    let Control::Table(w) = &original.sections[0].paragraphs[5].controls[0] else {
        panic!()
    };
    let p = &w.cells[0].paragraphs[7];
    d.doc_info = original.doc_info.clone();
    let mut plain = d.doc_info.para_shapes[p.para_shape_id as usize].clone();
    plain.raw_data = None;
    plain.indent = 0;
    plain.alignment = rhwp::model::style::Alignment::Left;
    let plain_id = d.doc_info.para_shapes.len() as u16;
    d.doc_info.para_shapes.push(plain);
    d.sections[0].section_def.page_def = original.sections[0].section_def.page_def.clone();
    d.sections[0].section_def.page_border_fill.border_fill_id = 0;
    for f in &mut d.sections[0].section_def.extra_page_border_fills {
        f.border_fill_id = 0;
    }
    let head = &mut d.sections[0].paragraphs[0];
    let Control::Table(t) = head
        .controls
        .iter_mut()
        .find(|c| matches!(c, Control::Table(_)))
        .unwrap()
    else {
        panic!()
    };
    t.common.width = 48182;
    t.common.height = 15000;
    t.row_sizes = vec![15000];
    t.cells[0].width = 48182;
    t.cells[0].height = 15000;
    t.cells[0].paragraphs = vec![p.clone()];
    let mut child = t.clone();
    child.common.width = 21201;
    child.common.height = 5000;
    child.row_sizes = vec![5000];
    child.cells[0].width = 21201;
    child.cells[0].height = 5000;
    let mut label = w.cells[0].paragraphs[7].clone();
    label.controls.clear();
    label.text = "SPACE BEFORE TABLE".into();
    label.char_count = label.text.len() as u32 + 1;
    label.char_offsets = (0..label.text.len() as u32).collect();
    label.para_shape_id = plain_id;
    child.cells[0].paragraphs = vec![label];
    t.cells[0].paragraphs[0].controls = vec![Control::Table(child)];
    // Fresh authoring: keep text/control order and properties, discard saved layout.
    fn fresh(p: &mut rhwp::model::paragraph::Paragraph) {
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
    head.para_shape_id = plain_id;
    fresh(head);
    let tail = &mut d.sections[0].paragraphs[1];
    tail.para_shape_id = plain_id;
    fresh(tail);
    std::fs::write(
        "tests/fixtures/issue7353_tac_space_review/space-input.hwpx",
        rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
