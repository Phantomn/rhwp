//! Shared paint-tree operations. Only resolved nodes are consumed; no table
//! measurement, pagination, paragraph anchoring or source-model reflow.
use super::page_layout::PageLayoutInfo;
use super::render_tree::{BoundingBox, LineNode, PageLayoutContext, RenderNode, RenderNodeType};
use super::{LineStyle, StrokeDash};
use crate::model::shape::TextWrap;

/// Shared paint/hit-test order: text-wrap plane, z-order, then the source
/// document path. Node allocation IDs and packed stable_index are not source
/// ownership; positionless decorations retain stable insertion order.
pub(crate) fn paper_node_sort_key(node: &RenderNode) -> (u8, i32, super::render_tree::DocPath) {
    let plane = match node.layer.and_then(|layer| layer.text_wrap) {
        Some(TextWrap::BehindText) => 1,
        Some(TextWrap::InFrontOfText) => 3,
        _ => 2,
    };
    let z_order = node.layer.map(|layer| layer.z_order).unwrap_or(0);
    let doc_path = super::render_tree::doc_path_for_node(node).unwrap_or_default();
    (plane, z_order, doc_path)
}

pub(crate) fn append_column_separators(
    tree: &mut PageLayoutContext,
    body_node: &mut RenderNode,
    zone_layout: &PageLayoutInfo,
    y_start: f64,
    y_end: f64,
) {
    // [Task #1333 v2] 콘텐츠 높이(y_end)가 body 영역 하단을 넘으면 하단에서 자른다.
    // 꽉 찬 페이지에서 prev_zone_y_end 가 trailing 간격 등으로 body 를 초과해 구분선이
    // 페이지 밖까지 그려지던 결함(예: 대상문서 p22 105%) 정정. 부분 페이지(콘텐츠 < body
    // 하단)와 sub-page zone 은 영향 없음.
    let body_bottom = zone_layout.body_area.y + zone_layout.body_area.height;
    let y_end = y_end.min(body_bottom);
    if zone_layout.column_areas.len() < 2 || zone_layout.separator_type == 0 || y_end <= y_start {
        return;
    }
    let line_width = super::layout::border_width_to_px(zone_layout.separator_width).max(0.5);
    let dash = match zone_layout.separator_type {
        2 => StrokeDash::Dash,
        3 => StrokeDash::Dot,
        4 => StrokeDash::DashDot,
        5 => StrokeDash::DashDotDot,
        6 => StrokeDash::Dash,
        7 => StrokeDash::Dot,
        _ => StrokeDash::Solid,
    };
    for i in 0..zone_layout.column_areas.len() - 1 {
        let left = &zone_layout.column_areas[i];
        let right = &zone_layout.column_areas[i + 1];
        let (left, right) = if left.x <= right.x {
            (left, right)
        } else {
            (right, left)
        };
        let sep_x = (left.x + left.width + right.x) / 2.0;
        let sep_id = tree.next_id();
        let sep_line = LineNode::new(
            sep_x,
            y_start,
            sep_x,
            y_end,
            LineStyle {
                color: zone_layout.separator_color,
                width: line_width,
                dash,
                ..Default::default()
            },
        );
        let sep_bbox = sep_line.ink_bbox();
        let sep_node = RenderNode::new(sep_id, RenderNodeType::Line(sep_line), sep_bbox);
        body_node.children.push(sep_node);
    }
}

pub(crate) fn set_body_clip(body: &mut RenderNode, page_height: f64) {
    let body_bbox = body.bbox;
    // [#3127] clip 하방 확장은 콘텐츠 종류로 갈린다.
    //
    // - 흐름 콘텐츠(표/문단/셀 등)는 body_area 를 넘겨 배치돼도 잘리면 안 된다.
    //   (예: body 바닥 아래로 늘어난 표 마지막 셀의 용지 규격 줄
    //   `210mm×297mm(백상지 ㎡)`. 브라우저는 clip 밖도 그리지만 svg2pdf 는
    //   엄격히 잘라 PDF 에서 소실됐다.)
    // - 부동 그림/도형은 body_bottom+10 상한을 유지한다 — 대형 부동 그림이
    //   꼬리말 영역까지 clip 을 확장하던 Task #460 회귀를 막기 위해서다.
    //
    // 그래서 흐름 콘텐츠 bbox 는 상한 없이 반영하고, 부동 그림 bbox 는 상한
    // 적용분과 별도로 모아 두 결과를 합친다.
    fn is_floating_object(node: &RenderNode) -> bool {
        matches!(
            node.node_type,
            RenderNodeType::Image(_)
                | RenderNodeType::Group(_)
                | RenderNodeType::Path(_)
                | RenderNodeType::Ellipse(_)
                | RenderNodeType::Rectangle(_)
                | RenderNodeType::Line(_)
                | RenderNodeType::TextBox
                | RenderNodeType::Placeholder(_)
                | RenderNodeType::RawSvg(_)
        )
    }
    // clip 을 **가시** 자식 bbox 로 확장. `float_subtree` 가 참이면 그 서브트리는
    // 부동 그림으로 취급해 상한 적용 대상 clip 만 넓힌다.
    //
    // TableCell 자체가 clip이면 그 cell 밖의 자손은 현재 PageRenderTree에는
    // 존재해도 이전/다음 페이지용 연속 흐름일 뿐 paint되지 않는다. 그 tail을
    // body clip 확장에 재귀 반영하면 Canvas/WASM이 물리 쪽 밖을 재생할 수 있고,
    // SVG의 cell clip과도 의미가 달라진다(42065 RowBreak 1×1 중첩 표).
    fn expand_clip(
        flow: &mut BoundingBox,
        float: &mut BoundingBox,
        node: &RenderNode,
        float_subtree: bool,
    ) {
        let cb = &node.bbox;
        let is_float = float_subtree || is_floating_object(node);
        let target: &mut BoundingBox = if is_float { &mut *float } else { &mut *flow };
        let child_bottom = cb.y + cb.height;
        let child_right = cb.x + cb.width;
        if child_bottom > target.y + target.height {
            target.height = child_bottom - target.y;
        }
        if child_right > target.x + target.width {
            target.width = child_right - target.x;
        }
        if cb.x < target.x {
            target.width += target.x - cb.x;
            target.x = cb.x;
        }
        if cb.y < target.y {
            target.height += target.y - cb.y;
            target.y = cb.y;
        }
        let clips_descendants = matches!(
            node.node_type,
            RenderNodeType::TableCell(ref cell) if cell.clip
        );
        if !clips_descendants {
            for child in &node.children {
                expand_clip(flow, float, child, is_float);
            }
        }
    }
    let mut flow_clip = body_bbox;
    let mut float_clip = body_bbox;
    for child in &body.children {
        expand_clip(&mut flow_clip, &mut float_clip, child, false);
    }
    // [#5855] 부동 개체 clip 의 하한은 **용지 하단**이다.
    //
    // 한글은 쪽 기준으로 앉힌 개체를 본문 영역에 가두지 않는다 — 꼬리말 자리에
    // 놓인 로고 띠(156618554_petfood_press: 정답지 이미지 하단 1056.0px, 본문 하단
    // 1028.1px)가 그대로 보인다. `body_bottom + 10` 상한은 그 20.9px 를 지웠다.
    //
    // Task #460 이 이 상한으로 막으려던 것은 대형 부동 그림이 body clip 을 넓혀
    // **흐름 콘텐츠**까지 꼬리말로 새게 하는 것이었다. 그런데 #3127 이후 흐름
    // clip(`flow_clip`)은 상한 없이 따로 잡히므로, 합집합의 하단은 이미 흐름
    // 콘텐츠가 결정한다. 이 상한이 실제로 자르고 있는 것은 부동 개체 자신뿐이다.
    // 용지 밖으로는 여전히 나가지 못한다.
    let max_bottom = page_height.max(body_bbox.y + body_bbox.height);
    if float_clip.y + float_clip.height > max_bottom {
        float_clip.height = max_bottom - float_clip.y;
    }
    // 두 clip 의 합집합 = 흐름 오버플로는 보존, 부동 그림은 상한 적용.
    let x0 = flow_clip.x.min(float_clip.x);
    let y0 = flow_clip.y.min(float_clip.y);
    let x1 = (flow_clip.x + flow_clip.width).max(float_clip.x + float_clip.width);
    let y1 = (flow_clip.y + flow_clip.height).max(float_clip.y + float_clip.height);
    let clip = BoundingBox {
        x: x0,
        y: y0,
        width: x1 - x0,
        height: y1 - y0,
    };
    body.node_type = RenderNodeType::Body {
        clip_rect: Some(clip),
    };
}

pub(crate) fn lift_cell_anchored_objects_above_text(node: &mut RenderNode) {
    for child in &mut node.children {
        lift_cell_anchored_objects_above_text(child);
    }
    if !matches!(node.node_type, RenderNodeType::TableCell(_)) {
        return;
    }
    let lifts = |child: &RenderNode| {
        child
            .layer
            .is_some_and(|layer| !matches!(layer.text_wrap, Some(TextWrap::BehindText)))
    };
    if !node.children.iter().any(lifts) {
        return;
    }
    let mut kept: Vec<RenderNode> = Vec::with_capacity(node.children.len());
    let mut lifted: Vec<RenderNode> = Vec::new();
    for child in node.children.drain(..) {
        if lifts(&child) {
            lifted.push(child);
        } else {
            kept.push(child);
        }
    }
    lifted.sort_by_key(|child| {
        child
            .layer
            .map(|layer| (layer.z_order, layer.stable_index))
            .unwrap_or((0, 0))
    });
    kept.extend(lifted);
    node.children = kept;
}
