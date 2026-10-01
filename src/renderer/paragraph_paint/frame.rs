use crate::model::bin_data::BinDataContent;
use crate::model::control::Control;
use crate::model::paragraph::{LineSeg, Paragraph};
use crate::model::shape::{ShapeObject, TextWrap};
use crate::model::style::{Alignment, HeadType, LineSpacingType};
use crate::model::table::Table;
use crate::renderer::composer::{
    compose_paragraph, effective_text_for_metrics, ComposedLine, ComposedParagraph,
};
use crate::renderer::layout::text_measurement::{estimate_text_width, resolved_to_text_style};
use crate::renderer::layout::CellContext;
use crate::renderer::page_layout::LayoutRect;
use crate::renderer::render_tree::*;
use crate::renderer::style_resolver::ResolvedStyleSet;
use crate::renderer::{hwpunit_to_px, px_to_hwpunit};

use super::helpers::*;
use super::ParagraphPaintSession;

impl ParagraphPaintSession<'_> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn layout_composed_paragraph_in_frame(
        &self,
        tree: &mut PageLayoutContext,
        col_node: &mut RenderNode,
        composed: &ComposedParagraph,
        styles: &ResolvedStyleSet,
        col_area: &LayoutRect,
        y_start: f64,
        start_line: usize,
        end_line: usize,
        section_index: usize,
        para_index: usize,
        cell_ctx: Option<CellContext>,
        suppress_column_top_vpos_fallback: bool,
        is_last_cell_para: bool,
        first_line_x_offset: f64,
        multi_col_width_hu: Option<i32>,
        para: Option<&Paragraph>,
        bin_data_content: Option<&[BinDataContent]>,
        wrap_anchor: Option<&crate::renderer::pagination::WrapAnchorRef>,
        physical_rows: Option<&[std::ops::Range<f64>]>,
        squeeze_stored_line: bool,
    ) -> f64 {
        let physical_frame_rows = physical_rows.is_some();
        let mut y = y_start;
        let end = end_line.min(composed.lines.len());
        // [#4968 R4D-1] 한 문단의 모든 최종 emitted run이 같은 registry
        // generation과 per-face parse cache를 소비한다.
        let mut kerning_layout_session = self.font_state.exact_font_layout_session();

        // 문단 스타일에서 여백 및 정렬 정보
        let para_style = styles.para_styles.get(composed.para_style_id as usize);
        let box_margin_left = para_style.map(|s| s.margin_left).unwrap_or(0.0);
        let box_margin_right = para_style.map(|s| s.margin_right).unwrap_or(0.0);
        let indent = para_style.map(|s| s.indent).unwrap_or(0.0);

        // [Task #547] paragraph margin_left/right 는 텍스트 좌/우 inset 으로 한 번만
        // 적용. Task #544 후 box outline = col_area (margin 미적용) 이므로 박스 안
        // 좌측 여백 = box_margin_left (PDF 한컴 2010 정합).
        // 이전 코드는 paragraph border + border_spacing=0 인 경우 inner_pad_left =
        // box_margin_left 로 한 번 더 더해 이중 inset 부작용 발생 (Task #544 전 박스도
        // margin 적용했을 때만 의미가 있던 분기).
        let margin_left = box_margin_left;
        let margin_right = box_margin_right;
        let alignment = para_style
            .map(|s| s.alignment)
            .unwrap_or(Alignment::Justify);
        let spacing_before = crate::renderer::hwp3_variant_flow_spacing_before(
            para_style.map(|s| s.spacing_before).unwrap_or(0.0),
            self.use_hwp3_origin_flow_spacing_before.get(),
        );
        let spacing_after = para_style.map(|s| s.spacing_after).unwrap_or(0.0);
        // [Task #874 Case 3] `<...>` 단독 paragraph 의 paragraph-level extra spacing 제거.
        // typeset.rs::format_paragraph 측 동일 제거 — solo_zone_pad (zone 전환 패딩) 만 유지.
        let tab_width = para_style.map(|s| s.default_tab_width).unwrap_or(0.0);
        let tab_stops = para_style.map(|s| s.tab_stops.clone()).unwrap_or_default();
        let auto_tab_right = para_style.map(|s| s.auto_tab_right).unwrap_or(false);

        // [Task #489] 비-TAC Picture/Shape with wrap=Square 보유 여부.
        // 한컴은 어울림 그림이 있는 paragraph 의 LINE_SEG.cs/sw 를 그림 너비만큼 좁혀
        // 인코딩한다. 표 Square wrap (#362/#439/#463) 은 caller 가 col_area 를 좁혀
        // wrap_area 로 우회하지만, Picture/Shape 는 호스트 paragraph 와 같은 paragraph
        // 에 anchor 되므로 별도 우회 경로가 없다. 이 플래그가 true 면 줄별 루프에서
        // LINE_SEG.cs/sw 를 effective col_x/col_width 로 사용한다.
        let has_picture_shape_square_wrap = para
            .map(|p| {
                p.controls.iter().any(|c| {
                    let common_opt = match c {
                        Control::Picture(pic) if !pic.common.treat_as_char => Some(&pic.common),
                        Control::Shape(s) if !s.common().treat_as_char => Some(s.common()),
                        _ => None,
                    };
                    common_opt
                        .map(|cm| matches!(cm.text_wrap, TextWrap::Square))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        let has_ole_shape_square_wrap = para
            .map(|p| {
                p.controls.iter().any(|c| {
                    matches!(
                        c,
                        Control::Shape(shape)
                            if matches!(shape.as_ref(), ShapeObject::Ole(_))
                                && !shape.common().treat_as_char
                                && matches!(shape.common().text_wrap, TextWrap::Square)
                    )
                })
            })
            .unwrap_or(false);
        // [Task #1209 Stage5] 비-TAC `자리차지(TopAndBottom)` 개체가 같은 문단에
        // 있으면 한컴은 LINE_SEG.vertical_pos 로 각 줄의 실제 흐름 위치를 저장한다.
        // 첫 줄 vpos 만 한 번 더하는 fallback 으로는 “텍스트-그림-텍스트”처럼
        // 한 문단 안에서 그림 위/아래로 흐름이 갈라지는 케이스를 처리할 수 없다.
        let has_para_topbottom_float =
            has_para_topbottom_float_affecting_column(para, col_area, self.dpi);
        let col_area_w_hu = px_to_hwpunit(col_area.width, self.dpi);

        // treat_as_char 컨트롤의 px 폭 목록 (절대 char 위치, px 폭, control_index) — 정렬 보장
        let tac_offsets_px: Vec<(usize, f64, usize)> = {
            let mut v: Vec<(usize, f64, usize)> = composed
                .tac_controls
                .iter()
                .map(|(pos, w_hu, ci)| {
                    let width = para
                        .and_then(|p| p.controls.get(*ci))
                        .and_then(|ctrl| match ctrl {
                            Control::Picture(pic) => {
                                // [#6603] 줄 안에서 차지하는 폭은 잉크 + 좌우 바깥 여백.
                                let (margin_left, margin_right, _, _) =
                                    tac_picture_outer_margins_px(pic, self.dpi);
                                Some(
                                    self.resolve_inline_picture_size(pic, col_area).0
                                        + margin_left
                                        + margin_right,
                                )
                            }
                            Control::Shape(shape) => {
                                // [#6606] 도형·묶음도 줄 안에서 상자(폭 + 좌우 여백)를 차지한다.
                                let (margin_left, margin_right, _, _) =
                                    tac_object_outer_margins_px(shape.common(), self.dpi);
                                Some(hwpunit_to_px(*w_hu, self.dpi) + margin_left + margin_right)
                            }
                            _ => None,
                        })
                        .unwrap_or_else(|| hwpunit_to_px(*w_hu, self.dpi));
                    (*pos, width, *ci)
                })
                .collect();
            v.sort_by_key(|(p, _, _)| *p);
            v
        };
        // 문단 배경색: border_fill_id 조회
        let para_border_fill_id = para_style.map(|s| s.border_fill_id).unwrap_or(0);
        let para_fill_color = if para_border_fill_id > 0 {
            let idx = (para_border_fill_id as usize).saturating_sub(1);
            styles.border_styles.get(idx).and_then(|bs| bs.fill_color)
        } else {
            None
        };
        let horizontal_shaping_initial_lane = horizontal_shaping_initial_lane_preflight(
            composed,
            para,
            styles,
            start_line,
            end,
            alignment,
            para_border_fill_id,
        );

        // 문단 앞 간격 (첫 줄일 때만)
        // 단/페이지의 맨 처음 문단(column-top)은 spacing_before 를 통째 적용하면 한컴보다
        // 아래로 밀리므로 종전엔 0 으로 버렸다. 다만 섹션의 첫 문단(para_index==0, 예: 제목)은
        // 한컴 PDF 가 LINE_SEG.vertical_pos(실제 렌더한 첫 줄 흐름 위치)만큼 앞 간격을 두므로
        // (제목: spacing_before=52.9px 이지만 vertical_pos=26.5px), 그 경우 spacing_before 를
        // LINE_SEG.vertical_pos 로 상한 클램프해 적용한다. 페이지 break 후 이어진 column-top
        // (para_index>0)은 종전대로 0. (Task #853)
        let is_column_top = (y - col_area.y).abs() < 1.0;
        // [Task #1728 v2] RowBreak 셀-내 continuation 조각의 첫 가시 문단은 셀-상단이지만
        // (is_column_top) 셀-상대 인덱스>0 이라 아래 para_index==0 클램프 분기에도 못 든다.
        // 한컴은 이 첫 문단의 앞 간격(spacing_before)을 유지하므로, 토글이 켜진 이 문단만
        // column-top 이 아닌 것처럼 spacing_before 를 전량 적용한다.
        let keep_continuation_spacing_before =
            self.keep_continuation_column_top_spacing_before.get();
        // [#5601] 셀 저장-앵커 스냅 문단 — 호출자가 para_y 에서 spacing_before 를
        // 미리 뺐으므로 column-top 트림과 무관하게 전량 재가산해야 vpos 와 맞는다.
        let reapply_snap_spacing_before = self.reapply_snap_anchored_spacing_before.replace(false);
        if start_line == 0 && spacing_before > 0.0 {
            if !is_column_top || keep_continuation_spacing_before || reapply_snap_spacing_before {
                y += spacing_before;
            } else if para_index == 0 && !suppress_column_top_vpos_fallback {
                let vpos0_px = para
                    .and_then(|p| p.line_segs.first())
                    .map(|ls| hwpunit_to_px(ls.vertical_pos, self.dpi))
                    .unwrap_or(0.0);
                y += spacing_before.min(vpos0_px.max(0.0));
            } else if !suppress_column_top_vpos_fallback {
                // [Task #1811] 쪽 상단(para_index>0) 문단도 저장 첫 줄 vpos 가 증거다 —
                // 한컴이 앞 간격을 유지한 문서는 쪽-상대 vpos ≈ spacing_before 로 저장되고
                // (task1750 샘플 p2: sb=700HU, vpos=700 — 트림 시 페이지 전체가 5pt 위로
                // 밀려 visual sweep 이중상), 트림한 문서는 vpos=0 이다. #853 의
                // para_index==0 클램프를 저장 증거 기반으로 일반화하되, 누적축 vpos
                // 인코딩(vpos ≫ sb)은 쪽-상대 증거가 아니므로 종전(트림) 유지.
                let vpos0_px = para
                    .and_then(|p| p.line_segs.first().map(|ls| (p, ls)))
                    .filter(|(_, ls)| ls.tag & LineSeg::TAG_IMPLEMENTATION_PROPERTY == 0)
                    .map(|(p, ls)| {
                        // 누락 개체 줄 보충 때문에 재사다리화한 좌표는 쪽-상대 증거가
                        // 아니다. 정규화 전에 보존한 원본 첫 줄 위치로 간격을 판독한다.
                        let source_vpos = p
                            .source_line_seg_vertical_pos
                            .as_ref()
                            .and_then(|positions| positions.first().copied())
                            .unwrap_or(ls.vertical_pos);
                        hwpunit_to_px(source_vpos, self.dpi)
                    })
                    .unwrap_or(0.0);
                if vpos0_px > 0.0 && vpos0_px <= spacing_before + 0.5 {
                    y += vpos0_px;
                }
            }
        }
        // [Task #1012] paragraph 첫 line vpos > 0 인데 spacing_before=0 으로
        // 위 블록 진입 안한 경우 (test-image.hwp page 1: TopAndBottom Picture)
        // line_seg.vpos 를 직접 y 에 가산하여 텍스트가 wrap shape 아래로
        // 위치하도록 함. wrap 메커니즘이 별도로 처리하지 못하는 case 의
        // fallback. start_line==0 + column-top + para_index==0 으로 한정.
        if start_line == 0
            && spacing_before == 0.0
            && is_column_top
            && para_index == 0
            && !has_para_topbottom_float
            && !suppress_column_top_vpos_fallback
        {
            let vpos0_px = para
                .and_then(|p| p.line_segs.first())
                .map(|ls| hwpunit_to_px(ls.vertical_pos, self.dpi))
                .unwrap_or(0.0);
            // [편집 세션] Enter로 자란 자리차지 표의 post-text가 typeset에서 다음
            // 쪽으로 재배정된 경우, 저장 vpos는 앞 쪽 하단 좌표라 무효다 — 절대
            // 가산하면 새 쪽에서도 쪽 하단에 그려져 문구·로고가 잘린다(셀 끝
            // Enter 재현). 단 절반을 넘는 과대 vpos만 차단해 상단 여백
            // 재현(test-image.hwp 폴백 목적)은 유지한다.
            let session_stale_vpos =
                self.profile.get().session_edited() && vpos0_px > col_area.height * 0.5;
            if vpos0_px > 0.0 && !session_stale_vpos {
                y += vpos0_px;
            }
        }

        // 문단 전체에서 모든 라인의 runs가 비어있는지 확인
        // (텍스트 없이 TAC 이미지만 있는 문단)
        //
        // [Issue #1945] `start_line` 은 PartialTable/Partial 이월 경로에서 인자로
        // 전달되며 `end`(= end_line.min(lines.len())) 와 독립 계산이라, 이월 루프가
        // 조판 라인 수를 넘겨 `start_line > end`(또는 > lines.len())가 되면 직접
        // 슬라이스가 패닉했다(실문서 크래시). 아래 렌더 루프(`for line_idx in
        // start_line..end`)는 빈 범위를 안전히 처리하므로, 여기서도 `get()` 으로
        // 방어해 범위 밖이면 "가시 run 없음"(vacuously true)으로 본다.
        let all_runs_empty = composed
            .lines
            .get(start_line..end)
            .map_or(true, |slice| slice.iter().all(|l| l.runs.is_empty()));

        // 개요 번호/글머리표 마커 폭 사전 계산 (첫 줄 가용폭 차감용)
        let numbering_width = if start_line == 0 {
            if let Some(ref num_text) = composed.numbering_text {
                let num_style = numbering_marker_text_style(
                    styles,
                    para,
                    composed.lines.first().and_then(|l| l.runs.first()),
                );
                estimate_text_width(num_text, &num_style)
            } else {
                0.0
            }
        } else {
            0.0
        };

        // 배경/테두리 렌더링을 위한 시작 위치 기록
        // 문단 경계 = 이전 문단 끝 = y_start (spacing_before 적용 전)
        let bg_y_start = if para_border_fill_id > 0 { y_start } else { y };
        let bg_insert_idx = col_node.children.len();

        // start_line까지의 누적 문자 오프셋 계산 (편집용 문서 좌표)
        let mut char_offset: usize = 0;
        for li in 0..start_line.min(composed.lines.len()) {
            for run in &composed.lines[li].runs {
                char_offset += run.text.chars().count();
            }
            // 강제 줄바꿈(\n)은 run 텍스트에서 제거되었으므로 별도 가산
            if composed.lines[li].has_line_break {
                char_offset += 1;
            }
        }

        // [Issue #926] Endnote 인라인 마커 — 첫 줄 앞에 일반 텍스트로 emit
        // 한컴에서 미주 마커는 위첨자가 아닌 본문 크기 텍스트로 표시
        let mut endnote_marker_x_advance = 0.0f64;
        if start_line == 0 {
            if let Some(p) = para {
                let ctrl_positions = p.control_text_positions();
                let first_line_char_start = composed
                    .lines
                    .first()
                    .map(|line| line.char_start)
                    .unwrap_or(0);
                for (ctrl_idx, ctrl) in p.controls.iter().enumerate() {
                    if let Control::Endnote(en) = ctrl {
                        let Some(marker_pos) = ctrl_positions.get(ctrl_idx).copied() else {
                            continue;
                        };
                        if !is_leading_endnote_marker_rendered_as_prefix(
                            para,
                            ctrl_idx,
                            0,
                            start_line,
                            marker_pos,
                            first_line_char_start,
                        ) {
                            continue;
                        }
                        let marker_text =
                            format!("{} ", note_marker_text_from_control(Some(ctrl), en.number));
                        let first_cs_id = p
                            .char_shapes
                            .first()
                            .map(|cs| cs.char_shape_id as usize)
                            .unwrap_or(0);
                        let ts = resolved_to_text_style(styles, first_cs_id as u32, 0);
                        let marker_w = estimate_text_width(&marker_text, &ts);
                        let marker_y = y
                            + spacing_before
                            + hwpunit_to_px(
                                composed
                                    .lines
                                    .first()
                                    .map(|l| l.baseline_distance)
                                    .unwrap_or(0),
                                self.dpi,
                            );
                        let marker_x = col_area.x + margin_left + indent;
                        let marker_id = tree.next_id();
                        let marker_node = RenderNode::new(
                            marker_id,
                            RenderNodeType::TextRun(TextRunNode {
                                text: marker_text,
                                style: ts,
                                char_shape_id: Some(first_cs_id as u32),
                                para_shape_id: Some(composed.para_style_id),
                                section_index: Some(section_index),
                                para_index: Some(para_index),
                                char_start: Some(0),
                                cell_context: None,
                                is_para_end: false,
                                is_line_break_end: false,
                                rotation: 0.0,
                                is_vertical: false,
                                char_overlap: None,
                                border_fill_id: 0,
                                baseline: hwpunit_to_px(
                                    composed
                                        .lines
                                        .first()
                                        .map(|l| l.baseline_distance)
                                        .unwrap_or(0),
                                    self.dpi,
                                ),
                                field_marker: FieldMarkerType::None,
                                layout_positions: None,
                                display_text: None,
                            }),
                            BoundingBox::new(
                                marker_x,
                                y + spacing_before,
                                marker_w,
                                hwpunit_to_px(
                                    composed.lines.first().map(|l| l.line_height).unwrap_or(0),
                                    self.dpi,
                                ),
                            ),
                        );
                        col_node.children.push(marker_node);
                        endnote_marker_x_advance += marker_w;
                    }
                }
            }
        }

        let endnote_line_vpos_base: Option<(i32, f64)> = {
            let base = self.endnote_para_base.get();
            if cell_ctx.is_none() && para_index >= base && end > start_line + 1 {
                para.and_then(|p| {
                    let base_line_idx = if line_is_leading_empty_equation_tac_guide(
                        Some(p),
                        composed,
                        &tac_offsets_px,
                        start_line,
                    ) {
                        start_line + 1
                    } else {
                        start_line
                    };
                    let range = p.line_segs.get(base_line_idx..end)?;
                    if range
                        .windows(2)
                        .all(|w| w[1].vertical_pos >= w[0].vertical_pos)
                    {
                        range.first().map(|seg| (seg.vertical_pos, y))
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        };
        // [#6545] 쪽을 넘어온 미주 문단 꼬리는 **첫 걸음에서만** 사다리가 뒤로 감긴다.
        //
        // HWPX 미주는 쪽 경계에서 `vertpos=0` 으로 리셋하고 그 뒤 줄들을 새 쪽 좌표계로
        // 적는다. 파서(`normalize_hwpx_note_line_vpos`, #1692)는 note 안의 `0` 을 연속줄
        // 아티팩트로 보고 `prev + line_height + line_spacing` 으로 되돌리는데, 뒤 줄들은
        // 새 좌표계 그대로라 **리셋 줄 하나만** 앞 쪽 좌표로 튄다.
        //
        //   seg9 1487426 → seg10 *1505494* → seg11 1450574 → seg12 1451926
        //                  (되돌린 값)        ← 여기서만 감김, 이후는 단조
        //
        // 그 한 걸음 때문에 위 단조 검사가 꺼지면 꼬리 **전체**가 저장 배치를 잃고
        // `line_height`(=`vertsize`) 누적으로 떨어진다. `vertsize > textheight` 인 줄에서
        // 진행량이 부풀어(2205+452 vs 900+452) 다음 문단이 수식 위에 겹친다
        // (3-09월_교육_통합_2022 23쪽).
        //
        // IR 을 고치면 미주 흐름 회계 전체가 흔들리므로(형제 문서 회귀 확인), 배치만
        // 되살린다 — 리셋 줄은 흐름대로 두고, **둘째 줄부터** 저장 델타를 쓴다. 기준 y 는
        // 그 줄에 도달했을 때의 흐름 y 라 루프 안에서 늦게 잡는다.
        let endnote_lazy_vpos_base_from: Option<usize> = {
            let base = self.endnote_para_base.get();
            if endnote_line_vpos_base.is_none()
                && cell_ctx.is_none()
                && para_index >= base
                && start_line > 0
                && end > start_line + 2
            {
                para.and_then(|p| {
                    let range = p.line_segs.get(start_line..end)?;
                    let backward_at_first_step = range[0].vertical_pos > range[1].vertical_pos;
                    let rest_is_monotonic = range[1..]
                        .windows(2)
                        .all(|w| w[1].vertical_pos >= w[0].vertical_pos);
                    (backward_at_first_step && rest_is_monotonic).then_some(start_line + 1)
                })
            } else {
                None
            }
        };
        let mut endnote_line_vpos_base = endnote_line_vpos_base;
        let para_topbottom_line_vpos_base: Option<(i32, f64)> = {
            if cell_ctx.is_none() && has_para_topbottom_float {
                para.and_then(|p| {
                    if p.stored_text_partition_dirty {
                        return None;
                    }
                    let range = p.line_segs.get(start_line..end)?;
                    if range.iter().any(|seg| seg.vertical_pos > 0)
                        && range
                            .windows(2)
                            .all(|w| w[1].vertical_pos >= w[0].vertical_pos)
                        // vpos 는 쪽(단) 상단 기준 쪽-상대 좌표다(아래 #3637 주석).
                        // 단 높이를 유의미하게 넘는 vpos 는 앞 쪽 좌표계의 잔재다 —
                        // 셀 편집으로 커진 자리차지 표가 분할 이월된 뒤의 host 후행
                        // 줄(재현 실측: vpos 가 단 높이 초과)을 절대 스냅하면
                        // 다음 쪽 본문 밖에 그려져 하단 문구가 소실된다. 이때는
                        // 흐름 y(분할 조각 하단)로 폴백한다.
                        && range.iter().all(|seg| {
                            hwpunit_to_px(seg.vertical_pos, self.dpi)
                                <= col_area.height + 60.0
                        })
                        // [편집 세션] Enter로 자란 자리차지 표의 post-text가 다음 쪽으로
                        // 재배정되면 저장 vpos(앞 쪽 하단 좌표)는 무효다 — 스냅하면 새
                        // 쪽에서도 쪽 하단에 그려져 잘린다(셀 끝 Enter 재현). 스냅
                        // 목적지가 흐름 커서보다 단 절반 이상 아래면 흐름 y 로
                        // 폴백한다. 같은 쪽 배치(괴리 소폭)는 종전 스냅을 유지한다.
                        // 반대 방향도 같다 — 목적지가 흐름보다 8px 넘게 **위**면 편집
                        // 성장 전 좌표라 앞 표에 겹친다(셀 Enter 재현: 후행 안내
                        // 문구가 저장 vpos 로 스냅돼 커진 표 하단 위에 얹힘).
                        // 8px 는 vpos_adjust 백워드 클램프와 동일.
                        && !(self.profile.get().session_edited()
                            && range.first().is_some_and(|seg| {
                                let snap_y =
                                    col_area.y + hwpunit_to_px(seg.vertical_pos, self.dpi);
                                snap_y > y + col_area.height * 0.5 || y - snap_y > 8.0
                            }))
                    {
                        // [#3637] 기준은 **단 상단**이다 (원점 0).
                        //
                        // `LINE_SEG.vertical_pos` 는 문단 기준이 아니라 쪽(단) 상단 기준
                        // 누적 절대값이다 — 같은 쪽에서 pi=5 → 13949, pi=17 → 58149 로
                        // 문단을 가로질러 단조 증가한다. 따라서 줄의 y 는
                        // `단 상단 + vpos` 이지, `흐름 커서 + vpos` 가 아니다.
                        //
                        // 종전에는 `start_line == 0` 일 때 기준 vpos 만 0 으로 두고 기준 y 는
                        // 흐름 커서(`y`)로 두어, 절대값이 커서 위에 **한 번 더** 얹혔다.
                        // 문단이 쪽 상단이면 커서≈0 이라 무해했지만, 쪽 중간 문단이면 자기
                        // vpos 만큼 아래로 밀려 쪽 밖으로 나간다.
                        //
                        // 실측 (해양 모빌리티 보도자료 pi=17):
                        //   단 상단 94.5 + vpos 775.3 = 869.8px 가 정답인데 흐름 커서
                        //   869.8 에 vpos 를 또 더해 1660px 에 그렸다. 쪽 하단 1028px 를
                        //   632px 넘겨 세 줄 93글자가 SVG·PNG 어느 경로에서도 보이지 않았다.
                        //
                        // 기준 y 를 단 상단으로 내리면 첫 줄이 개체 아래로 밀린 경우
                        // (#1459 자리차지 그림 + TAC 그림 스택)도 그 밀림이 vpos 에 이미
                        // 담겨 있어 그대로 재현된다. 첫 줄 vpos 를 기준 삼으면 그 밀림이
                        // 사라져 두 그림이 같은 y 에 겹친다 — 실제로 겪은 회귀다.
                        Some((0, col_area.y))
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        };
        let mut endnote_line_vpos_y_end: Option<f64> = None;
        let mut endnote_auto_wrap_y_end: Option<f64> = None;
        let mut prev_line_reserved_tac_picture_height: Option<f64> = None;
        // [#5711] 마지막으로 그린 줄 상자의 아래 경계. 문단 테두리의 아래 변은 이 값을
        // 따라야 한다 — 줄간격이 음수인 문단에서는 전진값 `y` 가 줄 상자 아래보다 위로
        // 올라가, 테두리가 글자를 가로지른다(3143955 제목: 줄 상자 아래 171.6px, 전진 y
        // 162.0px, 실제로 그려진 선 159.7/163.7).
        let mut last_line_box_bottom: Option<f64> = None;
        let mut last_line_border_bottom: Option<f64> = None;
        let stored_tac_assignment =
            para.and_then(|p| crate::renderer::composer::stored_tac_line_assignment(p, composed));
        for line_idx in start_line..end {
            let source_line_tacs;
            let tac_offsets_px = if let Some(assign) = stored_tac_assignment.as_ref() {
                source_line_tacs = tac_offsets_px
                    .iter()
                    .copied()
                    .filter(|(_, _, ci)| {
                        assign
                            .iter()
                            .any(|(control, owner)| control == ci && *owner == line_idx)
                    })
                    .collect::<Vec<_>>();
                &source_line_tacs
            } else {
                &tac_offsets_px
            };
            let comp_line = &composed.lines[line_idx];
            let mut current_line_reserved_tac_picture_height: Option<f64> = None;
            let mut endnote_used_auto_wrap_y = false;
            // [#6545] 감긴 첫 걸음을 지난 지점에서 기준을 늦게 잡는다 — 이 줄의 흐름 y 가
            // 곧 기준 y 이므로 이 줄 자신의 위치는 바뀌지 않고, 뒤 줄들만 저장 델타를 탄다.
            if endnote_line_vpos_base.is_none() && endnote_lazy_vpos_base_from == Some(line_idx) {
                endnote_line_vpos_base = para
                    .and_then(|p| p.line_segs.get(line_idx))
                    .map(|seg| (seg.vertical_pos, y));
            }
            if let (Some((base_vpos, base_y)), Some(seg)) = (
                endnote_line_vpos_base,
                para.and_then(|p| p.line_segs.get(line_idx)),
            ) {
                let vpos_y = base_y + hwpunit_to_px(seg.vertical_pos - base_vpos, self.dpi);
                if let Some(prev) = endnote_auto_wrap_y_end {
                    if prev > vpos_y + 0.5 {
                        y = prev;
                        endnote_used_auto_wrap_y = true;
                    } else {
                        y = vpos_y;
                        endnote_auto_wrap_y_end = None;
                    }
                } else {
                    y = vpos_y;
                }
            } else if let (Some((base_vpos, base_y)), Some(seg)) = (
                para_topbottom_line_vpos_base,
                para.and_then(|p| p.line_segs.get(line_idx)),
            ) {
                y = base_y + hwpunit_to_px(seg.vertical_pos - base_vpos, self.dpi);
            }

            if physical_frame_rows {
                if let Some(row) = para.and_then(|p| p.line_segs.get(line_idx)) {
                    y = y_start + spacing_before + hwpunit_to_px(row.vertical_pos, self.dpi);
                }
            }
            // 다단 필터링: segment_width가 현재 단 너비와 불일치하면 건너뜀
            if let Some(col_w) = multi_col_width_hu {
                if comp_line.segment_width > 0 && (comp_line.segment_width - col_w).abs() > 200 {
                    // char_offset만 진행하고 렌더링 건너뜀
                    for run in &comp_line.runs {
                        char_offset += run.text.chars().count();
                    }
                    if comp_line.has_line_break {
                        char_offset += 1;
                    }
                    continue;
                }
            }

            // 저장 LINE_SEG 없는 실제 빈 문단은 compose의 400HU 안내 줄이 아니라
            // 원래 글자 모양과 줄간격을 사용한다. HeightMeasurer의 동일 보정과
            // 맞춰 pagination과 render의 y advance가 갈라지지 않게 한다.
            let empty_no_lineseg_metrics = if line_idx == 0 {
                para.and_then(|p| {
                    empty_no_lineseg_paragraph_metrics(
                        p,
                        styles,
                        para_style,
                        self.profile.get().hwp3_layout(),
                        self.dpi,
                    )
                })
            } else {
                None
            };

            // 최대 폰트 크기 계산 (line_height 최솟값 보정에도 사용)
            let mut max_fs = comp_line
                .runs
                .iter()
                .map(|r| {
                    let ts = r.text_style(styles);
                    if ts.font_size > 0.0 {
                        ts.font_size
                    } else {
                        12.0
                    }
                })
                .fold(0.0f64, f64::max);
            if let Some((_, _, font_size)) = empty_no_lineseg_metrics {
                max_fs = font_size;
            }
            // [#5854] 통짜 합성 사다리 문서의 빈 문단은 조합 줄에 run 이 하나도 없어
            // 위 fold 가 0 을 준다. 조판(typeset)은 이미 `composed_line_max_font_size`
            // 로 저장 글자모양을 보조 근거로 쓰므로, 렌더도 같은 근거를 써야 두 경로의
            // 줄 진행이 갈라지지 않는다.
            let uniform_filler_ladder = self.uniform_filler_ladder.get();
            if uniform_filler_ladder && max_fs <= 0.0 {
                if let Some(p) = para {
                    max_fs = crate::renderer::composed_line_max_font_size(comp_line, p, styles);
                }
            }
            let mut line_tac_offsets = tac_offsets_for_line(composed, tac_offsets_px, line_idx);
            if let Some(offsets) =
                repeated_empty_tac_line_offset(composed, tac_offsets_px, line_idx)
            {
                line_tac_offsets = offsets;
            }
            if stored_tac_assignment.is_some() {
                line_tac_offsets = tac_offsets_px.to_vec();
            }
            let runs_all_whitespace = comp_line.runs.iter().all(|r| r.text.trim().is_empty());
            // 정렬 폭은 실제 run 방출과 같은 TAC 귀속을 쓴다. 끝 위치 TAC를 빼면
            // 그림은 그리되 Center/Right 시작점이 그림 폭만큼 우측으로 밀린다 (#3257).
            let mut line_tac_offsets_for_width =
                tac_offsets_for_line_width(composed, tac_offsets_px, line_idx);
            if stored_tac_assignment.is_some() {
                line_tac_offsets_for_width = line_tac_offsets.clone();
            }
            // 표의 그리기 전진 폭(#3396)과 정렬 폭을 맞춘다. 공통 flow_width_hu에
            // 더하면 여백을 이미 합산하는 표 전용 문단 경로에서 중복 계산된다.
            if let Some(para) = para {
                for (_, width, ci) in &mut line_tac_offsets_for_width {
                    if let Some(Control::Table(table)) = para.controls.get(*ci) {
                        *width += hwpunit_to_px(table.outer_margin_left as i32, self.dpi)
                            + hwpunit_to_px(table.outer_margin_right as i32, self.dpi);
                    }
                }
            }
            let empty_tac_guide_line = comp_line.runs.is_empty() && !line_tac_offsets.is_empty();
            // LineSeg.line_height는 HWP에서 줄간격이 이미 반영된 값.
            // PARA_LINE_SEG가 없는 폴백(400 HWPUNIT=5.333px) 등 line_height가 폰트 크기보다 작으면,
            // ParaShape의 줄간격 설정(line_spacing_type + line_spacing)으로 올바른 줄 높이를 계산한다.
            let raw_lh = hwpunit_to_px(comp_line.line_height, self.dpi);
            let text_before_picture_line = text_line_is_picture_lead_in(
                para,
                composed,
                tac_offsets_px,
                line_idx,
                raw_lh,
                max_fs,
                self.dpi,
            );
            let ls_val = para_style.map(|s| s.line_spacing).unwrap_or(160.0);
            let ls_type = para_style
                .map(|s| s.line_spacing_type)
                .unwrap_or(LineSpacingType::Percent);
            let raw_text_height = para
                .and_then(|p| p.line_segs.get(line_idx))
                .map(|seg| hwpunit_to_px(seg.text_height, self.dpi))
                .unwrap_or(0.0);
            let use_stored_text_height = para.map(|p| p.controls.is_empty()).unwrap_or(false)
                && (self.profile.get().hwpx_stored_layout() || cell_ctx.is_none());
            let source_metrics_reflow_eligible = para
                .map(|p| crate::renderer::controls_mark_section_start(&p.controls))
                .unwrap_or(false)
                && self.profile.get().hwpx_stored_layout();
            let source_metrics_reflowed = crate::renderer::source_line_metrics_need_reflow(
                raw_lh,
                raw_text_height,
                max_fs,
                ls_type,
                ls_val,
                source_metrics_reflow_eligible,
            );
            let (line_height, line_spacing_px) = empty_no_lineseg_metrics
                .map(|(line_height, line_spacing_px, _)| (line_height, line_spacing_px))
                .unwrap_or_else(|| {
                    // [#5854] 통짜 합성 사다리는 저장 `line_height` 가 글자 크기보다
                    // 크든 작든 실측이 아니다 — 조판(typeset)과 같은 규칙으로 항상
                    // 글꼴·문단 스타일에서 다시 뽑는다.
                    if uniform_filler_ladder && max_fs > 0.0 && !text_before_picture_line {
                        return crate::renderer::corrected_line_metrics(
                            0.0, 0.0, max_fs, ls_type, ls_val,
                        );
                    }
                    crate::renderer::corrected_line_metrics_for_source(
                        raw_lh,
                        raw_text_height,
                        hwpunit_to_px(comp_line.line_spacing, self.dpi),
                        max_fs,
                        ls_type,
                        ls_val,
                        use_stored_text_height,
                        source_metrics_reflow_eligible,
                    )
                });
            // [#2279 진단] 줄별 pitch 분해 — 동작 불변.
            if let Ok(pat) = std::env::var("RHWP_DIAG_PITCH") {
                if para.map(|p| p.text.contains(&pat)).unwrap_or(false) {
                    eprintln!(
                        "DIAG_PITCH li={} raw_lh={:.2} raw_ls={:.2} max_fs={:.2} -> lh={:.2} ls={:.2} stored_ls_cnt={}",
                        line_idx,
                        raw_lh,
                        hwpunit_to_px(comp_line.line_spacing, self.dpi),
                        max_fs,
                        line_height,
                        line_spacing_px,
                        para.map(|p| p.line_segs.len()).unwrap_or(0),
                    );
                }
            }
            // 인라인 Shape(글상자)가 있는 줄: line_height에 Shape 높이가 포함됨
            // Shape는 별도 패스에서 para_y 기준으로 렌더링되므로,
            // 텍스트의 y와 line_height를 폰트 기반으로 보정하여 baseline 정렬
            let has_tac_shape = !tac_offsets_px.is_empty()
                && para
                    .map(|p| {
                        tac_offsets_px.iter().any(|(_, _, ci)| {
                            p.controls
                                .get(*ci)
                                .map(|c| matches!(c, Control::Shape(_)))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false);
            // 도형의 흐름 높이가 저장 프레임이 아니라 `current_height` 에서 오는 줄은
            // 저장 줄 높이를 그대로 둔다 (아래 baseline 정렬 축소 제외).
            let empty_tac_guide_has_explicit_shape_height = empty_tac_guide_line
                && para.is_some_and(|p| {
                    line_tac_offsets.iter().any(|(_, _, ci)| {
                        p.controls.get(*ci).is_some_and(|ctrl| match ctrl {
                            Control::Shape(shape) if shape.common().treat_as_char => {
                                shape.flow_height_hu() > shape.common().height as i32
                            }
                            _ => false,
                        })
                    })
                });
            let (line_height, baseline) = if text_before_picture_line {
                let font_lh = max_fs.max(1.0);
                let font_bl = max_fs * 0.85;
                (font_lh, ensure_min_baseline(font_bl, max_fs))
            } else if has_tac_shape
                && !empty_tac_guide_has_explicit_shape_height
                // [#6632] 셀 안에서는 접지 않는다. 접힌 높이를 되돌리는 바닥값
                // (`layout_column_item` 의 `para_start + max(seg_lh, shape_max_h)`)은 본문
                // 문단에만 있어서, 셀에서 접으면 다음 문단이 도형 높이만큼 위로 올라온다
                // (exam_kor 5쪽 셀: 글자+글상자 줄 lh 26.5 → 18.4, 뒤 그림 줄 8.1px 위).
                // 행 높이 측정은 저장 lh 를 믿으므로 배치도 같은 값을 써야 맞는다.
                && cell_ctx.is_none()
                && raw_lh > max_fs * 1.5
            {
                // Shape와 텍스트가 같은 줄에 있으면 Shape 높이가 line_height에 포함된다.
                // [#1842] 셀 내부에서 max_fs=0(텍스트 없는 tac-전용 줄)이면 이 보정의
                // 전제("Shape 와 텍스트의 baseline 정렬")가 성립하지 않는다 — 종전에는
                // raw_lh > 0*1.5 가 항상 참이라 font_lh=0 으로 퇴화해, 셀 내부
                // tac 묶음 전용 문단의 저장 lh(예: 3401HU)가 소실되고 후속 블록이
                // 통째로 당겨졌다 (3114781 p2 −33pt, 한글 2022 오라클 정합 확인).
                // 본문(cell_ctx 없음)에서 max_fs=0 인 도형-전용 줄이 lh=0 으로 접히는 것은
                // **의도된 짝**이다. 짝의 나머지 반쪽은 `layout_column_item` 의 TAC-Shape
                // 높이 바닥값(`renderer/layout.rs:7037-7076`,
                // `para_start + max(seg_lh, shape_max_h)`)이다 — 접힘이 줄 루프의 진행을
                // 바닥값 아래로 눌러, 문단 진행을 바닥값이 지배하게 만든다. 한컴에 맞춰진
                // 숫자는 그 바닥값(꼬리 줄간격을 포함하지 않는 값)이지 이 줄의 lh 가 아니다.
                //
                // 실측 (`samples/hwp3-sample16-hwp5.hwp` 구역0 문단71,
                // `RHWP_DEBUG_PARA_TAC` + `RHWP_DEBUG_TAC_CURSOR`):
                //   TAC_ADV    pi=71 raw_lh=130.2 lh=0.0 ls=10.4   ← 루프 내 진행은 10.4
                //   TAC_CURSOR FullPara pi=71 dy=130.2            ← 바닥값이 문 결과
                // 접힘만 없애면 루프 진행이 raw_lh+ls=140.6 으로 바닥값 130.2 를 넘어
                // 문단이 정확히 `LineSeg.line_spacing`(10.4px) 만큼 밀리고,
                // `tests/issue_1116.rs` 의 한컴 PDF 대조 핀 둘이 그만큼 깨진다.
                //
                // 보상자는 `HeightCursor::vpos_adjust` 가 **아니다** — 그 함수는 이 문단
                // 다음 항목(pi=72)에서 `lazy_base < 0` 으로 조기 반환한다
                // (`RHWP_VPOS_DEBUG` → `VPOS_CORR_SKIP: pi=72 ... lazy_base=-72`).
                // 이 자리의 옛 주석이 "reserved/skip-advance 보상 기계"를 지목해 앞선
                // 조사를 `vpos_adjust` 로 잘못 보냈다 (#4333).
                let font_lh = max_fs * 1.2; // 폰트 크기의 120%
                let font_bl = max_fs * 0.85;
                (font_lh, ensure_min_baseline(font_bl, max_fs))
            } else {
                // [#5825] 퇴화 저장 baseline 클램프 — 기계생성 통계표는 lineseg 에
                // baseline == textheight(하강부 0)를 저장한다(156673604 34쪽 표 두 개:
                // bl=1100=vertsize·spacing=0). 받침이 내려갈 자리가 없어 글자가 아래
                // 괘선을 지나간다. 한글 2022 는 이 값을 무시하고 표준 ascent 로
                // 그린다(실측 12.62px = 0.86×; 같은 문서의 정상 표 저장값도
                // 935 = 0.85×1100). 하강부가 0 인 baseline 만 0.85×textheight 로
                // 되돌리고, 정상 저장 baseline(bl < th)은 그대로 둔다.
                let stored_bl = hwpunit_to_px(comp_line.baseline_distance, self.dpi);
                let stored_bl = if raw_text_height > 0.0 && stored_bl >= raw_text_height - 0.01 {
                    raw_text_height * 0.85
                } else {
                    stored_bl
                };
                (
                    line_height,
                    ensure_min_baseline(
                        crate::renderer::corrected_line_baseline_for_source(
                            stored_bl,
                            max_fs,
                            source_metrics_reflowed,
                        ),
                        max_fs,
                    ),
                )
            };
            // 들여쓰기/내어쓰기: 문단 여백은 무조건 적용
            // - 보통(ind=0): 모든 줄 margin_left
            // - 들여쓰기(ind>0): 첫줄 margin_left+indent, 다음줄 margin_left
            // - 내어쓰기(ind<0): 첫줄 margin_left, 다음줄 margin_left+|indent|
            //
            // [Issue #6190] **저장 LINE_SEG 의 `TAG_INDENTATION`(bit 20)이 정답지다.**
            // 이 비트는 "이 줄에 들여쓰기가 적용됐다"는 한글의 줄별 기록이다. 비트가
            // 꺼진 줄에 우리가 들여쓰기를 얹으면 그 줄과, 그 문단이 호스트하는 표까지
            // 함께 밀린다(156458354 3쪽 `경 력 사 항` +68.1px, 마지막 표는 용지 밖 36px).
            //
            // 한글 통제 실험으로 확인했다 — 같은 문단의 `indent` 만 바꿔 한글로 PDF 를
            // 떠서 재면:
            //
            // | 문서 | ls[0].tag | indent 0→20445 스윕 | 한글 x |
            // |---|---|---|---|
            // | 156458354 pi=28 | `0x60000` (bit20 꺼짐) | 0 · 2000 · 6000 · 10000 · 20445 | **전부 345.60 (불변)** |
            // | 36313646 pi=2 | `0x160000` (bit20 켜짐) | 0 · 660 · 4000 · 10000 · 20445 | 352.77 → 420.89 (**정확히 indent/4 씩**) |
            //
            // 내어쓰기 문단이 `ls[0]=0x60000, ls[1..]=0x160000` 인 것도 같은 의미다 —
            // 내어쓰기는 둘째 줄부터 적용되고, 비트가 줄마다 그것을 기록한다.
            // 합성 사다리(`TAG_IMPLEMENTATION_PROPERTY`)는 이 증언이 없으므로 제외한다.
            //
            // 편집으로 줄 수가 달라진 문단은 저장 사다리가 더는 이 조판을 설명하지
            // 못한다 — 그때는 비트도 낡은 기록이다(#6204 계열). `composed.lines` 와
            // 저장 세그 수가 같을 때만 증언으로 쓴다.
            //
            // **본문 흐름 한정**이다 — 표 셀 안 문단은 한글이 들여쓰기를 적용한다
            // (오라클 7문서: 2777015 · 156548319 · 156658621 · 156428389 등 모두 셀 안
            // Center 문단이고 한글 x 가 `indent/2` 반영값과 0.0~0.4px 로 일치).
            let stored_ladder_covers_lines = cell_ctx.is_none()
                && para.is_some_and(|p| p.line_segs.len() == composed.lines.len());
            let stored_seg_denies_indent = stored_ladder_covers_lines
                && para
                    .and_then(|p| p.line_segs.get(line_idx))
                    .is_some_and(|seg| {
                        seg.tag & LineSeg::TAG_IMPLEMENTATION_PROPERTY == 0
                            && seg.tag & LineSeg::TAG_INDENTATION == 0
                    });
            let line_indent = if stored_seg_denies_indent {
                0.0
            } else {
                crate::renderer::equation_tac_flow::paragraph_line_indent(indent, line_idx)
            };
            let styled_margin_left = margin_left + line_indent;

            // [Task #489] Picture/Shape Square wrap (어울림) 시 LINE_SEG.cs/sw 적용.
            // 한컴이 인코딩한 정답값을 그대로 사용 (휴리스틱 없음).
            // 표 Square wrap 케이스는 caller 가 col_area 를 이미 wrap_area 로 좁혀
            // 호출하므로 segment_width ≈ col_area_w_hu → 조건 미발동 (회귀 차단).
            // 200 HU 임계값은 paragraph_layout 의 multi-col filter 와 동일 (페이지네이션 노이즈 제거).
            //
            // [Task #568] 인라인 TAC 표(treat_as_char=true) 가 있는 줄도 동일 처리.
            // HWP 는 인라인 TAC 표가 있는 줄의 segment_width 를 표 폭 + 잔여로 좁게
            // 인코딩한다 (wrap=TopAndBottom 영향). col_area.width 로 잡으면
            // Justify slack 이 과대 산출되어 선두 공백이 80 px/space 로 부풀어 표를
            // 우측으로 민다 (exam_science.hwp pi=61 12번 응답: +175 px 편위).
            let line_has_inline_tac_table = !tac_offsets_px.is_empty()
                && para
                    .map(|p| {
                        line_tac_offsets.iter().any(|(_, _, ci)| {
                            matches!(p.controls.get(*ci),
                            Some(Control::Table(t)) if t.common.treat_as_char)
                        })
                    })
                    .unwrap_or(false);

            // [Task #568] 임계값에 column_start 포함 — 실제 가용 line 폭은 (sw + cs).
            // 단락 들여쓰기를 LINE_SEG.column_start 로 인코딩한 paragraph 의
            // 정상 라인은 (sw + cs) ≈ col_w_hu 이므로 새 분기 미진입.
            // Picture/Shape Square wrap 은 cs=0 이라 기존 동작과 동일.
            let line_avail_hu = comp_line
                .segment_width
                .saturating_add(comp_line.column_start);
            // [Task #901] cs > 0 + sw < col_w 인 경우도 effective_col_x 적용.
            // pic2.hwp paragraph 0 의 ls[1] (cs=39123 sw=3397, avail=col_w) 같은
            // wrap zone 우측 영역 case 의 X 위치 정합 — paragraph 0 의 한글 char
            // ("우/리/나/라") 가 그림 사이/우측 좁은 영역에 그려져야 함.
            // 기존 조건 `avail < col_w - 200` 만으로는 avail == col_w 인 wrap zone
            // 라인이 분기 미진입 → col_area.x 좌측에 잘못 그려짐.
            let cs_significant = comp_line.column_start > 0
                && comp_line.segment_width > 0
                && comp_line.segment_width < col_area_w_hu;
            // [Task #1440] anchor 매칭이 없는 후속 body 문단이라도 LINE_SEG 자체가
            // 단 폭보다 확연히 좁은 wrap zone 을 보존하면 그 저장 폭을 따른다.
            // 정상 들여쓰기 계열은 cs+sw ~= col_w 이므로 제외한다.
            //
            // LineSeg cs/sw 만으로 wrap zone 을 판정하면 paragraph border 박스의 내부
            // inset도 그림 어울림으로 오인된다(#547 passage box, #1440 6쪽 지문 박스).
            // anchor 메타데이터가 없는 fallback 보정은 같은 문단 안에서 실제로 좁은 줄과
            // 넓은 줄이 섞인 precomputed picture-wrap 흐름에만 제한한다.
            let para_has_mixed_segment_widths = para
                .map(|p| {
                    let mut min_sw = i32::MAX;
                    let mut max_sw = 0;
                    for seg in p.line_segs.iter().filter(|seg| seg.segment_width > 0) {
                        min_sw = min_sw.min(seg.segment_width);
                        max_sw = max_sw.max(seg.segment_width);
                    }
                    min_sw != i32::MAX && max_sw.saturating_sub(min_sw) > 1000
                })
                .unwrap_or(false);
            let precomputed_body_wrap_line = cell_ctx.is_none()
                && para_has_mixed_segment_widths
                && comp_line.segment_width > 0
                && line_avail_hu < col_area_w_hu - 200
                && para
                    .and_then(|p| p.line_segs.get(line_idx))
                    .map(|seg| seg.is_in_wrap_zone(col_area_w_hu))
                    .unwrap_or(false);
            // [#5677] `column_start` 가 **문단 자신의 `margin_left`** 로 설명되면 그
            // 좁음의 출처는 외부 기하가 아니라 문단 여백이다. 그런 줄에 저장 기하를
            // 쓰면 `effective_col_x = col_area.x + cs` 로 여백을 한 번 먹고, 아래
            // `hwp5_stored_line_start_eligible` 이 `!uses_stored_segment_geometry` 를
            // 요구해 거짓이 되므로 `margin_left` 를 **또** 더한다.
            //
            // hwp3-sample 문단 53(빈 본문, `margin_left=18.56px`, 저장 `cs=1392HU`)이
            // 그 형상으로, 줄 좌단이 단 좌단 + **37.12px**(=2×18.56)에 놓였다.
            // `ParagraphBox::body_for_style` 의 원점 차단이 `head_type` 을 키로 잡아
            // `HeadType::None` 인 본문 문단은 빠져 있었는데, 그 주석이 스스로 적어
            // 두었듯 위험은 목록이 아니라 **비어 있음**에 있다.
            let own_margin_hu = crate::renderer::px_to_hwpunit(margin_left, self.dpi);
            let cs_is_own_margin = comp_line.column_start.abs_diff(own_margin_hu)
                <= EMPTY_LINE_OWN_MARGIN_TOLERANCE_HU as u32;
            let empty_stored_wrap_line = cell_ctx.is_none()
                && para
                    .map(|p| p.text.is_empty() && p.controls.is_empty())
                    .unwrap_or(false)
                && comp_line.column_start > 0
                && !cs_is_own_margin
                && comp_line.segment_width > 0
                && comp_line.segment_width < col_area_w_hu;
            // [#5818] 어울림(Square 계열) float 그림이 있는 **셀** 의 줄도 저장
            // cs/sw(한컴이 인코딩한 wrap 배제)를 존중한다. 종전 게이트는 전부
            // cell_ctx.is_none() 이라 셀 줄이 배제를 무시하고 셀 왼끝에서 시작해
            // 로고를 파고들었다(156599239 머리 표: 저장 cs=4037HU=53.8px, 한글
            // 실측 x=151.7 = 셀 콘텐츠 왼끝+cs ↔ rhwp 102.2). 신호는 같은 셀에
            // Square float 가 실재할 때만 켜져(#547 문단 테두리 inset 오인 차단),
            // cs>0 && sw<셀폭 인 줄에 한정한다.
            let cell_square_wrap_stored_line = cell_ctx.is_some()
                && self.cell_has_square_float.get()
                && comp_line.column_start > 0
                && comp_line.segment_width > 0
                && comp_line.segment_width < col_area_w_hu;
            // [#6175] 문단 **전체**가 개체 옆에 들어가면 같은 문단 안에 넓은 줄이
            // 없어 `precomputed_body_wrap_line`(혼합 폭)이 발화하지 않는다. 그때는
            // 문서에 실재하는 같은 세로 band의 어울림 개체가 증거다 — 저장 행이 남긴
            // 결손 폭과 위치를 그 개체가 함께 설명하면 좁음의 출처는 외부 기하다.
            //
            // ⚠ "균일하게 좁다"만으로 켜면 문단 테두리 박스의 inset 을 어울림으로
            // 오인해 #547·#1440 핀이 깨진다(#6129 반증). 판별은 개체 폭과 세로 band 대조다 —
            // 셀의 #5818 계약("같은 셀에 Square float 실재")과 같은 원리의 본문 판.
            // 컴포저의 `stored_rows_require_external_geometry` 가 같은 증거로 저장
            // 행을 지켜 두므로, 두 층이 같은 판정을 공유한다.
            let body_square_wrap_stored_line = cell_ctx.is_none()
                && !para_has_mixed_segment_widths
                && comp_line.column_start == 0
                && comp_line.segment_width > 0
                && {
                    let evidence = self.body_float_carve_evidence.borrow();
                    let missing = col_area_w_hu.saturating_sub(line_avail_hu);
                    !evidence.is_empty()
                        && missing > 1200
                        && para.is_some_and(|paragraph| {
                            evidence.iter().any(|candidate| {
                                candidate.matches_stored_rows(missing, &paragraph.line_segs, 1200)
                            })
                        })
                };
            // 셀 안 TAC 줄의 저장 폭이 문단 오른쪽 여백만큼 좁으면 어울림 영역이
            // 아니라 이미 여백을 뺀 폭이다. 이를 다시 열 폭으로 쓰면 아래에서
            // 여백을 두 번 뺀다(issue_1285: 4px + 4px). 실제로 더 좁은 줄은 유지한다.
            let inline_tac_segment_is_paragraph_width = cell_ctx.is_some()
                && comp_line.column_start == 0
                && styled_margin_left == 0.0
                && margin_right > 0.0
                && comp_line
                    .segment_width
                    .abs_diff(px_to_hwpunit(col_area.width - margin_right, self.dpi))
                    <= 1;
            // NO_LS 문단의 comp_line cs/sw 는 저장 기하가 아니라 프레임 재래핑이
            // 방금 새긴 합성값이다 — cs 가 문단 자신의 margin_left 라서 저장 기하로
            // 읽으면 `col_x + cs` 로 여백을 한 번 먹고 아래 일반 여백 처리가 또
            // 더한다(#5677 과 같은 이중 적용, 2×18.3px). 저장 lineseg 가 있을 때만
            // 이 경로를 연다.
            let stored_geometry_source = para
                .map(|p| !crate::renderer::para_has_no_stored_line_segs(p))
                .unwrap_or(false);
            let uses_stored_segment_geometry = physical_frame_rows
                || (stored_geometry_source
                    && (has_picture_shape_square_wrap
                        || (line_has_inline_tac_table && !inline_tac_segment_is_paragraph_width)
                        || precomputed_body_wrap_line
                        || empty_stored_wrap_line
                        || body_square_wrap_stored_line
                        || cell_square_wrap_stored_line)
                    && comp_line.segment_width > 0
                    && (line_avail_hu < col_area_w_hu - 200 || cs_significant));
            let (effective_col_x, effective_col_w) = if let Some(rows) = physical_rows {
                // Already resolved by the line owner, including half-HU insets.
                // Do not infer indentation or quantize this physical interval.
                let row = &rows[line_idx];
                (col_area.x + row.start, row.end - row.start)
            } else if uses_stored_segment_geometry {
                let cs_px = hwpunit_to_px(comp_line.column_start, self.dpi);
                let sw_px = hwpunit_to_px(comp_line.segment_width, self.dpi);
                (col_area.x + cs_px, sw_px)
            } else {
                (col_area.x, col_area.width)
            };
            let profile = self.profile.get();
            let hwp5_stored_line_start_eligible = cell_ctx.is_none()
                && self.is_body_flow_col_area(col_area)
                && matches!(alignment, Alignment::Justify | Alignment::Left)
                && wrap_anchor.is_none()
                && !uses_stored_segment_geometry
                && composed.numbering_text.is_none()
                && para.map(|p| p.controls.is_empty()).unwrap_or(false)
                // [#3837] rhwp 가 HWP5 원본에서 내보낸 HWPX 도 같은 계약이다 — 저장
                // LINE_SEG 가 그 HWP5 의 것이라 `column_start` 가 여전히 권위다. 이 조건이
                // 없으면 왕복만으로 들여쓴 줄이 왼쪽으로 밀린다(1370000-200800015: 저장
                // cs=22677 = 302.4px 가 무시돼 글리프 595개가 그만큼 이동).
                // 원본 HWPX 는 건드리지 않는다 — 그쪽 저장 계약은 별개 축이다.
                && uses_hwp5_stored_line_start_profile(profile);
            // 암호 HWP3의 Square-wrap Picture/Shape 저장 cs/sw는 문단 좌·우 inset까지
            // 포함한 완성 line box다. 여기서 ParaShape margin을 다시 더하거나 빼면
            // 그림과 글자 사이에 여백이 한 번 더 생기고 right edge도 불필요하게 줄어든다.
            // 일반 HWP3/HWP5의 저장 segment 계약은 다르므로 기존 여백 처리를 유지한다.
            let hwp3_password_stored_segment_line_box =
                uses_stored_segment_geometry && self.profile.get().hwp3_password_layout();
            // [#4690] 저장 cs/sw 조각이 문단 여백을 담지 못하면 여백을 적용하지 않는다.
            //
            // 이 경로의 `effective_col_x/w` 는 이미 저장 `cs`/`sw` 가 정한 줄 상자다. 그
            // 상자가 좁은데 `margin_left + line_indent + margin_right` 를 그대로 얹으면
            // 줄이 상자 오른쪽 밖에서 시작하고 폭이 음수가 된다 — 그 줄의 글자는 정상적으로
            // 그려질 수 없다(30098 p3 pi48 L1: x=721.2 폭 −1.6, 문서 전체 18줄. 저장
            // 사다리 값은 x=679.7 폭 38.4). 여백이 담기지 않는다는 것 자체가 그 조각을
            // 완성된 line box 로 읽어야 한다는 신호이므로, 암호 HWP3 경로와 같은 처리를
            // 한다.
            //
            // 이 가드는 **여백이 상자를 넘칠 때만** 발동한다. 여백이 들어가는 정상 어울림
            // 줄은 종전대로 둔다 — `line_indent` 를 이 경로에서 일괄로 빼면 wrap 텍스트가
            // 그림 영역으로 침범한다(#1230 `exam_science` pi=21). 저장 cs 가 내어쓰기를
            // 대체한다는 해석도 성립하지 않는다: 정답지 `pdf/exam_kor-2022.pdf`
            // (Hwp 2022 12.0.0.4426) p5 에서 첫/마지막 lineseg 의 cs 가 둘 다 1130 으로
            // 같은 문단인데도 한/글은 이어지는 줄을 `|indent|` 만큼 들여 그린다
            // (99.12pt ↔ 110.4pt = 132.16px ↔ 147.20px @96dpi). 즉 cs 는 그 줄의 확정
            // 시작점이 아니라 문단 왼쪽 여백이다.
            let stored_segment_line_box_cannot_hold_margins = uses_stored_segment_geometry
                && !hwp3_password_stored_segment_line_box
                && styled_margin_left + margin_right >= effective_col_w;
            // 저장 줄의 cs가 문단의 왼쪽 여백 자체이면 완성 줄 상자에 같은 여백을
            // 두 번 더하지 않는다. 셀의 cs/안쪽 여백 계약과는 구분한다 (#6706).
            let stored_inline_own_margin = uses_stored_segment_geometry
                && cell_ctx.is_none()
                && cs_is_own_margin
                && stored_tac_assignment.is_some();
            let (effective_margin_left, effective_margin_right) = if physical_frame_rows
                || stored_inline_own_margin
                || hwp3_password_stored_segment_line_box
                || stored_segment_line_box_cannot_hold_margins
            {
                (0.0, 0.0)
            } else {
                (
                    authoritative_stored_line_start_px(
                        styled_margin_left,
                        para.and_then(|p| p.line_segs.get(line_idx)),
                        col_area_w_hu,
                        self.dpi,
                        hwp5_stored_line_start_eligible,
                    ),
                    margin_right,
                )
            };

            // [#5598] 내어쓰기가 줄 상자를 한 글자도 못 담을 만큼 먹으면 적용하지 않는다.
            //
            // 좁은 표 칸에서 문단 내어쓰기(|indent|)가 칸 안쪽 폭에 육박하면, 이어지는 줄의
            // 상자가 몇 px 로 무너져 글자가 칸 오른쪽 밖으로 밀려 나간다(2995759 `분류처우위원회
            // 심의ㆍ의결` 칸: 안쪽 폭 107.7px, indent −104.4px → 둘째 줄 상자 x=193.3 w=3.3,
            // `의결` 이 칸 밖). 한글은 같은 문단의 두 줄을 모두 칸 안쪽 폭으로 조판한다
            // (저장 LINE_SEG 두 줄 모두 cs=200 sw=8076).
            //
            // 첫 줄은 내어쓰기의 기준선이므로 건드리지 않고, 이어지는 줄에만 적용한다.
            let effective_margin_left = if line_indent > 0.0 {
                let min_line_w = max_fs.max(1.0);
                let avail = effective_col_w - effective_margin_left - effective_margin_right;
                if avail < min_line_w {
                    margin_left.min(effective_margin_left)
                } else {
                    effective_margin_left
                }
            } else {
                effective_margin_left
            };

            // 인라인 Shape가 있는 줄: 텍스트 y를 Shape 하단 baseline에 맞춤
            let text_y = if has_tac_shape
                && !empty_tac_guide_has_explicit_shape_height
                && raw_lh > max_fs * 1.5
            {
                // raw_lh는 Shape 높이 포함 원본 줄 높이, line_height는 폰트 기반 보정 높이
                // 텍스트를 Shape 하단 근처로 이동 (Shape 높이 - 폰트 줄 높이)
                y + (raw_lh - line_height).max(0.0)
            } else {
                y
            };
            // Task #332 Stage 4b: clamp 제거. 단 하단을 초과하는 줄은 그대로 그린다
            // (시각 경계 약간 넘김 허용). 기존의 `text_y = col_bottom - line_height`
            // 클램프는 여러 overflow 줄을 같은 y 에 piling 해 글자 겹침을 만들었으나,
            // 클램프 없이 원래 y 에 그리면 piling 자체가 발생하지 않는다. 콘텐츠 손실
            // (stop drawing) 도 발생하지 않으며, drift 의 본질적 해결은 Stage 5 에서.
            let col_bottom = col_area.y + col_area.height;
            let line_visual_bottom = text_y + line_height;
            let is_body_flow_col_area = self.is_body_flow_col_area(col_area);
            let is_endnote_virtual_para = para_index >= self.endnote_para_base.get();
            let blank_spacer_line = is_blank_spacer_line(
                para,
                is_endnote_virtual_para,
                runs_all_whitespace,
                &line_tac_offsets,
            );
            let equation_only_endnote_tail_line = is_body_flow_col_area
                && cell_ctx.is_none()
                && is_endnote_virtual_para
                && line_idx + 1 >= end
                && is_equation_only_tac_line(para, runs_all_whitespace, &line_tac_offsets);
            let tolerated_endnote_bottom_bleed = self.is_tolerated_current_endnote_bottom_bleed(
                is_body_flow_col_area && cell_ctx.is_none() && is_endnote_virtual_para,
                line_visual_bottom,
                col_bottom,
                equation_only_endnote_tail_line,
            );
            if is_body_flow_col_area
                && cell_ctx.is_none()
                && line_visual_bottom > col_bottom + 0.5
                && !blank_spacer_line
                && !tolerated_endnote_bottom_bleed
            {
                eprintln!(
                    "LAYOUT_OVERFLOW_DRAW: section={} pi={} line={} y={:.1} col_bottom={:.1} overflow={:.1}px",
                    section_index, para_index, line_idx,
                    line_visual_bottom, col_bottom, line_visual_bottom - col_bottom,
                );
            }
            // [#3637] 셀 안 줄이 **쪽 본문 하단**을 넘는 경우.
            //
            // 위 진단은 `is_body_flow_col_area && cell_ctx.is_none()` 이라 본문 흐름만
            // 본다. 셀은 `col_area` 가 셀 사각형이라 그 조건이 언제나 거짓이고, 그래서
            // 셀 안에서 쪽 밖으로 나간 글자는 **한 줄도 보고되지 않았다**.
            //
            // 실측: 쪽 밖 글자가 있는 문서 91건 중 8건이 이 침묵 구간이었다
            // (총 2,910자, 최대 471.8px 초과). 텍스트 추출에는 남아 있어 텍스트 diff 로도
            // 안 잡히고, 진단마저 없어 관측 자체가 불가능했다.
            //
            // 기준선 두 가지가 함께 맞아야 오탐이 사라진다.
            //
            // 1. **쪽 하단** (본문 하단 아님). 본문 하단과 쪽 하단 사이는 아래 여백·꼬리말
            //    구간이라 거기 그려진 글자는 실제로 보인다. 본문 하단으로 재면 그 구간이
            //    통째로 오탐이 된다.
            // 2. 줄의 **윗변**(`text_y`). 아랫변으로 재면 마지막 줄 디센더가 경계를 스치는
            //    정상 상태까지 잡는다. 윗변이 이미 쪽 밖이면 그 줄은 **어느 부분도 그려지지
            //    않는다** — 배율·글꼴에 무관한 판정이다.
            //
            // MATCH 대조군 80건 실측: 아랫변 기준은 9건(11%) 오탐, 초과폭이 전부
            // 5.4~23.9px(줄 높이 이내)였다. 윗변으로 바꾸니 7건, 기준을 쪽 하단으로 옮겨야
            // 0 이 된다. 진짜 침묵 구간 8건은 146.0~512.0px 라 어느 기준에서도 남는다.
            if cell_ctx.is_some() && !blank_spacer_line {
                let page_h = self.current_page_height.get();
                if page_h > 0.0 && text_y > page_h + 0.5 {
                    // [#3668] stderr 진단과 같은 조건에서 집계 카운터도 올린다.
                    self.overflow_cell_lines
                        .set(self.overflow_cell_lines.get() + 1);
                    eprintln!(
                        "LAYOUT_OVERFLOW_CELL: section={} pi={} line={} y={:.1} \
                         page_bottom={:.1} overflow={:.1}px",
                        section_index,
                        para_index,
                        line_idx,
                        text_y,
                        page_h,
                        text_y - page_h,
                    );
                    if std::env::var("RHWP_DIAG_OVERFLOW_CELL").is_ok() {
                        eprintln!(
                            "DIAG_OVERFLOW_CELL_CTX: section={} pi={} line={} ctx={cell_ctx:?}",
                            section_index, para_index, line_idx,
                        );
                    }
                }
            }
            // [Task #604 R3] wrap_anchor 가 있으면 본 문단은 anchor 그림/표 옆 wrap text.
            // 각 라인의 LineSeg cs(column_start)/sw(segment_width)를 x 오프셋/너비로 적용.
            // typeset 의 wrap_around state machine 매칭 결과 (ColumnContent.wrap_anchors)
            // 가 layout 에 전달되어 본 분기가 동작.
            //
            // [Task #722] inter-image-text gap 보정 — 한컴 viewer 는 anchor image 의
            // outer margin_right (HU) 만큼 cs 에 더해 text 시작 x 결정. sw 에서 동일량
            // 차감하여 가용 폭 정합. WrapAnchorRef.anchor_image_margin_right 활용.
            //
            // `LineSeg.sw`는 문단의 left/right margin을 포함한 source line box 폭이다.
            // 따라서 일반 stored-segment 경로와 마찬가지로 TextLine bbox의 usable width에서는
            // margin을 빼야 한다. wrap-anchor 경로가 `sw`를 그대로 override하면 hanging
            // indent가 image 쪽으로 한 번 더 돌출한다(HWP5 p127 그림 56 / p156 그림 64).
            let (line_cs_offset, line_avail_w_override) = if let Some(anchor) = wrap_anchor {
                let seg = para.and_then(|p| p.line_segs.get(line_idx));
                // NO_LS 문단은 줄별 저장 cs/sw 가 없으므로
                // 합성 anchor 의 존을 모든 줄에 적용한다.
                let (cs, sw, synthetic_zone) = match seg {
                    Some(s) => (s.column_start as i32, s.segment_width as i32, false),
                    None => {
                        // 줄 단위 배제 밴드: anchor 에 y 밴드가 실려 있으면 그
                        // 밴드와 교차하는 줄에만 감폭을 적용한다(출석부 형상 —
                        // 문단 첫 줄은 전폭, 개체 옆 줄만 회피).
                        let in_band = anchor.band_y_range.is_none_or(|(band_top, band_bottom)| {
                            // 눈금 오차(판정/렌더 장부 차)에 강하도록 줄 중심으로 판정.
                            let line_center = text_y - y_start + line_height * 0.5;
                            line_center > band_top - 2.0 && line_center < band_bottom + 2.0
                        });
                        if in_band {
                            (anchor.anchor_cs, anchor.anchor_sw, true)
                        } else {
                            (0, 0, false)
                        }
                    }
                };
                let mr = anchor.anchor_image_margin_right;
                let cs_px = crate::renderer::hwpunit_to_px(cs + mr, self.dpi);
                if synthetic_zone {
                    // 합성 배제 존: 한글의 문단 왼 여백은 **열 기준** 들여쓰기라,
                    // 개체 회피 지점이 이미 여백보다 오른쪽이면 여백은 소진된다.
                    // cs 와 margin 을 가산하면 아이콘 옆 제목이 여백만큼 한 번 더
                    // 벌어진다(재현: 아이콘 오른쪽 +3.8 이어야 할 제목이 +22.4).
                    // x 계산부(아래 bbox)가 margin 을 더하므로 여기서는 여백을
                    // 넘는 초과분만 offset 으로 남긴다.
                    let absorbed_cs = (cs_px - effective_margin_left).max(0.0);
                    // 텍스트 시작 = col + margin_l + absorbed_cs = col + max(margin_l, cs).
                    // 가용 폭은 배제 존 폭(sw)에서 시작이 cs 보다 오른쪽으로 밀린
                    // 양(max(0, margin_l - cs))과 오른 여백만 뺀다.
                    let sw_px = if sw > 0 {
                        Some(
                            (crate::renderer::hwpunit_to_px((sw - mr).max(0), self.dpi)
                                - (effective_margin_left - cs_px).max(0.0)
                                - effective_margin_right)
                                .max(0.0),
                        )
                    } else {
                        None
                    };
                    (absorbed_cs, sw_px)
                } else {
                    let sw_px = if sw > 0 {
                        Some(
                            (crate::renderer::hwpunit_to_px((sw - mr).max(0), self.dpi)
                                - effective_margin_left
                                - effective_margin_right)
                                .max(0.0),
                        )
                    } else {
                        None
                    };
                    (cs_px, sw_px)
                }
            } else {
                (0.0, None)
            };

            let line_id = tree.next_id();
            let mut line_node = RenderNode::new(
                line_id,
                RenderNodeType::TextLine({
                    let vpos = para
                        .and_then(|p| p.line_segs.get(line_idx))
                        .map(|ls| ls.vertical_pos)
                        .unwrap_or(0);
                    TextLineNode::with_para_vpos(
                        line_height,
                        baseline,
                        section_index,
                        para_index,
                        line_idx as u32,
                        vpos,
                    )
                }),
                BoundingBox::new(
                    // [Task #604 R3] wrap_anchor 가 있으면 line_cs_offset 사용 (col_area.x 기준),
                    // 아니면 Task #489 effective_col_x 사용. 두 경로 중복 적용 방지.
                    if wrap_anchor.is_some() {
                        col_area.x + effective_margin_left + line_cs_offset
                    } else {
                        effective_col_x + effective_margin_left
                    },
                    text_y,
                    line_avail_w_override.unwrap_or(
                        effective_col_w - effective_margin_left - effective_margin_right,
                    ),
                    line_height,
                ),
            );

            let inline_offset = if line_idx == start_line {
                first_line_x_offset + endnote_marker_x_advance
            } else {
                0.0
            };
            // 번호/글머리표 마커: 모든 줄에서 마커 폭만큼 가용폭 차감 (행잉 인덴트)
            let num_offset = if numbering_width > 0.0 {
                numbering_width
            } else {
                0.0
            };
            let available_width = line_avail_w_override
                .map(|w| w - inline_offset - num_offset)
                .unwrap_or(
                    effective_col_w
                        - effective_margin_left
                        - effective_margin_right
                        - inline_offset
                        - num_offset,
                );
            // [Task #1472] IR indent 를 full 로 되돌리면서(parser/mod.rs) 미주 TAC 수식
            // available_width 의 effective indent 를 불변 유지: 변환본은 scale 을 절반으로.
            // (종전: IR(half)×2.0=full → 현재: IR(full)×1.0=full)
            let equation_indent_scale = (if cell_ctx.is_some() { 1.0 } else { 2.0 })
                * if self.profile.get().hwp3_layout() {
                    0.5
                } else {
                    1.0
                };
            let equation_first_effective_margin_left =
                crate::renderer::equation_tac_flow::paragraph_effective_margin_left_with_indent_scale(
                    margin_left,
                    indent,
                    0,
                    equation_indent_scale,
                );
            let equation_continuation_effective_margin_left =
                crate::renderer::equation_tac_flow::paragraph_effective_margin_left_with_indent_scale(
                    margin_left,
                    indent,
                    1,
                    equation_indent_scale,
                );
            let equation_first_available_width = line_avail_w_override
                .map(|w| w - inline_offset - num_offset)
                .unwrap_or(
                    effective_col_w
                        - equation_first_effective_margin_left
                        - effective_margin_right
                        - inline_offset
                        - num_offset,
                );
            let equation_continuation_available_width = line_avail_w_override
                .map(|w| w - inline_offset - num_offset)
                .unwrap_or(
                    effective_col_w
                        - equation_continuation_effective_margin_left
                        - effective_margin_right
                        - inline_offset
                        - num_offset,
                );
            let equation_tac_line_flow =
                crate::renderer::equation_tac_flow::compute_equation_only_tac_line_flow(
                    para,
                    composed,
                    tac_offsets_px,
                    line_idx,
                    if cell_ctx.is_some() {
                        f64::INFINITY
                    } else {
                        equation_first_available_width
                    },
                    if cell_ctx.is_some() {
                        f64::INFINITY
                    } else {
                        equation_continuation_available_width
                    },
                );
            let equation_tac_extra_rows = equation_tac_line_flow
                .as_ref()
                .map(|flow| flow.extra_rows)
                .unwrap_or(0);
            // [#6656] 문단 안 다음 줄까지의 전진은 저장 줄의 **글자 높이(th)** 다. 줄 상자
            // 높이(lh)는 그 줄이 품은 개체까지 덮지만, 한/글은 다음 줄을 th + ls 자리에
            // 놓고 개체가 아래 줄 공간을 침범하게 둔다. 코퍼스 전수(samples↔pdf 215문서,
            // lh≠th 인 문단 안 연속 줄): th+ls 45건 / lh+ls 1건.
            // 예) hwpctl_ParameterSetID_Item_v1.2 문단 0.7: ls[2] vpos=0 lh=1560 th=1000
            // ls=600 → ls[3] vpos=1600 (= th+ls). 한/글 3쪽 둘째 줄 89.3, rhwp 는 96.8.
            // 상자 높이(`line_height`)는 그대로 두고 전진만 줄인다.
            // 전진값은 추정하지 않고 **저장 사다리의 다음 줄 vpos 차**를 그대로 쓴다.
            // 세 가지를 함께 요구한다. ① 사다리를 다시 짜지 않은 문단일 것. ② 이 줄의 상자
            // 높이가 저장 `lh` 그대로일 것 — 재조판된 상자에 저장 전진을 섞으면 사다리도
            // 상자도 아닌 값이 된다. ③ 전진이 실제 글자(`max_fs`)를 담을 것 — HWP3 변환본
            // 처럼 낡은 값이면 다음 줄이 글자 위로 올라온다(hwp3-empty-cell 겹침 1건).
            let stored_line_advance = para.and_then(|p| {
                let seg = p.line_segs.get(line_idx)?;
                let next = p.line_segs.get(line_idx + 1)?;
                // [#6928] 줄 바닥은 **글자 높이**(`max_fs`)로 지켜 왔는데, 글자처럼 취급
                // 개체(그림·표)가 줄 높이를 정하는 줄에는 글리프가 없어 `max_fs` 가 0 이다.
                // 그래서 저장 사다리가 주는 작은 걸음이 무방비로 통과하고, 뒤 내용이 그
                // 개체 위로 포개진다 — 148769979 1쪽: 배너 그림 높이 107.1px 인 줄의
                // 저장 걸음이 2.7px 라 표·엠바고·로고가 전부 106px 위로 올라왔다.
                //
                // 그런 줄의 바닥은 개체가 정한 줄 높이 자체다. `step < line_height` 와 함께
                // 걸리므로 이 줄은 저장 걸음을 쓰지 않고 `line_height` 로 전진한다.
                let line_has_as_char_object = composed.inline_controls.iter().any(|c| {
                    c.line_index == line_idx
                        && matches!(
                            c.control_type,
                            crate::renderer::composer::InlineControlType::Table
                                | crate::renderer::composer::InlineControlType::Shape
                        )
                });
                let flow_floor = if line_has_as_char_object {
                    max_fs.max(line_height)
                } else {
                    max_fs
                };
                crate::renderer::stored_line_flow_height(
                    seg,
                    next,
                    line_height,
                    line_spacing_px,
                    flow_floor,
                    self.dpi,
                    source_metrics_reflowed,
                )
            });
            let flow_step = stored_line_advance.unwrap_or(line_height);
            let line_flow_height =
                flow_step + equation_tac_extra_rows as f64 * (line_height + line_spacing_px);
            let render_line_flow_height =
                if cell_ctx.is_none() && para_index >= self.endnote_para_base.get() {
                    // 미주 lineSeg의 행 진행값이 실제 TextLine bbox보다 작으면 단일 줄 미주가
                    // 서로 겹친다. Pagination은 별도 압축 흐름을 쓰더라도 렌더 y 진행은
                    // 실제 그려진 줄 높이를 최소값으로 보존한다.
                    line_flow_height.max(max_fs).max(line_node.bbox.height)
                } else {
                    line_flow_height
                };
            let render_line_spacing_px =
                if cell_ctx.is_none() && para_index >= self.endnote_para_base.get() {
                    // 비가시 구분선/0mm 미주는 pagination과 render가 같은 압축 spacing을
                    // 써야 단 하단 클리핑이 생기지 않는다. 다만 과한 음수값은 글자 겹침을
                    // 만들 수 있으므로 실제 glyph 높이의 10% 범위로 제한한다.
                    if line_spacing_px < 0.0 {
                        line_spacing_px.max(-render_line_flow_height * 0.10)
                    } else {
                        line_spacing_px
                    }
                } else {
                    line_spacing_px
                };
            if equation_tac_extra_rows > 0 {
                line_node.bbox.height = line_flow_height;
                if let RenderNodeType::TextLine(ref mut text_line) = line_node.node_type {
                    text_line.line_height = line_flow_height;
                }
            }

            // 텍스트 정렬을 위한 전체 줄 폭 계산 (자연 폭, 추가 간격 미포함)
            // treat_as_char 이미지 폭도 포함하여 정확한 폭 산출
            // [Task #604 Stage 2] wrap_anchor 가 있는 줄: line_cs_offset 을 est_x 기준점에
            // 포함 (line_x_offset 은 col_area.x 기준 상대좌표).
            let est_x_start = effective_margin_left + line_cs_offset + inline_offset;
            let line_width_est = self.estimate_line_run_widths(
                comp_line,
                composed,
                para,
                styles,
                &tab_stops,
                tab_width,
                auto_tab_right,
                &line_tac_offsets_for_width,
                effective_margin_left,
                available_width,
                start_line,
                line_idx,
                est_x_start,
            );
            let est_x = line_width_est.est_x;
            let included_tac_width_in_est = line_width_est.included_tac_width;
            // 교차 run 탭으로 인한 역방향 이동이 있을 수 있으므로
            // est_x 차이로 정확한 점유 폭을 계산
            let mut total_text_width = (est_x - est_x_start).max(0.0);
            // TAC 이미지/Shape 폭이 est_x에 미포함된 경우 별도 추가
            // (이미지가 텍스트 끝 위치에 있으면 run 범위 필터에서 제외됨)
            //
            // [Task #1219] 줄-경계 정규 집합 line_tac_offsets 로 통일.
            // 기존 `pos <= line_end` 는 줄 끝 위치(다음 줄 선두) 수식을 포함하는
            // 동일 결함을 가졌다. line_tac_offsets 는 이미 줄-범위 집합이므로 폭만 합산.
            let total_tac_width_in_line: f64 =
                line_tac_offsets_for_width.iter().map(|(_, w, _)| w).sum();
            let missing_tac_width = (total_tac_width_in_line - included_tac_width_in_est).max(0.0);
            if missing_tac_width > 0.0 && total_text_width < total_tac_width_in_line {
                total_text_width += missing_tac_width;
            }
            // 빈 후속 lane은 같은 물리 행의 배제 영역을 표현할 뿐, 다음 글줄이 아니다.
            // 마지막 가시 segment를 일반 문단 마지막 줄처럼 정렬한다.
            let is_last_line_of_para = end == composed.lines.len()
                && (line_idx == end - 1
                    || (physical_frame_rows
                        && para.is_some_and(|p| {
                            p.line_segs[line_idx + 1..end]
                                .iter()
                                .all(|s| s.tag & LineSeg::TAG_EMPTY_SEGMENT != 0)
                        })));

            // 정렬별 간격 분배 계산
            let has_forced_break = comp_line.has_line_break;
            // [#6864] 저장 줄 정보나 머리말/꼬리말 위치는 정렬 의미를 바꾸지
            // 않는다. 마지막 줄 분배가 필요한 원본은 파서가 Split으로 전달한다.
            let needs_justify =
                needs_word_distribution(alignment, is_last_line_of_para, has_forced_break);
            let needs_distribute = alignment == Alignment::Distribute;

            let has_tabs = comp_line.runs.iter().any(|r| r.text.contains('\t'));
            let renders_synthetic_wrap_trailing_space = !is_last_line_of_para
                && para
                    .and_then(|p| p.line_segs.get(line_idx))
                    .is_some_and(|seg| seg.tag & LineSeg::TAG_IMPLEMENTATION_PROPERTY != 0)
                && comp_line
                    .runs
                    .iter()
                    .rev()
                    .find_map(|run| run.text.chars().next_back())
                    == Some(' ');
            // 자간은 **그려지는 글자**에 나눠 붙으므로 폭(`total_text_width`)과 같은
            // 텍스트로 센다. 머리말 필드처럼 모델 1자가 표시 N자면 모델로 세었을 때
            // 글자당 몫이 N배로 부풀어 글자가 흩어진다 (Task #3216).
            let total_char_count: usize = comp_line
                .runs
                .iter()
                .map(|r| {
                    effective_text_for_metrics(r)
                        .chars()
                        .filter(|c| *c != '\t')
                        .count()
                })
                .sum();
            // [Issue #6196] 저장 사다리가 이 셀 문단을 **한 줄**로, 그것도 **셀 안쪽 폭
            // 그대로** 적어 두었으면 "한글이 이 문장을 이 폭에 담았다"는 증언이다.
            // 우리 폰트 메트릭의 자연 폭이 그보다 넓다고 압축을 억제하면 문장 꼬리가
            // 칸 밖으로 나가 잘린다(156543798 4쪽 `우수 내용` 칸 9행 중 7행 소실 —
            // 자연 폭 290~331px vs 저장 줄폭 229.2px).
            let stored_single_line_fits_cell = cell_ctx.is_some()
                && composed.lines.len() == 1
                && para.is_some_and(|p| {
                    p.line_segs.len() == 1
                        && p.line_segs[0].tag
                            & crate::model::paragraph::LineSeg::TAG_IMPLEMENTATION_PROPERTY
                            == 0
                        && (hwpunit_to_px(p.line_segs[0].segment_width, self.dpi) - available_width)
                            .abs()
                            <= 2.0
                });
            // [#6389] 저장 사다리 증언의 다줄 일반화. 조합이 저장 줄 수를 그대로
            // 따랐고 모든 저장 줄폭이 셀 열폭 이내면, 한글이 이 내용을 이 폭에
            // 담았다는 증언이다 — 편람 p68 셀은 kopub/no-ttf 오라클 PDF 모두
            // 저장 줄과 문자 단위로 일치하는데, 내장 메트릭 진행폭이 실측(0.83em)
            // 보다 넓어(1.0em) 압축을 억제하면 `○` 문단 줄들이 셀 우측 테두리를
            // +72~85px 넘는다. 열폭보다 넓게 기록된 사다리(병합·재저장 안 된 낡은
            // 캐시)는 증언이 성립하지 않으므로 종전대로 억제(클리핑)한다.
            let stored_ladder_fits_frame = cell_ctx.is_some()
                && para.is_some_and(|p| {
                    !p.line_segs.is_empty()
                        && composed.lines.len() == p.line_segs.len()
                        && p.line_segs.iter().all(|seg| {
                            seg.tag & crate::model::paragraph::LineSeg::TAG_IMPLEMENTATION_PROPERTY
                                == 0
                                && hwpunit_to_px(seg.segment_width, self.dpi)
                                    <= effective_col_w + 2.0
                        })
                });
            let suppress_cell_overflow_spacing = cell_ctx.is_some()
                && total_text_width > available_width * 1.15
                && !stored_single_line_fits_cell
                && !stored_ladder_fits_frame;
            // [#6303] 자동 축소는 저장 한 줄이 **안쪽 폭을 놓친** 칸에만 수렴한다.
            // 일반 셀·문단의 선형 slack/N 을 바꾸면 page-local hash 와 text-overlap 이
            // 흔들린다. 1.15 는 #6196 억제 임계와 같다.
            let converge_auto_shrink_cell = (stored_single_line_fits_cell
                && total_text_width > available_width * 1.15)
                || (squeeze_stored_line && total_text_width > available_width);
            let is_hancom_company_pua_logo_line =
                is_hancom_company_pua_logo_line(comp_line, alignment);

            let trailing_space_limit = trailing_space_limit_after_last_inline_object(
                comp_line,
                line_tac_offsets_for_width
                    .iter()
                    .map(|(pos, _, _)| *pos)
                    .max(),
            );
            let (extra_word_sp, extra_char_sp, extra_dash_sp) = if is_hancom_company_pua_logo_line {
                // 이 줄의 trailing space는 뒤의 treat-as-char logo 그림 앞 공백이다.
                // 회사명 자체에는 자간을 추가하지 않고 이 공백 하나가 남는 폭을 전부
                // 흡수하게 해야 Hancom PDF의 좌측 회사명·우측 logo 배치가 유지된다.
                ((available_width - total_text_width).max(0.0), 0.0, 0.0)
            } else if runs_all_whitespace
                && !is_last_line_of_para
                && !has_forced_break
                && line_tac_offsets_for_width.is_empty()
                && !needs_justify
                && !needs_distribute
            {
                // Soft wrapping can consume a row of separator spaces beyond
                // the frame. Keep their decoration advances instead of fitting
                // them like glyphs. Paragraph-end/forced-break spaces remain
                // authored content and retain the normal line-fit contract.
                (0.0, 0.0, 0.0)
            } else {
                compute_line_extra_spacing(
                    comp_line,
                    trailing_space_limit,
                    styles,
                    alignment,
                    cell_ctx.is_some(),
                    needs_justify,
                    alignment == Alignment::Justify && is_last_line_of_para && !needs_justify,
                    false,
                    needs_distribute,
                    has_tabs,
                    renders_synthetic_wrap_trailing_space,
                    suppress_cell_overflow_spacing,
                    converge_auto_shrink_cell,
                    total_char_count,
                    total_text_width,
                    available_width,
                    tab_width,
                )
            };

            let line_plain_text: String = comp_line.runs.iter().map(|r| r.text.as_str()).collect();
            let is_answer_sheet_number_label =
                cell_ctx.is_some() && line_plain_text.trim() == "수험번호";
            // [Task #1308 CI follow-up / #1256 regression]
            // 본문/미주 흐름의 TAC 수식-only 줄은 저장된 LINE_SEG x 흐름을 따라야 한다.
            // 빈 TextRun 이 있는 수식-only 문단은 일반 정렬 경로로 들어오므로,
            // Distribute/Center 의 잔여 폭 중앙 오프셋을 적용하면 한컴과 달리 수식 블록이
            // 열 안쪽으로 밀린다. 그림/표 TAC는 문단 정렬 폭을 따라야 하며, 표 셀 안 수식은
            // 기존처럼 셀 정렬을 따른다.
            let non_cell_tac_only_line = cell_ctx.is_none()
                && !line_tac_offsets_for_width.is_empty()
                && line_plain_text.trim().is_empty()
                && line_tac_offsets_for_width.iter().any(|(_, _, ci)| {
                    is_treat_as_char_equation_control(para.and_then(|p| p.controls.get(*ci)))
                });

            // 셀 overflow/underflow 분기로 자간 보정된 경우 정렬 기준 폭은 실제 렌더 폭이어야 함.
            // 특히 #1285 답안지 `수험번호` 라벨은 음수 자간으로 압축된 텍스트를 자연 폭 기준으로
            // 정렬하면 압축 후 남은 폭만큼 왼쪽에 붙는다. 일반 셀은 기존 단순 보정 경로를 유지한다.
            let effective_text_width = if is_answer_sheet_number_label
                && extra_char_sp.abs() > 0.001
                && cell_ctx.is_some()
                && !needs_justify
                && !needs_distribute
                && total_char_count > 1
                && !has_tabs
            {
                comp_line
                    .runs
                    .iter()
                    .map(|r| {
                        let mut ts = r.text_style(styles);
                        ts.default_tab_width = tab_width;
                        ts.tab_stops = tab_stops.clone();
                        ts.auto_tab_right = auto_tab_right;
                        ts.available_width = available_width;
                        ts.text_start_offset = effective_margin_left;
                        ts.inline_tabs = composed.tab_extended.clone();
                        ts.extra_char_spacing = extra_char_sp;
                        if r.char_overlap.is_some() {
                            let fs = if ts.font_size > 0.0 {
                                ts.font_size
                            } else {
                                12.0
                            };
                            let chars: Vec<char> = r.text.chars().collect();
                            fs * crate::renderer::composer::char_overlap_advance_units(&chars)
                                as f64
                        } else {
                            estimate_text_width(effective_text_for_metrics(r), &ts)
                        }
                    })
                    .sum()
            } else if extra_char_sp > 0.0
                && cell_ctx.is_some()
                && !needs_justify
                && !needs_distribute
                && total_char_count > 1
            {
                total_text_width + extra_char_sp * total_char_count as f64
            } else {
                total_text_width
            };

            // [Task #1285] 답안지 머리말의 `수험번호` 라벨은
            // 파일상 ParaShape가 Center로 들어오더라도 한컴 출력에서는 셀 오른쪽에
            // 붙어 보인다. 기존 중앙 정렬 셀을 흔들지 않도록 해당 라벨에만 적용한다.
            let center_packed_cell_label_as_right = is_answer_sheet_number_label
                && alignment == Alignment::Center
                && !has_tabs
                && line_node.bbox.width <= 110.0
                && effective_text_width >= line_node.bbox.width * 0.75;

            // 비첫줄에서 번호 마커 오프셋 (첫 줄은 마커 렌더링이 x를 전진시킴)
            let num_x_offset = if num_offset > 0.0 && !(line_idx == start_line && start_line == 0) {
                num_offset
            } else {
                0.0
            };
            // [Task #604 R3] wrap_anchor 가 있으면 col_area.x + line_cs_offset 기준,
            // 아니면 effective_col_x (Task #489) 기준.
            let x_base = if wrap_anchor.is_some() {
                col_area.x + effective_margin_left + line_cs_offset
            } else {
                effective_col_x + effective_margin_left
            };
            // 한글은 셀 밖 오른쪽/가운데 정렬 폭에서 말미 공백을 제외한다
            // (needs_justify 의 후행 공백 제외와 동일 규칙). 포함하면
            // [그림+말미공백72] 꼬리말이 공백 폭(447px)만큼 왼쪽으로 이탈 —
            // 식약처 보도자료 OPEN 로고 실측(한글 x=607.3). Center 는 30213
            // 의결서 위원 서명 줄 실측(말미 공백 8칸 포함 줄만 한글 대비 43px
            // 좌측 이탈, 한글 PDF x=229.56pt 는 공백 제외 중심). 반례 셋으로
            // 한정한다: ① 셀 내부는 한글이 말미 공백을 포함해 정렬(issue_1285
            // 수험번호 TAC 우단 = 셀 inner 우단 오라클 앵커) — cell_ctx 부재.
            // ② soft-wrap 지점의 줄끝 공백은 포함 — 문단 마지막 줄 한정.
            // ③ TAC 컨트롤이 있는 줄은 공백이 시각적 말미가 아니다 —
            // line_tac_offsets_for_width 비어 있을 때 한정. ④ 전부 공백인
            // 줄(밑줄 친 서명란)과 밑줄 스타일 말미 공백은 보이는 콘텐츠라
            // 유지(issue_157 직선 골든 — 제외하면 우측 클립까지 이탈).
            // [#7081] `cell_ctx.is_none()` 을 뺀다 — 칸 예외의 근거였던 `issue_1285` 는
            // **TAC 개체가 우단을 잡는 줄**이고, 그 형상은 바로 아래
            // `line_tac_offsets_for_width.is_empty()` 가 이미 거른다. 순수 텍스트 줄에서는
            // 한/글도 칸 안에서 말미 공백을 빼고 정렬한다(3079571 취소신청서 1쪽,
            // '신청하는' + 공백 5칸 — 한/글 대비 -15.02px = 5 × 6.008 ÷ 2).
            let center_excludes_trailing_ws = alignment == Alignment::Center
                && is_last_line_of_para
                && line_tac_offsets_for_width.is_empty()
                && comp_line
                    .runs
                    .iter()
                    .any(|r| r.text.chars().any(|c| c != ' '));
            // [#5820] 글상자(drawText) 안 문단은 표 셀이 아니다 — 한글은 글상자
            // 안에서도 오른쪽 정렬의 말미 공백을 제외한다(156560092 글상자:
            // [로고A][로고B][공백5] RIGHT 문단 — 한글 로고 우변 여백 4.1px,
            // 포함 시 공백 폭 32.7px 만큼 좌측 이탈).
            //
            // [#7081] 칸 예외를 **형상**으로 좁힌다. `issue_1285` 의 근거는 "셀 내부"가
            // 아니라 **TAC 개체가 우단을 잡는 줄**이다(수험번호 TAC 우단 = 셀 inner 우단).
            // 순수 텍스트 줄에서는 한/글도 칸 안에서 말미 공백을 빼고 정렬한다.
            //
            //   3030681 이의신청서 1쪽  '신청인(대표자)' + 공백 15칸, 칸 안 RIGHT
            //     한/글 가시 텍스트 끝 404.8 · rhwp 307.1 + 공백 97.5 = 404.6
            //     -> 글자가 통째로 97.56px(= 6.504 × 15) 좌측 이탈. 같은 쪽 219자의
            //        세로 편차는 0.09px 로, 어긋난 것은 이 한 줄의 가로뿐이다.
            //   3079571 취소신청서 1쪽  '신청하는' + 공백 5칸, 칸 안 CENTER
            //     -15.02px = 5 × 6.008 ÷ 2 — 가운데 정렬이라 절반만 밀린다.
            //
            // Center 쪽은 이미 같은 가드(`line_tac_offsets_for_width.is_empty()`)를 갖고
            // 있고, Right 에만 없었다. 두 정렬의 조건을 같은 형상으로 맞춘다.
            let right_align_excludes_trailing_ws = alignment == Alignment::Right
                && (cell_ctx.as_ref().is_none_or(|c| c.in_textbox)
                    || line_tac_offsets_for_width.is_empty());
            let trailing_ws_width =
                if right_align_excludes_trailing_ws || center_excludes_trailing_ws {
                    trailing_space_width_after_last_inline_object(
                        comp_line,
                        line_tac_offsets_for_width
                            .iter()
                            .map(|(pos, _, _)| *pos)
                            .max(),
                        styles,
                        // ④ 밑줄 친 말미 공백은 보이는 콘텐츠 — Center 는 제외 대상에서
                        // 뺀다(Right 는 기존 검증 동작 유지).
                        center_excludes_trailing_ws,
                    )
                } else {
                    0.0
                };
            let terminal_tracking_width = if cell_ctx.is_some()
                && matches!(alignment, Alignment::Center | Alignment::Right)
                && !center_packed_cell_label_as_right
                && !has_tabs
                && extra_char_sp == 0.0
            {
                terminal_tracking_after_inline_picture(
                    comp_line,
                    para,
                    styles,
                    &line_tac_offsets_for_width,
                )
            } else {
                0.0
            };
            let x_start = match alignment {
                Alignment::Center => {
                    let align_offset = if center_packed_cell_label_as_right {
                        (available_width - effective_text_width).max(0.0)
                    } else if non_cell_tac_only_line {
                        0.0
                    } else {
                        (available_width
                            - (effective_text_width - trailing_ws_width - terminal_tracking_width))
                            .max(0.0)
                            / 2.0
                    };
                    x_base + inline_offset + num_x_offset + align_offset
                }
                Alignment::Distribute if !needs_distribute || total_char_count <= 1 => {
                    let align_offset = if non_cell_tac_only_line {
                        0.0
                    } else {
                        (available_width - effective_text_width).max(0.0) / 2.0
                    };
                    x_base + inline_offset + num_x_offset + align_offset
                }
                Alignment::Right => {
                    x_base
                        + inline_offset
                        + num_x_offset
                        + (available_width
                            - (effective_text_width - trailing_ws_width - terminal_tracking_width))
                            .max(0.0)
                }
                _ => x_base + inline_offset + num_x_offset, // Left, Justify, Split, Distribute(분배중)
            };

            // TextRun 노드 생성
            // 선행 공백은 x좌표 오프셋으로 처리하여 SVG 뷰어의 폰트 메트릭과 무관하게 정렬
            let mut x = x_start;

            // 개요 번호/글머리표: 첫 줄에서 별도 TextRunNode로 렌더링 (char_start: None)
            if line_idx == start_line && start_line == 0 {
                if let Some(ref num_text) = composed.numbering_text {
                    let num_style =
                        numbering_marker_text_style(styles, para, comp_line.runs.first());
                    let num_width = estimate_text_width(num_text, &num_style);
                    let num_id = tree.next_id();
                    let num_node = RenderNode::new(
                        num_id,
                        RenderNodeType::TextRun(TextRunNode {
                            text: num_text.clone(),
                            style: num_style,
                            char_shape_id: None,
                            para_shape_id: Some(composed.para_style_id),
                            section_index: Some(section_index),
                            para_index: Some(para_index),
                            char_start: None, // 문서 좌표에 포함되지 않음
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
                        BoundingBox::new(x, y, num_width, line_height),
                    );
                    line_node.children.push(num_node);
                    x += num_width;
                }
            }

            // char_offset→x 매핑 (필드 마커 위치 계산용)
            let mut char_x_map: Vec<(usize, f64)> = Vec::new();
            char_x_map.push((comp_line.char_start, x));

            // 조판부호 모드: 인라인 도형 마커 위치 수집
            let show_ctrl = self.show_control_codes.get();
            let shape_markers: Vec<(usize, String)> = collect_shape_marker_labels(show_ctrl, para);

            // 각주 마커 위치 수집
            let fn_positions: &[(usize, u16, usize)] = &composed.footnote_positions;
            let mut fn_marker_inserted = vec![false; fn_positions.len()];

            let mut pending_right_tab_render: Option<(f64, u8, u8)> = None;
            let mut pending_right_leader_digit_render = false;
            let mut run_char_pos = comp_line.char_start;
            // 이미 삽입한 도형 마커 추적
            let mut shape_marker_inserted = vec![false; shape_markers.len()];
            // cross-run 탭 감지용 inline_tabs(composed.tab_extended) 커서 — Task #290
            let mut inline_tab_cursor_render: usize = 0;
            let emit_state = self.emit_line_runs(
                tree,
                &mut line_node,
                col_node,
                comp_line,
                composed,
                para,
                bin_data_content,
                styles,
                &cell_ctx,
                &tab_stops,
                tac_offsets_px,
                &line_tac_offsets_for_width,
                &shape_markers,
                fn_positions,
                &mut fn_marker_inserted,
                &mut shape_marker_inserted,
                &mut char_x_map,
                para_topbottom_line_vpos_base,
                col_area,
                &mut kerning_layout_session,
                RunEmitVars {
                    stored_tac_assignment: stored_tac_assignment.is_some(),
                    trailing_space_limit,
                    baseline,
                    raw_lh,
                    alignment,
                    auto_tab_right,
                    available_width,
                    effective_margin_left,
                    end,
                    extra_char_sp,
                    extra_dash_sp,
                    extra_word_sp,
                    has_tabs,
                    horizontal_shaping_initial_lane,
                    is_last_line_of_para,
                    line_height,
                    line_idx,
                    line_spacing_px,
                    max_fs,
                    runs_all_whitespace,
                    renders_synthetic_wrap_trailing_space,
                    start_line,
                    tab_width,
                    section_index,
                    para_index,
                },
                RunEmitState {
                    x,
                    y,
                    char_offset,
                    run_char_pos,
                    inline_tab_cursor_render,
                    pending_right_tab_render,
                    pending_right_leader_digit_render,
                    current_line_reserved_tac_picture_height,
                },
            );
            x = emit_state.x;
            y = emit_state.y;
            char_offset = emit_state.char_offset;
            run_char_pos = emit_state.run_char_pos;
            inline_tab_cursor_render = emit_state.inline_tab_cursor_render;
            pending_right_tab_render = emit_state.pending_right_tab_render;
            pending_right_leader_digit_render = emit_state.pending_right_leader_digit_render;
            current_line_reserved_tac_picture_height =
                emit_state.current_line_reserved_tac_picture_height;

            // 조판부호: 텍스트 뒤에 위치한 미삽입 도형 마커 추가
            for (smi, (spos, stext)) in shape_markers.iter().enumerate() {
                if !shape_marker_inserted[smi] {
                    shape_marker_inserted[smi] = true;
                    let base_style = resolved_to_text_style(styles, 0, 0);
                    let mut ms = base_style;
                    ms.color = 0x0000FF;
                    ms.font_size *= 0.55;
                    let mw = estimate_text_width(stext, &ms);
                    let mid = tree.next_id();
                    let mn = RenderNode::new(
                        mid,
                        RenderNodeType::TextRun(TextRunNode {
                            text: stext.clone(),
                            style: ms,
                            char_shape_id: None,
                            para_shape_id: Some(composed.para_style_id),
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
                            field_marker: FieldMarkerType::ShapeMarker(*spos),
                            layout_positions: None,
                            display_text: None,
                        }),
                        BoundingBox::new(x, y, mw, line_height),
                    );
                    line_node.children.push(mn);
                    x += mw;
                }
            }

            x = self.place_unmatched_line_tac_pictures(
                tree,
                &mut line_node,
                comp_line,
                para,
                bin_data_content,
                tac_offsets_px,
                col_area,
                cell_ctx.as_ref(),
                &mut current_line_reserved_tac_picture_height,
                TacPictureLineVars {
                    run_char_pos,
                    x,
                    y,
                    baseline,
                    raw_lh,
                    section_index,
                    para_index,
                },
            );

            x = self.place_empty_line_tac_forms(
                tree,
                &mut line_node,
                comp_line,
                para,
                tac_offsets_px,
                cell_ctx.as_ref(),
                x,
                y,
                baseline,
                section_index,
                para_index,
            );

            let defer_empty_line_control_marker = comp_line.runs.is_empty()
                && !tac_offsets_px.is_empty()
                && equation_tac_line_flow.is_some();

            // runs가 비어있으면 빈 TextRun 생성 (빈 셀 편집용)
            if comp_line.runs.is_empty() {
                self.layout_empty_runs_line(
                    tree,
                    &mut line_node,
                    comp_line,
                    composed,
                    para,
                    bin_data_content,
                    styles,
                    &cell_ctx,
                    &line_tac_offsets,
                    col_area,
                    EmptyRunsLineVars {
                        physical_frame_rows,
                        alignment,
                        available_width,
                        effective_col_x,
                        effective_margin_left,
                        x_start,
                        line_char_end: char_offset,
                        y,
                        baseline,
                        raw_lh,
                        runs_all_whitespace,
                        max_fs,
                        line_spacing_px,
                        has_topbottom_vpos_base: para_topbottom_line_vpos_base.is_some(),
                        is_last_line_of_para,
                        defer_empty_line_control_marker,
                        line_flow_height,
                        section_index,
                        para_index,
                        line_idx,
                    },
                    &mut current_line_reserved_tac_picture_height,
                );
            }

            // [Task #287] 빈 runs 줄의 TAC 수식 인라인 처리 — #2067 추출
            self.place_empty_line_inline_equations(
                tree,
                &mut line_node,
                comp_line,
                composed,
                para,
                styles,
                &cell_ctx,
                tac_offsets_px,
                &line_tac_offsets,
                &equation_tac_line_flow,
                EquationTacLineVars {
                    line_idx,
                    line_end: end,
                    alignment,
                    available_width,
                    margin_left,
                    indent,
                    effective_col_x,
                    y,
                    baseline,
                    line_height,
                    line_spacing_px,
                    col_area_y: col_area.y,
                    col_bottom,
                    line_char_end: char_offset,
                    is_last_line_of_para,
                    defer_empty_line_control_marker,
                    equation_tac_extra_rows,
                    hwp3_indent_scale: if self.profile.get().hwp3_layout() {
                        0.5
                    } else {
                        1.0
                    },
                    section_index,
                    para_index,
                },
            );

            // ClickHere 필드 처리: 안내문 + 조판부호 마커 — #1925 추출
            if let Some(p) = para {
                x += self.layout_click_here_and_bookmark_markers(
                    tree,
                    &mut line_node,
                    p,
                    comp_line,
                    &char_x_map,
                    styles,
                    composed.para_style_id,
                    section_index,
                    para_index,
                    &cell_ctx,
                    char_offset,
                    composed
                        .lines
                        .get(line_idx + 1)
                        .is_some_and(|next| next.char_start == char_offset),
                    x,
                    y,
                    line_height,
                    baseline,
                );
            }

            // 강제 줄바꿈(\n)이 이 줄에서 제거되었으므로 char_offset에 1을 더하여
            // 다음 줄의 TextRun.char_start가 올바른 문서 좌표를 가리키도록 한다.
            if comp_line.has_line_break {
                char_offset += 1;
            }

            let following_text_xs: Vec<f64> = line_node
                .children
                .iter()
                .filter_map(|child| {
                    if let RenderNodeType::TextRun(tr) = &child.node_type {
                        if !tr.text.trim().is_empty() {
                            return Some(child.bbox.x);
                        }
                    }
                    None
                })
                .collect();
            for child in &mut line_node.children {
                if let RenderNodeType::TextRun(tr) = &mut child.node_type {
                    if tr.style.tab_leaders.is_empty() {
                        continue;
                    }
                    let space_gap = if tr.style.font_size > 0.0 {
                        tr.style.font_size * 0.25
                    } else {
                        3.0
                    };
                    for leader in &mut tr.style.tab_leaders {
                        let abs_start = child.bbox.x + leader.start_x;
                        if let Some(next_x) = following_text_xs
                            .iter()
                            .copied()
                            .filter(|x| *x > abs_start + 0.5)
                            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                        {
                            let new_end_x = (next_x - child.bbox.x - space_gap).max(leader.start_x);
                            if new_end_x < leader.end_x {
                                leader.end_x = new_end_x;
                            }
                        }
                    }
                }
            }

            col_node.children.push(line_node);
            // 줄간격 적용:
            //   - 셀 내 마지막 문단의 마지막 줄: trailing line_spacing 제외
            //     (셀 높이 모델은 trailing 미포함, 셀 내부와 정합)
            //   - 그 외 모든 줄(본문 단락의 마지막 줄 포함): trailing line_spacing 가산
            //     pagination/engine.rs 의 current_height 누적(para_height = sum(lh+ls))
            //     과 정합. (Task #452: 이전 #332 의 layout-only trailing 제외 →
            //     pagination 과 1 ls drift 발생 → 회복)
            let is_cell_last_line = is_last_cell_para && line_idx + 1 >= end;
            // [Task #901 Stage 5/6] wrap zone paragraph 의 empty-runs / whitespace-only
            // line 은 y advance 건너뜀.
            // pic2.hwp paragraph 0 case: 8 line_segs (4 visible "우/리/나/라" + 4 empty
            // phantom lines for wrap zone 의 다른 column). 추가로 첫 idx=0 은 cs=24470
            // (LEFT narrow wrap zone) 의 공백 한 글자만 가짐 — 한컴 viewer 가 wrap zone
            // 좌측 영역에 텍스트 미배치한 결과. has_picture_shape_square_wrap 게이트로
            // wrap zone 호스트 paragraph 만 영향.
            if !runs_all_whitespace
                && !text_before_picture_line
                && current_line_reserved_tac_picture_height.is_none()
            {
                current_line_reserved_tac_picture_height = para.and_then(|p| {
                    crate::renderer::line_owning_tac_object_height_px(p, raw_lh, self.dpi)
                });
                if current_line_reserved_tac_picture_height.is_none()
                    && has_treat_as_char_picture_or_shape(para)
                    && max_fs > 0.0
                    && raw_lh > max_fs * 2.0
                {
                    current_line_reserved_tac_picture_height = Some(raw_lh);
                }
            }
            let tac_picture_label_extra = tac_picture_label_extra_for_line(
                cell_ctx.as_ref(),
                runs_all_whitespace,
                raw_lh,
                current_line_reserved_tac_picture_height,
                max_fs,
                line_spacing_px,
            );
            // Square wrap host 의 빈 guide 줄은 advance 를 건너뛰지만, 같은 줄에
            // TAC 수식/개체가 있으면 실제 콘텐츠 줄이므로 높이를 보존한다.
            // The preceding visible fragment deferred its advance to this row's
            // last fragment. An empty right-hand fragment must still pay it.
            let completes_visible_stored_row = cell_ctx.is_some()
                && para.is_some_and(|p| {
                    crate::renderer::height_measurer::stored_seg_is_row_fragment(p, line_idx)
                        && (0..line_idx)
                            .rev()
                            .take_while(|&idx| {
                                p.line_segs
                                    .get(idx)
                                    .zip(p.line_segs.get(line_idx))
                                    .is_some_and(|(a, b)| a.vertical_pos == b.vertical_pos)
                            })
                            .any(|idx| {
                                composed.lines.get(idx).is_some_and(|line| {
                                    line.runs.iter().any(|run| !run.text.trim().is_empty())
                                })
                            })
                });
            let skip_advance_empty_wrap = has_picture_shape_square_wrap
                && !has_ole_shape_square_wrap
                && runs_all_whitespace
                && !completes_visible_stored_row
                && !line_has_tac_control(composed, line_idx);
            // 촘촘한 미주 수식 문단에는 다음 줄과 char_start가 같은 선행
            // 퇴화 LINE_SEG가 들어오는 경우가 있다. 해당 줄 자체에는 TAC가
            // 없으며, 한컴은 첫 수식 앞에 이 안내 줄 높이를 예약하지 않는다.
            let skip_advance_empty_tac_lead = cell_ctx.is_none()
                && !tac_offsets_px.is_empty()
                && line_is_leading_empty_equation_tac_guide(
                    para,
                    composed,
                    tac_offsets_px,
                    line_idx,
                );
            // [#6545] 저장 사다리가 이 줄에 **자기 vertical_pos** 를 줬다면 앞 줄이 예약한
            // TAC 개체 높이의 유령 사본이 아니다 — 파일이 두 줄로 적어 둔 실제 줄이다.
            // 높이만 같다고 건너뛰면 수식 줄이 흐름에 0 을 내고, 뒤 문단 전체가 그
            // 한 줄만큼 위로 올라붙는다 (3-09월_교육_통합_2022 23쪽: −13.5pt).
            let stored_line_has_own_vpos = endnote_line_vpos_base.is_some()
                && para
                    .and_then(|p| {
                        let prev = p.line_segs.get(line_idx.checked_sub(1)?)?;
                        let cur = p.line_segs.get(line_idx)?;
                        Some(cur.vertical_pos != prev.vertical_pos)
                    })
                    .unwrap_or(false);
            let skip_advance_empty_tac_picture = runs_all_whitespace
                && current_line_reserved_tac_picture_height.is_none()
                && !stored_line_has_own_vpos
                && prev_line_reserved_tac_picture_height
                    .map(|pic_h| (raw_lh - pic_h).abs() <= 4.0)
                    .unwrap_or(false);
            let skip_advance_empty_line = skip_advance_empty_wrap
                || skip_advance_empty_tac_picture
                || skip_advance_empty_tac_lead;
            // RHWP_DEBUG_PARA_TAC="95,96,97" — 대상 pi 를 콤마 목록으로 지정 (빈 값/all=전체).
            if std::env::var("RHWP_DEBUG_PARA_TAC").is_ok_and(|v| {
                v.is_empty()
                    || v == "all"
                    || v.split(',').any(|t| t.trim().parse() == Ok(para_index))
            }) {
                eprintln!(
                    "  TAC_ADV pi={} line_idx={} y={:.1} raw_lh={:.1} lh={:.1} ls={:.1} label_extra={:.1} whitespace={} cur_pic={:?} prev_pic={:?} skip_wrap={} skip_pic={} skip={}",
                    para_index,
                    line_idx,
                    y,
                    raw_lh,
                    line_height,
                    line_spacing_px,
                    tac_picture_label_extra,
                    runs_all_whitespace,
                    current_line_reserved_tac_picture_height,
                    prev_line_reserved_tac_picture_height,
                    skip_advance_empty_wrap,
                    skip_advance_empty_tac_picture,
                    skip_advance_empty_line,
                );
            }
            // [Task #1046 Stage 3 Class D] 본문 문단(셀 밖)의 콘텐츠 하단(=현재 줄 텍스트
            // 바닥, trailing 줄간격/spacing_after 제외) 기록. overflow 검출이 페이지 바닥
            // 후행 줄간격을 콘텐츠 초과로 오판하지 않도록 한다(페이지네이터의 마지막 줄
            // trailing_ls 허용 #359/#404 와 정합). 매 줄 갱신 → 마지막 렌더 줄 값이 남는다.
            if cell_ctx.is_none() && !skip_advance_empty_line {
                let content_bottom = if blank_spacer_line {
                    y
                } else {
                    y + render_line_flow_height
                };
                self.last_item_content_bottom.set(content_bottom);
                if equation_only_endnote_tail_line && content_bottom > col_bottom {
                    self.last_item_endnote_equation_tail_line_box.set(true);
                }
            }
            if endnote_line_vpos_base.is_some() {
                let line_bottom = if skip_advance_empty_line {
                    y
                } else {
                    // [Task #1236] 다줄 미주 문단의 마지막 줄: 다음 문단이 **같은 미주**
                    // 연속이면 trailing 줄간격을 포함해 풀이 줄간격을 균일하게 한다
                    // (간헐적 좁아짐 해소). 미주 마지막 문단(=문제 경계)이면 0 유지해
                    // between-notes margin 과 중복 가산되지 않게 한다.
                    let trailing = if line_idx + 1 < end
                        || self.endnote_para_has_same_endnote_successor(para_index)
                    {
                        render_line_spacing_px
                    } else {
                        0.0
                    };
                    y + render_line_flow_height + trailing + tac_picture_label_extra
                };
                let next_y = endnote_line_vpos_y_end
                    .map(|prev| prev.max(line_bottom))
                    .unwrap_or(line_bottom);
                endnote_line_vpos_y_end = Some(next_y);
                if equation_tac_extra_rows > 0 || endnote_used_auto_wrap_y {
                    endnote_auto_wrap_y_end = Some(line_bottom);
                }
                y = next_y;
            } else if is_cell_last_line && cell_ctx.is_some() {
                // 셀 정렬의 점유 높이에서 마지막 줄간격을 빼더라도 문단 테두리는
                // 원래 줄 상자와 후행 간격을 둘러싼다. paint 영역을 흐름 전진에
                // 다시 가산하지 않는다 (한컴 저장 1056+632HU 문단 테두리).
                last_line_border_bottom =
                    Some(y + line_flow_height + render_line_spacing_px.max(0.0));
                last_line_box_bottom = Some(y + line_flow_height);
                y += line_flow_height;
            } else if skip_advance_empty_line {
                // no advance
            } else if para.is_some_and(|p| {
                crate::renderer::height_measurer::stored_seg_is_row_fragment(p, line_idx + 1)
            }) {
                // [#6299] 다음 줄이 이 줄의 **가로 조각**(같은 vertical_pos, 다른
                // column_start)이면 같은 물리 줄이다 — 어울림 개체 좌·우로 쪼개진 짝을
                // 세로로 쌓으면 문단이 조각 수만큼 길어져 칸 밖으로 흘러내린다
                // (156518878 1쪽 머리글 칸: 101.3 / 122.6 / 143.9 / 165.3 으로 4단
                // 적층, 마지막 줄이 칸 바닥 164.5 를 넘었다). 측정 쪽 회계와 같은
                // 계약이라 두 경로가 갈리지 않는다.
                last_line_box_bottom = Some(y + render_line_flow_height);
            } else {
                last_line_box_bottom = Some(y + render_line_flow_height);
                y += render_line_flow_height + render_line_spacing_px + tac_picture_label_extra;
            }
            prev_line_reserved_tac_picture_height = current_line_reserved_tac_picture_height;
        }

        // 문단 테두리/배경 범위 수집 (build_single_column에서 연속 그룹으로 병합 렌더링)
        // margin_left/margin_right를 반영하여 박스 위치·폭 조정.
        // Task #463: 셀 안 단락은 본문 큐에 leakage 하지 않도록 cell_ctx 게이팅.
        // 셀 외곽선은 별도 경로(table_layout/border_rendering)에서 처리되므로
        // 본문 단락의 연속 외곽선 merge 가 셀 단락 좌표/시그니처에 의해 깨지지 않게 한다.
        if para_border_fill_id > 0 && (cell_ctx.is_none() || self.collect_cell_para_borders.get()) {
            // [#5711] 줄간격이 음수인 문단은 전진값 `y` 가 마지막 줄 상자 아래보다 위에
            // 있다. 그 값을 테두리 아래 변으로 쓰면 테두리가 글자를 가로지른다. 다음 문단
            // 시작 y 는 종전대로 두어 문단 간 간격 계약은 바꾸지 않는다.
            let border_bottom = last_line_border_bottom
                .or(last_line_box_bottom)
                .map_or(y, |bottom| y.max(bottom));
            let bg_height = border_bottom - bg_y_start;
            if bg_height > 0.0 {
                // margin_left/margin_right는 이미 px 단위 (style_resolver에서 변환됨)
                // border_spacing[2]/[3] (top/bottom) 을 inset 으로 전달 — 병합 그룹의 첫/마지막 range 에서만 적용됨.
                let top_inset = para_style.map(|s| s.border_spacing[2]).unwrap_or(0.0);
                let bottom_inset = para_style.map(|s| s.border_spacing[3]).unwrap_or(0.0);
                // 컬럼/페이지 wrap 시 inner edge 미렌더링용 partial 플래그
                let is_partial_start = start_line > 0;
                let is_partial_end = end < composed.lines.len();
                // Task #463: wrap=Square 호스트 문단의 텍스트는 좁은 wrap_area 에서
                // 렌더링되지만 외곽선은 원래 col_area 너비로 그려야 floating 표를
                // 박스가 둘러쌈. layout_wrap_around_paras 가 override 를 설정.
                // override 가 활성된 경우(wrap host), 박스 우측은 floating 표의 끝
                // 까지 확장된 width 그대로 사용 — margin_right 차감하지 않는다
                // (그렇지 않으면 표가 박스 밖으로 다시 튀어나옴).
                // [Task #544] paragraph margin_left/right 는 텍스트 inset 으로만 사용,
                // 박스 outline 좌표는 col_area 전체 (PDF 정합). wrap=Square 호스트
                // (border_box_override) 케이스는 layout_wrap_around_paras 가 설정한
                // override 좌표 그대로 사용 (margin 미적용).
                let (box_x, box_w) = if let Some((ox, ow)) = self.border_box_override.get() {
                    (ox, ow)
                } else {
                    (col_area.x, col_area.width)
                };
                self.para_border_ranges.borrow_mut().push((
                    para_border_fill_id,
                    box_x,
                    bg_y_start,
                    box_w,
                    border_bottom,
                    top_inset,
                    bottom_inset,
                    is_partial_start,
                    is_partial_end,
                    para_index,
                ));
            }
        }

        // ComposedLine이 없으면 빈 TextRun 생성 (편집용). `compose_paragraph()`는
        // 빈 문단에 줄을 만들지 않을 수 있는데, 종전 400HU 고정 advance는
        // pagination의 NO_LS 빈 문단 메트릭과 달라 다음 표/문단을 위로 끌어올렸다.
        // 이 경로도 원래 글자모양·줄간격을 사용해 두 경로를 일치시킨다 (#3820 p81–82).
        if composed.lines.is_empty() && start_line == 0 {
            let (default_height, default_spacing) = para
                .and_then(|p| {
                    empty_no_lineseg_paragraph_metrics(
                        p,
                        styles,
                        para_style,
                        self.profile.get().hwp3_layout(),
                        self.dpi,
                    )
                })
                .map(|(line_height, line_spacing, font_size)| {
                    // #7032: cell measurement omits trailing spacing on its
                    // last visible empty paragraph and uses the glyph em box.
                    // Preserve body/HWP3 fallback contracts.
                    if cell_ctx.is_some() && is_last_cell_para && !self.profile.get().hwp3_layout()
                    {
                        (font_size, 0.0)
                    } else {
                        (line_height, line_spacing)
                    }
                })
                .or_else(|| {
                    // [#7062] 저장 LINE_SEG 없이 TAC(글자처럼) 개체만 앵커한 문단은
                    // 빈 문단이 아니다. 400HU(5.3px) 고정 advance 는 개체 높이를 통째로
                    // 버려 뒤 내용이 개체 한가운데 겹쳐 그려진다(156060125 2쪽: 그림
                    // 548.1..948.1 안쪽 553.4 에서 뒤 표가 시작).
                    // typeset(`format_paragraph_for_flow`)·측정(`height_measurer`)이
                    // 이미 쓰는 #2287 합성을 **같은 헬퍼·같은 가용 폭**으로 불러
                    // 세 경로의 문단 전진값을 일치시킨다. 이 폴백은 줄 노드를 하나만
                    // 만드는 자리라 합성 줄들의 높이 합을 그 한 줄에 싣는다 —
                    // 전진 총량은 두 측정 경로와 같고, 개체 자체는 별도 노드로 그려진다.
                    //
                    // [#7079] 합성 줄의 leading 은 줄 **뒤**에 남는다 — 개체 잉크는
                    // 움직이지 않는다. 한컴 engine 2020 출력을 같은 96dpi 래스터로 겹쳐
                    // 재면 정본은 앞 본문줄→도해 잉크 84px · 도해→`※` 상자 360px 인데,
                    // leading 이 없으면 뒤 거리가 350px 로 짧고 leading 을 개체 위로
                    // 올리면 앞 거리가 96px 로 벌어진다. 뒤에 두면 86/360/366 으로
                    // 세 거리가 모두 맞는다.
                    let avail = {
                        let margin_l = para_style.map(|s| s.margin_left).unwrap_or(0.0);
                        let margin_r = para_style.map(|s| s.margin_right).unwrap_or(0.0);
                        (col_area.width - margin_l - margin_r).max(0.0)
                    };
                    para.and_then(|p| {
                        crate::renderer::tac_object_stack_line_metrics(
                            p,
                            self.dpi,
                            Some(avail),
                            styles,
                            para_style,
                        )
                    })
                    .map(|metrics| {
                        let height: f64 = metrics.iter().map(|(h, _)| *h).sum();
                        let leading: f64 = metrics.iter().map(|(_, s)| *s).sum();
                        (height, leading)
                    })
                })
                .unwrap_or((hwpunit_to_px(400, self.dpi), 0.0));
            let line_id = tree.next_id();
            let mut line_node = RenderNode::new(
                line_id,
                RenderNodeType::TextLine(TextLineNode::with_para(
                    default_height,
                    default_height * 0.8,
                    section_index,
                    para_index,
                )),
                BoundingBox::new(col_area.x, y, col_area.width, default_height),
            );

            // 빈 문단에도 TextRun 노드를 생성하여 캐럿 위치 제공
            let run_id = tree.next_id();
            let (text_style, char_shape_id) =
                paragraph_active_text_style(styles, para, char_offset);
            let run_node = RenderNode::new(
                run_id,
                RenderNodeType::TextRun(TextRunNode {
                    text: String::new(),
                    style: text_style,
                    char_shape_id,
                    para_shape_id: Some(composed.para_style_id),
                    section_index: Some(section_index),
                    para_index: Some(para_index),
                    char_start: Some(char_offset),
                    cell_context: cell_ctx.clone(),
                    is_para_end: true,
                    is_line_break_end: false,
                    rotation: 0.0,
                    is_vertical: false,
                    char_overlap: None,
                    border_fill_id: 0,
                    baseline: default_height * 0.85,
                    field_marker: FieldMarkerType::None,
                    layout_positions: None,
                    display_text: None,
                }),
                BoundingBox::new(col_area.x, y, col_area.width, default_height),
            );
            line_node.children.push(run_node);

            col_node.children.push(line_node);
            y += default_height + default_spacing;
        }

        // 문단 뒤 간격 (spacing_after). 빈 composed 문단도 실제 한 줄 advance 뒤에
        // 적용해야 일반 composed 문단과 동일한 순서를 따른다.
        if spacing_after > 0.0 && end == composed.lines.len() {
            y += spacing_after;
        }

        y
    }
}
