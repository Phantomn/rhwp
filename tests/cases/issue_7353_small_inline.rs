//! Small inline objects retain the paragraph's character-height strut.
//! Normal Hancom save/PDF, including asymmetric outer margins, is independent
//! of the implementation. Print coordinates are quantized to 12HU.
use rhwp::{
    model::{control::Control, document::Document},
    renderer::{
        render_tree::{RenderNode, RenderNodeType},
        table_v2::*,
    },
};
fn source() -> Document {
    rhwp::parse_document(include_bytes!(
        "../fixtures/issue7353_small_inline_review/expanded.hwp"
    ))
    .unwrap()
}
fn nodes(n: &RenderNode) -> Vec<&RenderNode> {
    std::iter::once(n)
        .chain(n.children.iter().flat_map(nodes))
        .collect()
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}
#[test]
fn saved_small_and_tall_lines_share_occupancy_and_final_endpoints() {
    let d = source();
    let before = format!("{d:?}");
    for dpi in [96., 192.] {
        let u = dpi / 7200.;
        let s =
            HostedSectionSession::from_document(&d, 0, dpi, CellEndPolicy::OmitFinalParagraphGap)
                .unwrap();
        assert_eq!(s.pagination().pages.len(), 1);
        let tree = s.render_page(0).unwrap();
        let all = nodes(&tree.root);
        let lines: Vec<_> = all
            .iter()
            .filter(|n| matches!(n.node_type, RenderNodeType::Line(_)))
            .collect();
        assert_eq!(lines.len(), 8);
        // Independent PDF path starts (pt), from expanded-2020.pdf. Its print
        // matrix scales y by 1.0015 and quantizes coordinates to 12HU.
        let pdf_y = [
            84.606995, 132.437988, 180.990997, 255.983002, 303.815002, 377.726013, 432.528015,
            483.484009,
        ];
        // Vertical positions from saved rows; object offset from baseline
        // alignment of the OUTER box against the 1100/2000HU character strut.
        for (i, (pi, dy, h)) in [
            (2, 931.6, 4.),
            (5, 425., 600.),
            (8, 0., 2400.),
            (11, 900., 600.),
            (14, 0., 600.),
            (17, 1696.6, 4.),
            (20, 455., 600.),
            (23, 255., 600.),
        ]
        .into_iter()
        .enumerate()
        {
            let r = &d.sections[0].paragraphs[pi].line_segs[0];
            let y = (4000. + f64::from(r.vertical_pos) + dy) * u;
            let n = lines[i];
            near(n.bbox.x, 2400. * u);
            near(n.bbox.y, y);
            near(n.bbox.height, h * u);
            assert!((n.bbox.y * 72. / dpi * 1.0015 - pdf_y[i]).abs() < 0.13);
            let RenderNodeType::Line(l) = &n.node_type else {
                panic!()
            };
            near(l.x1, n.bbox.x);
            near(l.y1, y);
            near(l.x2, 20400. * u);
            near(l.y2, y + h * u);
            let owner = all
                .iter()
                .find(|q| q.children.iter().any(|c| c.id == n.id))
                .unwrap();
            near(owner.bbox.height, f64::from(r.line_height) * u);
            assert!(
                n.bbox.y >= owner.bbox.y
                    && n.bbox.y + n.bbox.height <= owner.bbox.y + owner.bbox.height + 1e-7
            );
            let next = &d.sections[0].paragraphs[pi + 1].line_segs[0];
            assert!(all
                .iter()
                .any(|q| matches!(q.node_type, RenderNodeType::TextLine(_))
                    && (q.bbox.y - (4000. + f64::from(next.vertical_pos)) * u).abs() < 1e-7));
        }
    }
    assert_eq!(format!("{d:?}"), before);
}
#[test]
fn incompatible_cache_is_not_stretched_to_admit_a_line() {
    for delta in [-20, 20] {
        let mut d = source();
        let r = &mut d.sections[0].paragraphs[2].line_segs[0];
        r.line_height += delta;
        r.text_height += delta;
        assert!(HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()).is_err());
    }
}
#[test]
fn unsupported_rotations_remain_rejected() {
    let mut d = source();
    let Control::Shape(sh) = &mut d.sections[0].paragraphs[2].controls[0] else {
        panic!()
    };
    let rhwp::model::shape::ShapeObject::Line(l) = sh.as_mut() else {
        panic!()
    };
    l.drawing.shape_attr.rotation_angle = 20;
    assert!(HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::default()).is_err());
}

#[test]
fn cell_fit_reserves_the_line_not_just_the_small_object_and_translates_paint() {
    use rhwp::model::table::{Cell, Table};
    use rhwp::renderer::{render_tree::PageRenderTree, style_resolver::resolve_styles};
    let d = source();
    // Synthetic cell adapter variation of the independently saved row.
    let mut para = d.sections[0].paragraphs[2].clone();
    para.line_segs[0].vertical_pos = 0;
    let t = Table {
        row_count: 1,
        col_count: 1,
        cells: vec![Cell {
            width: 30000,
            apply_inner_margin: true,
            paragraphs: vec![para],
            row_span: 1,
            col_span: 1,
            ..Default::default()
        }],
        ..Default::default()
    };
    let prepared = PreparedTextTable::prepare_with_end_policy(
        &t,
        &resolve_styles(&d.doc_info, 96.),
        96.,
        &[],
        CellEndPolicy::OmitFinalParagraphGap,
    )
    .unwrap();
    let area = |h| PageArea {
        bounds: Rect {
            x: 20.,
            y: 30.,
            width: 400.,
            height: h,
        },
    };
    assert!(matches!(
        prepared.start().fit(area(1099. / 75.)).unwrap(),
        TextFragmentFit::DoesNotFit { .. }
    ));
    let f = match prepared.start().fit(area(1100. * (96. / 7200.))).unwrap() {
        TextFragmentFit::Placed(f) => f,
        TextFragmentFit::DoesNotFit {
            required_width,
            required_height,
        } => panic!("required {required_width} x {required_height}"),
        TextFragmentFit::Complete => panic!("already complete"),
    };
    near(f.geometry().reserved_height(), 1100. / 75.);
    let mut tree = PageRenderTree::new(0, 600., 600.);
    f.append_to(&mut tree).unwrap();
    let all = nodes(&tree.root);
    let n = all
        .into_iter()
        .find(|n| matches!(n.node_type, RenderNodeType::Line(_)))
        .unwrap();
    let RenderNodeType::Line(l) = &n.node_type else {
        panic!()
    };
    near(n.bbox.y, 30. + 931.6 / 75.);
    near(l.y1, n.bbox.y);
    near(l.x1, n.bbox.x);
    assert!(matches!(
        f.continuation().fit(area(100.)).unwrap(),
        TextFragmentFit::Complete
    ));
}

#[test]
fn multiple_small_objects_use_one_character_strut_and_keep_both_owners() {
    let mut d = source();
    let p = &mut d.sections[0].paragraphs[2];
    let Control::Shape(s) = &mut p.controls[0] else {
        panic!()
    };
    s.common_mut().width = 6000;
    let mut second = p.controls[0].clone();
    let Control::Shape(s) = &mut second else {
        panic!()
    };
    s.common_mut().height = 600;
    p.controls.push(second);
    p.char_count = 17;
    let session =
        HostedSectionSession::from_document(&d, 0, 96., CellEndPolicy::OmitFinalParagraphGap)
            .unwrap();
    let tree = session.render_page(0).unwrap();
    let all = nodes(&tree.root);
    let line = all
        .iter()
        .find(|n| {
            n.children
                .iter()
                .filter(|c| matches!(c.node_type, RenderNodeType::Line(_)))
                .count()
                == 2
        })
        .unwrap();
    near(line.bbox.height, 1100. / 75.);
    let first = &line.children[0];
    let second = &line.children[1];
    near(first.bbox.y, line.bbox.y + 931.6 / 75.);
    near(second.bbox.y, line.bbox.y + 425. / 75.);
    near(second.bbox.x, first.bbox.x + 6000. / 75.);
}
