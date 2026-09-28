//! Normally saved URC CHAR indentation, with unchanged stored line ownership.
use rhwp::renderer::table_v2::DocumentV2Session;
use serde_json::Value;

fn lines<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
    if node["node_type"].get("TextLine").is_some() {
        out.push(node);
    }
    for child in node["children"].as_array().unwrap() {
        lines(child, out);
    }
}

fn text(node: &Value) -> String {
    node["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["node_type"]["TextRun"]["text"].as_str().unwrap())
        .collect()
}

fn check_saved_indent(bytes: &[u8], normal_size: i32) {
    let doc = rhwp::parse_document(bytes).unwrap();
    let p = &doc.sections[0].paragraphs[20];
    // Normal Hancom save retains the odd URC property and bit20 ownership.
    // This source property, not the implementation's measured width, is the oracle.
    assert_eq!(
        doc.doc_info.para_shapes[p.para_shape_id as usize].indent,
        -3001
    );
    assert_eq!(p.line_segs.len(), 7);
    assert!(!p.line_segs[0].has_indentation());
    assert!(p.line_segs[1..].iter().all(|r| r.has_indentation()));
    let chars: Vec<_> = p.text.chars().collect();
    let expected: Vec<String> = (0..p.line_segs.len())
        .map(|i| {
            let start = p
                .char_offsets
                .partition_point(|&o| o < p.line_seg_text_start(i));
            let end = if i + 1 == p.line_segs.len() {
                chars.len()
            } else {
                p.char_offsets
                    .partition_point(|&o| o < p.line_seg_text_start(i + 1))
            };
            chars[start..end].iter().collect()
        })
        .collect();
    let normal = &doc.doc_info.char_shapes[doc.doc_info.styles[0].char_shape_id as usize];
    assert_eq!(normal.base_size, normal_size);
    // Official URC decode: (-3001 >> 1)/100 = -15.01 ch.
    // Normal Latin half-em is5pt in the source,6pt in the differential control.
    // Independent PDFs put these lines about75/90pt to the right respectively.
    // DPI changes must not quantize the inset or alter source membership.
    for dpi in [96.0, 144.0] {
        let mut session = DocumentV2Session::from_bytes(
            bytes,
            &format!(
                r#"{{"dpi":{dpi},"max_pages":20,"cell_end_policy":"omit_final_paragraph_gap"}}"#
            ),
        )
        .unwrap();
        let mut pages = Vec::new();
        while let Some(raw) = session.next_page_json().unwrap() {
            pages.push(serde_json::from_str::<Value>(&raw).unwrap());
        }
        let mut all = Vec::new();
        for page in &pages {
            lines(&page["render_tree"]["root"], &mut all);
        }
        let actual: Vec<_> = all
            .into_iter()
            .filter(|n| expected.contains(&text(n)))
            .collect();
        assert_eq!(actual.iter().map(|n| text(n)).collect::<Vec<_>>(), expected);
        let x0 = actual[0]["bbox"]["x"].as_f64().unwrap();
        let y0 = actual[0]["bbox"]["y"].as_f64().unwrap();
        for (i, line) in actual.iter().enumerate() {
            let source = &p.line_segs[i];
            let inset = if i == 0 {
                0.0
            } else {
                15.01 * f64::from(normal_size) / 2.0 * dpi / 7200.0
            };
            let x = line["bbox"]["x"].as_f64().unwrap();
            let w = line["bbox"]["width"].as_f64().unwrap();
            assert!((x - x0 - inset).abs() < 1e-9);
            assert!((w + inset - f64::from(source.segment_width) * dpi / 7200.0).abs() < 1e-9);
            assert!((line["children"][0]["bbox"]["x"].as_f64().unwrap() - x).abs() < 1e-9);
            assert!(
                (line["bbox"]["y"].as_f64().unwrap()
                    - y0
                    - f64::from(source.vertical_pos - p.line_segs[0].vertical_pos) * dpi / 7200.0)
                    .abs()
                    < 1e-9
            );
        }
    }
}

#[test]
fn saved_urc_hanging_indent_reaches_final_line_and_glyph_boxes() {
    check_saved_indent(
        include_bytes!("../fixtures/issue7353/half-unit-indent/prefix21-saved.hwp"),
        1000,
    );
}

#[test]
fn normal_style_basis_changes_indent_not_source_run_sizes() {
    let a = include_bytes!("../fixtures/issue7353/half-unit-indent/prefix21-saved.hwp");
    let b = include_bytes!("../fixtures/issue7353/half-unit-indent/normal12-saved.hwp");
    let original = rhwp::parse_document(a).unwrap();
    let control = rhwp::parse_document(b).unwrap();
    let pa = &original.sections[0].paragraphs[20];
    let pb = &control.sections[0].paragraphs[20];
    assert_eq!(pa.text, pb.text);
    // Hancom recomposes the narrower lane when saving: keep each file's own
    // new partitions, not the original10pt-basis partition in the12pt control.
    assert_eq!(
        pb.line_segs
            .iter()
            .map(|s| s.text_start)
            .collect::<Vec<_>>(),
        [0, 39, 73, 104, 140, 173, 204]
    );
    for (ra, rb) in pa.char_shapes.iter().zip(&pb.char_shapes) {
        assert_eq!(
            original.doc_info.char_shapes[ra.char_shape_id as usize].base_size,
            control.doc_info.char_shapes[rb.char_shape_id as usize].base_size
        );
    }
    check_saved_indent(b, 1200);
}

#[test]
fn selected_table_uses_same_urc_projection_and_rejects_missing_basis() {
    use rhwp::{
        model::{
            control::Control,
            paragraph::{LineSeg, Paragraph},
            table::{Cell, Table},
        },
        renderer::table_v2::*,
    };
    // Synthetic cell carrier for the real saved paragraph. This is a route
    // contract, not a claim that this carrier was authored in Hancom.
    for positive in [false, true] {
        let mut doc = rhwp::parse_document(include_bytes!(
            "../fixtures/issue7353/half-unit-indent/prefix21-saved.hwp"
        ))
        .unwrap();
        let mut p = doc.sections[0].paragraphs[20].clone();
        if positive {
            doc.doc_info.para_shapes[p.para_shape_id as usize].indent = (1501 << 1) | 1;
            // The real first line fills its nonindented lane. Do not retain
            // that invalid partition after insetting it: use a two-letter
            // synthetic carrier for the positive-sign decode contract.
            p.text = "AB".into();
            p.char_count = 3;
            p.char_offsets = vec![0, 1];
            p.char_shapes.truncate(1);
            p.line_segs.truncate(2);
            for (i, row) in p.line_segs.iter_mut().enumerate() {
                row.text_start = i as u32;
                row.vertical_pos = i as i32 * 2400;
                row.line_height = 1500;
                row.text_height = 1500;
                row.baseline_distance = 1275;
                row.line_spacing = 900;
                row.tag &= !LineSeg::TAG_INDENTATION;
                if i == 0 {
                    row.tag |= LineSeg::TAG_INDENTATION;
                }
            }
        }
        let table = Table {
            row_count: 1,
            col_count: 1,
            cells: vec![Cell {
                width: 48188,
                row_span: 1,
                col_span: 1,
                paragraphs: vec![p],
                ..Default::default()
            }],
            ..Default::default()
        };
        doc.sections[0].paragraphs = vec![Paragraph {
            controls: vec![Control::Table(Box::new(table))],
            ..Default::default()
        }];
        let create = |d: &rhwp::model::document::Document| {
            TablePreviewSession::from_document(
                d,
                TableSelection {
                    section: 0,
                    paragraph: 0,
                    control: 0,
                },
                96.0,
                TablePreviewPages {
                    width: 800.0,
                    height: 1000.0,
                    body: Rect {
                        x: 50.0,
                        y: 50.0,
                        width: 650.0,
                        height: 900.0,
                    },
                    first_y: 50.0,
                },
                10,
            )
        };
        let mut preview = create(&doc).unwrap_or_else(|e| panic!("positive={positive}: {e:?}"));
        let page = preview.next_page().unwrap().unwrap();
        let root = serde_json::to_value(&page.tree.root).unwrap();
        let mut actual = Vec::new();
        lines(&root, &mut actual);
        assert_eq!(actual.len(), if positive { 2 } else { 7 });
        for (i, line) in actual.iter().enumerate() {
            let inset = if (i == 0) == positive {
                15.01 * 5.0 * 96.0 / 72.0
            } else {
                0.0
            };
            assert!((line["bbox"]["x"].as_f64().unwrap() - 50.0 - inset).abs() < 1e-9);
            assert!(
                (line["children"][0]["bbox"]["x"].as_f64().unwrap() - 50.0 - inset).abs() < 1e-9
            );
        }
        assert!(preview.next_page().unwrap().is_none());
        let shape_id = if let Control::Table(t) = &doc.sections[0].paragraphs[0].controls[0] {
            t.cells[0].paragraphs[0].para_shape_id as usize
        } else {
            unreachable!()
        };
        doc.doc_info.para_shapes[shape_id].hwpx_plain_para_margin = true;
        assert!(
            create(&doc).is_err(),
            "odd plain HWPX is not evidence of URC"
        );
        doc.doc_info.para_shapes[shape_id].hwpx_plain_para_margin = false;
        doc.doc_info.styles.clear();
        assert!(create(&doc).is_err(), "no silent basis fallback");
    }
}
