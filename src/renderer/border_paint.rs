//! Border stroke primitives shared by paragraph, page and table paint.
//! Table grids, cell ownership and pagination are not handled here.

use crate::model::style::{BorderLine, BorderLineType};
use crate::renderer::render_tree::*;
use crate::renderer::{LineStyle, StrokeDash};

/// 테두리선 Line 노드 생성 (이중선/삼중선 지원)
/// None 타입이면 빈 벡터 반환
pub(crate) fn create_border_line_nodes(
    tree: &mut PageLayoutContext,
    border: &BorderLine,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
) -> Vec<RenderNode> {
    if border.line_type == BorderLineType::None {
        return vec![];
    }

    let base_width = border_width_to_px(border.width);

    match border.line_type {
        BorderLineType::None => vec![],

        // 이중선 (동일 굵기)
        BorderLineType::Double => {
            let total = base_width.max(3.0);
            let sub_w = (total * 0.3).max(0.4);
            let gap = (total * 0.4).max(1.0);
            let offset = (gap + sub_w) / 2.0;
            create_parallel_lines(
                tree,
                border.color,
                x1,
                y1,
                x2,
                y2,
                &[(-offset, sub_w), (offset, sub_w)],
                StrokeDash::Solid,
            )
        }

        // 가는선-굵은선 이중선
        BorderLineType::ThinThickDouble => {
            let total = base_width.max(3.0);
            let thin_w = (total * 0.2).max(0.4);
            let thick_w = (total * 0.4).max(0.6);
            let gap = (total * 0.4).max(1.0);
            let thin_offset = -(gap + thin_w) / 2.0;
            let thick_offset = (gap + thick_w) / 2.0;
            create_parallel_lines(
                tree,
                border.color,
                x1,
                y1,
                x2,
                y2,
                &[(thin_offset, thin_w), (thick_offset, thick_w)],
                StrokeDash::Solid,
            )
        }

        // 굵은선-가는선 이중선
        BorderLineType::ThickThinDouble => {
            let total = base_width.max(3.0);
            let thick_w = (total * 0.4).max(0.6);
            let thin_w = (total * 0.2).max(0.4);
            let gap = (total * 0.4).max(1.0);
            let thick_offset = -(gap + thick_w) / 2.0;
            let thin_offset = (gap + thin_w) / 2.0;
            create_parallel_lines(
                tree,
                border.color,
                x1,
                y1,
                x2,
                y2,
                &[(thick_offset, thick_w), (thin_offset, thin_w)],
                StrokeDash::Solid,
            )
        }

        // 가는선-굵은선-가는선 삼중선
        BorderLineType::ThinThickThinTriple => {
            let total = base_width.max(4.0);
            let thin_w = (total * 0.15).max(0.4);
            let thick_w = (total * 0.3).max(0.6);
            let gap = (total * 0.15).max(0.8);
            let outer_offset = thick_w / 2.0 + gap + thin_w / 2.0;
            create_parallel_lines(
                tree,
                border.color,
                x1,
                y1,
                x2,
                y2,
                &[
                    (-outer_offset, thin_w),
                    (0.0, thick_w),
                    (outer_offset, thin_w),
                ],
                StrokeDash::Solid,
            )
        }

        // 단일선 타입들
        _ => {
            if let Some(dash) = border_line_type_to_dash(border.line_type) {
                create_single_line(tree, border.color, base_width, dash, x1, y1, x2, y2)
            } else {
                vec![]
            }
        }
    }
}

/// 평행선 노드 생성 (이중선/삼중선용)
/// lines: &[(offset, width)] — offset은 선 중심의 수직 이동량
fn create_parallel_lines(
    tree: &mut PageLayoutContext,
    color: u32,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    lines: &[(f64, f64)],
    dash: StrokeDash,
) -> Vec<RenderNode> {
    let is_horizontal = (y2 - y1).abs() < (x2 - x1).abs();
    let mut nodes = Vec::with_capacity(lines.len());

    for &(offset, width) in lines {
        let (lx1, ly1, lx2, ly2) = if is_horizontal {
            (x1, y1 + offset, x2, y2 + offset)
        } else {
            (x1 + offset, y1, x2 + offset, y2)
        };

        let id = tree.next_id();
        let line = LineNode::new(
            lx1,
            ly1,
            lx2,
            ly2,
            LineStyle {
                color,
                width,
                dash,
                ..Default::default()
            },
        );
        let bbox = line.ink_bbox();
        nodes.push(RenderNode::new(id, RenderNodeType::Line(line), bbox));
    }

    nodes
}

/// 임의 방향 평행선 노드 생성 (대각선 이중선/삼중선용)
pub(crate) fn create_parallel_lines_perpendicular(
    tree: &mut PageLayoutContext,
    color: u32,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    lines: &[(f64, f64)],
    dash: StrokeDash,
) -> Vec<RenderNode> {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.01 {
        return vec![];
    }
    let nx = -dy / len;
    let ny = dx / len;
    let mut nodes = Vec::with_capacity(lines.len());

    for &(offset, width) in lines {
        let lx1 = x1 + nx * offset;
        let ly1 = y1 + ny * offset;
        let lx2 = x2 + nx * offset;
        let ly2 = y2 + ny * offset;

        let id = tree.next_id();
        let line = LineNode::new(
            lx1,
            ly1,
            lx2,
            ly2,
            LineStyle {
                color,
                width,
                dash,
                ..Default::default()
            },
        );
        let bbox = line.ink_bbox();
        nodes.push(RenderNode::new(id, RenderNodeType::Line(line), bbox));
    }

    nodes
}

/// 단일선 노드 생성
pub(crate) fn create_single_line(
    tree: &mut PageLayoutContext,
    color: u32,
    width: f64,
    dash: StrokeDash,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
) -> Vec<RenderNode> {
    let id = tree.next_id();
    let line = LineNode::new(
        x1,
        y1,
        x2,
        y2,
        LineStyle {
            color,
            width,
            dash,
            ..Default::default()
        },
    );
    let bbox = line.ink_bbox();
    vec![RenderNode::new(id, RenderNodeType::Line(line), bbox)]
}

/// BorderLine이 시각적으로 차지하는 전체 폭(px).
///
/// `create_border_line_nodes`의 이중선/삼중선 분해 규칙과 같은 값을 써서,
/// 쪽 기준 테두리 박스를 바깥쪽으로 확장할 때 렌더된 선 묶음이 본문 쪽으로
/// 파고들지 않게 한다.
pub(crate) fn border_line_visual_span(border: &BorderLine) -> f64 {
    if border.line_type == BorderLineType::None {
        return 0.0;
    }

    let base_width = border_width_to_px(border.width);
    match border.line_type {
        BorderLineType::Double
        | BorderLineType::ThinThickDouble
        | BorderLineType::ThickThinDouble => base_width.max(3.0),
        BorderLineType::ThinThickThinTriple => base_width.max(4.0),
        _ => base_width,
    }
}

/// 쪽 기준 페이지 테두리를 본문 영역 바깥쪽에 배치할 때 쓰는 보정 폭(px).
///
/// 한컴오피스는 `쪽 기준` 이중선 페이지 테두리에서 저장된 간격값에 선 묶음의
/// 시각 폭을 한 번 더 반영해, 테두리가 본문/객체 쪽으로 파고들지 않게 그린다.
/// 표/문단 테두리의 선 자체 분해 규칙은 그대로 두고, 페이지 테두리 위치 계산에만
/// 이 값을 사용한다.
pub(crate) fn body_page_border_outset(border: &BorderLine) -> f64 {
    const BODY_PAGE_DOUBLE_LINE_OUTSET_FACTOR: f64 = 2.5;
    let span = border_line_visual_span(border);
    match border.line_type {
        BorderLineType::Double
        | BorderLineType::ThinThickDouble
        | BorderLineType::ThickThinDouble
        | BorderLineType::ThinThickThinTriple => span * BODY_PAGE_DOUBLE_LINE_OUTSET_FACTOR,
        _ => span,
    }
}

/// HWP 테두리 굵기 인덱스 → 픽셀 변환 (96dpi 고정)
///
/// [#6913] 한/글은 테두리 굵기를 **1/600 inch 격자에 반올림해서** 그린다. 정본 PDF 의
/// stroke width 는 언제나 `units × 0.12 pt` 로 떨어진다. 격자 계산은 두 단계다 —
/// 선언 mm 를 HWPUNIT(1/7200 inch)으로 반올림한 뒤 그것을 600dpi 로 **half-up**
/// 반올림한다. 한 단계로 `round(mm × 600/25.4)` 를 쓰면 정확히 `.5` 에 걸리는
/// 0.7mm(16.5)와 4.0mm(94.5)에서 1 units 씩 어긋난다.
///
/// 16단계 전부를 단일 변수 실험으로 쟀다 — 같은 문서(`samples/issue6913/…`)의
/// `borderFill 14` 굵기만 바꾼 변형본을 engine 2020 으로 각각 변환해 1쪽 머리 표의
/// stroke width 를 읽었다(안 건드린 0.12mm 칸이 매 변형본에서 0.36pt 로 남아
/// 통제군이 된다). **16/16 일치.**
///
/// ```text
///   선언 mm   정본 pt   ÷0.12   격자 units   px@96
///     0.1      0.240      2            2             0.32
///     0.12     0.360      3            3             0.48
///     0.15     0.480      4            4             0.64
///     0.2      0.600      5            5             0.80
///     0.25     0.720      6            6             0.96
///     0.3      0.840      7            7             1.12
///     0.4      1.079      9            9             1.44
///     0.5      1.439     12           12             1.92
///     0.6      1.679     14           14             2.24
///     0.7      2.039     17           17             2.72
///     1.0      2.878     24           24             3.84
///     1.5      4.198     35           35             5.60
///     2.0      5.637     47           47             7.52
///     3.0      8.515     71           71            11.36
///     4.0     11.394     95           95            15.20
///     5.0     14.152    118          118            18.88
/// ```
///
/// 종전 표는 `mm × 96/25.4` 를 소수 첫째자리로 반올림한 값이라 격자를 못 맞췄다.
/// 특히 얇은 쪽이 크게 틀렸다 — 0.1mm 는 0.4px 로 **25% 두꺼웠고**, 0.2mm 는
/// 0.75px 로 6% 얇았다.
pub(crate) fn border_width_to_px(width: u8) -> f64 {
    /// 한/글이 굵기를 반올림하는 격자 — 1/600 inch.
    const GRID_DPI: f64 = 600.0;
    /// 산출 dpi. 이 함수는 96dpi 고정 표다.
    const OUT_DPI: f64 = 96.0;
    const WIDTHS_MM: [f64; 16] = [
        0.1, 0.12, 0.15, 0.2, 0.25, 0.3, 0.4, 0.5, 0.6, 0.7, 1.0, 1.5, 2.0, 3.0, 4.0, 5.0,
    ];
    const WIDTHS_PX: [f64; 16] = [
        0.32,  // 0: 0.1mm   → 2 units
        0.48,  // 1: 0.12mm  → 3
        0.64,  // 2: 0.15mm  → 4
        0.8,   // 3: 0.2mm   → 5
        0.96,  // 4: 0.25mm  → 6
        1.12,  // 5: 0.3mm   → 7
        1.44,  // 6: 0.4mm   → 9
        1.92,  // 7: 0.5mm   → 12
        2.24,  // 8: 0.6mm   → 14
        2.72,  // 9: 0.7mm   → 17
        3.84,  // 10: 1.0mm  → 24
        5.6,   // 11: 1.5mm  → 35
        7.52,  // 12: 2.0mm  → 47
        11.36, // 13: 3.0mm  → 71
        15.2,  // 14: 4.0mm  → 95
        18.88, // 15: 5.0mm  → 118
    ];
    debug_assert!(
        WIDTHS_MM.iter().zip(WIDTHS_PX.iter()).all(|(mm, px)| {
            // mm → HWPUNIT(1/7200 inch) → 600dpi 격자. 두 번째 반올림은 half-up
            // 이어야 0.7mm(16.5)와 4.0mm(94.5)가 맞는다.
            let hwpunit = (mm * 7200.0 / 25.4).round();
            let units = (hwpunit * GRID_DPI / 7200.0 + 0.5).floor();
            (units * OUT_DPI / GRID_DPI - px).abs() < 1e-9
        }),
        "WIDTHS_PX 는 mm → HWPUNIT → 600dpi 격자(half-up) × 96/600 이어야 한다"
    );
    if let Some(&px) = WIDTHS_PX.get(width as usize) {
        px
    } else {
        (width as f64 * 1.2).max(0.4).min(20.0)
    }
}

/// BorderLineType → StrokeDash 변환 (None이면 None 반환)
pub(crate) fn border_line_type_to_dash(lt: BorderLineType) -> Option<StrokeDash> {
    match lt {
        BorderLineType::None => None,
        BorderLineType::Solid => Some(StrokeDash::Solid),
        BorderLineType::Dash | BorderLineType::LongDash => Some(StrokeDash::Dash),
        BorderLineType::Dot | BorderLineType::Circle => Some(StrokeDash::Dot),
        BorderLineType::DashDot => Some(StrokeDash::DashDot),
        BorderLineType::DashDotDot => Some(StrokeDash::DashDotDot),
        _ => Some(StrokeDash::Solid), // Double, Wave 등은 Solid로 대체
    }
}
