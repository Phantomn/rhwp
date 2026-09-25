//! Paragraph-relative exclusion after an intact stored host. This is not a
//! side-wrap/TAC solver and does not guess ownership from intersecting boxes.
use crate::model::{
    shape::{HorzAlign, HorzRelTo, TextWrap, VertAlign, VertRelTo},
    table::Table,
};
use crate::renderer::hwpunit_to_px;

use super::{GeometryError, ParagraphEnd};

/// One coordinate result for the positioned table's reservation query. The
/// initial band is committed only with an accepted fragment, not inserted into
/// the text story. Continuation never reapplies the source offset. The trailing
/// band follows the final child fragment only.
pub(super) struct BodyAnchor {
    pub x: f64,
    pub before: f64,
    pub after: f64,
    pub restart_top: f64,
}

impl BodyAnchor {
    pub fn resolve(
        table: &Table,
        host: &ParagraphEnd,
        table_width: f64,
        body_width: f64,
        dpi: f64,
    ) -> Result<Self, GeometryError> {
        let a = &table.common;
        if a.treat_as_char
            || a.text_wrap != TextWrap::TopAndBottom
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
        // Only disjoint, already-composed host rows are admitted. In particular
        // an empty string does NOT remove its line or authorize an overlap.
        if top < host.occupied_end() || top < host.next_origin() {
            return Err(GeometryError::Unsupported(
                "stored body anchor intersects host flow",
            ));
        }
        if x + table_width > body_width {
            return Err(GeometryError::Unsupported(
                "stored body anchor outside body",
            ));
        }
        Ok(Self {
            x,
            before: top - host.next_origin(),
            after: hwpunit_to_px(i32::from(a.margin.bottom), dpi),
            restart_top: hwpunit_to_px(i32::from(a.margin.top), dpi),
        })
    }
}
