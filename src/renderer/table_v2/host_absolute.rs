//! Stored page/paper-relative tables. The accepted owner line selects the page
//! and column; coordinates never select ownership. One immutable V2 plan supplies
//! the aligned extent, fit and paint. Collision requires reflow, not a clamp.
use super::{
    CellEndPolicy, GeometryError, Insets, PageArea, PreparedTextTable, Rect, TextFragmentFit,
};
use crate::{
    model::{
        control::Control,
        document::Document,
        paragraph::Paragraph,
        shape::{CommonObjAttr, HorzAlign, HorzRelTo, TextWrap, VertAlign, VertRelTo},
    },
    renderer::{
        page_layout::PageLayoutInfo,
        render_tree::{RenderNode, RenderNodeType},
        style_resolver::ResolvedStyleSet,
    },
};

pub(super) struct HostedAbsolute {
    pub owner_line: usize,
    pub control: usize,
    common: CommonObjAttr,
    margin: Insets,
    prepared: PreparedTextTable,
    scale: f64,
}

impl HostedAbsolute {
    pub fn prepare(
        doc: &Document,
        para: &Paragraph,
        control: usize,
        styles: &ResolvedStyleSet,
        dpi: f64,
        policy: CellEndPolicy,
    ) -> Result<Self, GeometryError> {
        let fail = || GeometryError::Unsupported("stored absolute table anchor");
        let Control::Table(t) = &para.controls[control] else {
            return Err(fail());
        };
        let c = &t.common;
        if para.line_segs.is_empty()
            || para.stored_text_partition_is_dirty()
            || c.treat_as_char
            || c.text_wrap != TextWrap::TopAndBottom
            || !matches!(c.vert_rel_to, VertRelTo::Paper | VertRelTo::Page)
            || !matches!(
                c.horz_rel_to,
                HorzRelTo::Paper | HorzRelTo::Page | HorzRelTo::Column
            )
            || !matches!(
                c.horz_align,
                HorzAlign::Left | HorzAlign::Center | HorzAlign::Right
            )
            || !matches!(
                c.vert_align,
                VertAlign::Top | VertAlign::Center | VertAlign::Bottom
            )
            || c.allow_overlap
            || c.prevent_page_break != 0
            || t.caption.is_some()
        {
            return Err(fail());
        }
        for (v, mirror) in [
            (c.margin.left, t.outer_margin_left),
            (c.margin.right, t.outer_margin_right),
            (c.margin.top, t.outer_margin_top),
            (c.margin.bottom, t.outer_margin_bottom),
        ] {
            if v < 0 || v != mirror {
                return Err(fail());
            }
        }
        let position = *para
            .control_utf16_positions()
            .get(control)
            .ok_or_else(fail)?;
        let owner_line = para
            .line_segs
            .iter()
            .rposition(|s| s.text_start <= position)
            .ok_or_else(fail)?;
        super::decoration::validate_source(t, &doc.doc_info)?;
        let prepared = PreparedTextTable::prepare_with_end_policy(
            t,
            styles,
            dpi,
            &doc.bin_data_content,
            policy,
        )?;
        let scale = dpi / 7200.;
        Ok(Self {
            owner_line,
            control,
            common: c.clone(),
            prepared,
            scale,
            margin: Insets {
                left: f64::from(c.margin.left) * scale,
                right: f64::from(c.margin.right) * scale,
                top: f64::from(c.margin.top) * scale,
                bottom: f64::from(c.margin.bottom) * scale,
            },
        })
    }

    pub fn place(
        &self,
        layout: &PageLayoutInfo,
        column: usize,
        section: usize,
        paragraph: usize,
        number: u32,
    ) -> Result<(RenderNode, Rect), GeometryError> {
        let fail =
            || GeometryError::Unsupported("absolute table requires intact in-paper placement");
        let c = &self.common;
        let lane = layout.column_areas.get(column).ok_or_else(fail)?;
        let body = layout.body_area;
        let (x, w) = match c.horz_rel_to {
            HorzRelTo::Paper => (0., layout.page_width),
            HorzRelTo::Page => (body.x, body.width),
            HorzRelTo::Column => (lane.x, lane.width),
            _ => return Err(fail()),
        };
        let (y, h) = match c.vert_rel_to {
            VertRelTo::Paper => (0., layout.page_height),
            VertRelTo::Page => (body.y, body.height),
            _ => return Err(fail()),
        };
        let m = self.margin;
        let width = self.prepared.plan.width;
        let height = self.prepared.plan.height;
        let dx = f64::from(c.horizontal_offset as i32) * self.scale;
        let dy = f64::from(c.vertical_offset as i32) * self.scale;
        let x = match c.horz_align {
            HorzAlign::Left => x + dx + m.left,
            HorzAlign::Center => x + (w - width) / 2. + dx,
            HorzAlign::Right => x + w - width - dx - m.right,
            _ => return Err(fail()),
        };
        let y = match c.vert_align {
            VertAlign::Top => y + dy + m.top,
            VertAlign::Center => y + (h - height) / 2. + dy,
            VertAlign::Bottom => y + h - height - dy - m.bottom,
            _ => return Err(fail()),
        };
        let bounds = Rect {
            x,
            y,
            width,
            height,
        };
        let outer = super::body_flow::exclusion_bounds(bounds, m);
        let roundoff = 8. * f64::EPSILON * layout.page_width.max(layout.page_height);
        if outer.x < -roundoff
            || outer.y < -roundoff
            || outer.x + outer.width > layout.page_width + roundoff
            || outer.y + outer.height > layout.page_height + roundoff
        {
            return Err(fail());
        }
        let TextFragmentFit::Placed(fragment) = self.prepared.start().fit(PageArea { bounds })?
        else {
            return Err(fail());
        };
        if !fragment.continuation().is_complete()
            || !super::tac::same(fragment.geometry().reserved_height(), height)
        {
            return Err(fail());
        }
        let mut node = fragment.build_node(Some(number))?;
        if let RenderNodeType::Table(t) = &mut node.node_type {
            t.section_index = Some(section);
            t.para_index = Some(paragraph);
            t.control_index = Some(self.control);
        }
        Ok((node, outer))
    }
}
