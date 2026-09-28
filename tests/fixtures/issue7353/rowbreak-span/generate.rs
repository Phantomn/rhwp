//! Reproduce the controlled HWPX variants before normal Hancom save/export.
//! This utility is not an independent layout oracle.
use rhwp::model::control::Control;
fn main() {
    let output = std::env::args().nth(1).expect("output directory");
    std::fs::create_dir_all(&output).unwrap();
    for mode in ["margin", "no-bottom"] {
        let mut doc = rhwp::parse_document(include_bytes!("prefix14-saved.hwp")).unwrap();
        if mode == "margin" {
            doc.sections[0].section_def.page_def.margin_bottom += 600;
        } else {
            for control in &mut doc.sections[0].paragraphs[13].controls {
                if let Control::Table(table) = control {
                    table.common.margin.bottom = 0;
                    table.outer_margin_bottom = 0;
                }
            }
        }
        let section = doc.sections[0].section_def.clone();
        for paragraph in &mut doc.sections[0].paragraphs {
            for control in &mut paragraph.controls {
                if let Control::SectionDef(def) = control {
                    **def = section.clone();
                }
            }
        }
        std::fs::write(
            format!("{output}/{mode}-input.hwpx"),
            rhwp::serializer::hwpx::serialize_hwpx(&doc).unwrap(),
        )
        .unwrap();
    }
}
