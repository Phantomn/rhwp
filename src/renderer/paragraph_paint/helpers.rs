use crate::model::control::Control;
use crate::model::paragraph::{LineSeg, Paragraph};
use crate::model::shape::{
    Caption, CaptionDirection, CommonObjAttr, HorzAlign, HorzRelTo, TextWrap, VertRelTo,
};
use crate::model::style::{Alignment, LineSpacingType, UnderlineType};
use crate::model::table::Table;
use crate::renderer::composer::{
    compose_paragraph, effective_text_for_metrics, ComposedLine, ComposedParagraph, ComposedTextRun,
};
use crate::renderer::kerning::{
    ExactFontSlot, KerningLayoutSession, KerningRunMeasurementDisposition,
};
use crate::renderer::layout::text_measurement::{
    compute_char_positions, estimate_text_width, estimate_text_width_exact,
    estimate_text_width_unrounded, find_next_tab_stop, resolved_to_text_style,
};
use crate::renderer::layout::utils::{extract_shape_transform, picture_display_size_hu};
use crate::renderer::layout::CellContext;
use crate::renderer::page_layout::LayoutRect;
use crate::renderer::render_tree::*;
use crate::renderer::style_resolver::ResolvedStyleSet;
use crate::renderer::{format_number, hwpunit_to_px, NumberFormat as NumFmt, TabStop, TextStyle};

pub(crate) const CAPTION_CELL_SENTINEL: usize = 65534;

/// [#6699] 그림 뒤 문자열의 마지막 자간은 다음 글자가 없는 정렬 폭에 넣지 않는다.
/// 글자 전진 폭과 그림 뒤 원본 공백은 유지하고, 가운데/오른쪽 정렬의 점유 폭만 줄인다.
pub(crate) fn terminal_tracking_after_inline_picture(
    line: &ComposedLine,
    para: Option<&Paragraph>,
    styles: &ResolvedStyleSet,
    tac_offsets: &[(usize, f64, usize)],
) -> f64 {
    if tac_offsets.is_empty()
        || line
            .runs
            .iter()
            .any(|run| run.char_overlap.is_some() || run.display_text.is_some())
    {
        return 0.0;
    }
    let text_end = line.char_start
        + line
            .runs
            .iter()
            .map(|run| run.text.chars().count())
            .sum::<usize>();
    // 문자열 뒤 개체에는 실제로 자간 다음 내용이 있으므로 기존 전진 폭을 쓴다.
    if tac_offsets.iter().any(|(pos, _, _)| *pos >= text_end)
        || !tac_offsets.iter().any(|(_, _, index)| {
            matches!(
                para.and_then(|p| p.controls.get(*index)),
                Some(Control::Picture(picture)) if picture.common.treat_as_char
            )
        })
    {
        return 0.0;
    }
    let Some(run) = line.runs.iter().rev().find(|run| !run.text.is_empty()) else {
        return 0.0;
    };
    let Some(last) =
        unicode_segmentation::UnicodeSegmentation::graphemes(run.text.as_str(), true).next_back()
    else {
        return 0.0;
    };
    if last.chars().any(char::is_whitespace) {
        return 0.0;
    }
    let mut style = run.text_style(styles);
    if style.letter_spacing <= 0.0 || style.kerning {
        return 0.0;
    }
    // 폰트별 글자 폭에 비례하는 자간을 기존 측정 함수로 구한다. 정수 반올림은 하지 않는다.
    let tracked = estimate_text_width_unrounded(last, &style);
    style.letter_spacing = 0.0;
    (tracked - estimate_text_width_unrounded(last, &style)).max(0.0)
}

/// 최종 emitted text run에서만 exact pair positions를 게시한다.
///
/// `fallback_width`는 K0의 기존 반올림·field projection·줄-말미 공백 회수 계약을
/// 그대로 보존한다. exact pair가 실제 적용된 경우에만 최종 positions의 끝값을
/// bbox/다음 run advance로 사용한다.
#[allow(clippy::too_many_arguments)]
pub(crate) fn emitted_run_layout_positions(
    session: &mut KerningLayoutSession<'_>,
    slot: ExactFontSlot,
    replay_text: &str,
    style: &TextStyle,
    fallback_width: f64,
    trailing_space_count: usize,
    eligible: bool,
) -> (f64, Option<Vec<f64>>) {
    if !eligible
        || !style.kerning
        || replay_text.is_empty()
        || session.source_handle(slot).is_none()
    {
        return (fallback_width, None);
    }

    let mut base_positions = compute_char_positions(replay_text, style);
    let scalar_count = replay_text.chars().count();
    let trailing_space_count = trailing_space_count.min(scalar_count);
    if trailing_space_count > 0 && style.extra_word_spacing != 0.0 {
        let trailing_start = scalar_count - trailing_space_count;
        for (index, position) in base_positions
            .iter_mut()
            .enumerate()
            .skip(trailing_start + 1)
        {
            *position -= style.extra_word_spacing * (index - trailing_start) as f64;
        }
    }

    let base_font_size = if style.font_size > 0.0 {
        style.font_size
    } else {
        12.0
    };
    let effective_font_size_px = if style.superscript || style.subscript {
        base_font_size * crate::renderer::SCRIPT_FONT_SCALE
    } else {
        base_font_size
    };
    let width_ratio = if style.ratio > 0.0 { style.ratio } else { 1.0 };
    let measurement = session.measure_run(
        slot,
        replay_text,
        true,
        base_positions,
        effective_font_size_px,
        width_ratio,
    );
    if measurement.disposition != KerningRunMeasurementDisposition::PairAdjusted {
        return (fallback_width, None);
    }
    let Some(positions) = measurement.pair_adjusted_positions else {
        return (fallback_width, None);
    };
    let Some(width) = positions.last().copied() else {
        return (fallback_width, None);
    };
    if !width.is_finite() || width < 0.0 {
        return (fallback_width, None);
    }
    (width, Some(positions))
}

/// Q2-D4-B 최초 lane의 문단 전체 feature detection이다. 버전이나 파일 형식은
/// 보지 않고 최종 composed surface와 현재 style capability만 판정한다.
pub(crate) fn horizontal_shaping_initial_lane_preflight(
    composed: &ComposedParagraph,
    para: Option<&Paragraph>,
    styles: &ResolvedStyleSet,
    start_line: usize,
    end_line: usize,
    alignment: Alignment,
    para_border_fill_id: u16,
) -> bool {
    let (Some(para), Some(context), Some(outcome)) = (
        para,
        styles.horizontal_shaping_context.as_ref(),
        composed.horizontal_shaping.as_ref(),
    ) else {
        return false;
    };
    let (Some(line), Some(final_line)) = (composed.lines.first(), outcome.lines.first()) else {
        return false;
    };
    let Some(run) = line.runs.first() else {
        return false;
    };
    let Some(target) = final_line.target_runs.first() else {
        return false;
    };
    let style = run.text_style(styles);
    let scalar_count = run.text.chars().count();
    let instance_request = target.measurement.instance_request;
    let ratio_supported = if instance_request.is_some() {
        style.ratio <= 16.0
    } else {
        style.ratio < 0.999
    };
    let request_provenance_current = instance_request.is_none_or(|provenance| {
        provenance.slot
            == crate::renderer::kerning::ExactFontSlot::new(run.char_style_id, run.lang_index)
            && provenance.request_generation == context.instance_request_generation()
    });
    let style_surface_supported = !style.bold
        && !style.italic
        && style.font_size.is_finite()
        && style.font_size > 0.0
        && style.font_size <= 4_096.0
        && style.ratio.is_finite()
        && style.ratio > 0.0
        && ratio_supported
        && style.letter_spacing.abs() <= f64::EPSILON
        && style.underline == UnderlineType::None
        && !style.strikethrough
        && style.outline_type == 0
        && style.shadow_type == 0
        && !style.emboss
        && !style.engrave
        && !style.superscript
        && !style.subscript
        && style.emphasis_dot == 0
        && crate::model::color::char_shade(style.shade_color).is_none();
    let para_style_supported = styles
        .para_styles
        .get(composed.para_style_id as usize)
        .is_some_and(|style| {
            style.alignment == Alignment::Left
                && style.border_fill_id == 0
                && style.condense_min_space == 0
                && !style.auto_tab_right
        });
    let raw_vertical_positioning_is_zero = target
        .measurement
        .applied
        .glyphs
        .iter()
        .all(|glyph| glyph.y_offset == 0 && glyph.y_advance == 0);

    start_line == 0
        && end_line >= composed.lines.len()
        && composed.lines.len() == 1
        && line.runs.len() == 1
        && !line.has_line_break
        && line.char_start == 0
        && composed.numbering_text.is_none()
        && composed.inline_controls.is_empty()
        && composed.tac_controls.is_empty()
        && composed.footnote_positions.is_empty()
        && composed.tab_extended.is_empty()
        && para.controls.is_empty()
        && para.range_tags.is_empty()
        && para.field_ranges.is_empty()
        && para.orphan_field_ends.is_empty()
        && !para.text.chars().any(|character| {
            matches!(character, '\t' | '\n' | '\r' | '\u{fffc}') || character.is_control()
        })
        && para.text == run.text
        && !run.text.is_empty()
        && run.display_text.is_none()
        && run.char_overlap.is_none()
        && run.footnote_marker.is_none()
        && alignment == Alignment::Left
        && para_border_fill_id == 0
        && style_surface_supported
        && para_style_supported
        && outcome.lines.len() == 1
        && final_line.scalar_start == 0
        && final_line.scalar_end == scalar_count
        && final_line.target_runs.len() == 1
        && target.scalar_start == 0
        && target.scalar_end == scalar_count
        && target.measurement.code_point_count == scalar_count
        && target.measurement.registry_generation == context.registry_generation()
        && request_provenance_current
        && raw_vertical_positioning_is_zero
}

/// Mapping, exact-source certification, page attach가 모두 성공한 뒤에만
/// measurement advance를 반환한다. None이면 호출자는 K1/K0 legacy 경로를 그대로 탄다.
pub(crate) fn attach_horizontal_shaping_initial_lane(
    tree: &mut PageLayoutContext,
    composed: &ComposedParagraph,
    para: &Paragraph,
    styles: &ResolvedStyleSet,
    run: &ComposedTextRun,
    node_id: NodeId,
    scalar_start: usize,
    origin_x_px: f64,
    legacy_width_px: f64,
    split_cell: bool,
) -> Option<f64> {
    let outcome = composed.horizontal_shaping.as_ref()?;
    let context = styles.horizontal_shaping_context.as_ref()?;
    let candidate = crate::renderer::shaping_composition::HorizontalShapingEmittedRunCandidate {
        node_id,
        paragraph_text: &para.text,
        emitted_text: &run.text,
        scalar_start,
        origin_x_px,
        layout_positions_present: false,
        display_projection_present: false,
        horizontal_ltr_bidi0: true,
        has_field_or_note_split: false,
        has_char_overlap: false,
        has_border_or_background: false,
        has_decoration: false,
    };

    if crate::renderer::para_has_no_stored_line_segs(para) {
        let frame_interval_count = para.line_segs.first().map_or(1, |segment| {
            usize::from(
                segment.tag & LineSeg::TAG_SINGLE_SEGMENT_LINE == LineSeg::TAG_SINGLE_SEGMENT_LINE,
            )
        });
        let transaction = crate::renderer::shaping_composition::prepare_horizontal_shaping_no_lineseg_owner_transaction(
            context,
            std::sync::Arc::clone(outcome),
            crate::renderer::shaping_composition::HorizontalShapingNoLineSegSurface {
                model_line_seg_count: 0,
                frame_interval_count,
                edit_reflow: para.stored_text_partition_is_dirty(),
                stored_prefix: scalar_start != 0,
                split_cell,
                has_inline_control: !para.controls.is_empty(),
            },
            candidate,
            crate::renderer::shaping_composition::HorizontalShapingLegacyGeometry {
                line_width_px: legacy_width_px,
                bbox_width_px: legacy_width_px,
                next_origin_x_px: origin_x_px + legacy_width_px,
            },
        )
        .ok()?;
        let publication = tree
            .publish_horizontal_shaping_no_lineseg_owner_transaction(transaction)
            .ok()?;
        debug_assert!(publication.product_published());
        debug_assert!(std::sync::Arc::ptr_eq(
            publication.line_selection_measurement(),
            publication.bbox_measurement()
        ));
        debug_assert!(std::sync::Arc::ptr_eq(
            publication.line_selection_measurement(),
            publication.next_origin_measurement()
        ));
        debug_assert!(std::sync::Arc::ptr_eq(
            publication.line_selection_measurement(),
            publication.sidecar_measurement()
        ));
        debug_assert!((publication.line_width_px() - publication.bbox_width_px()).abs() <= 1.0e-9);
        debug_assert!(
            ((publication.next_origin_x_px() - origin_x_px) - publication.line_width_px()).abs()
                <= 1.0e-9
        );
        return Some(publication.bbox_width_px());
    }

    let mapped = crate::renderer::shaping_composition::map_horizontal_shaping_emitted_run(
        outcome, candidate,
    )
    .ok()?;
    let decision = crate::renderer::shaping_composition::certify_horizontal_shaping_mapped_run(
        context, &mapped,
    )
    .ok()?;
    tree.attach_horizontal_shaping_sidecar(mapped.node_id, mapped.range, decision)
        .ok()?;
    Some(mapped.bbox_width_px)
}

/// `RHWP_LAYOUT_DEBUG=1` 로 활성화되는 layout 디버그 로깅 여부.
/// Phase 1 (#517) — 본질 정정 (#467/#491/#496) 시 결함 측정·재현 자동화에 사용.
#[inline]
pub(crate) fn layout_debug_enabled() -> bool {
    std::env::var("RHWP_LAYOUT_DEBUG")
        .map(|v| v == "1")
        .unwrap_or(false)
}

/// lineseg baseline_distance를 폰트 어센트 기준으로 보정한다.
/// CENTER 문단 수직정렬 등으로 baseline이 50% 이하로 설정된 경우,
/// 텍스트 어센트(~80%)가 줄 박스 밖으로 넘치지 않도록 보장한다.
pub(crate) fn ensure_min_baseline(raw_baseline: f64, max_font_size: f64) -> f64 {
    if max_font_size <= 0.0 {
        return raw_baseline;
    }
    let min_baseline = max_font_size * 0.8;
    raw_baseline.max(min_baseline)
}

/// Resolve the character style at the model offset, including an empty paragraph.
pub(crate) fn paragraph_active_text_style(
    styles: &ResolvedStyleSet,
    para: Option<&Paragraph>,
    char_offset: usize,
) -> (TextStyle, Option<u32>) {
    let char_shape_id = para
        .and_then(|p| p.char_shape_id_at(char_offset))
        .or_else(|| para.and_then(|p| p.char_shapes.first().map(|cs| cs.char_shape_id)));

    if let Some(id) = char_shape_id {
        (resolved_to_text_style(styles, id, 0), Some(id))
    } else {
        (resolved_to_text_style(styles, 0, 0), None)
    }
}

/// 저장 LINE_SEG 없는 실제 빈 문단의 한컴 줄 metrics를 복원한다.
///
/// `compose_paragraph()` 는 렌더러 내부 안내용 400HU 줄을 남기지만, HWP5 원본의
/// 빈 문단 높이는 그 값이 아니라 글자 모양과 ParaShape 줄간격에서 결정된다.
/// HWP3 변환본만 기존 page-count 계약을 위해 작은 글꼴 cap을 유지한다.
pub(crate) fn empty_no_lineseg_paragraph_metrics(
    para: &Paragraph,
    styles: &ResolvedStyleSet,
    para_style: Option<&crate::renderer::style_resolver::ResolvedParaStyle>,
    hwp3_legacy_caps: bool,
    dpi: f64,
) -> Option<(f64, f64, f64)> {
    // typeset 쪽 empty_paragraph_fallback_line_metrics 와
    // 동일 완화 — 비자리차지(글앞/글뒤/어울림) 앵커 도형·그림만 가진 빈 문단도 한글은
    // 완전한 em 줄박스를 부여한다. 두 장부(판정·그리기)가 같은 규칙을 가져야 렌더 y 와
    // 단 경계가 일치한다.
    let controls_flow_neutral = para.controls.iter().all(|c| {
        let common = match c {
            crate::model::control::Control::Picture(p) => &p.common,
            crate::model::control::Control::Shape(s) => s.common(),
            _ => return false,
        };
        !common.treat_as_char
            && matches!(
                common.text_wrap,
                crate::model::shape::TextWrap::InFrontOfText
                    | crate::model::shape::TextWrap::BehindText
                    | crate::model::shape::TextWrap::Square
            )
    });
    if !para.text.trim().is_empty()
        || !(para.controls.is_empty() || controls_flow_neutral)
        || !para.line_segs.is_empty()
        || (para.char_count == 0 && para.controls.is_empty())
    {
        return None;
    }
    let char_shape_id = para
        .char_shape_id_at(0)
        .or_else(|| para.char_shapes.first().map(|shape| shape.char_shape_id))?
        as usize;
    let char_style = styles.char_styles.get(char_shape_id)?;
    let font_size = char_style.font_size;
    if font_size <= 0.0 {
        return None;
    }
    if hwp3_legacy_caps {
        let small_empty_para_max_font = hwpunit_to_px(1000, dpi);
        if font_size > small_empty_para_max_font + 0.1 {
            return None;
        }
        let meaningful_empty_para_min_font = hwpunit_to_px(800, dpi);
        if !char_style.bold && font_size < meaningful_empty_para_min_font - 0.1 {
            return None;
        }
    }
    let line_spacing = para_style.map(|style| style.line_spacing).unwrap_or(160.0);
    let line_spacing_type = para_style
        .map(|style| style.line_spacing_type)
        .unwrap_or(LineSpacingType::Percent);
    let (line_height, line_spacing_px) = crate::renderer::corrected_line_metrics(
        0.0,
        0.0,
        font_size,
        line_spacing_type,
        line_spacing,
    );
    Some((line_height, line_spacing_px, font_size))
}

pub(crate) fn numbering_marker_text_style(
    styles: &ResolvedStyleSet,
    para: Option<&Paragraph>,
    first_run: Option<&ComposedTextRun>,
) -> TextStyle {
    if let Some(run) = first_run {
        run.text_style(styles)
    } else {
        paragraph_active_text_style(styles, para, 0).0
    }
}

pub(crate) fn para_float_horz_intersects_column(
    common: &CommonObjAttr,
    width_hu: i32,
    col_area: &LayoutRect,
    dpi: f64,
) -> bool {
    if !matches!(common.horz_rel_to, HorzRelTo::Column | HorzRelTo::Para) {
        return true;
    }

    let width_px = hwpunit_to_px(width_hu, dpi);
    let h_offset_px = hwpunit_to_px(common.horizontal_offset as i32, dpi);
    let left = match common.horz_align {
        HorzAlign::Left | HorzAlign::Inside => col_area.x + h_offset_px,
        HorzAlign::Center => col_area.x + (col_area.width - width_px) / 2.0 + h_offset_px,
        HorzAlign::Right | HorzAlign::Outside => {
            col_area.x + col_area.width - width_px - h_offset_px
        }
    };
    let right = left + width_px;

    right > col_area.x + 0.5 && left < col_area.x + col_area.width - 0.5
}

pub(crate) fn has_para_topbottom_float_affecting_column(
    para: Option<&Paragraph>,
    col_area: &LayoutRect,
    dpi: f64,
) -> bool {
    para.map(|p| {
        p.controls.iter().any(|ctrl| match ctrl {
            Control::Picture(pic) => {
                !pic.common.treat_as_char
                    && matches!(pic.common.text_wrap, TextWrap::TopAndBottom)
                    && matches!(pic.common.vert_rel_to, VertRelTo::Para)
                    && {
                        let (width_hu, _) = picture_display_size_hu(pic);
                        para_float_horz_intersects_column(&pic.common, width_hu, col_area, dpi)
                    }
            }
            Control::Shape(shape) => {
                let common = shape.common();
                !common.treat_as_char
                    && matches!(common.text_wrap, TextWrap::TopAndBottom)
                    && matches!(common.vert_rel_to, VertRelTo::Para)
                    && para_float_horz_intersects_column(common, common.width as i32, col_area, dpi)
            }
            _ => false,
        })
    })
    .unwrap_or(false)
}

pub(crate) fn is_treat_as_char_equation_control(ctrl: Option<&Control>) -> bool {
    matches!(ctrl, Some(Control::Equation(eq)) if eq.common.treat_as_char)
}

pub(crate) fn is_caption_cell_context(cell_ctx: Option<&CellContext>) -> bool {
    cell_ctx
        .and_then(|ctx| ctx.path.last())
        .is_some_and(|entry| entry.cell_index == CAPTION_CELL_SENTINEL)
}

/// HWP5 원본 LineSeg가 저장한 column-relative 줄 시작점을 일반 본문 줄에 적용한다.
///
/// ParaShape의 margin/indent는 재조판 기본값이고, 원본 LineSeg.column_start는 해당
/// 줄의 확정 좌표다. 다만 cs+sw가 단 너비와 같은 일반 줄에만 적용한다. 그림 어울림,
/// 표 셀, 합성 LineSeg는 각각 별도 좌표계를 사용하므로 caller가 `eligible=false`로
/// 제외해 column_start가 이중 적용되지 않게 한다.
pub(crate) fn authoritative_stored_line_start_px(
    styled_margin_left: f64,
    line_seg: Option<&LineSeg>,
    column_width_hu: i32,
    dpi: f64,
    eligible: bool,
) -> f64 {
    let Some(line_seg) = line_seg else {
        return styled_margin_left;
    };
    let full_width_line = line_seg.column_start > 0
        && line_seg.segment_width > 0
        && line_seg
            .column_start
            .saturating_add(line_seg.segment_width)
            .saturating_sub(column_width_hu)
            .abs()
            <= 200;
    let authoritative = line_seg.tag & LineSeg::TAG_IMPLEMENTATION_PROPERTY == 0;
    if !eligible || !authoritative || !full_width_line {
        return styled_margin_left;
    }

    styled_margin_left.max(hwpunit_to_px(line_seg.column_start, dpi))
}

/// HWP5의 저장 `column_start`를 권위로 해석할 수 있는 출처 경계.
///
/// HWP5-origin HWPX는 컨테이너만 HWPX일 뿐 저장 LINE_SEG는 HWP5 원본의 것이므로
/// 원본 HWP5와 같은 계약을 쓴다. 원본 HWPX까지 넓히면 별도 저장 계약을 침범한다.
pub(crate) fn uses_hwp5_stored_line_start_profile(
    profile: crate::model::provenance::LayoutCompatibilityProfile,
) -> bool {
    profile.hwp5_stored_pagination_layout()
}

pub(crate) fn composed_line_char_end(comp: &ComposedParagraph, line_idx: usize) -> usize {
    if let Some(next) = comp.lines.get(line_idx + 1) {
        return next.char_start;
    }
    let Some(line) = comp.lines.get(line_idx) else {
        return 0;
    };
    line.char_start
        + line
            .runs
            .iter()
            .map(|run| run.text.chars().count())
            .sum::<usize>()
        + usize::from(line.has_line_break)
}

pub(crate) fn char_pos_in_line(pos: usize, start: usize, end: usize) -> bool {
    if end > start {
        pos >= start && pos < end
    } else {
        pos == start
    }
}

/// [#5727] 이 TAC 위치를 **앞선 빈 composed 줄**이 이미 소유하는가.
///
/// 저장 lineseg 가 TAC 개체에 자기 줄을 배정하면(제어문자만 담아 텍스트 범위가
/// 비는 줄) 그 빈 줄과 다음 줄의 `char_start` 가 같은 텍스트 인덱스로 붕괴한다
/// (제어문자는 `text` 에 없고 `char_offsets` 갭으로만 남는다). 이때 다음 줄이
/// 그 TAC 를 다시 집으면 개체가 다음 줄로 끌려 내려가고, 그 줄 텍스트는 개체
/// 폭만큼 오른쪽에서 시작한다 — 156732636 로고 칸 실측: `노동부` 가 저장
/// horzpos=0 인데 +172px(로고 폭)에서 시작. 빈 줄 소유 TAC 는 다음 줄 귀속에서
/// 제외한다.
pub(crate) fn tac_owned_by_prior_empty_line(
    comp: &ComposedParagraph,
    line_idx: usize,
    pos: usize,
) -> bool {
    if line_idx == 0 {
        return false;
    }
    let Some(line) = comp.lines.get(line_idx) else {
        return false;
    };
    // 현재 줄이 **텍스트 줄**일 때만 적용 — 빈 줄 연쇄(빈 문단 + TAC 여러 개,
    // 59043 p12 실측)는 기존 반복-빈-줄 기제가 소유를 배정하므로 건드리지 않는다.
    if line.char_start != pos || line.runs.is_empty() {
        return false;
    }
    comp.lines
        .get(line_idx - 1)
        .is_some_and(|prev| prev.runs.is_empty() && prev.char_start == pos)
}

pub(crate) fn line_has_tac_control(comp: &ComposedParagraph, line_idx: usize) -> bool {
    let Some(line) = comp.lines.get(line_idx) else {
        return false;
    };
    let start = line.char_start;
    let end = comp
        .lines
        .get(line_idx + 1)
        .map(|next| next.char_start)
        .unwrap_or(usize::MAX);
    comp.tac_controls
        .iter()
        .any(|(pos, _, _)| char_pos_in_line(*pos, start, end))
}

pub(crate) fn line_has_strict_tac_control(
    comp: &ComposedParagraph,
    tac_offsets_px: &[(usize, f64, usize)],
    line_idx: usize,
) -> bool {
    let Some(line) = comp.lines.get(line_idx) else {
        return false;
    };
    let start = line.char_start;
    let end = composed_line_char_end(comp, line_idx);
    end > start
        && tac_offsets_px
            .iter()
            .any(|(pos, _, _)| *pos >= start && *pos < end)
}

pub(crate) fn line_has_strict_equation_tac_control(
    para: Option<&Paragraph>,
    comp: &ComposedParagraph,
    tac_offsets_px: &[(usize, f64, usize)],
    line_idx: usize,
) -> bool {
    let Some(para) = para else {
        return false;
    };
    let Some(line) = comp.lines.get(line_idx) else {
        return false;
    };
    let start = line.char_start;
    let end = composed_line_char_end(comp, line_idx);
    end > start
        && tac_offsets_px.iter().any(|(pos, _, ci)| {
            *pos >= start && *pos < end && is_treat_as_char_equation_control(para.controls.get(*ci))
        })
}

pub(crate) fn line_is_leading_empty_equation_tac_guide(
    para: Option<&Paragraph>,
    comp: &ComposedParagraph,
    tac_offsets_px: &[(usize, f64, usize)],
    line_idx: usize,
) -> bool {
    let Some(line) = comp.lines.get(line_idx) else {
        return false;
    };
    let Some(next) = comp.lines.get(line_idx + 1) else {
        return false;
    };
    line.runs.is_empty()
        && line.char_start == next.char_start
        && !line_has_strict_tac_control(comp, tac_offsets_px, line_idx)
        && line_has_strict_equation_tac_control(para, comp, tac_offsets_px, line_idx + 1)
}

/// [#1925 추출] `layout_empty_runs_line` 줄-스코프 스칼라 입력 묶음.
#[derive(Clone, Copy)]
pub(crate) struct EmptyRunsLineVars {
    /// Physical rows already own the hit/flow box. The empty run is a
    /// zero-advance caret anchor, including center/right aligned anchors.
    pub(crate) physical_frame_rows: bool,
    pub(crate) alignment: crate::model::style::Alignment,
    pub(crate) available_width: f64,
    pub(crate) effective_col_x: f64,
    pub(crate) effective_margin_left: f64,
    pub(crate) x_start: f64,
    /// 이 줄 끝의 문서 char 좌표 (원본: char_offset)
    pub(crate) line_char_end: usize,
    pub(crate) y: f64,
    pub(crate) baseline: f64,
    pub(crate) raw_lh: f64,
    pub(crate) runs_all_whitespace: bool,
    pub(crate) max_fs: f64,
    pub(crate) line_spacing_px: f64,
    /// para_topbottom_line_vpos_base.is_some()
    pub(crate) has_topbottom_vpos_base: bool,
    pub(crate) is_last_line_of_para: bool,
    pub(crate) defer_empty_line_control_marker: bool,
    pub(crate) line_flow_height: f64,
    pub(crate) section_index: usize,
    pub(crate) para_index: usize,
    /// [#5727] composed 줄 인덱스 — 경계 TAC 자기-줄 판정에 사용.
    pub(crate) line_idx: usize,
}
/// [#2003] run 방출 루프의 줄-간 캐리오버 묶음 (Copy 스칼라 9종) — 값 전달 + 반환.
#[derive(Clone, Copy)]
pub(crate) struct RunEmitState {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) char_offset: usize,
    pub(crate) run_char_pos: usize,
    pub(crate) inline_tab_cursor_render: usize,
    pub(crate) pending_right_tab_render: Option<(f64, u8, u8)>,
    pub(crate) pending_right_leader_digit_render: bool,
    pub(crate) current_line_reserved_tac_picture_height: Option<f64>,
}

/// [#2067] TAC 그림 배치의 줄-스코프 스칼라 입력 묶음.
#[derive(Clone, Copy)]
pub(crate) struct TacPictureLineVars {
    pub(crate) run_char_pos: usize,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) baseline: f64,
    pub(crate) raw_lh: f64,
    pub(crate) section_index: usize,
    pub(crate) para_index: usize,
}

/// [#2067] 빈 runs 줄 TAC 수식 인라인 배치의 줄-스코프 스칼라 입력 묶음.
#[derive(Clone, Copy)]
pub(crate) struct EquationTacLineVars {
    pub(crate) line_idx: usize,
    /// 이 문단에서 배치하는 마지막 줄 인덱스 상한 (원본: end)
    pub(crate) line_end: usize,
    pub(crate) alignment: crate::model::style::Alignment,
    pub(crate) available_width: f64,
    pub(crate) margin_left: f64,
    pub(crate) indent: f64,
    pub(crate) effective_col_x: f64,
    pub(crate) y: f64,
    pub(crate) baseline: f64,
    pub(crate) line_height: f64,
    pub(crate) line_spacing_px: f64,
    pub(crate) col_area_y: f64,
    pub(crate) col_bottom: f64,
    /// 이 줄 끝의 문서 char 좌표 (원본: char_offset)
    pub(crate) line_char_end: usize,
    pub(crate) is_last_line_of_para: bool,
    pub(crate) defer_empty_line_control_marker: bool,
    pub(crate) equation_tac_extra_rows: usize,
    /// [Task #1472] hwp3 변환본 indent scale 배율 — 소스분기는 caller 유지.
    pub(crate) hwp3_indent_scale: f64,
    pub(crate) section_index: usize,
    pub(crate) para_index: usize,
}

/// [#2003] run 방출 루프의 줄-스코프 읽기 스칼라 묶음.
#[derive(Clone, Copy)]
pub(crate) struct RunEmitVars {
    pub(crate) stored_tac_assignment: bool,
    pub(crate) trailing_space_limit: usize,
    pub(crate) baseline: f64,
    pub(crate) raw_lh: f64,
    pub(crate) alignment: crate::model::style::Alignment,
    pub(crate) auto_tab_right: bool,
    pub(crate) available_width: f64,
    pub(crate) effective_margin_left: f64,
    pub(crate) end: usize,
    pub(crate) extra_char_sp: f64,
    pub(crate) extra_dash_sp: f64,
    pub(crate) extra_word_sp: f64,
    pub(crate) has_tabs: bool,
    pub(crate) horizontal_shaping_initial_lane: bool,
    pub(crate) is_last_line_of_para: bool,
    pub(crate) line_height: f64,
    pub(crate) line_idx: usize,
    pub(crate) line_spacing_px: f64,
    pub(crate) max_fs: f64,
    pub(crate) runs_all_whitespace: bool,
    pub(crate) renders_synthetic_wrap_trailing_space: bool,
    pub(crate) start_line: usize,
    pub(crate) tab_width: f64,
    pub(crate) section_index: usize,
    pub(crate) para_index: usize,
}

// [#2510] 종전 `receipt_date_stamp_shift_px`(#2020) 제거 — 접수증 ㊞ 를
// "한컴 공백 0.42em" 가정으로 −21px 이동시키던 보정. 실측(무신축 래더)
// space=0.505em 균일이라 가정이 허구였고, 실체는 구 HY 테이블의 글자
// 과대폭(+20px)을 도장 위치에서만 상쇄하던 것 — #2430 실측 메트릭 교정으로
// 불필요·유해(㊞ 오라클 −15px)해져 제거. 제거+교정 시 ㊞ = 오라클 +6.2px,
// issue_2020 도장 정렬 핀 4/4 유지 (PR #2510 코멘트 5017316669 실측).

/// [#1925 추출] `estimate_line_run_widths` 결과 — est 사전 폭 추정 산출물.
pub(crate) struct LineWidthEst {
    /// 추정 종료 x (초기값 기준 누적 점유 폭 계산용)
    pub(crate) est_x: f64,
    /// 추정에 포함된 tac 개체 폭 합
    pub(crate) included_tac_width: f64,
}
pub(crate) fn tac_offsets_for_line(
    comp: &ComposedParagraph,
    tac_offsets_px: &[(usize, f64, usize)],
    line_idx: usize,
) -> Vec<(usize, f64, usize)> {
    let Some(line) = comp.lines.get(line_idx) else {
        return Vec::new();
    };
    let start = line.char_start;
    let end = composed_line_char_end(comp, line_idx);
    // [#6754] 폭이 0 인 줄은 `char_pos_in_line` 이 TAC 를 **하나만** 소유하게 한다.
    // 글자 없이 TAC 개체만 담은 **한 줄짜리** 문단은 그 개체들이 모두 같은 줄에
    // 나란히 놓이는데(저장 사다리도 같은 `vpos` 를 적는다), 그 규칙 때문에 둘째부터
    // 어느 줄에도 안 실려 그려지지 않았다 — 156585314 3쪽: 그림(pos 0)만 그려지고
    // 4×8 표(pos 1)가 사라진다.
    let single_empty_line = comp.lines.len() == 1 && end <= start;
    tac_offsets_px
        .iter()
        .copied()
        .filter(|(pos, _, _)| {
            (single_empty_line || char_pos_in_line(*pos, start, end))
                // [#5727] 앞선 빈 줄(개체 자기 줄)이 소유한 경계 TAC 는 제외
                && !tac_owned_by_prior_empty_line(comp, line_idx, *pos)
        })
        .collect()
}

/// 정렬 폭 산정에 사용할 줄 단위 TAC 집합.
///
/// 기본 줄 범위는 [`tac_offsets_for_line`]과 동일하게 엄격한 반열림 구간이다. 다만
/// 실제 렌더 경로(`emit_line_runs`)는 문단 마지막 run 또는 명시 줄바꿈의 마지막 run
/// 끝에 놓인 TAC를 현재 줄에 방출한다. 그 TAC를 폭 계산에서 제외하면 Center/Right
/// 정렬의 시작점만 그림 폭만큼 어긋난다 (#3257).
///
/// 다음 composed line이 정확히 같은 run 끝 위치에서 시작하면 그 TAC는 다음 줄 선두다.
/// #1219의 줄 경계 수식 중복·폭 오포함을 막기 위해 이 경우에는 추가하지 않는다.
/// [#5820 → Issue #6173] 오른쪽/가운데 정렬이 폭에서 제외할 **줄 말미 공백** 폭 (px).
///
/// 말미 공백이 서로 다른 글꼴·글자 크기의 run 경계를 넘을 수 있으므로, 전체 공백을
/// 마지막 run 의 style 로 재측정하지 않고 뒤에서부터 각 run 의 실제 style 폭을 더한다.
///
/// **[Issue #6173] 자리차지(TAC) 개체 앞 공백은 말미 공백이 아니다.** 인라인 개체는
/// run 을 쪼개지 않고 run 안 char 위치에 놓이므로 `[그림A][공백4][그림B][공백2]` 가
/// 공백 6칸짜리 run **하나**로 합성된다. run 만 보고 뒤에서 공백을 세면 그림 사이 4칸까지
/// 말미로 걷어내 오른쪽 앵커가 그만큼(26.7px) 우측으로 밀리고, 마지막 그림이 글상자
/// 우단을 넘어 잘린다(156740495 2쪽). 줄의 **마지막 개체 위치 뒤** 공백만 말미다.
///
/// - `last_inline_object_pos`: 이 줄이 소유한 TAC 개체 중 마지막 것의 절대 char 위치.
///   개체가 없으면 `None` — 종전 동작 그대로.
/// - `stop_on_underline`: 밑줄 친 말미 공백에서 멈춘다(가운데 정렬 전용 규칙).
pub(crate) fn trailing_space_width_after_last_inline_object(
    line: &ComposedLine,
    last_inline_object_pos: Option<usize>,
    styles: &ResolvedStyleSet,
    stop_on_underline: bool,
) -> f64 {
    let run_chars = |r: &crate::renderer::composer::ComposedTextRun| -> usize {
        if r.char_overlap.is_some() {
            let chars: Vec<char> = r.text.chars().collect();
            crate::renderer::composer::char_overlap_advance_units(&chars)
        } else {
            r.text.chars().count()
        }
    };
    let mut run_end_pos = line.char_start + line.runs.iter().map(run_chars).sum::<usize>();
    let mut width = 0.0;
    for run in line.runs.iter().rev() {
        let run_char_count = run_chars(run);
        let run_start_pos = run_end_pos.saturating_sub(run_char_count);
        let mut trailing_spaces = run.text.chars().rev().take_while(|c| *c == ' ').count();
        if let Some(obj_pos) = last_inline_object_pos {
            // 마지막 개체 뒤로 자른다 — 개체 자리 이전 공백은 콘텐츠다.
            let floor = obj_pos.max(run_start_pos);
            trailing_spaces = trailing_spaces.min(run_end_pos.saturating_sub(floor));
        }
        if trailing_spaces == 0 {
            break;
        }
        let ts = run.text_style(styles);
        if stop_on_underline && ts.underline != crate::renderer::UnderlineType::None {
            break;
        }
        // Alignment subtracts this suffix from the unrounded line/run advance.
        // Rounding only the suffix shifts the visible right edge (and the
        // center) even though every glyph keeps the same replay metrics.
        width += estimate_text_width_exact(&" ".repeat(trailing_spaces), &ts);
        if trailing_spaces != run_char_count {
            break;
        }
        run_end_pos = run_start_pos;
    }
    width
}

/// 마지막 개체 앞의 공백은 내부 공백이다. 개체가 없으면 기존 말미 공백 수를 제한하지 않는다.
pub(crate) fn trailing_space_limit_after_last_inline_object(
    line: &ComposedLine,
    last_inline_object_pos: Option<usize>,
) -> usize {
    last_inline_object_pos.map_or(usize::MAX, |pos| {
        let end = line.char_start
            + line
                .runs
                .iter()
                .map(|run| {
                    if run.char_overlap.is_some() {
                        let chars: Vec<char> = run.text.chars().collect();
                        crate::renderer::composer::char_overlap_advance_units(&chars)
                    } else {
                        run.text.chars().count()
                    }
                })
                .sum::<usize>();
        end.saturating_sub(pos)
    })
}

pub(crate) fn tac_offsets_for_line_width(
    comp: &ComposedParagraph,
    tac_offsets_px: &[(usize, f64, usize)],
    line_idx: usize,
) -> Vec<(usize, f64, usize)> {
    let mut offsets = tac_offsets_for_line(comp, tac_offsets_px, line_idx);
    let Some(line) = comp.lines.get(line_idx) else {
        return offsets;
    };
    if line.runs.is_empty() {
        return offsets;
    }

    let run_end = line.char_start
        + line
            .runs
            .iter()
            .map(|run| run.text.chars().count())
            .sum::<usize>();
    let is_last_line = comp.lines.get(line_idx + 1).is_none();
    let next_starts_at_run_end = comp
        .lines
        .get(line_idx + 1)
        .is_some_and(|next| next.char_start == run_end);
    let emits_trailing_tac = (is_last_line || line.has_line_break) && !next_starts_at_run_end;
    if !emits_trailing_tac {
        return offsets;
    }

    for offset @ (pos, _, _) in tac_offsets_px.iter().copied() {
        if pos == run_end && !offsets.iter().any(|(_, _, ci)| *ci == offset.2) {
            offsets.push(offset);
        }
    }
    offsets
}

pub(crate) fn repeated_empty_tac_line_offset(
    comp: &ComposedParagraph,
    tac_offsets_px: &[(usize, f64, usize)],
    line_idx: usize,
) -> Option<Vec<(usize, f64, usize)>> {
    let line = comp.lines.get(line_idx)?;
    if !line.runs.is_empty() {
        return None;
    }

    let start = line.char_start;
    let repeated_empty_line_count = comp
        .lines
        .iter()
        .filter(|candidate| candidate.runs.is_empty() && candidate.char_start == start)
        .count();
    if repeated_empty_line_count <= 1 {
        return None;
    }

    let line_ordinal = comp
        .lines
        .iter()
        .take(line_idx)
        .filter(|candidate| candidate.runs.is_empty() && candidate.char_start == start)
        .count();
    let line_tac_sequence = tac_offsets_px
        .iter()
        .copied()
        .filter(|(pos, _, _)| *pos >= start && *pos < start + repeated_empty_line_count)
        .collect::<Vec<_>>();

    // 텍스트 없는 HWP 문단은 LINE_SEG 여러 줄이 같은 text_start 를 가질 수 있다.
    // TAC가 빈 줄보다 적으면 앞 줄부터 하나씩만 귀속하고, 나머지 guide 줄에는
    // 이미 귀속한 개체를 되풀이해 그리지 않는다. TAC 수와 빈 줄 수가 정확히
    // 같은 기존 사례도 같은 순서 배정으로 보존된다.
    if !line_tac_sequence.is_empty() && line_tac_sequence.len() <= repeated_empty_line_count {
        // 후보가 모자란 뒤쪽 guide 줄도 `Some(vec![])`으로 명시해야 한다. `None`을
        // 반환하면 호출자가 기본 줄-범위 집합으로 되돌아가 같은 TAC를 재배정한다.
        Some(
            line_tac_sequence
                .get(line_ordinal)
                .copied()
                .into_iter()
                .collect(),
        )
    } else {
        None
    }
}

pub(crate) fn note_number_format_from_hwp_code(code: u8) -> NumFmt {
    match code {
        0 => NumFmt::Digit,
        1 => NumFmt::CircledDigit,
        2 => NumFmt::RomanUpper,
        3 => NumFmt::RomanLower,
        4 => NumFmt::LatinUpper,
        5 => NumFmt::LatinLower,
        8 => NumFmt::HangulGaNaDa,
        12 => NumFmt::HangulNumber,
        13 => NumFmt::HanjaNumber,
        _ => NumFmt::Digit,
    }
}

pub(crate) fn note_decoration_char(value: u16) -> Option<char> {
    if value == 0 {
        None
    } else {
        char::from_u32(value as u32).filter(|ch| *ch != '\0')
    }
}

pub(crate) fn format_note_marker_text(
    number: u16,
    number_shape: u32,
    before_decoration_letter: u16,
    after_decoration_letter: u16,
) -> String {
    let number = format_number(number, note_number_format_from_hwp_code(number_shape as u8));
    let prefix = note_decoration_char(before_decoration_letter)
        .map(|ch| ch.to_string())
        .unwrap_or_default();
    let suffix = note_decoration_char(after_decoration_letter)
        .unwrap_or(')')
        .to_string();
    format!("{}{}{}", prefix, number, suffix)
}

pub(crate) fn note_marker_text_from_control(
    ctrl: Option<&Control>,
    fallback_number: u16,
) -> String {
    match ctrl {
        Some(Control::Footnote(footnote)) => format_note_marker_text(
            fallback_number,
            footnote.number_shape,
            footnote.before_decoration_letter,
            footnote.after_decoration_letter,
        ),
        Some(Control::Endnote(endnote)) => format_note_marker_text(
            fallback_number,
            endnote.number_shape,
            endnote.before_decoration_letter,
            endnote.after_decoration_letter,
        ),
        _ => format!("{})", fallback_number),
    }
}

pub(crate) fn is_leading_endnote_marker_rendered_as_prefix(
    para: Option<&Paragraph>,
    control_index: usize,
    line_idx: usize,
    start_line: usize,
    marker_pos: usize,
    line_char_start: usize,
) -> bool {
    line_idx == start_line
        && start_line == 0
        && marker_pos == line_char_start
        && matches!(
            para.and_then(|p| p.controls.get(control_index)),
            Some(Control::Endnote(_))
        )
}

pub(crate) fn line_tac_picture_or_shape_height(
    para: Option<&Paragraph>,
    comp: &ComposedParagraph,
    tac_offsets_px: &[(usize, f64, usize)],
    line_idx: usize,
    dpi: f64,
) -> Option<f64> {
    let para = para?;
    tac_offsets_for_line(comp, tac_offsets_px, line_idx)
        .iter()
        .find_map(|(_, _, ci)| {
            para.controls
                .get(*ci)
                .and_then(|ctrl| crate::renderer::tac_object_flow_height_px(ctrl, dpi))
        })
}

pub(crate) fn text_line_is_picture_lead_in(
    para: Option<&Paragraph>,
    comp: &ComposedParagraph,
    tac_offsets_px: &[(usize, f64, usize)],
    line_idx: usize,
    raw_lh: f64,
    max_fs: f64,
    dpi: f64,
) -> bool {
    if max_fs <= 0.0 || raw_lh <= max_fs * 2.0 {
        return false;
    }
    let Some(line) = comp.lines.get(line_idx) else {
        return false;
    };
    if line.runs.iter().all(|run| run.text.trim().is_empty())
        || line_tac_picture_or_shape_height(para, comp, tac_offsets_px, line_idx, dpi).is_some()
    {
        return false;
    }
    let Some(next) = comp.lines.get(line_idx + 1) else {
        return false;
    };
    if !next.runs.iter().all(|run| run.text.trim().is_empty()) {
        return false;
    }
    line_tac_picture_or_shape_height(para, comp, tac_offsets_px, line_idx + 1, dpi)
        .map(|height| (raw_lh - height).abs() <= 8.0)
        .unwrap_or(false)
}

pub(crate) fn has_treat_as_char_picture_or_shape(para: Option<&Paragraph>) -> bool {
    para.map(|para| {
        para.controls.iter().any(|ctrl| {
            matches!(
                ctrl,
                Control::Picture(pic) if pic.common.treat_as_char
            ) || matches!(
                ctrl,
                Control::Shape(shape) if shape.common().treat_as_char
            )
        })
    })
    .unwrap_or(false)
}

pub(crate) fn is_blank_spacer_line(
    para: Option<&Paragraph>,
    is_endnote_virtual_para: bool,
    runs_all_whitespace: bool,
    line_tac_offsets: &[(usize, f64, usize)],
) -> bool {
    if !runs_all_whitespace || !line_tac_offsets.is_empty() {
        return false;
    }
    is_endnote_virtual_para || para.map(|p| p.controls.is_empty()).unwrap_or(false)
}

pub(crate) fn is_equation_only_tac_line(
    para: Option<&Paragraph>,
    runs_all_whitespace: bool,
    line_tac_offsets: &[(usize, f64, usize)],
) -> bool {
    let Some(para) = para else {
        return false;
    };
    runs_all_whitespace
        && !line_tac_offsets.is_empty()
        && line_tac_offsets
            .iter()
            .all(|(_, _, ci)| is_treat_as_char_equation_control(para.controls.get(*ci)))
}

pub(crate) fn tac_picture_label_extra_px(
    runs_all_whitespace: bool,
    raw_line_height: f64,
    reserved_picture_height: Option<f64>,
    max_font_size: f64,
    line_spacing_px: f64,
) -> f64 {
    let Some(pic_h) = reserved_picture_height else {
        return 0.0;
    };
    if runs_all_whitespace || max_font_size <= 0.0 {
        return 0.0;
    }
    if (raw_line_height - pic_h).abs() > 4.0 || raw_line_height <= max_font_size * 2.0 {
        return 0.0;
    }
    max_font_size + line_spacing_px.max(0.0)
}

/// [#6575] TAC 개체를 baseline 에 앉힐 때 쓰는 **개체 상자 전체 높이**(px).
///
/// 종전에는 그림 높이만으로 `y + baseline - pic_h` 를 잡았다. 그런데 위/아래 캡션이
/// 붙은 그림은 저장 줄이 **그림 + 캡션 간격 + 캡션**을 통째로 예약하므로, 그림만
/// 바닥맞춤하면 캡션 높이만큼 아래로 내려간다.
///
/// 실측 `156489219` 5쪽 `pi=43`(한글 2024 오라클):
///
/// ```text
/// y=233.7  baseline=240.7  pic_h=205.7  raw_lh=283.1  caption=Bottom(spacing 850, 3문단)
///   종전:  233.7 + 240.7 - 205.7            = 268.7   (한/글 233.7 대비 +35.0px)
///   상자:  (233.7 + 240.7 - 283.1).max(233.7) = 233.7  ✔
/// ```
///
/// 상자가 baseline 보다 크면 기존 `.max(y)` 클램프가 그대로 줄 상단을 준다 — 한컴이
/// 이런 줄에서 개체를 줄 상단에 붙이는 동작과 같은 답이다.
///
/// 좌/우 캡션은 폭을 늘릴 뿐 높이를 늘리지 않으므로 세로 방향(Top/Bottom)만 센다.
pub(crate) fn tac_object_box_height_px(object_h: f64, caption: &Option<Caption>, dpi: f64) -> f64 {
    let Some(cap) = caption else {
        return object_h;
    };
    if !matches!(
        cap.direction,
        CaptionDirection::Top | CaptionDirection::Bottom
    ) || cap.paragraphs.is_empty()
    {
        return object_h;
    }
    let caption_h = crate::renderer::composer::caption_height_px(caption, dpi);
    if caption_h <= 0.0 {
        return object_h;
    }
    object_h + hwpunit_to_px(i32::from(cap.spacing), dpi) + caption_h
}

/// [#6603] 글자처럼 그림의 바깥 여백(px) — (왼쪽, 오른쪽, 위, 아래).
///
/// 한/글은 여백을 포함한 상자를 줄 안에 놓고 잉크를 상자의 (왼쪽 여백, 위 여백)
/// 안쪽에 그린다. 줄 안 폭과 baseline 에 앉히는 높이는 상자로 세고, ImageNode 는
/// 잉크 크기로 낸다. 실측(samples↔pdf 215문서): 사방 3.01mm 여백의 빈 문단 TAC
/// 그림이 왼쪽·양쪽 정렬에서 (−11.33, −11.22)px, 가운데 정렬에서 (0, −11.22)px
/// 어긋났다 — 가운데는 좌우 여백이 같아 x 가 상쇄된다 (hwp3-sample14-hwp5 3쪽 pi=29).
pub(crate) fn tac_picture_outer_margins_px(
    pic: &crate::model::image::Picture,
    dpi: f64,
) -> (f64, f64, f64, f64) {
    tac_object_outer_margins_px(&pic.common, dpi)
}

/// [#6606] 글자처럼 개체(그림·도형·묶음 공통)의 바깥 여백(px) — (왼쪽, 오른쪽, 위, 아래).
/// 도형도 같은 상자 규칙을 따른다: `draw-group` 의 글자처럼 묶음(좌우 3.20mm)은 자식
/// 그림 10장이 전부 왼쪽 여백만큼(−12.05px) 왼쪽에 그려졌다.
pub(crate) fn tac_object_outer_margins_px(
    common: &crate::model::shape::CommonObjAttr,
    dpi: f64,
) -> (f64, f64, f64, f64) {
    let m = &common.margin;
    (
        hwpunit_to_px(i32::from(m.left), dpi),
        hwpunit_to_px(i32::from(m.right), dpi),
        hwpunit_to_px(i32::from(m.top), dpi),
        hwpunit_to_px(i32::from(m.bottom), dpi),
    )
}

pub(crate) fn tac_picture_label_extra_for_line(
    _cell_ctx: Option<&CellContext>,
    runs_all_whitespace: bool,
    raw_line_height: f64,
    reserved_picture_height: Option<f64>,
    max_font_size: f64,
    line_spacing_px: f64,
) -> f64 {
    // #1352/#1486: "TAC picture + 실제 텍스트" 줄은 한컴 PDF 기준
    // picture와 텍스트가 같은 세로 위치에 놓인다. label 보정은 TAC-only 라인에만 남긴다.
    if !runs_all_whitespace {
        return 0.0;
    }
    tac_picture_label_extra_px(
        runs_all_whitespace,
        raw_line_height,
        reserved_picture_height,
        max_font_size,
        line_spacing_px,
    )
}

/// run 이 `\t` 로 끝날 때, 그 마지막 `\t` 가 cross-run 우측/가운데 탭으로 동작해야 하는지 판정한다.
///
/// HWP 본문 탭에는 두 가지 정보원이 있다:
/// - `tab_extended` (inline tab): `ext[2]` 고바이트 = 탭 종류 (1=LEFT, 2=RIGHT, 3=CENTER, 4=DECIMAL)
/// - `TabDef` (문단 모양의 탭 정의): 절대 위치 + type/fill
///
/// inline 이 커버하는 `\t` 는 inline 의 종류가 우선이며, LEFT 이면 cross-run 재배치 없음.
/// inline 이 비었거나 `\t` 인덱스를 초과하는 경우에만 `find_next_tab_stop` 기반 TabDef 폴백으로 판정한다.
///
/// 반환 `Some((tab_pos, tab_type, fill_type))` 은 `pending_right_tab_*` 에 그대로 대입 가능 (tab_type ∈ {1, 2}).
/// fill_type 은 호출 측에서 리더(점선/실선/파선 등) 가 있는 RIGHT 탭을 단 우측 끝으로 보정하는 용도.
#[allow(clippy::too_many_arguments)]
pub(crate) fn resolve_last_tab_pending(
    run_text: &str,
    last_inline_idx: usize,
    tab_extended: &[[u16; 7]],
    text_style: &TextStyle,
    tab_stops: &[TabStop],
    tab_width: f64,
    auto_tab_right: bool,
    available_width: f64,
) -> Option<(f64, u8, u8)> {
    // 1) inline_tabs 가 마지막 \t 를 커버하는 경우: ext[2] 고바이트로 종류 판정
    //    [#7170] 자리표는 저장 데이터가 아니다 — 커버하지 않는 것으로 보고 2)로 내려간다.
    if tab_extended
        .get(last_inline_idx)
        .is_some_and(|ext| !crate::model::paragraph::tab_ext_is_placeholder(ext))
    {
        let inline_type = ((tab_extended[last_inline_idx][2] >> 8) & 0xFF) as u8;
        match inline_type {
            // 1=LEFT (explicit), 0=unspecified → cross-run pending 없음 (본 수정의 핵심)
            0 | 1 => return None,
            // 2=RIGHT, 3=CENTER → TabDef 기반 위치 계산으로 폴스루
            2 | 3 => {}
            // 미지 값 (4=DECIMAL 등) → 보수적으로 LEFT 취급
            _ => return None,
        }
    }

    // 2) inline 이 LEFT 아님 (RIGHT/CENTER) 또는 inline 없음 → TabDef find_next_tab_stop 으로 판정
    let last_tab_byte = run_text.rfind('\t')?;
    let text_before = &run_text[..last_tab_byte];
    let w_before = estimate_text_width(text_before, text_style);
    let abs_before = text_style.line_x_offset + w_before;
    let tw = if tab_width > 0.0 { tab_width } else { 48.0 };
    let (tp, tt, ft) =
        find_next_tab_stop(abs_before, tab_stops, tw, auto_tab_right, available_width);
    if tt == 1 || tt == 2 {
        Some((tp, tt, ft))
    } else {
        None
    }
}

/// [#6844] 런 **안**에 오른쪽/가운데 탭이 있는 런의 bbox 를 **문자 위치에 맞춘다.**
///
/// `pending_right_tab_render` 는 런이 탭으로 **끝날 때**만 서므로(정렬 대상이 다음 런에
/// 있는 교차-run 형상), 한컴 목차가 흔히 쓰는 `"\t8"`(탭 + 쪽번호를 한 런에) 형상은
/// 그 경로를 안 탄다. 그런 런은 `layout_positions` 가 이미 오른쪽 정렬된 자리를 담는데
/// **bbox 폭만 `estimate_text_width` 값(탭 스톱까지)으로 남아** 글리프가 자기 상자
/// **밖**에 그려졌다.
///
/// ```text
///   30269 목차 4번째 줄   런 "\t8"  x=595.4
///     bbox   595.4 .. 673.4   (w=78.0)
///     글리프 683.9 .. 693.7   ← 상자 밖. 형제 줄들도 693.7 에서 끝난다.
/// ```
///
/// 이 런은 탭 뒤가 **가시문자로 끝나므로** 마지막 문자 경계가 곧 잉크의 끝이다. 폭을
/// 거기에 맞추면 bbox·장식·리더·문자 위치가 하나의 값을 공유한다.
///
/// 대상은 코퍼스 실측으로 좁혔다 — 우/가운데 스톱에 걸리는 **런의 마지막 탭**이고 그 런이
/// **줄의 마지막**인 형상(중간탭 9,923건 중 2,556건; 그중 998건이 글리프가 상자 밖).
///
/// ⚠ 문자 위치 자체를 옮기지는 않는다. 교차-run 경로의 `effective_pos` 변환을 그대로
/// 가져와 재정렬해 봤더니 34건이 움직였고 그중 `3142535`(별지 5 징수결정액통지서)의
/// 글자가 `x=671.4 → 1000.1` 로 **용지(793.7) 밖**으로 나갔다. 이 런들의 위치는 이미
/// 기존 기계가 정하고 있고, 그 계약은 이 이슈의 범위가 아니다.
#[allow(clippy::too_many_arguments)]
pub(crate) fn resolve_intra_run_right_tab(
    run_text: &str,
    full_width: f64,
    layout_positions: Option<Vec<f64>>,
    text_style: &TextStyle,
    tab_extended: &[[u16; 7]],
    inline_tab_base: usize,
    tab_stops: &[TabStop],
    tab_width: f64,
    auto_tab_right: bool,
    available_width: f64,
) -> (f64, Option<Vec<f64>>) {
    let chars: Vec<char> = run_text.chars().collect();
    let Some(tab_idx) = chars.iter().rposition(|c| *c == '\t') else {
        return (full_width, layout_positions);
    };
    if chars[tab_idx + 1..]
        .iter()
        .collect::<String>()
        .trim()
        .is_empty()
    {
        return (full_width, layout_positions);
    }
    // inline_tabs 가 LEFT 를 명시하면 대상이 아니다 (`resolve_last_tab_pending` 과 같은 규칙).
    let tab_ordinal = chars[..tab_idx].iter().filter(|c| **c == '\t').count();
    let inline_idx = inline_tab_base + tab_ordinal;
    // [#7170] 자리표는 저장 데이터가 아니다 — 위와 같은 규칙으로 건너뛴다.
    if tab_extended
        .get(inline_idx)
        .is_some_and(|ext| !crate::model::paragraph::tab_ext_is_placeholder(ext))
    {
        match ((tab_extended[inline_idx][2] >> 8) & 0xFF) as u8 {
            2 | 3 => {}
            _ => return (full_width, layout_positions),
        }
    }
    // ⚠ `layout_positions` 는 **그대로 돌려준다.** 소비자(`replay_positions_for`)는
    // `None` 이면 style 로 같은 배열을 다시 계산하므로, 여기서 채워 넣어도 값은 같지만
    // "positions 가 있는가"로 갈리는 하류 분기가 움직인다(골든 SVG clip 폭이
    // `642.5333333333334 → …35` 로 흔들렸다). 폭만 고친다.
    let ink_end = match layout_positions.as_deref() {
        Some(positions) if positions.len() == chars.len() + 1 => positions[chars.len()],
        None => {
            let computed = compute_char_positions(run_text, text_style);
            if computed.len() != chars.len() + 1 {
                return (full_width, layout_positions);
            }
            computed[chars.len()]
        }
        _ => return (full_width, layout_positions),
    };
    let before: String = chars[..tab_idx].iter().collect();
    let w_before = estimate_text_width(&before, text_style);
    let abs_before = text_style.line_x_offset + w_before;
    let tw = if tab_width > 0.0 { tab_width } else { 48.0 };
    let (_tab_pos, tab_type, _fill_type) =
        find_next_tab_stop(abs_before, tab_stops, tw, auto_tab_right, available_width);
    if tab_type != 1 && tab_type != 2 {
        return (full_width, layout_positions);
    }
    // 두 값이 실질적으로 같으면 손대지 않는다 — 부동소수 잡음으로 골든을 흔들지 않는다.
    if !ink_end.is_finite() || ink_end < 0.0 || (ink_end - full_width).abs() <= 0.05 {
        return (full_width, layout_positions);
    }
    (ink_end, layout_positions)
}

/// 우측/가운데 탭 정렬 단위의 폭(px).
///
/// 탭 직후 run(`start`)부터 `\t` 를 포함하지 않는 연속 run 들의 `estimate_text_width` 합산.
/// composer(`split_runs_by_lang` / `split_by_char_shapes`)가 char-shape·스크립트 경계로 run 을
/// 쪼개므로(예: `"Ctrl+(회색)5"` → `["Ctrl+(", "회색)", "5"]`), 탭 직후 한 개 run 폭만 쓰면
/// 나머지 run 이 탭스톱 우측으로 흘러넘친다 (Issue #842, 결함 #4).
#[allow(clippy::too_many_arguments)]
pub(crate) fn right_tab_block_width(
    runs: &[crate::renderer::composer::ComposedTextRun],
    start: usize,
    styles: &ResolvedStyleSet,
    default_tab_width: f64,
    tab_stops: &[TabStop],
    auto_tab_right: bool,
    available_width: f64,
) -> f64 {
    let mut w = 0.0;
    for r in runs.iter().skip(start) {
        if r.text.contains('\t') {
            break;
        }
        if let Some(_ov) = &r.char_overlap {
            let chars: Vec<char> = r.text.chars().collect();
            let fs = {
                let ts = r.text_style(styles);
                if ts.font_size > 0.0 {
                    ts.font_size
                } else {
                    12.0
                }
            };
            w += fs * crate::renderer::composer::char_overlap_advance_units(&chars) as f64;
            continue;
        }
        let mut ts = r.text_style(styles);
        ts.default_tab_width = default_tab_width;
        ts.tab_stops = tab_stops.to_vec();
        ts.auto_tab_right = auto_tab_right;
        ts.available_width = available_width;
        // [Task #874] text_start_offset 은 right_tab_block_width 가 측정만 하므로
        // 영향 없음 — 0 그대로.
        w += estimate_text_width(effective_text_for_metrics(r), &ts);
    }
    w
}

/// [Issue #6179] 오른쪽 탭 뒤에 오는 **자리차지(TAC) 개체**까지 포함한 정렬 블록 폭.
///
/// `auto_tab_right` 오른쪽 탭은 "탭 뒤 블록의 **오른쪽 변**을 우단에 맞춘다"는 뜻이고,
/// 그 되밀기 폭은 `text_measurement` 가 탭 뒤 **글자**만 재서 구한다. 그런데 run 은
/// TAC 개체 위치에서 조각으로 쪼개져 측정되므로, 탭 바로 뒤가 개체면 측정 대상 조각에
/// 남는 글자가 없어 되밀기 폭이 0 이 된다 → 개체의 **왼쪽** 변이 우단에 놓여, 개체는
/// 정확히 제 폭만큼 우측(용지 밖)으로 밀린다.
///
/// 여기서 탭 뒤 잔여 글자 폭 + 탭 뒤 TAC 개체 폭을 합해
/// `right_tab_block_width_override` 로 주입한다. 탭 뒤에 또 탭이 있으면
/// (`has_more_tabs_after`) 측정 쪽이 override 를 쓰지 않으므로 `None` 을 돌려준다.
///
/// - `run_chars`: run 전체 문자열 (조각이 아니라 run 단위 — 탭 뒤 잔여가 다음 조각에
///   있을 수 있다)
/// - `tab_rel`: run 안 마지막 탭의 문자 인덱스
/// - `run_tacs`: run 안 TAC 목록 `(rel_pos, width_px, control_index)`
pub(crate) fn right_tab_block_width_with_tac(
    run_chars: &[char],
    tab_rel: usize,
    run_tacs: &[(usize, f64, usize)],
    style: &TextStyle,
) -> Option<f64> {
    if run_chars[tab_rel + 1..].contains(&'\t') {
        return None;
    }
    let tac_w: f64 = run_tacs
        .iter()
        .filter(|(rel, _, _)| *rel > tab_rel)
        .map(|(_, w, _)| *w)
        .sum();
    if tac_w <= 0.0 {
        return None;
    }
    let tail: String = run_chars[tab_rel + 1..].iter().collect();
    let mut ts = style.clone();
    ts.right_tab_block_width_override = None;
    Some(estimate_text_width(&tail, &ts) + tac_w)
}

/// [#6303] 칸 폭 자동 축소(#6196) 셀의 오버플로우 자간을 안쪽 폭에 수렴시킨다.
///
/// 저장 사다리가 한 줄·안쪽 폭으로 적어 둔 칸이 자연 폭에서 안쪽 폭을 15% 넘게
/// 놓친 경우에만 쓴다. 선형 1회 `slack/N` 은 말미 글자·narrow glyph 클램프 때문에
/// 목표보다 1~2% 헐겁다. 일반 문단·일반 셀의 자간은 그대로 둔다.
pub(crate) fn converge_cell_overflow_char_spacing(
    comp_line: &ComposedLine,
    styles: &ResolvedStyleSet,
    tab_width: f64,
    total_char_count: usize,
    total_text_width: f64,
    available_width: f64,
) -> f64 {
    let avg_char_w = total_text_width / total_char_count as f64;
    let min_sp = -avg_char_w * 0.5;
    let mut extra = ((available_width - total_text_width) / total_char_count as f64).max(min_sp);
    for _ in 0..4 {
        let mut measured = 0.0f64;
        for run in &comp_line.runs {
            let mut ts = run.text_style(styles);
            ts.default_tab_width = tab_width;
            ts.extra_char_spacing = extra;
            measured += estimate_text_width(&run.text, &ts);
        }
        let delta = available_width - measured;
        if delta >= -0.05 && delta.abs() < 0.25 {
            break;
        }
        extra = (extra + delta / total_char_count as f64).max(min_sp);
    }
    extra.min(0.0)
}

/// Natural trailing advances excluded by word justification. Use the same
/// unrounded measurement as line/run placement and retain each suffix run's
/// style. Applying the last run's style to the entire suffix changes the
/// visible right edge when trailing spaces cross a character-style boundary.
pub(crate) fn justified_trailing_space_width(
    line: &ComposedLine,
    mut remaining: usize,
    styles: &ResolvedStyleSet,
    tab_width: f64,
) -> f64 {
    let mut width = 0.0;
    for run in line.runs.iter().rev() {
        if remaining == 0 {
            break;
        }
        let text = effective_text_for_metrics(run);
        let count = text
            .chars()
            .rev()
            .take_while(|c| *c == ' ')
            .count()
            .min(remaining);
        let mut style = run.text_style(styles);
        style.default_tab_width = tab_width;
        width += estimate_text_width_exact(&" ".repeat(count), &style);
        remaining -= count;
        if count < text.chars().count() {
            break;
        }
    }
    width
}

/// [Task #2067] 정렬(양쪽/배분/나눔)·오버플로우·셀 underflow 에 따른 여분 간격 계산.
/// 반환 = (extra_word_sp, extra_char_sp, extra_dash_sp). Task #352 dash leader 분배 포함.
#[allow(clippy::too_many_arguments)]
pub(crate) fn compute_line_extra_spacing(
    comp_line: &ComposedLine,
    trailing_space_limit: usize,
    styles: &ResolvedStyleSet,
    alignment: Alignment,
    in_cell: bool,
    needs_justify: bool,
    // [#6443] 양쪽정렬이 **일부러 제외한** 마지막 줄인가 (Justify 문단의 마지막 줄).
    is_excluded_justify_last_line: bool,
    justify_spaces_only: bool,
    needs_distribute: bool,
    has_tabs: bool,
    renders_synthetic_wrap_trailing_space: bool,
    suppress_cell_overflow_spacing: bool,
    converge_auto_shrink_cell: bool,
    total_char_count: usize,
    total_text_width: f64,
    available_width: f64,
    tab_width: f64,
) -> (f64, f64, f64) {
    // 음수 자간은 마지막 글자의 advance도 줄이지만 실제 glyph 잉크 폭은 줄이지 않는다.
    // 나눔정렬에서 advance만 셀 끝에 맞추면 정상 폭으로 그린 마지막 glyph가 clip을
    // 넘어가므로, 마지막 가시 글자의 음수 자간만 시각 점유 폭에 되돌린다.
    let trailing_glyph_ink_overhang = || -> f64 {
        for run in comp_line.runs.iter().rev() {
            if let Some(last_visible) = run.text.chars().rev().find(|c| *c != ' ') {
                if last_visible == '\t' || last_visible == '\u{FFFC}' {
                    return 0.0;
                }
                let mut with_spacing = run.text_style(styles);
                with_spacing.default_tab_width = tab_width;
                if with_spacing.letter_spacing >= 0.0 {
                    return 0.0;
                }
                let glyph = last_visible.to_string();
                let spaced_width = estimate_text_width(&glyph, &with_spacing);
                with_spacing.letter_spacing = 0.0;
                let ink_advance = estimate_text_width(&glyph, &with_spacing);
                return (ink_advance - spaced_width).max(0.0);
            }
        }
        0.0
    };

    // Task #352: 라인 내 dash leader (3+ 연속 '-') 글자 수 카운트.
    // visible_count 까지의 chars 에서만 카운트 (후행 공백 제외).
    let count_dash_leaders = |chars: &[char]| -> usize {
        let mut count = 0;
        let n = chars.len();
        let mut i = 0;
        while i < n {
            if chars[i] == '-' {
                let mut j = i;
                while j < n && chars[j] == '-' {
                    j += 1;
                }
                let run_len = j - i;
                if run_len >= 3 {
                    count += run_len;
                }
                i = j;
            } else {
                i += 1;
            }
        }
        count
    };

    // [#5830] 양쪽정렬 배분 대상이 아닌 줄(문단 마지막 줄·강제 줄바꿈 줄)의 dash leader.
    //
    // 종전에는 이 줄들에서 슬랙 자체가 계산되지 않아 `char_width_decision` 의 leader
    // 클램프 `min(자연폭, font_size * 0.3)` 에 머물렀다 — 한글 2022 정본 대비 폭 절반.
    //
    // 정본(86712 규제영향분석서 p34·p35, PDF 글리프 원점 실측)의 마지막 줄 규칙:
    //   - 여백이 충분하면 dash 는 **자연 폭**으로 그린다 (p35 10자·18자 런 = 8.00pt
    //     = 0.571em, 오른쪽 여백에 닿지 않고 끝난다 — 무한 신장이 아니다).
    //   - 여백이 그보다 좁으면 **여백까지만** 좁힌다 (p34 10자 런 = 7.00pt = 0.499em,
    //     끝점이 정확히 여백 x≈530pt).
    // 즉 마지막 줄에서는 leader 클램프를 **자연 폭 한도 안에서 슬랙만큼** 되돌린다.
    // needs_justify 줄의 기존 탄력 흡수(Task #352, 여백까지 확장)는 그대로다.
    //
    // 정렬이 여백까지 채우는 종류(Justify·Split)일 때만 연다 — 왼쪽/가운데 정렬의
    // 짧은 dash 는 저자가 의도한 길이일 수 있다.
    let last_line_leader_fill = if !needs_justify
        && matches!(alignment, Alignment::Justify | Alignment::Split)
    {
        let all_chars: Vec<char> = comp_line.runs.iter().flat_map(|r| r.text.chars()).collect();
        let trailing_spaces = all_chars
            .iter()
            .rev()
            .take_while(|c| **c == ' ')
            .count()
            .min(trailing_space_limit);
        let visible_count = all_chars.len() - trailing_spaces;
        let leader_dashes = count_dash_leaders(&all_chars[..visible_count]);
        if leader_dashes > 0 {
            // 클램프가 깎아낸 폭 = 자연 advance − min(자연, 0.3em). 단독 '-' 는 3+ 연속
            // leader 가 아니므로 estimate_text_width 가 클램프 없는 자연 폭을 돌려준다.
            let per_dash_restore = comp_line
                .runs
                .iter()
                .find(|r| {
                    let chars: Vec<char> = r.text.chars().collect();
                    (0..chars.len()).any(|i| {
                        chars[i] == '-' && chars[i..].iter().take_while(|c| **c == '-').count() >= 3
                    })
                })
                .map(|r| {
                    let mut ts = r.text_style(styles);
                    ts.default_tab_width = tab_width;
                    let natural = estimate_text_width("-", &ts);
                    (natural - natural.min(ts.font_size * 0.3)).max(0.0)
                })
                .unwrap_or(0.0);
            let trailing_width = if trailing_spaces > 0 {
                if let Some(last_run) = comp_line.runs.last() {
                    let mut ts = last_run.text_style(styles);
                    ts.default_tab_width = tab_width;
                    estimate_text_width(&" ".repeat(trailing_spaces), &ts)
                } else {
                    0.0
                }
            } else {
                0.0
            };
            let slack = available_width - (total_text_width - trailing_width);
            // 슬랙이 없으면(이미 꽉 찬 줄) 아래 기존 분기(오버플로우 압축 등)로 흘린다.
            let extra = (slack / leader_dashes as f64).min(per_dash_restore);
            (extra > 0.0).then_some(extra)
        } else {
            None
        }
    } else {
        None
    };

    if needs_justify {
        // 양쪽 정렬: 후행 공백 제외한 내부 공백에 분배.
        //
        // [#5899] 공백은 **그려지는 텍스트**로 센다. `extra_word_spacing` 은
        // text_measurement 가 표시 텍스트의 공백마다 붙이므로, 모델 텍스트(`run.text`)
        // 로 세면 분모(내부 공백 수)와 실제 적용 대상이 어긋난다. 머리말/꼬리말
        // 쪽번호 필드는 #3216 규약대로 모델 1자(공백 placeholder)를 유지하고
        // `display_text` 만 번호로 바꾸므로, 모델로 세면 `… Inc.` + 공백 75개로
        // **끝나는 줄**로 보여 슬랙이 내부 공백 2개에만 나뉜다. 그 여분(262.9px)이
        // 표시 텍스트의 공백 76개 전부에 붙어 쪽번호가 종이 밖 x≈20,163px 로
        // 밀려났다. 폭(`total_text_width`)·글자수(`total_char_count`)는 이미 표시
        // 텍스트 기준이라 여기만 축이 달랐다.
        let all_chars: Vec<char> = comp_line
            .runs
            .iter()
            .flat_map(|r| effective_text_for_metrics(r).chars())
            .collect();
        let trailing_spaces = all_chars
            .iter()
            .rev()
            .take_while(|c| **c == ' ')
            .count()
            .min(trailing_space_limit);
        let visible_count = all_chars.len() - trailing_spaces;
        let interior_spaces = all_chars[..visible_count]
            .iter()
            .filter(|c| **c == ' ')
            .count();
        let leader_dashes = count_dash_leaders(&all_chars[..visible_count]);
        if interior_spaces > 0 {
            let trailing_width =
                justified_trailing_space_width(comp_line, trailing_spaces, styles, tab_width);
            let split_ink_overhang = if alignment == Alignment::Split {
                trailing_glyph_ink_overhang()
            } else {
                0.0
            };
            // A fresh soft-wrap consumes the separator before starting the next
            // row, but LineSeg can encode only the next row's start. Composition
            // therefore leaves that separator at the end of this row. The run
            // painter advances every preserved space, so its distribution must
            // account for that slot and its natural width; otherwise a corrected
            // break immediately before a word can push the trailing separator
            // beyond the line box (#4956).
            let rendered_space_slots = if renders_synthetic_wrap_trailing_space {
                interior_spaces + trailing_spaces
            } else {
                interior_spaces
            };
            let effective_used = if renders_synthetic_wrap_trailing_space {
                total_text_width + split_ink_overhang
            } else {
                total_text_width - trailing_width + split_ink_overhang
            };
            let slack = available_width - effective_used;
            if leader_dashes > 0 && slack > 0.0 {
                // Task #352: 라인에 dash leader 가 있고 슬랙이 양수면
                // dash 가 흡수 (PDF elastic leader 동작 모방). 공백·일반
                // 글자 자연 폭 유지.
                (0.0, 0.0, slack / leader_dashes as f64)
            } else if suppress_cell_overflow_spacing && slack < 0.0 {
                // 셀 내부 폭이 글자 자연 폭보다 작아도 한컴처럼 글자를 압축하지 않는다.
                // 줄바꿈은 LINE_SEG/리플로우가 결정하고, 그린 글자는 셀 경계에서만 클리핑한다.
                (0.0, 0.0, 0.0)
            } else {
                // 양쪽 정렬: 단어 간격 분배 (또는 음수 슬랙 시 압축)
                let raw_ews = slack / rendered_space_slots as f64;
                let space_base_w = estimate_text_width(
                    " ",
                    &resolved_to_text_style(
                        styles,
                        comp_line.runs[0].char_style_id,
                        comp_line.runs[0].lang_index,
                    ),
                );
                let min_ews = -(space_base_w * 0.5);
                let ews = raw_ews.max(min_ews);
                // [Task #2189] 저장 줄바꿈(LINE_SEG) 셀에서 대체 폰트 advance 가 한컴
                // 실폰트보다 넓으면 공백 -50% 클램프만으로는 잔여 초과가 남아 우측
                // 테두리에서 클리핑된다. 공백-없는 분기와 동일하게 잔여 음수 슬랙을
                // 자간으로 흡수한다 (narrow glyph 역진은 #229 per-char 클램프가 방어).
                let leftover = slack - ews * rendered_space_slots as f64;
                let ecs = if in_cell && leftover < 0.0 && total_char_count > 1 && !has_tabs {
                    let avg_char_w = total_text_width / total_char_count as f64;
                    let min_ecs = -avg_char_w * 0.5;
                    let mut ecs = (leftover / total_char_count as f64).max(min_ecs);
                    // narrow glyph per-char 클램프가 음수 자간 기여 일부를 되돌리므로
                    // 선형 1회 분배로는 부족하다 — underflow 확장과 동일하게 실효 폭
                    // 재측정으로 수렴 반복한다.
                    let measure_with = |ecs: f64| -> f64 {
                        let mut measured = 0.0f64;
                        for r in &comp_line.runs {
                            let mut ts = r.text_style(styles);
                            ts.default_tab_width = tab_width;
                            ts.extra_word_spacing = ews;
                            ts.extra_char_spacing = ecs;
                            measured += estimate_text_width(&r.text, &ts);
                        }
                        if trailing_spaces > 0 {
                            if let Some(last_run) = comp_line.runs.last() {
                                let mut ts = last_run.text_style(styles);
                                ts.default_tab_width = tab_width;
                                ts.extra_word_spacing = ews;
                                ts.extra_char_spacing = ecs;
                                measured -= estimate_text_width(&" ".repeat(trailing_spaces), &ts);
                            }
                        }
                        measured
                    };
                    for _ in 0..3 {
                        let delta = available_width - measure_with(ecs);
                        if delta.abs() < 0.5 {
                            break;
                        }
                        ecs = (ecs + delta / total_char_count as f64).max(min_ecs);
                    }
                    ecs.min(0.0)
                } else {
                    0.0
                };
                (ews, ecs, 0.0)
            }
        } else if total_char_count > 1 {
            // 양쪽 정렬이지만 공백 없음 (일본어/숫자 등):
            let slack = available_width - total_text_width;
            if justify_spaces_only && slack > 0.0 {
                // [#4516] 머리말/꼬리말 예외로만 justify 된 마지막 줄은 한컴처럼
                // **공백만** 벌린다. 공백 없는 줄(영문 문서번호 등)에 양수 slack 을
                // 자간으로 살포하면 글자가 전체 폭으로 흩어지므로 자연 폭 유지.
                (0.0, 0.0, 0.0)
            } else if leader_dashes > 0 && slack > 0.0 {
                (0.0, 0.0, slack / leader_dashes as f64)
            } else if suppress_cell_overflow_spacing && slack < 0.0 {
                // 셀의 좁은 내부 폭은 줄바꿈 기준일 뿐, 숫자/문자를 수평 압축하지 않는다.
                (0.0, 0.0, 0.0)
            } else if converge_auto_shrink_cell && slack < 0.0 {
                (
                    0.0,
                    converge_cell_overflow_char_spacing(
                        comp_line,
                        styles,
                        tab_width,
                        total_char_count,
                        total_text_width,
                        available_width,
                    ),
                    0.0,
                )
            } else {
                let raw = slack / total_char_count as f64;
                let avg_char_w = total_text_width / total_char_count as f64;
                let min_sp = -avg_char_w * 0.5;
                (0.0, raw.max(min_sp), 0.0)
            }
        } else {
            (0.0, 0.0, 0.0)
        }
    } else if let Some(extra_dash) = last_line_leader_fill {
        // [#5830] 마지막 줄·강제 줄바꿈 줄의 dash leader 채움.
        (0.0, 0.0, extra_dash)
    } else if needs_distribute && total_char_count > 1 {
        // [#4657] 배분 정렬: 남는 폭을 글자 **사이**(N-1곳)에 균등 분배.
        // extra_char_spacing 은 각 글자 advance 뒤에 붙으므로 마지막 glyph 의
        // 잉크 오른쪽 끝은 `W + (N-1)·extra` — N 으로 나누면 짧은 줄일수록
        // 마지막 글자가 slack/N 만큼 안쪽으로 밀려 문단마다 오른쪽 끝이
        // 어긋난다(한컴은 줄 길이와 무관하게 오른쪽 끝을 문단 폭에 맞춘다).
        // 말미 공백은 보이는 글자가 아니므로 분배 대상과 기준 폭에서 제외한다.
        let trailing_spaces = comp_line
            .runs
            .iter()
            .rev()
            .flat_map(|r| r.text.chars().rev())
            .take_while(|c| *c == ' ')
            .count()
            .min(trailing_space_limit);
        let visible_count = total_char_count.saturating_sub(trailing_spaces);
        if visible_count <= 1 {
            (0.0, 0.0, 0.0)
        } else {
            let trailing_width = if trailing_spaces > 0 {
                if let Some(last_run) = comp_line.runs.last() {
                    let mut ts = last_run.text_style(styles);
                    ts.default_tab_width = tab_width;
                    estimate_text_width(&" ".repeat(trailing_spaces), &ts)
                } else {
                    0.0
                }
            } else {
                0.0
            };
            let visible_width = total_text_width - trailing_width;
            let raw = (available_width - visible_width) / (visible_count - 1) as f64;
            if suppress_cell_overflow_spacing && raw < 0.0 {
                (0.0, 0.0, 0.0)
            } else {
                let avg_char_w = visible_width / visible_count as f64;
                let min_sp = -avg_char_w * 0.5;
                (0.0, raw.max(min_sp), 0.0)
            }
        }
    } else if total_text_width > available_width && total_char_count > 1 && !has_tabs {
        // 비정렬(왼쪽/오른쪽/가운데) 텍스트가 오버플로우할 때 글자 간격 압축
        if suppress_cell_overflow_spacing {
            (0.0, 0.0, 0.0)
        } else if converge_auto_shrink_cell {
            // [#6303] 칸 폭 자동 축소(#6196) 가 선형 slack/N 한 번이면 목표가
            // 1~2% 헐거워 긴 행 꼬리가 괘선 밖으로 나간다. 저장 한 줄이 안쪽 폭을
            // 15% 넘게 넘는 칸에서만 줄바꿈은 그대로 두고 실측 폭을 수렴시킨다.
            // 일반 in_cell 줄까지 수렴하면 page-local hash·text-overlap 이 흔들린다.
            (
                0.0,
                converge_cell_overflow_char_spacing(
                    comp_line,
                    styles,
                    tab_width,
                    total_char_count,
                    total_text_width,
                    available_width,
                ),
                0.0,
            )
        } else {
            let raw = (available_width - total_text_width) / total_char_count as f64;
            let avg_char_w = total_text_width / total_char_count as f64;
            let min_sp = -avg_char_w * 0.5;
            (0.0, raw.max(min_sp), 0.0)
        }
    } else if in_cell
        && total_char_count > 1
        && !has_tabs
        && alignment != Alignment::Left
        && total_text_width < available_width
        && total_text_width > 0.0
        && comp_line.runs.iter().any(|r| {
            let ts = r.text_style(styles);
            ts.letter_spacing < -0.01
        })
        && {
            // 자연 폭(letter_spacing=0)이 셀 inner 폭보다 커야만 "문서가
            // 셀에 맞추기 위해 음수 자간으로 압축했던" 케이스로 간주. 그렇지
            // 않으면 음수 자간은 장식적 의도이므로 기존 동작(natural width
            // 그대로, 좌우 여백 유지)을 유지한다.
            let natural_w: f64 = comp_line
                .runs
                .iter()
                .map(|r| {
                    let mut ts = r.text_style(styles);
                    ts.default_tab_width = tab_width;
                    ts.letter_spacing = 0.0;
                    estimate_text_width(&r.text, &ts)
                })
                .sum();
            natural_w > available_width
        }
        // [#6443] 양쪽정렬이 **일부러 제외한** 마지막 줄은 여기서도 늘리지 않는다.
        //
        // `needs_word_distribution` 은 Justify 문단의 마지막 줄을 분배에서 뺀다. 그런데
        // 이 규칙이 같은 줄을 칸 폭까지 되늘리면 두 규칙이 서로를 무효화한다 —
        // 3123751 8쪽 `산 출 내 역` 열(한 줄짜리 Justify 문단, 자간 −16%)이 그 예로,
        // 한글은 괘선 22pt 안쪽에서 멈추는데 rhwp 만 괘선까지 채웠다
        // (글자 전진 한글 8.40pt vs rhwp 8.95pt).
        && !is_excluded_justify_last_line
    {
        // 표 셀 내부 underflow: HWP 편집기가 자연 폭이 셀을 넘는 텍스트를
        // 음수 자간으로 셀 폭에 맞춰 저장했으므로, 재렌더 시 우리 폰트
        // 메트릭으로 좁게 측정되더라도 셀 폭을 채우도록 자간을 양수로 보정.
        //
        // narrow glyph per-char 클램프가 개입하면 선형 분배와 실제 렌더 폭이
        // 어긋나므로 수렴 반복으로 보정한다.
        let mut extra = (available_width - total_text_width) / total_char_count as f64;
        for _ in 0..3 {
            let mut measured = 0.0f64;
            for r in &comp_line.runs {
                let mut ts = r.text_style(styles);
                ts.default_tab_width = tab_width;
                ts.extra_char_spacing = extra;
                measured += estimate_text_width(&r.text, &ts);
            }
            let delta = available_width - measured;
            if delta.abs() < 0.5 {
                break;
            }
            extra += delta / total_char_count as f64;
        }
        (0.0, extra, 0.0)
    } else {
        (0.0, 0.0, 0.0)
    }
}

/// `한글 97 안내문` 머리말의 한컴 회사명 PUA 여섯 글자와 뒤따르는 inline
/// logo 그림은, HWPX상 `DISTRIBUTE_SPACE` 문단 하나로 저장돼 있다.
///
/// 한컴은 회사명 내부 글자에는 나눔 자간을 넣지 않고, 그 뒤 공백 하나로 logo를
/// 오른쪽에 보낸다. 일반 `DISTRIBUTE_SPACE` 규칙(모든 글자에 자간 분배)을 그대로
/// 적용하면 PUA가 공개 글꼴에서 tofu로 보일 뿐 아니라, 표준 한글로 투영한 뒤에도
/// `한 글 과 컴 퓨 터`처럼 흩어진다. header/footer 내부 문단은 원문 문단 번호를
/// 재사용하므로 호출부의 header 플래그에 의존하지 않고, 확인된 **완전한** 원문
/// 시퀀스와 정렬값만으로 좁게 판별한다.
pub(crate) fn is_hancom_company_pua_logo_line(
    comp_line: &ComposedLine,
    alignment: Alignment,
) -> bool {
    // OWPML `DISTRIBUTE_SPACE`는 parser에서 `Split`(공백에만 나눔)으로
    // 보존한다. `Distribute`는 글자마다 나눔인 별도 값이다.
    if alignment != Alignment::Split {
        return false;
    }

    let raw: String = comp_line.runs.iter().map(|run| run.text.as_str()).collect();
    let company_pua = "\u{F03EF}\u{F03F0}\u{F03F1}\u{F03F2}\u{F03F3}\u{F03F4}";
    raw == format!("{company_pua} ")
}

/// 문단 정렬이 현재 줄의 공백 폭을 끝까지 배분해야 하는지 판정한다.
///
/// `Justify`는 마지막 줄과 강제 줄바꿈 줄을 제외하지만, HWP5 `Split`
/// (HWPX `DISTRIBUTE_SPACE`, 한컴 UI의 나눔 정렬)은 문단의 마지막 줄까지
/// 공백에 배분한다. 강제 줄바꿈 줄의 기존 억제 동작은 유지한다.
pub(crate) fn needs_word_distribution(
    alignment: Alignment,
    is_last_line_of_para: bool,
    has_forced_break: bool,
) -> bool {
    match alignment {
        Alignment::Split => !has_forced_break,
        Alignment::Justify => !is_last_line_of_para && !has_forced_break,
        _ => false,
    }
}

/// [Task #2067] 조판부호 모드의 인라인 컨트롤 마커 라벨 수집 — (논리 위치, 라벨).
pub(crate) fn collect_shape_marker_labels(
    show_ctrl: bool,
    para: Option<&Paragraph>,
) -> Vec<(usize, String)> {
    if show_ctrl {
        if let Some(ref pa) = para {
            let ctrl_positions = pa.logical_control_positions();
            pa.controls
                .iter()
                .enumerate()
                .filter_map(|(ci, ctrl)| {
                    let pos = ctrl_positions.get(ci).copied().unwrap_or(0);
                    match ctrl {
                        Control::Shape(s) => Some((pos, format!("[{}]", s.shape_name()))),
                        Control::Picture(_) => Some((pos, "[그림]".to_string())),
                        Control::Table(t) if t.common.treat_as_char => {
                            Some((pos, "[표]".to_string()))
                        }
                        Control::PageHide(_) => Some((pos, "[감추기]".to_string())),
                        Control::PageNumberPos(_) => Some((pos, "[쪽 번호 위치]".to_string())),
                        Control::Header(h) => {
                            let apply = match h.apply_to {
                                crate::model::header_footer::HeaderFooterApply::Both => "양 쪽",
                                crate::model::header_footer::HeaderFooterApply::Even => "짝수 쪽",
                                crate::model::header_footer::HeaderFooterApply::Odd => "홀수 쪽",
                            };
                            Some((pos, format!("[머리말({})]", apply)))
                        }
                        Control::Footer(f) => {
                            let apply = match f.apply_to {
                                crate::model::header_footer::HeaderFooterApply::Both => "양 쪽",
                                crate::model::header_footer::HeaderFooterApply::Even => "짝수 쪽",
                                crate::model::header_footer::HeaderFooterApply::Odd => "홀수 쪽",
                            };
                            Some((pos, format!("[꼬리말({})]", apply)))
                        }
                        Control::Footnote(_) => Some((pos, "[각주]".to_string())),
                        Control::Endnote(_) => Some((pos, "[미주]".to_string())),
                        Control::NewNumber(_) => Some((pos, "[새 번호]".to_string())),
                        Control::Bookmark(bm) => Some((pos, format!("[책갈피:{}]", bm.name))),
                        _ => None,
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    }
}

/// [#5677] 빈 문단의 저장 `column_start` 가 그 문단 자신의 `margin_left` 와 같다고 볼
/// 허용치 (HWPUNIT, 2pt). 발행 시 기하 pitch(`snap_base_left`)가 몇 HWPUNIT 을 올릴 수
/// 있어 정확 일치를 요구하지 않는다. 진짜 어울림 배제는 이보다 훨씬 크게 벌어진다.
pub(crate) const EMPTY_LINE_OWN_MARGIN_TOLERANCE_HU: i32 = 200;

pub(crate) fn make_picture_image_node(
    tree: &mut PageLayoutContext,
    pic: &crate::model::image::Picture,
    section_index: usize,
    para_index: usize,
    ctrl_idx: usize,
    cell_ctx: Option<&CellContext>,
    crop: Option<(i32, i32, i32, i32)>,
    original_size_hu: Option<(u32, u32)>,
    bin_data_id: u16,
    image_data: Option<Vec<u8>>,
    bbox: BoundingBox,
) -> RenderNode {
    let (cei, cpi, otci) = cell_ctx
        .map(|c| c.last_image_indices())
        .unwrap_or((None, None, None));
    let para_for_image = cell_ctx.map(|c| c.parent_para_index).unwrap_or(para_index);
    let img_id = tree.next_id();
    RenderNode::new(
        img_id,
        RenderNodeType::Image(ImageNode {
            section_index: Some(section_index),
            para_index: Some(para_for_image),
            control_index: Some(ctrl_idx),
            cell_index: cei,
            cell_para_index: cpi,
            outer_table_control_index: otci,
            // [Task #1161] 전체 다단계 경로 보존(스칼라는 위 innermost 투영).
            cell_context: cell_ctx.cloned(),
            crop,
            original_size_hu,
            effect: pic.image_attr.effect,
            brightness: pic.image_attr.brightness,
            contrast: pic.image_attr.contrast,
            opacity: pic.image_attr.opacity(),
            // Inline glyphs stay in flow even if the saved object retains a
            // floating wrap mode. Otherwise a textbox fill covers its pictures.
            text_wrap: (!pic.common.treat_as_char).then_some(pic.common.text_wrap),
            transform: extract_shape_transform(&pic.shape_attr),
            external_path: pic.image_attr.external_path.clone(),
            content_inset: crate::renderer::layout::utils::picture_content_inset(pic),
            ..ImageNode::new(bin_data_id, image_data)
        }),
        bbox,
    )
}

pub(crate) fn split_guide_text_to_width<'a>(
    guide: &'a str,
    style: &TextStyle,
    limit: f64,
) -> Vec<&'a str> {
    if limit <= 0.0 || estimate_text_width(guide, style) <= limit {
        return vec![guide];
    }
    let mut chunks = Vec::new();
    let mut rest = guide;
    while !rest.is_empty() {
        let mut end = rest.len();
        let mut cut = None;
        for (idx, _) in rest.char_indices().skip(1) {
            if estimate_text_width(&rest[..idx], style) > limit {
                end = idx;
                break;
            }
            cut = Some(idx);
        }
        // 폭 안에 들어가는 마지막 경계까지 자른다. 한 글자도 안 들어가면 한 글자.
        let take = match cut {
            Some(idx) if idx < rest.len() => idx,
            _ => end,
        };
        let take = if take == 0 {
            rest.char_indices()
                .nth(1)
                .map(|(i, _)| i)
                .unwrap_or(rest.len())
        } else {
            take
        };
        let (head, tail) = rest.split_at(take);
        chunks.push(head);
        rest = tail;
        if chunks.len() > 64 {
            chunks.push(rest);
            break;
        }
    }
    chunks
}

pub(crate) fn form_color_to_css(color: u32) -> String {
    let b = (color >> 16) & 0xFF;
    let g = (color >> 8) & 0xFF;
    let r = color & 0xFF;
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

pub(crate) fn calc_sibling_topandbottom_reserved_hu(
    controls: &[crate::model::control::Control],
) -> i32 {
    use crate::model::control::Control;
    use crate::model::shape::TextWrap;
    controls
        .iter()
        .map(|c| match c {
            Control::Table(t)
                if matches!(t.common.text_wrap, TextWrap::TopAndBottom)
                    && !t.common.treat_as_char =>
            {
                t.common.height as i32 + t.outer_margin_top as i32 + t.outer_margin_bottom as i32
            }
            Control::Picture(p)
                if matches!(p.common.text_wrap, TextWrap::TopAndBottom)
                    && !p.common.treat_as_char =>
            {
                p.common.height as i32 + p.common.margin.top as i32 + p.common.margin.bottom as i32
            }
            Control::Shape(s)
                if matches!(s.common().text_wrap, TextWrap::TopAndBottom)
                    && !s.common().treat_as_char =>
            {
                let common = s.common();
                common.height as i32 + common.margin.top as i32 + common.margin.bottom as i32
            }
            _ => 0,
        })
        .sum()
}

pub fn map_pua_bullet_char(ch: char) -> char {
    let code = ch as u32;

    // Supplementary PUA-A 저영역 — 한컴 자체 영역 (Task #588 한컴 정답지 정합)
    if (0xF0000..=0xF00CF).contains(&code) {
        return match code {
            // exam_eng.hwp p7 #40 요약형 문항 글상자 사이 화살표.
            // 한컴 PDF (HCRBatang 임베디드 폰트) 글리프 외곽 분석:
            //   stem 35% × arrowhead 100% × solid filled (1 contour, 7 pts) → ↓
            0xF003B => '\u{2193}', // ↓ DOWNWARDS ARROW
            _ => ch,
        };
    }

    // Supplementary PUA-A — 한컴 자체 영역 (Task #509 한컴 정답지 정합)
    if (0xF02B0..=0xF02FF).contains(&code) {
        return match code {
            // 캡스톤 F-1 (2026-05-16): U+F02B1~F02C4 사각 안 숫자 한컴 자체 PUA 글리프.
            // 한글 2024 복사 + PowerShell 디코딩으로 "사각 안 1" = 0xF02B1 확정.
            // 이전 표준 U+2460-U+2473 매핑 (Task #509 mel-001 영역) 은 fallback chain 효과
            // 못 받음 — 매핑 결과 표준 ① 가 1순위 폰트 (맑은 고딕 등) 의 원 안 글리프로
            // 즉시 렌더링 (글리프 단위 fallback 작동 안 함). raw PUA passthrough +
            // generic_fallback() 의 함초롬바탕 확장B 등이 PUA 영역 글리프 (사각 안) 매칭.
            // 두 대상 파일 (HWPX 스마트행정팀, HWP 공직기강) 모두 같은 PUA, 한컴 동일 글리프.
            //
            // KTX 회귀 origin — 한컴 PDF 시각 = · (Middle dot), ★ 아님
            // (작업지시자 정정 — 이전 ★ U+2605 매핑은 잘못)
            0xF02EF => '\u{00B7}', // · Middle dot
            _ => ch,
        };
    }

    // Supplementary PUA-A — 한컴 책괄호 / 예시 마커 (Task #528 exam_kor p17)
    // exam_kor p17 측정: F0854/F0855 각 33회 (책 제목 둘러싸기), F00DA 2회
    if (0xF00D0..=0xF09FF).contains(&code) {
        return match code {
            // 책괄호 (한국어 도서 제목) — 용비어천가, 석보상절, 월인천강지곡 등
            0xF0854 => '\u{300A}', // 《 LEFT DOUBLE ANGLE BRACKET
            0xF0855 => '\u{300B}', // 》 RIGHT DOUBLE ANGLE BRACKET
            // 예시 마커 — `(F00DA 단풍 철 : 철 성분)` 패턴 — 한컴 PDF 시각 검증 필요
            0xF00DA => '\u{25B8}', // ▸ BLACK SMALL TRIANGLE (잠정, 시각 판정 후 정정)
            // [Task #826] HWP3 한컴 PUA 그래픽 라인 (PR #753 후속 — johab.rs:65,67).
            // 한컴 함초롬 폰트는 PUA glyph 보유, rhwp-studio 번들 폰트 (오픈 라이선스)
            // 부재 → render-time substitution. 측정/렌더링 양쪽 자동 적용.
            // sample11.hwp 머리말/꼬리말 가로선 패턴 (각 85+ 회) 시각 정합.
            0xF080F => '\u{2501}', // ━ BOX DRAWINGS HEAVY HORIZONTAL (한컴 — 굵은 가로선)
            // [Task #1692 Stage 9] HWP3 관계도 계열 선문자.
            // 한컴은 U+F0811/F0817/F081A를 자체 글리프로 이어진 선처럼 렌더한다.
            // 공개 폰트 경로에서는 .notdef 두부가 나오므로 대응 가능한 box drawing으로 낮춘다.
            0xF0811 => '\u{250C}', // ┌ BOX DRAWINGS LIGHT DOWN AND RIGHT
            0xF0817 => '\u{2514}', // └ BOX DRAWINGS LIGHT UP AND RIGHT
            0xF081A => '\u{2500}', // ─ BOX DRAWINGS LIGHT HORIZONTAL
            // [#5793] 시각 판정 완료 — 한글 2022 는 이중 가로선(제목 밑 이중 밑줄,
            // 반각 6.66px/자)으로 그린다. ■(전각)로 두면 띠가 2배 길어져 제목을
            // 겹친다(1776332, layout-anomaly text-overlap w=213 1위). 이웃
            // 0xF0832 → ═ 와 같은 이중선 계열.
            0xF0827 => '\u{2550}', // ═ BOX DRAWINGS DOUBLE HORIZONTAL
            _ => ch,
        };
    }

    if !(0xF020..=0xF0FF).contains(&code) {
        return ch;
    }
    let w = (code - 0xF000) as u8;
    match w {
        // 도형/기호 (0x6C~0x7E)
        0x6C => '\u{25CF}', // ● Black circle
        0x6D => '\u{25CF}', // ● (Lower right shadowed white circle → 근사값)
        0x6E => '\u{25A0}', // ■ Black square
        0x6F => '\u{25A1}', // □ White square
        0x70 => '\u{25A1}', // □ (Bold white square → 근사값)
        0x71 => '\u{25A1}', // □ (Lower right shadowed → 근사값)
        0x72 => '\u{25A1}', // □ (Upper right shadowed → 근사값)
        0x73 => '\u{2B27}', // ⬧ Black medium lozenge
        0x74 => '\u{29EB}', // ⧫ Black lozenge
        0x75 => '\u{25C6}', // ◆ Black diamond
        0x76 => '\u{2756}', // ❖ Black diamond minus white X
        0x77 => '\u{2B25}', // ⬥ Black medium diamond
        // 체크/별/점 (0x9E~0xAF)
        0x9E => '\u{00B7}', // · Middle dot
        0x9F => '\u{2022}', // • Bullet
        // [Task #509] 0xA0 → · U+00B7 (Middle dot) — 한컴 PDF 정답지 시각 정합.
        // ▪ U+25AA (Black small square) 영역 아님 (synam-001 사용 영역).
        0xA0 => '\u{00B7}', // · Middle dot
        0xA1 => '\u{26AA}', // ⚪ Medium white circle
        0xA2 => '\u{25CB}', // ○ (Heavy large circle → 근사값)
        0xA3 => '\u{25CB}', // ○ (Very heavy white circle → 근사값)
        0xA4 => '\u{25C9}', // ◉ Fisheye
        0xA5 => '\u{25CE}', // ◎ Bullseye
        0xA7 => '\u{25AA}', // ▪ Black small square
        0xA8 => '\u{25FB}', // ◻ White medium square
        0xAA => '\u{2726}', // ✦ Black four pointed star
        0xAB => '\u{2605}', // ★ Black star
        0xAC => '\u{2736}', // ✶ Six pointed black star
        0xAD => '\u{2734}', // ✴ Eight pointed black star
        0xAE => '\u{2739}', // ✹ Twelve pointed black star
        // 손 모양 (0x45~0x48)
        0x45 => '\u{261C}', // ☜ White left pointing index
        0x46 => '\u{261E}', // ☞ White right pointing index
        0x47 => '\u{261D}', // ☝ White up pointing index
        0x48 => '\u{261F}', // ☟ White down pointing index
        // 체크마크 (0xFB~0xFE)
        0xFB => '\u{2717}', // ✗ Ballot X (근사값)
        0xFC => '\u{2714}', // ✔ Heavy check mark
        0xFD => '\u{2612}', // ☒ Ballot box with X (근사값)
        0xFE => '\u{2611}', // ☑ Ballot box with check (근사값)
        // 화살표 (0xEF~0xF8)
        // [Task #509] 0xE8 → ➔ U+2794 (Heavy wide-headed rightwards arrow) —
        // 한컴 PDF 정답지 시각 정합. ➤ U+27A4 (Black rightwards) 와 글리프 형태
        // 차이 — 한컴은 wide-headed arrow 영역.
        0xE8 => '\u{2794}', // ➔ Heavy wide-headed rightwards arrow
        0xEF => '\u{21E6}', // ⇦ Leftwards white arrow
        0xF0 => '\u{21E8}', // ⇨ Rightwards white arrow
        0xF1 => '\u{21E7}', // ⇧ Upwards white arrow
        0xF2 => '\u{21E9}', // ⇩ Downwards white arrow
        // 기타 자주 쓰이는 기호
        0x22 => '\u{2702}', // ✂ Black scissors
        0x36 => '\u{231B}', // ⌛ Hourglass
        0x4A => '\u{263A}', // ☺ White smiling face
        0x4E => '\u{2620}', // ☠ Skull and crossbones
        0x52 => '\u{263C}', // ☼ White sun with rays
        0x54 => '\u{2744}', // ❄ Snowflake
        0x58 => '\u{2720}', // ✠ Maltese cross
        0x59 => '\u{2721}', // ✡ Star of David
        // 매핑 없는 PUA 문자는 원본 유지
        _ => ch,
    }
}
