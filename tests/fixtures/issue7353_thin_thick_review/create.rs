use rhwp::model::{
    control::Control,
    paragraph::Paragraph,
    style::{BorderFill, BorderLine, BorderLineType},
};
fn clear(ps: &mut [Paragraph]) {
    for p in ps {
        p.line_segs.clear();
        for c in &mut p.controls {
            if let Control::Table(t) = c {
                for cell in &mut t.cells {
                    clear(&mut cell.paragraphs);
                }
            }
        }
    }
}
fn main() {
    let mut d = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue7353_tac_noop_review/noop-saved.hwp").unwrap(),
    )
    .unwrap();
    let catalog = std::env::args().any(|arg| arg == "--catalog");
    let specs: Vec<_> = if catalog {
        (0..16)
            .map(|width| (3, width))
            .chain([(2, 9), (0, 9), (1, 9)])
            .collect()
    } else {
        vec![
            (3, 0),
            (3, 3),
            (3, 7),
            (3, 9),
            (3, 11),
            (2, 9),
            (0, 9),
            (1, 9),
        ]
    };
    let height = if catalog { 1200 } else { 2600 };
    let mut ids = Vec::new();
    for &(side, width) in &specs {
        let mut edges = [BorderLine {
            line_type: BorderLineType::None,
            ..Default::default()
        }; 4];
        edges[side] = BorderLine {
            line_type: BorderLineType::ThinThickDouble,
            width,
            color: 0,
        };
        d.doc_info.border_fills.push(BorderFill {
            borders: edges,
            ..Default::default()
        });
        ids.push(d.doc_info.border_fills.len() as u16);
    }
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
    let carrier = parent.cells[0].paragraphs[1].clone();
    let before = parent.cells[0].paragraphs[0].clone();
    let after = parent.cells[0].paragraphs.last().unwrap().clone();
    let mut ps = vec![before];
    for ((side, width), id) in specs.into_iter().zip(ids) {
        let mut p = carrier.clone();
        let t = p
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
        t.common.width = 24000;
        t.common.height = height;
        t.border_fill_id = id;
        t.cell_grid.clear();
        t.row_sizes = vec![height as i16];
        t.cells[0].width = 24000;
        t.cells[0].height = height;
        t.cells[0].border_fill_id = id;
        let text = &mut t.cells[0].paragraphs[0];
        text.text = format!("SIDE {side} / WIDTH {width}");
        text.char_count = text.text.len() as u32 + 1;
        text.char_offsets = (0..text.text.len() as u32).collect();
        text.controls.clear();
        ps.push(p);
    }
    ps.push(after);
    parent.cells[0].paragraphs = ps;
    parent.common.height = 32000;
    parent.cells[0].height = 32000;
    parent.row_sizes = vec![32000];
    parent.cell_grid.clear();
    clear(&mut d.sections[0].paragraphs);
    std::fs::write(
        if catalog {
            "tests/fixtures/issue7353_thin_thick_review/catalog-input.hwpx"
        } else {
            "tests/fixtures/issue7353_thin_thick_review/borders-input.hwpx"
        },
        rhwp::serializer::serialize_hwpx(&d).unwrap(),
    )
    .unwrap();
}
