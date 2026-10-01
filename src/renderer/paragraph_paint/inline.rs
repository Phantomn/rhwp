use crate::model::bin_data::BinDataContent;
use crate::model::control::Control;
use crate::model::paragraph::Paragraph;
use crate::model::style::Alignment;
use crate::renderer::composer::{ComposedLine, ComposedParagraph};
use crate::renderer::hwpunit_to_px;
use crate::renderer::layout::utils::find_bin_data_bytes;
use crate::renderer::layout::CellContext;
use crate::renderer::page_layout::LayoutRect;
use crate::renderer::render_tree::*;
use crate::renderer::style_resolver::ResolvedStyleSet;

use super::helpers::*;
use super::ParagraphPaintSession;

impl ParagraphPaintSession<'_> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn place_unmatched_line_tac_pictures(
        &self,
        tree: &mut PageLayoutContext,
        line_node: &mut RenderNode,
        comp_line: &ComposedLine,
        para: Option<&Paragraph>,
        bin_data_content: Option<&[BinDataContent]>,
        tac_offsets_px: &[(usize, f64, usize)],
        col_area: &LayoutRect,
        cell_ctx: Option<&CellContext>,
        reserved_tac_picture_height: &mut Option<f64>,
        v: TacPictureLineVars,
    ) -> f64 {
        let TacPictureLineVars {
            run_char_pos,
            mut x,
            y,
            baseline,
            raw_lh,
            section_index,
            para_index,
        } = v;
        if !comp_line.runs.is_empty() && !tac_offsets_px.is_empty() {
            if let (Some(p), Some(bdc)) = (para, bin_data_content) {
                let line_start_char = comp_line.char_start;
                let line_end_char = line_start_char
                    + comp_line
                        .runs
                        .iter()
                        .map(|r| r.text.chars().count())
                        .sum::<usize>();
                for &(tac_pos, tac_w, tac_ci) in tac_offsets_px {
                    if tac_pos <= run_char_pos || tac_pos > line_end_char {
                        continue; // run 범위 내/끝 또는 미래 줄 TAC: 여기서 처리하지 않음
                    }
                    if let Some(ctrl) = p.controls.get(tac_ci) {
                        if let Control::Picture(pic) = ctrl {
                            let (pic_w, pic_h) = self.resolve_inline_picture_size(pic, col_area);
                            if raw_lh + 4.0 >= pic_h {
                                *reserved_tac_picture_height = Some(pic_h);
                            }
                            // [#6603] 상자(잉크 + 캡션 + 위아래 여백)를 baseline 에 앉히고
                            // 잉크는 상자의 (왼쪽, 위) 여백 안쪽에 그린다.
                            let (margin_left, _, margin_top, margin_bottom) =
                                tac_picture_outer_margins_px(pic, self.dpi);
                            let box_h = tac_object_box_height_px(pic_h, &pic.caption, self.dpi)
                                + margin_top
                                + margin_bottom;
                            let img_y = (y + baseline - box_h).max(y) + margin_top;
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
                                section_index,
                                para_index,
                                tac_ci,
                                cell_ctx,
                                crop,
                                original_size_hu,
                                bin_data_id,
                                image_data,
                                BoundingBox::new(x + margin_left, img_y, pic_w, pic_h),
                            );
                            line_node.children.push(img_node);
                            x += tac_w;
                        }
                    }
                }
            }
        }
        x
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn place_empty_line_tac_forms(
        &self,
        tree: &mut PageLayoutContext,
        line_node: &mut RenderNode,
        comp_line: &ComposedLine,
        para: Option<&Paragraph>,
        tac_offsets_px: &[(usize, f64, usize)],
        cell_ctx: Option<&CellContext>,
        mut x: f64,
        y: f64,
        baseline: f64,
        section_index: usize,
        para_index: usize,
    ) -> f64 {
        if comp_line.runs.is_empty() && !tac_offsets_px.is_empty() {
            if let Some(p) = para {
                for &(_tac_pos, tac_w, tac_ci) in tac_offsets_px {
                    if let Some(Control::Form(f)) = p.controls.get(tac_ci) {
                        let form_h = hwpunit_to_px(f.height as i32, self.dpi);
                        let form_y = (y + baseline - form_h).max(y);
                        let cell_location = cell_ctx.and_then(|ctx| {
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
                        x += tac_w;
                    }
                }
            }
        }
        x
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn place_empty_line_inline_equations(
        &self,
        tree: &mut PageLayoutContext,
        line_node: &mut RenderNode,
        comp_line: &ComposedLine,
        composed: &ComposedParagraph,
        para: Option<&Paragraph>,
        styles: &ResolvedStyleSet,
        cell_ctx: &Option<CellContext>,
        tac_offsets_px: &[(usize, f64, usize)],
        line_tac_offsets: &[(usize, f64, usize)],
        equation_tac_line_flow: &Option<crate::renderer::equation_tac_flow::EquationTacLineFlow>,
        v: EquationTacLineVars,
    ) {
        if !comp_line.runs.is_empty() || tac_offsets_px.is_empty() {
            return;
        }
        let EquationTacLineVars {
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
            col_area_y,
            col_bottom,
            line_char_end,
            is_last_line_of_para,
            defer_empty_line_control_marker,
            equation_tac_extra_rows,
            hwp3_indent_scale,
            section_index,
            para_index,
        } = v;
        let line_start_char = comp_line.char_start;
        let line_end_char = composed
            .lines
            .get(line_idx + 1)
            .map(|l| l.char_start)
            .unwrap_or(usize::MAX);
        let tac_on_line = |k: usize, pos: usize| -> bool {
            if let Some(ref flow) = equation_tac_line_flow {
                flow.row_for_tac(k).is_some()
            } else {
                pos >= line_start_char && pos < line_end_char
            }
        };
        let tac_row_for = |k: usize| -> usize {
            equation_tac_line_flow
                .as_ref()
                .and_then(|flow| flow.row_for_tac(k))
                .unwrap_or(0)
        };
        // [Task #490] 셀에 텍스트 없이 수식만 있을 때는 셀 ParaShape alignment 를
        // 따라야 한다. 단, [Task #1245] 본문/미주 수식-only 줄은 저장된 LINE_SEG
        // 흐름을 따라야 하며 문단 alignment 를 다시 적용하면 열 안에서 중앙으로 밀린다.
        // [Task #489] effective_col_x 적용 (Picture+Square wrap LINE_SEG cs/sw 좁은 영역).
        let mut row_tac_widths = vec![0.0f64; equation_tac_extra_rows + 1];
        for (k, (pos, w, _)) in tac_offsets_px.iter().enumerate() {
            if tac_on_line(k, *pos) {
                let row = tac_row_for(k).min(row_tac_widths.len() - 1);
                row_tac_widths[row] += *w;
            }
        }
        let line_tac_width: f64 = row_tac_widths.iter().sum();
        // [#5583] 본문 흐름의 수식-only 줄도 문단 정렬을 따른다 — 단 저장 LINE_SEG 가 줄 시작
        // 위치를 담고 있으면(그 값이 권위다) 종전대로 저장 흐름을 쓴다.
        //
        // 종전에는 비-셀이면 무조건 0.0 이라 가운데 정렬 문단의 수식이 단 왼쪽 끝에 붙었다
        // (3252633 국가유산수리 감리대가 기준 2·3쪽: 저장 cs=0 sw=48188 인데 수식 x=75.6 =
        // 본문 좌측, 가운데라면 269.6). `column_start > 0` 인 줄은 한컴이 흐름 x 를 적어 둔
        // 경우이므로 #1256/#1308 계약대로 그 값을 존중한다.
        let align_offset = if cell_ctx.is_some() || comp_line.column_start == 0 {
            match alignment {
                Alignment::Center | Alignment::Distribute => {
                    (available_width - line_tac_width).max(0.0) / 2.0
                }
                Alignment::Right => (available_width - line_tac_width).max(0.0),
                _ => 0.0,
            }
        } else {
            0.0
        };
        // Empty-run TAC-only lines still belong to the visual line flow.
        // Therefore paragraph margins and first-line/hanging indent must
        // use the same x origin as ordinary TextLine nodes.
        let row_base_x = |row: usize| -> f64 {
            let visual_line_idx = equation_tac_line_flow
                .as_ref()
                .map(|flow| flow.visual_line_idx_for_row(row))
                .unwrap_or(line_idx + row);
            let row_effective_margin_left =
                    crate::renderer::equation_tac_flow::paragraph_effective_margin_left_with_indent_scale(
                        margin_left,
                        indent,
                        visual_line_idx,
                        // [Task #1472] 변환본은 effective indent 불변 위해 scale 절반.
                        (if equation_tac_line_flow.is_some() && cell_ctx.is_none() {
                            2.0
                        } else {
                            1.0
                        }) * hwp3_indent_scale,
                    );
            effective_col_x + row_effective_margin_left
        };
        let mut row_inline_x: Vec<f64> = (0..=equation_tac_extra_rows)
            .map(|row| {
                let row_width = row_tac_widths.get(row).copied().unwrap_or(0.0);
                let row_align_offset = if cell_ctx.is_some() {
                    match alignment {
                        Alignment::Center | Alignment::Distribute => {
                            (available_width - row_width).max(0.0) / 2.0
                        }
                        Alignment::Right => (available_width - row_width).max(0.0),
                        _ => 0.0,
                    }
                } else {
                    align_offset
                };
                row_base_x(row) + row_align_offset
            })
            .collect();
        let zero_endnote_boundary_result_shift = if cell_ctx.is_none()
            && self.current_endnote_zero_spacing_profile()
            && para_index >= self.endnote_para_base.get()
            && !self.endnote_para_has_same_endnote_successor(para_index)
            && line_idx + 1 >= end
            && equation_tac_extra_rows == 0
            && line_tac_offsets.len() == 1
            && comp_line.runs.is_empty()
            && y + line_height > col_bottom - 20.0
            && line_tac_offsets.iter().all(|(_, _, ci)| {
                para.is_some_and(|p| {
                    matches!(
                        p.controls.get(*ci),
                        Some(Control::Equation(eq))
                            if eq.common.treat_as_char && eq.common.height <= 1200
                    )
                })
            }) {
            // 0/0/0 미주에서는 새 미주 제목이 바로 뒤따르는 작은 결과식 tail이
            // 저장 LINE_SEG 하단에 놓이면 제목과 순서가 뒤집혀 보일 수 있다.
            // 물리 흐름은 유지하고 마지막 작은 수식 표시만 한 줄 위 결과 위치로 붙인다.
            ((line_height + line_spacing_px) * 2.0).clamp(24.0, 42.0)
        } else {
            0.0
        };
        for (tac_k, &(tac_pos, tac_w, tac_ci)) in tac_offsets_px.iter().enumerate() {
            if !tac_on_line(tac_k, tac_pos) {
                continue;
            }
            if let Some(p) = para {
                if let Some(Control::Equation(eq)) = p.controls.get(tac_ci) {
                    let tokens = crate::renderer::equation::tokenizer::tokenize(&eq.script);
                    let ast = crate::renderer::equation::parser::EqParser::new(tokens).parse();
                    let font_size_px = hwpunit_to_px(eq.font_size as i32, self.dpi);
                    let layout_box =
                        crate::renderer::equation::layout::EqLayout::new(font_size_px).layout(&ast);
                    let color_str =
                        crate::renderer::equation::svg_render::eq_color_to_svg(eq.color);
                    let svg_content = crate::renderer::equation::svg_render::render_equation_svg(
                        &layout_box,
                        &color_str,
                        font_size_px,
                    );
                    let hwp_eq_h = hwpunit_to_px(eq.common.height as i32, self.dpi);
                    let eq_h = if hwp_eq_h > 0.0 {
                        hwp_eq_h
                    } else {
                        layout_box.height
                    };
                    let tac_row = tac_row_for(tac_k).min(row_inline_x.len() - 1);
                    let row_y = (y + tac_row as f64 * (line_height + line_spacing_px)
                        - zero_endnote_boundary_result_shift)
                        .max(col_area_y);
                    let inline_x = row_inline_x[tac_row];
                    let eq_y = if cell_ctx.is_some() {
                        (row_y + baseline - layout_box.baseline).max(row_y)
                    } else {
                        row_y + baseline - layout_box.baseline
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
                        RenderNodeType::Equation(crate::renderer::render_tree::EquationNode {
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
                                ctx.path.first().map(|e| e.control_index).or(Some(tac_ci))
                            } else {
                                Some(tac_ci)
                            },
                            cell_index: eq_cell_idx,
                            cell_para_index: eq_cell_para_idx,
                            note_ref,
                        }),
                        BoundingBox::new(inline_x, eq_y, tac_w, eq_h),
                    );
                    line_node.children.push(eq_node);
                    tree.set_inline_shape_position(
                        section_index,
                        para_index,
                        tac_ci,
                        cell_ctx.as_ref(),
                        inline_x,
                        eq_y,
                    );
                    row_inline_x[tac_row] += tac_w;
                }
            }
        }

        if defer_empty_line_control_marker
            && (is_last_line_of_para || comp_line.has_line_break)
            && !row_inline_x.is_empty()
        {
            let marker_row = row_tac_widths
                .iter()
                .enumerate()
                .rev()
                .find_map(|(row, width)| if *width > 0.0 { Some(row) } else { None })
                .unwrap_or(0)
                .min(row_inline_x.len() - 1);
            let marker_x = row_inline_x[marker_row];
            let marker_y = y + marker_row as f64 * (line_height + line_spacing_px);
            let marker_id = tree.next_id();
            let marker_style = paragraph_active_text_style(styles, para, line_char_end).0;
            let marker_node = RenderNode::new(
                marker_id,
                RenderNodeType::TextRun(TextRunNode {
                    text: String::new(),
                    style: marker_style,
                    char_shape_id: None,
                    para_shape_id: Some(composed.para_style_id),
                    section_index: Some(section_index),
                    para_index: Some(para_index),
                    char_start: None,
                    cell_context: cell_ctx.clone(),
                    is_para_end: is_last_line_of_para,
                    is_line_break_end: comp_line.has_line_break,
                    rotation: 0.0,
                    is_vertical: false,
                    char_overlap: None,
                    border_fill_id: 0,
                    baseline,
                    field_marker: FieldMarkerType::None,
                    layout_positions: None,
                    display_text: None,
                }),
                BoundingBox::new(marker_x, marker_y, 0.0, line_height),
            );
            line_node.children.push(marker_node);
        }
    }

    #[allow(clippy::too_many_arguments)]
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

        super::object_size(&picture.common, col_area, &body_area, &paper_area, self.dpi)
    }
}
