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
    let mut d = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue7353_tac_space_review/space-saved.hwp").unwrap(),
    )
    .unwrap();
    let mut cs = d.doc_info.char_shapes[0].clone();
    cs.base_size = 1000;
    cs.raw_data = None;
    let cs_id = d.doc_info.char_shapes.len() as u32;
    d.doc_info.char_shapes.push(cs);
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
    let Control::Table(child) = &mut host.controls[0] else {
        panic!()
    };
    child.col_count = 3;
    child.common.width = 27303;
    let cell = child.cells[0].clone();
    child.cells.clear();
    child.padding.left = 510;
    child.padding.right = 510;
    for (i, (width, text)) in [
        (6000, "LEFT"),
        (1303, "A"),
        (20000, "RIGHT - narrow middle cell"),
    ]
    .into_iter()
    .enumerate()
    {
        let mut c = cell.clone();
        c.width = width;
        c.col = i as u16;
        c.apply_inner_margin = false;
        let p = &mut c.paragraphs[0];
        p.text = text.into();
        p.char_offsets = (0..text.len() as u32).collect();
        p.char_count = text.len() as u32 + 1;
        p.char_shapes = vec![rhwp::model::paragraph::CharShapeRef {
            start_pos: 0,
            char_shape_id: cs_id,
        }];
        child.cells.push(c);
    }
    for p in &mut d.sections[0].paragraphs {
        fresh(p);
    }
    std::fs::create_dir_all("output/7353/r19/narrow-cell/label").unwrap();
    std::fs::write(
        "output/7353/r19/narrow-cell/label/narrow-input.hwpx",
        rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
