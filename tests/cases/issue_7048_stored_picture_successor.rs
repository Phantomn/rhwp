//! 저장 그림 하단과 다음 저장 글줄의 경계 계약을 전체 원본에서 검증한다.
//! 한컴2020 PDF 물리16쪽 본문 기준선은96DPI에서974.72/1020.32px다.
//! 축소본의 렌더링 회귀는 피델리티 미달로 #7445에 이관했다.
#![cfg(not(target_arch = "wasm32"))]

use rhwp::document_core::DocumentCore;
use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};

const FULL: &str = "samples/issue6782/1480000-201900042-chemical-product-labeling-study.hwp";
fn bytes(path: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap()
}
fn runs<'a>(node: &'a RenderNode, out: &mut Vec<&'a RenderNode>) {
    if matches!(node.node_type, RenderNodeType::TextRun(_)) {
        out.push(node);
    }
    for child in &node.children {
        runs(child, out);
    }
}
fn check_sample(path: &str) {
    let core = DocumentCore::from_bytes(&bytes(path)).unwrap();
    let mut found = false;
    for page in 0..core.page_count() {
        let tree = core.build_page_render_tree(page).unwrap();
        let mut nodes = Vec::new();
        runs(&tree.root, &mut nodes);
        if !nodes.iter().any(|node| {
            matches!(&node.node_type,
            RenderNodeType::TextRun(run) if run.text == "최종 표시도안(예시)")
        }) {
            continue;
        }
        let target = nodes.iter().find(|node| {
            matches!(&node.node_type,
            RenderNodeType::TextRun(run) if run.text.contains("규제영향분석서 작성"))
        });
        let Some(target) = target else {
            continue;
        };
        found = true;
        let RenderNodeType::TextRun(run) = &target.node_type else {
            unreachable!()
        };
        assert!(
            (target.bbox.y + run.baseline - 1020.32).abs() < 1.0,
            "Hancom PDF baseline: {:?}, baseline {}",
            target.bbox,
            run.baseline
        );
        let preceding = nodes.iter().find(|node| matches!(&node.node_type,
            RenderNodeType::TextRun(run) if run.text.contains("안전확인대상생활화학제품 및 살생물제품 표시기준"))).unwrap();
        let RenderNodeType::TextRun(run) = &preceding.node_type else {
            unreachable!()
        };
        assert!((preceding.bbox.y + run.baseline - 974.72).abs() < 1.0);
    }
    assert!(found, "content-matched PDF page must exist");
}
#[test]
fn issue_7048_full_original_body_baselines_match_hancom() {
    check_sample(FULL);
}

#[test]
fn issue_7048_table_first_fragment_keeps_the_footer_clear() {
    fn tables<'a>(node: &'a RenderNode, out: &mut Vec<&'a RenderNode>) {
        if matches!(&node.node_type, RenderNodeType::Table(table)
            if table.section_index == Some(2) && table.para_index == Some(162))
        {
            out.push(node);
        }
        for child in &node.children {
            tables(child, out);
        }
    }
    for path in [FULL] {
        let core = DocumentCore::from_bytes(&bytes(path)).unwrap();
        let mut fragment_count = 0;
        for page in 0..core.page_count() {
            let tree = core.build_page_render_tree(page).unwrap();
            let mut fragments = Vec::new();
            tables(&tree.root, &mut fragments);
            for table in fragments {
                if fragment_count == 0 {
                    let last_row = table
                        .children
                        .iter()
                        .filter_map(|cell| match &cell.node_type {
                            RenderNodeType::TableCell(c) => Some(c.row + c.row_span - 1),
                            _ => None,
                        })
                        .max()
                        .unwrap();
                    assert_eq!(last_row, 21, "Hancom PDF 57 ends before 품목별 특성 row");
                    assert!(
                        (table.bbox.y + table.bbox.height - 1006.6).abs() < 1.0,
                        "Hancom first fragment bottom: {:?}",
                        table.bbox
                    );
                }
                fragment_count += 1;
            }
        }
        assert_eq!(fragment_count, 2);
    }
}

#[test]
fn issue_7048_stored_picture_does_not_rewind_across_measured_picture_flow() {
    fn collect<'a>(node: &'a RenderNode, out: &mut Vec<&'a RenderNode>) {
        out.push(node);
        for child in &node.children {
            collect(child, out);
        }
    }
    for path in [FULL] {
        let core = DocumentCore::from_bytes(&bytes(path)).unwrap();
        let mut found = false;
        for page in 0..core.page_count() {
            let tree = core.build_page_render_tree(page).unwrap();
            let mut nodes = Vec::new();
            collect(&tree.root, &mut nodes);
            let image = nodes.iter().find(|node| matches!(&node.node_type,
                RenderNodeType::Image(image) if image.section_index == Some(4) && image.para_index == Some(209)));
            let Some(image) = image else {
                continue;
            };
            let heading = nodes.iter().find(|node| matches!(&node.node_type,
                RenderNodeType::TextRun(run) if run.para_index == Some(208) && run.text.contains("미만인 제품")))
                .expect("the preceding heading must remain on the picture page");
            assert!(
                image.bbox.y >= heading.bbox.y + heading.bbox.height,
                "stored picture must not cover measured-flow heading: {:?} / {:?}",
                image.bbox,
                heading.bbox
            );
            found = true;
        }
        assert!(
            found,
            "full and reduced fixtures must preserve the figure section"
        );
    }
}
