//! Run at repository root; output is diagnostic HWPX, never a hand-edited saved HWP.
use rhwp::model::{
    control::Control,
    paragraph::Paragraph,
    style::{BorderLine, BorderLineType},
};

fn fresh(p: &mut Paragraph) {
    p.line_segs.clear();
    for c in &mut p.controls {
        if let Control::Table(t) = c {
            for c in &mut t.cells {
                for p in &mut c.paragraphs {
                    fresh(p);
                }
            }
        }
    }
}
fn main() {
    let out = "output/7353/r19/decoration-priority";
    std::fs::create_dir_all(out).unwrap();
    let mut d = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue7353_double_review/grid-7-saved.hwp").unwrap(),
    )
    .unwrap();
    let Control::Table(parent) = d.sections[0].paragraphs[0]
        .controls
        .iter_mut()
        .find(|c| matches!(c, Control::Table(_)))
        .unwrap()
    else {
        panic!()
    };
    let Control::Table(child) = &mut parent.cells[0].paragraphs[0].controls[0] else {
        panic!()
    };
    // Clone ONLY child cell fills. Changing all document fills also modifies
    // paragraph/page decorations and is not this experiment.
    for cell in &mut child.cells {
        let mut b = d.doc_info.border_fills[cell.border_fill_id as usize - 1].clone();
        b.borders = [BorderLine {
            line_type: BorderLineType::Solid,
            width: 1,
            color: 0,
        }; 4];
        b.raw_data = None;
        d.doc_info.border_fills.push(b);
        cell.border_fill_id = d.doc_info.border_fills.len() as u16;
    }
    let mut red = d.doc_info.border_fills[child.border_fill_id as usize - 1].clone();
    red.borders = [BorderLine {
        line_type: BorderLineType::Solid,
        width: 11,
        color: 255,
    }; 4];
    red.raw_data = None;
    d.doc_info.border_fills.push(red);
    child.border_fill_id = d.doc_info.border_fills.len() as u16;
    for (index, side) in [(0, 2), (3, 3)] {
        let mut b = d.doc_info.border_fills[child.cells[index].border_fill_id as usize - 1].clone();
        b.borders[side].line_type = BorderLineType::None;
        b.raw_data = None;
        d.doc_info.border_fills.push(b);
        child.cells[index].border_fill_id = d.doc_info.border_fills.len() as u16;
    }
    for p in &mut d.sections[0].paragraphs {
        fresh(p);
    }
    std::fs::write(
        format!("{out}/outline-clean-input.hwpx"),
        rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();

    let mut d = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue7353_zone_review/zone-lines-saved.hwp").unwrap(),
    )
    .unwrap();
    let Control::Table(t) = d.sections[0]
        .paragraphs
        .iter_mut()
        .flat_map(|p| p.controls.iter_mut())
        .find(|c| matches!(c, Control::Table(_)))
        .unwrap()
    else {
        panic!()
    };
    let mut inner = t.zones[0].clone();
    inner.start_row = 3;
    inner.end_row = 6;
    inner.end_col = 1;
    t.zones.push(inner);
    for p in &mut d.sections[0].paragraphs {
        fresh(p);
    }
    std::fs::write(
        format!("{out}/zones-input.hwpx"),
        rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
