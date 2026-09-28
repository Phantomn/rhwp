use rhwp::model::{
    control::Control,
    paragraph::Paragraph,
    style::{BorderFill, BorderLine, BorderLineType},
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
    for (name, horizontal, sw) in [("vertical", false, 1), ("horizontal", true, 7)] {
        let mut d = rhwp::parse_document(
            &std::fs::read("tests/fixtures/issue7353_double_review/grid-7-input.hwpx").unwrap(),
        )
        .unwrap();
        let id = d.doc_info.border_fills.len() as u16 + 1;
        let solid = BorderLine {
            line_type: BorderLineType::Solid,
            width: sw,
            color: 0,
        };
        d.doc_info.border_fills.push(BorderFill {
            borders: [solid; 4],
            ..Default::default()
        });
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
        child.border_fill_id = id;
        for cell in &mut child.cells {
            let mut borders = [solid; 4];
            let side = if horizontal {
                if cell.row == 0 {
                    3
                } else {
                    2
                }
            } else if cell.col == 0 {
                1
            } else {
                0
            };
            borders[side] = BorderLine {
                line_type: BorderLineType::Double,
                width: 7,
                color: 0,
            };
            cell.border_fill_id = d.doc_info.border_fills.len() as u16 + 1;
            d.doc_info.border_fills.push(BorderFill {
                borders,
                ..Default::default()
            });
        }
        for p in &mut d.sections[0].paragraphs {
            fresh(p);
        }
        let path = format!("output/7353/r19/mixed-borders/{name}-inner-input.hwpx");
        assert!(!std::path::Path::new(&path).exists());
        std::fs::write(path, rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap()).unwrap();
    }
}
