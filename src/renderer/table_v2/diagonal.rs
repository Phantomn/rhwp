//! Qualified straight diagonal pens. Only accepted physical rectangles are
//! consumed here; content cuts, row heights and flow advancement are untouched.
use super::{GeometryError, Rect};
use crate::{
    model::style::{CenterLine, BORDER_WIDTHS},
    renderer::{
        layout::border_width_to_px,
        render_tree::{LineNode, RenderNode, RenderNodeType},
        style_resolver::{ResolvedBorderStyle, ResolvedStyleSet},
        LineStyle,
    },
};

pub(super) struct Diagonal {
    slash: bool,
    backslash: bool,
    color: u32,
    width: u8,
}

pub(super) fn supported_attr(attr: u16) -> bool {
    matches!(attr, 0 | 8 | 64 | 72)
}

impl Diagonal {
    pub fn resolve(id: u16, styles: &ResolvedStyleSet) -> Result<Option<Self>, GeometryError> {
        if id == 0 {
            return Ok(None);
        }
        Self::from_style(styles.border_styles.get(usize::from(id) - 1).ok_or(
            GeometryError::Unsupported("missing table borderFill reference"),
        )?)
    }

    pub fn from_style(style: &ResolvedBorderStyle) -> Result<Option<Self>, GeometryError> {
        if !supported_attr(style.diagonal_attr) || style.center_line != CenterLine::None {
            return Err(GeometryError::Unsupported("V2 diagonal shape"));
        }
        let pen = &style.diagonal;
        // A direction without a visible pen and a pen without a direction are
        // both inactive (Hancom-saved diagonal review rows4/5).
        if style.diagonal_attr == 0 || pen.diagonal_type == 0 {
            return Ok(None);
        }
        if pen.diagonal_type != 1
            || usize::from(pen.width) >= BORDER_WIDTHS.len()
            || pen.color >> 24 != 0
        {
            return Err(GeometryError::Unsupported("V2 diagonal pen"));
        }
        Ok(Some(Self {
            slash: style.diagonal_attr & 8 != 0,
            backslash: style.diagonal_attr & 64 != 0,
            color: pen.color,
            width: pen.width,
        }))
    }

    pub fn append(&self, node: &mut RenderNode, bounds: Rect, dpi: f64) {
        for (active, y1, y2) in [
            (self.slash, bounds.y + bounds.height, bounds.y),
            (self.backslash, bounds.y, bounds.y + bounds.height),
        ] {
            if !active {
                continue;
            }
            let line = LineNode::new(
                bounds.x,
                y1,
                bounds.x + bounds.width,
                y2,
                LineStyle {
                    color: self.color,
                    width: border_width_to_px(self.width) * dpi / 96.0,
                    ..Default::default()
                },
            );
            let bbox = line.ink_bbox();
            node.children
                .push(RenderNode::new(0, RenderNodeType::Line(line), bbox));
        }
    }
}
