//! Shared-host stored Square contracts. Normal source/PDF provenance and HU
//! expectations: issue7353_{host,body}_wrap_review/README.md. Mutations are
//! synthetic boundary checks, not independent Hancom output.
use rhwp::{
    model::{control::Control, document::Document},
    renderer::{
        render_tree::{RenderNode, RenderNodeType},
        table_v2::{CellEndPolicy, HostedSectionSession},
    },
};

fn doc(host: bool) -> Document {
    let path = if host {
        "host_wrap_review/host"
    } else {
        "body_wrap_review/wrap"
    };
    rhwp::parse_document(
        &std::fs::read(format!(
            "{}/tests/fixtures/issue7353_{path}-saved.hwp",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    )
    .unwrap()
}
fn open(d: &Document, dpi: f64) -> Result<HostedSectionSession, String> {
    HostedSectionSession::from_document(d, 0, dpi, CellEndPolicy::OmitFinalParagraphGap)
        .map_err(|e| e.to_string())
}
fn all(n: &RenderNode) -> Vec<&RenderNode> {
    std::iter::once(n)
        .chain(n.children.iter().flat_map(all))
        .collect()
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}
fn page_height(d: &mut Document, height: u32) {
    d.sections[0].section_def.page_def.height = height;
    let def = d.sections[0].section_def.clone();
    for c in &mut d.sections[0].paragraphs[0].controls {
        if let Control::SectionDef(v) = c {
            **v = def.clone();
        }
    }
}
fn rejected(d: &Document, reason: &str) {
    let e = open(d, 96.)
        .err()
        .expect("unsupported geometry must not publish a session");
    assert!(e.contains(reason), "{e}");
}

#[test]
fn normal_hwp_and_hwpx_keep_side_lanes_blank_owner_footer_and_full_width_return() {
    for host in [false, true] {
        let original = doc(host);
        let roundtrip =
            rhwp::parse_document(&rhwp::serializer::serialize_hwpx(&original).unwrap()).unwrap();
        for d in [&original, &roundtrip] {
            let snapshot = format!("{:?}", d.sections);
            for dpi in [96., 192.] {
                let s = open(d, dpi).unwrap();
                assert_eq!(s.pagination().pages.len(), 1);
                let tree = s.render_page(0).unwrap();
                let nodes = all(&tree.root);
                let line = |pi, li| {
                    *nodes
                        .iter()
                        .find(|n| {
                            matches!(&n.node_type,
                    RenderNodeType::TextLine(l) if l.para_index==Some(pi) && l.line_index==Some(li))
                        })
                        .unwrap()
                };
                let table = nodes
                    .iter()
                    .find(|n| {
                        matches!(&n.node_type,
                    RenderNodeType::Table(t) if t.para_index==Some(1))
                    })
                    .unwrap();
                let u = dpi / 7200.;
                let owner = if host { 6764. } else { 4696. };
                let offset = if host { 498. } else { 834. };
                near(line(1, 0).bbox.y, (5668. + owner) * u);
                near(line(1, 0).bbox.height, 800. * u);
                near(table.bbox.x, (5669. + 26319. + 1417.) * u);
                near(table.bbox.y, (5668. + owner + offset + 138.) * u);
                near(table.bbox.width, 21022. * u);
                near(table.bbox.height, 13241. * u);
                let count = if host { 5 } else { 7 };
                for i in 0..count {
                    near(
                        line(2, i).bbox.y,
                        (5668. + owner + 1200. + i as f64 * 2252.) * u,
                    );
                    near(
                        line(2, i).bbox.width,
                        if !host && i == 6 {
                            48188. * u
                        } else {
                            26319. * u
                        },
                    );
                }
                for pi in [2, if host { 4 } else { 3 }] {
                    let text: String = nodes
                        .iter()
                        .filter_map(|n| match &n.node_type {
                            RenderNodeType::TextRun(r) if r.para_index == Some(pi) => {
                                Some(r.text.as_str())
                            }
                            _ => None,
                        })
                        .collect();
                    assert_eq!(text, d.sections[0].paragraphs[pi].text);
                }
                assert_eq!(
                    nodes
                        .iter()
                        .filter(
                            |n| matches!(&n.node_type,RenderNodeType::TextRun(r) if r.text=="- 1 -")
                        )
                        .count(),
                    1
                );
                assert_eq!(
                    s.render_page_json(0).unwrap(),
                    s.render_page_json(0).unwrap()
                );
            }
            assert_eq!(snapshot, format!("{:?}", d.sections));
        }
    }
}

#[test]
fn synthetic_blank_and_visible_host_collision_is_rejected() {
    for visible in [false, true] {
        let mut d = doc(true);
        if visible {
            d.sections[0].paragraphs[1].text = "HOST".into();
            d.sections[0].paragraphs[1]
                .controls
                .retain(|c| !matches!(c, Control::PageNumberPos(_)));
            // Re-encode this synthetic text/control edit so character offsets
            // describe HOST rather than the source's empty control carrier.
            d = rhwp::parse_document(&rhwp::serializer::serialize_hwpx(&d).unwrap()).unwrap();
            assert!(open(&d, 96.).is_ok(), "visible safe lane control");
        }
        d.sections[0].paragraphs[1].line_segs[0].segment_width = 48188;
        rejected(&d, "side-wrap intersects flow");
    }
}

#[test]
fn synthetic_following_line_and_other_table_cannot_enter_exclusion() {
    let mut d = doc(true);
    d.sections[0].paragraphs[2].line_segs[0].segment_width = 48188;
    rejected(&d, "side-wrap intersects flow");
    let mut d = doc(true);
    // An identical second object overlaps, though both owners fit their lanes.
    let owner = d.sections[0].paragraphs[1].clone();
    d.sections[0].paragraphs.insert(2, owner);
    rejected(&d, "side-wrap intersects flow");
}

#[test]
fn synthetic_exact_box_margin_budget_and_terminal_occupancy() {
    for (height, fits) in [(32116, true), (32114, false)] {
        let mut d = doc(true);
        page_height(&mut d, height);
        if fits {
            assert_eq!(open(&d, 96.).unwrap().pagination().pages.len(), 1);
        } else {
            rejected(&d, "side-wrap across pages");
        }
    }
    // Story ends before the floating bottom. Column used_height must still
    // include the object and its bottom margin without fabricating a blank page.
    let mut d = doc(true);
    d.sections[0].paragraphs.truncate(2);
    let s = open(&d, 96.).unwrap();
    assert_eq!(s.pagination().pages.len(), 1);
    near(
        s.pagination().pages[0].column_contents[0].used_height,
        (6764. + 498. + 138. + 13241. + 138.) / 75.,
    );
}

#[test]
fn synthetic_complete_visible_host_is_allowed_but_split_host_is_rejected() {
    let mut d = doc(true);
    let table = d.sections[0].paragraphs[1].controls.remove(1);
    d.sections[0].paragraphs[2].controls.push(table);
    assert_eq!(open(&d, 96.).unwrap().pagination().pages.len(), 1);
    page_height(&mut d, 29336);
    rejected(&d, "side-wrap across pages");
}

#[test]
fn synthetic_exclusion_ends_at_physical_page_boundary() {
    let mut d = doc(true);
    d.sections[0].paragraphs[2].column_type = rhwp::model::paragraph::ColumnBreakType::Page;
    for row in &mut d.sections[0].paragraphs[2].line_segs {
        row.segment_width = 48188;
    }
    let s = open(&d, 96.).unwrap();
    assert_eq!(s.pagination().pages.len(), 2);
    near(
        s.pagination().pages[0].column_contents[0].used_height,
        (6764. + 498. + 138. + 13241. + 138.) / 75.,
    );
    let p = s.render_page(1).unwrap();
    let line = all(&p.root)
        .into_iter()
        .find(|n| matches!(&n.node_type,RenderNodeType::TextLine(l) if l.para_index==Some(2)))
        .unwrap();
    near(line.bbox.y, 5668. / 75.);
    near(line.bbox.width, 48188. / 75.);
}
