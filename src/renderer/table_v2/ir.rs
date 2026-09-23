//! Document-IR ownership adapter. Paragraph composition is an explicit dependency,
//! not a fallback to Legacy table measurement or an interpretation of saved vpos.
use std::sync::Arc;

use crate::model::control::Control;
use crate::model::paragraph::{ColumnBreakType, Paragraph};
use crate::model::shape::{HorzAlign, HorzRelTo, SizeCriterion, TextWrap, VertAlign, VertRelTo};
use crate::model::table::{Table, TablePageBreak, VerticalAlign};

use super::{
    ControlOwner, FlowBlock, FlowCellInput, FlowRowInput, GeometryError, Insets, LineBox,
    LineOwner, Rect, SplitPolicy, TableContentPlan,
};

/// Output of a width-bound paragraph composer. A table slot refers to the actual
/// Paragraph.controls index; it never contains an independently rebuilt table.
pub enum ParagraphItem {
    Space(f64),
    Lines {
        height: f64,
        lines: Vec<(usize, Rect)>,
    },
    TableControl(usize),
}

/// The implementation must include paragraph spacing, empty lines, explicit
/// breaks and text layout. This adapter does not qualify saved LineSeg caches.
/// The text preview adapter supplies only a fresh, plain-text subset. Production
/// document composition and stored-LineSeg qualification are still separate work.
pub trait CellParagraphComposer {
    fn compose(
        &self,
        paragraph: &Paragraph,
        width: f64,
    ) -> Result<Vec<ParagraphItem>, GeometryError>;
}

impl TableContentPlan {
    /// Build *local* table content from Document IR. The outer paragraph anchor
    /// and document engine selection remain the caller's responsibility.
    /// Only plain cells and zero-offset TopAndBottom nested controls are admitted.
    pub fn from_ir_contents(
        table: &Table,
        units_per_hwp: f64,
        composer: &impl CellParagraphComposer,
    ) -> Result<Self, GeometryError> {
        if !units_per_hwp.is_finite() || units_per_hwp <= 0.0 {
            return Err(GeometryError::InvalidNumber("HWP unit scale"));
        }
        bind_table(table, units_per_hwp, composer, 0)
    }
}

fn bind_table(
    table: &Table,
    scale: f64,
    composer: &impl CellParagraphComposer,
    depth: usize,
) -> Result<TableContentPlan, GeometryError> {
    if depth >= 64 {
        return Err(GeometryError::Unsupported("table nesting resource limit"));
    }
    if table.caption.is_some() || table.cell_spacing != 0 {
        return Err(GeometryError::Unsupported("caption or cell spacing"));
    }
    if table.common.width_criterion != SizeCriterion::Absolute
        || table.common.height_criterion != SizeCriterion::Absolute
    {
        return Err(GeometryError::Unsupported("relative table size"));
    }
    let nr = table.row_count as usize;
    let nc = table.col_count as usize;
    if nr == 0 || nc == 0 {
        return Err(GeometryError::EmptyTable);
    }
    if table.cells.len() != nr * nc {
        return Err(GeometryError::Unsupported("incomplete or merged grid"));
    }
    let mut grid = vec![None; nr * nc];
    for cell in &table.cells {
        let (r, c) = (cell.row as usize, cell.col as usize);
        if r >= nr || c >= nc || cell.row_span != 1 || cell.col_span != 1 {
            return Err(GeometryError::Unsupported("cell address or span"));
        }
        if grid[r * nc + c].replace(cell).is_some() {
            return Err(GeometryError::Unsupported("duplicate cell address"));
        }
        if cell.vertical_align != VerticalAlign::Top
            || cell.text_direction != 0
            || cell.line_wrap != 0
        {
            return Err(GeometryError::Unsupported(
                "cell alignment, direction or line wrap",
            ));
        }
    }
    let mut widths = Vec::with_capacity(nc);
    // The preview admits only whole, contiguous leading header rows. Partial
    // header cells, spans, or scattered markers need a separate qualification;
    // do not infer their repetition from a row's incidental content.
    let mut header_rows = 0;
    if table.repeat_header {
        for r in 0..nr {
            let count = grid[r * nc..(r + 1) * nc]
                .iter()
                .filter(|c| c.is_some_and(|c| c.is_header))
                .count();
            if count != 0 {
                if count != nc || r != header_rows {
                    return Err(GeometryError::Unsupported(
                        "partial or non-leading header rows",
                    ));
                }
                header_rows += 1;
            }
        }
    }
    for c in 0..nc {
        let width = grid[c]
            .ok_or(GeometryError::Unsupported("missing cell"))?
            .width;
        if (1..nr).any(|r| grid[r * nc + c].is_none_or(|cell| cell.width != width)) {
            return Err(GeometryError::Unsupported("inconsistent column widths"));
        }
        widths.push(width as f64 * scale);
    }
    let mut rows = Vec::with_capacity(nr);
    for r in 0..nr {
        let mut cells = Vec::with_capacity(nc);
        for c in 0..nc {
            let cell = grid[r * nc + c].ok_or(GeometryError::Unsupported("missing cell"))?;
            let p = if cell.apply_inner_margin {
                cell.padding
            } else {
                table.padding
            };
            let padding = Insets {
                left: p.left as f64 * scale,
                right: p.right as f64 * scale,
                top: p.top as f64 * scale,
                bottom: p.bottom as f64 * scale,
            };
            for value in [padding.left, padding.right, padding.top, padding.bottom] {
                super::contracts::nonnegative(value, "resolved IR padding")?;
            }
            let inner_width = widths[c] - padding.left - padding.right;
            super::contracts::nonnegative(inner_width, "IR content width")?;
            let mut blocks = Vec::new();
            for (pi, para) in cell.paragraphs.iter().enumerate() {
                if para.column_type != ColumnBreakType::None {
                    return Err(GeometryError::Unsupported(
                        "explicit paragraph page/column break",
                    ));
                }
                for ctrl in &para.controls {
                    let Control::Table(child) = ctrl else {
                        return Err(GeometryError::Unsupported("non-table cell control"));
                    };
                    let a = &child.common;
                    if a.treat_as_char
                        || a.text_wrap != TextWrap::TopAndBottom
                        || a.vert_rel_to != VertRelTo::Para
                        || a.vert_align != VertAlign::Top
                        || a.horz_rel_to != HorzRelTo::Para
                        || a.vertical_offset != 0
                        || a.horizontal_offset != 0
                        || a.horz_align != HorzAlign::Left
                        || a.prevent_page_break != 0
                        || [
                            a.margin.left,
                            a.margin.right,
                            a.margin.top,
                            a.margin.bottom,
                            child.outer_margin_left,
                            child.outer_margin_right,
                            child.outer_margin_top,
                            child.outer_margin_bottom,
                        ]
                        .iter()
                        .any(|v| *v != 0)
                    {
                        return Err(GeometryError::Unsupported(
                            "nested anchor, TAC, wrap or outer margin",
                        ));
                    }
                }
                let mut seen = vec![false; para.controls.len()];
                for item in composer.compose(para, inner_width)? {
                    match item {
                        ParagraphItem::Space(h) => blocks.push(FlowBlock::Space(h)),
                        ParagraphItem::Lines { height, lines } => blocks.push(FlowBlock::Lines {
                            height,
                            lines: lines
                                .into_iter()
                                .map(|(line, bounds)| LineBox {
                                    owner: LineOwner {
                                        paragraph: pi,
                                        line,
                                    },
                                    bounds,
                                })
                                .collect(),
                        }),
                        ParagraphItem::TableControl(ci) => {
                            if seen.get(ci).copied() != Some(false) {
                                return Err(GeometryError::Unsupported(
                                    "invalid or repeated table slot",
                                ));
                            }
                            let Control::Table(child) = &para.controls[ci] else {
                                unreachable!()
                            };
                            blocks.push(FlowBlock::Table {
                                owner: ControlOwner {
                                    paragraph: pi,
                                    control: ci,
                                },
                                plan: Arc::new(bind_table(child, scale, composer, depth + 1)?),
                            });
                            seen[ci] = true;
                        }
                    }
                }
                if seen.contains(&false) {
                    return Err(GeometryError::Unsupported("unconsumed table control"));
                }
            }
            cells.push(FlowCellInput {
                padding,
                minimum_height: cell.height as f64 * scale,
                width: inner_width,
                blocks,
            });
        }
        rows.push(FlowRowInput { cells });
    }
    let policy = match table.page_break {
        TablePageBreak::None => SplitPolicy::Never,
        TablePageBreak::RowBreak => SplitPolicy::BetweenRows,
        TablePageBreak::CellBreak => SplitPolicy::WithinCells,
    };
    let mut plan = TableContentPlan::from_flow_rows(widths, rows, 0.0, policy)?;
    plan.header_rows = header_rows;
    Ok(plan)
}
