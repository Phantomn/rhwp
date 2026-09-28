//! Paragraph-relative exclusion alongside or after an intact stored host. Square uses
//! validated saved lanes, not a fresh side-wrap/TAC reflow solver.
use crate::model::{
    shape::{HorzAlign, HorzRelTo, TextFlow, TextWrap, VertAlign, VertRelTo},
    table::Table,
};
use crate::renderer::hwpunit_to_px;

use super::{GeometryError, Insets, ParagraphEnd};

/// One coordinate result for the positioned table's reservation query. The
/// initial band is committed only with an accepted fragment, not inserted into
/// the text story. Continuation never reapplies the source offset. The trailing
/// band follows the final child fragment only.
pub(super) struct BodyAnchor {
    pub x: f64,
    /// Distance from the composed host's occupied end, not its following pen.
    pub before: f64,
    /// Signed origin from the composed host end for Square. This is an object
    /// coordinate, never negative story space; final 2D exclusion is validated.
    pub host_end_offset: f64,
    pub after: f64,
    pub restart_top: f64,
    /// A stored Square object reserves a physical box, not full-width story
    /// advance. Final body rows must still prove that its exclusion is respected.
    pub side_wrap: Option<Insets>,
}

impl BodyAnchor {
    pub fn resolve(
        table: &Table,
        host: &ParagraphEnd,
        table_width: f64,
        body_width: f64,
        paper_right: f64,
        dpi: f64,
    ) -> Result<Self, GeometryError> {
        let a = &table.common;
        let side_wrap = a.text_wrap == TextWrap::Square && a.text_flow == TextFlow::BothSides;
        if a.treat_as_char
            || (!side_wrap && a.text_wrap != TextWrap::TopAndBottom)
            || a.vert_rel_to != VertRelTo::Para
            || a.vert_align != VertAlign::Top
            || a.horz_rel_to != HorzRelTo::Para
            || a.horz_align != HorzAlign::Left
            || !a.flow_with_text
            || a.allow_overlap
            || a.prevent_page_break != 0
            || (a.vertical_offset as i32) < 0
            || (a.horizontal_offset as i32) < 0
        {
            return Err(GeometryError::Unsupported("stored body anchor mode"));
        }
        // These are two IR representations of ONE source margin record. Never
        // add them together. Parsed HWP/HWPX synchronize them; disagreement is
        // not permission to silently pick a different layout contract.
        for (common, mirror) in [
            (a.margin.left, table.outer_margin_left),
            (a.margin.right, table.outer_margin_right),
            (a.margin.top, table.outer_margin_top),
            (a.margin.bottom, table.outer_margin_bottom),
        ] {
            if common < 0 || common != mirror {
                return Err(GeometryError::Unsupported("stored body anchor margins"));
            }
        }
        let x = hwpunit_to_px(a.horizontal_offset as i32, dpi)
            + hwpunit_to_px(i32::from(a.margin.left), dpi);
        let top = hwpunit_to_px(a.vertical_offset as i32, dpi)
            + hwpunit_to_px(i32::from(a.margin.top), dpi);
        // Full-width exclusion cannot share the host's occupied vertical band.
        // Square can use its side lane, but only final 2D geometry proves that:
        // blank and visible host rows are both validated before page commit.
        if !side_wrap && top < host.occupied_end() {
            return Err(GeometryError::Unsupported(
                "stored body anchor intersects host flow",
            ));
        }
        let limit = if side_wrap { paper_right } else { body_width };
        let end = x
            + table_width
            + if side_wrap {
                hwpunit_to_px(i32::from(a.margin.right), dpi)
            } else {
                0.0
            };
        if end > limit && (!side_wrap || !super::tac::same(end, limit)) {
            return Err(GeometryError::Unsupported(
                "stored body anchor outside body",
            ));
        }
        Ok(Self {
            x,
            before: if side_wrap {
                0.0
            } else {
                top - host.occupied_end()
            },
            host_end_offset: if side_wrap {
                top - host.occupied_end()
            } else {
                0.0
            },
            after: hwpunit_to_px(i32::from(a.margin.bottom), dpi),
            restart_top: hwpunit_to_px(i32::from(a.margin.top), dpi),
            side_wrap: side_wrap.then_some(Insets {
                left: hwpunit_to_px(i32::from(a.margin.left), dpi),
                right: hwpunit_to_px(i32::from(a.margin.right), dpi),
                top: hwpunit_to_px(i32::from(a.margin.top), dpi),
                bottom: hwpunit_to_px(i32::from(a.margin.bottom), dpi),
            }),
        })
    }
}
