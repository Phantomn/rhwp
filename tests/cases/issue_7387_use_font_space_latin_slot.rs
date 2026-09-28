//! #7387: `CharShape.useFontSpace` 가 켜진 run 의 공백은 반각이 아니라
//! **영문 슬롯 글꼴이 선언한 공백 전진폭**이다.
//!
//! 종전에는 `useFontSpace` 를 렌더러가 아예 읽지 않아 공백이 언제나 `em/2` 였다.
//!
//! # 왜 절대 좌표로 재는가
//!
//! 이 검사는 `em` 으로 정규화하지 않는다. 앞선 PR(#7389)은 `exam_eng` 를 앵커로
//! 잡고 `폭 / 글꼴크기` 를 정본의 `폭 / 글꼴크기` 와 비교했는데, **그 문서는 정본과
//! rhwp 의 글꼴 크기가 6.43% 다르다**(#7398). 분모가 서로 달라, 고쳐진 뒤에도 남아 있던
//! 5.4% 폭 부족이 상쇄되어 "일치"로 보였다. 그래서 그 PR 을 닫았다.
//!
//! 여기서는 글꼴 크기 confound 가 없는 문서(`scripts/oracle_comparability.py` 판정
//! `fontScale` 0.996)를 앵커로 쓰고, **px 절대 폭**을 본다.
//!
//! # 기대값
//!
//! 한컴 정본 `pdf/hwpctl_API_v2.4-2022.pdf` 를 `mutool draw -F stext` 로 읽어, 같은
//! 글자열 줄의 첫 글자 `x0` 부터 마지막 글자 `x1` 까지를 96dpi px 로 잰 값이다.
//!
//! ```text
//!   쪽   정본 잉크폭   수정 전 점유폭   수정 후 점유폭
//!   14      416.66      432.50 (+15.8)   416.60 (-0.1)
//!   49      430.42      454.20 (+23.8)   432.30 (+1.9)
//!   88      302.42      322.10 (+19.7)   306.20 (+3.8)
//! ```
//!
//! rhwp 쪽은 run 의 **점유폭**이라 마지막 글자의 전진폭까지 포함한다 — 정본의 잉크폭보다
//! 한 글자 몫 안쪽에서 조금 크다. 그래서 등식이 아니라 `±8px` 구간으로 판정한다.
//! 수정 전 값은 전부 그 구간 위에 있다.
use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};
use rhwp::DocumentCore;

struct Run {
    text: String,
    x: f64,
    width: f64,
    y: f64,
    font_size: f64,
}

fn collect(node: &RenderNode, out: &mut Vec<Run>) {
    if let RenderNodeType::TextRun(run) = &node.node_type {
        out.push(Run {
            text: run.text.clone(),
            x: node.bbox.x,
            width: node.bbox.width,
            y: node.bbox.y,
            font_size: run.style.font_size,
        });
    }
    for child in &node.children {
        collect(child, out);
    }
}

fn open(name: &str) -> DocumentCore {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("samples/{name}"));
    DocumentCore::from_bytes(&std::fs::read(path).expect("공개 회귀 문서")).expect("문서 파싱")
}

fn strip(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

/// 같은 baseline 의 run 을 모아 글자열이 `needle` 인 줄의 점유폭과 쪽 너비를 낸다.
fn line_span(doc: &DocumentCore, page: u32, needle: &str) -> Option<(f64, f64)> {
    let tree = doc.build_page_render_tree(page - 1).ok()?;
    let mut runs = Vec::new();
    collect(&tree.root, &mut runs);
    let mut baselines: Vec<f64> = runs.iter().map(|r| r.y).collect();
    baselines.sort_by(|a, b| a.partial_cmp(b).unwrap());
    baselines.dedup_by(|a, b| (*a - *b).abs() < 0.5);
    for baseline in baselines {
        let line: Vec<&Run> = runs
            .iter()
            .filter(|r| (r.y - baseline).abs() < 0.5)
            .collect();
        let joined: String = {
            let mut sorted: Vec<&&Run> = line.iter().collect();
            sorted.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
            strip(&sorted.iter().map(|r| r.text.as_str()).collect::<String>())
        };
        if joined == needle {
            let left = line.iter().fold(f64::MAX, |a, r| a.min(r.x));
            let right = line.iter().fold(f64::MIN, |a, r| a.max(r.x + r.width));
            return Some((right - left, tree.root.bbox.width));
        }
    }
    None
}

/// 반례 가드 — 규칙은 상수가 아니라 **영문 슬롯 글꼴의 값**이다.
///
/// `1382000_domestic_violence_survey.hwp` 의 charPr 26 은 `useFontSpace="1"` 인데
/// 영문 슬롯이 `휴먼명조`(대체 없이 그대로 쓰인다)이고 그 글꼴의 공백은 256/512 =
/// 0.5 em 이다. 정본 p19 에서 이 charPr 의 공백은 n=38 관측 최빈 **0.489 em** 으로,
/// 같은 쪽 다른 `useFontSpace="1"` charPr 들의 0.337 em 과 뚜렷이 갈린다.
///
/// 이 검사는 수정 전에도 통과한다(수정 전 391.40px / 수정 후 392.00px) — 결함 검출
/// 증거가 아니라, 뒤에 이 갈래를 상수로 눌러 버리는 변경을 막는 경계다. 공백 6개짜리
/// 줄이라 0.25 em 으로 누르면 `6 × 0.25 × 14.667px = 22px` 가 빠져 약 370px 이 되고
/// 아래 구간을 벗어난다.
#[test]
fn latin_slot_with_half_em_space_keeps_half_width() {
    let doc = open("task2430/1382000_domestic_violence_survey.hwp");
    // 줄 앞의 `7.` 은 문단 자동 번호라 별도 run 이다. 줄 전체를 짝짓는다.
    let needle = "7.만일이연구에참여하지않는다면불이익이있습니까?";
    let (span, _) = line_span(&doc, 17, needle).expect("17쪽의 7번 문항 줄");
    assert!(
        (382.0..402.0).contains(&span),
        "17쪽 7번 문항 점유폭 {span:.2}px 이 382~402px 밖이다. \
         영문 슬롯이 0.5 em 공백을 선언한 run 까지 좁히면 안 된다.",
    );
}
