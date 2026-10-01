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
    width: f64,
}

impl CellParagraphComposer for IrTextComposer<'_> {
    fn compose_with_cell_wrap(
        &self,
        para: &Paragraph,
        width: f64,
        single_column: bool,
        line_wrap: u8,
    ) -> Result<Vec<ParagraphItem>, GeometryError> {
        if super::cell_page_field::qualify(para)? {
            if line_wrap != 0 {
                return Err(GeometryError::Unsupported("page field cell wrap policy"));
            }
            let items = self.text.compose_page_field(para, width, None)?;
            return self.record_items(items, width);
        }
        // bind_table qualified the initial one-column declaration and its
        // source slot. It establishes the cell lane, not an inline occupant.
        // SQUEEZE must use the same stored-text projection as BREAK without
        // dropping its no-wrap policy or shifting the saved UTF-16 positions.
        let text_only;
        let para = if super::page_number::cell_declaration(para)?
            || (line_wrap == crate::model::table::CELL_LINE_WRAP_SQUEEZE
                && single_column
                && matches!(para.controls.as_slice(), [Control::ColumnDef(_)]))
        {
            text_only = {
                let mut p = para.clone();
                p.controls.clear();
                p.ctrl_data_records.clear();
                p
            };
            &text_only
        } else {
            para
        };
        super::stored_text::validate_cell_wrap(para, line_wrap)?;
        if line_wrap == crate::model::table::CELL_LINE_WRAP_SQUEEZE {
            let items = self
                .text
                .compose_with_cell_wrap(para, width, single_column, line_wrap)?;
            self.record_items(items, width)
        } else {
            self.compose_in_cell(para, width, single_column)
        }
    }

    fn stored_child_frame_tails(
        &self,
        paragraphs: &[Paragraph],
    ) -> Result<Vec<(usize, f64)>, GeometryError> {
        self.text.stored_child_frame_tails(paragraphs)
    }

    fn stored_frame_starts(
        &self,
        paragraphs: &[Paragraph],
    ) -> Result<Vec<(usize, usize)>, GeometryError> {
        self.text.stored_frame_starts(paragraphs)
    }

    fn compose(&self, para: &Paragraph, width: f64) -> Result<Vec<ParagraphItem>, GeometryError> {
        self.compose_in_cell(para, width, false)
    }

    fn compose_in_cell(
        &self,
        para: &Paragraph,
        width: f64,
        single_column: bool,
    ) -> Result<Vec<ParagraphItem>, GeometryError> {
        let items = self.compose_items(para, width, single_column)?;
        self.record_items(items, width)
    }
}

impl IrTextComposer<'_> {
    fn record_items(
        &self,
        items: Vec<ParagraphItem>,
        width: f64,
    ) -> Result<Vec<ParagraphItem>, GeometryError> {
        // Store the very same ordered ownership recipe that geometry consumes.
        // Paint must not reconstruct slot order from the IR control array later.
        let mut slots = Vec::new();
        for item in &items {
            match item {
                ParagraphItem::PositionedTable { control, .. } => {
                    slots.push(PaintSlot::Table(*control))
                }
                ParagraphItem::ExcludedTable { control, line, .. } => {
                    slots.push(PaintSlot::Line(*line));
                    slots.push(PaintSlot::Table(*control));
                }
                ParagraphItem::ObjectRow { line, .. } => slots.push(PaintSlot::Line(*line)),
                ParagraphItem::Lines { lines, .. } => {
                    slots.extend(lines.iter().map(|(line, _)| PaintSlot::Line(*line)))
                }
                ParagraphItem::TableControl(ci) => slots.push(PaintSlot::Table(*ci)),
                ParagraphItem::InlineTables { tables, lines, .. } => {
                    slots.extend(lines.iter().map(|(line, _)| PaintSlot::Line(*line)));
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
        self.paragraphs.borrow_mut().push(ParagraphPaint {
            slots,
            lines,
            width,
        });
        Ok(items)
    }
}

impl IrTextComposer<'_> {
    fn compose_items(
        &self,
        para: &Paragraph,
        width: f64,
        single_column: bool,
    ) -> Result<Vec<ParagraphItem>, GeometryError> {
        if super::shapes::floating_candidate(para) {
            let (items, nodes) = super::shapes::compose_floating(
                para,
                width,
                self.text.styles,
                self.text.dpi,
                self.resources,
            )?;
            self.text.payloads.borrow_mut().push(nodes);
            return Ok(items);
        }
        if super::shapes::mixed_inline_candidate(para) {
            return self
                .text
                .compose_stored_inline_shapes(para, width, self.resources);
        }
        if super::pictures::excluded_cell_candidate(para) {
            let (items, nodes) = super::pictures::compose_excluded_cell(
                para,
                width,
                self.text.styles,
                self.text.dpi,
                self.resources,
            )?;
            self.text.payloads.borrow_mut().push(nodes);
            return Ok(items);
        }
        if super::cell_anchor::candidate(para) {
            let (item, node) = super::cell_anchor::compose(
                para,
                width,
                self.text.styles,
                self.text.dpi,
                single_column,
            )?;
            self.text.payloads.borrow_mut().push(vec![node]);
            return Ok(vec![item]);
        }
        if super::cell_anchor::following_candidate(para) {
            let item = super::cell_anchor::compose_following(
                para,
                width,
                self.text.styles,
                self.text.dpi,
            )?;
            let mut text_only = para.clone();
            text_only.controls.clear();
            text_only.ctrl_data_records.clear();
            // Keep stored line partition, offsets and signed spacing. The child
            // excludes the lane; visible and whitespace-only hosts are equal.
            let mut items = vec![item];
            items.extend(self.text.compose(&text_only, width)?);
            return Ok(items);
        }
        // The IR adapter has qualified the initial cell lane. A structural
        // slot has no paint/advance, but its source text offsets must survive.
        if !para.controls.is_empty()
            && para
                .controls
                .iter()
                .all(|c| matches!(c, Control::ColumnDef(_)))
        {
            let mut text_only = para.clone();
            text_only.controls.clear();
            text_only.ctrl_data_records.clear();
            return self.text.compose(&text_only, width);
        }
        if para.controls.is_empty() || super::fields::stored_result(para)? {
            return self.text.compose(para, width);
        }
        if para.controls.iter().all(|c| {
            matches!(
                c,
                Control::Picture(_) | Control::Shape(_) | Control::ColumnDef(_)
            )
        }) {
            // bind_table already qualified an initial one-column cell story.
            // Keep the structural slot: stored_object_rows maps the remaining
            // pictures to their original UTF-16 positions and saved lines.
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
        if super::fields::stored_tac_prefix(para)
            || para.controls.iter().all(|c| {
                matches!(c, Control::ColumnDef(_))
                    || matches!(c, Control::Table(t) if t.common.treat_as_char)
            })
        {
            let (items, nodes) = super::tac::compose(para, width, self.text.styles, self.text.dpi)?;
            self.text.payloads.borrow_mut().push(nodes);
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
    let zones = super::zones::Zone::prepare(table, styles)?;
    if super::diagonal::Diagonal::resolve(table.border_fill_id, styles)?.is_some() {
        return Err(GeometryError::Unsupported("V2 whole-table diagonal"));
    }
    let mut paint = TextPaint {
        rows: table.row_count,
        columns: table.col_count,
        lines: HashMap::new(),
        page_fields: HashMap::new(),
        tables: HashMap::new(),
        background: super::decoration::Background::resolve(
            table.border_fill_id,
            styles,
            table.page_break == crate::model::table::TablePageBreak::None,
        )?,
        cells: HashMap::new(),
        cell_indices: table
            .cells
            .iter()
            .enumerate()
            .map(|(i, c)| ((usize::from(c.row), usize::from(c.col)), i as u32))
            .collect(),
        borders: super::borders::CellBorders::prepare(table, styles, dpi)?,
        zones,
        diagonals: HashMap::new(),
        dpi,
    };
    let mut cells: Vec<_> = table.cells.iter().collect();
    cells.sort_by_key(|c| (c.row, c.col));
    for cell in cells {
        if let Some(diagonal) = super::diagonal::Diagonal::resolve(cell.border_fill_id, styles)? {
            // The final placement, not split permission, decides whether this
            // cell is cut. TextPaint rejects partial diagonal cells below.
            paint
                .diagonals
                .insert((usize::from(cell.row), usize::from(cell.col)), diagonal);
        }
        paint.cells.insert(
            (usize::from(cell.row), usize::from(cell.col)),
            super::decoration::Background::resolve_cell(
                cell.border_fill_id,
                styles,
                true, // Actual partial gradient cells are rejected by TextPaint.
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
                        if super::cell_page_field::qualify(para)? {
                            paint.page_fields.insert(
                                (cell.row as usize, cell.col as usize, pi, li),
                                super::cell_page_field::PageField::new(
                                    para, styles, host.width, dpi,
                                ),
                            );
                        }
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
