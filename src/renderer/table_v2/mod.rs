//! Experimental table layout, developed alongside the unchanged Legacy path.
//!
//! This entry point accepts **already composed**, width-bound cell content. It
//! does not parse stored LineSeg, shape text, or select an engine in DocumentCore.
//! All coordinates use one caller-chosen unit (normally layout pixels). Querying
//! a fragment does not mutate a document, a page, or its continuation cursor.
//! Public only as an experimental consumer boundary, not a stable document API.

mod content;
mod contracts;
mod flow;
mod fragment;
mod ir;

pub use content::TableContentPlan;
pub use contracts::{
    CellInput, CellPlacement, ComposedCell, ControlOwner, FlowBlock, FlowCellInput, FlowRowInput,
    GeometryError, Insets, LineBox, LineOwner, LinePlacement, NestedTablePlacement, PageArea, Rect,
    RowInput, SplitPolicy, TablePlacement,
};
pub use fragment::{FragmentFit, TableCursor, TableFragmentPlan};
pub use ir::{CellParagraphComposer, ParagraphItem};
