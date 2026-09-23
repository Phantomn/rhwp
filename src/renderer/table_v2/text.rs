//! Table-local text preview. Composition owns both occupied lines and paint payloads.
//! No DocumentCore engine switch, stored-LineSeg admission, or Legacy table layout.
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
    TableContentPlan, TableCursor, TableFragmentPlan,
};

type PayloadKey = (usize, usize, usize, usize); // row, column, paragraph, line

/// An immutable text-only, borderless table preview. This is not an editable
/// document session: source hit testing, anchors and backend sidecars are not bound.
/// Unsupported input is rejected, never sent to the Legacy table engine.
pub struct PreparedTextTable {
    cursor: TableCursor,
    paint: Arc<TextPaint>,
}

struct TextPaint {
    rows: u16,
    columns: u16,
    lines: HashMap<PayloadKey, RenderNode>,
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
    /// are pixels at `dpi`; IR cell sizes remain HWP units. Only fresh plain text
    /// is currently admitted. Saved rows, controls, borders and keep constraints
    /// need their own qualified contracts before this preview can accept them.
    pub fn prepare(
        table: &Table,
        styles: &ResolvedStyleSet,
        dpi: f64,
    ) -> Result<Self, GeometryError> {
        if !dpi.is_finite() || dpi <= 0.0 {
            return Err(GeometryError::InvalidNumber("text DPI"));
        }
        if table.border_fill_id != 0
            || table.cells.iter().any(|c| c.border_fill_id != 0)
            || !table.zones.is_empty()
        {
            return Err(GeometryError::Unsupported("text preview table borders"));
        }
        if styles.hwp3_variant
            || styles.kerning_measurement_context.is_some()
            || styles.supplemental_metrics.is_some()
        {
            return Err(GeometryError::Unsupported(
                "text preview document metric context",
            ));
        }
        let composer = TextComposer {
            styles,
            dpi,
            payloads: RefCell::new(Vec::new()),
        };
        let plan = TableContentPlan::from_ir_contents(table, dpi / 7200.0, &composer)?;
        let mut paragraphs = composer.payloads.into_inner().into_iter();
        let mut cells: Vec<_> = table.cells.iter().collect();
        cells.sort_by_key(|c| (c.row, c.col));
        let mut lines = HashMap::new();
        for cell in cells {
            for pi in 0..cell.paragraphs.len() {
                for (li, node) in paragraphs
                    .next()
                    .ok_or(GeometryError::InconsistentAtomicPlan)?
                    .into_iter()
                    .enumerate()
                {
                    lines.insert((cell.row as usize, cell.col as usize, pi, li), node);
                }
            }
        }
        if paragraphs.next().is_some() {
            return Err(GeometryError::InconsistentAtomicPlan);
        }
        Ok(Self {
            cursor: plan.start(),
            paint: Arc::new(TextPaint {
                rows: table.row_count,
                columns: table.col_count,
                lines,
            }),
        })
    }

    pub fn start(&self) -> TextTableCursor {
        TextTableCursor {
            cursor: self.cursor.clone(),
            paint: self.paint.clone(),
        }
    }
}

impl TextTableCursor {
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
        let placement = self.geometry.placement();
        let mut table = RenderNode::new(
            0,
            RenderNodeType::Table(TableNode {
                row_count: self.paint.rows,
                col_count: self.paint.columns,
                border_fill_id: 0,
                section_index: None,
                para_index: None,
                control_index: None,
                cell_context: None,
            }),
            bbox(placement.bounds),
        );
        for cell in &placement.cells {
            let mut node = RenderNode::new(
                0,
                RenderNodeType::TableCell(TableCellNode {
                    row: cell.row as u16,
                    col: cell.column as u16,
                    row_span: 1,
                    col_span: 1,
                    border_fill_id: 0,
                    text_direction: 0,
                    clip: false,
                    page_fragment: true,
                    model_cell_index: None,
                }),
                bbox(cell.bounds),
            );
            for line in &cell.lines {
                let key = (cell.row, cell.column, line.owner.paragraph, line.owner.line);
                let mut payload = self
                    .paint
                    .lines
                    .get(&key)
                    .ok_or(GeometryError::InconsistentAtomicPlan)?
                    .clone();
                let dx = line.bounds.x - payload.bbox.x;
                let dy = line.bounds.y - payload.bbox.y;
                translate(&mut payload, dx, dy);
                node.children.push(payload);
            }
            table.children.push(node);
        }
        assign_ids(&mut table, page.frame_mut());
        page.root.children.push(table);
        Ok(())
    }
}

fn bbox(r: Rect) -> BoundingBox {
    BoundingBox::new(r.x, r.y, r.width, r.height)
}

fn translate(node: &mut RenderNode, dx: f64, dy: f64) {
    node.bbox.x += dx;
    node.bbox.y += dy;
    for child in &mut node.children {
        translate(child, dx, dy);
    }
}

fn assign_ids(node: &mut RenderNode, page: &mut PageLayoutContext) {
    node.id = page.next_id();
    for child in &mut node.children {
        assign_ids(child, page);
    }
}

struct TextComposer<'a> {
    styles: &'a ResolvedStyleSet,
    dpi: f64,
    payloads: RefCell<Vec<Vec<RenderNode>>>,
}

impl CellParagraphComposer for TextComposer<'_> {
    fn compose(&self, para: &Paragraph, width: f64) -> Result<Vec<ParagraphItem>, GeometryError> {
        if !para.controls.is_empty()
            || !para.line_segs.is_empty()
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
        let mut fresh = para.clone();
        fresh.line_segs =
            layout_paragraph_in_frame(para, &mut frame_box.frame(0), self.styles, self.dpi)
                .ok_or(GeometryError::Unsupported("text preview frame composition"))?;
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
