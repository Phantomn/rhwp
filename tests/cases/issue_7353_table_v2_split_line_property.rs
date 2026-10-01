//! Header schema boolean/default=false contract, not enabled-line paint fidelity.
//! Patch XML independently of the writer so two missing implementations cannot
//! accidentally cancel each other in a round-trip assertion.
use std::io::{Cursor, Read, Write};

use rhwp::{
    model::{
        control::Control,
        document::{Document, Section},
        paragraph::{CharShapeRef, Paragraph},
        shape::{CommonObjAttr, HorzRelTo, TextWrap, VertRelTo},
        style::{BorderFill, CharShape, LineSpacingType, ParaShape},
        table::{Cell, Table, TablePageBreak},
    },
    renderer::{
        style_resolver::resolve_styles,
        table_v2::{PreparedTextTable, TablePreviewExportSession},
    },
};
use serde_json::{json, Value};

fn table() -> Table {
    Table {
        row_count: 1,
        col_count: 1,
        page_break: TablePageBreak::CellBreak,
        common: CommonObjAttr {
            text_wrap: TextWrap::TopAndBottom,
            horz_rel_to: HorzRelTo::Para,
            vert_rel_to: VertRelTo::Para,
            ..Default::default()
        },
        cells: vec![Cell {
            row_span: 1,
            col_span: 1,
            width: 15000,
            border_fill_id: 1,
            paragraphs: vec![Paragraph {
                text: "A".into(),
                char_count: 1,
                char_offsets: vec![0],
                char_shapes: vec![CharShapeRef {
                    start_pos: 0,
                    char_shape_id: 0,
                }],
                ..Default::default()
            }],
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn document(t: Table) -> Document {
    let mut d = Document::default();
    d.doc_info.char_shapes.push(CharShape {
        base_size: 900,
        ..Default::default()
    });
    d.doc_info.para_shapes.push(ParaShape {
        line_spacing_type: LineSpacingType::Fixed,
        line_spacing: 2700,
        ..Default::default()
    });
    d.doc_info.border_fills.push(BorderFill::default());
    d.sections = vec![Section {
        paragraphs: vec![Paragraph {
            controls: vec![Control::Table(Box::new(t))],
            ..Default::default()
        }],
        ..Default::default()
    }];
    d
}

fn header(bytes: &[u8]) -> String {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut xml = String::new();
    zip.by_name("Contents/header.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    xml
}

fn input(t: Table, value: Option<&str>) -> Vec<u8> {
    let bytes = rhwp::serializer::serialize_hwpx(&document(t)).unwrap();
    let mut source = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..source.len() {
        let mut file = source.by_index(i).unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        if file.name() == "Contents/header.xml" {
            let xml = String::from_utf8(data).unwrap();
            assert_eq!(xml.matches("breakCellSeparateLine=\"0\"").count(), 1);
            let replacement = value
                .map(|v| format!("breakCellSeparateLine=\"{v}\""))
                .unwrap_or_default();
            data = xml
                .replace("breakCellSeparateLine=\"0\"", &replacement)
                .into_bytes();
        }
        out.start_file(
            file.name(),
            zip::write::SimpleFileOptions::default().compression_method(file.compression()),
        )
        .unwrap();
        out.write_all(&data).unwrap();
    }
    out.finish().unwrap().into_inner()
}

fn options(data: &[u8]) -> Value {
    let d = rhwp::parse_document(data).unwrap();
    let control = d.sections[0].paragraphs[0]
        .controls
        .iter()
        .position(|c| matches!(c, Control::Table(_)))
        .unwrap();
    json!({"selection":{"section":0,"paragraph":0,"control":control},"dpi":96,
        "pages":{"width":400,"height":400,"body":{"x":20,"y":30,"width":300,"height":36},"first_y":30},"max_pages":100})
}

fn capture(name: &str, data: &[u8]) {
    if let Ok(dir) = std::env::var("ISSUE7353_EXPORT_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/{name}.hwpx"), data).unwrap();
        std::fs::write(
            format!("{dir}/{name}.options.json"),
            options(data).to_string(),
        )
        .unwrap();
    }
}

#[test]
fn true_and_one_survive_ir_and_hwpx_round_trip() {
    for value in ["true", "1"] {
        let d = rhwp::parse_document(&input(table(), Some(value))).unwrap();
        assert_eq!(
            serde_json::to_value(&d.doc_info.border_fills[0]).unwrap()["break_cell_separate_line"],
            true
        );
        let output = rhwp::serializer::serialize_hwpx(&d).unwrap();
        assert!(header(&output).contains("breakCellSeparateLine=\"1\""));
        let reopened = rhwp::parse_document(&output).unwrap();
        assert_eq!(reopened.doc_info.border_fills, d.doc_info.border_fills);
    }
}

#[test]
fn writer_preserves_enabled_attribute_independently_of_ir_json() {
    let d = rhwp::parse_document(&input(table(), Some("1"))).unwrap();
    assert!(header(&rhwp::serializer::serialize_hwpx(&d).unwrap())
        .contains("breakCellSeparateLine=\"1\""));
}

#[test]
fn absent_false_and_zero_keep_the_same_output() {
    let mut reference = None;
    for value in [None, Some("false"), Some("0")] {
        let data = input(table(), value);
        let config = options(&data);
        let mut session =
            TablePreviewExportSession::from_bytes(&data, &config.to_string()).unwrap();
        let output = session.next_page_json().unwrap().unwrap();
        assert!(session.next_page_json().unwrap().is_none());
        if let Some(previous) = &reference {
            assert_eq!(previous, &output);
        }
        reference = Some(output);
        let d = rhwp::parse_document(&data).unwrap();
        assert!(header(&rhwp::serializer::serialize_hwpx(&d).unwrap())
            .contains("breakCellSeparateLine=\"0\""));
        assert!(rhwp::serializer::serialize_document(&d).is_ok());
    }
}

#[test]
fn enabled_property_is_rejected_before_any_page_even_without_visible_edges() {
    for (name, placement) in [
        ("separate-cell", 0),
        ("separate-table", 1),
        ("separate-child", 2),
    ] {
        let mut t = table();
        if placement == 1 {
            t.cells[0].border_fill_id = 0;
            t.border_fill_id = 1;
        }
        if placement == 2 {
            t.cells[0].border_fill_id = 0;
            let p = &mut t.cells[0].paragraphs[0];
            p.char_count += 8;
            p.char_offsets[0] += 8;
            p.controls.push(Control::Table(Box::new(table())));
        }
        let data = input(t, Some("1"));
        capture(name, &data);
        for _ in 0..2 {
            let error =
                TablePreviewExportSession::from_bytes(&data, &options(&data).to_string()).err();
            assert!(
                format!("{error:?}").contains("V2 separate split-cell border"),
                "{name}: {error:?}"
            );
        }
        // Also qualify table-level and recursive styles through the lower-level
        // entry, which never calls the source DocInfo guard.
        let d = rhwp::parse_document(&data).unwrap();
        let styles = resolve_styles(&d.doc_info, 96.0);
        let selected = options(&data)["selection"]["control"].as_u64().unwrap() as usize;
        let Control::Table(t) = &d.sections[0].paragraphs[0].controls[selected] else {
            panic!("selected table");
        };
        let error = PreparedTextTable::prepare(t, &styles, 96.0).err();
        assert!(
            format!("{error:?}").contains("V2 separate split-cell border"),
            "{name}: {error:?}"
        );
    }
}

#[test]
fn resolved_style_entry_cannot_bypass_property_for_any_split_policy() {
    for policy in [
        TablePageBreak::CellBreak,
        TablePageBreak::RowBreak,
        TablePageBreak::None,
    ] {
        let mut t = table();
        t.page_break = policy;
        let d = rhwp::parse_document(&input(t, Some("true"))).unwrap();
        let styles = resolve_styles(&d.doc_info, 96.0);
        let t = d.sections[0].paragraphs[0]
            .controls
            .iter()
            .find_map(|c| {
                if let Control::Table(t) = c {
                    Some(t.as_ref())
                } else {
                    None
                }
            })
            .unwrap();
        let error = PreparedTextTable::prepare(t, &styles, 96.0).err();
        assert!(
            format!("{error:?}").contains("V2 separate split-cell border"),
            "{error:?}"
        );
    }
}

#[test]
fn unused_enabled_style_does_not_reject_selected_table() {
    let mut t = table();
    t.cells[0].border_fill_id = 0;
    let data = input(t, Some("1"));
    let mut session =
        TablePreviewExportSession::from_bytes(&data, &options(&data).to_string()).unwrap();
    assert!(session.next_page_json().unwrap().is_some());
    assert!(session.next_page_json().unwrap().is_none());
}

#[test]
fn hwp_export_preserves_enabled_property_in_plain_report_and_encrypted_paths() {
    let d = rhwp::parse_document(&input(table(), Some("1"))).unwrap();
    for result in [
        rhwp::serializer::serialize_document(&d),
        rhwp::serializer::serialize_document_with_report(&d).map(|r| r.into_bytes()),
    ] {
        let bytes = result.expect("supported HWP5 property");
        let reopened = rhwp::parse_document(&bytes).unwrap();
        assert!(reopened.doc_info.border_fills[0].break_cell_separate_line);
    }
    let bytes = rhwp::serializer::serialize_hwp_with_password(&d, b"example").unwrap();
    let reopened = rhwp::parser::parse_document_with_password(&bytes, b"example").unwrap();
    assert!(reopened.doc_info.border_fills[0].break_cell_separate_line);
}

#[test]
fn hancom_border_fill_record_separates_split_line_from_center_line() {
    // Hancom 11.0.0.9136 saved #3528's enabled BorderFill 15. The OFF
    // counterpart is byte-identical except attr=0 (deduplicated to id 8).
    // Source/artifact hashes and jobs: task_m100_7353_stage19.md, 2026-10-01.
    let hex = "00040101000000000101000000000101000000000101000000000001000000000000000000000000";
    let payload: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    for (attr, enabled, center) in [
        (0x0000_u16, false, rhwp::model::style::CenterLine::None),
        (0x0400, true, rhwp::model::style::CenterLine::None),
        // Hancom stores the direction in bits 8..9, not in bit 10.
        // Sample 대각선샘플3.hwp / Hancom PDF: 0x2300 draws BOTH center lines.
        (0x2100, false, rhwp::model::style::CenterLine::Vertical),
        (0x2500, true, rhwp::model::style::CenterLine::Vertical),
        (0x2200, false, rhwp::model::style::CenterLine::Horizontal),
        (0x2600, true, rhwp::model::style::CenterLine::Horizontal),
        (0x2300, false, rhwp::model::style::CenterLine::Cross),
        (0x2700, true, rhwp::model::style::CenterLine::Cross),
    ] {
        let mut bytes = payload.clone();
        bytes[..2].copy_from_slice(&attr.to_le_bytes());
        let record = rhwp::serializer::record_writer::write_record(20, 1, &bytes);
        let (info, _) = rhwp::parser::doc_info::parse_doc_info(&record).unwrap();
        let bf = &info.border_fills[0];
        assert_eq!(bf.break_cell_separate_line, enabled, "{attr:#06x}");
        assert_eq!(bf.center_line, center, "{attr:#06x}");
        let mut updated = bf.clone();
        updated.break_cell_separate_line = !enabled;
        let out = rhwp::serializer::doc_info::serialize_border_fill(&updated);
        assert_eq!(u16::from_le_bytes([out[0], out[1]]), attr ^ 0x0400);
        assert_eq!(&out[2..], &bytes[2..]);
    }
}

#[test]
fn center_directions_survive_both_formats_independently_of_split_line() {
    use rhwp::model::style::CenterLine;

    for (direction, crooked) in [
        (CenterLine::Vertical, 1),
        (CenterLine::Horizontal, 2),
        (CenterLine::Cross, 3),
    ] {
        for separate in [false, true] {
            let mut d = document(table());
            d.doc_info.border_fills[0].center_line = direction;
            d.doc_info.border_fills[0].break_cell_separate_line = separate;
            let bytes = rhwp::serializer::serialize_document(&d).unwrap();
            let parsed = rhwp::parse_document(&bytes).unwrap();
            let bf = &parsed.doc_info.border_fills[0];
            assert_eq!(bf.center_line, direction);
            assert_eq!(bf.break_cell_separate_line, separate);
            assert_eq!((bf.attr >> 8) & 3, crooked);
            assert_eq!(bf.attr & (1 << 10), 0);

            let hwpx = rhwp::serializer::serialize_hwpx(&parsed).unwrap();
            let xml = header(&hwpx);
            assert!(xml.contains("centerLine=\"VERTICAL\""));
            assert!(xml.contains(&format!("<hh:slash type=\"NONE\" Crooked=\"{crooked}\"")));
            assert!(xml.contains("<hh:backSlash type=\"NONE\" Crooked=\"0\""));
            let reparsed = rhwp::parse_document(&hwpx).unwrap();
            assert_eq!(reparsed.doc_info.border_fills[0].center_line, direction);
            assert_eq!(
                reparsed.doc_info.border_fills[0].break_cell_separate_line,
                separate
            );
        }
    }
}

#[test]
fn hancom_original_cross_is_not_misread_as_horizontal() {
    // Independent Hancom PDF of this original HWP shows the two crossing
    // center strokes inside the first cell. HWPX is the matching saved sample.
    for path in ["samples/대각선샘플3.hwp", "samples/대각선샘플3.hwpx"] {
        let d = rhwp::parse_document(&std::fs::read(path).unwrap()).unwrap();
        let bf = &d.doc_info.border_fills[4];
        assert_eq!(
            bf.center_line,
            rhwp::model::style::CenterLine::Cross,
            "{path}"
        );
        assert!(!bf.break_cell_separate_line, "{path}");
    }
}

#[test]
fn hwp_property_edits_invalidate_raw_reuse_and_do_not_leak_into_hwpx_diagonals() {
    let doc = rhwp::parse_document(&input(table(), Some("1"))).unwrap();
    let bytes = rhwp::serializer::serialize_document(&doc).unwrap();
    let mut parsed = rhwp::parse_document(&bytes).unwrap();
    assert!(parsed.doc_info.raw_stream.is_some());
    let hwpx = rhwp::serializer::serialize_hwpx(&parsed).unwrap();
    assert!(header(&hwpx).contains("breakCellSeparateLine=\"1\""));
    assert!(header(&hwpx).contains("<hh:backSlash type=\"NONE\" Crooked=\"0\""));
    for enabled in [false, true] {
        parsed.doc_info.border_fills[0].break_cell_separate_line = enabled;
        let bytes = rhwp::serializer::serialize_document(&parsed).unwrap();
        parsed = rhwp::parse_document(&bytes).unwrap();
        assert_eq!(
            parsed.doc_info.border_fills[0].break_cell_separate_line,
            enabled
        );
    }
}

#[test]
fn hml_preflight_blocks_enabled_property_but_keeps_default_exportable() {
    let xml = br#"<HWPML Version="2.91"><HEAD/><BODY><SECTION><P><TEXT/></P></SECTION></BODY><TAIL/></HWPML>"#;
    let mut parsed = rhwp::parser::parse_document_with_metadata(xml).unwrap();
    let metadata = parsed.hml_metadata.unwrap();
    parsed.document.doc_info.border_fills = vec![BorderFill::default()];
    rhwp::serializer::serialize_hml(&parsed.document, &metadata).unwrap();
    let enabled = rhwp::parse_document(&input(table(), Some("1"))).unwrap();
    parsed.document.doc_info.border_fills = enabled.doc_info.border_fills;
    let error = rhwp::serializer::serialize_hml(&parsed.document, &metadata).unwrap_err();
    assert!(error
        .blockers()
        .iter()
        .any(|b| b.xml_path.contains("BORDERFILL")
            && b.message.contains("border-fill fields omitted")));
}
