use rhwp::model::{
    control::Control,
    paragraph::Paragraph,
    style::{BorderFill, BorderLine, BorderLineType},
    table::TablePageBreak,
};
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
    let Control::Table(w) = &original.sections[0].paragraphs[5].controls[0] else {
        panic!()
    };
    let mut d = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue7353_tac_space_review/space-input.hwpx").unwrap(),
    )
    .unwrap();
    // The input's styles already come from the original. No stored lines are injected.
    let mut carrier = w.cells[0].paragraphs[7].clone();
    let Control::Table(t) = d.sections[0].paragraphs[0]
        .controls
        .iter_mut()
        .find(|c| matches!(c, Control::Table(_)))
        .unwrap()
    else {
        panic!()
    };
    t.common.height = 8000;
    t.row_sizes = vec![8000];
    t.cells[0].height = 8000;
    t.cells[0].paragraphs = vec![carrier.clone()];
    for p in &mut d.sections[0].paragraphs {
        fresh(p);
    }
    std::fs::write(
        "output/7353/r19/double/title-input.hwpx",
        rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
    // Normal-authoring controls: equal double edges in a 2x2 grid, multiple widths.
    for width in [0, 3, 7, 11] {
        let id = d.doc_info.border_fills.len() as u16 + 1;
        d.doc_info.border_fills.push(BorderFill {
            borders: [BorderLine {
                line_type: BorderLineType::Double,
                width,
                color: 0,
            }; 4],
            ..Default::default()
        });
        let Control::Table(child) = &mut carrier.controls[0] else {
            panic!()
        };
        let proto = child.cells[0].clone();
        child.row_count = 2;
        child.col_count = 2;
        child.common.width = 30000;
        child.common.height = 8000;
        child.common.margin = Default::default();
        child.row_sizes = vec![4000; 2];
        child.cells.clear();
        child.cell_grid.clear();
        child.zones.clear();
        child.border_fill_id = id;
        child.page_break = TablePageBreak::None;
        child.repeat_header = false;
        for row in 0..2 {
            for col in 0..2 {
                let mut c = proto.clone();
                c.row = row;
                c.col = col;
                c.row_span = 1;
                c.col_span = 1;
                c.width = 15000;
                c.height = 4000;
                c.border_fill_id = id;
                c.is_header = false;
                let mut p = c.paragraphs[0].clone();
                p.controls.clear();
                p.text = format!("W{width} R{row} C{col}");
                p.char_count = p.text.len() as u32 + 1;
                p.char_offsets = (0..p.text.len() as u32).collect();
                c.paragraphs = vec![p];
                child.cells.push(c);
            }
        }
        let Control::Table(t) = d.sections[0].paragraphs[0]
            .controls
            .iter_mut()
            .find(|c| matches!(c, Control::Table(_)))
            .unwrap()
        else {
            panic!()
        };
        t.common.height = 16000;
        t.row_sizes = vec![16000];
        t.cells[0].height = 16000;
        t.cells[0].paragraphs = vec![carrier.clone()];
        for p in &mut d.sections[0].paragraphs {
            fresh(p);
        }
        std::fs::write(
            format!("output/7353/r19/double/grid-{width}-input.hwpx"),
            rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap(),
        )
        .unwrap();
    }
}
