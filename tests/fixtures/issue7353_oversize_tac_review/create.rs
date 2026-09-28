// Fresh source geometry, without invented stored LineSegs. HWPX is the
// independent Hancom observation input. HWP preserves explicit structural
// slots for the currently qualified V2 fresh-TAC input path.
use rhwp::model::{control::Control, table::TablePageBreak};
fn main() {
    let base = rhwp::parse_document(
        &std::fs::read("tests/fixtures/issue7353_fresh_tac_box_review/carrier.hwpx").unwrap(),
    )
    .unwrap();
    let page = &base.sections[0].section_def.page_def;
    let body = page.height
        - page.margin_top
        - page.margin_header
        - page.margin_bottom
        - page.margin_footer;
    for (name, preceded, extra) in [
        ("first", false, 3000),
        ("preceded", true, 3000),
        ("grown", false, 18000),
    ] {
        let mut d = base.clone();
        let mut p = d.sections[0].paragraphs[1].clone();
        p.controls.truncate(1);
        p.char_count = 9;
        let Control::Table(t) = &mut p.controls[0] else {
            panic!()
        };
        t.page_break = TablePageBreak::RowBreak;
        t.common.height = body + extra;
        t.cells[0].height = body + extra;
        let after = d.sections[0].paragraphs[2].clone();
        d.sections[0].paragraphs = if preceded {
            vec![d.sections[0].paragraphs[0].clone(), p, after]
        } else {
            vec![p, after]
        };
        let dir = "tests/fixtures/issue7353_oversize_tac_review";
        std::fs::write(
            format!("{dir}/{name}.hwpx"),
            rhwp::serializer::serialize_hwpx(&d).unwrap(),
        )
        .unwrap();
        if !preceded {
            let def = d.sections[0].section_def.clone();
            let first = &mut d.sections[0].paragraphs[0];
            first.controls.insert(0, Control::SectionDef(Box::new(def)));
            first
                .controls
                .insert(1, Control::ColumnDef(Default::default()));
            first.char_count += 16;
            for offset in &mut first.char_offsets {
                *offset += 16;
            }
            for line in &mut first.line_segs {
                if line.text_start != 0 {
                    line.text_start += 16;
                }
            }
        }
        std::fs::write(
            format!("{dir}/{name}.hwp"),
            rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap(),
        )
        .unwrap();
    }
}
