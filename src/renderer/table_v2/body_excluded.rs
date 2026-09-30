//! Shared composition for a saved zero-width owner and its positioned table.
//! The host owns page advancement; offset/margins and continuation belong to
//! this same flow block in standalone and section-hosted execution.
use super::{
    body_inline::InlineParagraph, CellEndPolicy, ControlOwner, FlowBlock, GeometryError, LineBox,
    LineOwner, ParagraphItem, PreparedTextTable,
};
use crate::{
    model::{control::Control, document::Document, paragraph::Paragraph},
    renderer::{hwpunit_to_px, render_tree::RenderNodeType, style_resolver::ResolvedStyleSet},
};

#[allow(clippy::too_many_arguments)]
pub(super) fn prepare(
    document: &Document,
    source: &Paragraph,
    pi: usize,
    width: f64,
    paper_right: f64,
    styles: &ResolvedStyleSet,
    dpi: f64,
    policy: CellEndPolicy,
) -> Result<InlineParagraph, GeometryError> {
    super::decoration::validate_paragraph_source(source.para_shape_id, &document.doc_info)?;
    let (item, mut node) = super::cell_anchor::compose_body(source, width, styles, dpi)?;
    let ParagraphItem::ExcludedTable {
        control,
        line,
        host,
        host_advance,
        x,
        offset_y,
        top,
        bottom,
    } = item
    else {
        return Err(GeometryError::InconsistentAtomicPlan);
    };
    let Control::Table(table) = &source.controls[control] else {
        return Err(GeometryError::InconsistentAtomicPlan);
    };
    super::decoration::validate_source(table, &document.doc_info)?;
    let prepared = PreparedTextTable::prepare_with_end_policy(
        table,
        styles,
        dpi,
        &document.bin_data_content,
        policy,
    )?;
    // The host supplies the physical paper edge relative to this lane. A
    // floating table may extend beyond body width, but is never scaled to fit.
    let right = hwpunit_to_px(i32::from(table.common.margin.right), dpi);
    let end = x + prepared.plan.width + right;
    let roundoff = 4.0 * f64::EPSILON * end.abs().max(paper_right.abs());
    if end - paper_right > roundoff {
        return Err(GeometryError::Unsupported(
            "stored body anchor outside paper",
        ));
    }
    let owner = ControlOwner {
        paragraph: pi,
        control,
    };
    let line_owner = LineOwner {
        paragraph: pi,
        line,
    };
    if let RenderNodeType::TextLine(value) = &mut node.node_type {
        value.section_index = Some(0);
        value.para_index = Some(pi);
    }
    Ok(InlineParagraph {
        lines: [(line_owner, (0, node))].into(),
        tables: [(owner, (1, prepared.paint))].into(),
        blocks: vec![FlowBlock::AnchoredTable {
            owner,
            host: Some(LineBox {
                owner: line_owner,
                bounds: host,
            }),
            host_advance,
            offset_x: x,
            offset_y,
            available_width: prepared.plan.width,
            top,
            bottom,
            plan: prepared.plan,
        }],
    })
}
