//! Explicit cell-flow assembly. This is downstream of anchor/line ownership:
//! it does not infer TAC, wrapping, saved LineSeg, or floating positions from IR.
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::{model::paragraph::Paragraph, renderer::style_resolver::ResolvedStyleSet};

use super::{
    text::{validate_text_context, OrderedPaint, TextComposer, TextPaint},
    CellParagraphComposer, ControlOwner, FlowBlock, FlowCellInput, FlowRowInput, GeometryError,
    Insets, LineBox, LineOwner, ParagraphItem, PreparedTextTable, SplitPolicy, TableContentPlan,
};

/// Explicit sequential flow, not an interpretation of Paragraph.controls.
/// A blank paragraph remains a composed line; Space is additional physical space.
/// A child retains its immutable plan AND paint, never a cursor partway through it.
pub enum TextFlowBlock {
    Space(f64),
    Paragraph {
        owner: usize,
        paragraph: Box<Paragraph>,
    },
    Table {
        owner: ControlOwner,
        table: PreparedTextTable,
    },
}

pub struct TextFlowCell {
    pub padding: Insets,
    pub minimum_height: f64,
    pub blocks: Vec<TextFlowBlock>,
}

pub struct TextFlowRow {
    pub cells: Vec<TextFlowCell>,
}

impl PreparedTextTable {
    /// Compose fresh paragraphs at each cell's effective width and bind nested
    /// snapshots. All dimensions are pixels at `dpi`. Column/row order is explicit;
    /// spans, borders, document anchors and repeat headers are not represented.
    /// This API requires an already resolved sequential flow. In particular, a
    /// floating control's host paragraph must NOT be blindly placed before it.
    pub fn from_flow_rows(
        column_widths: Vec<f64>,
        rows: Vec<TextFlowRow>,
        row_spacing: f64,
        policy: SplitPolicy,
        styles: &ResolvedStyleSet,
        dpi: f64,
    ) -> Result<Self, GeometryError> {
        validate_text_context(styles, dpi)?;
        let row_count =
            u16::try_from(rows.len()).map_err(|_| GeometryError::Unsupported("row count"))?;
        let columns = u16::try_from(column_widths.len())
            .map_err(|_| GeometryError::Unsupported("column count"))?;
        let mut paint = TextPaint {
            rows: row_count,
            columns,
            lines: HashMap::new(),
            tables: HashMap::new(),
            background: Default::default(),
            cells: HashMap::new(),
            borders: None,
        };
        let composer = TextComposer {
            styles,
            dpi,
            payloads: RefCell::new(Vec::new()),
        };
        let mut flow_rows = Vec::new();
        for (r, row) in rows.into_iter().enumerate() {
            if row.cells.len() != column_widths.len() {
                return Err(GeometryError::CellCount { row: r });
            }
            let mut cells = Vec::new();
            for (c, cell) in row.cells.into_iter().enumerate() {
                let width = column_widths[c] - cell.padding.left - cell.padding.right;
                super::contracts::nonnegative(width, "text flow content width")?;
                let mut blocks = Vec::new();
                let mut paragraphs = HashSet::new();
                let mut order = 0;
                for block in cell.blocks {
                    match block {
                        TextFlowBlock::Space(height) => blocks.push(FlowBlock::Space(height)),
                        TextFlowBlock::Paragraph { owner, paragraph } => {
                            if !paragraphs.insert(owner) {
                                return Err(GeometryError::Unsupported(
                                    "duplicate paragraph owner",
                                ));
                            }
                            for item in composer.compose(&paragraph, width)? {
                                match item {
                                    ParagraphItem::Space(h) => blocks.push(FlowBlock::Space(h)),
                                    ParagraphItem::Lines { height, lines } => {
                                        blocks.push(FlowBlock::Lines {
                                            height,
                                            lines: lines
                                                .into_iter()
                                                .map(|(line, bounds)| LineBox {
                                                    owner: LineOwner {
                                                        paragraph: owner,
                                                        line,
                                                    },
                                                    bounds,
                                                })
                                                .collect(),
                                        })
                                    }
                                    ParagraphItem::TableControl(_) => {
                                        return Err(GeometryError::InconsistentAtomicPlan)
                                    }
                                }
                            }
                            let payloads = composer
                                .payloads
                                .borrow_mut()
                                .pop()
                                .ok_or(GeometryError::InconsistentAtomicPlan)?;
                            for (line, value) in payloads.into_iter().enumerate() {
                                paint
                                    .lines
                                    .insert((r, c, owner, line), OrderedPaint { order, value });
                                order += 1;
                            }
                        }
                        TextFlowBlock::Table { owner, table } => {
                            if table.dpi != dpi {
                                return Err(GeometryError::Unsupported(
                                    "nested text flow DPI mismatch",
                                ));
                            }
                            if paint
                                .tables
                                .insert(
                                    (r, c, owner.paragraph, owner.control),
                                    OrderedPaint {
                                        order,
                                        value: table.paint,
                                    },
                                )
                                .is_some()
                            {
                                return Err(GeometryError::Unsupported("duplicate table owner"));
                            }
                            order += 1;
                            blocks.push(FlowBlock::Table {
                                offset_x: 0.0,
                                owner,
                                plan: table.plan,
                            });
                        }
                    }
                }
                cells.push(FlowCellInput {
                    padding: cell.padding,
                    minimum_height: cell.minimum_height,
                    width,
                    blocks,
                });
            }
            flow_rows.push(FlowRowInput { cells });
        }
        let plan = TableContentPlan::from_flow_rows(column_widths, flow_rows, row_spacing, policy)?;
        Ok(Self {
            plan: Arc::new(plan),
            paint: Arc::new(paint),
            dpi,
        })
    }
}
