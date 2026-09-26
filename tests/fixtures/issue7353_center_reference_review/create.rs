//! Regenerate paired CENTER/BASELINE controls from the original market table.
//! Uniform borders isolate text alignment from unsupported shared-edge conflicts.
use rhwp::model::{
    control::Control,
    paragraph::Paragraph,
    style::{BorderFill, BorderLine, BorderLineType},
};

fn clear_rows(p: &mut Paragraph) {
    p.line_segs.clear();
    for control in &mut p.controls {
        if let Control::Table(table) = control {
            for cell in &mut table.cells {
                for p in &mut cell.paragraphs {
                    clear_rows(p);
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
    for name in ["center", "baseline"] {
        let mut d = original.clone();
        d.sections[0].paragraphs = d.sections[0].paragraphs[21..26].to_vec();
        d.doc_info.border_fills.push(BorderFill {
            borders: [BorderLine {
                line_type: BorderLineType::Solid,
                width: 1,
                color: 0,
            }; 4],
            ..Default::default()
        });
        let border_id = d.doc_info.border_fills.len() as u16;
        for p in &mut d.sections[0].paragraphs {
            clear_rows(p);
            for control in &mut p.controls {
                if let Control::Table(table) = control {
                    table.border_fill_id = border_id;
                    table.zones.clear();
                    for cell in &mut table.cells {
                        cell.border_fill_id = border_id;
                        for p in &mut cell.paragraphs {
                            let mut shape =
                                d.doc_info.para_shapes[p.para_shape_id as usize].clone();
                            shape.raw_data = None;
                            shape.attr1 = (shape.attr1 & !(3 << 20))
                                | if name == "center" { 2 << 20 } else { 0 };
                            p.para_shape_id = d.doc_info.para_shapes.len() as u16;
                            d.doc_info.para_shapes.push(shape);
                        }
                    }
                }
            }
        }
        std::fs::write(
            format!("output/7353/r19/stored-paint/{name}-input.hwpx"),
            rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap(),
        )
        .unwrap();
    }
}
