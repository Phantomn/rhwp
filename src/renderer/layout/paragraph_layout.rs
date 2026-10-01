//! 문단 레이아웃 (인라인 표, 문단 전체/부분, composed/raw) + 번호 매기기

use super::super::composer::{
    compose_paragraph, effective_text_for_metrics, ComposedLine, ComposedParagraph, ComposedTextRun,
};
use super::super::height_measurer::MeasuredTable;
use super::super::kerning::{
    ExactFontSlot, KerningLayoutSession, KerningRunMeasurementDisposition,
};
use super::super::page_layout::LayoutRect;
use super::super::render_tree::*;
use super::super::style_resolver::ResolvedStyleSet;
use super::super::{
    format_number, hwpunit_to_px, px_to_hwpunit, AutoNumberCounter, NumberFormat as NumFmt,
    ShapeStyle, TabStop, TextStyle,
};
use super::border_rendering::create_border_line_nodes;
use super::text_measurement::{
    compute_char_positions, estimate_text_width, estimate_text_width_exact,
    estimate_text_width_unrounded, extract_tab_leaders_with_extended, find_next_tab_stop,
    resolved_to_text_style,
};
use super::utils::{
    expand_numbering_format, extract_shape_transform, find_bin_data_bytes,
    numbering_format_to_number_format, picture_display_size_hu, resolve_numbering_id,
};
use super::{CellContext, LayoutEngine};
use crate::model::bin_data::BinDataContent;
use crate::model::control::Control;
use crate::model::paragraph::{LineSeg, Paragraph};
use crate::model::shape::{
    Caption, CaptionDirection, CommonObjAttr, HorzAlign, HorzRelTo, ShapeObject, TextWrap,
    VertRelTo,
};
use crate::model::style::{Alignment, HeadType, LineSpacingType, Numbering, UnderlineType};
use crate::model::table::Table;

pub(crate) use crate::renderer::paragraph_paint::helpers::*;

/// 인라인으로 분류된 TAC 표의 줄바꿈 여부. 표 분류·배치는 Legacy 소유다.
pub(crate) fn should_wrap_middle_anchored_table(
    control_position: Option<usize>,
    text_len: usize,
    occupied_width: f64,
    table_footprint: f64,
    line_width: f64,
) -> bool {
    // [#4370] 끝 앵커(position == text_len)도 포함한다 — 본문 텍스트 뒤에 붙은
    // tac 표가 남은 줄 폭에 안 들어가면 페이지 우측 밖으로 방출되던 결함.
    control_position.is_some_and(|position| position > 0 && position <= text_len)
        && occupied_width > 1.0
        && occupied_width + table_footprint > line_width + 0.5
}

/// 선행 inline TAC 표의 Bottom caption이 첫 저장 줄을 소유하고, 표 뒤의 첫 visible
/// 문자가 두 번째 저장 줄에서 시작하는 좁은 HWP5 계약인지 판정한다.
///
/// `LINE_SEG.text_start`는 extended control의 8 UTF-16 unit을 포함한다. 따라서 선행
/// 표 하나 뒤의 첫 글자 offset과 `line_segs[1].text_start`가 모두 8이면, visible text
/// 관점의 break index는 0이다. 일반 문단에서 index 0을 허용하면 저장 정보가 불충분한
/// control 문단까지 강제 개행할 수 있으므로 아래 구조가 모두 입증될 때만 보존한다.
pub(crate) fn preserves_stored_first_visible_break_after_bottom_caption_table(
    para: &Paragraph,
) -> bool {
    let Some(&first_visible_offset) = para.char_offsets.first() else {
        return false;
    };
    // [#5961] `first_visible_offset` 은 `char_offsets` 값이라 HWP5 축이다. 저장
    // `text_start` 는 출처에 따라 더 짧은 축일 수 있으므로 올려서 견준다.
    if first_visible_offset != 8
        || para.text.is_empty()
        || para.line_segs.first().map(|ls| ls.text_start) != Some(0)
        || para.line_segs.len() < 2
        || para.line_seg_text_start(1) != first_visible_offset
    {
        return false;
    }

    let control_positions = para.control_text_positions();
    let mut leading_controls = para
        .controls
        .iter()
        .enumerate()
        .filter(|(control_index, _)| control_positions.get(*control_index) == Some(&0));
    let Some((_, Control::Table(table))) = leading_controls.next() else {
        return false;
    };
    // first_visible_offset == 8은 저장 stream의 선행 extended control이 정확히 하나라는
    // 뜻이다. IR에서도 owner를 하나로 확정해 다른 co-anchored control에는 확장하지 않는다.
    if leading_controls.next().is_some() {
        return false;
    }

    let has_bottom_caption = table.caption.as_ref().is_some_and(|caption| {
        caption.direction == CaptionDirection::Bottom && !caption.paragraphs.is_empty()
    });
    let segment_width = para
        .line_segs
        .first()
        .map(|ls| ls.segment_width)
        .unwrap_or_default();

    table.common.treat_as_char
        && has_bottom_caption
        && segment_width > 0
        && crate::renderer::height_measurer::is_tac_table_inline_in_para(table, segment_width, para)
}

/// inline TAC 문단의 저장 `LINE_SEG` 시작점을 visible character index로 변환한다.
///
/// 보통 index 0은 실질적인 개행이 아니므로 제외한다. 단,
/// [`preserves_stored_first_visible_break_after_bottom_caption_table`]가 소유권을 증명하면
/// 두 번째 저장 줄의 index 0만 보존한다.
pub(crate) fn inline_table_stored_line_break_char_indices(para: &Paragraph) -> Vec<usize> {
    inline_table_stored_line_breaks(para)
        .into_iter()
        .map(|(char_idx, _)| char_idx)
        .collect()
}

/// [#6181] 위와 같은 줄 나눔 목록에 **그 줄을 소유한 `line_segs` 인덱스**를 함께 준다.
///
/// `char_idx == 0` 인 저장 줄은 실질 개행이 아니라 걸러지므로, 목록의 n 번째 나눔이
/// `line_segs[n + 1]` 이라고 가정할 수 없다. 줄 상단을 저장 `vertical_pos` 로 잡으려면
/// 그 대응이 필요하다.
pub(crate) fn inline_table_stored_line_breaks(para: &Paragraph) -> Vec<(usize, usize)> {
    if para.line_segs.len() <= 1 || para.char_offsets.is_empty() {
        return Vec::new();
    }

    let text_len = para.text.chars().count();
    let preserves_first_visible_break =
        preserves_stored_first_visible_break_after_bottom_caption_table(para);
    let mut indices: Vec<(usize, usize)> = Vec::new();
    for (line_index, _line_seg) in para.line_segs.iter().enumerate().skip(1) {
        // [#5961] `char_offsets` 는 HWP5 축이므로 저장 `text_start` 를 같은 자로 올린다.
        let seg_start = para.line_seg_text_start(line_index);
        let char_idx = para
            .char_offsets
            .iter()
            .position(|&offset| offset >= seg_start)
            .unwrap_or(text_len);
        let is_owned_first_visible_break =
            line_index == 1 && char_idx == 0 && preserves_first_visible_break;
        if (char_idx > 0 || is_owned_first_visible_break)
            && char_idx <= text_len
            && indices
                .last()
                .map(|&(previous, _)| char_idx > previous)
                .unwrap_or(true)
        {
            indices.push((char_idx, line_index));
        }
    }
    indices
}

/// [#6181] 인라인 TAC 표를 품은 문단의 줄 상단은 저장 `LINE_SEG` 의 `vertical_pos` 델타가
/// 정답지다 — 첫 줄 기준 오프셋(px)을 준다.
///
/// 종전에는 첫 줄바꿈을 **표 하단**(`max_table_bottom`)에 붙였다. 표 폭 때문에 글이
/// 아래로 밀리는 형상에는 맞지만, 표가 줄 안에 들어가는(`신규` 배지 같은) 문단에서는
/// 앞 줄의 `line_height` 초과분과 `line_spacing` 을 함께 버린다.
///
/// 실측(156562368 인쇄 4쪽 para#114·#119, `lineSpacing PERCENT 120`):
///
/// ```text
/// ls[0] vertpos=24112 vertsize=1778 spacing=976
/// ls[1] vertpos=26866 vertsize=1500 spacing=976   → 델타 2754HU = 36.72px
/// rhwp 실측 20.00px (= 표 하단, 뒤 줄 vertsize 만) — 13.6px(40%) 소실
/// ```
///
/// 합성 사다리(`TAG_IMPLEMENTATION_PROPERTY`)와 전진하지 않는 값은 `None` 으로 물러나
/// 종전 추정을 그대로 쓴다.
pub(crate) fn inline_table_stored_line_top_offset_px(
    para: &Paragraph,
    line_index: usize,
    dpi: f64,
) -> Option<f64> {
    if line_index == 0 {
        return None;
    }
    let synth = crate::model::paragraph::LineSeg::TAG_IMPLEMENTATION_PROPERTY;
    let first = para.line_segs.first()?;
    let seg = para.line_segs.get(line_index)?;
    if first.tag & synth != 0 || seg.tag & synth != 0 {
        return None;
    }
    let delta = seg.vertical_pos - first.vertical_pos;
    // [#4599] 사다리 델타가 이 문단 **자신의** 줄 진행량 합을 크게 넘으면, 그 초과분은
    // 문단이 아니라 이미 따로 그려진 자리차지 밴드의 공간이다. 그대로 쓰면 밴드를 두 번
    // 센다(36374873 야간방호일지 pi=4: 기대 3014HU 자리에 54302HU — 초과 51288HU=683.8px
    // 가 pi=3 소유 13x8 자리차지 표의 공간이고, 그 표는 이미 Table 항목으로 726.7px
    // 소비했다). #6181 의 표본은 델타가 진행량 합과 **정확히** 일치하므로(1778+976=2754)
    // 영향받지 않는다. 어긋나면 None 으로 물러나 종전 추정(표 하단 기준)을 쓴다.
    let own_advance: i32 = para.line_segs[..line_index]
        .iter()
        .map(|s| s.line_height.max(s.text_height) + s.line_spacing)
        .sum();
    if own_advance > 0 && delta > own_advance + own_advance / 4 {
        return None;
    }
    (delta > 0).then(|| hwpunit_to_px(delta, dpi))
}

impl LayoutEngine {
    /// [#5729] 저장 줄 밴드가 정확히 `om_top + 선언높이 + om_bottom` 인 TAC 표는
    /// 한글이 표 상단을 **줄 상단 + om_top** 에 앉힌다 (156505870 4표 실측:
    /// 밴드 5195=283+4629+283 등 전부 일치). 종전 baseline-하단 정렬은 측정
    /// 높이 흔들림이 그대로 y 오차가 되어 이중 괘선 사이가 4.3px 벌어지고
    /// 글자가 위 괘선을 뚫었다. 밴드 증거가 없으면 None(종전 경로).
    fn tac_table_stored_outer_band_top(
        &self,
        para: &Paragraph,
        tbl: &crate::model::table::Table,
        current_y: f64,
    ) -> Option<f64> {
        if !Self::tac_stored_band_is_outer_box(para, tbl) {
            return None;
        }
        Some(current_y + hwpunit_to_px(tbl.outer_margin_top as i32, self.dpi))
    }

    /// [#5729] 호스트 줄의 저장 밴드가 정확히 `om_top + 선언높이 + om_bottom`
    /// 인가 — 참이면 한글은 표 상단을 줄 상단 + om_top 에 앉힌다.
    pub(crate) fn tac_stored_band_is_outer_box(
        para: &Paragraph,
        tbl: &crate::model::table::Table,
    ) -> bool {
        let om_top_hu = i64::from(tbl.outer_margin_top);
        let om_bottom_hu = i64::from(tbl.outer_margin_bottom);
        if om_top_hu <= 0 || om_bottom_hu <= 0 {
            return false;
        }
        let declared = i64::from(tbl.common.height.min(i32::MAX as u32));
        if declared <= 0 {
            return false;
        }
        // 이 헬퍼에는 control 위치가 전달되지 않는다. 저장 밴드가 첫 줄이라는 사실만으로
        // 문단 안의 모든 TAC 표가 그 줄을 소유한다고 확대하면, 뒤 segment의 표까지
        // 줄바꿈을 건너뛰게 된다. 단일 segment에서는 그 소유 관계가 자명하고, 다중
        // segment는 control별 line-seg 조회가 가능한 별도 경로가 생길 때까지 종전 배치를
        // 유지한다.
        if para.line_segs.len() != 1 {
            return false;
        }
        let Some(ls) = para.line_segs.first() else {
            return false;
        };
        if ls.tag & crate::model::paragraph::LineSeg::TAG_IMPLEMENTATION_PROPERTY != 0 {
            return false;
        }
        (i64::from(ls.line_height) - (om_top_hu + declared + om_bottom_hu)).abs() <= 8
    }

    /// #6812: 측정/fit 소유자가 확정한 줄 결과를 그린다. 여기서 회피·줄바꿈을 재판정하지 않는다.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn layout_inline_flow_plan(
        &self,
        tree: &mut PageLayoutContext,
        col_node: &mut RenderNode,
        para: &Paragraph,
        styles: &ResolvedStyleSet,
        col_area: &LayoutRect,
        section_index: usize,
        para_index: usize,
        bin_data_content: &[BinDataContent],
        measured_tables: &[MeasuredTable],
        plan: &crate::renderer::inline_flow::InlineFlowPlan,
    ) {
        use crate::renderer::inline_flow::InlineFlowContent;
        if let Some(rows) = &plan.text_rows {
            let mut projected = para.clone();
            projected.line_segs = rows.clone();
            projected.hwpx_axis_shift = 0;
            let composed =
                crate::renderer::composer::compose_paragraph_in_context(&projected, styles);
            let physical_rows: Vec<_> = rows
                .iter()
                .map(|row| {
                    let x = hwpunit_to_px(row.column_start, self.dpi);
                    x..x + hwpunit_to_px(row.segment_width, self.dpi)
                })
                .collect();
            self.layout_composed_paragraph_in_frame(
                tree,
                col_node,
                &composed,
                styles,
                col_area,
                col_area.y + plan.start,
                0,
                composed.lines.len(),
                section_index,
                para_index,
                None,
                true,
                false,
                0.0,
                None,
                Some(&projected),
                Some(bin_data_content),
                None,
                Some(&physical_rows),
                false,
            );
            return;
        }
        let chars: Vec<_> = para.text.chars().collect();
        for item in &plan.boxes {
            let x = col_area.x + item.x;
            let y = col_area.y + item.y;
            match &item.content {
                InlineFlowContent::Text { range, style, lang } => {
                    let text: String = chars[range.clone()].iter().collect();
                    let text_style = resolved_to_text_style(styles, *style, *lang);
                    let node = RenderNode::new(
                        tree.next_id(),
                        RenderNodeType::TextRun(TextRunNode {
                            text,
                            style: text_style,
                            char_shape_id: Some(*style),
                            para_shape_id: Some(para.para_shape_id),
                            section_index: Some(section_index),
                            para_index: Some(para_index),
                            char_start: Some(range.start),
                            cell_context: None,
                            is_para_end: range.end == chars.len(),
                            is_line_break_end: false,
                            rotation: 0.0,
                            is_vertical: false,
                            char_overlap: None,
                            border_fill_id: styles
                                .char_styles
                                .get(*style as usize)
                                .map_or(0, |s| s.border_fill_id),
                            baseline: item.baseline,
                            field_marker: FieldMarkerType::None,
                            layout_positions: None,
                            display_text: None,
                        }),
                        BoundingBox::new(x, y, item.width, item.height),
                    );
                    col_node.children.push(node);
                }
                InlineFlowContent::Table {
                    control,
                    margin_left,
                    margin_top,
                } => {
                    let crate::model::control::Control::Table(table) = &para.controls[*control]
                    else {
                        continue;
                    };
                    let measured = measured_tables
                        .iter()
                        .find(|m| m.para_index == para_index && m.control_index == *control);
                    self.layout_table(
                        tree,
                        col_node,
                        table,
                        section_index,
                        styles,
                        0,
                        col_area,
                        y + margin_top,
                        bin_data_content,
                        measured,
                        0,
                        Some((para_index, *control)),
                        Alignment::Left,
                        None,
                        0.0,
                        0.0,
                        Some(x + margin_left),
                        None,
                        Some(col_area.y + plan.start),
                        None,
                        false,
                        false,
                        false,
                        None,
                        Self::standalone_table_char_border_fill(Some(para), table, styles),
                    );
                }
            }
        }
    }

    pub(crate) fn layout_inline_table_paragraph(
        &self,
        tree: &mut PageLayoutContext,
        col_node: &mut RenderNode,
        para: &Paragraph,
        composed: Option<&ComposedParagraph>,
        styles: &ResolvedStyleSet,
        col_area: &LayoutRect,
        y_start: f64,
        section_index: usize,
        para_index: usize,
        bin_data_content: &[BinDataContent],
        measured_tables: &[MeasuredTable],
    ) -> f64 {
        use crate::model::control::Control;

        // 1. 문단 스타일 조회
        let para_style_id = composed
            .map(|c| c.para_style_id as usize)
            .unwrap_or(para.para_shape_id as usize);
        let para_style = styles.para_styles.get(para_style_id);
        let margin_left = para_style.map(|s| s.margin_left).unwrap_or(0.0);
        let margin_right = para_style.map(|s| s.margin_right).unwrap_or(0.0);
        let spacing_before = crate::renderer::hwp3_variant_flow_spacing_before(
            para_style.map(|s| s.spacing_before).unwrap_or(0.0),
            self.use_hwp3_origin_flow_spacing_before.get(),
        );
        let spacing_after = para_style.map(|s| s.spacing_after).unwrap_or(0.0);
        let alignment = para_style.map(|s| s.alignment).unwrap_or(Alignment::Left);

        // 2. treat_as_char 표 목록과 폭 수집
        let inline_tables: Vec<(usize, &crate::model::table::Table)> = para
            .controls
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                if let Control::Table(t) = c {
                    if t.common.treat_as_char {
                        return Some((i, t.as_ref()));
                    }
                }
                None
            })
            .collect();
        let flow_anchor_y = y_start + spacing_before;
        let has_detached_para_object = inline_tables.iter().any(|(_, table)| {
            table
                .cells
                .iter()
                .flat_map(|cell| cell.paragraphs.iter())
                .flat_map(|p| p.controls.iter())
                .any(|ctrl| match ctrl {
                    Control::Picture(pic) => {
                        !pic.common.treat_as_char
                            && !pic.common.flow_with_text
                            && matches!(
                                pic.common.text_wrap,
                                crate::model::shape::TextWrap::TopAndBottom
                            )
                            && matches!(
                                pic.common.vert_rel_to,
                                crate::model::shape::VertRelTo::Para
                            )
                    }
                    Control::Shape(shape) => {
                        let common = shape.common();
                        !common.treat_as_char
                            && !common.flow_with_text
                            && matches!(
                                common.text_wrap,
                                crate::model::shape::TextWrap::TopAndBottom
                            )
                            && matches!(common.vert_rel_to, crate::model::shape::VertRelTo::Para)
                    }
                    _ => false,
                })
        });
        let inline_table_line_shift = if has_detached_para_object {
            para.line_segs
                .first()
                .filter(|seg| seg.vertical_pos > 0)
                .map(|seg| hwpunit_to_px(seg.vertical_pos, self.dpi))
                .unwrap_or(0.0)
        } else {
            0.0
        };
        let y = flow_anchor_y + inline_table_line_shift;
        let table_para_y = if inline_table_line_shift > 0.0 {
            Some(flow_anchor_y)
        } else {
            None
        };

        // [Task #517 Stage 1] RHWP_LAYOUT_DEBUG 진단 로깅
        if layout_debug_enabled() {
            eprintln!(
                "LAYOUT_INLINE_TABLE_PARA: pi={} sec={} col_x={:.1} col_w={:.1} y_start={:.1} y={:.1} sb={:.1} sa={:.1} ml={:.1} mr={:.1} align={:?} ls_count={} tables={}",
                para_index, section_index, col_area.x, col_area.width, y_start, y,
                spacing_before, spacing_after, margin_left, margin_right, alignment,
                para.line_segs.len(), inline_tables.len(),
            );
            for (li, seg) in para.line_segs.iter().enumerate() {
                eprintln!(
                    "  LAYOUT_LS[{}]: vpos={} lh={} ls={} bl={} text_start={} sw={}",
                    li,
                    seg.vertical_pos,
                    seg.line_height,
                    seg.line_spacing,
                    seg.baseline_distance,
                    seg.text_start,
                    seg.segment_width,
                );
            }
            for (ti, (ci, tbl)) in inline_tables.iter().enumerate() {
                eprintln!(
                    "  LAYOUT_INLINE_TBL[{}]: ctrl_idx={} rows={} cols={} w={} h={} vert={:?} horz={:?} wrap={:?}",
                    ti, ci, tbl.row_count, tbl.col_count,
                    tbl.common.width, tbl.common.height,
                    tbl.common.vert_align, tbl.common.horz_align, tbl.common.text_wrap,
                );
            }
        }

        // 3. char_offsets 갭 분석으로 텍스트 세그먼트 분할
        // 확장 컨트롤은 8 UTF-16 코드 유닛을 차지
        let text_chars: Vec<char> = para.text.chars().collect();
        let metric_scope = super::super::composer::supplemental_clusters::ParagraphMetricScope::new(
            &text_chars,
            styles,
        );
        let offsets = &para.char_offsets;

        // 텍스트 세그먼트 분리: 갭이 8 이상이면 컨트롤 위치
        let mut segments: Vec<(usize, usize)> = Vec::new(); // (start_char_idx, end_char_idx)

        // 선행 컨트롤 감지: 첫 텍스트 문자 앞에 컨트롤이 있으면 빈 세그먼트 추가.
        //
        // [#6601] 종전에는 `offsets[0] / 8` 로 셌다. 그 값은 **모든** 선행 컨트롤을
        // 세므로(구역정의·단정의 등 비-인라인 포함) 실제보다 크게 나오고, 그만큼 빈
        // 세그먼트를 더 앞세워 **표와 텍스트의 순서가 뒤집힌다.**
        //
        // 실측 `36331407_결재문서본문.hwpx` pi=0:
        //
        // ```text
        // controls = [구역정의(0), 단정의(0), 표(0), 표(3)]   text = "   " (3칸)
        // offsets[0] = 24 → num_leading = 3 → 빈 세그먼트 2개
        //   배치 순서  빈 · 표0 · 빈 · 표1 · 텍스트   ← 공백 45px 이 두 표 뒤로
        //   한/글      표0 · 텍스트 · 표1            ← 공백이 두 표 사이 (33.75pt)
        // ```
        //
        // 선행 개수는 **인라인 표 중 문자 위치가 0 인 것**으로 센다.
        let control_positions_for_lead = para.control_text_positions();
        let leading_inline_tables = inline_tables
            .iter()
            .filter(|(ctrl_idx, _)| {
                control_positions_for_lead
                    .get(*ctrl_idx)
                    .is_some_and(|&position| position == 0)
            })
            .count();
        for _ in 0..leading_inline_tables {
            segments.push((0, 0)); // 빈 세그먼트 → 표가 텍스트 앞에 배치됨
        }

        let mut seg_start = 0;
        for i in 1..offsets.len() {
            let prev_char_utf16_len = if text_chars[i - 1] >= '\u{10000}' {
                2u32
            } else {
                1
            };
            let gap = offsets[i] - offsets[i - 1];
            if gap > prev_char_utf16_len + 4 {
                // 갭에 컨트롤이 있음
                segments.push((seg_start, i));
                seg_start = i;
            }
        }
        segments.push((seg_start, text_chars.len()));

        // 배치 순서: segment[0], table[0], segment[1], table[1], ...
        // 선행 컨트롤이 있으면: empty_seg, table[0], text_seg, table[1], ...

        // 4. 각 요소의 폭 계산
        // 4a. 표 폭 계산
        let table_widths: Vec<f64> = inline_tables
            .iter()
            .map(|(_, t)| {
                // col_widths로부터 table_width 계산
                let col_count = t.col_count as usize;
                let cell_spacing = hwpunit_to_px(t.cell_spacing as i32, self.dpi);
                let mut col_widths = vec![0.0f64; col_count];
                for cell in &t.cells {
                    let c = cell.col as usize;
                    let span = cell.col_span.max(1) as usize;
                    if c + span <= col_count {
                        let w = hwpunit_to_px(cell.width as i32, self.dpi);
                        if span == 1 {
                            if w > col_widths[c] {
                                col_widths[c] = w;
                            }
                        }
                    }
                }
                let total: f64 = col_widths.iter().sum::<f64>()
                    + cell_spacing * (col_count.saturating_sub(1) as f64);
                total
            })
            .collect();
        // [Issue #3396] 한글은 TAC 표를 "outMargin 포함 폭의 문자"로 배치한다 —
        // 정렬/전진 폭에는 outMargin 좌/우가 포함되고, 괘선(테두리)은
        // pen + outMargin.left 에 그려진다 (오라클 실측: 156678235 p1/p5/p7).
        let table_om_px: Vec<(f64, f64)> = inline_tables
            .iter()
            .map(|(_, t)| {
                (
                    hwpunit_to_px(t.outer_margin_left as i32, self.dpi),
                    hwpunit_to_px(t.outer_margin_right as i32, self.dpi),
                )
            })
            .collect();

        // 4b. 텍스트 세그먼트 폭 계산
        let char_style_id = para
            .char_shapes
            .first()
            .map(|cs| cs.char_shape_id as u32)
            .unwrap_or(0);

        let seg_widths: Vec<f64> = segments
            .iter()
            .map(|(s, e)| {
                let seg_text: String = text_chars[*s..*e].iter().collect();
                if seg_text.is_empty() {
                    return 0.0;
                }
                // 세그먼트 내 char_shape 변경을 고려한 폭 계산
                let mut total = 0.0;
                for ch_idx in *s..*e {
                    // 해당 문자의 char_shape 찾기
                    let utf16_pos = offsets[ch_idx];
                    let cs_id = para
                        .char_shapes
                        .iter()
                        .rev()
                        .find(|cs| cs.start_pos <= utf16_pos)
                        .map(|cs| cs.char_shape_id as u32)
                        .unwrap_or(char_style_id);
                    let ch = map_pua_bullet_char(text_chars[ch_idx]);
                    let lang = super::super::style_resolver::detect_lang_category(ch);
                    let ts = metric_scope.style(styles, cs_id, lang, ch_idx);
                    total += estimate_text_width(&ch.to_string(), &ts);
                }
                total
            })
            .collect();

        // 5. 총 폭과 정렬 계산 (TAC 표는 outMargin 좌/우 포함 폭 — Issue #3396)
        // [#6601] 정렬 폭은 **선언 폭**을 우선한다 — `table_widths` 는 열별 셀 폭
        // (`col_span == 1` max 합)이라 병합 셀이 많은 표에서 과소합산된다. 같은 함수
        // 안의 줄넘김 검사(`should_wrap_middle_anchored_table`)는 선언 폭 기반
        // `table_footprint` 를 쓰므로, 둘이 갈리면 정렬이 줄 폭을 잘못 나눠 준다.
        //
        // 실측 `36331407_결재문서본문.hwpx` pi=0 (TAC 표 2개, 한글 2024 는 나란히):
        //
        // ```text
        // table_widths = [172.96, **124.95**]      선언 폭은 33920HU = 452.3px
        // total 350.3 → Center 시작 x 가 +165.0    (실제로 필요한 건 +23.6)
        // 그 165.0 때문에 341.7 + 455.9 = 797.6 > 680.3 → 둘째 표가 줄을 넘는다
        // 선언 폭을 쓰면 176.6 + 455.9 = 632.5 < 680.3 → 한 줄에 나란히
        // ```
        //
        // `#5785` 가 `is_tac_table_inline` 에 세운 계약과 같다.
        let table_declared_widths: Vec<f64> = inline_tables
            .iter()
            .zip(table_widths.iter())
            .map(|((_, t), colsum)| {
                let declared = hwpunit_to_px(t.flow_width_hu() as i32, self.dpi);
                if declared > 0.0 {
                    declared
                } else {
                    *colsum
                }
            })
            .collect();
        let total_width: f64 = seg_widths.iter().sum::<f64>()
            + table_declared_widths.iter().sum::<f64>()
            + table_om_px.iter().map(|(l, r)| l + r).sum::<f64>();
        let available_width = col_area.width - margin_left - margin_right;
        let start_x = match alignment {
            Alignment::Center | Alignment::Distribute => {
                col_area.x + margin_left + (available_width - total_width).max(0.0) / 2.0
            }
            Alignment::Right => col_area.x + margin_left + (available_width - total_width).max(0.0),
            _ => col_area.x + margin_left,
        };

        // 6. 줄 높이 계산 (line_seg 기반)
        // line_seg[0]은 표를 포함한 줄 (표 높이 반영), line_seg[1]은 텍스트 줄
        //
        // [#6078] 단, 그 순서는 **가정이 아니라 조회**여야 한다. HWP3 국세청 납세담보
        // 확인서는 반대로 저장한다 — `ls[0] lh=1300`(제목 텍스트 줄), `ls[1] lh=67616`
        // (표 줄). 0/1 을 고정하면 `￼` 자리표시 조각이 **표 줄의 baseline**(57473HU
        // =766.3px)을 텍스트 줄 높이로 받아 문단 바닥을 표 높이만큼 한 번 더 밀고,
        // 뒤 문단(용지 규격 줄)이 용지 밖(+827px)으로 나가 소실된다. 표가 실제로 속한
        // seg 는 `control_line_seg_index` 가 안다.
        //
        // 판별은 **기하**로 한다 — 표를 담을 수 있는 줄 높이를 가진 seg 가 표 줄이다.
        // (`control_line_seg_index` 는 선행 컨트롤에서 0 대신 1 을 돌려준다: 컨트롤이
        // 문자 0 이고 첫 글자 offset 이 8 이면 `p >= start_txt` 가 0 >= 0 으로 참이 된다.
        // 정책연구용역사업 중간진도보고서 pi=428 이 그 형상 — 표 h=13956 이 ls[0]
        // lh=16086 에 담기는데 seg 1(lh=1000)을 표 줄로 오인해 되레 깨진다.)
        let table_seg_index = inline_tables
            .first()
            .and_then(|(_, tbl)| {
                let need = tbl.common.height;
                para.line_segs
                    .iter()
                    .enumerate()
                    .filter(|(_, seg)| seg.line_height as u32 >= need)
                    .map(|(idx, _)| idx)
                    .next()
            })
            .unwrap_or(0);
        let text_seg_index = (0..para.line_segs.len()).find(|idx| *idx != table_seg_index);
        let table_seg = para.line_segs.get(table_seg_index);
        let text_seg = text_seg_index.and_then(|idx| para.line_segs.get(idx));

        let line_height = if let Some(ls) = table_seg {
            hwpunit_to_px(ls.line_height, self.dpi)
        } else {
            hwpunit_to_px(400, self.dpi)
        };
        let line_spacing = if let Some(ls) = table_seg {
            hwpunit_to_px(ls.line_spacing, self.dpi)
        } else {
            0.0
        };
        // 폰트 어센트 보정용: 문단 내 최대 폰트 크기
        let para_max_font_size = {
            let default_cs = para
                .char_shapes
                .first()
                .map(|cs| cs.char_shape_id as u32)
                .unwrap_or(0);
            let ts = resolved_to_text_style(styles, default_cs, 0);
            if ts.font_size > 0.0 {
                ts.font_size
            } else {
                12.0
            }
        };

        // [#7018] **런이 속한 저장 줄의 baseline 을 쓴다** — 문단 단위 판정이 아니다.
        //
        // 한/글은 자리차지 표를 담는 문단을 `textpos` 로 줄로 가른다. 표가 줄 하나를 통째로
        // 가지면(`2769535` 2쪽) 그 앞 텍스트는 **자기 줄**에 앉고, 표와 같은 줄에 얹히는
        // 진짜 인라인 형상이면 표 줄에 앉는다. 어느 쪽인지는 문단 전체에 한 번 정할 값이
        // 아니라 **런마다** 그 런이 속한 줄로 정해진다 — 여러 텍스트 줄·여러 표가 있는
        // 문단에서 첫 표의 판정이 다른 줄로 번지면 안 된다 (PR #7044 검토 지적 2).
        //
        // 축: `LineSeg.textpos` 와 `para.char_offsets` 는 같은 문단 UTF-16 축이고 **컨트롤
        // 슬롯을 포함**한다(`char_offsets[i]` = `text[i]` 의 원본 UTF-16 인덱스). 그래서
        // 글자 인덱스를 `char_offsets` 로 그 축에 올려 비교한다 — `para.text` 문자만
        // 합산하면 선행 컨트롤이 있는 문단에서 어긋난다 (검토 지적 3).
        //
        // 실측(2769535 2쪽 `  마. 행정박물류` + 자리차지 표):
        //   seg[0] textpos=0  bl=1020  (13.6px)   ← 글자 줄
        //   seg[1] textpos=11 bl=13423 (179.0px)  ← 표 줄
        // 종전에는 글자 런이 179.0px 를 받아 baseline 이 605.8+179.0=784.8 에 찍혔다.
        // 한/글 2020 오라클 잉크는 605.5..620.5 이고 seg[0] 로 계산한 619.4 와 맞는다.
        let stored_line_baseline_at = |char_idx: usize| -> Option<f64> {
            if para.line_segs.len() < 2 {
                return None;
            }
            let pos = *para.char_offsets.get(char_idx)?;
            let idx = (0..para.line_segs.len())
                .rev()
                .find(|i| para.line_seg_text_start(*i) <= pos)?;
            let seg = para.line_segs.get(idx)?;
            Some(ensure_min_baseline(
                hwpunit_to_px(seg.baseline_distance, self.dpi),
                para_max_font_size,
            ))
        };
        let baseline_dist = if let Some(ls) = table_seg {
            ensure_min_baseline(
                hwpunit_to_px(ls.baseline_distance, self.dpi),
                para_max_font_size,
            )
        } else {
            line_height * 0.8
        };
        // 텍스트 줄(표 아래) 전용 메트릭: 표 줄이 아닌 seg 가 있으면 사용
        let text_line_baseline = if let Some(ls) = text_seg {
            ensure_min_baseline(
                hwpunit_to_px(ls.baseline_distance, self.dpi),
                para_max_font_size,
            )
        } else {
            baseline_dist
        };
        let text_line_height = if let Some(ls) = text_seg {
            hwpunit_to_px(ls.line_height, self.dpi)
        } else {
            line_height
        };
        let text_line_spacing = if let Some(ls) = text_seg {
            hwpunit_to_px(ls.line_spacing, self.dpi)
        } else {
            line_spacing
        };

        // 7. 가로 배치: 텍스트 세그먼트와 표를 순차 배치
        let right_margin = col_area.x + col_area.width - margin_right;
        let line_start_x = col_area.x + margin_left;
        // 텍스트 줄바꿈 시 줄 높이: line_seg[0]은 표 높이를 포함하므로
        // line_seg[1]이 있으면 사용 (텍스트 줄 높이), 없으면 baseline_dist 기반
        let line_step = if let Some(ls) = text_seg {
            hwpunit_to_px(ls.line_height, self.dpi) + hwpunit_to_px(ls.line_spacing, self.dpi)
        } else if let Some(ls) = para.line_segs.first() {
            hwpunit_to_px(ls.line_height, self.dpi) + hwpunit_to_px(ls.line_spacing, self.dpi)
        } else {
            baseline_dist * 1.5
        };

        // [Task #518 Phase 2] LINE_SEG 기반 줄 나눔 위치 결정:
        // ls[1..] 의 text_start (raw UTF-16 위치, controls 포함) 를 char index 로 변환.
        // char_offsets[i] = text_chars[i] 의 원본 UTF-16 위치 → char_offsets[i] >= ts 인 첫 i 가 break.
        //
        // 이전: ctrl_gap 을 paragraph 전체 controls 합으로 over-subtract → controls 가 있는
        // paragraph 에서 saturating 0 으로 항상 break 미감지 (#496 케이스).
        // 이전: ls[1] 만 사용. 다중 줄 paragraph 에서 ls[2..] 무시 → dynamic reflow.
        let stored_line_breaks = inline_table_stored_line_breaks(para);
        let line_break_char_indices: Vec<usize> = stored_line_breaks
            .iter()
            .map(|&(char_idx, _)| char_idx)
            .collect();
        if layout_debug_enabled() {
            eprintln!(
                "  LAYOUT_BREAK_INDICES: pi={} indices={:?} (from ls[1..])",
                para_index, line_break_char_indices,
            );
        }

        let mut inline_x = start_x;
        let mut current_y = y;
        let mut table_idx = 0;
        let mut max_table_bottom = y; // 표의 최대 하단 y (표 높이를 줄 높이로 사용하기 위함)
        let mut wrapped_below_table = false; // 텍스트가 표 아래로 줄바꿈되었는지
                                             // [Task #518] 다음 break 인덱스 (line_break_char_indices 안에서)
        let mut next_break: usize = 0;
        let control_positions = para.control_text_positions();

        for (s, e) in &segments {
            // 텍스트 세그먼트 렌더링 (줄바꿈 지원)
            if *s < *e {
                let seg_text: String = text_chars[*s..*e].iter().collect();
                if !seg_text.is_empty() {
                    // 문자별로 처리하며 줄바꿈 판단
                    let run_start = *s;
                    let mut line_run_start = *s; // 현재 줄 run의 시작
                    let mut line_run_x = inline_x; // 현재 줄 run의 x 시작
                    let mut current_cs_id = {
                        let utf16_pos = offsets[*s];
                        para.char_shapes
                            .iter()
                            .rev()
                            .find(|cs| cs.start_pos <= utf16_pos)
                            .map(|cs| cs.char_shape_id as u32)
                            .unwrap_or(char_style_id)
                    };

                    for ch_idx in *s..*e {
                        // 각주 마커 삽입: 현재 문자 위치에 각주가 있으면 먼저 run flush + FootnoteMarker 노드 삽입
                        if let Some(&(_, fn_num, fn_ctrl_idx)) = composed.and_then(|c| {
                            c.footnote_positions
                                .iter()
                                .find(|&&(pos, _, _)| pos == ch_idx)
                        }) {
                            // 현재까지 누적된 run 출력
                            if ch_idx > line_run_start {
                                let run_text: String =
                                    text_chars[line_run_start..ch_idx].iter().collect();
                                let first_lang = super::super::style_resolver::detect_lang_category(
                                    text_chars[line_run_start],
                                );
                                let run_ts = metric_scope.style(
                                    styles,
                                    current_cs_id,
                                    first_lang,
                                    line_run_start,
                                );
                                let run_width = estimate_text_width(&run_text, &run_ts);
                                let run_bbox_h = stored_line_baseline_at(line_run_start).unwrap_or(
                                    if wrapped_below_table {
                                        text_line_baseline
                                    } else {
                                        baseline_dist
                                    },
                                );
                                let run_id = tree.next_id();
                                let run_node = RenderNode::new(
                                    run_id,
                                    RenderNodeType::TextRun(TextRunNode {
                                        text: run_text,
                                        style: run_ts,
                                        char_shape_id: Some(current_cs_id),
                                        para_shape_id: Some(para_style_id as u16),
                                        section_index: Some(section_index),
                                        para_index: Some(para_index),
                                        char_start: Some(line_run_start),
                                        cell_context: None,
                                        is_para_end: false,
                                        is_line_break_end: false,
                                        rotation: 0.0,
                                        is_vertical: false,
                                        char_overlap: None,
                                        border_fill_id: styles
                                            .char_styles
                                            .get(current_cs_id as usize)
                                            .map(|cs| cs.border_fill_id)
                                            .unwrap_or(0),
                                        baseline: run_bbox_h,
                                        field_marker: FieldMarkerType::None,
                                        layout_positions: None,
                                        display_text: None,
                                    }),
                                    BoundingBox::new(line_run_x, current_y, run_width, run_bbox_h),
                                );
                                col_node.children.push(run_node);
                                inline_x += run_width;
                                line_run_x = inline_x;
                                line_run_start = ch_idx;
                            }
                            // FootnoteMarker 노드 삽입 (위첨자로 렌더링됨)
                            let fn_text = note_marker_text_from_control(
                                para.controls.get(fn_ctrl_idx),
                                fn_num,
                            );
                            let base_ts = resolved_to_text_style(styles, current_cs_id, 0);
                            let sup_font_size = (base_ts.font_size * 0.55).max(7.0);
                            let sup_ts = TextStyle {
                                font_size: sup_font_size,
                                font_family: base_ts.font_family.clone(),
                                ..Default::default()
                            };
                            let sup_w = estimate_text_width(&fn_text, &sup_ts);
                            let run_bbox_h =
                                stored_line_baseline_at(ch_idx).unwrap_or(if wrapped_below_table {
                                    text_line_baseline
                                } else {
                                    baseline_dist
                                });
                            let marker_id = tree.next_id();
                            let marker_node = RenderNode::new(
                                marker_id,
                                RenderNodeType::FootnoteMarker(FootnoteMarkerNode {
                                    number: fn_num,
                                    text: fn_text,
                                    base_font_size: base_ts.font_size,
                                    font_family: base_ts.font_family.clone(),
                                    color: base_ts.color,
                                    section_index,
                                    para_index,
                                    control_index: fn_ctrl_idx,
                                }),
                                BoundingBox::new(inline_x, current_y, sup_w, run_bbox_h),
                            );
                            col_node.children.push(marker_node);
                            inline_x += sup_w;
                            line_run_x = inline_x;
                        }

                        let utf16_pos = offsets[ch_idx];
                        let cs_id = para
                            .char_shapes
                            .iter()
                            .rev()
                            .find(|cs| cs.start_pos <= utf16_pos)
                            .map(|cs| cs.char_shape_id as u32)
                            .unwrap_or(char_style_id);

                        let ch = text_chars[ch_idx];
                        let lang = super::super::style_resolver::detect_lang_category(ch);
                        let ts = metric_scope.style(styles, cs_id, lang, ch_idx);
                        let ch_w = estimate_text_width(&ch.to_string(), &ts);

                        // char_shape 변경 또는 줄바꿈 시 누적된 run을 출력
                        // [Task #518] LINE_SEG 기반 줄 나눔: ls[1..] 의 text_start 위치 모두 사용.
                        // break 가 모두 소진되거나 미존재 시 right_margin 동적 reflow 로 fallback.
                        // [#6181] 저장 나눔으로 넘어간 줄은 그 줄을 소유한 `line_segs`
                        // 인덱스를 함께 들고 간다 — 아래에서 줄 상단을 저장 `vertical_pos`
                        // 로 잡는 데 쓴다.
                        let mut stored_break_seg: Option<usize> = None;
                        let need_wrap = if next_break < line_break_char_indices.len()
                            && ch_idx >= line_break_char_indices[next_break]
                        {
                            stored_break_seg = stored_line_breaks.get(next_break).map(|&(_, s)| s);
                            next_break += 1;
                            true
                        } else if next_break < line_break_char_indices.len() {
                            // [#6180] 아직 도달하지 않은 저장 나눔이 남아 있으면 그것이 이
                            // 줄의 권위다 — 측정 폭이 한 글자 일찍 넘쳐도 접지 않는다.
                            //
                            // 156745974 7쪽 pi=94: 저장 나눔은 35 인데 rhwp 측정 폭이 34 에서
                            // 넘쳐 `안전` 의 `전` 하나가 제 줄로 떨어졌다(2줄이어야 할 문단이
                            // 3줄). 한/글은 `… 협력업체의 안전` 까지 한 줄에 담는다.
                            //
                            // 저장 나눔을 다 쓴 뒤(재래핑 구간)에는 종전대로 폭으로 접는다.
                            false
                        } else {
                            inline_x + ch_w > right_margin + 0.5 && inline_x > line_start_x + 1.0
                        };
                        let cs_changed = cs_id != current_cs_id
                            || metric_scope.allows(ch_idx) != metric_scope.allows(line_run_start);

                        // 줄바꿈된 텍스트의 BoundingBox 높이: 표 줄 vs 텍스트 줄
                        let run_bbox_h = stored_line_baseline_at(line_run_start).unwrap_or(
                            if wrapped_below_table {
                                text_line_baseline
                            } else {
                                baseline_dist
                            },
                        );

                        if (cs_changed || need_wrap) && ch_idx > line_run_start {
                            // 누적된 run 출력
                            let run_text: String =
                                text_chars[line_run_start..ch_idx].iter().collect();
                            let first_lang = super::super::style_resolver::detect_lang_category(
                                text_chars[line_run_start],
                            );
                            let run_ts = metric_scope.style(
                                styles,
                                current_cs_id,
                                first_lang,
                                line_run_start,
                            );
                            let run_width = estimate_text_width(&run_text, &run_ts);

                            let run_id = tree.next_id();
                            let run_node = RenderNode::new(
                                run_id,
                                RenderNodeType::TextRun(TextRunNode {
                                    text: run_text,
                                    style: run_ts,
                                    char_shape_id: Some(current_cs_id),
                                    para_shape_id: Some(para_style_id as u16),
                                    section_index: Some(section_index),
                                    para_index: Some(para_index),
                                    char_start: Some(line_run_start),
                                    cell_context: None,
                                    is_para_end: false,
                                    is_line_break_end: false,
                                    rotation: 0.0,
                                    is_vertical: false,
                                    char_overlap: None,
                                    border_fill_id: styles
                                        .char_styles
                                        .get(current_cs_id as usize)
                                        .map(|cs| cs.border_fill_id)
                                        .unwrap_or(0),
                                    baseline: run_bbox_h,
                                    field_marker: FieldMarkerType::None,
                                    layout_positions: None,
                                    display_text: None,
                                }),
                                BoundingBox::new(line_run_x, current_y, run_width, run_bbox_h),
                            );
                            col_node.children.push(run_node);
                            line_run_start = ch_idx;
                            line_run_x = inline_x;
                        }

                        if need_wrap {
                            // [#6181] 저장 사다리가 이 줄의 상단을 직접 말하면 그것이
                            // 정답지다 — 표 하단 추정보다 우선한다. 표가 줄 **안**에
                            // 들어가는 형상(`신규` 배지 같은 1×1 TAC 표)에서는 표 하단에
                            // 붙이면 앞 줄 `vertsize` 초과분과 `spacing` 을 함께 버려
                            // 둘째 줄이 첫 줄에 바짝 붙는다(156562368 인쇄 4~9쪽 25줄:
                            // 36.72px 이어야 할 전진이 20.00px).
                            let stored_top = stored_break_seg.and_then(|seg| {
                                inline_table_stored_line_top_offset_px(para, seg, self.dpi)
                            });
                            // 줄바꿈: 표 아래로 넘어가는 경우 표 하단 기준 배치
                            if let Some(offset) = stored_top {
                                current_y = y + offset;
                                wrapped_below_table = true;
                            } else if !wrapped_below_table && max_table_bottom > y {
                                // 첫 번째 줄바꿈 시 표 아래로 이동
                                // HWP: 표 너비로 인한 텍스트 오버플로우에는 줄간격 미적용
                                // (텍스트만의 오버플로우에는 줄간격 적용)
                                current_y = max_table_bottom;
                                wrapped_below_table = true;
                            } else {
                                current_y += line_step;
                            }
                            inline_x = line_start_x;
                            line_run_x = inline_x;
                        }

                        current_cs_id = cs_id;
                        inline_x += ch_w;
                    }

                    // 남은 run의 BoundingBox 높이
                    // [#7044 검토 지적 1] 마지막 run 도 같은 줄 소속 규칙을 쓴다. 이 문단은
                    // 제목(`charPrIDRef=16`)과 후행 공백·표(`=17`)로 나뉘어, 제목은 스타일
                    // 변경 시 중간 flush 로 위 경로를 타지만 후행 공백은 이 경로로 나온다.
                    let remaining_bbox_h =
                        stored_line_baseline_at(line_run_start).unwrap_or(if wrapped_below_table {
                            text_line_baseline
                        } else {
                            baseline_dist
                        });

                    // 남은 run 출력
                    if line_run_start < *e {
                        let run_text: String = text_chars[line_run_start..*e].iter().collect();
                        let first_lang = super::super::style_resolver::detect_lang_category(
                            text_chars[line_run_start],
                        );
                        let run_ts =
                            metric_scope.style(styles, current_cs_id, first_lang, line_run_start);
                        let run_width = estimate_text_width(&run_text, &run_ts);

                        let run_id = tree.next_id();
                        let run_node = RenderNode::new(
                            run_id,
                            RenderNodeType::TextRun(TextRunNode {
                                text: run_text,
                                style: run_ts,
                                char_shape_id: Some(current_cs_id),
                                para_shape_id: Some(para_style_id as u16),
                                section_index: Some(section_index),
                                para_index: Some(para_index),
                                char_start: Some(line_run_start),
                                cell_context: None,
                                is_para_end: false,
                                is_line_break_end: false,
                                rotation: 0.0,
                                is_vertical: false,
                                char_overlap: None,
                                border_fill_id: styles
                                    .char_styles
                                    .get(current_cs_id as usize)
                                    .map(|cs| cs.border_fill_id)
                                    .unwrap_or(0),
                                baseline: remaining_bbox_h,
                                field_marker: FieldMarkerType::None,
                                layout_positions: None,
                                display_text: None,
                            }),
                            BoundingBox::new(line_run_x, current_y, run_width, remaining_bbox_h),
                        );
                        col_node.children.push(run_node);
                    }
                }
            }

            // 텍스트 세그먼트 뒤의 표 배치
            // 표 하단 = 베이스라인 + outer_margin_bottom
            if table_idx < inline_tables.len() {
                let (ctrl_idx, tbl) = &inline_tables[table_idx];
                let mt = measured_tables
                    .iter()
                    .find(|mt| mt.para_index == para_index && mt.control_index == *ctrl_idx);
                let tw = table_widths[table_idx];
                let tbl_h = mt
                    .map(|m| m.total_height)
                    .unwrap_or_else(|| hwpunit_to_px(tbl.common.height as i32, self.dpi));
                let table_footprint = tw.max(
                    hwpunit_to_px(tbl.common.width as i32, self.dpi)
                        + hwpunit_to_px(
                            tbl.outer_margin_left as i32 + tbl.outer_margin_right as i32,
                            self.dpi,
                        ),
                );
                // [#7312] 저장 밴드가 `om_top + 선언높이 + om_bottom` 로 **이 표 하나를**
                // 담고 있다고 증명하면(`tac_stored_band_is_outer_box`) 중간앵커 줄바꿈을
                // 적용하지 않는다. 그 술어는 바로 아래 `tac_table_stored_outer_band_top` 의
                // 게이트이기도 하고, `#5729` 가 "참이면 한글은 표 상단을 줄 상단 + om_top 에
                // 앉힌다" 로 계약을 세운 자리다. 곧 **저장 사다리가 "이 표가 이 줄을 통째로
                // 차지한다" 고 말하는데** 폭 판정이 표를 다음 줄로 내려보내면 그 계약이
                // 무력화된다 — 내려간 `current_y` 를 그 함수가 그대로 받기 때문이다.
                //
                // 실측 `36494702_결재문서본문.hwpx` pi=2 (한/글 2022 정본 대조):
                //   저장 ls[0].lh 6896 == om_top 283 + 선언 6330 + om_bottom 283  (오차 0)
                //   occupied 294.00 + footprint 362.09 > line_w 642.53  → 줄바꿈 발동
                //   표 상단   종전 237.5 (= 140.4 + line_step 93.28 + om_top 3.77)
                //             수정 144.2 (= 140.4 + om_top 3.77)        정본 145.7
                //   표 좌단   종전  79.4 (줄 시작으로 되돌림)  수정 373.4  정본 373.9
                //   그 문단 뒤 본문 전체가 181.4px 내려가 있었다(`pi=11` 1050.3 → 868.9,
                //   정본 869.6).
                let table_wrapped = should_wrap_middle_anchored_table(
                    control_positions.get(*ctrl_idx).copied(),
                    text_chars.len(),
                    inline_x - line_start_x,
                    table_footprint,
                    right_margin - line_start_x,
                ) && !Self::tac_stored_band_is_outer_box(para, tbl);
                if table_wrapped {
                    current_y += line_step;
                    inline_x = line_start_x;
                }
                let (om_left, om_right) = table_om_px[table_idx];
                let om_bottom = hwpunit_to_px(tbl.outer_margin_bottom as i32, self.dpi);
                let tbl_y = self
                    .tac_table_stored_outer_band_top(para, tbl, current_y)
                    .unwrap_or_else(|| {
                        let raw = current_y + baseline_dist + om_bottom - tbl_h;
                        if raw < current_y {
                            // [#3820] 베이스라인-하단 식이 줄 상단 **위로** 올라가면
                            // (표가 줄의 baseline 여유보다 크다) 그 모델은 성립하지
                            // 않는다. 종전 `.max(current_y)` 는 그때 선언된 위 바깥
                            // 여백을 조용히 버렸다. 한/글은 줄 상단 + `om_top` 에 둔다.
                            //
                            //   간장 기증자 보고서 33쪽 5행11열 (om_top=140HU)
                            //     cur_y 83.16 · base 182.31 · om_b 1.87 · tbl_h 210.75
                            //     raw 56.59 < cur_y  → 종전 83.16 / 한/글 괘선 85.03
                            //     cur_y + om_top = 85.03  (정확 일치)
                            current_y + hwpunit_to_px(tbl.outer_margin_top as i32, self.dpi)
                        } else {
                            raw
                        }
                    });

                let table_bottom = self.layout_table(
                    tree,
                    col_node,
                    tbl,
                    section_index,
                    styles,
                    0,
                    col_area,
                    tbl_y,
                    bin_data_content,
                    mt,
                    0,
                    Some((para_index, *ctrl_idx)),
                    Alignment::Left,
                    None,
                    0.0,
                    0.0,
                    Some(inline_x + om_left),
                    None,
                    table_para_y,
                    None,
                    false,
                    false,
                    false,
                    None,
                    Self::standalone_table_char_border_fill(Some(para), tbl, styles),
                );
                if table_bottom > max_table_bottom {
                    max_table_bottom = table_bottom;
                }

                if table_wrapped {
                    current_y = table_bottom;
                    inline_x = line_start_x;
                    wrapped_below_table = true;
                } else {
                    inline_x += tw + om_left + om_right;
                }
                table_idx += 1;
            }
        }

        // 후행 표 (텍스트 세그먼트보다 표가 더 많은 경우)
        while table_idx < inline_tables.len() {
            let (ctrl_idx, tbl) = &inline_tables[table_idx];
            let mt = measured_tables
                .iter()
                .find(|mt| mt.para_index == para_index && mt.control_index == *ctrl_idx);
            let tw = table_widths[table_idx];
            let (om_left, om_right) = table_om_px[table_idx];
            let tbl_h = mt
                .map(|m| m.total_height)
                .unwrap_or_else(|| hwpunit_to_px(tbl.common.height as i32, self.dpi));
            let om_bottom = hwpunit_to_px(tbl.outer_margin_bottom as i32, self.dpi);
            let tbl_y = self
                .tac_table_stored_outer_band_top(para, tbl, current_y)
                .unwrap_or_else(|| (current_y + baseline_dist + om_bottom - tbl_h).max(current_y));

            let table_bottom = self.layout_table(
                tree,
                col_node,
                tbl,
                section_index,
                styles,
                0,
                col_area,
                tbl_y,
                bin_data_content,
                mt,
                0,
                Some((para_index, *ctrl_idx)),
                Alignment::Left,
                None,
                0.0,
                0.0,
                Some(inline_x + om_left),
                None,
                table_para_y,
                None,
                false,
                false,
                false,
                None,
                Self::standalone_table_char_border_fill(Some(para), tbl, styles),
            );
            if table_bottom > max_table_bottom {
                max_table_bottom = table_bottom;
            }

            inline_x += tw + om_left + om_right;
            table_idx += 1;
        }

        // 텍스트가 줄바꿈된 경우 텍스트 하단 고려
        // 줄바꿈된 텍스트는 텍스트 줄 높이 기준, 아니면 표 줄 높이 기준
        let text_bottom = if wrapped_below_table {
            current_y + text_line_height + line_spacing
        } else {
            current_y + line_height + line_spacing
        };
        // 표와 텍스트 중 더 큰 하단을 사용
        let effective_line_bottom = max_table_bottom
            .max(text_bottom)
            .max(y + line_height + line_spacing);
        effective_line_bottom + spacing_after
    }

    /// 문단 전체를 레이아웃하여 단 노드에 추가
    pub(crate) fn layout_paragraph(
        &self,
        tree: &mut PageLayoutContext,
        col_node: &mut RenderNode,
        para: &Paragraph,
        composed: Option<&ComposedParagraph>,
        styles: &ResolvedStyleSet,
        col_area: &LayoutRect,
        y_start: f64,
        section_index: usize,
        para_index: usize,
        multi_col_width_hu: Option<i32>,
        bin_data_content: Option<&[BinDataContent]>,
        wrap_anchor: Option<&crate::renderer::pagination::WrapAnchorRef>,
    ) -> f64 {
        let end_line = composed
            .map(|c| c.lines.len())
            .unwrap_or(para.line_segs.len());
        self.layout_partial_paragraph(
            tree,
            col_node,
            para,
            composed,
            styles,
            styles.hwp3_variant && self.endnote_para_source_for(para_index).is_none(),
            col_area,
            y_start,
            0,
            end_line,
            section_index,
            para_index,
            multi_col_width_hu,
            bin_data_content,
            wrap_anchor,
        )
    }

    /// 저장 줄이 없는 본문의 exclusion 검사도 paint와 같은 frame의 첫 줄을 쓴다.
    /// 0 높이 probe는 줄 시작이 표 위에 있다는 이유로 실제 잉크 겹침을 놓친다.
    /// 개체를 가진 문단은 해당 개체의 흐름 owner에 남긴다.
    pub(crate) fn computed_plain_text_probe_height(
        &self,
        para: &Paragraph,
        composed: Option<&ComposedParagraph>,
        styles: &ResolvedStyleSet,
        column_width: f64,
        line_index: usize,
        known_square_band: bool,
    ) -> Option<f64> {
        if !para.line_segs.is_empty() || !para.controls.is_empty() || para.text.trim().is_empty() {
            return None;
        }
        let comp = composed?;
        let style = styles.para_styles.get(comp.para_style_id as usize);
        let inner = column_width - style.map_or(0.0, |s| s.margin_left + s.margin_right);
        let frame =
            crate::renderer::composer::ParagraphBox::body_for_style(column_width, style, self.dpi);
        let current =
            crate::renderer::composer::recompose_stored_lines_in_frame_with_known_square_band(
                comp,
                para,
                frame,
                inner,
                styles,
                self.dpi,
                self.profile.get().legacy_hwp3_stored_geometry(),
                crate::renderer::composer::StoredRowMissPolicy::Reflow,
                &self.body_float_carve_evidence.borrow(),
                known_square_band,
            )?;
        let line = current.lines.get(line_index)?;
        let font_size = crate::renderer::composed_line_max_font_size(line, para, styles);
        let (height, _) = crate::renderer::corrected_line_metrics(
            hwpunit_to_px(line.line_height, self.dpi),
            hwpunit_to_px(line.line_spacing, self.dpi),
            font_size,
            style.map_or(crate::model::style::LineSpacingType::Percent, |s| {
                s.line_spacing_type
            }),
            style.map_or(160.0, |s| s.line_spacing),
        );
        (height.is_finite() && height > 0.0).then_some(height)
    }

    /// 문단 일부를 레이아웃하여 단 노드에 추가
    pub(crate) fn layout_partial_paragraph(
        &self,
        tree: &mut PageLayoutContext,
        col_node: &mut RenderNode,
        para: &Paragraph,
        composed: Option<&ComposedParagraph>,
        styles: &ResolvedStyleSet,
        hwp3_body_reflow: bool,
        col_area: &LayoutRect,
        y_start: f64,
        start_line: usize,
        end_line: usize,
        section_index: usize,
        para_index: usize,
        multi_col_width_hu: Option<i32>,
        bin_data_content: Option<&[BinDataContent]>,
        wrap_anchor: Option<&crate::renderer::pagination::WrapAnchorRef>,
    ) -> f64 {
        if let Some(comp) = composed {
            // [Task #1042 Stage 6b] 본문 paragraph 의 line_segs.empty case 의 wrap 정합 —
            // compose_lines fallback (CHARS_PER_LINE=45 heuristic) 결과를 column inner width
            // 기반으로 re-split. cell paragraph (Stage 6a 의 height_measurer 호출) 와 동일
            // recompose path 사용.
            let recomposed: Option<ComposedParagraph> = {
                let para_style = styles.para_styles.get(comp.para_style_id as usize);
                let margin_l = para_style.map(|s| s.margin_left).unwrap_or(0.0);
                let margin_r = para_style.map(|s| s.margin_right).unwrap_or(0.0);
                let frame_width = if crate::renderer::para_has_no_stored_line_segs(para) {
                    crate::renderer::synthetic_wrap_column_width(
                        col_area.width,
                        margin_l,
                        wrap_anchor,
                        self.dpi,
                    )
                } else {
                    col_area.width
                };
                let column_inner_width = (frame_width - margin_l - margin_r).max(0.0);
                // 문단 상자는 편집 경로(`DocumentCore::reflow_paragraph`)의 가용 폭과
                // 같아야 한다 — 한 문단이 어느 경로로 왔는지에 따라 다른 폭을 갖지
                // 않게 한다(typeset 의 동일 산출과 맞춘다). 들여쓰기/내어쓰기는 이
                // 상자 **안에서** `layout_paragraph_in_frame` 의 indent_px 가 적용한다.
                // `body_for_style`, not `body` — see the note in `typeset.rs`.
                let paragraph_box = crate::renderer::composer::ParagraphBox::body_for_style(
                    frame_width,
                    para_style,
                    self.dpi,
                );
                // NO_LS 와 저장분할 both go to the frame: no stored record means
                // the rebuild case outright, and the frame's fill owns it.
                if column_inner_width > 0.0 {
                    crate::renderer::composer::recompose_stored_lines_in_frame_with_known_square_band(
                        comp,
                        para,
                        paragraph_box,
                        column_inner_width,
                        styles,
                        self.dpi,
                        self.profile.get().legacy_hwp3_stored_geometry(),
                        crate::renderer::composer::StoredRowMissPolicy::Reflow,
                        &self.body_float_carve_evidence.borrow(),
                        wrap_anchor.is_some(),
                    )
                } else {
                    None
                }
            };
            let comp_ref = recomposed.as_ref().unwrap_or(comp);
            // [#2279] 전체-문단 요청(start=0, end=원본 줄수 이상)은 재래핑 후 줄수로
            // 확장한다. 종전에는 재래핑이 줄수를 늘린 문단(45자 폴백 3줄 → 실폭 4줄,
            // 86712 pi=22)에서 원본 줄수로 클램프되어 마지막 줄이 렌더에서 소실됐다
            // (측정 4줄 fit vs 렌더 3줄 — maintainer PR #2284 리뷰 p10 픽셀 하락과
            // 정합). 분할(partial) 요청의 라인 범위는 종전 클램프 유지.
            let end_line_adjusted = if start_line == 0 && end_line >= comp.lines.len() {
                comp_ref.lines.len()
            } else {
                end_line.min(comp_ref.lines.len()).max(start_line)
            };
            return self.layout_composed_paragraph(
                tree,
                col_node,
                comp_ref,
                styles,
                col_area,
                y_start,
                start_line,
                end_line_adjusted,
                section_index,
                para_index,
                None,
                // 현재 frame에서 재조판한 줄에 이전 저장 vpos를 다시 적용하지 않는다.
                recomposed.is_some(),
                false,
                0.0,
                multi_col_width_hu,
                Some(para),
                bin_data_content,
                wrap_anchor,
            );
        }

        // ComposedParagraph 없는 경우 기존 방식 fallback
        self.layout_raw_paragraph(
            tree, col_node, para, col_area, y_start, start_line, end_line,
        )
    }

    /// ComposedParagraph를 사용한 레이아웃
    /// `is_last_cell_para`: 셀 내 마지막 문단이면 true (마지막 줄의 trailing line_spacing 제외)
    /// `suppress_column_top_vpos_fallback`: caller가 첫 줄 vpos를 이미 y에 반영한
    /// 경우 true. 글상자 내부 문단처럼 LINE_SEG.vertical_pos 기반으로 선배치한 뒤
    /// 다시 column-top fallback을 적용하면 y가 이중 보정된다.
    /// `multi_col_width_hu`: 다단 문서에서 현재 단 너비(HWPUNIT). Some이면 segment_width 불일치 줄 건너뜀.
    /// `para`: 원본 문단 (treat_as_char 이미지 인라인 렌더링에 사용)
    /// `bin_data_content`: 이미지 데이터 (treat_as_char 이미지 인라인 렌더링에 사용)
    /// [Task #2067] run 루프 종료 후, run 범위 밖(pos >= run_char_pos)의 미매칭
    /// TAC 이미지 배치. 갱신된 x 를 반환한다.
    /// HWP `treat_as_char` 그림도 일반 개체와 같이 size criterion을 해석한다.
    /// HWP5의 `PAPER`/`PAGE` 값은 HWPUNIT가 아니라 기준 영역의 1/100 % 단위다.
    /// 이 경로에서 원시 `common.width`를 HWPUNIT으로 바꾸면 42520(=425.20%) 같은
    /// 그림을 42.52 mm로 축소해 렌더한다.
    pub(crate) fn resolve_inline_picture_size(
        &self,
        picture: &crate::model::image::Picture,
        col_area: &LayoutRect,
    ) -> (f64, f64) {
        let (body_x, body_y, body_w, body_h) = self.current_body_area.get();
        let body_area = if body_w > 0.0 && body_h > 0.0 {
            LayoutRect {
                x: body_x,
                y: body_y,
                width: body_w,
                height: body_h,
            }
        } else {
            *col_area
        };
        let paper_area = LayoutRect {
            x: 0.0,
            y: 0.0,
            width: self.current_paper_width.get().max(col_area.width),
            height: self.current_page_height.get().max(col_area.height),
        };

        self.resolve_object_size(&picture.common, col_area, &body_area, &paper_area)
    }

    pub(crate) fn layout_composed_paragraph(
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
    ) -> f64 {
        self.layout_composed_paragraph_in_frame(
            tree,
            col_node,
            composed,
            styles,
            col_area,
            y_start,
            start_line,
            end_line,
            section_index,
            para_index,
            cell_ctx,
            suppress_column_top_vpos_fallback,
            is_last_cell_para,
            first_line_x_offset,
            multi_col_width_hu,
            para,
            bin_data_content,
            wrap_anchor,
            None,
            false,
        )
    }

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
        self.paragraph_paint_session()
            .layout_composed_paragraph_in_frame(
                tree,
                col_node,
                composed,
                styles,
                col_area,
                y_start,
                start_line,
                end_line,
                section_index,
                para_index,
                cell_ctx,
                suppress_column_top_vpos_fallback,
                is_last_cell_para,
                first_line_x_offset,
                multi_col_width_hu,
                para,
                bin_data_content,
                wrap_anchor,
                physical_rows,
                squeeze_stored_line,
            )
    }

    /// 원본 문단 데이터로 레이아웃 (ComposedParagraph 없는 경우 fallback)
    pub(crate) fn layout_raw_paragraph(
        &self,
        tree: &mut PageLayoutContext,
        col_node: &mut RenderNode,
        para: &Paragraph,
        col_area: &LayoutRect,
        y_start: f64,
        start_line: usize,
        end_line: usize,
    ) -> f64 {
        let mut y = y_start;
        let end = end_line.min(para.line_segs.len());

        for line_idx in start_line..end {
            let line_seg = &para.line_segs[line_idx];
            let line_height = hwpunit_to_px(line_seg.line_height, self.dpi);
            let baseline = ensure_min_baseline(
                hwpunit_to_px(line_seg.baseline_distance, self.dpi),
                line_height * 0.8, // fallback: 줄 높이 기반 최소 어센트
            );

            // Task #332 Stage 4b: clamp 제거, overflow 그대로 그림 (piling 차단)
            let col_bottom = col_area.y + col_area.height;
            if self.is_body_flow_col_area(col_area) && y + line_height > col_bottom + 0.5 {
                eprintln!(
                    "LAYOUT_OVERFLOW_DRAW: line={} y={:.1} col_bottom={:.1} overflow={:.1}px (fast path)",
                    line_idx, y + line_height, col_bottom, y + line_height - col_bottom,
                );
            }
            let y_clamped = y;
            let line_id = tree.next_id();
            let mut line_node = RenderNode::new(
                line_id,
                RenderNodeType::TextLine(TextLineNode::new(line_height, baseline)),
                BoundingBox::new(col_area.x, y_clamped, col_area.width, line_height),
            );

            if !para.text.is_empty() && line_idx == start_line {
                let run_id = tree.next_id();
                let run_node = RenderNode::new(
                    run_id,
                    RenderNodeType::TextRun(TextRunNode {
                        text: para.text.clone(),
                        style: TextStyle::default(),
                        char_shape_id: None,
                        para_shape_id: None,
                        section_index: None,
                        para_index: None,
                        char_start: None,
                        cell_context: None,
                        is_para_end: line_idx == end - 1,
                        is_line_break_end: false,
                        rotation: 0.0,
                        is_vertical: false,
                        char_overlap: None,
                        border_fill_id: 0,
                        baseline: line_height * 0.85,
                        field_marker: FieldMarkerType::None,
                        layout_positions: None,
                        display_text: None,
                    }),
                    BoundingBox::new(col_area.x, y_clamped, col_area.width, line_height),
                );
                line_node.children.push(run_node);
            }

            col_node.children.push(line_node);
            // 줄간격 적용: line_height에 line_spacing 추가
            let line_spacing_px = hwpunit_to_px(line_seg.line_spacing, self.dpi);
            y += line_height + line_spacing_px;
        }

        if para.line_segs.is_empty() {
            let default_height = hwpunit_to_px(400, self.dpi);
            let line_id = tree.next_id();
            let mut line_node = RenderNode::new(
                line_id,
                RenderNodeType::TextLine(TextLineNode::new(default_height, default_height * 0.8)),
                BoundingBox::new(col_area.x, y, col_area.width, default_height),
            );

            if !para.text.is_empty() {
                let run_id = tree.next_id();
                let run_node = RenderNode::new(
                    run_id,
                    RenderNodeType::TextRun(TextRunNode {
                        text: para.text.clone(),
                        style: TextStyle::default(),
                        char_shape_id: None,
                        para_shape_id: None,
                        section_index: None,
                        para_index: None,
                        char_start: None,
                        cell_context: None,
                        is_para_end: true,
                        is_line_break_end: false,
                        rotation: 0.0,
                        is_vertical: false,
                        char_overlap: None,
                        border_fill_id: 0,
                        baseline: default_height * 0.8,
                        field_marker: FieldMarkerType::None,
                        layout_positions: None,
                        display_text: None,
                    }),
                    BoundingBox::new(col_area.x, y, col_area.width, default_height),
                );
                line_node.children.push(run_node);
            }

            col_node.children.push(line_node);
            y += default_height;
        }

        y
    }

    pub(crate) fn apply_paragraph_numbering(
        &self,
        composed: Option<&ComposedParagraph>,
        para: &Paragraph,
        styles: &ResolvedStyleSet,
        outline_numbering_id: u16,
    ) -> Option<ComposedParagraph> {
        let para_style = styles.para_styles.get(para.para_shape_id as usize)?;

        let head_text = match para_style.head_type {
            HeadType::None => return None,
            HeadType::Outline | HeadType::Number => {
                let numbering_id = resolve_numbering_id(
                    para_style.head_type,
                    para_style.numbering_id,
                    outline_numbering_id,
                );
                let level = para_style.para_level;
                // [#3307] 개요 문단이 유효한 정의에 도달하지 못하면 한컴 내장
                // 기본 모양(전 수준 ^N)으로 fallback 한다. NUMBER 는 불변 —
                // 정의 없는 NUMBER 는 종전대로 번호를 그리지 않는다.
                let synthesized_default;
                let numbering = match numbering_id
                    .checked_sub(1)
                    .and_then(|i| styles.numberings.get(i as usize))
                {
                    Some(n) => n,
                    None if para_style.head_type == HeadType::Outline => {
                        synthesized_default =
                            crate::renderer::layout::utils::default_outline_numbering();
                        &synthesized_default
                    }
                    None => return None,
                };

                let counters = self.numbering_state.borrow_mut().advance(
                    numbering_id,
                    level,
                    para.numbering_restart,
                );
                let start_numbers = numbering.level_start_numbers;

                let level_idx = (level as usize).min(6);
                let format_str = &numbering.level_formats[level_idx];
                if format_str.is_empty() {
                    return None;
                }

                let text = expand_numbering_format(
                    format_str,
                    &counters,
                    numbering,
                    &start_numbers,
                    level_idx,
                );
                if text.is_empty() {
                    return None;
                }
                let has_distance = numbering
                    .heads
                    .get(level_idx)
                    .map(|h| h.text_distance > 0)
                    .unwrap_or(false);
                if has_distance {
                    format!("{} ", text)
                } else {
                    text
                }
            }
            HeadType::Bullet => {
                // Bullet: numbering_id(1-based)로 Bullet 참조
                let bullet_id = para_style.numbering_id;
                if bullet_id == 0 {
                    return None;
                }
                let bullet = styles.bullets.get((bullet_id - 1) as usize)?;
                // U+FFFF는 이미지 글머리표 표시자 — 문자 렌더링 불가, 건너뜀
                if bullet.bullet_char == '\u{FFFF}' {
                    return None;
                }
                // PUA 문자(0xF000~0xF0FF)를 표준 Unicode로 매핑
                // HWP는 Symbol 폰트 문자를 PUA(0xF000+code)로 저장
                let bullet_ch = map_pua_bullet_char(bullet.bullet_char);
                // 글머리 기호 + 본문과의 거리(text_distance)에 따른 간격
                if bullet.text_distance > 0 {
                    format!("{} ", bullet_ch)
                } else {
                    format!("{}", bullet_ch)
                }
            }
        };

        // 번호 텍스트를 별도 필드에 저장 (첫 run에 prepend하지 않음)
        // 렌더링 시 별도 TextRunNode로 생성하여 char_offset에 영향을 주지 않는다.
        let comp = composed?;
        let mut modified = comp.clone();
        modified.numbering_text = Some(head_text);

        Some(modified)
    }

    /// 조합된 문단의 텍스트에 AutoNumber를 적용한다.
    pub(crate) fn apply_auto_numbers_to_composed(
        &self,
        composed: &mut ComposedParagraph,
        para: &Paragraph,
        _counter: &mut super::AutoNumberCounter, // 더 이상 사용하지 않음 (파싱 시 할당됨)
    ) {
        // AutoNumber 컨트롤이 있는지 확인
        for ctrl in &para.controls {
            if let Control::AutoNumber(an) = ctrl {
                // 파싱 시점에 할당된 번호를 번호 형식에 맞게 변환 + 장식 문자 적용
                let num_fmt = NumFmt::from_hwp_format(an.format);
                let num_str = format_number(an.assigned_number, num_fmt);
                let num_str = if an.prefix_char != '\0' || an.suffix_char != '\0' {
                    format!(
                        "{}{}{}",
                        if an.prefix_char != '\0' {
                            an.prefix_char.to_string()
                        } else {
                            String::new()
                        },
                        num_str,
                        if an.suffix_char != '\0' {
                            an.suffix_char.to_string()
                        } else {
                            String::new()
                        },
                    )
                } else {
                    num_str
                };

                // 각 줄의 텍스트에서 AutoNumber 위치를 찾아 번호로 대체
                // HWP5/HWPX/HWP3 공통: 공백 두 개("  ") 패턴 탐색
                for line in &mut composed.lines {
                    for run in &mut line.runs {
                        if let Some(pos) = run.text.find("  ") {
                            run.text = format!(
                                "{}{}{}",
                                &run.text[..pos + 1],
                                num_str,
                                &run.text[pos + 1..]
                            );
                            return;
                        }
                    }
                }
            }
        }
    }
}

/// [Task #1151 v9 결함 D] paragraph 의 sibling TAC picture 들의 (control_idx, width_px)
/// 시퀀스 수집 (시점순). layout_shape_item 의 가로 분배 cursor / alignment 계산용.
///
/// 한컴 native 정합: 동일 paragraph 안 sibling tac=true picture 들이 가로로 inline
/// 분배 (inline glyph 처럼). 첫 picture 시점에 전체 시퀀스 폭을 알아야 alignment
/// (center / right) 의 시작 x 가 정확히 계산되므로 pre-scan helper 가 필요.
pub(crate) fn collect_sibling_tac_picture_widths_px(
    controls: &[crate::model::control::Control],
    dpi: f64,
) -> Vec<(usize, f64)> {
    use crate::model::control::Control;
    controls
        .iter()
        .enumerate()
        .filter_map(|(ci, c)| match c {
            Control::Picture(p) if p.common.treat_as_char => {
                Some((ci, hwpunit_to_px(p.common.width as i32, dpi)))
            }
            _ => None,
        })
        .collect()
}

/// [Task #1151 v9 결함 D] paragraph 단위 inline picture 의 가로 분배 cursor 상태.
/// layout_shape_item 이 같은 paragraph 의 sibling TAC picture 들을 순서대로 처리할 때
/// HashMap<para_index, ParaInlineState> 에 보관하여 가로 누적 + line wrap 처리.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ParaInlineState {
    /// 다음 picture 의 x 시작점 (paper-relative px)
    pub cursor_x: f64,
    /// 현재 line 의 y (= first picture 의 pic_y, 가로 분배 시 유지)
    pub line_top_y: f64,
    /// 현재 line 의 최대 picture height (line wrap 임계 + 다음 line advance 용)
    pub line_height: f64,
}

#[cfg(test)]
mod issue_4370_tac_table_wrap_tests {
    use super::should_wrap_middle_anchored_table;

    /// [#4370] 끝 앵커(텍스트 마지막 문자 뒤) tac 표도 남은 폭 초과 시 wrap 된다.
    #[test]
    fn end_anchored_table_wraps_when_exceeding_line_width() {
        assert!(should_wrap_middle_anchored_table(
            Some(25),
            25,
            300.0,
            480.0,
            567.0
        ));
    }

    #[test]
    fn end_anchored_table_stays_inline_when_it_fits() {
        assert!(!should_wrap_middle_anchored_table(
            Some(25),
            25,
            300.0,
            200.0,
            567.0
        ));
    }

    /// 문단 선두 앵커(position == 0)는 점유 폭이 없으므로 wrap 하지 않는다.
    #[test]
    fn leading_anchor_never_wraps() {
        assert!(!should_wrap_middle_anchored_table(
            Some(0),
            25,
            0.0,
            480.0,
            567.0
        ));
    }

    #[test]
    fn middle_anchor_wrap_preserved() {
        assert!(should_wrap_middle_anchored_table(
            Some(10),
            20,
            120.0,
            480.0,
            567.0
        ));
    }
}

#[cfg(test)]
mod issue_2809_split_alignment_tests {
    use super::{compute_line_extra_spacing, needs_word_distribution};
    use crate::model::style::Alignment;
    use crate::renderer::composer::{ComposedLine, ComposedTextRun};
    use crate::renderer::layout::text_measurement::{estimate_text_width, resolved_to_text_style};
    use crate::renderer::style_resolver::{ResolvedCharStyle, ResolvedStyleSet};

    fn split_label_line() -> ComposedLine {
        ComposedLine {
            runs: vec![ComposedTextRun {
                text: "다 같 이".to_string(),
                ..Default::default()
            }],
            line_height: 1120,
            baseline_distance: 952,
            segment_width: 6972,
            column_start: 0,
            line_spacing: 560,
            has_line_break: false,
            char_start: 0,
        }
    }

    #[test]
    fn split_distributes_single_last_line_but_justify_does_not() {
        assert!(needs_word_distribution(Alignment::Split, true, false));
        assert!(!needs_word_distribution(Alignment::Justify, true, false));
        assert!(needs_word_distribution(Alignment::Justify, false, false));
        assert!(!needs_word_distribution(Alignment::Justify, false, true));
        assert!(!needs_word_distribution(Alignment::Split, true, true));
    }

    #[test]
    fn split_label_assigns_positive_slack_to_interior_spaces() {
        let line = split_label_line();
        let (extra_word, extra_char, extra_dash) = compute_line_extra_spacing(
            &line,
            usize::MAX,
            &ResolvedStyleSet::default(),
            Alignment::Split,
            true,
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            5,
            30.0,
            90.0,
            40.0,
        );

        assert!((extra_word - 30.0).abs() < 0.001);
        assert_eq!(extra_char, 0.0);
        assert_eq!(extra_dash, 0.0);

        // A synthetic soft-wrap keeps its consumed separator in the projected
        // run. The renderer advances that final space too, so it participates
        // in the slot count and the complete run stays inside the line box.
        let trailing_line = ComposedLine {
            runs: vec![ComposedTextRun {
                text: "다 같 이 ".to_string(),
                ..Default::default()
            }],
            ..line
        };
        let styles = ResolvedStyleSet::default();
        let text_style = resolved_to_text_style(&styles, 0, 0);
        let natural_width = estimate_text_width("다 같 이 ", &text_style);
        let available_width = natural_width + 60.0;
        let (extra_word, extra_char, extra_dash) = compute_line_extra_spacing(
            &trailing_line,
            usize::MAX,
            &styles,
            Alignment::Justify,
            false,
            true,
            false,
            false,
            false,
            false,
            true,
            false,
            false,
            6,
            natural_width,
            available_width,
            40.0,
        );
        let mut distributed_style = text_style;
        distributed_style.extra_word_spacing = extra_word;
        assert!(
            (estimate_text_width("다 같 이 ", &distributed_style) - available_width).abs() < 0.001
        );
        assert_eq!(extra_char, 0.0);
        assert_eq!(extra_dash, 0.0);
    }

    #[test]
    fn split_reserves_last_glyph_ink_when_letter_spacing_is_negative() {
        let line = split_label_line();
        let styles = ResolvedStyleSet {
            char_styles: vec![ResolvedCharStyle {
                font_size: 12.0,
                letter_spacing: -6.0,
                ..Default::default()
            }],
            ..Default::default()
        };
        let text_style = resolved_to_text_style(&styles, 0, 0);
        let total_text_width = estimate_text_width("다 같 이", &text_style);
        let (extra_word, extra_char, extra_dash) = compute_line_extra_spacing(
            &line,
            usize::MAX,
            &styles,
            Alignment::Split,
            true,
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            5,
            total_text_width,
            90.0,
            40.0,
        );

        let mut distributed_style = text_style.clone();
        distributed_style.extra_word_spacing = extra_word;
        let advance = estimate_text_width("다 같 이", &distributed_style);
        let mut ink_style = text_style;
        ink_style.letter_spacing = 0.0;
        let trailing_ink_overhang =
            estimate_text_width("이", &ink_style) - estimate_text_width("이", &distributed_style);

        assert!((advance + trailing_ink_overhang - 90.0).abs() < 0.001);
        assert_eq!(extra_char, 0.0);
        assert_eq!(extra_dash, 0.0);
    }

    /// [#4516] 머리말/꼬리말 예외로만 justify 된 마지막 줄: 공백 없는 영문
    /// 문서번호에 양수 slack 을 자간으로 살포하지 않는다 (자연 폭 유지).
    #[test]
    fn footer_last_line_justify_without_spaces_keeps_natural_width() {
        let line = ComposedLine {
            runs: vec![ComposedTextRun {
                text: "RVT-QI-02-03".to_string(),
                ..Default::default()
            }],
            line_height: 1120,
            baseline_distance: 952,
            segment_width: 48188,
            column_start: 0,
            line_spacing: 560,
            has_line_break: false,
            char_start: 0,
        };
        // 꼬리말 마지막 줄 예외 (justify_spaces_only = true): 분배 없음
        let (extra_word, extra_char, extra_dash) = compute_line_extra_spacing(
            &line,
            usize::MAX,
            &ResolvedStyleSet::default(),
            Alignment::Justify,
            false,
            true,
            false,
            true,
            false,
            false,
            false,
            false,
            false,
            12,
            62.5,
            481.8,
            40.0,
        );
        assert_eq!(extra_word, 0.0);
        assert_eq!(extra_char, 0.0);
        assert_eq!(extra_dash, 0.0);

        // 본문 중간 줄 justify (justify_spaces_only = false): 기존 자간 분배 유지
        let (_, extra_char_mid, _) = compute_line_extra_spacing(
            &line,
            usize::MAX,
            &ResolvedStyleSet::default(),
            Alignment::Justify,
            false,
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            12,
            62.5,
            481.8,
            40.0,
        );
        assert!(extra_char_mid > 0.0);
    }
}

#[cfg(test)]
mod issue_4657_distribute_alignment_tests {
    use super::compute_line_extra_spacing;
    use crate::model::style::Alignment;
    use crate::renderer::composer::{ComposedLine, ComposedTextRun};
    use crate::renderer::layout::text_measurement::{estimate_text_width, resolved_to_text_style};
    use crate::renderer::style_resolver::ResolvedStyleSet;

    fn line(text: &str) -> ComposedLine {
        ComposedLine {
            runs: vec![ComposedTextRun {
                text: text.to_string(),
                ..Default::default()
            }],
            line_height: 1120,
            baseline_distance: 952,
            segment_width: 6972,
            column_start: 0,
            line_spacing: 560,
            has_line_break: false,
            char_start: 0,
        }
    }

    fn distribute_extra(text: &str, char_count: usize, text_width: f64, avail: f64) -> f64 {
        let (extra_word, extra_char, extra_dash) = compute_line_extra_spacing(
            &line(text),
            usize::MAX,
            &ResolvedStyleSet::default(),
            Alignment::Distribute,
            false,
            false,
            false,
            false,
            true,
            false,
            false,
            false,
            false,
            char_count,
            text_width,
            avail,
            40.0,
        );
        assert_eq!(extra_word, 0.0);
        assert_eq!(extra_dash, 0.0);
        extra_char
    }

    /// 배분 정렬은 남는 폭을 글자 사이(N-1곳)에 나눠, 마지막 glyph 잉크의
    /// 오른쪽 끝(`W + (N-1)·extra`)이 줄 길이와 무관하게 문단 폭에 닿는다.
    #[test]
    fn distribute_fills_full_width_regardless_of_line_length() {
        let avail = 368.0;
        for (text, n, w) in [("문서관리번호 :", 8usize, 92.9), ("기관명 :", 5, 52.5)] {
            let extra = distribute_extra(text, n, w, avail);
            let last_ink_right = w + (n - 1) as f64 * extra;
            assert!(
                (last_ink_right - avail).abs() < 0.001,
                "{text}: 오른쪽 끝 {last_ink_right} != 문단 폭 {avail}"
            );
        }
    }

    /// 말미 공백은 배분 대상이 아니다 — 마지막 보이는 글자가 오른쪽 끝에 닿는다.
    #[test]
    fn distribute_excludes_trailing_spaces() {
        let styles = ResolvedStyleSet::default();
        let mut ts = resolved_to_text_style(&styles, 0, 0);
        ts.default_tab_width = 40.0;
        let space_w = estimate_text_width(" ", &ts);
        let avail = 368.0;
        let visible_w = 52.5;
        let extra = distribute_extra("기관명 : ", 6, visible_w + space_w, avail);
        let last_visible_ink_right = visible_w + 4.0 * extra;
        assert!(
            (last_visible_ink_right - avail).abs() < 0.001,
            "말미 공백 제외 후 오른쪽 끝 {last_visible_ink_right} != {avail}"
        );
    }

    /// 한 글자 + 말미 공백뿐인 줄은 분배하지 않는다 (0-division 가드).
    #[test]
    fn distribute_single_visible_char_keeps_natural_width() {
        let extra = distribute_extra("가  ", 3, 30.0, 368.0);
        assert_eq!(extra, 0.0);
    }
}

#[cfg(test)]
mod issue_3486_hancom_company_pua_alignment_tests {
    use super::is_hancom_company_pua_logo_line;
    use crate::model::style::Alignment;
    use crate::renderer::composer::{ComposedLine, ComposedTextRun};

    fn company_line(text: &str) -> ComposedLine {
        ComposedLine {
            runs: vec![ComposedTextRun {
                text: text.to_string(),
                ..Default::default()
            }],
            line_height: 1_000,
            baseline_distance: 850,
            segment_width: 42_520,
            column_start: 0,
            line_spacing: 500,
            has_line_break: false,
            char_start: 0,
        }
    }

    #[test]
    fn company_pua_logo_line_uses_its_space_not_internal_character_distribution() {
        let line = company_line("\u{F03EF}\u{F03F0}\u{F03F1}\u{F03F2}\u{F03F3}\u{F03F4} ");
        assert!(is_hancom_company_pua_logo_line(&line, Alignment::Split));
        assert!(
            !is_hancom_company_pua_logo_line(&line, Alignment::Left),
            "나눔 정렬이 아닌 문단에는 보정을 적용하면 안 됨",
        );
        assert!(
            !is_hancom_company_pua_logo_line(
                &company_line("\u{F03EF}\u{F03F0}\u{F03F1}\u{F03F2}\u{F03F3}\u{F03F4} 본문"),
                Alignment::Split,
            ),
            "회사명 뒤의 logo-gap 공백까지 일치할 때만 보정한다",
        );
    }
}

#[cfg(test)]
mod trailing_tac_width_tests {
    use super::tac_offsets_for_line_width;
    use crate::renderer::composer::{ComposedLine, ComposedParagraph, ComposedTextRun};

    fn line(text: &str, char_start: usize, has_line_break: bool) -> ComposedLine {
        ComposedLine {
            runs: vec![ComposedTextRun {
                text: text.to_string(),
                ..Default::default()
            }],
            line_height: 1_000,
            baseline_distance: 800,
            segment_width: 10_000,
            column_start: 0,
            line_spacing: 0,
            has_line_break,
            char_start,
        }
    }

    fn composed(lines: Vec<ComposedLine>) -> ComposedParagraph {
        ComposedParagraph {
            lines,
            para_style_id: 0,
            inline_controls: Vec::new(),
            numbering_text: None,
            tac_controls: Vec::new(),
            footnote_positions: Vec::new(),
            tab_extended: Vec::new(),
            horizontal_shaping: None,
        }
    }

    #[test]
    fn final_run_trailing_tac_is_included_in_alignment_width() {
        let comp = composed(vec![line("      ", 0, false)]);
        let offsets = tac_offsets_for_line_width(&comp, &[(6, 574.08, 0)], 0);

        assert_eq!(offsets, vec![(6, 574.08, 0)]);
    }

    #[test]
    fn next_line_leading_tac_is_not_back_attributed_to_previous_width() {
        let comp = composed(vec![line("A", 0, false), line("B", 1, false)]);
        let offsets = [(1, 55.0, 0)];

        assert!(tac_offsets_for_line_width(&comp, &offsets, 0).is_empty());
        assert_eq!(tac_offsets_for_line_width(&comp, &offsets, 1), offsets);
    }

    #[test]
    fn forced_break_trailing_tac_stays_with_emitting_line() {
        let comp = composed(vec![line("A", 0, true), line("B", 2, false)]);
        let offsets = [(1, 55.0, 0)];

        assert_eq!(tac_offsets_for_line_width(&comp, &offsets, 0), offsets);
        assert!(tac_offsets_for_line_width(&comp, &offsets, 1).is_empty());
    }
}

#[cfg(test)]
mod issue_2439_lineseg_indent_tests;

#[cfg(test)]
mod issue_1151_v3_helper_tests {
    //! Issue #1151 v3/#1459: sibling TopAndBottom 예약 높이 helper 단위 검증.
    //!
    //! 한컴 정합: wrap=TopAndBottom + tac=false 인 개체가 vertical 영역
    //! reservation 으로 합산된다. TAC 개체와 Square wrap 은 제외한다.

    use super::calc_sibling_topandbottom_reserved_hu;
    use crate::model::control::Control;
    use crate::model::image::Picture;
    use crate::model::shape::{CommonObjAttr, TextWrap};
    use crate::model::table::Table;

    fn make_table(width: u32, height: u32, wrap: TextWrap, tac: bool) -> Table {
        Table {
            common: CommonObjAttr {
                width,
                height,
                text_wrap: wrap,
                treat_as_char: tac,
                ..Default::default()
            },
            outer_margin_left: 283,
            outer_margin_right: 283,
            outer_margin_top: 283,
            outer_margin_bottom: 283,
            ..Default::default()
        }
    }

    #[test]
    fn topandbottom_table_reserved_single() {
        // scenario-a-after.hwp 의 표: 13630×12498, outer_margin (top=283, bottom=283).
        // 합산 = 12498 + 283 + 283 = 13064 HU.
        let table = make_table(13630, 12498, TextWrap::TopAndBottom, false);
        let controls = vec![Control::Table(Box::new(table))];
        assert_eq!(calc_sibling_topandbottom_reserved_hu(&controls), 13064);
    }

    #[test]
    fn topandbottom_table_reserved_none_when_no_table() {
        let controls: Vec<Control> = vec![];
        assert_eq!(calc_sibling_topandbottom_reserved_hu(&controls), 0);
    }

    #[test]
    fn topandbottom_table_reserved_excludes_tac_table() {
        let table = make_table(13630, 12498, TextWrap::TopAndBottom, true); // tac=true 제외
        let controls = vec![Control::Table(Box::new(table))];
        assert_eq!(calc_sibling_topandbottom_reserved_hu(&controls), 0);
    }

    #[test]
    fn topandbottom_table_reserved_excludes_square_wrap() {
        let table = make_table(13630, 12498, TextWrap::Square, false); // wrap=Square 제외
        let controls = vec![Control::Table(Box::new(table))];
        assert_eq!(calc_sibling_topandbottom_reserved_hu(&controls), 0);
    }

    #[test]
    fn topandbottom_reserved_includes_non_tac_picture_control() {
        let mut pic = Picture::default();
        pic.common.text_wrap = TextWrap::TopAndBottom;
        pic.common.treat_as_char = false;
        pic.common.height = 7733;
        pic.common.margin.top = 100;
        pic.common.margin.bottom = 200;
        let controls = vec![Control::Picture(Box::new(pic))];
        assert_eq!(calc_sibling_topandbottom_reserved_hu(&controls), 8033);
    }

    #[test]
    fn topandbottom_reserved_excludes_tac_picture_control() {
        let mut pic = Picture::default();
        pic.common.text_wrap = TextWrap::TopAndBottom;
        pic.common.treat_as_char = true;
        pic.common.height = 7733;
        let controls = vec![Control::Picture(Box::new(pic))];
        assert_eq!(calc_sibling_topandbottom_reserved_hu(&controls), 0);
    }

    #[test]
    fn topandbottom_table_reserved_sums_multiple_tables() {
        let t1 = make_table(13630, 10000, TextWrap::TopAndBottom, false);
        let t2 = make_table(13630, 5000, TextWrap::TopAndBottom, false);
        let controls = vec![Control::Table(Box::new(t1)), Control::Table(Box::new(t2))];
        // (10000 + 283 + 283) + (5000 + 283 + 283) = 10566 + 5566 = 16132
        assert_eq!(calc_sibling_topandbottom_reserved_hu(&controls), 16132);
    }
}

#[cfg(test)]
mod issue_1151_v9_helper_tests {
    //! [Task #1151 v9 결함 D] collect_sibling_tac_picture_widths_px helper 단위 검증.

    use super::collect_sibling_tac_picture_widths_px;
    use crate::model::control::Control;
    use crate::model::image::Picture;
    use crate::model::shape::CommonObjAttr;
    use crate::model::table::Table;

    fn make_pic(width: u32, height: u32, tac: bool) -> Picture {
        Picture {
            common: CommonObjAttr {
                width,
                height,
                treat_as_char: tac,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn empty_controls_returns_empty() {
        assert!(collect_sibling_tac_picture_widths_px(&[], 96.0).is_empty());
    }

    #[test]
    fn collects_single_tac_picture() {
        // 5670 HU @ 96 dpi = 5670 * 96 / 7200 = 75.6 px
        let controls = vec![Control::Picture(Box::new(make_pic(5670, 5670, true)))];
        let result = collect_sibling_tac_picture_widths_px(&controls, 96.0);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, 0);
        assert!((result[0].1 - 75.6).abs() < 0.01);
    }

    #[test]
    fn collects_multiple_tac_pictures_in_order() {
        let controls = vec![
            Control::Picture(Box::new(make_pic(3000, 3000, true))),
            Control::Picture(Box::new(make_pic(4500, 4500, true))),
        ];
        let result = collect_sibling_tac_picture_widths_px(&controls, 96.0);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, 0);
        assert_eq!(result[1].0, 1);
    }

    #[test]
    fn skips_non_tac_picture() {
        // tac=false 인 picture (floating) 는 가로 분배 대상 아님 — 제외.
        let controls = vec![
            Control::Picture(Box::new(make_pic(3000, 3000, false))),
            Control::Picture(Box::new(make_pic(4500, 4500, true))),
        ];
        let result = collect_sibling_tac_picture_widths_px(&controls, 96.0);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, 1); // 두 번째 (tac=true) 만
    }

    #[test]
    fn skips_table_and_other_controls() {
        // Table / Shape 는 가로 분배 대상 아님 (Picture 만).
        let controls = vec![
            Control::Table(Box::default()),
            Control::Picture(Box::new(make_pic(5670, 5670, true))),
            Control::Picture(Box::new(make_pic(5670, 5670, true))),
        ];
        let result = collect_sibling_tac_picture_widths_px(&controls, 96.0);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, 1);
        assert_eq!(result[1].0, 2);
    }

    #[test]
    fn realistic_v1_scenario_1x1_table_two_tac_pictures() {
        // 사용자 시연 정확 재현: [Table(tac=false), Pic1(tac=true), Pic2(tac=true)]
        let controls = vec![
            Control::Table(Box::default()),
            Control::Picture(Box::new(make_pic(5670, 5670, true))),
            Control::Picture(Box::new(make_pic(5670, 5670, true))),
        ];
        let result = collect_sibling_tac_picture_widths_px(&controls, 96.0);
        assert_eq!(result.len(), 2);
        let total_width: f64 = result.iter().map(|(_, w)| w).sum();
        assert!((total_width - 151.2).abs() < 0.01); // 75.6 + 75.6
    }
}

#[cfg(test)]
mod pua_mapping_tests {
    use super::map_pua_bullet_char;

    #[test]
    fn supplementary_pua_a_passthrough_for_boxed_digits() {
        // 캡스톤 F-1 (2026-05-16): U+F02B1~F02C4 사각 안 숫자 한컴 자체 PUA — raw
        // passthrough (이전 ①~⑳ 표준 매핑은 fallback chain 효과 못 받아 NG). 시스템
        // 한컴 폰트 (함초롬바탕 확장B 등) 가 PUA 영역에서 사각 글리프 렌더링.
        for cp in 0xF02B1..=0xF02C4 {
            let ch = char::from_u32(cp).unwrap();
            assert_eq!(
                map_pua_bullet_char(ch),
                ch,
                "U+{:05X} should passthrough",
                cp
            );
        }
    }

    #[test]
    fn supplementary_pua_a_maps_middle_dot() {
        // [Task #509] U+F02EF → U+00B7 · Middle dot (KTX p10 표 회귀 origin)
        // 한컴 PDF 시각 정답지: dot (·) — ★ 가 아님 (작업지시자 정정)
        assert_eq!(map_pua_bullet_char('\u{F02EF}'), '\u{00B7}');
    }

    #[test]
    fn basic_pua_arrow_e8() {
        // [Task #509] U+0F0E8 → U+2794 ➔ (Heavy wide-headed rightwards arrow,
        // 한컴 PDF 정답지 시각 정합)
        assert_eq!(map_pua_bullet_char('\u{F0E8}'), '\u{2794}');
    }

    #[test]
    fn supplementary_pua_a_unmapped_returns_original() {
        // 매핑 표 외 영역은 원본 유지
        assert_eq!(map_pua_bullet_char('\u{F0500}'), '\u{F0500}');
    }

    #[test]
    fn basic_pua_outside_range_returns_original() {
        // 0xF020~0xF0FF 외 Basic PUA 는 원본 유지 (예: U+0F53A 한글 "흔")
        assert_eq!(map_pua_bullet_char('\u{F53A}'), '\u{F53A}');
    }

    #[test]
    fn supplementary_pua_a_low_range_maps_down_arrow() {
        // [Task #588] U+F003B → U+2193 ↓ (DOWNWARDS ARROW)
        // exam_eng.hwp p7 #40 요약형 문항 글상자 사이 화살표.
        // 한컴 PDF (HCRBatang) 임베디드 폰트 글리프 외곽 분석으로 확정.
        assert_eq!(map_pua_bullet_char('\u{F003B}'), '\u{2193}');
    }

    #[test]
    fn supplementary_pua_a_low_range_unmapped_returns_original() {
        // [Task #588] 0xF0000~0xF00CF 영역의 매핑 표 외 코드포인트는 원본 유지
        // (예: U+F0090 — img-start-001.hwp 1건, 별도 task 후보)
        assert_eq!(map_pua_bullet_char('\u{F0090}'), '\u{F0090}');
        assert_eq!(map_pua_bullet_char('\u{F0000}'), '\u{F0000}');
        assert_eq!(map_pua_bullet_char('\u{F00CF}'), '\u{F00CF}');
    }
}

impl LayoutEngine {
    fn paragraph_paint_session(
        &self,
    ) -> crate::renderer::paragraph_paint::ParagraphPaintSession<'_> {
        crate::renderer::paragraph_paint::ParagraphPaintSession {
            dpi: self.dpi,
            font_state: &self.font_state,
            host: Some(self),
            use_hwp3_origin_flow_spacing_before: &self.use_hwp3_origin_flow_spacing_before,
            keep_continuation_column_top_spacing_before: &self
                .keep_continuation_column_top_spacing_before,
            reapply_snap_anchored_spacing_before: &self.reapply_snap_anchored_spacing_before,
            profile: &self.profile,
            endnote_para_base: &self.endnote_para_base,
            uniform_filler_ladder: &self.uniform_filler_ladder,
            cell_has_square_float: &self.cell_has_square_float,
            body_float_carve_evidence: &self.body_float_carve_evidence,
            current_page_height: &self.current_page_height,
            overflow_cell_lines: &self.overflow_cell_lines,
            show_control_codes: &self.show_control_codes,
            last_item_content_bottom: &self.last_item_content_bottom,
            last_item_endnote_equation_tail_line_box: &self
                .last_item_endnote_equation_tail_line_box,
            collect_cell_para_borders: &self.collect_cell_para_borders,
            border_box_override: &self.border_box_override,
            para_border_ranges: &self.para_border_ranges,
            active_field: &self.active_field,
            current_body_area: &self.current_body_area,
            current_paper_width: &self.current_paper_width,
        }
    }
}

impl crate::renderer::paragraph_paint::ParagraphPaintHost for LayoutEngine {
    #[allow(clippy::too_many_arguments)]
    fn paint_inline_table_in_run(
        &self,
        tree: &mut PageLayoutContext,
        col_node: &mut RenderNode,
        p: &Paragraph,
        bdc: &[BinDataContent],
        styles: &ResolvedStyleSet,
        cell_ctx: &Option<CellContext>,
        col_area: &LayoutRect,
        tac_ci: usize,
        x: f64,
        y: f64,
        baseline: f64,
        raw_lh: f64,
        alignment: Alignment,
        section_index: usize,
        para_index: usize,
        line_tac_offsets: &[(usize, f64, usize)],
    ) -> (f64, f64) {
        let para = Some(p);
        let line_table_owner = para.and_then(|p| {
            line_tac_offsets
                .iter()
                .find_map(|(_, _, ci)| match p.controls.get(*ci) {
                    Some(Control::Table(table)) if table.common.treat_as_char => {
                        let h = hwpunit_to_px(table.common.height as i32, self.dpi);
                        let mt = hwpunit_to_px(table.outer_margin_top as i32, self.dpi);
                        let mb = hwpunit_to_px(table.outer_margin_bottom as i32, self.dpi);
                        ((mt > 0.0 || mb > 0.0)
                            && (h + mt + mb - 0.2..=h + mt + mb + 0.2).contains(&raw_lh))
                        .then_some((h, mt))
                    }
                    _ => None,
                })
        });

        let mut tac_table_om = (0.0, 0.0);
        if let Some(Control::Table(t)) = p.controls.get(tac_ci) {
            let raw_seg_width = p.line_segs.first().map(|s| s.segment_width).unwrap_or(0);
            let seg_width = if raw_seg_width > 0 {
                raw_seg_width
            } else {
                px_to_hwpunit(col_area.width, self.dpi)
            };
            let should_render_inline = cell_ctx.is_some()
                || crate::renderer::height_measurer::is_tac_table_inline_in_para(t, seg_width, p);
            let already_rendered = tree
                .get_inline_shape_position(section_index, para_index, tac_ci, cell_ctx.as_ref())
                .is_some();
            if t.common.treat_as_char && should_render_inline {
                // [Issue #3396] 렌더 여부와 무관하게 이 줄에서 표가
                // 문자로 취급되면 전진 폭에 outMargin 좌/우를 포함.
                tac_table_om = (
                    hwpunit_to_px(t.outer_margin_left as i32, self.dpi),
                    hwpunit_to_px(t.outer_margin_right as i32, self.dpi),
                );
            }
            if t.common.treat_as_char && should_render_inline && !already_rendered {
                let table_h = hwpunit_to_px(t.common.height as i32, self.dpi);
                let om_top = hwpunit_to_px(t.outer_margin_top as i32, self.dpi);
                let om_bottom = hwpunit_to_px(t.outer_margin_bottom as i32, self.dpi);
                // [#3386] 저장 lh 가 표+상하 외곽여백을 수용하는 줄
                // (한글이 lh = h + om 으로 저장한 표 전용 줄)은 표
                // 상단 = 줄 상단 + om_top 이 한글 실좌표다 (156678235
                // p5: 저장 vpos+om_top == 한글 PDF 상단, 종전 baseline
                // 하단정렬식은 om_top 을 소실해 3.8px 상향). #2220 의
                // stored_lh_covers_om 과 동일 술어의 px 판.
                // [#7049] `lh = h + om` 인 **표 전용 줄**만이라는 위
                // 계약대로 양쪽을 본다. 종전 `>=` 한쪽 판정은 밴드가
                // 줄에 **들어가기만** 하면 발동해, 한 글줄에 높이가 다른
                // TAC 표가 둘 이상일 때 전부 `y + om_top` 이 되어
                // **상단이 붙었다**. 한/글은 그 표들을 기준선에 앉혀
                // 하단을 높이차의 0.15 배로 벌린다 — 한/글 2020 실측
                // (HWPUNIT): `36384689_…화재발생종합보고서` 높이차 4708
                // → 하단차 707 · `issue2083_hide_fill_page` 9135 → 1366
                // · `issue2470/36382471_masked` 3126 → 467. 종전 규칙은
                // 이 차이를 전부 0 으로 본다.
                //
                // 줄 높이를 정하는 가장 높은 표는 `lh == 밴드` 라 그대로
                // 이 분기에 남고, `#3386` 의 표본(`156678235` 4쪽: 한/글
                // 536.69 vs rhwp 537.30)도 표가 하나뿐이라 불변이다.
                // [#7150] 같은 줄에 여러 표가 있어도 저장 lh가 자기
                // 높이와 상하 여백의 합이면 자기 바깥여백에 앉힌다.
                // 동반 표는 위에서 한 번 선택한 줄별 앵커를 공유한다.
                let stored_lh_covers_om = (om_top > 0.0 || om_bottom > 0.0)
                    && (table_h + om_top + om_bottom - 0.2..=table_h + om_top + om_bottom + 0.2)
                        .contains(&raw_lh);
                let table_y = if stored_lh_covers_om {
                    y + om_top
                } else if let Some((owner_h, owner_om_top)) = line_table_owner {
                    // [#7150] 소유자가 `y + owner_om_top` 에 앉으면 그 상자
                    // 하단이 `기준선 + 0.15×owner_h` 이므로 공유 기준선은
                    // `y + owner_om_top + 0.85×owner_h` 다. 이 표를 거기에
                    // 앉히면 `y + owner_om_top + 0.85×(owner_h − table_h)`.
                    // 자기 여백이 들어가지 않는 것이 실측과 맞는다
                    // (#7049 의 `21_언어_기출`: 여백 283/283 과 0/0 인 두
                    // 상자를 한/글이 같은 y 에 놓는다).
                    //
                    // 저장 기준선을 쓰던 종전 식은 소유자의 om_top 을
                    // 잃어 줄 전체가 `0.85×(om_top+om_bottom) − om_top`
                    // 만큼 내려앉았다 — issue2470 1쪽 결재표 1.31px.
                    (y + owner_om_top + (owner_h - table_h) * 0.85).max(y)
                } else {
                    // [#7049] 글자처럼 취급되는 표는 글자처럼 기준선에
                    // 앉는다 — 높이의 85% 가 기준선 위, 15% 가 아래다.
                    // 이 레포가 이미 쓰는 비율이다(`composer/
                    // line_breaking.rs` 가 `baseline_distance` 를
                    // `line_height * 0.85` 로 계산한다).
                    //
                    // 종전 `+ om_bottom` 은 상자 하단을 기준선 바로 밑에
                    // 붙여, 높이가 다른 표들의 하단 간격을 좁혔다.
                    // 한/글 2020 실측 하단차(px) — 위 술어 교정과 함께:
                    //   issue2083  121.80 → 35.20 → **16.9** (한/글 18.22)
                    //   issue2470   41.60 → 19.40 → ** 4.9** (한/글  6.23)
                    //   36384689    62.80 → 22.20 → ** 8.2** (한/글  9.43)
                    (y + baseline - table_h * 0.85).max(y)
                };
                // [Task #2212] 셀 안 인라인 TAC 표는 외곽 셀 경로를
                // 확장한 2단 cell_context 로 렌더해야 경로 기반 조회
                // (get_table_cell_bboxes_by_path 등)가 내부 셀을 찾는다.
                // table_layout 중첩 분기(:3475)와 동일한 확장 규칙 —
                // 내부 entry 의 cell/cp 는 layout_table 셀 루프가 채운다.
                let nested_ctx = cell_ctx.as_ref().map(|ctx| {
                    let mut c = ctx.clone();
                    c.path.push(crate::renderer::layout::CellPathEntry {
                        control_index: tac_ci,
                        cell_index: 0,
                        cell_para_index: 0,
                        text_direction: 0,
                    });
                    c
                });
                let nested_depth = usize::from(cell_ctx.is_some());
                self.layout_table(
                    tree,
                    col_node,
                    t,
                    section_index,
                    styles,
                    0,
                    col_area,
                    table_y,
                    bdc,
                    None,
                    nested_depth,
                    Some((para_index, tac_ci)),
                    alignment,
                    nested_ctx,
                    0.0,
                    0.0,
                    Some(x + tac_table_om.0),
                    None,
                    None,
                    None,
                    false,
                    false,
                    false,
                    None,
                    Self::standalone_table_char_border_fill(Some(p), t, styles),
                );
                // 스킵 마커 등록 (별도 Table PageItem에서 중복 렌더 방지)
                tree.set_inline_shape_position(
                    section_index,
                    para_index,
                    tac_ci,
                    cell_ctx.as_ref(),
                    x + tac_table_om.0,
                    table_y,
                );
            }
        }

        tac_table_om
    }
    #[allow(clippy::too_many_arguments)]
    fn paint_inline_table_in_empty_line(
        &self,
        tree: &mut PageLayoutContext,
        line_node: &mut RenderNode,
        p: &Paragraph,
        ctrl: &Control,
        bin_data_content: Option<&[BinDataContent]>,
        styles: &ResolvedStyleSet,
        cell_ctx: &Option<CellContext>,
        col_area: &LayoutRect,
        tac_ci: usize,
        img_x: f64,
        y: f64,
        alignment: Alignment,
        section_index: usize,
        para_index: usize,
    ) -> bool {
        if let Control::Table(t) = ctrl {
            if t.common.treat_as_char
                && tree
                    .get_inline_shape_position(section_index, para_index, tac_ci, cell_ctx.as_ref())
                    .is_none()
            {
                let om_l = hwpunit_to_px(t.outer_margin_left as i32, self.dpi);
                let om_top = hwpunit_to_px(t.outer_margin_top as i32, self.dpi);
                let table_x = img_x + om_l;
                let table_y = y + om_top;
                if let Some(bdc) = bin_data_content {
                    self.layout_table(
                        tree,
                        line_node,
                        t,
                        section_index,
                        styles,
                        0,
                        col_area,
                        table_y,
                        bdc,
                        None,
                        0,
                        Some((para_index, tac_ci)),
                        alignment,
                        cell_ctx.clone(),
                        0.0,
                        0.0,
                        Some(table_x),
                        None,
                        None,
                        None,
                        false,
                        false,
                        false,
                        None,
                        Self::standalone_table_char_border_fill(Some(p), t, styles),
                    );
                }
                tree.set_inline_shape_position(
                    section_index,
                    para_index,
                    tac_ci,
                    cell_ctx.as_ref(),
                    table_x,
                    table_y,
                );
                return true;
            }
            return false;
        }

        false
    }
    fn endnote_para_has_same_endnote_successor(&self, para_index: usize) -> bool {
        LayoutEngine::endnote_para_has_same_endnote_successor(self, para_index)
    }
    fn current_endnote_zero_spacing_profile(&self) -> bool {
        LayoutEngine::current_endnote_zero_spacing_profile(self)
    }
    fn is_tolerated_current_endnote_bottom_bleed(
        &self,
        is_endnote_flow: bool,
        content_bottom: f64,
        col_bottom: f64,
        equation_tail_line_box: bool,
    ) -> bool {
        LayoutEngine::is_tolerated_current_endnote_bottom_bleed(
            self,
            is_endnote_flow,
            content_bottom,
            col_bottom,
            equation_tail_line_box,
        )
    }
    fn note_ref_for_endnote_equation(
        &self,
        para_index: usize,
        inner_control_index: usize,
    ) -> Option<NoteControlRef> {
        LayoutEngine::note_ref_for_endnote_equation(self, para_index, inner_control_index)
    }
}
