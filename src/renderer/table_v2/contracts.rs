//! Input at the composition boundary and output at the reservation/placement boundary.

/// A rectangle in the same coordinate system as its enclosing input/output.
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Insets {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}

/// Relative to the cell. The containing cell/row is part of the placement owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineOwner {
    pub paragraph: usize,
    pub line: usize,
}

/// An occupied line box, not an ink box. Empty lines have the same contract.
#[derive(Debug, Clone, PartialEq)]
pub struct LineBox {
    pub owner: LineOwner,
    /// Relative to the content origin; composition has already resolved spacing.
    pub bounds: Rect,
}

#[derive(Debug, Clone)]
pub struct ComposedCell {
    /// Exact width constraint used by the composer, excluding cell padding.
    pub width: f64,
    /// Physical content extent, including empty/trailing bands. Not text length.
    pub height: f64,
    pub lines: Vec<LineBox>,
}

#[derive(Debug, Clone)]
pub struct CellInput {
    /// Resolved padding. Choosing table-default vs cell-specific belongs to the IR adapter.
    pub padding: Insets,
    /// Minimum physical height, including padding (not a page-count correction).
    pub minimum_height: f64,
    pub content: ComposedCell,
}

/// Rectangular, non-spanning rows only. Nested content must not be flattened into
/// these lines: its cuts and ownership require the recursive contract extension.
#[derive(Debug, Clone)]
pub struct RowInput {
    pub cells: Vec<CellInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ControlOwner {
    pub paragraph: usize,
    pub control: usize,
}

/// Composition owns atomic line groups; pagination never infers them from ink.
#[derive(Debug)]
pub enum FlowBlock {
    Space(f64),
    Lines {
        height: f64,
        lines: Vec<LineBox>,
    },
    Table {
        owner: ControlOwner,
        plan: std::sync::Arc<super::TableContentPlan>,
    },
}

#[derive(Debug)]
pub struct FlowCellInput {
    pub padding: Insets,
    pub minimum_height: f64,
    pub width: f64,
    pub blocks: Vec<FlowBlock>,
}

#[derive(Debug)]
pub struct FlowRowInput {
    pub cells: Vec<FlowCellInput>,
}

/// Deliberately independent of TAC/wrap/anchor selection in the paragraph flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitPolicy {
    Never,
    BetweenRows,
    /// Representable so callers receive an explicit error instead of fallback.
    WithinCells,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageArea {
    /// Actual available area after external reservations, in page coordinates.
    pub bounds: Rect,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinePlacement {
    pub owner: LineOwner,
    pub bounds: Rect,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CellPlacement {
    pub row: usize,
    pub column: usize,
    pub column_span: usize,
    pub bounds: Rect,
    pub content_origin: (f64, f64),
    pub lines: Vec<LinePlacement>,
    pub tables: Vec<NestedTablePlacement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NestedTablePlacement {
    pub owner: ControlOwner,
    pub placement: Box<TablePlacement>,
}

/// Final geometry, consumed verbatim by placement; no second height calculation.
#[derive(Debug, Clone, PartialEq)]
pub struct TablePlacement {
    pub bounds: Rect,
    pub cells: Vec<CellPlacement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeometryError {
    InvalidNumber(&'static str),
    EmptyTable,
    CellCount { row: usize },
    ContentWidth { row: usize, column: usize },
    ContentBounds { row: usize, column: usize },
    DuplicateLineOwner { row: usize, column: usize },
    UnsupportedCellSplit,
    InconsistentAtomicPlan,
    Unsupported(&'static str),
}

impl std::fmt::Display for GeometryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "table_v2 geometry: {self:?}")
    }
}

impl std::error::Error for GeometryError {}

pub(super) fn nonnegative(value: f64, name: &'static str) -> Result<(), GeometryError> {
    if !value.is_finite() || value < 0.0 {
        return Err(GeometryError::InvalidNumber(name));
    }
    Ok(())
}

pub(super) fn finite(value: f64, name: &'static str) -> Result<(), GeometryError> {
    if !value.is_finite() {
        return Err(GeometryError::InvalidNumber(name));
    }
    Ok(())
}
