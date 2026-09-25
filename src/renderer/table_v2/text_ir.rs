//! Fresh IR content binding for V2 previews. An anchored TopAndBottom
//! child excludes the whole host width; at paragraph-top + zero offset the host
//! lines therefore start below the child. Qualified stored TAC carriers use a
//! separate row composition; side-wrap and fresh inline composition are unsupported.
use std::{cell::RefCell, collections::HashMap, sync::Arc};

use crate::{
    model::{control::Control, paragraph::Paragraph, table::Table},
    renderer::{render_tree::RenderNode, style_resolver::ResolvedStyleSet},
};

use super::{
    text::{validate_text_context, OrderedPaint, TextComposer, TextPaint},
    CellParagraphComposer, GeometryError, ParagraphItem, PreparedTextTable, TableContentPlan,
};

pub(super) fn prepare(
    table: &Table,
    styles: &ResolvedStyleSet,
    dpi: f64,
    resources: &[crate::model::bin_data::BinDataContent],
    policy: super::CellEndPolicy,
) -> Result<PreparedTextTable, GeometryError> {
    validate_text_context(styles, dpi)?;
    let composer = IrTextComposer {
        resources,
        text: TextComposer {
            styles,
            dpi,
            payloads: RefCell::new(Vec::new()),
        },
        paragraphs: RefCell::new(Vec::new()),
    };
    // IR binding qualifies the grid, child anchors, margins, and depth BEFORE
    // each compose call. Composition records host payloads before child payloads.
    let plan =
        TableContentPlan::from_ir_contents_with_end_policy(table, dpi / 7200.0, &composer, policy)?;
    let mut payloads = composer.paragraphs.into_inner().into_iter();
    let paint = bind_paint(table, styles, dpi, &mut payloads)?;
    if payloads.next().is_some() {
        return Err(GeometryError::InconsistentAtomicPlan);
    }
    Ok(PreparedTextTable {
        plan: Arc::new(plan),
        paint: Arc::new(paint),
        dpi,
    })
}

struct IrTextComposer<'a> {
    resources: &'a [crate::model::bin_data::BinDataContent],
    text: TextComposer<'a>,
    paragraphs: RefCell<Vec<ParagraphPaint>>,
}

enum PaintSlot {
    Line(usize),
    Table(usize),
}

struct ParagraphPaint {
    slots: Vec<PaintSlot>,
    lines: Vec<RenderNode>,
}

impl CellParagraphComposer for IrTextComposer<'_> {
    fn compose(&self, para: &Paragraph, width: f64) -> Result<Vec<ParagraphItem>, GeometryError> {
        let items = self.compose_items(para, width)?;
        // Store the very same ordered ownership recipe that geometry consumes.
        // Paint must not reconstruct slot order from the IR control array later.
        let mut slots = Vec::new();
        for item in &items {
            match item {
                ParagraphItem::ObjectRow { line, .. } => slots.push(PaintSlot::Line(*line)),
                ParagraphItem::Lines { lines, .. } => {
                    slots.extend(lines.iter().map(|(line, _)| PaintSlot::Line(*line)))
                }
                ParagraphItem::TableControl(ci) => slots.push(PaintSlot::Table(*ci)),
                ParagraphItem::InlineTables { tables, .. } => {
                    slots.extend(tables.iter().map(|(ci, _)| PaintSlot::Table(*ci)));
                }
                ParagraphItem::Space(_) | ParagraphItem::End(_) => {}
            }
        }
        let lines = self
            .text
            .payloads
            .borrow_mut()
            .pop()
            .ok_or(GeometryError::InconsistentAtomicPlan)?;
        self.paragraphs
            .borrow_mut()
            .push(ParagraphPaint { slots, lines });
        Ok(items)
    }
}

impl IrTextComposer<'_> {
    fn compose_items(
        &self,
        para: &Paragraph,
        width: f64,
    ) -> Result<Vec<ParagraphItem>, GeometryError> {
        if para.controls.is_empty() {
            return self.text.compose(para, width);
        }
        if para
            .controls
            .iter()
            .all(|c| matches!(c, Control::Picture(_)))
        {
            let (items, nodes) = super::pictures::compose(
                para,
                width,
                self.text.styles,
                self.text.dpi,
                self.resources,
            )?;
            self.text.payloads.borrow_mut().push(nodes);
            return Ok(items);
        }
        if para
            .controls
            .iter()
            .all(|c| matches!(c, Control::Table(t) if t.common.treat_as_char))
        {
            let items = super::tac::compose(para, width, self.text.styles, self.text.dpi)?;
            self.text.payloads.borrow_mut().push(Vec::new());
            return Ok(items);
        }
        if !para.line_segs.is_empty() {
            return Err(GeometryError::Unsupported("stored child anchor ownership"));
        }
        // Two floating objects at the same anchor need overlap/avoidance rules,
        // not declaration-order stacking. Leave that input explicitly unsupported.
        if para.controls.len() != 1 || !matches!(&para.controls[0], Control::Table(_)) {
            return Err(GeometryError::Unsupported(
                "multiple anchored cell controls",
            ));
        }
        let style = self
            .text
            .styles
            .para_styles
            .get(para.para_shape_id as usize)
            .ok_or(GeometryError::Unsupported("missing paragraph style"))?;
        // These change the paragraph's anchor/frame relative to the cell origin.
        // Do not silently apply the child at the cell origin with different text
        // insets, or move paragraph-before spacing to below the child.
        if [
            style.margin_left,
            style.margin_right,
            style.indent,
            style.spacing_before,
        ]
        .iter()
        .any(|v| *v != 0.0)
        {
            return Err(GeometryError::Unsupported("anchored host paragraph insets"));
        }
        let mut text_only = para.clone();
        text_only.controls.clear();
        text_only.ctrl_data_records.clear();
        // Preserve UTF-16 offsets and char-shape references. The non-inline
        // object is not a replacement character in the composed host text.
        // Saved rows are NOT cleared; the fresh composer still rejects them.
        let mut items = vec![ParagraphItem::TableControl(0)];
        items.extend(self.text.compose(&text_only, width)?);
        Ok(items)
    }
}

fn bind_paint(
    table: &Table,
    styles: &ResolvedStyleSet,
    dpi: f64,
    payloads: &mut impl Iterator<Item = ParagraphPaint>,
) -> Result<TextPaint, GeometryError> {
    // Depth/grid/one-control admission has already succeeded in from_ir_contents.
    if !table.zones.is_empty() {
        return Err(GeometryError::Unsupported("V2 table background zones"));
    }
    let mut paint = TextPaint {
        rows: table.row_count,
        columns: table.col_count,
        lines: HashMap::new(),
        tables: HashMap::new(),
        background: super::decoration::Background::resolve(
            table.border_fill_id,
            styles,
            table.page_break == crate::model::table::TablePageBreak::None,
        )?,
        cells: HashMap::new(),
        borders: super::borders::CellBorders::prepare(table, styles, dpi)?,
    };
    let mut cells: Vec<_> = table.cells.iter().collect();
    cells.sort_by_key(|c| (c.row, c.col));
    for cell in cells {
        paint.cells.insert(
            (usize::from(cell.row), usize::from(cell.col)),
            super::decoration::Background::resolve_cell(
                cell.border_fill_id,
                styles,
                table.page_break != crate::model::table::TablePageBreak::CellBreak,
            )?,
        );
        let mut order = 0;
        for (pi, para) in cell.paragraphs.iter().enumerate() {
            let host = payloads
                .next()
                .ok_or(GeometryError::InconsistentAtomicPlan)?;
            let mut lines: Vec<_> = host.lines.into_iter().map(Some).collect();
            for slot in host.slots {
                match slot {
                    PaintSlot::Table(ci) => {
                        let Some(Control::Table(child)) = para.controls.get(ci) else {
                            return Err(GeometryError::InconsistentAtomicPlan);
                        };
                        paint.tables.insert(
                            (cell.row as usize, cell.col as usize, pi, ci),
                            OrderedPaint {
                                order,
                                value: Arc::new(bind_paint(child, styles, dpi, payloads)?),
                            },
                        );
                    }
                    PaintSlot::Line(li) => {
                        let value = lines
                            .get_mut(li)
                            .and_then(Option::take)
                            .ok_or(GeometryError::InconsistentAtomicPlan)?;
                        paint.lines.insert(
                            (cell.row as usize, cell.col as usize, pi, li),
                            OrderedPaint { order, value },
                        );
                    }
                }
                order += 1;
            }
            if lines.iter().any(Option::is_some) {
                return Err(GeometryError::InconsistentAtomicPlan);
            }
        }
    }
    Ok(paint)
}
