//! Normally saved source with paragraph-internal cell frame continuations.
use rhwp::renderer::table_v2::DocumentV2Session;
use serde_json::Value;

fn nodes<'a>(n: &'a Value, kind: &str) -> Vec<&'a Value> {
    let mut out = Vec::new();
    if n["node_type"].get(kind).is_some() {
        out.push(n);
    }
    for child in n["children"].as_array().unwrap() {
        out.extend(nodes(child, kind));
    }
    out
}

#[test]
fn saved_cell_paragraph_continues_without_losing_line_ownership() {
    let bytes = include_bytes!("../fixtures/issue7353/cell-line-continuation/prefix17-saved.hwp");
    let mut s = DocumentV2Session::from_bytes(
        bytes,
        r#"{"dpi":96,"max_pages":30,"cell_end_policy":"omit_final_paragraph_gap"}"#,
    )
    .unwrap();
    let mut pages = Vec::new();
    while let Some(raw) = s.next_page_json().unwrap() {
        pages.push(serde_json::from_str::<Value>(&raw).unwrap());
    }
    // Independent Hancom PDF p4 ends mid-paragraph after the line containing
    // 주민대표단의 대표, 감사 및 단원; its continuation begins p5.
    let lines = |page: usize| {
        nodes(&pages[page]["render_tree"]["root"], "TextLine")
            .iter()
            .map(|line| {
                nodes(line, "TextRun")
                    .iter()
                    .map(|r| r["node_type"]["TextRun"]["text"].as_str().unwrap())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
    };
    assert!(lines(3)
        .iter()
        .any(|s| s.contains("주민대표단의 대표, 감사 및 단원")));
    assert_eq!(
        lines(3)
            .iter()
            .filter(|s| s.trim() == "의 주소ㆍ성명")
            .count(),
        1
    );
    assert_eq!(
        lines(4)
            .iter()
            .filter(|s| s.trim() == "의 주소ㆍ성명")
            .count(),
        1
    );
    let doc = rhwp::parse_document(bytes).unwrap();
    let rhwp::model::control::Control::Table(source) = &doc.sections[0].paragraphs[15].controls[0]
    else {
        panic!()
    };
    for col in 0..2 {
        let cell = source
            .cells
            .iter()
            .find(|c| c.row == 1 && c.col == col)
            .unwrap();
        let expected: Vec<_> = cell
            .paragraphs
            .iter()
            .flat_map(|p| {
                let chars: Vec<_> = p.text.chars().collect();
                (0..p.line_segs.len()).map(move |li| {
                    let start = p
                        .char_offsets
                        .partition_point(|&o| o < p.line_seg_text_start(li));
                    let end = if li + 1 == p.line_segs.len() {
                        chars.len()
                    } else {
                        p.char_offsets
                            .partition_point(|&o| o < p.line_seg_text_start(li + 1))
                    };
                    chars[start..end].iter().collect::<String>()
                })
            })
            .collect();
        let mut actual = Vec::new();
        for page in &pages[3..9] {
            let root = &page["render_tree"]["root"];
            let table = nodes(root, "Table")[0];
            let cell = nodes(table, "TableCell")
                .into_iter()
                .find(|c| {
                    c["node_type"]["TableCell"]["row"] == 1
                        && c["node_type"]["TableCell"]["col"] == col
                })
                .unwrap();
            let y = cell["bbox"]["y"].as_f64().unwrap();
            let end = y + cell["bbox"]["height"].as_f64().unwrap();
            for line in nodes(cell, "TextLine") {
                let ly = line["bbox"]["y"].as_f64().unwrap();
                assert!(ly >= y && ly + line["bbox"]["height"].as_f64().unwrap() <= end + 1e-8);
                actual.push(
                    nodes(line, "TextRun")
                        .iter()
                        .map(|r| r["node_type"]["TextRun"]["text"].as_str().unwrap())
                        .collect::<String>(),
                );
            }
            // Repeated title and body share the same final geometry.
            assert_eq!(
                nodes(table, "TableCell")
                    .iter()
                    .filter(|c| c["node_type"]["TableCell"]["row"] == 0)
                    .count(),
                2
            );
        }
        assert_eq!(
            actual, expected,
            "every stored line, including blanks, exactly once in col{col}"
        );
    }
    assert_eq!(pages.len(), 10);
    assert!(lines(9)
        .iter()
        .any(|s| s.contains("규제의 필요성 및 대안선택")));
}

#[test]
fn internal_cut_reserves_both_insets_but_not_interline_tail_and_never_keeps_pitch() {
    use rhwp::{
        model::{control::Control, table::TablePageBreak},
        renderer::{style_resolver::resolve_styles, table_v2::*},
    };
    let d = rhwp::parse_document(include_bytes!(
        "../fixtures/issue7353/cell-line-continuation/prefix17-saved.hwp"
    ))
    .unwrap();
    let Control::Table(source) = &d.sections[0].paragraphs[15].controls[0] else {
        panic!()
    };
    let mut t = (**source).clone();
    let mut c = t
        .cells
        .iter()
        .find(|c| c.row == 1 && c.col == 1)
        .unwrap()
        .clone();
    c.row = 0;
    c.col = 0;
    c.height = 0;
    c.is_header = false;
    c.paragraphs = vec![c.paragraphs[7].clone()];
    t.row_count = 1;
    t.col_count = 1;
    t.row_sizes = vec![1];
    t.repeat_header = false;
    t.common.width = c.width;
    t.common.height = 0;
    t.cells = vec![c];
    let styles = resolve_styles(&d.doc_info, 96.0);
    let area = |height| PageArea {
        bounds: Rect {
            x: 3.0,
            y: 7.0,
            width: 400.0,
            height,
        },
    };
    // Independent stored metrics:1400HU line,1120HU gap,141HU per inset.
    let required = (1400.0 + 282.0) / 75.0;
    for policy in [TablePageBreak::RowBreak, TablePageBreak::None] {
        t.page_break = policy;
        let p = PreparedTextTable::prepare_with_end_policy(
            &t,
            &styles,
            96.0,
            &[],
            CellEndPolicy::OmitFinalParagraphGap,
        )
        .unwrap();
        let cursor = p.start();
        if policy == TablePageBreak::RowBreak {
            assert!(matches!(
                cursor.fit(area(required - 1.0 / 75.0)).unwrap(),
                TextFragmentFit::DoesNotFit { .. }
            ));
            let TextFragmentFit::Placed(first) = cursor.fit(area(100.0)).unwrap() else {
                panic!()
            };
            assert!((first.geometry().reserved_height() - required).abs() < 1e-8);
            assert_eq!(
                first.geometry().placement().cells[0].lines[0].owner,
                LineOwner {
                    paragraph: 0,
                    line: 0
                }
            );
            assert_eq!(first.geometry().placement().cells[0].lines.len(), 1);
            let TextFragmentFit::Placed(last) = first.continuation().fit(area(required)).unwrap()
            else {
                panic!()
            };
            assert_eq!(
                last.geometry().placement().cells[0].lines[0].owner,
                LineOwner {
                    paragraph: 0,
                    line: 1
                }
            );
            assert!(
                (last.geometry().placement().cells[0].lines[0].bounds.y - 7.0 - 141.0 / 75.0).abs()
                    < 1e-8
            );
            assert!(matches!(
                last.continuation().fit(area(100.0)).unwrap(),
                TextFragmentFit::Complete
            ));
        } else {
            let TextFragmentFit::Placed(f) = cursor.fit(area(100.0)).unwrap() else {
                panic!()
            };
            assert_eq!(f.geometry().placement().cells[0].lines.len(), 2);
            assert!(
                (f.geometry().reserved_height() - (2800.0 + 1120.0 + 282.0) / 75.0).abs() < 1e-8
            );
        }
    }
}

#[test]
fn repeated_header_capacity_is_not_the_current_remainder_or_header_only_progress() {
    use rhwp::{
        model::{
            paragraph::Paragraph,
            table::{Cell, Table, TablePageBreak},
        },
        renderer::table_v2::*,
    };
    struct Composer;
    impl CellParagraphComposer for Composer {
        fn compose(&self, p: &Paragraph, _: f64) -> Result<Vec<ParagraphItem>, GeometryError> {
            let height: f64 = p.text.parse().unwrap();
            Ok(vec![ParagraphItem::Lines {
                height,
                advance: height,
                lines: vec![(
                    0,
                    Rect {
                        x: 0.0,
                        y: 0.0,
                        width: 80.0,
                        height,
                    },
                )],
            }])
        }
    }
    let paragraph = |text: &str| Paragraph {
        text: text.into(),
        ..Default::default()
    };
    let mut table = Table {
        row_count: 2,
        col_count: 1,
        repeat_header: true,
        page_break: TablePageBreak::CellBreak,
        cells: vec![
            Cell {
                row: 0,
                col: 0,
                row_span: 1,
                col_span: 1,
                width: 7500,
                is_header: true,
                paragraphs: vec![paragraph("10")],
                ..Default::default()
            },
            Cell {
                row: 1,
                col: 0,
                row_span: 1,
                col_span: 1,
                width: 7500,
                paragraphs: vec![paragraph("20"), paragraph("20")],
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let area = |height| PageArea {
        bounds: Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height,
        },
    };
    let cursor = TableContentPlan::from_ir_contents(&table, 1.0 / 75.0, &Composer)
        .unwrap()
        .start();
    // Body40 fits fresh50 minus header10: insufficient remainder must defer.
    assert!(matches!(
        cursor.fit_in_page(area(30.0), 50.0).unwrap(),
        FragmentFit::DoesNotFit {
            required_height: 50.0,
            ..
        }
    ));
    // Header occupancy alone must not turn the ordinary40px row into an
    // oversized row. Keep the caller's fresh-page classification unchanged.
    assert!(matches!(
        cursor.fit_in_page(area(45.0), 45.0).unwrap(),
        FragmentFit::DoesNotFit {
            required_height: 50.0,
            ..
        }
    ));
    table.cells[1].paragraphs.push(paragraph("20"));
    let cursor = TableContentPlan::from_ir_contents(&table, 1.0 / 75.0, &Composer)
        .unwrap()
        .start();
    // Body60 really exceeds fresh50: use composed units below the title.
    assert!(matches!(
        cursor.fit_in_page(area(29.0), 50.0).unwrap(),
        FragmentFit::DoesNotFit {
            required_height: 30.0,
            ..
        }
    ));
    let FragmentFit::Placed(first) = cursor.fit_in_page(area(30.0), 50.0).unwrap() else {
        panic!()
    };
    assert_eq!(first.reserved_height(), 30.0);
    assert_eq!(
        first.placement().cells[1].lines[0].owner,
        LineOwner {
            paragraph: 0,
            line: 0
        }
    );
    let FragmentFit::Placed(last) = first.continuation().fit_in_page(area(50.0), 50.0).unwrap()
    else {
        panic!()
    };
    assert_eq!(last.reserved_height(), 50.0);
    assert_eq!(last.placement().cells[0].lines.len(), 1);
    assert_eq!(
        last.placement().cells[1].lines[0].owner,
        LineOwner {
            paragraph: 1,
            line: 0
        }
    );
    assert_eq!(
        last.placement().cells[1].lines[1].owner,
        LineOwner {
            paragraph: 2,
            line: 0
        }
    );
    assert!(last.continuation().is_complete());
}

#[test]
fn body_column_reference_does_not_become_a_cell_relative_anchor() {
    use rhwp::{
        model::table::{Cell, Table},
        renderer::{style_resolver::resolve_styles, table_v2::PreparedTextTable},
    };
    let d = rhwp::parse_document(include_bytes!(
        "../fixtures/issue7353/cell-line-continuation/prefix17-saved.hwp"
    ))
    .unwrap();
    let t = Table {
        row_count: 1,
        col_count: 1,
        cells: vec![Cell {
            row_span: 1,
            col_span: 1,
            width: 55000,
            paragraphs: vec![d.sections[0].paragraphs[15].clone()],
            ..Default::default()
        }],
        ..Default::default()
    };
    match PreparedTextTable::prepare(&t, &resolve_styles(&d.doc_info, 96.0), 96.0) {
        Err(rhwp::renderer::table_v2::GeometryError::Unsupported(
            "stored excluded cell anchor",
        )) => (),
        _ => panic!("a cell does not establish the document column coordinate frame"),
    }
}

#[test]
fn nonzero_backward_origin_is_not_qualified_as_a_saved_cell_frame() {
    use rhwp::{
        model::control::Control,
        renderer::{style_resolver::resolve_styles, table_v2::*},
    };
    let d = rhwp::parse_document(include_bytes!(
        "../fixtures/issue7353/cell-line-continuation/prefix17-saved.hwp"
    ))
    .unwrap();
    let Control::Table(source) = &d.sections[0].paragraphs[15].controls[0] else {
        panic!()
    };
    let mut t = (**source).clone();
    t.cells
        .iter_mut()
        .find(|c| c.row == 1 && c.col == 1)
        .unwrap()
        .paragraphs[7]
        .line_segs[1]
        .vertical_pos = 100;
    assert!(matches!(
        PreparedTextTable::prepare(&t, &resolve_styles(&d.doc_info, 96.0), 96.0),
        Err(GeometryError::Unsupported(
            "unqualified stored cell frame reset"
        ))
    ));
}
