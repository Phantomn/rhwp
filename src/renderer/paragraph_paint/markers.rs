use crate::model::control::Control;
use crate::model::paragraph::Paragraph;
use crate::model::style::Alignment;
use crate::renderer::cell_context::CellContext;
use crate::renderer::composer::ComposedLine;
use crate::renderer::render_tree::*;
use crate::renderer::style_resolver::ResolvedStyleSet;
use crate::renderer::text_measurement::{estimate_text_width, resolved_to_text_style};

use super::helpers::*;
use super::ParagraphPaintSession;

impl ParagraphPaintSession<'_> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn layout_click_here_and_bookmark_markers(
        &self,
        tree: &mut PageLayoutContext,
        line_node: &mut RenderNode,
        p: &Paragraph,
        comp_line: &crate::renderer::composer::ComposedLine,
        char_x_map: &[(usize, f64)],
        styles: &ResolvedStyleSet,
        para_style_id: u16,
        section_index: usize,
        para_index: usize,
        cell_ctx: &Option<CellContext>,
        line_char_end: usize,
        // [#6111] 다음 줄이 이 줄의 끝 문자에서 시작하는가. 그렇다면 그 경계
        // 문자에 걸린 누름틀은 **다음 줄**이 소유한다 — 같은 파일의 TAC 계약
        // (`next_line_starts_at_run_end`)과 같은 규칙이다.
        next_line_starts_at_line_end: bool,
        x: f64,
        y: f64,
        line_height: f64,
        baseline: f64,
    ) -> f64 {
        // line_char_end: 파라미터로 수령 (원본: char_offset)
        let line_char_start = comp_line.char_start;
        let active = self.active_field.borrow();
        let ctrl_codes = self.show_control_codes.get();

        // char_x_map에서 특정 char_idx에 해당하는 x 좌표를 보간 계산
        let find_x_for_char = |target: usize| -> f64 {
            for i in 0..char_x_map.len().saturating_sub(1) {
                let (c0, x0) = char_x_map[i];
                let (c1, x1) = char_x_map[i + 1];
                if target >= c0 && target <= c1 {
                    if c1 == c0 {
                        return x0;
                    }
                    let ratio = (target - c0) as f64 / (c1 - c0) as f64;
                    return x0 + ratio * (x1 - x0);
                }
            }
            char_x_map.last().map(|&(_, xv)| xv).unwrap_or(x)
        };

        // 마커 삽입 정보 수집 (오른쪽→왼쪽 순으로 shift 처리)
        struct MarkerInsert {
            marker_x: f64,
            marker_w: f64,
            node: RenderNode,
        }
        let mut markers: Vec<MarkerInsert> = Vec::new();
        // [#6111] 접힌 안내문의 둘째 조각부터는 마커 shift 대상이 아니다 —
        // shift 를 끝낸 뒤 원래 x 그대로 얹는다.
        let mut wrapped_guide_overlays: Vec<RenderNode> = Vec::new();

        for fr in &p.field_ranges {
            if let Some(Control::Field(field)) = p.controls.get(fr.control_idx) {
                if field.field_type != crate::model::control::FieldType::ClickHere {
                    continue;
                }
                let is_empty = fr.start_char_idx == fr.end_char_idx;
                // [#6111] 줄 경계 문자에 걸린 누름틀은 **다음 줄**이 소유한다.
                //
                // 줄 끝 문자는 다음 줄의 시작 문자이기도 해서, 두 줄이 같은
                // 누름틀을 각자 그렸다 — 56345 7쪽은 빈 누름틀 안내문이 두 줄에
                // 중복되고, 그중 앞 줄은 **배분 정렬된 줄의 마지막 문자**라
                // char_x_map 이 본문 우단(718.6px)을 돌려줘 안내문이 쪽 밖으로
                // 나갔다. 같은 파일의 TAC 계약(`next_line_starts_at_run_end`)과
                // 같은 규칙으로 앞 줄의 소유권을 넘긴다.
                let owns_boundary = !next_line_starts_at_line_end;
                let start_in_line = fr.start_char_idx >= line_char_start
                    && (fr.start_char_idx < line_char_end
                        || (fr.start_char_idx == line_char_end && owns_boundary));
                let end_in_line = fr.end_char_idx >= line_char_start
                    && (fr.end_char_idx < line_char_end
                        || (fr.end_char_idx == line_char_end && owns_boundary));

                if !start_in_line && !end_in_line {
                    continue;
                }

                let is_active = if let Some((af_sec, af_para, af_ctrl, ref af_cell)) = *active {
                    if af_sec != section_index || af_para != para_index || af_ctrl != fr.control_idx
                    {
                        false
                    } else {
                        // cell_path 전체 일치 확인
                        match (af_cell, cell_ctx) {
                            (None, None) => true,
                            (Some(af_path), Some(ctx)) => {
                                // af_path와 ctx.path의 (control_index, cell_index) 쌍이 모두 일치해야 함
                                af_path.len() == ctx.path.len()
                                    && af_path.iter().zip(ctx.path.iter()).all(
                                        |(&(ac, ax, _ap), entry)| {
                                            ac == entry.control_index && ax == entry.cell_index
                                        },
                                    )
                            }
                            _ => false,
                        }
                    }
                } else {
                    false
                };

                let base_run = comp_line.runs.last().or(comp_line.runs.first());
                let base_style = if let Some(run) = base_run {
                    run.text_style(styles)
                } else {
                    resolved_to_text_style(styles, 0, 0)
                };

                // [누름틀 시작] 마커 — fr.start_char_idx 위치에 삽입
                if ctrl_codes && start_in_line {
                    let mut marker_style = base_style.clone();
                    marker_style.color = 0x0066CC; // BGR: 주황색 (#CC6600)
                    marker_style.font_size *= 0.55;
                    let marker_text = "[누름틀 시작]";
                    let marker_w = estimate_text_width(marker_text, &marker_style);
                    let marker_x = find_x_for_char(fr.start_char_idx);
                    let m_id = tree.next_id();
                    let m_node = RenderNode::new(
                        m_id,
                        RenderNodeType::TextRun(TextRunNode {
                            text: marker_text.to_string(),
                            style: marker_style,
                            char_shape_id: None,
                            para_shape_id: Some(para_style_id),
                            section_index: Some(section_index),
                            para_index: Some(para_index),
                            char_start: None,
                            cell_context: cell_ctx.clone(),
                            is_para_end: false,
                            is_line_break_end: false,
                            rotation: 0.0,
                            is_vertical: false,
                            char_overlap: None,
                            border_fill_id: 0,
                            baseline,
                            field_marker: FieldMarkerType::FieldBegin,
                            layout_positions: None,
                            display_text: None,
                        }),
                        BoundingBox::new(marker_x, y, marker_w, line_height),
                    );
                    markers.push(MarkerInsert {
                        marker_x,
                        marker_w,
                        node: m_node,
                    });
                }

                // 빈 필드 커서 앵커: getCursorRect가 필드 시작 위치를 찾을 수 있도록
                // char_start를 설정한 zero-width 노드 삽입
                if is_empty && start_in_line {
                    let anchor_x = find_x_for_char(fr.start_char_idx);
                    let anchor_id = tree.next_id();
                    let anchor_node = RenderNode::new(
                        anchor_id,
                        RenderNodeType::TextRun(TextRunNode {
                            text: String::new(),
                            style: base_style.clone(),
                            char_shape_id: None,
                            para_shape_id: Some(para_style_id),
                            section_index: Some(section_index),
                            para_index: Some(para_index),
                            char_start: Some(fr.start_char_idx),
                            cell_context: cell_ctx.clone(),
                            is_para_end: false,
                            is_line_break_end: false,
                            rotation: 0.0,
                            is_vertical: false,
                            char_overlap: None,
                            border_fill_id: 0,
                            baseline,
                            field_marker: FieldMarkerType::None,
                            layout_positions: None,
                            display_text: None,
                        }),
                        BoundingBox::new(anchor_x, y, 0.0, line_height),
                    );
                    markers.push(MarkerInsert {
                        marker_x: anchor_x,
                        marker_w: 0.0,
                        node: anchor_node,
                    });
                }

                // 빈 필드 안내문 (활성 필드가 아닐 때만)
                if is_empty && !is_active && start_in_line {
                    if let Some(guide) = field.guide_text() {
                        let mut guide_style = base_style.clone();
                        guide_style.color = 0x0000FF; // BGR: 빨간색
                        guide_style.italic = true;
                        // 안내문은 [누름틀 시작] 마커 뒤에 위치
                        let guide_x = find_x_for_char(fr.start_char_idx);
                        // [#6111] 긴 안내문을 한 줄로 그리면 본문·용지 밖까지 나간다
                        // (56345 7쪽: 49자 안내문이 x 93.7 → 943.7px, 용지 폭 794).
                        // 한글 편집기는 안내문을 누름틀 줄 상자 안에서 접는다. 안내문은
                        // 흐름에 영향이 없는 편집 전용 표시라(아래 `with_editor_only`),
                        // 접힌 뒤 줄들은 순수 오버레이로 아래에 쌓는다 — 첫 조각만
                        // 마커 shift 폭에 계상한다.
                        //
                        // [#6862] **칸 안도 접는다.** `#6111` 은 "셀은 가용 폭 기준이
                        // 다르므로 종전대로 한 줄"로 남겼는데, 그 기준은 이미 손에 있다 —
                        // **이 줄의 상자**(`line_node.bbox`)가 칸 안여백까지 반영한 텍스트
                        // 상자다. 2249811 1쪽은 안내문이 전부 표 칸 안이라 그 예외가
                        // 그대로 증상이 됐다(용지 밖 352.8px).
                        //
                        // ⚠ 본문 갈래는 종전 기준(`current_body_area`)을 그대로 둔다 —
                        // `#6111` 의 확정 핀이 그 값으로 잠겨 있다.
                        let (body_x, _, body_w, _) = self.current_body_area.get();
                        let line_right = line_node.bbox.x + line_node.bbox.width;
                        // [#6862] **빈 줄에서는 안내문 자신이 그 줄의 내용이다.**
                        //
                        // 빈 누름틀 줄은 글자 폭이 0 이라 `find_x_for_char` 가 돌려주는
                        // 것은 **정렬 앵커**(가운데 정렬이면 줄 중앙)다. 안내문을 거기서
                        // 오른쪽으로 그리면 통째로 폭의 절반만큼 밀린다.
                        //
                        // ```text
                        //   칸 286.9..670.1  중앙 478.5   안내문 폭 668.0
                        //     종전 시작 478.5           = 중앙 (폭을 안 뺐다)
                        //     정상 시작 478.5 − 334.0   = 144.5
                        // ```
                        //
                        // 줄에 보이는 글자가 있으면 그 앵커는 실제 글자 자리이므로
                        // 건드리지 않는다.
                        let line_has_visible_text =
                            comp_line.runs.iter().any(|run| !run.text.trim().is_empty());
                        let guide_alignment = styles
                            .para_styles
                            .get(para_style_id as usize)
                            .map(|style| style.alignment);
                        let guide_owns_the_line = !line_has_visible_text
                            && line_node.bbox.width > 0.0
                            && matches!(
                                guide_alignment,
                                Some(Alignment::Center) | Some(Alignment::Right)
                            );
                        let wrap_limit = if guide_owns_the_line {
                            line_node.bbox.width
                        } else if cell_ctx.is_none() && body_w > 0.0 {
                            (body_x + body_w - guide_x).max(0.0)
                        } else if line_right > guide_x {
                            line_right - guide_x
                        } else {
                            0.0
                        };
                        let guide_chunks =
                            split_guide_text_to_width(guide, &guide_style, wrap_limit);
                        let guide_width = guide_chunks
                            .first()
                            .map(|chunk| estimate_text_width(chunk, &guide_style))
                            .unwrap_or(0.0);
                        let guide_x = if guide_owns_the_line {
                            match guide_alignment {
                                Some(Alignment::Right) => {
                                    (line_right - guide_width).max(line_node.bbox.x)
                                }
                                _ => (line_node.bbox.x
                                    + (line_node.bbox.width - guide_width) / 2.0)
                                    .max(line_node.bbox.x),
                            }
                        } else {
                            guide_x
                        };
                        for (idx, chunk) in guide_chunks.iter().enumerate().skip(1) {
                            let extra_id = tree.next_id();
                            let extra = RenderNode::new(
                                extra_id,
                                RenderNodeType::TextRun(TextRunNode {
                                    text: (*chunk).to_string(),
                                    style: guide_style.clone(),
                                    char_shape_id: None,
                                    para_shape_id: Some(para_style_id),
                                    section_index: Some(section_index),
                                    para_index: Some(para_index),
                                    char_start: None,
                                    cell_context: cell_ctx.clone(),
                                    is_para_end: false,
                                    is_line_break_end: false,
                                    rotation: 0.0,
                                    is_vertical: false,
                                    char_overlap: None,
                                    border_fill_id: 0,
                                    baseline,
                                    field_marker: FieldMarkerType::None,
                                    layout_positions: None,
                                    display_text: None,
                                }),
                                BoundingBox::new(
                                    guide_x,
                                    y + line_height * idx as f64,
                                    estimate_text_width(chunk, &guide_style),
                                    line_height,
                                ),
                            )
                            .with_editor_only();
                            wrapped_guide_overlays.push(extra);
                        }
                        let guide = guide_chunks.first().copied().unwrap_or(guide);
                        let guide_id = tree.next_id();
                        let guide_node = RenderNode::new(
                            guide_id,
                            RenderNodeType::TextRun(TextRunNode {
                                text: guide.to_string(),
                                style: guide_style,
                                char_shape_id: None,
                                para_shape_id: Some(para_style_id),
                                section_index: Some(section_index),
                                para_index: Some(para_index),
                                char_start: None,
                                cell_context: cell_ctx.clone(),
                                is_para_end: false,
                                is_line_break_end: false,
                                rotation: 0.0,
                                is_vertical: false,
                                char_overlap: None,
                                border_fill_id: 0,
                                baseline,
                                field_marker: FieldMarkerType::None,
                                layout_positions: None,
                                display_text: None,
                            }),
                            BoundingBox::new(guide_x, y, guide_width, line_height),
                        );
                        // [#3375] 안내문은 한컴 편집 화면에서만 보이고 인쇄·PDF 에는 나가지
                        // 않는다. 그림 미지정 placeholder(#2225)와 같은 계약이라 같은
                        // `editor_only` 표시를 쓴다 — 흐름 폭에는 영향이 없으므로(별도 마커
                        // 노드) 쪽수·줄바꿈은 프로필과 무관하게 동일하다.
                        let guide_node = guide_node.with_editor_only();
                        markers.push(MarkerInsert {
                            marker_x: guide_x,
                            marker_w: guide_width,
                            node: guide_node,
                        });
                    }
                }

                // [누름틀 끝] 마커 — fr.end_char_idx 위치에 삽입
                if ctrl_codes && end_in_line {
                    let mut marker_style = base_style.clone();
                    marker_style.color = 0x0066CC; // BGR: 주황색
                    marker_style.font_size *= 0.55;
                    let marker_text = "[누름틀 끝]";
                    let marker_w = estimate_text_width(marker_text, &marker_style);
                    let marker_x = find_x_for_char(fr.end_char_idx);
                    let m_id = tree.next_id();
                    let m_node = RenderNode::new(
                        m_id,
                        RenderNodeType::TextRun(TextRunNode {
                            text: marker_text.to_string(),
                            style: marker_style,
                            char_shape_id: None,
                            para_shape_id: Some(para_style_id),
                            section_index: Some(section_index),
                            para_index: Some(para_index),
                            char_start: None,
                            cell_context: cell_ctx.clone(),
                            is_para_end: false,
                            is_line_break_end: false,
                            rotation: 0.0,
                            is_vertical: false,
                            char_overlap: None,
                            border_fill_id: 0,
                            baseline,
                            field_marker: FieldMarkerType::FieldEnd,
                            layout_positions: None,
                            display_text: None,
                        }),
                        BoundingBox::new(marker_x, y, marker_w, line_height),
                    );
                    markers.push(MarkerInsert {
                        marker_x,
                        marker_w,
                        node: m_node,
                    });
                }
            }
        }

        // 책갈피 조판부호 마커
        if ctrl_codes {
            let ctrl_positions = p.logical_control_positions();
            for (ci, ctrl) in p.controls.iter().enumerate() {
                if let Control::Bookmark(_bm) = ctrl {
                    let char_pos = ctrl_positions.get(ci).copied().unwrap_or(0);
                    if char_pos >= line_char_start && char_pos <= line_char_end {
                        let base_run = comp_line.runs.last().or(comp_line.runs.first());
                        let bm_base_style = if let Some(run) = base_run {
                            run.text_style(styles)
                        } else {
                            resolved_to_text_style(styles, 0, 0)
                        };
                        let mut marker_style = bm_base_style;
                        marker_style.color = 0x0000FF; // BGR: 빨간색 (#FF0000)
                        marker_style.font_size *= 0.55;
                        let marker_text = "[책갈피]".to_string();
                        let marker_w = estimate_text_width(&marker_text, &marker_style);
                        let marker_x = find_x_for_char(char_pos);
                        let m_id = tree.next_id();
                        let m_node = RenderNode::new(
                            m_id,
                            RenderNodeType::TextRun(TextRunNode {
                                text: marker_text,
                                style: marker_style,
                                char_shape_id: None,
                                para_shape_id: Some(para_style_id),
                                section_index: Some(section_index),
                                para_index: Some(para_index),
                                char_start: None,
                                cell_context: cell_ctx.clone(),
                                is_para_end: false,
                                is_line_break_end: false,
                                rotation: 0.0,
                                is_vertical: false,
                                char_overlap: None,
                                border_fill_id: 0,
                                baseline,
                                field_marker: FieldMarkerType::None,
                                layout_positions: None,
                                display_text: None,
                            }),
                            BoundingBox::new(marker_x, y, marker_w, line_height),
                        );
                        markers.push(MarkerInsert {
                            marker_x,
                            marker_w,
                            node: m_node,
                        });
                    }
                }
            }
        }

        // 도형 조판부호 마커는 텍스트 런 루프 내에서 직접 처리됨 (MarkerInsert 불사용)

        // 마커를 왼쪽부터 삽입하면서, 각 마커 뒤의 기존 노드와 이후 마커를 오른쪽으로 shift
        // zero-width 앵커(커서 위치용)는 shift하지 않고 원래 위치 유지
        markers.sort_by(|a, b| {
            a.marker_x
                .partial_cmp(&b.marker_x)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut accumulated_shift = 0.0_f64;
        for mi in 0..markers.len() {
            let mw = markers[mi].marker_w;
            if mw == 0.0 {
                // zero-width 앵커: shift 없이 원래 위치 유지
                continue;
            }
            let shift_x = markers[mi].marker_x + accumulated_shift;
            // 기존 children 중 이 마커 위치 이후의 노드를 오른쪽으로 shift
            for child in line_node.children.iter_mut() {
                if child.bbox.x >= shift_x {
                    child.bbox.x += mw;
                }
            }
            // 이미 삽입된 마커도 shift (이전 마커 중 이 위치 이후에 있는 것)
            // → accumulated_shift로 처리됨
            markers[mi].node.bbox.x = shift_x;
            accumulated_shift += mw;
        }
        // 모든 마커 노드를 children에 추가
        for overlay in wrapped_guide_overlays {
            line_node.children.push(overlay);
        }
        for mi in markers {
            line_node.children.push(mi.node);
        }
        accumulated_shift
    }
}
