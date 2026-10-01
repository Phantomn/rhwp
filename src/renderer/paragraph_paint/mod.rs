//! Shared text paint. Callers own line geometry, pagination and inline table placement.
//! Physical-frame paint does not construct a document or a Legacy layout engine.
use crate::model::bin_data::BinDataContent;
use crate::model::control::Control;
use crate::model::paragraph::Paragraph;
use crate::model::shape::CommonObjAttr;
use crate::model::style::Alignment;
use crate::model::table::Table;
use crate::renderer::cell_context::CellContext;
use crate::renderer::composer::ComposedParagraph;
use crate::renderer::hwpunit_to_px;
use crate::renderer::page_layout::LayoutRect;
use crate::renderer::render_tree::*;
use crate::renderer::style_resolver::ResolvedStyleSet;

pub(crate) mod fields;
mod frame;
pub(crate) mod helpers;
mod inline;
mod markers;
mod runs;

/// Compatibility hooks supplied only by the explicit Legacy paragraph caller.
/// V2 owns its tables and note sources before entering physical-row text paint.
pub(crate) trait ParagraphPaintHost {
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
    ) -> (f64, f64);
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
    ) -> bool;
    fn endnote_para_has_same_endnote_successor(&self, para_index: usize) -> bool;
    fn current_endnote_zero_spacing_profile(&self) -> bool;
    fn is_tolerated_current_endnote_bottom_bleed(
        &self,
        is_endnote_flow: bool,
        content_bottom: f64,
        col_bottom: f64,
        equation_tail_line_box: bool,
    ) -> bool;
    fn note_ref_for_endnote_equation(
        &self,
        para_index: usize,
        inner_control_index: usize,
    ) -> Option<NoteControlRef>;
}

/// Borrowed paragraph paint inputs and outputs; no table cache, paginator or engine owner.
pub(crate) struct ParagraphPaintSession<'a> {
    pub(crate) dpi: f64,
    pub(crate) font_state: &'a crate::renderer::font_layout_state::FontLayoutState,
    pub(crate) host: Option<&'a dyn ParagraphPaintHost>,
    pub(crate) use_hwp3_origin_flow_spacing_before: &'a std::cell::Cell<bool>,
    pub(crate) keep_continuation_column_top_spacing_before: &'a std::cell::Cell<bool>,
    pub(crate) reapply_snap_anchored_spacing_before: &'a std::cell::Cell<bool>,
    pub(crate) profile: &'a std::cell::Cell<crate::model::provenance::LayoutCompatibilityProfile>,
    pub(crate) endnote_para_base: &'a std::cell::Cell<usize>,
    pub(crate) uniform_filler_ladder: &'a std::cell::Cell<bool>,
    pub(crate) cell_has_square_float: &'a std::cell::Cell<bool>,
    pub(crate) body_float_carve_evidence:
        &'a std::cell::RefCell<Vec<crate::renderer::float_placement::FloatCarveEvidence>>,
    pub(crate) current_page_height: &'a std::cell::Cell<f64>,
    pub(crate) overflow_cell_lines: &'a std::cell::Cell<u32>,
    pub(crate) show_control_codes: &'a std::cell::Cell<bool>,
    pub(crate) last_item_content_bottom: &'a std::cell::Cell<f64>,
    pub(crate) last_item_endnote_equation_tail_line_box: &'a std::cell::Cell<bool>,
    pub(crate) collect_cell_para_borders: &'a std::cell::Cell<bool>,
    pub(crate) border_box_override: &'a std::cell::Cell<Option<(f64, f64)>>,
    pub(crate) para_border_ranges:
        &'a std::cell::RefCell<Vec<(u16, f64, f64, f64, f64, f64, f64, bool, bool, usize)>>,
    pub(crate) active_field:
        &'a std::cell::RefCell<Option<(usize, usize, usize, Option<Vec<(usize, usize, usize)>>)>>,
    pub(crate) current_body_area: &'a std::cell::Cell<(f64, f64, f64, f64)>,
    pub(crate) current_paper_width: &'a std::cell::Cell<f64>,
}

struct PhysicalFrameState {
    use_hwp3_origin_flow_spacing_before: std::cell::Cell<bool>,
    keep_continuation_column_top_spacing_before: std::cell::Cell<bool>,
    reapply_snap_anchored_spacing_before: std::cell::Cell<bool>,
    profile: std::cell::Cell<crate::model::provenance::LayoutCompatibilityProfile>,
    endnote_para_base: std::cell::Cell<usize>,
    uniform_filler_ladder: std::cell::Cell<bool>,
    cell_has_square_float: std::cell::Cell<bool>,
    body_float_carve_evidence:
        std::cell::RefCell<Vec<crate::renderer::float_placement::FloatCarveEvidence>>,
    current_page_height: std::cell::Cell<f64>,
    overflow_cell_lines: std::cell::Cell<u32>,
    show_control_codes: std::cell::Cell<bool>,
    last_item_content_bottom: std::cell::Cell<f64>,
    last_item_endnote_equation_tail_line_box: std::cell::Cell<bool>,
    collect_cell_para_borders: std::cell::Cell<bool>,
    border_box_override: std::cell::Cell<Option<(f64, f64)>>,
    para_border_ranges:
        std::cell::RefCell<Vec<(u16, f64, f64, f64, f64, f64, f64, bool, bool, usize)>>,
    active_field:
        std::cell::RefCell<Option<(usize, usize, usize, Option<Vec<(usize, usize, usize)>>)>>,
    current_body_area: std::cell::Cell<(f64, f64, f64, f64)>,
    current_paper_width: std::cell::Cell<f64>,
}
impl PhysicalFrameState {
    fn new() -> Self {
        Self {
            use_hwp3_origin_flow_spacing_before: std::cell::Cell::new(false),
            keep_continuation_column_top_spacing_before: std::cell::Cell::new(false),
            reapply_snap_anchored_spacing_before: std::cell::Cell::new(false),
            profile: std::cell::Cell::new(Default::default()),
            endnote_para_base: std::cell::Cell::new(usize::MAX),
            uniform_filler_ladder: std::cell::Cell::new(false),
            cell_has_square_float: std::cell::Cell::new(false),
            body_float_carve_evidence: std::cell::RefCell::new(Vec::new()),
            current_page_height: std::cell::Cell::new(0.0),
            overflow_cell_lines: std::cell::Cell::new(0),
            show_control_codes: std::cell::Cell::new(false),
            last_item_content_bottom: std::cell::Cell::new(f64::NAN),
            last_item_endnote_equation_tail_line_box: std::cell::Cell::new(false),
            collect_cell_para_borders: std::cell::Cell::new(false),
            border_box_override: std::cell::Cell::new(None),
            para_border_ranges: std::cell::RefCell::new(Vec::new()),
            active_field: std::cell::RefCell::new(None),
            current_body_area: std::cell::Cell::new((0.0, 0.0, 0.0, 0.0)),
            current_paper_width: std::cell::Cell::new(0.0),
        }
    }
    fn session<'a>(
        &'a self,
        dpi: f64,
        font_state: &'a crate::renderer::font_layout_state::FontLayoutState,
    ) -> ParagraphPaintSession<'a> {
        ParagraphPaintSession {
            dpi,
            font_state,
            host: None,
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

impl ParagraphPaintSession<'_> {
    fn is_body_flow_col_area(&self, area: &LayoutRect) -> bool {
        let (_, y, _, h) = self.current_body_area.get();
        h > 0.0 && (area.y - y).abs() < 1.0 && (area.height - h).abs() < 1.0
    }
    fn endnote_para_has_same_endnote_successor(&self, para_index: usize) -> bool {
        self.host.map_or(false, |h| {
            h.endnote_para_has_same_endnote_successor(para_index)
        })
    }
    fn current_endnote_zero_spacing_profile(&self) -> bool {
        self.host
            .map_or(true, |h| h.current_endnote_zero_spacing_profile())
    }
    fn is_tolerated_current_endnote_bottom_bleed(
        &self,
        is_endnote_flow: bool,
        content_bottom: f64,
        col_bottom: f64,
        equation_tail_line_box: bool,
    ) -> bool {
        self.host.map_or(false, |h| {
            h.is_tolerated_current_endnote_bottom_bleed(
                is_endnote_flow,
                content_bottom,
                col_bottom,
                equation_tail_line_box,
            )
        })
    }
    fn note_ref_for_endnote_equation(
        &self,
        para_index: usize,
        inner_control_index: usize,
    ) -> Option<NoteControlRef> {
        self.host.map_or(None, |h| {
            h.note_ref_for_endnote_equation(para_index, inner_control_index)
        })
    }
}

/// Only text/control glyphs enter this frame. V2 emits table fragments separately.
/// Reject an incorrect caller before emitting anything, never silently skip a table.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_physical_frame(
    tree: &mut PageLayoutContext,
    column: &mut RenderNode,
    composed: &ComposedParagraph,
    styles: &ResolvedStyleSet,
    area: &LayoutRect,
    para: &Paragraph,
    rows: &[std::ops::Range<f64>],
    squeeze: bool,
    dpi: f64,
) -> Result<f64, &'static str> {
    if para.controls.iter().any(|c| matches!(c, Control::Table(_))) {
        return Err("physical text paint received an unseparated table");
    }
    let state = PhysicalFrameState::new();
    let fonts = crate::renderer::font_layout_state::FontLayoutState::default();
    Ok(state
        .session(dpi, &fonts)
        .layout_composed_paragraph_in_frame(
            tree,
            column,
            composed,
            styles,
            area,
            0.0,
            0,
            composed.lines.len(),
            0,
            0,
            None,
            true,
            false,
            0.0,
            None,
            Some(para),
            None,
            None,
            Some(rows),
            squeeze,
        ))
}
pub(crate) fn object_size(
    common: &CommonObjAttr,
    col_area: &LayoutRect,
    body_area: &LayoutRect,
    paper_area: &LayoutRect,
    dpi: f64,
) -> (f64, f64) {
    use crate::model::shape::SizeCriterion;

    let raw_w = common.width as f64;
    let raw_h = common.height as f64;

    let obj_width = match common.width_criterion {
        SizeCriterion::Absolute => hwpunit_to_px(common.width as i32, dpi),
        SizeCriterion::Paper => paper_area.width * raw_w / 10000.0,
        SizeCriterion::Page => body_area.width * raw_w / 10000.0,
        SizeCriterion::Column => col_area.width * raw_w / 10000.0,
        SizeCriterion::Para => col_area.width * raw_w / 10000.0,
    };

    let obj_height = match common.height_criterion {
        SizeCriterion::Absolute => hwpunit_to_px(common.height as i32, dpi),
        SizeCriterion::Paper => paper_area.height * raw_h / 10000.0,
        SizeCriterion::Page => body_area.height * raw_h / 10000.0,
        _ => hwpunit_to_px(common.height as i32, dpi),
    };

    (obj_width, obj_height)
}
