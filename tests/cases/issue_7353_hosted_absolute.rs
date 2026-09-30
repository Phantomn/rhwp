//! Normal Hancom save: 14400HU columns, 1200HU gap, 1200+600HU
//! line pitch. Expected positions come from saved rows and the same-HWP PDF,
//! not from the implementation's table-height query.
use rhwp::{
    model::{
        control::Control,
        document::Document,
        shape::{HorzRelTo, VertRelTo},
    },
    renderer::{
        render_tree::{RenderNode, RenderNodeType},
        table_v2::*,
    },
};

fn source() -> Document {
    rhwp::parse_document(include_bytes!(
        "../fixtures/issue7353_host_absolute_review/saved.hwp"
    ))
    .unwrap()
}
fn session(d: &Document, dpi: f64) -> Result<HostedSectionSession, HostedTableError> {
    HostedSectionSession::from_document(d, 0, dpi, CellEndPolicy::OmitFinalParagraphGap)
}
fn tables(n: &RenderNode) -> Vec<&RenderNode> {
    let mut out = Vec::new();
    if matches!(n.node_type, RenderNodeType::Table(_)) {
        out.push(n);
    }
    for c in &n.children {
        out.extend(tables(c));
    }
    out
}
fn lines(n: &RenderNode) -> Vec<&RenderNode> {
    if matches!(n.node_type, RenderNodeType::Table(_)) {
        return vec![];
    }
    if matches!(n.node_type, RenderNodeType::TextLine(_)) {
        return vec![n];
    }
    n.children.iter().flat_map(lines).collect()
}
fn text(n: &RenderNode) -> String {
    let mut s = match &n.node_type {
        RenderNodeType::TextRun(r) => r.text.clone(),
        _ => String::new(),
    };
    for c in &n.children {
        s.push_str(&text(c));
    }
    s
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}
fn table_mut(d: &mut Document, pi: usize) -> &mut rhwp::model::table::Table {
    d.sections[0].paragraphs[pi]
        .controls
        .iter_mut()
        .find_map(|c| match c {
            Control::Table(t) => Some(t.as_mut()),
            _ => None,
        })
        .unwrap()
}

#[test]
fn saved_columns_bind_absolute_tables_to_the_actual_owner_frame() {
    let d = source();
    let before = format!("{d:?}");
    for d in [
        &d,
        &rhwp::parse_document(&rhwp::serializer::serialize_hwpx(&d).unwrap()).unwrap(),
    ] {
        for dpi in [96., 192.] {
            let s = session(d, dpi).unwrap();
            assert_eq!(s.pagination().pages.len(), 2);
            let a = s.render_page(0).unwrap();
            let b = s.render_page(1).unwrap();
            let scale = dpi / 7200.;
            let ts = tables(&a.root);
            assert_eq!(ts.len(), 2);
            assert!(tables(&b.root).is_empty());
            for (t, (x, y)) in ts.iter().zip([(13500., 2400.), (23400., 34600.)]) {
                close(t.bbox.x, x * scale);
                close(t.bbox.y, y * scale);
                close(t.bbox.width, 9000. * scale);
                close(t.bbox.height, 3000. * scale);
            }
            let expected = [
                "BEFORE",
                "LINE 01",
                "LINE 02",
                "",
                "LINE 04",
                "LINE 05",
                "LINE 06",
                "LINE 07",
                "LINE 08",
                "LINE 09",
                "FOOTER OWNER",
                "LINE 11",
                "LINE 12",
                "LINE 13",
                "LINE 14",
                "LINE 15",
            ];
            let rows = lines(&a.root);
            assert_eq!(rows.len(), expected.len());
            for (i, (row, label)) in rows.iter().zip(expected).enumerate() {
                assert_eq!(text(row), label);
                close(row.bbox.x, if i < 8 { 2400. } else { 18000. } * scale);
                close(row.bbox.y, (8000. + (i % 8) as f64 * 1800.) * scale);
            }
            let rows = lines(&b.root);
            assert_eq!(rows.len(), 4);
            for (i, (row, label)) in rows
                .iter()
                .zip(["LINE 16", "LINE 17", "LINE 18", "AFTER"])
                .enumerate()
            {
                assert_eq!(text(row), label);
                close(row.bbox.x, 2400. * scale);
                close(row.bbox.y, (8000. + i as f64 * 1800.) * scale);
            }
            assert_eq!(
                s.render_page_json(0).unwrap(),
                s.render_page_json(0).unwrap()
            );
        }
    }
    assert_eq!(format!("{d:?}"), before);
}

#[test]
fn pure_saved_columns_work_without_an_absolute_object() {
    let mut d = source();
    // Synthetic transport control: retain saved source offsets, no table paint.
    for p in &mut d.sections[0].paragraphs {
        p.controls.retain(|c| !matches!(c, Control::Table(_)));
    }
    let s = session(&d, 96.).unwrap();
    assert_eq!(s.pagination().pages.len(), 2);
    assert_eq!(lines(&s.render_page(0).unwrap().root).len(), 16);
}

#[test]
fn absolute_collision_and_paper_overflow_are_not_hidden_or_pushed() {
    let mut d = source();
    table_mut(&mut d, 0).common.vertical_offset = 8000;
    assert!(
        matches!(
            session(&d, 96.),
            Err(HostedTableError::Geometry(GeometryError::Unsupported(
                "stored body side-wrap intersects flow"
            )))
        ),
        "title would cover body lines"
    );
    let mut d = source();
    table_mut(&mut d, 2).common.vertical_offset = 39000;
    assert!(
        matches!(
            session(&d, 96.),
            Err(HostedTableError::Geometry(GeometryError::Unsupported(
                "absolute table requires intact in-paper placement"
            )))
        ),
        "bottom reference would place above paper"
    );
}

#[test]
fn page_reference_is_body_wide_not_the_second_column_or_symmetric_margin() {
    // Synthetic coordinate contract; Page is the whole body, Paper is the actual
    // sheet. Clear body conflicts by keeping this table in the unused band.
    let mut d = source();
    let t = table_mut(&mut d, 2);
    t.common.horz_rel_to = HorzRelTo::Page;
    t.common.vert_rel_to = VertRelTo::Page;
    t.common.vertical_offset = 0;
    // A larger body puts Page/Bottom below all remaining text. This is a fresh
    // budget variant, not a new claim about the fixture's Hancom pagination.
    d.sections[0].section_def.page_def.margin_bottom = 2000;
    let s = session(&d, 96.).unwrap();
    let tree = s.render_page(0).unwrap();
    let ts = tables(&tree.root);
    close(ts[1].bbox.x, 23400. * 96. / 7200.);
    close(ts[1].bbox.y, 35000. * 96. / 7200.);
}

#[test]
fn control_on_a_continuation_line_belongs_to_that_page_not_paragraph_start() {
    // Explicit synthetic raw-stream contract: put the footer control after
    // LINE18 on the next page, inside an already split paragraph. Preserve text
    // offsets; the eight-unit control is appended after the final character.
    let mut d = source();
    let table = table_mut(&mut d, 2).clone();
    d.sections[0].paragraphs[2].controls.clear();
    let p = &mut d.sections[0].paragraphs[3];
    p.controls = vec![Control::Table(Box::new(table))];
    p.char_count += 8;
    let s = session(&d, 96.).unwrap();
    let a = s.render_page(0).unwrap();
    let b = s.render_page(1).unwrap();
    assert_eq!(tables(&a.root).len(), 1);
    assert_eq!(tables(&b.root).len(), 1);
    let footer = tables(&b.root)[0];
    close(footer.bbox.x, 7800. * 96. / 7200.);
    close(footer.bbox.y, 34600. * 96. / 7200.);
    assert_eq!(text(lines(&b.root)[0]), "LINE 16");
}
