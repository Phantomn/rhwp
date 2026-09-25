//! Document-IR ownership adapter. Paragraph composition is an explicit dependency,
//! not a fallback to Legacy table measurement or an interpretation of saved vpos.
use std::sync::Arc;

use crate::model::control::Control;
use crate::model::paragraph::{ColumnBreakType, Paragraph};
use crate::model::shape::{HorzAlign, HorzRelTo, SizeCriterion, TextWrap, VertAlign, VertRelTo};
use crate::model::table::{Table, TablePageBreak};

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
    /// An indivisible painted line and the non-table control slots it consumes.
    ObjectRow {
        line: usize,
        bounds: Rect,
        controls: Vec<usize>,
    },
    TableControl(usize),
    InlineTables {
        height: f64,
        advance: f64,
        tables: Vec<(usize, Rect)>,
    },
}

/// The implementation must include paragraph spacing, empty lines, explicit
/// breaks and text layout. This adapter does not qualify saved LineSeg caches.
/// The text adapter qualifies plain stored text and control-only stored TAC rows;
/// other stored ownership and general inline recomposition remain unsupported.
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
    /// Horizontal spans, zero-offset TopAndBottom and composed inline rows are
    /// admitted. Row spans require resolved row heights and intact cell groups.
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

/// Shared admission for the same paragraph-relative exclusion rule in a cell
/// and in a document body. It is not an inferred TAC/side-wrap policy.
pub(super) fn validate_anchor(table: &Table) -> Result<(), GeometryError> {
    let a = &table.common;
    if a.treat_as_char
        || a.text_wrap != TextWrap::TopAndBottom
        || a.vert_rel_to != VertRelTo::Para
        || a.vert_align != VertAlign::Top
        || a.horz_rel_to != HorzRelTo::Para
        || a.vertical_offset != 0
        || a.horizontal_offset != 0
        || !matches!(
            a.horz_align,
            HorzAlign::Left | HorzAlign::Center | HorzAlign::Right
        )
        || a.prevent_page_break != 0
        || [
            a.margin.left,
            a.margin.right,
            a.margin.top,
            a.margin.bottom,
            table.outer_margin_left,
            table.outer_margin_right,
            table.outer_margin_top,
            table.outer_margin_bottom,
        ]
        .iter()
        .any(|v| *v != 0)
    {
        return Err(GeometryError::Unsupported(
            "nested anchor, TAC, wrap or outer margin",
        ));
    }
    Ok(())
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
    let resolved = super::grid::resolve(table, scale)?;
    for cell in &table.cells {
        if cell.text_direction != 0 || cell.line_wrap != 0 {
            return Err(GeometryError::Unsupported("cell direction or line wrap"));
        }
    }
    // The preview admits only whole, contiguous leading header rows. Partial
    // header cells or scattered markers need a separate qualification;
    // do not infer their repetition from a row's incidental content.
    let mut header_rows = 0;
    if table.repeat_header {
        for (r, row) in resolved.rows.iter().enumerate() {
            let count = row.iter().filter(|c| c.is_header).count();
            if count != 0 {
                if count != row.len() || r != header_rows {
                    return Err(GeometryError::Unsupported(
                        "partial or non-leading header rows",
                    ));
                }
                header_rows += 1;
            }
        }
        if table.cells.iter().any(|cell| {
            usize::from(cell.row) < header_rows
                && usize::from(cell.row) + usize::from(cell.row_span) > header_rows
        }) {
            return Err(GeometryError::Unsupported(
                "header boundary crosses rowspan",
            ));
        }
    }
    let mut rows = Vec::with_capacity(resolved.rows.len());
    for (r, row) in resolved.rows.iter().enumerate() {
        let mut cells = Vec::with_capacity(row.len());
        for (c, cell) in row.iter().enumerate() {
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
            let inner_width = resolved.tracks[r][c].width - padding.left - padding.right;
            super::contracts::nonnegative(inner_width, "IR content width")?;
            let mut blocks = Vec::new();
            for (pi, para) in cell.paragraphs.iter().enumerate() {
                if para.column_type != ColumnBreakType::None {
                    return Err(GeometryError::Unsupported(
                        "explicit paragraph page/column break",
                    ));
                }
                for ctrl in &para.controls {
                    if matches!(ctrl, Control::Picture(_)) {
                        continue;
                    }
                    let Control::Table(child) = ctrl else {
                        return Err(GeometryError::Unsupported("non-table cell control"));
                    };
                    if !child.common.treat_as_char {
                        validate_anchor(child)?;
                    }
                }
                let mut seen = vec![false; para.controls.len()];
                for item in composer.compose(para, inner_width)? {
                    match item {
                        ParagraphItem::ObjectRow {
                            line,
                            bounds,
                            controls,
                        } => {
                            if controls.is_empty() {
                                return Err(GeometryError::InconsistentAtomicPlan);
                            }
                            for ci in controls {
                                if seen.get(ci).copied() != Some(false)
                                    || !matches!(para.controls.get(ci), Some(Control::Picture(_)))
                                {
                                    return Err(GeometryError::Unsupported(
                                        "invalid or repeated picture slot",
                                    ));
                                }
                                seen[ci] = true;
                            }
                            blocks.push(FlowBlock::Lines {
                                height: bounds.height,
                                lines: vec![LineBox {
                                    owner: LineOwner {
                                        paragraph: pi,
                                        line,
                                    },
                                    bounds,
                                }],
                            });
                        }
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
                            validate_anchor(child)?;
                            let plan = bind_table(child, scale, composer, depth + 1)?;
                            // Resolve against the same padded content width used by
                            // paragraph composition, not the page or outer cell.
                            let free_width = inner_width - plan.width;
                            if free_width < 0.0 {
                                return Err(GeometryError::ContentWidth {
                                    row: r,
                                    column: resolved.tracks[r][c].column,
                                });
                            }
                            let offset_x = match child.common.horz_align {
                                HorzAlign::Left => 0.0,
                                HorzAlign::Center => free_width / 2.0,
                                HorzAlign::Right => free_width,
                                _ => unreachable!("anchor qualified above"),
                            };
                            blocks.push(FlowBlock::Table {
                                owner: ControlOwner {
                                    paragraph: pi,
                                    control: ci,
                                },
                                offset_x,
                                plan: Arc::new(plan),
                            });
                            seen[ci] = true;
                        }
                        ParagraphItem::InlineTables {
                            height,
                            advance,
                            tables,
                        } => {
                            let mut bound = Vec::new();
                            for (ci, rect) in tables {
                                if seen.get(ci).copied() != Some(false) {
                                    return Err(GeometryError::Unsupported(
                                        "invalid or repeated inline slot",
                                    ));
                                }
                                let Control::Table(child) = &para.controls[ci] else {
                                    unreachable!()
                                };
                                if !child.common.treat_as_char {
                                    return Err(GeometryError::Unsupported(
                                        "non-inline table in inline row",
                                    ));
                                }
                                let plan = bind_table(child, scale, composer, depth + 1)?;
                                bound.push(super::tac::bind(
                                    ControlOwner {
                                        paragraph: pi,
                                        control: ci,
                                    },
                                    rect,
                                    Arc::new(plan),
                                )?);
                                seen[ci] = true;
                            }
                            blocks.push(FlowBlock::InlineTables {
                                height,
                                advance,
                                tables: bound,
                            });
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
    let mut plan =
        TableContentPlan::from_grid_rows(resolved.tracks, resolved.width, rows, 0.0, policy)?;
    plan.header_rows = header_rows;
    Ok(plan)
}
