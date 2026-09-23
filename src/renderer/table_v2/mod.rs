//! Experimental table layout, developed alongside the unchanged Legacy path.
//!
//! The geometry boundary accepts **already composed**, width-bound cell content.
//! `PreparedTextTable` also supplies a narrow fresh-text preview adapter using
//! the shared paragraph composer. Neither path admits stored LineSeg or selects
//! an engine in DocumentCore.
//! All coordinates use one caller-chosen unit (normally layout pixels). Querying
//! a fragment does not mutate a document, a page, or its continuation cursor.
//! Public only as an experimental consumer boundary, not a stable document API.

mod content;
mod contracts;
mod flow;
mod fragment;
mod ir;
mod session;
mod text;
mod text_flow;

pub use content::TableContentPlan;
pub use contracts::{
    CellInput, CellPlacement, ComposedCell, ControlOwner, FlowBlock, FlowCellInput, FlowRowInput,
    GeometryError, Insets, LineBox, LineOwner, LinePlacement, NestedTablePlacement, PageArea, Rect,
    RowInput, SplitPolicy, TablePlacement,
};
pub use fragment::{FragmentFit, TableCursor, TableFragmentPlan};
pub use ir::{CellParagraphComposer, ParagraphItem};
pub use session::{
    TablePreviewError, TablePreviewPage, TablePreviewPages, TablePreviewSession, TableSelection,
};
pub use text::{PreparedTextTable, TextFragment, TextFragmentFit, TextTableCursor};
pub use text_flow::{TextFlowBlock, TextFlowCell, TextFlowRow};
