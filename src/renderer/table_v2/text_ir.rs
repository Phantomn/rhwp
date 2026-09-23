//! Fresh IR content binding for borderless V2 previews. An anchored TopAndBottom
//! child excludes the whole host width; at paragraph-top + zero offset the host
//! lines therefore start below the child. This is NOT the TAC/side-wrap rule.
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
) -> Result<PreparedTextTable, GeometryError> {
    validate_text_context(styles, dpi)?;
    let composer = IrTextComposer {
        text: TextComposer {
            styles,
            dpi,
            payloads: RefCell::new(Vec::new()),
        },
        paragraphs: RefCell::new(Vec::new()),
    };
    // IR binding qualifies the grid, child anchors, margins, and depth BEFORE
    // each compose call. Composition records host payloads before child payloads.
    let plan = TableContentPlan::from_ir_contents(table, dpi / 7200.0, &composer)?;
    let mut payloads = composer.paragraphs.into_inner().into_iter();
    let paint = bind_paint(table, &mut payloads)?;
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
                ParagraphItem::Lines { lines, .. } => {
                    slots.extend(lines.iter().map(|(line, _)| PaintSlot::Line(*line)))
                }
                ParagraphItem::TableControl(ci) => slots.push(PaintSlot::Table(*ci)),
                ParagraphItem::Space(_) => {}
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
    payloads: &mut impl Iterator<Item = ParagraphPaint>,
) -> Result<TextPaint, GeometryError> {
    // Depth/grid/one-control admission has already succeeded in from_ir_contents.
    // Reject decorations recursively rather than silently omitting child borders.
    if table.border_fill_id != 0
        || table.cells.iter().any(|c| c.border_fill_id != 0)
        || !table.zones.is_empty()
    {
        return Err(GeometryError::Unsupported("text preview table borders"));
    }
    let mut paint = TextPaint {
        rows: table.row_count,
        columns: table.col_count,
        lines: HashMap::new(),
        tables: HashMap::new(),
    };
    let mut cells: Vec<_> = table.cells.iter().collect();
    cells.sort_by_key(|c| (c.row, c.col));
    for cell in cells {
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
                                value: Arc::new(bind_paint(child, payloads)?),
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
