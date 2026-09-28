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
    /// A paragraph-top exclusion followed by its preserved host text lines.
    PositionedTable {
        control: usize,
        x: f64,
        top: f64,
        bottom: f64,
    },
    ExcludedTable {
        control: usize,
        line: usize,
        host: Rect,
        host_advance: f64,
        x: f64,
        offset_y: f64,
        top: f64,
        bottom: f64,
    },
    Space(f64),
    /// Producer-owned paragraph ending, distinct from additional cell space.
    End(super::ParagraphEnd),
    Lines {
        height: f64,
        advance: f64,
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
        /// Text payloads sharing this atomic row and its single flow advance.
        lines: Vec<(usize, Rect)>,
    },
}

/// The implementation must include paragraph spacing, empty lines, explicit
/// breaks and text layout. This adapter does not qualify saved LineSeg caches.
/// The text adapter qualifies plain stored text and control-only stored TAC rows;
/// other stored ownership and general inline recomposition remain unsupported.
pub trait CellParagraphComposer {
    /// Cell wrapping is a composition policy, not a grid property to ignore.
    /// Custom composers must opt in to non-default policies explicitly.
    fn compose_with_cell_wrap(
        &self,
        paragraph: &Paragraph,
        width: f64,
        single_column: bool,
        line_wrap: u8,
    ) -> Result<Vec<ParagraphItem>, GeometryError> {
        if line_wrap != 0 {
            return Err(GeometryError::Unsupported("cell line wrap policy"));
        }
        self.compose_in_cell(paragraph, width, single_column)
    }

    /// Candidate transitions owned by the preceding exclusion host. Only the
    /// stored composer supplies these; binding still verifies the child cuts.
    fn stored_child_frame_tails(
        &self,
        _paragraphs: &[Paragraph],
    ) -> Result<Vec<(usize, f64)>, GeometryError> {
        Ok(Vec::new())
    }

    /// The adapter qualifies a cell-local single column once for its story.
    /// It remains the reference of later anchored objects, independently of
    /// those paragraphs' text margins. Custom composers may ignore this frame.
    fn compose_in_cell(
        &self,
        paragraph: &Paragraph,
        width: f64,
        _single_column: bool,
    ) -> Result<Vec<ParagraphItem>, GeometryError> {
        self.compose(paragraph, width)
    }

    /// Source cuts qualified against the same styles used by composition.
    /// A custom/fresh composer must not inherit stale page-frame boundaries.
    fn stored_frame_starts(
        &self,
        _paragraphs: &[Paragraph],
    ) -> Result<Vec<(usize, usize)>, GeometryError> {
        Ok(Vec::new())
    }

    fn compose(
        &self,
        paragraph: &Paragraph,
        width: f64,
    ) -> Result<Vec<ParagraphItem>, GeometryError>;
}

// A cell starts a new local story. Its initial normal one-column declaration
// establishes that same lane; MultiColumn is not a request for a new page.
// Other zones require an actual column-flow implementation. Keep the source
// slot intact so the nested table and text retain their original UTF-16 owners.
pub(super) fn initial_cell_column(para: &Paragraph, pi: usize) -> Result<bool, GeometryError> {
    let count = para
        .controls
        .iter()
        .filter(|c| matches!(c, Control::ColumnDef(_)))
        .count();
    if count == 0 {
        return Ok(false);
    }
    let Some(Control::ColumnDef(cd)) = para.controls.first() else {
        return Err(GeometryError::Unsupported(
            "noninitial cell column definition",
        ));
    };
    if pi != 0
        || count != 1
        || !matches!(
            para.column_type,
            ColumnBreakType::None | ColumnBreakType::MultiColumn
        )
        || cd.column_type != crate::model::page::ColumnType::Normal
        || cd.column_count != 1
        || !cd.same_width
        || cd.spacing != 0
        || !cd.widths.is_empty()
        || !cd.gaps.is_empty()
        || cd.separator_type != 0
        || para.control_utf16_positions().first().copied() != Some(0)
    {
        return Err(GeometryError::Unsupported(
            "noninitial or multi-lane cell column definition",
        ));
    }
    Ok(true)
}

// The historical IR variant names are not pagination algorithms. HWP5 value2
// (RowBreak, HWPX CELL, UI "나눔") breaks at composed lines; value1
// (CellBreak, HWPX TABLE, UI "셀 단위로 나눔") moves the complete cell.
// Keep this translation shared with paint admission rather than matching enum
// names independently in each consumer. The parser/writer values stay intact.
impl From<TablePageBreak> for SplitPolicy {
    fn from(value: TablePageBreak) -> Self {
        match value {
            TablePageBreak::None => Self::Never,
            TablePageBreak::RowBreak => Self::WithinCells,
            TablePageBreak::CellBreak => Self::BetweenRows,
        }
    }
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
        Self::from_ir_contents_with_end_policy(
            table,
            units_per_hwp,
            composer,
            super::CellEndPolicy::default(),
        )
    }

    pub fn from_ir_contents_with_end_policy(
        table: &Table,
        units_per_hwp: f64,
        composer: &impl CellParagraphComposer,
        policy: super::CellEndPolicy,
    ) -> Result<Self, GeometryError> {
        if !units_per_hwp.is_finite() || units_per_hwp <= 0.0 {
            return Err(GeometryError::InvalidNumber("HWP unit scale"));
        }
        bind_table(table, units_per_hwp, composer, 0, policy)
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
    policy: super::CellEndPolicy,
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
    let mut resolved = super::grid::resolve(table, scale)?;
    for cell in &table.cells {
        if cell.text_direction != 0 {
            return Err(GeometryError::Unsupported("cell text direction"));
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
            let text_width = super::stored_text::cell_lane_width(
                &cell.paragraphs,
                resolved.tracks[r][c].width,
                padding,
                scale,
            )?;
            resolved.tracks[r][c].text_width = text_width;
            let stored_frames = composer.stored_frame_starts(&cell.paragraphs)?;
            let child_tails = composer.stored_child_frame_tails(&cell.paragraphs)?;
            let single_column = cell
                .paragraphs
                .first()
                .map(|p| initial_cell_column(p, 0))
                .transpose()?
                .unwrap_or(false);
            let mut blocks = Vec::new();
            for (pi, para) in cell.paragraphs.iter().enumerate() {
                let initial_column = initial_cell_column(para, pi)?;
                let local;
                let para = if initial_column {
                    local = {
                        let mut p = para.clone();
                        p.column_type = ColumnBreakType::None;
                        p
                    };
                    &local
                } else {
                    para
                };
                if stored_frames.contains(&(pi, 0)) {
                    blocks.push(FlowBlock::StoredFrameStart);
                }
                if para.column_type != ColumnBreakType::None {
                    return Err(GeometryError::Unsupported(
                        "explicit paragraph page/column break",
                    ));
                }
                let stored_fields = super::fields::stored_result(para)?;
                let page_number = super::page_number::cell_declaration(para)?;
                let tac_fields = super::fields::stored_tac_prefix(para);
                for ctrl in &para.controls {
                    if matches!(ctrl, Control::Picture(_))
                        || (page_number && matches!(ctrl, Control::PageNumberPos(_)))
                        || (initial_column && matches!(ctrl, Control::ColumnDef(_)))
                        || stored_fields
                        || (tac_fields && matches!(ctrl, Control::Field(_)))
                    {
                        continue;
                    }
                    let Control::Table(child) = ctrl else {
                        return Err(GeometryError::Unsupported("non-table cell control"));
                    };
                    if !child.common.treat_as_char
                        && !super::cell_anchor::candidate(para)
                        && !super::cell_anchor::following_candidate(para)
                    {
                        validate_anchor(child)?;
                    }
                }
                // Qualified fields are source markers replayed by the text
                // composer, not table owners awaiting a geometry item.
                let mut seen = vec![stored_fields || page_number; para.controls.len()];
                if tac_fields {
                    for (ci, control) in para.controls.iter().enumerate() {
                        if matches!(control, Control::Field(_)) {
                            seen[ci] = true;
                        }
                    }
                }
                if initial_column {
                    seen[0] = true;
                }
                let line_starts: Vec<_> = stored_frames
                    .iter()
                    .filter_map(|&(p, l)| (p == pi && l > 0).then_some(l))
                    .collect();
                let continuous;
                let text_para = if line_starts.is_empty() {
                    para
                } else {
                    continuous = super::stored_text::continuous_paragraph(para, &line_starts)?;
                    &continuous
                };
                let items = composer.compose_with_cell_wrap(
                    text_para,
                    text_width.unwrap_or(inner_width),
                    single_column,
                    cell.line_wrap,
                )?;
                let items = if stored_frames.contains(&(pi + 1, 0)) {
                    // Keep the ending typed until the actual fit query knows
                    // whether this frame is split or placed intact.
                    items
                } else {
                    super::paragraph_end::into_flow_items_at_end(
                        items,
                        policy,
                        pi + 1 == cell.paragraphs.len(),
                    )
                };
                for item in items {
                    match item {
                        ParagraphItem::PositionedTable {
                            control: ci,
                            x,
                            top,
                            bottom,
                        } => {
                            if seen.get(ci).copied() != Some(false) {
                                return Err(GeometryError::Unsupported(
                                    "invalid anchored table slot",
                                ));
                            }
                            let Control::Table(child) = &para.controls[ci] else {
                                return Err(GeometryError::Unsupported("non-table anchor slot"));
                            };
                            let plan = bind_table(child, scale, composer, depth + 1, policy)?;
                            // TopAndBottom already excludes the whole text lane.
                            // Its right avoidance margin is not extra table ink:
                            // require the positioned border box to fit the cell.
                            if x + plan.width > inner_width {
                                return Err(GeometryError::ContentWidth {
                                    row: r,
                                    column: resolved.tracks[r][c].column,
                                });
                            }
                            blocks.push(FlowBlock::AnchoredTable {
                                owner: ControlOwner {
                                    paragraph: pi,
                                    control: ci,
                                },
                                host: None,
                                host_advance: 0.0,
                                offset_x: x,
                                offset_y: 0.0,
                                available_width: plan.width,
                                top,
                                bottom,
                                plan: Arc::new(plan),
                            });
                            seen[ci] = true;
                        }
                        ParagraphItem::ExcludedTable {
                            control: ci,
                            line,
                            host,
                            host_advance,
                            x,
                            offset_y,
                            top,
                            bottom,
                        } => {
                            if seen.get(ci).copied() != Some(false) {
                                return Err(GeometryError::Unsupported(
                                    "invalid anchored table slot",
                                ));
                            }
                            let Control::Table(child) = &para.controls[ci] else {
                                return Err(GeometryError::Unsupported("non-table anchor slot"));
                            };
                            let mut plan = bind_table(child, scale, composer, depth + 1, policy)?;
                            if let Some((_, tail)) = child_tails.iter().find(|(p, _)| *p == pi) {
                                super::stored_child::qualify(
                                    &mut plan, child, scale, *tail, top, bottom,
                                )?;
                            }
                            // As above, side clearance does not enlarge the
                            // physical table when the whole lane is excluded.
                            if x + plan.width > inner_width {
                                return Err(GeometryError::ContentWidth {
                                    row: r,
                                    column: resolved.tracks[r][c].column,
                                });
                            }
                            blocks.push(FlowBlock::AnchoredTable {
                                owner: ControlOwner {
                                    paragraph: pi,
                                    control: ci,
                                },
                                host: Some(LineBox {
                                    owner: LineOwner {
                                        paragraph: pi,
                                        line,
                                    },
                                    bounds: host,
                                }),
                                host_advance,
                                offset_x: x,
                                offset_y,
                                available_width: plan.width,
                                top,
                                bottom,
                                plan: Arc::new(plan),
                            });
                            seen[ci] = true;
                        }
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
                                advance: bounds.height,
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
                        ParagraphItem::End(end) => blocks.extend(end.into_stored_frame_tail()),
                        ParagraphItem::Lines {
                            height,
                            advance,
                            lines,
                        } => {
                            if lines.iter().any(|(li, _)| line_starts.contains(li)) {
                                if lines.len() != 1 {
                                    return Err(GeometryError::Unsupported(
                                        "stored frame inside atomic row",
                                    ));
                                }
                                // The producer's interline band remains part of
                                // intact/Never flow, but not the split-frame end.
                                // It is not an authored empty line or paragraph.
                                let mut spaces = Vec::new();
                                while let Some(FlowBlock::Space(_)) = blocks.last() {
                                    let Some(FlowBlock::Space(h)) = blocks.pop() else {
                                        unreachable!()
                                    };
                                    spaces.push(h);
                                }
                                spaces.reverse();
                                blocks.push(FlowBlock::StoredFrameTail {
                                    spaces,
                                    paragraph_after: 0.0,
                                });
                                blocks.push(FlowBlock::StoredFrameStart);
                            }
                            blocks.push(FlowBlock::Lines {
                                height,
                                advance,
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
                            });
                        }
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
                            let plan = bind_table(child, scale, composer, depth + 1, policy)?;
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
                                restart_top: 0.0,
                                plan: Arc::new(plan),
                            });
                            seen[ci] = true;
                        }
                        ParagraphItem::InlineTables {
                            height,
                            advance,
                            tables,
                            lines,
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
                                let plan = bind_table(child, scale, composer, depth + 1, policy)?;
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
                                height: super::tac::bound_height(height, &bound),
                                advance,
                                tables: bound,
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
    let policy = SplitPolicy::from(table.page_break);
    let mut plan =
        TableContentPlan::from_grid_rows(resolved.tracks, resolved.width, rows, 0.0, policy)?;
    plan.header_rows = header_rows;
    Ok(plan)
}
