use std::collections::HashSet;
use std::sync::Arc;

use super::contracts::nonnegative;
use super::{
    FlowBlock, FlowCellInput, FlowRowInput, GeometryError, RowInput, SplitPolicy, TableCursor,
};

/// Immutable width-bound cell content. Child tables retain their own plans.
#[derive(Debug)]
pub struct TableContentPlan {
    pub(super) grid: Vec<Vec<CellTrack>>,
    pub(super) rows: Vec<FlowRowInput>,
    pub(super) row_heights: Vec<f64>,
    pub(super) row_spacing: f64,
    pub(super) width: f64,
    pub(super) policy: SplitPolicy,
    pub(super) height: f64,
    /// Atomic leading prefix, replayed in physical space without advancing the
    /// body cursor. Qualified by the IR adapter; zero for caller-composed flows.
    pub(super) header_rows: usize,
    depth: usize,
}

/// Alignment of complete cell content; internal cell cuts are not qualified.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

/// One physical cell, even when it covers several logical columns. Produced at
/// grid binding and consumed unchanged for composition width and final placement.
#[derive(Debug, Clone)]
pub(super) struct CellTrack {
    pub column: usize,
    pub span: usize,
    pub row_span: usize,
    pub left: f64,
    pub width: f64,
    pub alignment: VerticalAlignment,
    /// Qualified saved text lane, independent of the physical cell/child-table
    /// frame. Only the IR adapter can establish a minimum-width text lane.
    pub text_width: Option<f64>,
    /// Leading space resolved from the intact row's final physical height.
    pub content_offset_y: f64,
}

impl FlowBlock {
    pub(super) fn advance(&self) -> f64 {
        match self {
            Self::AnchoredTable { host_advance, .. } => self.height().max(*host_advance),
            Self::InlineTables { advance, .. } | Self::Lines { advance, .. } => *advance,
            _ => self.height(),
        }
    }
    pub(super) fn height(&self) -> f64 {
        match self {
            Self::AnchoredTable {
                host,
                top,
                bottom,
                plan,
                ..
            } => host.bounds.height.max(top + plan.height + bottom),
            Self::Space(height)
            | Self::Lines { height, .. }
            | Self::InlineTables { height, .. } => *height,
            Self::Table { plan, .. } => plan.height,
        }
    }
}

fn physical_extent(blocks: &[FlowBlock]) -> f64 {
    let mut pen: f64 = 0.0;
    let mut end: f64 = 0.0;
    for block in blocks {
        end = end.max(pen + block.height());
        pen += block.advance();
        end = end.max(pen);
    }
    end
}

impl TableContentPlan {
    /// Compatibility entry for already composed cells. A whole composed cell is
    /// atomic: this entry does not invent line-break units from overlapping boxes.
    pub fn new(
        column_widths: Vec<f64>,
        rows: Vec<RowInput>,
        row_spacing: f64,
        policy: SplitPolicy,
    ) -> Result<Self, GeometryError> {
        if policy == SplitPolicy::WithinCells {
            return Err(GeometryError::UnsupportedCellSplit);
        }
        Self::from_flow_rows(
            column_widths,
            rows.into_iter()
                .map(|row| FlowRowInput {
                    cells: row
                        .cells
                        .into_iter()
                        .map(|cell| FlowCellInput {
                            padding: cell.padding,
                            minimum_height: cell.minimum_height,
                            width: cell.content.width,
                            blocks: vec![FlowBlock::Lines {
                                height: cell.content.height,
                                advance: cell.content.height,
                                lines: cell.content.lines,
                            }],
                        })
                        .collect(),
                })
                .collect(),
            row_spacing,
            policy,
        )
    }

    /// Composition explicitly supplies atomic line groups and nested tables.
    /// Padding is physical space and is inserted once, not once per page.
    pub fn from_flow_rows(
        column_widths: Vec<f64>,
        rows: Vec<FlowRowInput>,
        row_spacing: f64,
        policy: SplitPolicy,
    ) -> Result<Self, GeometryError> {
        if column_widths.is_empty() || rows.is_empty() {
            return Err(GeometryError::EmptyTable);
        }
        for &width in &column_widths {
            nonnegative(width, "column width")?;
            if width == 0.0 {
                return Err(GeometryError::InvalidNumber("zero column width"));
            }
        }
        let width = column_widths.iter().sum();
        let mut left = 0.0;
        let tracks: Vec<_> = column_widths
            .into_iter()
            .enumerate()
            .map(|(column, width)| {
                let cell = CellTrack {
                    column,
                    span: 1,
                    row_span: 1,
                    left,
                    width,
                    alignment: VerticalAlignment::Top,
                    text_width: None,
                    content_offset_y: 0.0,
                };
                left += width;
                cell
            })
            .collect();
        Self::from_grid_rows(vec![tracks; rows.len()], width, rows, row_spacing, policy)
    }

    pub(super) fn from_grid_rows(
        mut grid: Vec<Vec<CellTrack>>,
        width: f64,
        mut rows: Vec<FlowRowInput>,
        row_spacing: f64,
        policy: SplitPolicy,
    ) -> Result<Self, GeometryError> {
        nonnegative(row_spacing, "row spacing")?;
        nonnegative(width, "table width")?;
        let mut row_heights = Vec::with_capacity(rows.len());
        let mut depth: usize = 1;
        for (row, input) in rows.iter_mut().enumerate() {
            if input.cells.len() != grid[row].len() {
                return Err(GeometryError::CellCount { row });
            }
            let mut height: f64 = 0.0;
            for (slot, cell) in input.cells.iter_mut().enumerate() {
                let track = &grid[row][slot];
                if track.row_span > 1 && policy == SplitPolicy::WithinCells {
                    return Err(GeometryError::Unsupported("rowspan cell-internal cuts"));
                }
                if policy == SplitPolicy::WithinCells && track.alignment != VerticalAlignment::Top {
                    return Err(GeometryError::Unsupported("split-cell vertical alignment"));
                }
                let column = track.column;
                let p = cell.padding;
                for v in [p.left, p.right, p.top, p.bottom] {
                    nonnegative(v, "padding")?;
                }
                nonnegative(cell.minimum_height, "minimum cell height")?;
                nonnegative(cell.width, "content width")?;
                let inner_width = track.width - p.left - p.right;
                if inner_width < 0.0 || inner_width != cell.width {
                    return Err(GeometryError::ContentWidth { row, column });
                }
                let text_width = track.text_width.unwrap_or(inner_width);
                nonnegative(text_width, "text lane width")?;
                let mut owners = HashSet::new();
                let mut controls = HashSet::new();
                for block in &cell.blocks {
                    nonnegative(block.height(), "block height")?;
                    match block {
                        FlowBlock::AnchoredTable {
                            owner,
                            host,
                            host_advance,
                            offset_x,
                            top,
                            bottom,
                            plan,
                        } => {
                            depth = depth.max(plan.depth + 1);
                            for v in [*host_advance, *offset_x, *top, *bottom, host.bounds.height] {
                                nonnegative(v, "anchored cell geometry")?;
                            }
                            if depth > 64
                                || host.bounds.x != 0.0
                                || host.bounds.y != 0.0
                                || host.bounds.width != 0.0
                                || host.bounds.height <= 0.0
                                || !(*offset_x + plan.width).is_finite()
                                || *offset_x + plan.width > inner_width
                            {
                                return Err(GeometryError::ContentBounds { row, column });
                            }
                            if !owners.insert(host.owner) || !controls.insert(*owner) {
                                return Err(GeometryError::DuplicateLineOwner { row, column });
                            }
                        }
                        FlowBlock::Lines {
                            height,
                            advance,
                            lines,
                        } => {
                            nonnegative(*advance, "line advance")?;
                            if *advance > *height {
                                return Err(GeometryError::Unsupported(
                                    "line advance outside envelope",
                                ));
                            }
                            for line in lines {
                                let b = line.bounds;
                                for v in [b.x, b.y, b.width, b.height] {
                                    nonnegative(v, "line bounds")?;
                                }
                                if !((b.x + b.width).is_finite() && (b.y + b.height).is_finite())
                                    || b.x + b.width > text_width
                                    || b.y + b.height > *height
                                {
                                    return Err(GeometryError::ContentBounds { row, column });
                                }
                                if !owners.insert(line.owner) {
                                    return Err(GeometryError::DuplicateLineOwner { row, column });
                                }
                            }
                        }
                        FlowBlock::Table {
                            owner,
                            offset_x,
                            restart_top,
                            plan,
                        } => {
                            depth = depth.max(plan.depth + 1);
                            if depth > 64 {
                                return Err(GeometryError::Unsupported(
                                    "table nesting resource limit",
                                ));
                            }
                            nonnegative(*offset_x, "nested horizontal offset")?;
                            nonnegative(*restart_top, "table restart margin")?;
                            if !(offset_x + plan.width).is_finite()
                                || offset_x + plan.width > inner_width
                            {
                                return Err(GeometryError::ContentWidth { row, column });
                            }
                            if !controls.insert(*owner) {
                                return Err(GeometryError::Unsupported("duplicate child owner"));
                            }
                        }
                        FlowBlock::InlineTables {
                            height,
                            advance,
                            tables,
                            lines,
                        } => {
                            nonnegative(*advance, "inline advance")?;
                            if *advance == 0.0 || *advance > *height {
                                return Err(GeometryError::Unsupported(
                                    "inline advance outside envelope",
                                ));
                            }
                            if tables.is_empty() {
                                return Err(GeometryError::Unsupported("empty inline group"));
                            }
                            for line in lines {
                                let b = line.bounds;
                                for v in [b.x, b.y, b.width, b.height] {
                                    nonnegative(v, "inline text bounds")?;
                                }
                                if b.x + b.width > inner_width || b.y + b.height > *height {
                                    return Err(GeometryError::ContentBounds { row, column });
                                }
                                if !owners.insert(line.owner) {
                                    return Err(GeometryError::DuplicateLineOwner { row, column });
                                }
                            }
                            for child in tables {
                                nonnegative(child.x, "inline table x")?;
                                nonnegative(child.y, "inline table y")?;
                                if child.x + child.plan.width > inner_width
                                    || child.y + child.plan.height > *height
                                {
                                    return Err(GeometryError::ContentBounds { row, column });
                                }
                                depth = depth.max(child.plan.depth + 1);
                                if depth > 64 || !controls.insert(child.owner) {
                                    return Err(GeometryError::Unsupported(
                                        "inline depth or duplicate owner",
                                    ));
                                }
                            }
                        }
                        FlowBlock::Space(_) => {}
                    }
                }
                cell.blocks.insert(0, FlowBlock::Space(p.top));
                cell.blocks.push(FlowBlock::Space(p.bottom));
                let physical = physical_extent(&cell.blocks);
                nonnegative(physical, "physical cell height")?;
                if track.row_span == 1 {
                    height = height.max(physical).max(cell.minimum_height);
                }
            }
            row_heights.push(height);
        }
        let has_spans = grid.iter().flatten().any(|track| track.row_span > 1);
        if has_spans && (row_spacing != 0.0 || row_heights.contains(&0.0)) {
            return Err(GeometryError::Unsupported(
                "unresolved rowspan row boundaries",
            ));
        }
        for (row, input) in rows.iter().enumerate() {
            // The whole row height is known only after EVERY cell was measured.
            // Empty line boxes and nested tables are physical content, not ink.
            // Do not center each page fragment or change declared minimum height.
            for (slot, cell) in input.cells.iter().enumerate() {
                let physical = physical_extent(&cell.blocks);
                let track = &mut grid[row][slot];
                let height = row_heights[row..row + track.row_span].iter().sum::<f64>();
                // Intact non-spanning cells establish the row boundaries. A
                // spanning cell cannot silently resize an arbitrary covered row.
                if physical > height || cell.minimum_height > height {
                    return Err(GeometryError::Unsupported(
                        "rowspan height needs redistribution",
                    ));
                }
                let slack = height - physical;
                track.content_offset_y = match track.alignment {
                    VerticalAlignment::Top => 0.0,
                    VerticalAlignment::Center => slack / 2.0,
                    VerticalAlignment::Bottom => slack,
                };
            }
        }
        let height = row_heights.iter().enumerate().fold(0.0, |h, (row, v)| {
            h + if row == 0 { 0.0 } else { row_spacing } + v
        });
        nonnegative(height, "physical table height")?;
        Ok(Self {
            grid,
            rows,
            row_heights,
            row_spacing,
            width,
            policy,
            height,
            header_rows: 0,
            depth,
        })
    }

    pub fn start(self) -> TableCursor {
        TableCursor::new(Arc::new(self))
    }
}
