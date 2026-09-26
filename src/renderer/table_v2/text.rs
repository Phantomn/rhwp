//! Table-local text preview. Composition owns both occupied lines and paint payloads.
//! Qualified plain stored rows retain their partition and metrics. No Legacy table layout.
use std::{cell::RefCell, collections::HashMap, sync::Arc};

use crate::model::{paragraph::Paragraph, style::HeadType, table::Table};
use crate::renderer::{
    composer::{compose_paragraph, layout_paragraph_in_physical_frame},
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
    pub zones: Vec<super::zones::Zone>,
    pub diagonals: HashMap<(usize, usize), super::diagonal::Diagonal>,
    pub dpi: f64,
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
        super::text_ir::prepare(table, styles, dpi, &[], super::CellEndPolicy::default())
    }

    /// Snapshot embedded resources for qualified control-only picture rows.
    pub fn prepare_with_resources(
        table: &Table,
        styles: &ResolvedStyleSet,
        dpi: f64,
        resources: &[crate::model::bin_data::BinDataContent],
    ) -> Result<Self, GeometryError> {
        Self::prepare_with_end_policy(
            table,
            styles,
            dpi,
            resources,
            super::CellEndPolicy::default(),
        )
    }

    /// Explicit cell-end experiment shared by Native and WASM previews.
    pub fn prepare_with_end_policy(
        table: &Table,
        styles: &ResolvedStyleSet,
        dpi: f64,
        resources: &[crate::model::bin_data::BinDataContent],
        policy: super::CellEndPolicy,
    ) -> Result<Self, GeometryError> {
        super::text_ir::prepare(table, styles, dpi, resources, policy)
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
        for zone in &self.zones {
            zone.append_background(placement, &mut table)?;
        }
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
            if let Some(diagonal) = self.diagonals.get(&(cell.row, cell.column)) {
                diagonal.append(&mut node, cell.bounds, self.dpi);
            }
            table.children.push(node);
        }
        if let Some(borders) = &self.borders {
            borders.append(placement, &self.zones, &mut table)?;
        }
        for zone in &self.zones {
            zone.append_diagonal(placement, &mut table, self.dpi)?;
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

/// Logical caret advance can extend past the aligned line's right edge for
/// plain trailing spaces. Keep the payload intact; qualify only its painted
/// inline extent, using the same replay positions that backends consume.
/// Decorations, interior spaces and visible glyphs keep their full bounds.
fn painted_inline_ends(nodes: &[RenderNode], styles: &ResolvedStyleSet) -> Vec<Option<f64>> {
    let decoration_target = nodes.iter().enumerate().rev().find_map(|(i, node)| {
        if let RenderNodeType::TextRun(run) = &node.node_type {
            run.soft_wrap_decoration_trim().map(|trim| (i, trim))
        } else {
            None
        }
    });
    let mut suffix = true;
    let mut ends = Vec::with_capacity(nodes.len());
    for (index, node) in nodes.iter().enumerate().rev() {
        let full = Some(node.bbox.x + node.bbox.width);
        let RenderNodeType::TextRun(run) = &node.node_type else {
            suffix = false;
            ends.push(full);
            continue;
        };
        let s = &run.style;
        let plain = run.display_text.is_none()
            && run.char_overlap.is_none()
            && run.rotation == 0.0
            && !run.is_vertical
            && run.field_marker == crate::renderer::render_tree::FieldMarkerType::None
            && ((s.underline == crate::model::style::UnderlineType::None && !s.strikethrough)
                || decoration_target.is_some_and(|(i, trim)| i == index && trim > 0))
            && crate::model::color::char_shade(s.shade_color).is_none()
            && s.tab_leaders.is_empty()
            && s.outline_type == 0
            && s.shadow_type == 0
            && !s.emboss
            && !s.engrave
            && s.emphasis_dot == 0
            && super::decoration::paragraph_is_unpainted(run.border_fill_id, styles);
        let trimmed = run.text.trim_end_matches(' ');
        let end = if suffix && plain && trimmed.len() < run.text.len() {
            if trimmed.is_empty() {
                None
            } else {
                let positions = run.replay_positions_for(&run.text);
                positions
                    .get(trimmed.chars().count())
                    .filter(|v| v.is_finite() && **v >= 0.0)
                    .map(|v| node.bbox.x + v)
                    .or(full)
            }
        } else {
            full
        };
        suffix &= plain && trimmed.is_empty();
        ends.push(end);
    }
    ends.reverse();
    ends
}

impl CellParagraphComposer for TextComposer<'_> {
    fn compose(&self, para: &Paragraph, width: f64) -> Result<Vec<ParagraphItem>, GeometryError> {
        super::stored_text::validate_tabs(para, self.dpi)?;
        let stored_fields = super::fields::stored_formula_result(para)?;
        if para.column_type != crate::model::paragraph::ColumnBreakType::None
            || (!para.controls.is_empty() && !stored_fields)
            || para.source_line_seg_vertical_pos.is_some()
            || para.layout_only_fill_lines != 0
            || (!para.field_ranges.is_empty() && !stored_fields)
            || !para.orphan_field_ends.is_empty()
            || !para.range_tags.is_empty()
            || !para.title_marks.is_empty()
            || !para.markpen_marks.is_empty()
            || para
                .text
                .chars()
                .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
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
        if !super::decoration::paragraph_is_unpainted(style.border_fill_id, self.styles)
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
        let mut fresh = if stored {
            super::stored_text::localize(
                para,
                style.margin_left..width - style.margin_right,
                style.indent,
                self.dpi,
            )?
        } else {
            let mut fresh = para.clone();
            fresh.line_segs = layout_paragraph_in_physical_frame(
                para,
                &mut frame_box.frame(0),
                self.styles,
                self.dpi,
            )
            .ok_or(GeometryError::Unsupported("text preview frame composition"))?;
            fresh
        };
        super::stored_text::resolve_vertical_alignment(&mut fresh, self.styles, self.dpi, stored)?;
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
        // Final nodes and shared paragraph end own the actual origin sequence.
        // Physical fit uses the full line box even when source spacing makes
        // the following origin precede that box's bottom.
        let following: Vec<_> = column
            .children
            .iter()
            .skip(1)
            .map(|n| n.bbox.y)
            .chain(std::iter::once(end - style.spacing_after))
            .collect();
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
            if b.y < cursor - 1e-7 || b.x < 0.0 || b.x + b.width > width + 1e-7 {
                return Err(GeometryError::Unsupported(
                    "text preview overlapping or overflowing rows",
                ));
            }
            let painted_ends = painted_inline_ends(&node.children, self.styles);
            for (run, painted_end) in node.children.iter_mut().zip(painted_ends) {
                let r = &run.bbox;
                if [r.x, r.y, r.width, r.height].iter().any(|v| !v.is_finite())
                    || r.width < 0.0
                    || r.height < 0.0
                    || r.x < b.x - 1e-7
                    || r.y < b.y - 1e-7
                    || painted_end.is_some_and(|end| end > b.x + b.width + 1e-7)
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
            let advance = b.height.min(following[i] - b.y);
            super::contracts::nonnegative(advance, "composed line advance")?;
            // Zero pitch is an authored spacing choice, not an absent line.
            // Flow consumes the line owner once and fits its full physical box;
            // only the next origin stays here (e.g. a right-aligned units label).
            items.push(ParagraphItem::Lines {
                height: b.height,
                advance,
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
            cursor = b.y + advance;
        }
        if column.children.is_empty() || end < cursor {
            return Err(GeometryError::Unsupported(
                "text preview missing physical line extent",
            ));
        }
        let tail = if end > cursor {
            vec![end - cursor]
        } else {
            Vec::new()
        };
        let ending = super::ParagraphEnd::from_composed(&items, tail, style.spacing_after)?;
        items.push(ParagraphItem::End(ending));
        self.payloads.borrow_mut().push(column.children);
        Ok(items)
    }
}
