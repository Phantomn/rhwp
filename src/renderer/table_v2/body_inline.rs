//! Shared TAC paragraph composition for standalone and hosted body flow.
//! Produces immutable line/table payloads and the exact blocks consumed by fit.
use super::{
    text::TextPaint, CellEndPolicy, ControlOwner, FlowBlock, GeometryError, LineBox, LineOwner,
    ParagraphItem, PreparedTextTable,
};
use crate::{
    model::{control::Control, document::Document, paragraph::Paragraph},
    renderer::{render_tree::RenderNode, style_resolver::ResolvedStyleSet},
};
use std::{collections::HashMap, sync::Arc};

pub(super) struct InlineParagraph {
    pub blocks: Vec<FlowBlock>,
    pub lines: HashMap<LineOwner, (usize, RenderNode)>,
    pub tables: HashMap<ControlOwner, (usize, Arc<TextPaint>)>,
}
#[allow(clippy::too_many_arguments)]
pub(super) fn prepare(
    document: &Document,
    source: &Paragraph,
    pi: usize,
    width: f64,
    styles: &ResolvedStyleSet,
    dpi: f64,
    cell_end_policy: CellEndPolicy,
    frame_end: bool,
) -> Result<InlineParagraph, GeometryError> {
    let mut blocks = Vec::new();
    let mut lines = HashMap::new();
    let mut tables = HashMap::new();
    let mut order = 0;
    // Fresh rows need the child's actual occupied box, not a stale
    // object declaration left over after editing. Retain each prepared
    // plan/paint pair; binding below consumes this exact result once.
    // Saved rows continue through their independent LineSeg contract.
    let mut prepared_children = HashMap::new();
    let (items, nodes) = if source.line_segs.is_empty() {
        let mut dimensions = Vec::new();
        for (ci, control) in source.controls.iter().enumerate() {
            if let Control::Table(table) = control {
                super::decoration::validate_source(table, &document.doc_info)?;
                let prepared = PreparedTextTable::prepare_with_end_policy(
                    table,
                    styles,
                    dpi,
                    &document.bin_data_content,
                    cell_end_policy,
                )?;
                dimensions.push((ci, prepared.plan.width, prepared.plan.height));
                prepared_children.insert(ci, prepared);
            }
        }
        super::tac_fresh::compose_with_dimensions(source, width, styles, dpi, &dimensions)?
    } else {
        super::tac::compose(source, width, styles, dpi)?
    };
    for (line, node) in nodes.into_iter().enumerate() {
        lines.insert(
            LineOwner {
                paragraph: pi,
                line,
            },
            (order, node),
        );
        order += 1;
    }
    // A TAC's following-line gap locates the next line in this body
    // frame. It is not a physical blank band after the story ends or
    // before an explicit page break. Keep paragraph-after spacing,
    // authored blank lines and object margins; do not trim raw Space.
    let items = if frame_end {
        super::paragraph_end::into_flow_items_at_end(
            items,
            super::CellEndPolicy::OmitFinalLineGap,
            true,
        )
    } else {
        items
    };
    for item in items {
        match item {
            ParagraphItem::Space(h) => blocks.push(FlowBlock::Space(h)),
            ParagraphItem::End(end) => blocks.extend(end.into_body_tail()),
            ParagraphItem::Lines {
                height,
                advance,
                lines: owned,
            } => {
                blocks.push(FlowBlock::Lines {
                    height,
                    advance,
                    lines: owned
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
            ParagraphItem::InlineTables {
                height,
                advance,
                tables: owned,
                lines: inline_lines,
            } => {
                let mut bound = Vec::new();
                for (ci, rect) in owned {
                    let Control::Table(table) = &source.controls[ci] else {
                        unreachable!()
                    };
                    super::decoration::validate_source(table, &document.doc_info)?;
                    let prepared = if let Some(prepared) = prepared_children.remove(&ci) {
                        prepared
                    } else {
                        if source.line_segs.is_empty() {
                            return Err(GeometryError::InconsistentAtomicPlan);
                        }
                        PreparedTextTable::prepare_with_end_policy(
                            table,
                            styles,
                            dpi,
                            &document.bin_data_content,
                            cell_end_policy,
                        )?
                    };
                    let owner = ControlOwner {
                        paragraph: pi,
                        control: ci,
                    };
                    bound.push(super::tac::bind(owner, rect, prepared.plan)?);
                    tables.insert(owner, (order, prepared.paint));
                    order += 1;
                }
                blocks.push(FlowBlock::InlineTables {
                    height: super::tac::bound_height(height, &bound),
                    advance,
                    tables: bound,
                    lines: inline_lines
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
            _ => return Err(GeometryError::InconsistentAtomicPlan),
        }
    }
    Ok(InlineParagraph {
        blocks,
        lines,
        tables,
    })
}
