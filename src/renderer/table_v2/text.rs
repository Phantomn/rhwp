//! Table-local text preview. Composition owns both occupied lines and paint payloads.
//! Qualified plain stored rows retain their partition and metrics. No Legacy table layout.
use std::{cell::RefCell, collections::HashMap, sync::Arc};

use crate::model::{paragraph::Paragraph, style::HeadType, table::Table};
use crate::renderer::{
    composer::{compose_paragraph, layout_paragraph_in_physical_frame},
    hwpunit_to_px,
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

#[derive(Clone, Copy)]
enum InlineContent<'a> {
    Plain,
    Shapes(&'a [crate::model::bin_data::BinDataContent]),
    PageField(Option<u32>),
}

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
    pub page_fields: HashMap<PayloadKey, super::cell_page_field::PageField>,
    pub tables: HashMap<PayloadKey, OrderedPaint<Arc<TextPaint>>>,
    pub background: super::decoration::Background,
    pub cells: HashMap<(usize, usize), super::decoration::Background>,
    pub cell_indices: HashMap<(usize, usize), u32>,
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
    /// Unsupported control, border and paragraph-link constraints fail explicitly.
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
        self.fit_with_page_height(area, None)
    }

    pub(super) fn fit_with_page_height(
        &self,
        area: PageArea,
        page_height: Option<f64>,
    ) -> Result<TextFragmentFit, GeometryError> {
        Ok(match self.cursor.fit_with_page_height(area, page_height)? {
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
        self.append_to_with_page_number(page, None)
    }

    /// The host supplies its resolved printed number, not this preview's index
    /// or a parser-assigned AutoNumber value. Missing context is an error only
    /// for a fragment that actually contains a page field.
    pub fn append_to_with_page_number(
        &self,
        page: &mut PageRenderTree,
        number: Option<u32>,
    ) -> Result<(), GeometryError> {
        // Build completely before mutating the page, including ID allocation.
        let mut table = self.paint.build_node(self.geometry.placement(), number)?;
        assign_ids(&mut table, page.frame_mut());
        page.root.children.push(table);
        Ok(())
    }

    pub(super) fn build_node(&self, number: Option<u32>) -> Result<RenderNode, GeometryError> {
        self.paint.build_node(self.geometry.placement(), number)
    }
}

impl TextPaint {
    pub(super) fn build_node(
        &self,
        placement: &TablePlacement,
        number: Option<u32>,
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
            if cell.partial {
                if background.is_some_and(|b| !b.supports_fragment()) {
                    return Err(GeometryError::Unsupported("V2 split gradient background"));
                }
                if self.diagonals.contains_key(&(cell.row, cell.column)) {
                    return Err(GeometryError::Unsupported(
                        "V2 cell-internal diagonal split",
                    ));
                }
            }
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
                    model_cell_index: self.cell_indices.get(&(cell.row, cell.column)).copied(),
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
                let mut payload = match self.page_fields.get(&key) {
                    Some(field) => field.render(number, &entry.value)?,
                    None => entry.value.clone(),
                };
                let dx = line.bounds.x - payload.bbox.x;
                let dy = line.bounds.y - payload.bbox.y;
                translate(&mut payload, dx, dy);
                if let RenderNodeType::TextLine(value) = &mut payload.node_type {
                    value.para_index = Some(line.owner.paragraph);
                }
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
                let mut nested = entry.value.build_node(&child.placement, number)?;
                if let RenderNodeType::Table(value) = &mut nested.node_type {
                    value.para_index = Some(child.owner.paragraph);
                    value.control_index = Some(child.owner.control);
                }
                ordered.push((entry.order, nested));
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
    if let RenderNodeType::Line(line) = &mut node.node_type {
        // Line paint consumes absolute endpoints, unlike rectangle/image paint
        // which consumes bbox. Keep both on the same fragment/page origin.
        line.x1 += dx;
        line.x2 += dx;
        line.y1 += dy;
        line.y2 += dy;
    }
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
        let end = if suffix && plain && trimmed.is_empty() {
            None
        } else if suffix
            && plain
            && s.underline == crate::model::style::UnderlineType::None
            && !s.strikethrough
            && s.extra_char_spacing > 0.0
        {
            // Distribution stores a caret advance after every cluster, but
            // replay fits the glyph without the positive inter-cluster gap.
            // A plain suffix space does not turn the preceding glyph's gap
            // into ink: project visible clusters using the ORIGINAL run's
            // positions, even when trailing spaces have their own style runs.
            // Consume those same positions and glyph-fit projection; do not
            // resize the run, the saved lane, or the following paragraph.
            let positions = run.replay_positions_for(&run.text);
            crate::renderer::layout::split_into_clusters(trimmed)
                .iter()
                .try_fold(0.0_f64, |end, (start, cluster)| {
                    let x = *positions.get(*start)?;
                    let advance = positions.get(start + cluster.chars().count())? - x;
                    Some(end.max(x + s.glyph_fit_advance(advance)?))
                })
                .map(|end| node.bbox.x + end)
                .or(full)
        } else if suffix && plain && trimmed.len() < run.text.len() {
            let positions = run.replay_positions_for(&run.text);
            positions
                .get(trimmed.chars().count())
                .filter(|v| v.is_finite() && **v >= 0.0)
                .map(|v| node.bbox.x + v)
                .or(full)
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
    fn compose_with_cell_wrap(
        &self,
        para: &Paragraph,
        width: f64,
        single_column: bool,
        line_wrap: u8,
    ) -> Result<Vec<ParagraphItem>, GeometryError> {
        super::stored_text::validate_cell_wrap(para, line_wrap)?;
        if line_wrap == crate::model::table::CELL_LINE_WRAP_SQUEEZE {
            self.compose_text(para, width, true)
        } else {
            self.compose_in_cell(para, width, single_column)
        }
    }

    fn stored_child_frame_tails(
        &self,
        paragraphs: &[Paragraph],
    ) -> Result<Vec<(usize, f64)>, GeometryError> {
        super::stored_text::child_frame_tails(paragraphs, self.styles, self.dpi)
    }

    fn stored_frame_starts(
        &self,
        paragraphs: &[Paragraph],
    ) -> Result<Vec<(usize, usize)>, GeometryError> {
        super::stored_text::cell_frame_starts(paragraphs, self.styles, self.dpi)
    }

    fn compose(&self, para: &Paragraph, width: f64) -> Result<Vec<ParagraphItem>, GeometryError> {
        self.compose_text(para, width, false)
    }
}

impl TextComposer<'_> {
    fn compose_text(
        &self,
        para: &Paragraph,
        width: f64,
        squeeze: bool,
    ) -> Result<Vec<ParagraphItem>, GeometryError> {
        self.compose_text_with_body_end(para, width, squeeze, None)
    }

    pub(super) fn compose_text_with_body_end(
        &self,
        para: &Paragraph,
        width: f64,
        squeeze: bool,
        body_end: Option<&super::fields::BodyFieldEnd>,
    ) -> Result<Vec<ParagraphItem>, GeometryError> {
        self.compose_shared(para, width, squeeze, body_end, InlineContent::Plain, false)
    }

    pub(super) fn compose_host(
        &self,
        para: &Paragraph,
        width: f64,
    ) -> Result<Vec<ParagraphItem>, GeometryError> {
        self.compose_shared(para, width, false, None, InlineContent::Plain, true)
    }

    pub(super) fn compose_stored_inline_shapes(
        &self,
        para: &Paragraph,
        width: f64,
        resources: &[crate::model::bin_data::BinDataContent],
    ) -> Result<Vec<ParagraphItem>, GeometryError> {
        super::shapes::validate_inline(para)?;
        self.compose_shared(
            para,
            width,
            false,
            None,
            InlineContent::Shapes(resources),
            false,
        )
    }

    /// Host fragments own paragraph outlines. Cell-local mixed shapes still
    /// use the unpainted-border contract above; no source style is erased.
    pub(super) fn compose_host_inline_shapes(
        &self,
        para: &Paragraph,
        width: f64,
        resources: &[crate::model::bin_data::BinDataContent],
    ) -> Result<Vec<ParagraphItem>, GeometryError> {
        super::shapes::validate_inline(para)?;
        self.compose_shared(
            para,
            width,
            false,
            None,
            InlineContent::Shapes(resources),
            true,
        )
    }

    pub(super) fn compose_page_field(
        &self,
        para: &Paragraph,
        width: f64,
        number: Option<u32>,
    ) -> Result<Vec<ParagraphItem>, GeometryError> {
        if !super::cell_page_field::qualify_textbox(para)? {
            return Err(GeometryError::InconsistentAtomicPlan);
        }
        self.compose_shared(
            para,
            width,
            false,
            None,
            InlineContent::PageField(number),
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn compose_shared(
        &self,
        para: &Paragraph,
        width: f64,
        squeeze: bool,
        body_end: Option<&super::fields::BodyFieldEnd>,
        inline_content: InlineContent<'_>,
        host_border: bool,
    ) -> Result<Vec<ParagraphItem>, GeometryError> {
        super::stored_text::validate_tabs(para, self.dpi)?;
        super::char_border::qualify(para, self.styles)?;
        // HWP range kind 1 maps to HWPX t/@charStyleIDRef: an editor's
        // named character-style association. Effective paint/metrics already
        // come from the run's charPrIDRef (IR char_shapes). Preserve the ranges
        // on the paragraph; do not apply the named style a second time.
        // Other kinds can carry paint (e.g. kind 2 markpen) and remain outside
        // this plain-text contract. See issue_7353_character_style_ranges.
        if para.range_tags.iter().any(|range| {
            range.tag >> 24 != 1 || range.start > range.end || range.end > para.char_count
        }) {
            return Err(GeometryError::Unsupported(
                "text preview unsupported or invalid range annotation",
            ));
        }
        let stored_fields = super::fields::stored_result(para)?;
        if para.column_type != crate::model::paragraph::ColumnBreakType::None
            || (!para.controls.is_empty()
                && !stored_fields
                && matches!(inline_content, InlineContent::Plain))
            || para.source_line_seg_vertical_pos.is_some()
            || para.layout_only_fill_lines != 0
            || (!para.field_ranges.is_empty() && !stored_fields)
            || (!para.orphan_field_ends.is_empty() && body_end.is_none())
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
        let border_supported = if host_border {
            super::host_border::qualify(style, self.styles)?;
            true
        } else {
            super::decoration::paragraph_is_unpainted(style.border_fill_id, self.styles)
        };
        if !border_supported
            || style.head_type != HeadType::None
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
        let (mut fresh, physical_rows) = if stored {
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
            let boxes = fresh
                .line_segs
                .iter()
                .map(|row| {
                    let x = hwpunit_to_px(row.column_start, self.dpi);
                    x..x + hwpunit_to_px(row.segment_width, self.dpi)
                })
                .collect::<Vec<_>>();
            (fresh, boxes)
        };
        let centers = super::stored_text::resolve_vertical_alignment(
            &mut fresh,
            self.styles,
            self.dpi,
            stored,
        )?;
        let mut composed = compose_paragraph(&fresh);
        if let InlineContent::PageField(Some(number)) = inline_content {
            crate::renderer::paragraph_paint::fields::substitute_auto_numbers_in_composed(
                &fresh,
                &mut composed,
                number,
                0,
            );
        }
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
        let end = crate::renderer::paragraph_paint::paint_physical_frame(
            &mut frame,
            &mut column,
            &composed,
            self.styles,
            &area,
            &fresh,
            &physical_rows,
            squeeze,
            self.dpi,
        )
        .map_err(GeometryError::Unsupported)?;
        super::contracts::nonnegative(end, "composed paragraph end")?;
        for line in &mut column.children {
            super::char_border::connect_outlines(line, self.styles)?;
        }
        if let Some(centers) = centers {
            super::stored_text::align_center_runs(&centers, &mut column.children, style, self.dpi)?;
        }
        if stored {
            super::stored_text::validate_paint(
                &fresh,
                &physical_rows,
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
            if !matches!(node.node_type, RenderNodeType::TextLine(_)) {
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
            for child in &node.children {
                if !matches!(child.node_type, RenderNodeType::TextRun(_)) {
                    super::char_border::validate_child(child, node)?;
                }
            }
            for (run, painted_end) in node.children.iter_mut().zip(painted_ends) {
                if !matches!(run.node_type, RenderNodeType::TextRun(_)) {
                    continue;
                }
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
        if style.keep_lines {
            items = super::paragraph_keep::group(items)?;
        }
        let ending = super::ParagraphEnd::from_composed(&items, tail, style.spacing_after)?;
        items.push(ParagraphItem::End(ending));
        if let InlineContent::Shapes(resources) = inline_content {
            super::shapes::attach_inline(
                &fresh,
                &frame,
                &mut column.children,
                &mut items,
                self.styles,
                self.dpi,
                resources,
            )?;
        }
        self.payloads.borrow_mut().push(column.children);
        Ok(items)
    }
}
