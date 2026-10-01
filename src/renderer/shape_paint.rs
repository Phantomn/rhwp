//! Stateless shape paint shared by V2 and the remaining Legacy caller.
//! Bounds have already been resolved by the owner; no document/table layout.
use super::{
    hwpunit_to_px,
    layout::drawing_to_line_style,
    render_tree::{BoundingBox, LineNode, PathNode, RenderNode, RenderNodeType, ShapeTransform},
    PathCommand, ShapeStyle,
};
pub(crate) fn line(
    line: &crate::model::shape::LineShape,
    bounds: BoundingBox,
    dpi: f64,
    transform: ShapeTransform,
) -> RenderNode {
    let BoundingBox {
        x: render_x,
        y: render_y,
        width: render_w,
        height: render_h,
    } = bounds;
    let sa = &line.drawing.shape_attr;
    let sx = if sa.original_width > 0 {
        render_w / hwpunit_to_px(sa.original_width as i32, dpi)
    } else {
        1.0
    };
    let sy = if sa.original_height > 0 {
        render_h / hwpunit_to_px(sa.original_height as i32, dpi)
    } else {
        1.0
    };

    // 연결선: 제어점이 있으면 Path로, 없으면 Line으로 렌더링
    if let Some(ref conn) = line.connector {
        if !conn.control_points.is_empty() {
            let mut line_style = drawing_to_line_style(&line.drawing);
            // 연결선 화살표: LinkLineType → ArrowStyle
            use crate::model::shape::LinkLineType;
            match conn.link_type {
                LinkLineType::StraightOneWay
                | LinkLineType::StrokeOneWay
                | LinkLineType::ArcOneWay => {
                    line_style.end_arrow = super::ArrowStyle::Arrow;
                    line_style.end_arrow_size = 4; // 중간 크기
                }
                LinkLineType::StraightBoth | LinkLineType::StrokeBoth | LinkLineType::ArcBoth => {
                    line_style.start_arrow = super::ArrowStyle::Arrow;
                    line_style.start_arrow_size = 4;
                    line_style.end_arrow = super::ArrowStyle::Arrow;
                    line_style.end_arrow_size = 4;
                }
                _ => {}
            }
            // 제어점으로 경로 생성
            let mut commands = Vec::new();
            let conn_x1 = render_x + hwpunit_to_px(line.start.x, dpi) * sx;
            let conn_y1 = render_y + hwpunit_to_px(line.start.y, dpi) * sy;
            let conn_x2 = render_x + hwpunit_to_px(line.end.x, dpi) * sx;
            let conn_y2 = render_y + hwpunit_to_px(line.end.y, dpi) * sy;
            let connector_point_xy = |cp: &crate::model::shape::ConnectorControlPoint| {
                (
                    render_x + hwpunit_to_px(cp.x, dpi) * sx,
                    render_y + hwpunit_to_px(cp.y, dpi) * sy,
                )
            };
            let first_control_is_start = conn
                .control_points
                .first()
                .map(|cp| cp.point_type == 3)
                .unwrap_or(false);
            let last_control_is_end = conn
                .control_points
                .last()
                .map(|cp| cp.point_type == 26)
                .unwrap_or(false);
            let (path_start_x, path_start_y) = if first_control_is_start {
                connector_point_xy(&conn.control_points[0])
            } else {
                (conn_x1, conn_y1)
            };
            let mut path_end_x = conn_x2;
            let mut path_end_y = conn_y2;
            commands.push(PathCommand::MoveTo(path_start_x, path_start_y));

            if conn.link_type.is_arc() {
                // 곡선 연결선: 제어점(type=2)을 bezier 제어점으로, 나머지는 앵커로 사용
                let cps = &conn.control_points;
                let end_x = conn_x2;
                let end_y = conn_y2;
                // type=2인 제어점만 추출
                let ctrl_pts: Vec<(f64, f64)> = cps
                    .iter()
                    .filter(|cp| cp.point_type == 2)
                    .map(|cp| {
                        (
                            render_x + hwpunit_to_px(cp.x, dpi) * sx,
                            render_y + hwpunit_to_px(cp.y, dpi) * sy,
                        )
                    })
                    .collect();
                match ctrl_pts.len() {
                    0 => {
                        // 제어점 없음 → 직선
                        commands.push(PathCommand::LineTo(end_x, end_y));
                    }
                    1 => {
                        // 제어점 1개 → quadratic bezier (cubic으로 변환)
                        let (qx, qy) = ctrl_pts[0];
                        let (sx0, sy0) = match commands.last() {
                            Some(PathCommand::MoveTo(x, y)) => (*x, *y),
                            _ => (render_x, render_y),
                        };
                        // Q→C 변환: C = (S + 2*Q)/3, (2*Q + E)/3, E
                        let cx1 = (sx0 + 2.0 * qx) / 3.0;
                        let cy1 = (sy0 + 2.0 * qy) / 3.0;
                        let cx2 = (2.0 * qx + end_x) / 3.0;
                        let cy2 = (2.0 * qy + end_y) / 3.0;
                        commands.push(PathCommand::CurveTo(cx1, cy1, cx2, cy2, end_x, end_y));
                    }
                    2 => {
                        // 제어점 2개 → cubic bezier
                        let (cx1, cy1) = ctrl_pts[0];
                        let (cx2, cy2) = ctrl_pts[1];
                        commands.push(PathCommand::CurveTo(cx1, cy1, cx2, cy2, end_x, end_y));
                    }
                    _ => {
                        // 3개 이상 → 여러 cubic bezier 세그먼트
                        let mut i = 0;
                        while i + 1 < ctrl_pts.len() {
                            let (cx1, cy1) = ctrl_pts[i];
                            let (cx2, cy2) = ctrl_pts[i + 1];
                            let (ex, ey) = if i + 2 < ctrl_pts.len() {
                                (
                                    (cx2 + ctrl_pts[i + 2].0) / 2.0,
                                    (cy2 + ctrl_pts[i + 2].1) / 2.0,
                                )
                            } else {
                                (end_x, end_y)
                            };
                            commands.push(PathCommand::CurveTo(cx1, cy1, cx2, cy2, ex, ey));
                            i += 2;
                        }
                    }
                }
            } else {
                for cp in
                    conn.control_points
                        .iter()
                        .skip(if first_control_is_start { 1 } else { 0 })
                {
                    let (cpx, cpy) = connector_point_xy(cp);
                    commands.push(PathCommand::LineTo(cpx, cpy));
                    path_end_x = cpx;
                    path_end_y = cpy;
                }
                if !last_control_is_end {
                    commands.push(PathCommand::LineTo(conn_x2, conn_y2));
                    path_end_x = conn_x2;
                    path_end_y = conn_y2;
                }
            }

            let style = ShapeStyle {
                stroke_color: Some(line_style.color),
                stroke_width: line_style.width,
                stroke_dash: line_style.dash.clone(),
                fill_color: None,
                ..Default::default()
            };
            let node_id = 0;
            let mut path_node = PathNode::new(commands, style, None);
            path_node.transform = transform;
            // 연결선: 시작/끝 좌표 (선 선택 방식용) + 화살표
            path_node.connector_endpoints =
                Some((path_start_x, path_start_y, path_end_x, path_end_y));
            if line_style.start_arrow != super::ArrowStyle::None
                || line_style.end_arrow != super::ArrowStyle::None
            {
                path_node.line_style = Some(line_style);
            }
            RenderNode::new(
                node_id,
                RenderNodeType::Path(path_node),
                BoundingBox::new(render_x, render_y, render_w, render_h),
            )
        } else {
            // 제어점 없는 연결선 → 직선으로 렌더링
            let mut line_style = drawing_to_line_style(&line.drawing);
            use crate::model::shape::LinkLineType;
            match conn.link_type {
                LinkLineType::StraightOneWay
                | LinkLineType::StrokeOneWay
                | LinkLineType::ArcOneWay => {
                    line_style.end_arrow = super::ArrowStyle::Arrow;
                    line_style.end_arrow_size = 4;
                }
                LinkLineType::StraightBoth | LinkLineType::StrokeBoth | LinkLineType::ArcBoth => {
                    line_style.start_arrow = super::ArrowStyle::Arrow;
                    line_style.start_arrow_size = 4;
                    line_style.end_arrow = super::ArrowStyle::Arrow;
                    line_style.end_arrow_size = 4;
                }
                _ => {}
            }
            let x1 = render_x + hwpunit_to_px(line.start.x, dpi) * sx;
            let y1 = render_y + hwpunit_to_px(line.start.y, dpi) * sy;
            let x2 = render_x + hwpunit_to_px(line.end.x, dpi) * sx;
            let y2 = render_y + hwpunit_to_px(line.end.y, dpi) * sy;
            let node_id = 0;
            let mut line_node = LineNode::new(x1, y1, x2, y2, line_style);
            line_node.transform = transform;
            RenderNode::new(
                node_id,
                RenderNodeType::Line(line_node),
                BoundingBox::new(render_x, render_y, render_w, render_h),
            )
        }
    } else {
        // 일반 직선
        let line_style = drawing_to_line_style(&line.drawing);
        let x1 = render_x + hwpunit_to_px(line.start.x, dpi) * sx;
        let y1 = render_y + hwpunit_to_px(line.start.y, dpi) * sy;
        let x2 = render_x + hwpunit_to_px(line.end.x, dpi) * sx;
        let y2 = render_y + hwpunit_to_px(line.end.y, dpi) * sy;
        let node_id = 0;
        let mut line_node = LineNode::new(x1, y1, x2, y2, line_style);
        line_node.transform = transform;
        RenderNode::new(
            node_id,
            RenderNodeType::Line(line_node),
            BoundingBox::new(render_x, render_y, render_w, render_h),
        )
    }
}
