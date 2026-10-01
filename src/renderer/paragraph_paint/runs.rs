use crate::model::bin_data::BinDataContent;
use crate::model::control::Control;
use crate::model::paragraph::{LineSeg, Paragraph};
use crate::model::style::Alignment;
use crate::model::table::Table;
use crate::renderer::border_paint::create_border_line_nodes;
use crate::renderer::cell_context::CellContext;
use crate::renderer::composer::{effective_text_for_metrics, ComposedLine, ComposedParagraph};
use crate::renderer::kerning::{ExactFontSlot, KerningLayoutSession};
use crate::renderer::page_layout::LayoutRect;
use crate::renderer::paint_resources::find_bin_data_bytes;
use crate::renderer::render_tree::*;
use crate::renderer::style_resolver::ResolvedStyleSet;
use crate::renderer::text_measurement::{
    compute_char_positions, estimate_text_width, estimate_text_width_exact,
    estimate_text_width_unrounded, extract_tab_leaders_with_extended,
};
use crate::renderer::{hwpunit_to_px, ShapeStyle, TabStop, TextStyle};

use super::helpers::*;
use super::ParagraphPaintSession;

impl ParagraphPaintSession<'_> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_line_runs(
        &self,
        tree: &mut PageLayoutContext,
        line_node: &mut RenderNode,
        col_node: &mut RenderNode,
        comp_line: &crate::renderer::composer::ComposedLine,
        composed: &ComposedParagraph,
        para: Option<&Paragraph>,
        bin_data_content: Option<&[BinDataContent]>,
        styles: &ResolvedStyleSet,
        cell_ctx: &Option<CellContext>,
        tab_stops: &[TabStop],
        tac_offsets_px: &[(usize, f64, usize)],
        line_tac_offsets: &[(usize, f64, usize)],
        shape_markers: &[(usize, String)],
        fn_positions: &[(usize, u16, usize)],
        fn_marker_inserted: &mut [bool],
        shape_marker_inserted: &mut [bool],
        char_x_map: &mut Vec<(usize, f64)>,
        para_topbottom_line_vpos_base: Option<(i32, f64)>,
        col_area: &LayoutRect,
        kerning_layout_session: &mut KerningLayoutSession<'_>,
        vars: RunEmitVars,
        st: RunEmitState,
    ) -> RunEmitState {
        let RunEmitVars {
            stored_tac_assignment,
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
        } = vars;
        let RunEmitState {
            mut x,
            y,
            mut char_offset,
            mut run_char_pos,
            mut inline_tab_cursor_render,
            mut pending_right_tab_render,
            mut pending_right_leader_digit_render,
            mut current_line_reserved_tac_picture_height,
        } = st;
        // [#7150] 정렬·방출이 사용하는 줄별 TAC 집합에서 표 앵커를 한 번 선택한다.
        // 저장 UTF-16 줄 경계에서는 이전 줄 끝 표와 다음 줄 첫 개체가 같은 가시
        // 문자 위치에 투영된다. composed.tac_controls를 문자 구간으로 다시 나누면
        // 다른 줄의 표가 소유자로 섞인다. caller가 복원한 저장 줄 배정과 마지막
        // run 끝 TAC를 포함한 공통 집합을 그대로 소비한다.
        let is_last_run_of_line = |idx: usize| idx == comp_line.runs.len() - 1;
        // [#5679] 줄-말미 공백에 배정된 배분 여분(extra_word_sp) 회수분.
        // 배분 몫은 **내부 공백 수**로 나눈다(위 needs_justify 분기의
        // rendered_space_slots) — 줄-말미 공백은 est 의 effective_used 에서도
        // 빠져 있다. 그런데 char_width_decision 은 모든 ' ' 에 여분을 붙이므로,
        // 말미 공백이 여분까지 얹어 그려져 run bbox 와 x 전진이 줄 상자를
        // 여분×말미공백수 만큼 넘는다(10857 p11: '외부 평가전문위원 ' 144.0 vs
        // 줄 122.1 — 가시 글리프는 정확히 줄 끝에서 끝나고 초과 전량이 부풀린
        // 말미 공백). 분모에 말미 공백을 넣는 synthetic-wrap 모드는 제외.
        let line_trailing_space_by_run: Vec<usize> = {
            let mut counts = vec![0usize; comp_line.runs.len()];
            if !renders_synthetic_wrap_trailing_space && extra_word_sp != 0.0 {
                let mut budget = comp_line
                    .runs
                    .iter()
                    .flat_map(|r| effective_text_for_metrics(r).chars())
                    .collect::<Vec<char>>()
                    .iter()
                    .rev()
                    .take_while(|c| **c == ' ')
                    .count()
                    .min(trailing_space_limit);
                for (ri, r) in comp_line.runs.iter().enumerate().rev() {
                    if budget == 0 {
                        break;
                    }
                    let rt = effective_text_for_metrics(r);
                    let chars_total = rt.chars().count();
                    let tail_sp = rt.chars().rev().take_while(|c| *c == ' ').count();
                    let take = tail_sp.min(budget);
                    counts[ri] = take;
                    budget -= take;
                    if take < chars_total {
                        break;
                    }
                }
            }
            counts
        };
        for (run_idx, run) in comp_line.runs.iter().enumerate() {
            // 조판부호: 이 run 시작 위치 이전의 도형 마커를 먼저 삽입
            for (smi, (spos, stext)) in shape_markers.iter().enumerate() {
                if !shape_marker_inserted[smi] && *spos <= run_char_pos {
                    shape_marker_inserted[smi] = true;
                    let base_style = run.text_style(styles);
                    let mut ms = base_style;
                    ms.color = 0x0000FF; // BGR: 빨간색
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
            let mut text_style = run.text_style(styles);
            text_style.default_tab_width = tab_width;
            text_style.tab_stops = tab_stops.to_vec();
            text_style.auto_tab_right = auto_tab_right;
            text_style.available_width = available_width;
            text_style.text_start_offset = effective_margin_left;
            text_style.inline_tabs = composed.tab_extended.clone();
            if pending_right_leader_digit_render {
                if run.text.trim().is_empty() {
                    pending_right_leader_digit_render = true;
                } else {
                    if run.text.trim().chars().all(|ch| ch.is_ascii_digit()) {
                        if let Some(tab) = tab_stops
                            .iter()
                            .rev()
                            .find(|tab| tab.tab_type == 1 && tab.fill_type != 0)
                        {
                            let digit_w = estimate_text_width(run.text.trim(), &text_style);
                            let target =
                                if composed.tab_extended.is_empty() && available_width > 0.0 {
                                    effective_margin_left + available_width
                                } else {
                                    tab.position
                                };
                            let gap = if composed.tab_extended.is_empty() {
                                0.0
                            } else {
                                text_style.font_size * 0.25
                            };
                            x = col_area.x + target - gap - digit_w;
                        }
                    }
                    pending_right_leader_digit_render = false;
                }
            }
            // 교차 run 오른쪽/가운데 탭: 이전 run이 \t로 끝났고
            // 해당 탭이 오른쪽/가운데 탭이면 이 run을 역방향으로 이동
            if let Some((tab_pos, tab_type, fill_type)) = pending_right_tab_render.take() {
                // [Task #279] 공백만 있는 run 은 right/center tab 정렬 단위가 아니다.
                // 한컴 목차의 장제목 케이스: "Ⅰ. 사업개요\t" + " " + "3" 으로 run 분리되며,
                // " " run 에 right tab 을 적용하면 페이지번호 "3" 이 effective_pos 보다
                // 공백 폭만큼 우측으로 밀려 소제목 정렬과 어긋난다. 공백 only run 은 정렬을
                // 건너뛰고 pending 을 다음 의미있는 run 으로 carry-over.
                if (tab_type == 1 || tab_type == 2) && run.text.trim().is_empty() {
                    // carry-over: 공백 run 은 정렬 단위가 아님. leader 보정도 다음 run 시점으로
                    // 위임 (그 시점의 leader-bearing TextRun 검색이 \t 가진 진짜 leader run 을 찾음).
                    pending_right_tab_render = Some((tab_pos, tab_type, fill_type));
                } else {
                    text_style.line_x_offset = x - col_area.x;
                    // [Task #279] 리더(fill_type ≠ 0) 가 있는 RIGHT 탭은 "이 줄 우측 끝까지" 의미.
                    // 한컴은 TabDef.position 을 절대 좌표로 신뢰하지 않고 리더 도트의 시멘틱
                    // (= 단/셀 콘텐츠 영역 우측 끝까지 채움) 으로 재해석한다.
                    // 셀 안 문단에서는 col_area 가 이미 cell padding 적용된 inner_area 이므로
                    // `effective_margin_left + available_width` 가 inner 우측 끝.
                    // tab_pos (HWP 저장값) 이 inner 우측 끝을 초과하면 셀 padding_right 침범이므로 강제 클램핑.
                    // [Task #874] auto_tab_right (fill_type=0) 도 effective_margin_left 변환 필요.
                    let effective_pos = if tab_type == 1 {
                        effective_margin_left
                            + (if fill_type != 0 {
                                available_width
                            } else {
                                tab_pos
                            })
                    } else {
                        tab_pos
                    };
                    // [Issue #842 #4] 탭 다음 콘텐츠가 여러 composed run 으로 쪼개진 경우
                    // (스크립트·char-shape 경계, 예 "Ctrl+(회색)5") 전체 블록 폭 기준 정렬.
                    let next_w = right_tab_block_width(
                        &comp_line.runs,
                        run_idx,
                        styles,
                        tab_width,
                        &tab_stops,
                        auto_tab_right,
                        available_width,
                    );
                    match tab_type {
                        1 => {
                            x = col_area.x + effective_pos - next_w;
                        }
                        2 => {
                            x = col_area.x + effective_pos - next_w / 2.0;
                        }
                        _ => {}
                    }
                    // [#6800] **같은 재배치가 그 탭 런의 advance 도 정한다.**
                    //
                    // 오른쪽/가운데 탭의 전진량은 "탭 스톱까지"가 아니라 "뒤따르는
                    // 블록이 스톱에 맞도록 필요한 만큼"이다. 그런데 런 폭은
                    // `estimate_text_width` 가 낸 **탭 스톱까지** 값이 그대로 남아,
                    // 그 런이 뒤 런을 통째로 덮은 것처럼 보였다
                    // (1192000-202100017 1쪽: `"  - 	"` 런이 x=221.8 w=496.0 인데
                    //  다음 런은 x=249.1 — 468.7px 가짜 겹침으로 계수된다).
                    // 잉크는 안 겹치므로 출력은 불변이지만, `text_overlap_baseline`
                    // 래칫이 그 가짜 겹침을 세어 **진짜 글자겹침을 가린다.**
                    //
                    // 아래 leader 보정과 **같은 `x`** 를 쓴다 — 재배치가 정한 값
                    // 하나로 bbox·리더 기하가 함께 정해진다. 여기서만 하므로
                    // ① 이 재배치를 유발한 **논리적 끝 탭**의 런에만 닿고
                    // ② 탭 뒤에 가시문자가 있는 런은 애초에 `pending` 을 세우지
                    //    않으므로 대상이 아니며
                    // ③ 재배치가 없는 줄의 런은 전혀 건드리지 않는다.
                    if let Some(tab_run_idx) = line_node.children.iter().rposition(|n| {
                        matches!(&n.node_type, RenderNodeType::TextRun(tr) if tr.display_or_text().ends_with('\t'))
                    }) {
                        let tab_run_x = line_node.children[tab_run_idx].bbox.x;
                        // 탭 런과 현재 런 사이에 이미 emit 된 런(공백 only carry-over)이
                        // 있으면 그 앞까지가 이 탭의 전진량이다.
                        let end_x = line_node.children[tab_run_idx + 1..]
                            .iter()
                            .filter(|n| matches!(n.node_type, RenderNodeType::TextRun(_)))
                            .map(|n| n.bbox.x)
                            .fold(x, f64::min);
                        let width = match &mut line_node.children[tab_run_idx].node_type {
                            RenderNodeType::TextRun(run) => {
                                run.resolve_trailing_tab_end(
                                    (end_x - tab_run_x).max(0.0),
                                    (x - tab_run_x).max(0.0),
                                )
                            }
                            _ => None,
                        };
                        if let Some(width) = width {
                            let old_width = line_node.children[tab_run_idx].bbox.width;
                            line_node.children[tab_run_idx].bbox.width = width;
                            // 이미 생성한 같은 런의 장식도 동일한 확정 끝을 사용한다.
                            // 장식은 TextRun보다 먼저 생성되므로 tab_run_idx 앞도 순회한다.
                            // 원 PR #6801의 c7dade57c 보정 취지를 유지한다.
                            // 다른 TextRun이나 크기가 다른 개체의 상자는 수정하지 않는다.
                            for node in &mut line_node.children {
                                if !matches!(node.node_type, RenderNodeType::TextRun(_))
                                    && (node.bbox.x - tab_run_x).abs() <= 0.5
                                    && (node.bbox.width - old_width).abs() <= 0.5
                                {
                                    node.bbox.width = (tab_run_x + width - node.bbox.x).max(0.0);
                                }
                            }
                        }
                    }
                    // [Task #279] 직전 run 의 leader 끝 위치를 페이지번호 시작 x 직전까지 단축.
                    // 한컴은 페이지번호 폭에 따라 리더 길이가 달라지도록 조판한다 (한 자리 vs
                    // 두 자리 페이지번호의 leader 끝점이 다름). cross-run RIGHT 정렬 후
                    // tab_leaders 가 있는 직전 TextRun 을 거슬러 찾아 마지막 항목 end_x 를 보정.
                    // 공백 only run carry-over 케이스 대비 — 가장 마지막 TextRun 이 공백 run 이고
                    // leader 가 없으면 그 이전 (\t 가진 leader-bearing) TextRun 을 찾음.
                    if let Some(prev_run_node) = line_node.children.iter_mut().rev().find(|n| {
                        if let RenderNodeType::TextRun(tr) = &n.node_type {
                            !tr.style.tab_leaders.is_empty()
                        } else {
                            false
                        }
                    }) {
                        let prev_bbox_x = prev_run_node.bbox.x;
                        if let RenderNodeType::TextRun(prev_text_run) = &mut prev_run_node.node_type
                        {
                            let space_gap = if text_style.font_size > 0.0 {
                                text_style.font_size * 0.25
                            } else {
                                3.0
                            };
                            for leader in &mut prev_text_run.style.tab_leaders {
                                let new_end_x = (x - prev_bbox_x - space_gap).max(leader.start_x);
                                if new_end_x < leader.end_x {
                                    leader.end_x = new_end_x;
                                }
                            }
                        }
                    }
                } // end else (non-blank run)
            }
            text_style.line_x_offset = x - col_area.x;
            text_style.extra_word_spacing = extra_word_sp;
            text_style.extra_char_spacing = extra_char_sp;
            text_style.extra_dash_advance = extra_dash_sp;
            // [Task #874 #2] composer lang split (예: "F3→Alt+I" → "F3"/"→"/"Alt+I")
            // 으로 auto_tab_right post-tab 콘텐츠가 후속 run 으로 흩어진 경우, 현재
            // run 내부 seg_w 만으로는 우측 정렬 위치가 어긋남. 후속 run 합산을 미리
            // 계산해 text_style.right_tab_block_width_override 로 주입한다.
            if auto_tab_right && run.text.contains('\t') && run_idx + 1 < comp_line.runs.len() {
                let tab_byte = run.text.rfind('\t').unwrap();
                let post_tab: String = run.text[tab_byte + '\t'.len_utf8()..].to_string();
                let no_more_tabs_after_in_run = !post_tab.contains('\t');
                let no_tabs_in_subsequent = comp_line
                    .runs
                    .iter()
                    .skip(run_idx + 1)
                    .all(|r| !r.text.contains('\t'));
                if no_more_tabs_after_in_run && no_tabs_in_subsequent {
                    let mut ts_measure = text_style.clone();
                    ts_measure.right_tab_block_width_override = None;
                    let post_tab_w = estimate_text_width(&post_tab, &ts_measure);
                    let subsequent_w = right_tab_block_width(
                        &comp_line.runs,
                        run_idx + 1,
                        styles,
                        tab_width,
                        &tab_stops,
                        auto_tab_right,
                        available_width,
                    );
                    text_style.right_tab_block_width_override = Some(post_tab_w + subsequent_w);
                }
            }
            let run_border_fill_id = styles
                .char_styles
                .get(run.char_style_id as usize)
                .map(|cs| cs.border_fill_id)
                .unwrap_or(0);
            let full_width = if run.char_overlap.is_some() {
                // 글자겹침: 한 컨트롤은 payload 글자 수와 무관하게 1글자 폭.
                let fs = if text_style.font_size > 0.0 {
                    text_style.font_size
                } else {
                    12.0
                };
                let chars: Vec<char> = run.text.chars().collect();
                fs * crate::renderer::composer::char_overlap_advance_units(&chars) as f64
            } else if run.display_text.is_some()
                && run.text.chars().count() == 1
                && matches!(
                    run.text.chars().next(),
                    Some('\u{0015}' | '\u{0016}' | '\u{0017}' | '\u{2007}')
                )
            {
                // 필드 marker 한 글자와 표시 문자열의 폭이 소수 px일 수 있다. 이 런은
                // 다음 조각과 분리되어 있으므로 정수 반올림을 하면 뒤의 fwSpace/텍스트
                // 앵커가 SVG 실제 glyph advance보다 앞선다 (#3216, #1100). field 런만
                // 비반올림 폭을 써서 모델 한 글자 경계와 표시 끝을 같은 좌표에 둔다.
                estimate_text_width_unrounded(effective_text_for_metrics(run), &text_style)
            } else {
                // [#7254] 배치 폭은 반올림하지 않는다 — 바로 위 field run 예외가 적어 둔
                // 사유(`#3216`·`#1100`: 정수 반올림이 뒤 앵커를 실제 glyph advance 보다
                // 앞세운다)가 일반 run 에도 그대로 적용된다. 줄 나눔은 이미 반올림하지
                // 않은 폭으로 줄을 짜므로, 여기서 반올림하면 같은 줄을 측정과 배치가 다른
                // 폭으로 소비한다.
                estimate_text_width_exact(effective_text_for_metrics(run), &text_style)
            };
            // [#5679] 줄-말미 공백의 배분 여분 회수 — 자연 폭은 유지한다(한글도
            // 말미 공백 자체는 줄 상자를 넘길 수 있다). 여분이 음수(압축)여도
            // est 가 말미 공백을 제외했으므로 동일하게 회수한다.
            let full_width =
                full_width - extra_word_sp * line_trailing_space_by_run[run_idx] as f64;
            // 각주/TAC/탭은 최종 TextRun 경계를 추가로 만든다. whole-run pair를
            // 먼저 적용하면 그 경계를 가로지르는 delta가 남으므로, sub-run
            // producer가 연결될 때까지 이 특수 run은 원자적으로 K0로 닫는다.
            let run_char_count_for_boundary = if run.char_overlap.is_some() {
                let chars: Vec<char> = run.text.chars().collect();
                crate::renderer::composer::char_overlap_advance_units(&chars)
            } else {
                run.text.chars().count()
            };
            let run_char_end_for_boundary = run_char_pos + run_char_count_for_boundary;
            let has_tac_boundary = tac_offsets_px.iter().any(|(position, _, _)| {
                *position >= run_char_pos && *position <= run_char_end_for_boundary
            });
            let has_note_boundary = fn_positions.iter().any(|(position, _, _)| {
                *position >= run_char_pos && *position <= run_char_end_for_boundary
            });
            let exact_replay_eligible = run.char_overlap.is_none()
                && !run
                    .text
                    .chars()
                    .any(|character| matches!(character, '\t' | '\n' | '\r'))
                && !has_tac_boundary
                && !has_note_boundary;
            let shaping_candidate = horizontal_shaping_initial_lane
                && run_idx == 0
                && run_char_pos == 0
                && char_offset == 0
                && !has_tabs
                && tac_offsets_px.is_empty()
                && fn_positions.is_empty()
                && shape_markers.is_empty()
                && run_border_fill_id == 0
                && extra_word_sp.abs() <= f64::EPSILON
                && extra_char_sp.abs() <= f64::EPSILON
                && extra_dash_sp.abs() <= f64::EPSILON
                && !renders_synthetic_wrap_trailing_space;
            // NodeId를 먼저 고정하되 attach가 실패하면 같은 id로 legacy TextRun을
            // 만든다. 따라서 실패는 id hole이나 K1 suppression을 남기지 않는다.
            let reserved_shaping_run_id = shaping_candidate.then(|| tree.next_id());
            let shaping_width = reserved_shaping_run_id.and_then(|node_id| {
                para.and_then(|para| {
                    attach_horizontal_shaping_initial_lane(
                        tree,
                        composed,
                        para,
                        styles,
                        run,
                        node_id,
                        run_char_pos,
                        x,
                        full_width,
                        cell_ctx.is_some() && (start_line != 0 || end != composed.lines.len()),
                    )
                })
            });
            let (full_width, layout_positions) = if let Some(shaping_width) = shaping_width {
                (shaping_width, None)
            } else {
                emitted_run_layout_positions(
                    kerning_layout_session,
                    ExactFontSlot::new(run.char_style_id, run.lang_index),
                    effective_text_for_metrics(run),
                    &text_style,
                    full_width,
                    line_trailing_space_by_run[run_idx],
                    exact_replay_eligible,
                )
            };
            // 탭 리더 계산: 탭이 포함된 run에서 채움 기호 정보 추출
            // inline_tabs를 일시 제거하여 tab_stops 기반 위치 계산과 일관되게 함
            if has_tabs && run.text.contains('\t') {
                let saved_inline_tabs = std::mem::take(&mut text_style.inline_tabs);
                let positions = compute_char_positions(&run.text, &text_style);
                text_style.inline_tabs = saved_inline_tabs;
                text_style.tab_leaders = extract_tab_leaders_with_extended(
                    &run.text,
                    &positions,
                    &text_style,
                    &composed.tab_extended,
                );
            }
            // [#6844] 런 안의 오른쪽/가운데 탭 — 정렬 블록이 이 런 안에서 끝나는 형상만.
            let (full_width, layout_positions) =
                if has_tabs && run.text.contains('\t') && run_idx + 1 == comp_line.runs.len() {
                    resolve_intra_run_right_tab(
                        &run.text,
                        full_width,
                        layout_positions,
                        &text_style,
                        &composed.tab_extended,
                        inline_tab_cursor_render,
                        &tab_stops,
                        tab_width,
                        auto_tab_right,
                        available_width,
                    )
                } else {
                    (full_width, layout_positions)
                };
            // 교차 run 오른쪽/가운데 탭 감지 — Task #290:
            // inline_tabs(composed.tab_extended) 가 LEFT 를 명시하면 cross-run pending 을 설정하지 않는다.
            // [Task #279] trailing 공백 (\t 뒤에 따라오는 ' ') 도 허용 — 목차 소제목의
            // 들여쓰기 문단에서 한컴이 "\t " 형태로 저장하는 케이스가 있음.
            let trimmed_end_r = run
                .text
                .trim_end_matches(|c: char| c == ' ' || c == '\u{2007}');
            if has_tabs && trimmed_end_r.ends_with('\t') {
                let run_tab_count = run.text.chars().filter(|c| *c == '\t').count();
                if run_tab_count > 0 {
                    let last_inline_idx = inline_tab_cursor_render + run_tab_count - 1;
                    pending_right_tab_render = resolve_last_tab_pending(
                        &run.text,
                        last_inline_idx,
                        &composed.tab_extended,
                        &text_style,
                        &tab_stops,
                        tab_width,
                        auto_tab_right,
                        available_width,
                    );
                }
            }
            if has_tabs
                && run.text.contains('\t')
                && run
                    .text
                    .rsplit_once('\t')
                    .map(|(_, after)| after.trim().is_empty())
                    .unwrap_or(false)
                && tab_stops
                    .iter()
                    .any(|tab| tab.tab_type == 1 && tab.fill_type != 0)
            {
                pending_right_leader_digit_render = true;
            }
            let run_char_count = if run.char_overlap.is_some() {
                // 글자겹침(CharOverlap)은 HWP char_offset 공간에서 1개 위치만 차지
                let chars: Vec<char> = run.text.chars().collect();
                crate::renderer::composer::char_overlap_advance_units(&chars)
            } else {
                run.text.chars().count()
            };
            let run_char_end = run_char_pos + run_char_count;
            let is_last_run = is_last_line_of_para && is_last_run_of_line(run_idx);
            let is_line_break = comp_line.has_line_break && is_last_run_of_line(run_idx);

            // treat_as_char 분기점: run 내 이미지 위치 목록 (rel_pos, width_px, control_index)
            // 마지막 run에서는 run_char_end 위치의 TAC도 포함 (문단 끝 수식/그림)
            // [Task #960] has_line_break line 의 마지막 run 도 run_char_end 위치 의 TAC
            // 포함. HWP3 의 char_offsets gap 분석으로 매핑된 control 위치가 `\n` 문자
            // 에 떨어지면 (예: 시험지 page 2 pi=117 의 cases formula at position 30 =
            // `\n` 위치), 그 line 의 chars range [start, end) 에서 end 가 `\n` 위치
            // 이므로 누락. has_line_break line 의 마지막 run 의 end position 도 TAC
            // 포함하면 line 의 정확한 위치에 inline emit.
            //
            // 다만 다음 LineSeg/ComposedLine 이 같은 char 위치에서 시작하면
            // 그 boundary TAC 는 다음 줄의 시작 글자처럼 취급해야 한다. 현재 줄에서도
            // end TAC 로 허용하면 미주 수식이 이전 줄 끝과 다음 줄 시작에 중복 emit 되어
            // 같은 수식이 겹친다.
            let next_line_starts_at_run_end = composed
                .lines
                .get(line_idx + 1)
                .is_some_and(|next| next.char_start == run_char_end);
            let allow_end_tac = (is_last_run
                || (comp_line.has_line_break && is_last_run_of_line(run_idx)))
                && !next_line_starts_at_run_end;
            let run_tacs: Vec<(usize, f64, usize)> = tac_offsets_px
                .iter()
                .filter(|(pos, _, _)| {
                    *pos >= run_char_pos
                        && (*pos < run_char_end
                            || ((allow_end_tac
                                || (stored_tac_assignment && is_last_run_of_line(run_idx)))
                                && *pos == run_char_end))
                        // [#5727] 저장 lineseg 가 개체에 배정한 빈 줄이 소유한 경계
                        // TAC 는 다음 줄 run 에 다시 싣지 않는다 — 실으면 개체가 이
                        // 줄로 끌려 내려오고 텍스트가 개체 폭만큼 오른쪽으로 밀린다.
                        && (stored_tac_assignment
                            || !tac_owned_by_prior_empty_line(composed, line_idx, *pos))
                })
                .map(|(pos, w, ci)| (pos - run_char_pos, *w, *ci))
                .collect();

            // [Task #960] env-gated TAC line-mapping 추적
            if std::env::var("RHWP_DEBUG_PARA_TAC").is_ok() && !tac_offsets_px.is_empty() {
                eprintln!("  TAC_LINE pi={} line_idx={} run_idx={} run_char_pos={} run_char_end={} y={:.1} lh={:.1} ls={:.1} raw_lh={:.1} baseline={:.1} run_tacs={:?}",
                    para_index, line_idx, run_idx, run_char_pos, run_char_end, y, line_height, line_spacing_px, raw_lh, baseline, run_tacs);
            }

            if run_tacs.is_empty() {
                // tac 없음: 기존 렌더링 경로
                // 선행 공백 분리
                let leading_spaces: String = run.text.chars().take_while(|c| *c == ' ').collect();
                let content = run.text.trim_start_matches(' ');

                // 글자 테두리/배경: bbox 계산용 run_x, run_w
                let (run_x, run_w) = if !leading_spaces.is_empty() && !content.is_empty() {
                    let leading_count = leading_spaces.chars().count();
                    if let Some(positions) = layout_positions.as_deref() {
                        let leading_end = positions.get(leading_count).copied();
                        let run_end = positions.last().copied();
                        if let (Some(leading_end), Some(run_end)) = (leading_end, run_end) {
                            (x + leading_end, run_end - leading_end)
                        } else {
                            let sw = estimate_text_width(&leading_spaces, &text_style);
                            (x + sw, estimate_text_width(content, &text_style))
                        }
                    } else {
                        let sw = estimate_text_width(&leading_spaces, &text_style);
                        (x + sw, estimate_text_width(content, &text_style))
                    }
                } else {
                    (x, full_width)
                };

                // 글자 배경 사각형 (텍스트 앞에 삽입)
                if run_border_fill_id > 0 {
                    let bf_idx = (run_border_fill_id as usize).saturating_sub(1);
                    if let Some(bs) = styles.border_styles.get(bf_idx) {
                        if let Some(fill_color) = bs.fill_color {
                            let rect_id = tree.next_id();
                            let rect_node = RenderNode::new(
                                rect_id,
                                RenderNodeType::Rectangle(RectangleNode::new(
                                    0.0,
                                    ShapeStyle {
                                        fill_color: Some(fill_color),
                                        stroke_color: None,
                                        stroke_width: 0.0,
                                        ..Default::default()
                                    },
                                    None,
                                )),
                                BoundingBox::new(run_x, y, run_w, line_height),
                            );
                            line_node.children.push(rect_node);
                        }
                    }
                }

                // 형광펜 배경 사각형 (RangeTag type=2)
                if let Some(p) = para {
                    if !p.range_tags.is_empty() {
                        let char_w = if run_char_count > 0 {
                            run_w / run_char_count as f64
                        } else {
                            0.0
                        };
                        for rt in &p.range_tags {
                            let rt_type = (rt.tag >> 24) & 0xFF;
                            if rt_type != 2 {
                                continue;
                            }
                            let rt_start = rt.start as usize;
                            let rt_end = rt.end as usize;
                            // run과 RangeTag가 겹치는 문자 범위
                            let overlap_start = rt_start.max(run_char_pos);
                            let overlap_end = rt_end.min(run_char_end);
                            if overlap_start >= overlap_end {
                                continue;
                            }
                            let hl_color = rt.tag & 0x00FFFFFF;
                            let relative_start = overlap_start - run_char_pos;
                            let relative_end = overlap_end - run_char_pos;
                            let exact_range = layout_positions.as_deref().and_then(|positions| {
                                if positions.len() != run.text.chars().count().saturating_add(1) {
                                    return None;
                                }
                                Some((
                                    *positions.get(relative_start)?,
                                    *positions.get(relative_end)?,
                                ))
                            });
                            let (hl_x, hl_w) = if let Some((start, end)) = exact_range {
                                (x + start, end - start)
                            } else {
                                (
                                    run_x + relative_start as f64 * char_w,
                                    (relative_end - relative_start) as f64 * char_w,
                                )
                            };
                            let rect_id = tree.next_id();
                            let rect_node = RenderNode::new(
                                rect_id,
                                RenderNodeType::Rectangle(RectangleNode::new(
                                    0.0,
                                    ShapeStyle {
                                        fill_color: Some(hl_color),
                                        stroke_color: None,
                                        stroke_width: 0.0,
                                        ..Default::default()
                                    },
                                    None,
                                )),
                                BoundingBox::new(hl_x, y, hl_w, line_height),
                            );
                            line_node.children.push(rect_node);
                        }
                    }
                }

                let mut fn_split_extra = 0.0f64; // 각주 마커 삽입으로 인한 추가 폭
                let mut emitted_text_width = full_width;
                {
                    // run 내 각주 위치 수집 (run 내 상대 위치, 각주 번호, fn_positions 인덱스, control 인덱스)
                    // 마지막 run에서는 run_char_end 위치의 각주도 포함 (문단 끝 각주)
                    let is_last = is_last_run_of_line(run_idx);
                    let run_fn_markers: Vec<(usize, u16, usize, usize)> = fn_positions
                        .iter()
                        .enumerate()
                        .filter_map(|(fni, &(fpos, fnum, ctrl_idx))| {
                            if is_leading_endnote_marker_rendered_as_prefix(
                                para,
                                ctrl_idx,
                                line_idx,
                                start_line,
                                fpos,
                                comp_line.char_start,
                            ) {
                                // 미주는 첫 줄 앞에 본문 크기 번호를 별도 TextRun으로 이미 그린다.
                                // 같은 위치의 위첨자 마커를 다시 만들면 `문26)`처럼 제목이 중복된다.
                                fn_marker_inserted[fni] = true;
                                return None;
                            }
                            let in_range = fpos >= run_char_pos
                                && (fpos < run_char_end || (is_last && fpos == run_char_end));
                            if !fn_marker_inserted[fni] && in_range {
                                Some((fpos - run_char_pos, fnum, fni, ctrl_idx))
                            } else {
                                None
                            }
                        })
                        .collect();

                    if run_fn_markers.is_empty() {
                        // 각주 없음: 기존 방식으로 전체 TextRun 생성
                        let run_x = x;
                        let run_id = reserved_shaping_run_id.unwrap_or_else(|| tree.next_id());
                        let run_node = RenderNode::new(
                            run_id,
                            RenderNodeType::TextRun(TextRunNode {
                                text: run.text.clone(),
                                display_text: run.display_text.clone(),
                                style: text_style,
                                char_shape_id: Some(run.char_style_id),
                                para_shape_id: Some(composed.para_style_id),
                                section_index: Some(section_index),
                                para_index: Some(para_index),
                                char_start: Some(char_offset),
                                cell_context: cell_ctx.clone(),
                                is_para_end: is_last_run,
                                is_line_break_end: is_line_break,
                                rotation: 0.0,
                                is_vertical: false,
                                char_overlap: run.char_overlap.clone(),
                                border_fill_id: run_border_fill_id,
                                baseline,
                                field_marker: FieldMarkerType::None,
                                layout_positions,
                            }),
                            BoundingBox::new(run_x, y, full_width, line_height),
                        );
                        line_node.children.push(run_node);
                    } else {
                        // 각주 있음: run을 각주 위치에서 분할하여 TextRun + FootnoteMarker 교차 생성
                        let run_chars: Vec<char> = run.text.chars().collect();
                        let mut seg_start = 0usize; // run 내 상대 문자 인덱스
                        let mut sub_x = x;
                        let mut sub_char_offset = char_offset;
                        emitted_text_width = 0.0;

                        for &(rel_pos, fnum, fni, ctrl_idx) in &run_fn_markers {
                            fn_marker_inserted[fni] = true;
                            // 각주 앞 텍스트 세그먼트
                            if rel_pos > seg_start {
                                let seg_text: String =
                                    run_chars[seg_start..rel_pos].iter().collect();
                                // [#7254] 조각 폭도 반올림하지 않는다 — 조각 경계마다 반올림하면 누적된다.
                                let seg_w = estimate_text_width_exact(&seg_text, &text_style);
                                let (seg_w, seg_layout_positions) = emitted_run_layout_positions(
                                    kerning_layout_session,
                                    ExactFontSlot::new(run.char_style_id, run.lang_index),
                                    &seg_text,
                                    &text_style,
                                    seg_w,
                                    0,
                                    run.char_overlap.is_none()
                                        && !seg_text.chars().any(|character| {
                                            matches!(character, '\t' | '\n' | '\r')
                                        }),
                                );
                                let seg_id = tree.next_id();
                                let seg_node = RenderNode::new(
                                    seg_id,
                                    RenderNodeType::TextRun(TextRunNode {
                                        text: seg_text,
                                        style: text_style.clone(),
                                        char_shape_id: Some(run.char_style_id),
                                        para_shape_id: Some(composed.para_style_id),
                                        section_index: Some(section_index),
                                        para_index: Some(para_index),
                                        char_start: Some(sub_char_offset),
                                        cell_context: cell_ctx.clone(),
                                        is_para_end: false,
                                        is_line_break_end: false,
                                        rotation: 0.0,
                                        is_vertical: false,
                                        char_overlap: None,
                                        border_fill_id: run_border_fill_id,
                                        baseline,
                                        field_marker: FieldMarkerType::None,
                                        layout_positions: seg_layout_positions,
                                        display_text: None,
                                    }),
                                    BoundingBox::new(sub_x, y, seg_w, line_height),
                                );
                                line_node.children.push(seg_node);
                                sub_x += seg_w;
                                emitted_text_width += seg_w;
                                sub_char_offset += rel_pos - seg_start;
                            }
                            // FootnoteMarker 노드
                            let fn_text = note_marker_text_from_control(
                                para.and_then(|p| p.controls.get(ctrl_idx)),
                                fnum,
                            );
                            let base_ts = &text_style;
                            let sup_size = (base_ts.font_size * 0.55).max(7.0);
                            let sup_ts = TextStyle {
                                font_size: sup_size,
                                font_family: base_ts.font_family.clone(),
                                color: base_ts.color,
                                ..Default::default()
                            };
                            let sup_w = estimate_text_width(&fn_text, &sup_ts);
                            let fid = tree.next_id();
                            let fn_node = RenderNode::new(
                                fid,
                                RenderNodeType::FootnoteMarker(FootnoteMarkerNode {
                                    number: fnum,
                                    text: fn_text,
                                    base_font_size: base_ts.font_size,
                                    font_family: base_ts.font_family.clone(),
                                    color: base_ts.color,
                                    section_index,
                                    para_index,
                                    control_index: ctrl_idx,
                                }),
                                BoundingBox::new(sub_x, y, sup_w, line_height),
                            );
                            line_node.children.push(fn_node);
                            sub_x += sup_w;
                            fn_split_extra += sup_w;
                            seg_start = rel_pos;
                        }
                        // 마지막 세그먼트 (각주 뒤 나머지 텍스트)
                        if seg_start < run_chars.len() {
                            let seg_text: String = run_chars[seg_start..].iter().collect();
                            // [#7254] 조각 폭도 반올림하지 않는다 — 조각 경계마다 반올림하면 누적된다.
                            let seg_w = estimate_text_width_exact(&seg_text, &text_style);
                            let trailing_space_count = line_trailing_space_by_run[run_idx]
                                .min(run_chars.len().saturating_sub(seg_start));
                            let (seg_w, seg_layout_positions) = emitted_run_layout_positions(
                                kerning_layout_session,
                                ExactFontSlot::new(run.char_style_id, run.lang_index),
                                &seg_text,
                                &text_style,
                                seg_w - extra_word_sp * trailing_space_count as f64,
                                trailing_space_count,
                                run.char_overlap.is_none()
                                    && !seg_text
                                        .chars()
                                        .any(|character| matches!(character, '\t' | '\n' | '\r')),
                            );
                            let seg_id = tree.next_id();
                            let seg_node = RenderNode::new(
                                seg_id,
                                RenderNodeType::TextRun(TextRunNode {
                                    text: seg_text,
                                    style: text_style,
                                    char_shape_id: Some(run.char_style_id),
                                    para_shape_id: Some(composed.para_style_id),
                                    section_index: Some(section_index),
                                    para_index: Some(para_index),
                                    char_start: Some(sub_char_offset),
                                    cell_context: cell_ctx.clone(),
                                    is_para_end: is_last_run,
                                    is_line_break_end: is_line_break,
                                    rotation: 0.0,
                                    is_vertical: false,
                                    char_overlap: run.char_overlap.clone(),
                                    border_fill_id: run_border_fill_id,
                                    baseline,
                                    field_marker: FieldMarkerType::None,
                                    layout_positions: seg_layout_positions,
                                    display_text: None,
                                }),
                                BoundingBox::new(sub_x, y, seg_w, line_height),
                            );
                            line_node.children.push(seg_node);
                            emitted_text_width += seg_w;
                        }
                    }
                }

                // 글자 테두리선 (텍스트 뒤에 삽입)
                if run_border_fill_id > 0 {
                    let bf_idx = (run_border_fill_id as usize).saturating_sub(1);
                    if let Some(bs) = styles.border_styles.get(bf_idx) {
                        let bx = run_x;
                        let by = y;
                        let bw = run_w;
                        let bh = line_height;
                        // borders[0]=left, [1]=right, [2]=top, [3]=bottom
                        let border_pairs: [(f64, f64, f64, f64, usize); 4] = [
                            (bx, by, bx, by + bh, 0),           // left
                            (bx + bw, by, bx + bw, by + bh, 1), // right
                            (bx, by, bx + bw, by, 2),           // top
                            (bx, by + bh, bx + bw, by + bh, 3), // bottom
                        ];
                        for (lx1, ly1, lx2, ly2, bi) in border_pairs {
                            let nodes =
                                create_border_line_nodes(tree, &bs.borders[bi], lx1, ly1, lx2, ly2);
                            for n in nodes {
                                line_node.children.push(n);
                            }
                        }
                    }
                }

                x += emitted_text_width + fn_split_extra;
            } else {
                // tac 있음: 분기점마다 하위 텍스트 런 생성 (이미지는 layout.rs에서 별도 렌더링)
                let run_chars: Vec<char> = run.text.chars().collect();
                let mut seg_start = 0usize;
                let mut sub_char_offset = char_offset;

                // [Task #455] 외부 문단 본문 텍스트는 글상자 유무와 무관하게 항상 렌더한다.
                // 글상자(TextBox) 자체와 그 내부 텍스트("개화" 같은)는
                // shape_layout 이 inline_shape_position 을 보고 별도 패스에서 렌더하므로 중복되지 않는다.

                for &(tac_rel, tac_w, tac_ci) in &run_tacs {
                    // [Issue #3396] 한글은 TAC 표를 "outMargin 포함 폭의 문자"로
                    // 배치한다 — 괘선(테두리)은 pen + outMargin.left 에 그려지고,
                    // 다음 문자는 outMargin.right 뒤에서 시작한다 (오라클 실측:
                    // 156678235 JUSTIFY 표 좌측 괘선 = 흐름 x + om_l). tac_w
                    // (composer 열폭 합)는 측정 경로 공유값이라 여기 렌더 전진에서만
                    // 보정한다. 아래 표 분기에서 채워진다.
                    let mut tac_table_om = (0.0f64, 0.0f64);
                    // tac 앞 텍스트 세그먼트 렌더링
                    if seg_start < tac_rel {
                        let seg_text: String = run_chars[seg_start..tac_rel].iter().collect();
                        let mut seg_style = text_style.clone();
                        seg_style.line_x_offset = x - col_area.x;
                        // [Issue #6179] 이 조각의 마지막 탭 뒤에 TAC 개체가 오면,
                        // 되밀기 폭에 그 개체 폭을 포함시킨다 (조각 경계로 잘려
                        // 측정 쪽에서는 보이지 않는다).
                        if auto_tab_right && seg_text.contains('\t') {
                            let tab_rel = seg_start
                                + run_chars[seg_start..tac_rel]
                                    .iter()
                                    .rposition(|c| *c == '\t')
                                    .expect("seg_text 가 탭을 포함한다");
                            seg_style.right_tab_block_width_override =
                                right_tab_block_width_with_tac(
                                    &run_chars, tab_rel, &run_tacs, &seg_style,
                                );
                        }
                        // 탭 리더 계산
                        if has_tabs && seg_text.contains('\t') {
                            let positions = compute_char_positions(&seg_text, &seg_style);
                            seg_style.tab_leaders = extract_tab_leaders_with_extended(
                                &seg_text,
                                &positions,
                                &seg_style,
                                &composed.tab_extended,
                            );
                        }
                        // [#7254] 조각 폭도 반올림하지 않는다 — 조각 경계마다 반올림하면 누적된다.
                        let seg_w = estimate_text_width_exact(&seg_text, &seg_style);
                        let (seg_w, seg_layout_positions) = emitted_run_layout_positions(
                            kerning_layout_session,
                            ExactFontSlot::new(run.char_style_id, run.lang_index),
                            &seg_text,
                            &seg_style,
                            seg_w,
                            0,
                            run.char_overlap.is_none()
                                && !seg_text
                                    .chars()
                                    .any(|character| matches!(character, '\t' | '\n' | '\r')),
                        );
                        let seg_char_count = tac_rel - seg_start;
                        {
                            let sub_run_id = tree.next_id();
                            let sub_run_node = RenderNode::new(
                                sub_run_id,
                                RenderNodeType::TextRun(TextRunNode {
                                    text: seg_text,
                                    style: seg_style,
                                    char_shape_id: Some(run.char_style_id),
                                    para_shape_id: Some(composed.para_style_id),
                                    section_index: Some(section_index),
                                    para_index: Some(para_index),
                                    char_start: Some(sub_char_offset),
                                    cell_context: cell_ctx.clone(),
                                    is_para_end: false,
                                    is_line_break_end: false,
                                    rotation: 0.0,
                                    is_vertical: false,
                                    char_overlap: run.char_overlap.clone(),
                                    border_fill_id: run_border_fill_id,
                                    baseline,
                                    field_marker: FieldMarkerType::None,
                                    layout_positions: seg_layout_positions,
                                    display_text: None,
                                }),
                                BoundingBox::new(x, y, seg_w, line_height),
                            );
                            line_node.children.push(sub_run_node);
                        }
                        x += seg_w;
                        sub_char_offset += seg_char_count;
                    }
                    // 인라인 이미지 렌더링: 텍스트 흐름 순서에 맞게 이 위치에서 직접 렌더링
                    if let (Some(p), Some(bdc)) = (para, bin_data_content) {
                        if let Some(ctrl) = p.controls.get(tac_ci) {
                            if let Control::Picture(pic) = ctrl {
                                let (pic_w, pic_h) =
                                    self.resolve_inline_picture_size(pic, col_area);
                                // [#6603] 잉크는 바깥 여백 상자의 (왼쪽, 위) 안쪽에 그린다.
                                let (margin_left, _, margin_top, margin_bottom) =
                                    tac_picture_outer_margins_px(pic, self.dpi);
                                // LINE_SEG vpos가 TopAndBottom 흐름 위치를 이미 담고 있으면
                                // sibling 예약 높이를 다시 더하지 않는다.
                                let sibling_reserved_px = if para_topbottom_line_vpos_base.is_some()
                                {
                                    0.0
                                } else {
                                    let raw = hwpunit_to_px(
                                        calc_sibling_topandbottom_reserved_hu(&p.controls),
                                        self.dpi,
                                    );
                                    // 줄 y 가 이미 형제 자리차지 예약 아래(최종 좌표)면
                                    // 이중 가산 금지 — host 문단의 꼬리 줄이 저장 vpos
                                    // 스냅으로 표 아래(쪽 하단)에 이미 놓였는데 표 높이
                                    // 를 또 더하면 tac 그림이 줄보다 예약 높이만큼 아래
                                    // (쪽 밖, #6271 실측 y=2113px > 단 하단 1115px)에
                                    // 그려져 소실된다.
                                    // 가산 결과가 단 하단을 넘는 경우도 stale
                                    // 예약(분할 이월 쪽의 통짜 가정)이므로 가산하지
                                    // 않는다.
                                    // [편집 세션] typeset 이 라인 흐름(표 아래·새 쪽
                                    // 재배정)을 이미 끝낸 상태라 저장-형상 가정의 예약
                                    // 가산이 이중이 된다 — 이월된 쪽에서 그림이 쪽
                                    // 하단 밖에 그려지던 결함(셀 끝 Enter 재현).
                                    // 흐름 y 를 그대로 신뢰한다.
                                    if self.profile.get().session_edited() {
                                        0.0
                                    } else if raw > 40.0
                                        && (y >= col_area.y + raw - 4.0
                                            || y + raw > col_area.y + col_area.height + 3.8)
                                    {
                                        0.0
                                    } else {
                                        raw
                                    }
                                };
                                if raw_lh + 4.0 >= pic_h {
                                    current_line_reserved_tac_picture_height = Some(pic_h);
                                }
                                let label_extra = tac_picture_label_extra_for_line(
                                    cell_ctx.as_ref(),
                                    runs_all_whitespace,
                                    raw_lh,
                                    current_line_reserved_tac_picture_height,
                                    max_fs,
                                    line_spacing_px,
                                );
                                let base_img_y = if label_extra > 0.0 {
                                    y + label_extra
                                } else {
                                    // [#6575] 같은 계약의 형제 경로 — 상자 전체로 맞춘다.
                                    // [#6603] 상자에는 위아래 바깥 여백도 들어간다.
                                    let box_h =
                                        tac_object_box_height_px(pic_h, &pic.caption, self.dpi)
                                            + margin_top
                                            + margin_bottom;
                                    (y + baseline - box_h).max(y)
                                };
                                let img_y = base_img_y + sibling_reserved_px + margin_top;
                                let bin_data_id = pic.image_attr.bin_data_id;
                                let image_data = find_bin_data_bytes(bdc, bin_data_id);
                                let crop = {
                                    let c = &pic.crop;
                                    if c.right > c.left
                                        && c.bottom > c.top
                                        && (c.left != 0
                                            || c.top != 0
                                            || c.right != 0
                                            || c.bottom != 0)
                                    {
                                        Some((c.left, c.top, c.right, c.bottom))
                                    } else {
                                        None
                                    }
                                };
                                let original_size_hu = pic.crop_reference_size();
                                // [Task #1151 v7 항목 7] ImageNode 생성 helper 통합.
                                let img_node = make_picture_image_node(
                                    tree,
                                    pic,
                                    section_index,
                                    para_index,
                                    tac_ci,
                                    cell_ctx.as_ref(),
                                    crop,
                                    original_size_hu,
                                    bin_data_id,
                                    image_data,
                                    BoundingBox::new(x + margin_left, img_y, pic_w, pic_h),
                                );
                                line_node.children.push(img_node);
                                // [Task #864 Stage G] inline TAC picture 의 위치 등록.
                                // layout.rs 의 TAC inline branch (line ~2906) 가
                                // already_registered 체크로 중복 emit 방지하나, 기존
                                // paragraph_layout 은 picture 에 대해 register 누락
                                // → layout.rs branch 가 또 emit 하여 동일 picture 가
                                // 두 위치 (top-aligned + baseline-aligned) 에 그려짐.
                                // HWP3 sample14 에서 caption 이 duplicate image 에 가려져
                                // 보이지 않던 결함 정정.
                                tree.set_inline_shape_position(
                                    section_index,
                                    para_index,
                                    tac_ci,
                                    cell_ctx.as_ref(),
                                    x + margin_left,
                                    img_y,
                                );
                            }
                        }
                    }
                    // 인라인 Shape(글상자) 렌더링: 텍스트 흐름 순서에 맞게 배치
                    // Shape 내부의 텍스트/테두리를 직접 렌더링하고, 별도 Shape 패스에서는 스킵
                    if let Some(p) = para {
                        if let Some(Control::Shape(shape)) = p.controls.get(tac_ci) {
                            let common = shape.common();
                            let shape_h = hwpunit_to_px(shape.flow_height_hu(), self.dpi);
                            if raw_lh + 4.0 >= shape_h {
                                current_line_reserved_tac_picture_height = Some(shape_h);
                            }
                            let label_extra = tac_picture_label_extra_for_line(
                                cell_ctx.as_ref(),
                                runs_all_whitespace,
                                raw_lh,
                                current_line_reserved_tac_picture_height,
                                max_fs,
                                line_spacing_px,
                            );
                            // [#6606] 상자(도형 + 위아래 여백)를 baseline 에 앉히고 도형은
                            // 상자의 (왼쪽, 위) 여백 안쪽에 둔다 — TAC 그림(#6603)과 같은 계약.
                            let (margin_left, _, margin_top, margin_bottom) =
                                tac_object_outer_margins_px(common, self.dpi);
                            let shape_y = if label_extra > 0.0 {
                                y + label_extra + margin_top
                            } else {
                                (y + baseline - shape_h - margin_top - margin_bottom).max(y)
                                    + margin_top
                            };
                            // 인라인 좌표 등록 → shape_layout.rs에서 이 Shape를 스킵
                            tree.set_inline_shape_position(
                                section_index,
                                para_index,
                                tac_ci,
                                cell_ctx.as_ref(),
                                x + margin_left,
                                shape_y,
                            );
                        }
                    }
                    // 인라인 수식: 직접 EquationNode로 렌더링
                    if let Some(p) = para {
                        if let Some(Control::Equation(eq)) = p.controls.get(tac_ci) {
                            // 수식 스크립트 → AST → 레이아웃 → SVG 조각
                            let tokens = crate::renderer::equation::tokenizer::tokenize(&eq.script);
                            let ast =
                                crate::renderer::equation::parser::EqParser::new(tokens).parse();
                            let font_size_px = hwpunit_to_px(eq.font_size as i32, self.dpi);
                            let layout_box =
                                crate::renderer::equation::layout::EqLayout::new(font_size_px)
                                    .layout(&ast);
                            let color_str =
                                crate::renderer::equation::svg_render::eq_color_to_svg(eq.color);
                            let svg_content =
                                crate::renderer::equation::svg_render::render_equation_svg(
                                    &layout_box,
                                    &color_str,
                                    font_size_px,
                                );
                            // HWP 저장 높이를 우선 사용 (한컴 조판 결과 기준)
                            let hwp_eq_h = hwpunit_to_px(eq.common.height as i32, self.dpi);
                            let eq_h = if hwp_eq_h > 0.0 {
                                hwp_eq_h
                            } else {
                                layout_box.height
                            };
                            // 텍스트와 섞인 인라인 수식뿐 아니라 공백 run 안의 TAC 수식도
                            // baseline을 맞춘다. 수식 renderer는 bbox 높이로 세로 스케일하지
                            // 않으므로 y에 직접 붙이면 큰 루트/분수 수식이 아래 줄을 덮는다.
                            let eq_y = if cell_ctx.is_none()
                                && comp_line.runs.iter().all(|r| {
                                    !r.text.chars().any(|c| c > '\u{001F}' && c != '\u{FFFC}')
                                }) {
                                y + baseline - layout_box.baseline
                            } else {
                                (y + baseline - layout_box.baseline).max(y)
                            };
                            let (eq_cell_idx, eq_cell_para_idx) = if let Some(ref ctx) = cell_ctx {
                                (
                                    ctx.path.first().map(|e| e.cell_index),
                                    ctx.path.first().map(|e| e.cell_para_index),
                                )
                            } else {
                                (None, None)
                            };
                            let note_ref = if cell_ctx.is_none() {
                                self.note_ref_for_endnote_equation(para_index, tac_ci)
                            } else {
                                None
                            };
                            let eq_node = RenderNode::new(
                                tree.next_id(),
                                RenderNodeType::Equation(
                                    crate::renderer::render_tree::EquationNode {
                                        svg_content,
                                        layout_box,
                                        color_str,
                                        color: eq.color,
                                        script: eq.script.clone(),
                                        font_size: font_size_px,
                                        section_index: note_ref
                                            .as_ref()
                                            .map(|r| r.section_index)
                                            .or(Some(section_index)),
                                        para_index: if let Some(ref ctx) = cell_ctx {
                                            Some(ctx.parent_para_index)
                                        } else {
                                            Some(para_index)
                                        },
                                        control_index: if let Some(ref ctx) = cell_ctx {
                                            ctx.path
                                                .first()
                                                .map(|e| e.control_index)
                                                .or(Some(tac_ci))
                                        } else {
                                            Some(tac_ci)
                                        },
                                        cell_index: eq_cell_idx,
                                        cell_para_index: eq_cell_para_idx,
                                        note_ref,
                                    },
                                ),
                                BoundingBox::new(x, eq_y, tac_w, eq_h),
                            );
                            line_node.children.push(eq_node);
                            // 인라인 좌표 등록 → shape_layout에서 이 수식을 스킵
                            tree.set_inline_shape_position(
                                section_index,
                                para_index,
                                tac_ci,
                                cell_ctx.as_ref(),
                                x,
                                eq_y,
                            );
                        }
                    }
                    if let (Some(p), Some(bdc), Some(host)) = (para, bin_data_content, self.host) {
                        tac_table_om = host.paint_inline_table_in_run(
                            tree,
                            col_node,
                            p,
                            bdc,
                            styles,
                            cell_ctx,
                            col_area,
                            tac_ci,
                            x,
                            y,
                            baseline,
                            raw_lh,
                            alignment,
                            section_index,
                            para_index,
                            line_tac_offsets,
                        );
                    }
                    // 인라인 양식 개체 렌더링
                    if let Some(p) = para {
                        if let Some(Control::Form(f)) = p.controls.get(tac_ci) {
                            let form_h = hwpunit_to_px(f.height as i32, self.dpi);
                            let form_y = (y + baseline - form_h).max(y);
                            // 셀 내부인 경우 cell_location 채우기 — 빈 경로면 None
                            let cell_location = cell_ctx.as_ref().and_then(|ctx| {
                                ctx.path.first().map(|e| {
                                    (
                                        ctx.parent_para_index,
                                        e.control_index,
                                        e.cell_index,
                                        e.cell_para_index,
                                    )
                                })
                            });
                            let form_node = RenderNode::new(
                                tree.next_id(),
                                RenderNodeType::FormObject(FormObjectNode {
                                    form_type: f.form_type,
                                    caption: f.caption.clone(),
                                    text: f.text.clone(),
                                    fore_color: form_color_to_css(f.fore_color),
                                    back_color: form_color_to_css(f.back_color),
                                    value: f.value,
                                    enabled: f.enabled,
                                    section_index,
                                    para_index,
                                    control_index: tac_ci,
                                    name: f.name.clone(),
                                    cell_location,
                                }),
                                BoundingBox::new(x, form_y, tac_w, form_h),
                            );
                            line_node.children.push(form_node);
                        }
                    }
                    // tac 폭만큼 x 전진 (+ TAC 표 outMargin 좌/우 — Issue #3396)
                    x += tac_w + tac_table_om.0 + tac_table_om.1;
                    sub_char_offset += 1;
                    seg_start = tac_rel;
                }

                // 마지막 tac 이후 텍스트 세그먼트 렌더링
                let remaining: String = run_chars[seg_start..].iter().collect();
                if !remaining.is_empty() {
                    let mut seg_style = text_style.clone();
                    seg_style.line_x_offset = x - col_area.x;
                    if has_tabs && remaining.contains('\t') {
                        let positions = compute_char_positions(&remaining, &seg_style);
                        seg_style.tab_leaders = extract_tab_leaders_with_extended(
                            &remaining,
                            &positions,
                            &seg_style,
                            &composed.tab_extended,
                        );
                    }
                    // Keep the tail on the same fractional advance contract as
                    // estimate_line_run_widths and segments before TAC objects.
                    // Integer rounding here changes both bbox and the next run
                    // origin, although backend glyph replay is unrounded.
                    let seg_w = estimate_text_width_exact(&remaining, &seg_style);
                    let trailing_space_count =
                        line_trailing_space_by_run[run_idx].min(remaining.chars().count());
                    let (seg_w, seg_layout_positions) = emitted_run_layout_positions(
                        kerning_layout_session,
                        ExactFontSlot::new(run.char_style_id, run.lang_index),
                        &remaining,
                        &seg_style,
                        seg_w - extra_word_sp * trailing_space_count as f64,
                        trailing_space_count,
                        run.char_overlap.is_none()
                            && !remaining
                                .chars()
                                .any(|character| matches!(character, '\t' | '\n' | '\r')),
                    );
                    {
                        let sub_run_id = tree.next_id();
                        let sub_run_node = RenderNode::new(
                            sub_run_id,
                            RenderNodeType::TextRun(TextRunNode {
                                text: remaining,
                                style: seg_style,
                                char_shape_id: Some(run.char_style_id),
                                para_shape_id: Some(composed.para_style_id),
                                section_index: Some(section_index),
                                para_index: Some(para_index),
                                char_start: Some(sub_char_offset),
                                cell_context: cell_ctx.clone(),
                                is_para_end: is_last_run,
                                is_line_break_end: is_line_break,
                                rotation: 0.0,
                                is_vertical: false,
                                char_overlap: run.char_overlap.clone(),
                                border_fill_id: run_border_fill_id,
                                baseline,
                                field_marker: FieldMarkerType::None,
                                layout_positions: seg_layout_positions,
                                display_text: None,
                            }),
                            BoundingBox::new(x, y, seg_w, line_height),
                        );
                        line_node.children.push(sub_run_node);
                    }
                    x += seg_w;
                } else if is_last_run {
                    // 마지막 run이 tac로 끝나는 경우: 빈 TextRun으로 is_para_end 표시
                    let mut seg_style = text_style.clone();
                    seg_style.line_x_offset = x - col_area.x;
                    let sub_run_id = tree.next_id();
                    let sub_run_node = RenderNode::new(
                        sub_run_id,
                        RenderNodeType::TextRun(TextRunNode {
                            text: String::new(),
                            style: seg_style,
                            char_shape_id: Some(run.char_style_id),
                            para_shape_id: Some(composed.para_style_id),
                            section_index: Some(section_index),
                            para_index: Some(para_index),
                            char_start: Some(sub_char_offset),
                            cell_context: cell_ctx.clone(),
                            is_para_end: true,
                            is_line_break_end: is_line_break,
                            rotation: 0.0,
                            is_vertical: false,
                            char_overlap: None,
                            border_fill_id: 0,
                            baseline,
                            field_marker: FieldMarkerType::None,
                            layout_positions: None,
                            display_text: None,
                        }),
                        BoundingBox::new(x, y, 0.0, line_height),
                    );
                    line_node.children.push(sub_run_node);
                }
                // x는 이미 sub-run 루프에서 갱신됨 (x += full_width 생략)
            }

            char_offset += run_char_count;
            run_char_pos = run_char_end;
            inline_tab_cursor_render += run.text.chars().filter(|c| *c == '\t').count();
            char_x_map.push((char_offset, x));
        }
        RunEmitState {
            x,
            y,
            char_offset,
            run_char_pos,
            inline_tab_cursor_render,
            pending_right_tab_render,
            pending_right_leader_digit_render,
            current_line_reserved_tac_picture_height,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn estimate_line_run_widths(
        &self,
        comp_line: &crate::renderer::composer::ComposedLine,
        composed: &ComposedParagraph,
        para: Option<&Paragraph>,
        styles: &ResolvedStyleSet,
        tab_stops: &[TabStop],
        tab_width: f64,
        auto_tab_right: bool,
        line_tac_offsets_for_width: &[(usize, f64, usize)],
        effective_margin_left: f64,
        available_width: f64,
        start_line: usize,
        line_idx: usize,
        est_x_init: f64,
    ) -> LineWidthEst {
        let mut est_x = est_x_init;
        let mut pending_right_tab_est: Option<(f64, u8, u8)> = None;
        let mut pending_right_leader_digit_est = false;
        let mut run_char_pos_est = comp_line.char_start;
        let mut included_tac_width_in_est = 0.0f64;
        // cross-run 탭 감지용 inline_tabs(composed.tab_extended) 커서 — Task #290
        let mut inline_tab_cursor_est: usize = 0;
        for (run_idx_est, run) in comp_line.runs.iter().enumerate() {
            let run_char_count_est = if run.char_overlap.is_some() {
                let chars: Vec<char> = run.text.chars().collect();
                crate::renderer::composer::char_overlap_advance_units(&chars)
            } else {
                run.text.chars().count()
            };
            let run_char_end_est = run_char_pos_est + run_char_count_est;
            let mut ts = run.text_style(styles);
            ts.default_tab_width = tab_width;
            ts.tab_stops = tab_stops.to_vec();
            ts.auto_tab_right = auto_tab_right;
            ts.available_width = available_width;
            ts.text_start_offset = effective_margin_left;
            ts.inline_tabs = composed.tab_extended.clone();
            if pending_right_leader_digit_est {
                if run.text.trim().is_empty() {
                    pending_right_leader_digit_est = true;
                } else {
                    if run.text.trim().chars().all(|ch| ch.is_ascii_digit()) {
                        if let Some(tab) = tab_stops
                            .iter()
                            .rev()
                            .find(|tab| tab.tab_type == 1 && tab.fill_type != 0)
                        {
                            let digit_w = estimate_text_width_exact(run.text.trim(), &ts);
                            let target =
                                if composed.tab_extended.is_empty() && available_width > 0.0 {
                                    effective_margin_left + available_width
                                } else {
                                    tab.position
                                };
                            let gap = if composed.tab_extended.is_empty() {
                                0.0
                            } else {
                                ts.font_size * 0.25
                            };
                            est_x = target - gap - digit_w;
                        }
                    }
                    pending_right_leader_digit_est = false;
                }
            }
            // 교차 run 오른쪽/가운데 탭: 이 run의 시작 위치를 역방향으로 조정
            if let Some((tab_pos, tab_type, fill_type)) = pending_right_tab_est.take() {
                // [Task #279] 공백만 있는 run 은 right/center tab 정렬 단위가 아니다.
                // (장제목 케이스: " " 단독 run → carry-over)
                if (tab_type == 1 || tab_type == 2) && run.text.trim().is_empty() {
                    pending_right_tab_est = Some((tab_pos, tab_type, fill_type));
                } else {
                    ts.line_x_offset = est_x;
                    // [Task #279] 리더(fill_type ≠ 0) 가 있는 RIGHT 탭은 "이 줄 우측 끝까지" 의미.
                    // 셀 안 문단에서는 col_area 가 이미 cell padding 적용된 inner_area 이므로
                    // `effective_margin_left + available_width` 가 inner 우측 끝.
                    // [Task #874] auto_tab_right 의 tab_pos = available_width (text-start
                    // 상대). RIGHT 탭은 모두 col-start 좌표계로 변환 시 effective_margin_left
                    // 더해야 함. 종전엔 fill_type ≠ 0 만 변환되어 leader 없는 auto_right tab
                    // (shortcut.hwp 인쇄/개체 모양 복사 등) 가 ~27 px 왼쪽으로 밀려 렌더됨.
                    let effective_pos = if tab_type == 1 {
                        effective_margin_left
                            + (if fill_type != 0 {
                                available_width
                            } else {
                                tab_pos
                            })
                    } else {
                        tab_pos
                    };
                    // [Issue #842 #4] 탭 다음 콘텐츠가 여러 composed run 으로 쪼개진 경우
                    // (스크립트·char-shape 경계) 전체 블록 폭 기준으로 정렬해야 마지막 글자가
                    // 탭스톱에 맞는다. (선행 공백 run "예 16" 케이스도 합산에 포함되어 동작 유지.)
                    let run_w = right_tab_block_width(
                        &comp_line.runs,
                        run_idx_est,
                        styles,
                        tab_width,
                        &tab_stops,
                        auto_tab_right,
                        available_width,
                    );
                    match tab_type {
                        1 => {
                            est_x = effective_pos - run_w;
                        }
                        2 => {
                            est_x = effective_pos - run_w / 2.0;
                        }
                        _ => {}
                    }
                }
            }
            // 글자겹침 run: PUA 다자리 숫자는 1글자 폭, 그 외는 font_size * char_count
            if run.char_overlap.is_some() {
                let fs = if ts.font_size > 0.0 {
                    ts.font_size
                } else {
                    12.0
                };
                let chars: Vec<char> = run.text.chars().collect();
                let w = fs * crate::renderer::composer::char_overlap_advance_units(&chars) as f64;
                est_x += w;
                run_char_pos_est = run_char_end_est;
                inline_tab_cursor_est += run.text.chars().filter(|c| *c == '\t').count();
                continue;
            }
            // treat_as_char 분기점 처리: run 내 tac 위치에서 이미지 폭 삽입
            // 마지막 run에서는 run_char_end 위치의 TAC도 포함
            //
            // [Task #1219] TAC 소스를 줄-경계 정규 집합 `line_tac_offsets`
            // (= tac_offsets_for_line, 렌더 경로와 동일한 `pos < 다음 줄 시작`
            // 엄격 미만 규칙)로 통일한다. 전역 tac_offsets_px 를 run 경계로
            // 재필터링하면 줄 끝 위치(== 다음 줄 선두)의 수식이 현재 줄 폭에
            // 오포함되어(문26 라인0 에 다음 줄 `a₁=b₁=1` 55px) 거짓 오버플로우
            // → 본문 한글 압축이 발생했다. line_tac_offsets 는 이미 줄-범위로
            // 필터링되어 있으므로 run 범위 필터만 적용한다.
            //
            // [Task #1285] 단, 오른쪽 정렬된 셀 안에서 `TAC 표 + 공백 + TAC 표`가
            // 같은 마지막 줄에 놓이는 경우 두 번째 TAC 표는 run 끝 위치(pos == end)에
            // 기록된다. 일반 줄 경계 판정에는 포함하지 않고, 위에서 좁게 만든
            // line_tac_offsets_for_width 에만 넣어 부모 줄 오른쪽 정렬 폭을 맞춘다.
            let run_chars_est: Vec<char> = run.text.chars().collect();
            let mut seg_start_est = 0usize;
            let is_last_run_est_tac = run_char_end_est
                >= comp_line
                    .runs
                    .iter()
                    .map(|r| r.text.chars().count())
                    .sum::<usize>()
                    + comp_line.char_start;
            for &(tac_abs_pos, tac_w, _) in
                line_tac_offsets_for_width.iter().filter(|(pos, _, _)| {
                    *pos >= run_char_pos_est
                        && (*pos < run_char_end_est
                            || (is_last_run_est_tac && *pos == run_char_end_est))
                })
            {
                let tac_rel = tac_abs_pos - run_char_pos_est;
                if seg_start_est < tac_rel {
                    let seg: String = run_chars_est[seg_start_est..tac_rel].iter().collect();
                    ts.line_x_offset = est_x;
                    est_x += estimate_text_width_exact(&seg, &ts);
                }
                est_x += tac_w;
                included_tac_width_in_est += tac_w;
                seg_start_est = tac_rel;
            }
            // 마지막 세그먼트 처리
            let mut remaining_est: String = run_chars_est[seg_start_est..].iter().collect();
            // TAC 로 쪼개지지 않은 런은 통째로 재므로, 표시 길이가 모델과 다르면
            // **그려지는 글자**로 잰다. 이 자연 폭이 정렬 간격 분배의 기준이라, 모델로
            // 재면 남는 폭이 과대평가돼 글자가 흩어진다 (Task #3216).
            if seg_start_est == 0 && run.display_text.is_some() {
                remaining_est = effective_text_for_metrics(run).to_string();
            }
            ts.line_x_offset = est_x;
            // [Task #874 #2] composer lang split (예: "F3→Alt+I" → "F3"/"→"/"Alt+I")
            // 으로 auto_tab_right post-tab 콘텐츠가 후속 run 으로 흩어진 경우, 현재
            // run 내부 seg_w 만으로는 우측 정렬 위치가 어긋남. 후속 run 합산을 미리
            // 계산해 ts.right_tab_block_width_override 로 주입한다.
            if auto_tab_right
                && remaining_est.contains('\t')
                && run_idx_est + 1 < comp_line.runs.len()
            {
                let tab_byte = remaining_est.rfind('\t').unwrap();
                let post_tab: String = remaining_est[tab_byte + '\t'.len_utf8()..].to_string();
                let no_more_tabs_after_in_run = !post_tab.contains('\t');
                let no_tabs_in_subsequent = comp_line
                    .runs
                    .iter()
                    .skip(run_idx_est + 1)
                    .all(|r| !r.text.contains('\t'));
                if no_more_tabs_after_in_run && no_tabs_in_subsequent {
                    let mut ts_measure = ts.clone();
                    ts_measure.right_tab_block_width_override = None;
                    let post_tab_w = estimate_text_width_exact(&post_tab, &ts_measure);
                    let subsequent_w = right_tab_block_width(
                        &comp_line.runs,
                        run_idx_est + 1,
                        styles,
                        tab_width,
                        &tab_stops,
                        auto_tab_right,
                        available_width,
                    );
                    ts.right_tab_block_width_override = Some(post_tab_w + subsequent_w);
                }
            }
            if !remaining_est.is_empty() {
                est_x += estimate_text_width_exact(&remaining_est, &ts);
            }
            // run이 \t로 끝나면 다음 run에 오른쪽/가운데 탭 조정 필요 — Task #290:
            // inline_tabs(composed.tab_extended) 가 LEFT 를 명시하면 cross-run pending 을 설정하지 않는다.
            // [Task #279] trailing 공백 (\t 뒤에 따라오는 ' ') 도 허용 — 목차 소제목의
            // 들여쓰기 문단에서 한컴이 "\t " 형태로 저장하는 케이스가 있음.
            let trimmed_end = run
                .text
                .trim_end_matches(|c: char| c == ' ' || c == '\u{2007}');
            if trimmed_end.ends_with('\t') {
                let run_tab_count = run.text.chars().filter(|c| *c == '\t').count();
                if run_tab_count > 0 {
                    let last_inline_idx = inline_tab_cursor_est + run_tab_count - 1;
                    pending_right_tab_est = resolve_last_tab_pending(
                        &run.text,
                        last_inline_idx,
                        &composed.tab_extended,
                        &ts,
                        &tab_stops,
                        tab_width,
                        auto_tab_right,
                        available_width,
                    );
                }
            }
            if run.text.contains('\t')
                && run
                    .text
                    .rsplit_once('\t')
                    .map(|(_, after)| after.trim().is_empty())
                    .unwrap_or(false)
                && tab_stops
                    .iter()
                    .any(|tab| tab.tab_type == 1 && tab.fill_type != 0)
            {
                pending_right_leader_digit_est = true;
            }
            // 각주 마커 폭: run 내에 각주가 있으면 마커 위첨자 폭 추가
            let is_last_run_est = run_char_end_est
                >= comp_line
                    .runs
                    .iter()
                    .map(|r| r.text.chars().count())
                    .sum::<usize>()
                    + comp_line.char_start;
            for &(fpos, fnum, ctrl_idx) in composed.footnote_positions.iter() {
                // [Task #1219 Stage 1b] 선두 미주 마커는 endnote_marker_x_advance
                // 가 풀사이즈 선두 마커로 렌더하고 그 폭을 inline_offset 에 이미
                // 반영했다(available_width 에서 차감). 렌더 경로는 이 미주의 인라인
                // 위첨자를 그리지 않으므로(문26 "공" x=78=선두 마커 끝), 측정에서도
                // est_x 에 위첨자 폭을 더하면 이중 계상 → 거짓 오버플로우.
                // start_line==0 의 미주(= endnote_marker_x_advance 처리 대상)는 제외.
                let is_leading_endnote_marker = is_leading_endnote_marker_rendered_as_prefix(
                    para,
                    ctrl_idx,
                    line_idx,
                    start_line,
                    fpos,
                    comp_line.char_start,
                );
                if is_leading_endnote_marker {
                    continue;
                }
                if fpos >= run_char_pos_est
                    && (fpos < run_char_end_est || (is_last_run_est && fpos == run_char_end_est))
                {
                    let fn_text = note_marker_text_from_control(
                        para.and_then(|p| p.controls.get(ctrl_idx)),
                        fnum,
                    );
                    let sup_size = (ts.font_size * 0.55).max(7.0);
                    let sup_ts = TextStyle {
                        font_size: sup_size,
                        font_family: ts.font_family.clone(),
                        ..Default::default()
                    };
                    est_x += estimate_text_width_exact(&fn_text, &sup_ts);
                }
            }
            run_char_pos_est = run_char_end_est;
            inline_tab_cursor_est += run.text.chars().filter(|c| *c == '\t').count();
        }
        LineWidthEst {
            est_x,
            included_tac_width: included_tac_width_in_est,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn layout_empty_runs_line(
        &self,
        tree: &mut PageLayoutContext,
        line_node: &mut RenderNode,
        comp_line: &crate::renderer::composer::ComposedLine,
        composed: &ComposedParagraph,
        para: Option<&Paragraph>,
        bin_data_content: Option<&[BinDataContent]>,
        styles: &ResolvedStyleSet,
        cell_ctx: &Option<CellContext>,
        line_tac_offsets: &[(usize, f64, usize)],
        col_area: &LayoutRect,
        vars: EmptyRunsLineVars,
        current_line_reserved_tac_picture_height: &mut Option<f64>,
    ) {
        let mut empty_line_mark_x = vars.x_start;
        let mut empty_line_logical_end = vars.line_char_end;
        // runs가 없는 빈 줄에서 treat_as_char 이미지 렌더링
        // 테이블 셀 내부에서는 table_layout.rs가 layout_picture로 이미 처리하므로 스킵.
        // 셀 외부에서 해당 줄 범위에 걸린 TAC만 여기서 렌더링.
        //
        // [#5727] 예외 — 저장 lineseg 가 TAC 개체에 배정한 자기 줄(빈 줄, 다음
        // 줄과 char_start 동일)은 셀 안에서도 여기서 그린다. 이 줄 소유 TAC 는
        // 다음 줄 run 귀속에서 제외되므로 여기서 그리지 않으면 개체가 사라진다.
        // 이미 등록된 개체는 건너뛰어 다른 경로와의 이중 렌더를 막는다.
        let owns_boundary_tac = composed
            .lines
            .get(vars.line_idx + 1)
            .is_some_and(|next| next.char_start == comp_line.char_start && !next.runs.is_empty());
        let empty_line_tac_allowed =
            cell_ctx.is_none() || is_caption_cell_context(cell_ctx.as_ref()) || owns_boundary_tac;
        if empty_line_tac_allowed && !line_tac_offsets.is_empty() {
            if let (Some(p), Some(bdc)) = (para, bin_data_content) {
                // TAC 이미지 전체 폭 계산 후 문단 정렬 적용
                let total_tac_width: f64 = line_tac_offsets.iter().map(|(_, w, _)| *w).sum();
                let align_offset = match vars.alignment {
                    Alignment::Center | Alignment::Distribute => {
                        (vars.available_width - total_tac_width).max(0.0) / 2.0
                    }
                    Alignment::Right => (vars.available_width - total_tac_width).max(0.0),
                    _ => 0.0, // Left, Justify
                };
                let mut img_x = vars.effective_col_x + vars.effective_margin_left + align_offset;
                for &(_, tac_w, tac_ci) in line_tac_offsets {
                    if let Some(ctrl) = p.controls.get(tac_ci) {
                        // [Issue #476] 빈 문단 + 인라인 Shape: inline_pos 등록 후 shape_layout 이 그리도록 위임.
                        // 등록하지 않으면 layout_shape 가 inline_pos=None 으로 받아 fallback 위치에 그리거나,
                        // #476 의 fallback 차단 분기로 박스가 누락된다.
                        if let Control::Shape(shape) = ctrl {
                            let common = shape.common();
                            let shape_h = hwpunit_to_px(shape.flow_height_hu(), self.dpi);
                            // [#5789] 빈 run 줄은 max_fs=0 이라 vars.baseline 이 0 으로
                            // 접힌다 — TAC 개체는 글자처럼 baseline 에 앉아야 하므로
                            // 저장 줄의 baseline_distance 로 폴백한다 (3143955 이중선:
                            // 줄 상자 top 161.99 ↔ 한글 baseline 182.4, 20.4px 어긋남).
                            let baseline = if vars.baseline > 0.01 {
                                vars.baseline
                            } else {
                                hwpunit_to_px(comp_line.baseline_distance, self.dpi)
                            };
                            // [#6606] 상자(도형 + 위아래 여백)를 baseline 에 앉히고 도형은
                            // 상자의 (왼쪽, 위) 여백 안쪽에 둔다 — TAC 그림(#6603)과 같은 계약.
                            let (margin_left, _, margin_top, margin_bottom) =
                                tac_object_outer_margins_px(common, self.dpi);
                            let box_h = shape_h + margin_top + margin_bottom;
                            let shape_y = (vars.y + baseline - box_h).max(vars.y) + margin_top;
                            tree.set_inline_shape_position(
                                vars.section_index,
                                vars.para_index,
                                tac_ci,
                                cell_ctx.as_ref(),
                                img_x + margin_left,
                                shape_y,
                            );
                            img_x += tac_w;
                            empty_line_mark_x = img_x;
                            empty_line_logical_end += 1;
                            continue;
                        }
                        if matches!(ctrl, Control::Table(_)) {
                            if let Some(host) = self.host {
                                if host.paint_inline_table_in_empty_line(
                                    tree,
                                    line_node,
                                    p,
                                    ctrl,
                                    bin_data_content,
                                    styles,
                                    cell_ctx,
                                    col_area,
                                    tac_ci,
                                    img_x,
                                    vars.y,
                                    vars.alignment,
                                    vars.section_index,
                                    vars.para_index,
                                ) {
                                    img_x += tac_w;
                                    empty_line_mark_x = img_x;
                                    empty_line_logical_end += 1;
                                }
                            }
                            continue;
                        }
                        if let Control::Picture(pic) = ctrl {
                            // [#5727] 셀 안 경로는 다른 패스가 먼저 그렸을 수 있다 —
                            // 등록된 개체는 건너뛰어 이중 렌더를 막는다.
                            if cell_ctx.is_some()
                                && tree
                                    .get_inline_shape_position(
                                        vars.section_index,
                                        vars.para_index,
                                        tac_ci,
                                        cell_ctx.as_ref(),
                                    )
                                    .is_some()
                            {
                                img_x += tac_w;
                                empty_line_mark_x = img_x;
                                empty_line_logical_end += 1;
                                continue;
                            }
                            let (pic_w, pic_h) = self.resolve_inline_picture_size(pic, col_area);
                            // [#6603] 잉크는 바깥 여백 상자의 (왼쪽, 위) 안쪽에 그린다.
                            let (margin_left, _, margin_top, margin_bottom) =
                                tac_picture_outer_margins_px(pic, self.dpi);
                            // LINE_SEG vpos가 TopAndBottom 흐름 위치를 이미 담고 있으면
                            // sibling 예약 높이를 다시 더하지 않는다.
                            let sibling_reserved_px = if vars.has_topbottom_vpos_base {
                                0.0
                            } else {
                                let raw = hwpunit_to_px(
                                    calc_sibling_topandbottom_reserved_hu(&p.controls),
                                    self.dpi,
                                );
                                // 위 텍스트 줄 경로와 동일한 가드 — 편집 세션은
                                // typeset 흐름이 재배정을 끝냈으므로 예약을 가산하지
                                // 않고, 열람은 이중 가산(줄 y 가 이미 예약 아래)만
                                // 차단한다.
                                if self.profile.get().session_edited() {
                                    0.0
                                } else if raw > 40.0 && vars.y >= raw - 4.0 {
                                    0.0
                                } else {
                                    raw
                                }
                            };
                            if vars.raw_lh + 4.0 >= pic_h {
                                *current_line_reserved_tac_picture_height = Some(pic_h);
                            }
                            let label_extra = tac_picture_label_extra_for_line(
                                cell_ctx.as_ref(),
                                vars.runs_all_whitespace,
                                vars.raw_lh,
                                *current_line_reserved_tac_picture_height,
                                vars.max_fs,
                                vars.line_spacing_px,
                            );
                            let base_img_y = if label_extra > 0.0 {
                                vars.y + label_extra
                            } else {
                                // [#6575] baseline 정렬 대상은 그림이 아니라 개체 상자 전체다.
                                // [#6603] 상자에는 위아래 바깥 여백도 들어간다.
                                let box_h = tac_object_box_height_px(pic_h, &pic.caption, self.dpi)
                                    + margin_top
                                    + margin_bottom;
                                (vars.y + vars.baseline - box_h).max(vars.y)
                            };
                            let img_y = base_img_y + sibling_reserved_px + margin_top;
                            let bin_data_id = pic.image_attr.bin_data_id;
                            let image_data = find_bin_data_bytes(bdc, bin_data_id);
                            let crop = {
                                let c = &pic.crop;
                                if c.right > c.left
                                    && c.bottom > c.top
                                    && (c.left != 0 || c.top != 0 || c.right != 0 || c.bottom != 0)
                                {
                                    Some((c.left, c.top, c.right, c.bottom))
                                } else {
                                    None
                                }
                            };
                            let original_size_hu = pic.crop_reference_size();
                            // [Task #1151 v7 항목 7] ImageNode 생성 helper 통합.
                            let img_node = make_picture_image_node(
                                tree,
                                pic,
                                vars.section_index,
                                vars.para_index,
                                tac_ci,
                                cell_ctx.as_ref(),
                                crop,
                                original_size_hu,
                                bin_data_id,
                                image_data,
                                BoundingBox::new(img_x + margin_left, img_y, pic_w, pic_h),
                            );
                            line_node.children.push(img_node);
                            // [Task #418/#376] layout_shape_item 의 Task #347 분기 (빈 문단 +
                            // TAC Picture 직접 emit) 와 이중 렌더링되지 않도록 인라인 위치를
                            // 등록한다. layout_shape_item 은 등록된 경우 push 를 스킵한다.
                            tree.set_inline_shape_position(
                                vars.section_index,
                                vars.para_index,
                                tac_ci,
                                cell_ctx.as_ref(),
                                img_x + margin_left,
                                img_y,
                            );
                            img_x += tac_w;
                            empty_line_mark_x = img_x;
                            empty_line_logical_end += 1;
                        }
                    }
                }
            }
        }

        let run_id = tree.next_id();
        let (text_style, char_shape_id) =
            paragraph_active_text_style(styles, para, vars.line_char_end);
        let run_node = RenderNode::new(
            run_id,
            RenderNodeType::TextRun(TextRunNode {
                text: String::new(),
                style: text_style,
                char_shape_id,
                para_shape_id: Some(composed.para_style_id),
                section_index: Some(vars.section_index),
                para_index: Some(vars.para_index),
                char_start: Some(empty_line_logical_end),
                cell_context: cell_ctx.clone(),
                is_para_end: vars.is_last_line_of_para && !vars.defer_empty_line_control_marker,
                is_line_break_end: comp_line.has_line_break
                    && !vars.defer_empty_line_control_marker,
                rotation: 0.0,
                is_vertical: false,
                char_overlap: None,
                border_fill_id: 0,
                baseline: vars.baseline,
                field_marker: FieldMarkerType::None,
                layout_positions: None,
                display_text: None,
            }),
            BoundingBox::new(
                empty_line_mark_x,
                vars.y,
                if vars.physical_frame_rows || empty_line_mark_x > vars.x_start {
                    0.0
                } else {
                    vars.available_width
                },
                vars.line_flow_height,
            ),
        );
        line_node.children.push(run_node);
    }
}
