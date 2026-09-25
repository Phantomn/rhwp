//! Table-local text preview. Composition owns both occupied lines and paint payloads.
//! Qualified plain stored rows retain their partition and metrics. No Legacy table layout.
use std::{cell::RefCell, collections::HashMap, sync::Arc};

use crate::model::{paragraph::Paragraph, style::HeadType, table::Table};
use crate::renderer::{
    composer::{compose_paragraph, layout_paragraph_in_frame},
    layout::LayoutEngine,
    layout_frame::ParagraphBox,
    page_layout::LayoutRect,
    render_tree::{
        BoundingBox, PageLayoutContext, PageRenderTree, RenderNode, RenderNodeType, TableCellNode,
        TableNode,
    },
    style_resolver::ResolvedStyleSet,
};

use super::{
    CellParagraphComposer, FragmentFit, GeometryError, PageArea, ParagraphItem, Rect,
    TableContentPlan, TableCursor, TableFragmentPlan, TablePlacement,
};

pub(super) type PayloadKey = (usize, usize, usize, usize); // row, column, paragraph, line/control

/// An immutable text/table-flow preview with optional solid backgrounds. This is not an editable
/// document session: source hit testing, anchors and backend sidecars are not bound.
/// Unsupported input is rejected, never sent to the Legacy table engine.
pub struct PreparedTextTable {
    pub(super) plan: Arc<TableContentPlan>,
    pub(super) paint: Arc<TextPaint>,
    pub(super) dpi: f64,
}

pub(super) struct OrderedPaint<T> {
    pub order: usize,
    pub value: T,
}

pub(super) struct TextPaint {
    pub rows: u16,
    pub columns: u16,
    pub lines: HashMap<PayloadKey, OrderedPaint<RenderNode>>,
    pub tables: HashMap<PayloadKey, OrderedPaint<Arc<TextPaint>>>,
    pub background: super::decoration::Background,
    pub cells: HashMap<(usize, usize), super::decoration::Background>,
    pub borders: Option<super::borders::CellBorders>,
}

/// The payload snapshot cannot be exchanged independently of its geometry cursor.
#[derive(Clone)]
pub struct TextTableCursor {
    cursor: TableCursor,
    paint: Arc<TextPaint>,
}

pub enum TextFragmentFit {
    Placed(TextFragment),
    DoesNotFit {
        required_width: f64,
        required_height: f64,
    },
    Complete,
}

pub struct TextFragment {
    geometry: TableFragmentPlan,
    paint: Arc<TextPaint>,
}

impl PreparedTextTable {
    /// Compose once at the declared cell width. Coordinates and style dimensions
    /// are pixels at `dpi`; IR cell sizes remain HWP units. Fresh plain text and
    /// one zero-offset paragraph-relative TopAndBottom child per host are admitted.
    /// Solid backgrounds use the supplied resolved styles. Source-only effects
    /// are qualified by TablePreviewSession::from_document before resolution.
    /// Solid cell edges follow accepted fragments for all three split policies.
    /// TAC, wrap, stored control hosts, complex borders and keep constraints remain unsupported.
    pub fn prepare(
        table: &Table,
        styles: &ResolvedStyleSet,
        dpi: f64,
    ) -> Result<Self, GeometryError> {
        super::text_ir::prepare(table, styles, dpi)
    }

    pub fn start(&self) -> TextTableCursor {
        TextTableCursor {
            cursor: TableCursor::new(self.plan.clone()),
            paint: self.paint.clone(),
        }
    }
}

impl TextTableCursor {
    pub(super) fn is_complete(&self) -> bool {
        self.cursor.is_complete()
    }

    pub fn fit(&self, area: PageArea) -> Result<TextFragmentFit, GeometryError> {
        Ok(match self.cursor.fit(area)? {
            FragmentFit::Placed(geometry) => TextFragmentFit::Placed(TextFragment {
                geometry,
                paint: self.paint.clone(),
            }),
            FragmentFit::DoesNotFit {
                required_width,
                required_height,
            } => TextFragmentFit::DoesNotFit {
                required_width,
                required_height,
            },
            FragmentFit::Complete => TextFragmentFit::Complete,
        })
    }
}

impl TextFragment {
    pub fn geometry(&self) -> &TableFragmentPlan {
        &self.geometry
    }

    pub fn continuation(&self) -> TextTableCursor {
        TextTableCursor {
            cursor: self.geometry.continuation(),
            paint: self.paint.clone(),
        }
    }

    /// Emit the exact reserved fragment. No text recomposition, row fitting,
    /// clipping, or post-layout height correction occurs here. Page coordinates
    /// come exclusively from fit; the caller owns page dimensions and flow advance.
    pub fn append_to(&self, page: &mut PageRenderTree) -> Result<(), GeometryError> {
        // Build completely before mutating the page, including ID allocation.
        let mut table = self.paint.build_node(self.geometry.placement())?;
        assign_ids(&mut table, page.frame_mut());
        page.root.children.push(table);
        Ok(())
    }
}

impl TextPaint {
    pub(super) fn build_node(
        &self,
        placement: &TablePlacement,
    ) -> Result<RenderNode, GeometryError> {
        let mut table = RenderNode::new(
            0,
            RenderNodeType::Table(TableNode {
                row_count: self.rows,
                col_count: self.columns,
                border_fill_id: self.background.id,
                section_index: None,
                para_index: None,
                control_index: None,
                cell_context: None,
            }),
            bbox(placement.bounds),
        );
        self.background.append(&mut table, placement.bounds);
        for cell in &placement.cells {
            let background = self.cells.get(&(cell.row, cell.column));
            let mut node = RenderNode::new(
                0,
                RenderNodeType::TableCell(TableCellNode {
                    row: cell.row as u16,
                    col: cell.column as u16,
                    row_span: cell.row_span as u16,
                    col_span: cell.column_span as u16,
                    border_fill_id: background.map_or(0, |b| b.id),
                    text_direction: 0,
                    clip: false,
                    page_fragment: true,
                    model_cell_index: None,
                }),
                bbox(cell.bounds),
            );
            if let Some(background) = background {
                background.append(&mut node, cell.bounds);
            }
            let mut ordered = Vec::new();
            for line in &cell.lines {
                let key = (cell.row, cell.column, line.owner.paragraph, line.owner.line);
                let entry = self
                    .lines
                    .get(&key)
                    .ok_or(GeometryError::InconsistentAtomicPlan)?;
                let mut payload = entry.value.clone();
                let dx = line.bounds.x - payload.bbox.x;
                let dy = line.bounds.y - payload.bbox.y;
                translate(&mut payload, dx, dy);
                ordered.push((entry.order, payload));
            }
            for child in &cell.tables {
                let key = (
                    cell.row,
                    cell.column,
                    child.owner.paragraph,
                    child.owner.control,
                );
                let entry = self
                    .tables
                    .get(&key)
                    .ok_or(GeometryError::InconsistentAtomicPlan)?;
                // Recursive fit already returned page coordinates. Do not add the
                // parent origin again or recompute the child's reserved height.
                ordered.push((entry.order, entry.value.build_node(&child.placement)?));
            }
            ordered.sort_by_key(|(order, _)| *order);
            node.children
                .extend(ordered.into_iter().map(|(_, child)| child));
            table.children.push(node);
        }
        if let Some(borders) = &self.borders {
            borders.append(placement, &mut table)?;
        }
        Ok(table)
    }
}

fn bbox(r: Rect) -> BoundingBox {
    BoundingBox::new(r.x, r.y, r.width, r.height)
}

pub(super) fn translate(node: &mut RenderNode, dx: f64, dy: f64) {
    node.bbox.x += dx;
    node.bbox.y += dy;
    for child in &mut node.children {
        translate(child, dx, dy);
    }
}

pub(super) fn assign_ids(node: &mut RenderNode, page: &mut PageLayoutContext) {
    node.id = page.next_id();
    for child in &mut node.children {
        assign_ids(child, page);
    }
}

pub(super) fn validate_text_context(
    styles: &ResolvedStyleSet,
    dpi: f64,
) -> Result<(), GeometryError> {
    if !dpi.is_finite() || dpi <= 0.0 {
        return Err(GeometryError::InvalidNumber("text DPI"));
    }
    if styles.hwp3_variant
        || styles.kerning_measurement_context.is_some()
        || styles.supplemental_metrics.is_some()
    {
        return Err(GeometryError::Unsupported(
            "text preview document metric context",
        ));
    }
    Ok(())
}

pub(super) struct TextComposer<'a> {
    pub styles: &'a ResolvedStyleSet,
    pub dpi: f64,
    pub payloads: RefCell<Vec<Vec<RenderNode>>>,
}

impl CellParagraphComposer for TextComposer<'_> {
    fn compose(&self, para: &Paragraph, width: f64) -> Result<Vec<ParagraphItem>, GeometryError> {
        if para.column_type != crate::model::paragraph::ColumnBreakType::None
            || !para.controls.is_empty()
            || para.source_line_seg_vertical_pos.is_some()
            || para.layout_only_fill_lines != 0
            || !para.field_ranges.is_empty()
            || !para.range_tags.is_empty()
            || !para.title_marks.is_empty()
            || !para.markpen_marks.is_empty()
            || para
                .text
                .chars()
                .any(|c| c.is_control() && c != '\n' && c != '\r')
        {
            return Err(GeometryError::Unsupported(
                "text preview stored rows or controls",
            ));
        }
        let style = self
            .styles
            .para_styles
            .get(para.para_shape_id as usize)
            .ok_or(GeometryError::Unsupported("missing paragraph style"))?;
        if style.border_fill_id != 0
            || style.head_type != HeadType::None
            || style.keep_lines
            || style.keep_with_next
            || style.widow_orphan
            || style.page_break_before
        {
            return Err(GeometryError::Unsupported(
                "text preview paragraph decoration or keep",
            ));
        }
        for value in [
            style.margin_left,
            style.margin_right,
            style.spacing_before,
            style.spacing_after,
        ] {
            super::contracts::nonnegative(value, "paragraph inset")?;
        }
        if para.char_shapes.is_empty()
            || para.char_shapes.iter().any(|r| {
                self.styles
                    .char_styles
                    .get(r.char_shape_id as usize)
                    .is_none()
            })
        {
            return Err(GeometryError::Unsupported("missing character style"));
        }
        for reference in &para.char_shapes {
            let font = &self.styles.char_styles[reference.char_shape_id as usize];
            if !font.font_size.is_finite() || font.font_size <= 0.0 {
                return Err(GeometryError::InvalidNumber("text font size"));
            }
        }
        let left = crate::renderer::px_to_hwpunit(style.margin_left, self.dpi);
        let right = crate::renderer::px_to_hwpunit(width - style.margin_right, self.dpi);
        let frame_box = ParagraphBox::content(left..right);
        if !frame_box.is_usable() {
            return Err(GeometryError::Unsupported(
                "text preview unusable paragraph width",
            ));
        }
        let stored = !para.line_segs.is_empty();
        let fresh = if stored {
            if style.margin_left != 0.0 || style.margin_right != 0.0 || style.indent != 0.0 {
                return Err(GeometryError::Unsupported("stored text paragraph insets"));
            }
            super::stored_text::localize(para, width, self.dpi)?
        } else {
            let mut fresh = para.clone();
            fresh.line_segs =
                layout_paragraph_in_frame(para, &mut frame_box.frame(0), self.styles, self.dpi)
                    .ok_or(GeometryError::Unsupported("text preview frame composition"))?;
            fresh
        };
        let composed = compose_paragraph(&fresh);
        let mut frame = PageLayoutContext::new(0, width, 0.0);
        let mut column = RenderNode::new(
            0,
            RenderNodeType::Column(0),
            BoundingBox::new(0.0, 0.0, width, 0.0),
        );
        let area = LayoutRect {
            x: 0.0,
            y: 0.0,
            width,
            height: 0.0,
        };
        let end = LayoutEngine::new(self.dpi).layout_composed_paragraph_in_frame(
            &mut frame,
            &mut column,
            &composed,
            self.styles,
            &area,
            0.0,
            0,
            composed.lines.len(),
            0,
            0,
            None,
            true,
            false,
            0.0,
            None,
            Some(&fresh),
            None,
            None,
            true,
        );
        super::contracts::nonnegative(end, "composed paragraph end")?;
        if stored {
            super::stored_text::validate_paint(
                &fresh,
                &column.children,
                end,
                style.spacing_before,
                style.spacing_after,
                self.dpi,
            )?;
        }
        let mut items = Vec::new();
        let mut cursor = 0.0;
        for (i, node) in column.children.iter_mut().enumerate() {
            if !matches!(node.node_type, RenderNodeType::TextLine(_))
                || node
                    .children
                    .iter()
                    .any(|n| !matches!(n.node_type, RenderNodeType::TextRun(_)))
            {
                return Err(GeometryError::Unsupported(
                    "text preview non-text paint payload",
                ));
            }
            let b = &node.bbox;
            if b.y < cursor || b.x < 0.0 || b.x + b.width > width + 1e-7 {
                return Err(GeometryError::Unsupported(
                    "text preview overlapping or overflowing rows",
                ));
            }
            for run in &mut node.children {
                let r = &run.bbox;
                if [r.x, r.y, r.width, r.height].iter().any(|v| !v.is_finite())
                    || r.width < 0.0
                    || r.height < 0.0
                    || r.x < b.x - 1e-7
                    || r.y < b.y - 1e-7
                    || r.x + r.width > b.x + b.width + 1e-7
                    || r.y + r.height > b.y + b.height + 1e-7
                {
                    return Err(GeometryError::Unsupported(
                        "text preview run outside occupied line",
                    ));
                }
                if let RenderNodeType::TextRun(text) = &mut run.node_type {
                    text.section_index = None;
                    text.para_index = None;
                }
            }
            if let RenderNodeType::TextLine(line) = &mut node.node_type {
                line.section_index = None;
                line.para_index = None;
            }
            if b.y > cursor {
                items.push(ParagraphItem::Space(b.y - cursor));
            }
            items.push(ParagraphItem::Lines {
                height: b.height,
                lines: vec![(
                    i,
                    Rect {
                        x: b.x,
                        y: 0.0,
                        width: b.width,
                        height: b.height,
                    },
                )],
            });
            cursor = b.y + b.height;
        }
        if column.children.is_empty() || end < cursor {
            return Err(GeometryError::Unsupported(
                "text preview missing physical line extent",
            ));
        }
        if end > cursor {
            items.push(ParagraphItem::Space(end - cursor));
        }
        self.payloads.borrow_mut().push(column.children);
        Ok(items)
    }
}
