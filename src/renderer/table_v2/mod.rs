//! Experimental table layout, developed alongside the unchanged Legacy path.
//!
//! The geometry boundary accepts **already composed**, width-bound cell content.
//! `PreparedTextTable` supplies fresh text and qualified plain stored rows using
//! the shared paragraph composer. DocumentV2Session is explicitly selected;
//! neither path switches the engine in DocumentCore.
//! All coordinates use one caller-chosen unit (normally layout pixels). Querying
//! a fragment does not mutate a document, a page, or its continuation cursor.
//! Public only as an experimental consumer boundary, not a stable document API.

mod borders;
mod content;
mod contracts;
mod decoration;
mod document;
mod document_input;
mod export;
mod flow;
mod fragment;
mod grid;
mod ir;
mod session;
mod stored_text;
mod text;
mod text_flow;
mod text_ir;

pub use content::TableContentPlan;
pub use contracts::{
    CellInput, CellPlacement, ComposedCell, ControlOwner, FlowBlock, FlowCellInput, FlowRowInput,
    GeometryError, Insets, LineBox, LineOwner, LinePlacement, NestedTablePlacement, PageArea, Rect,
    RowInput, SplitPolicy, TablePlacement,
};
pub use document::{DocumentV2Error, DocumentV2Session};
pub use export::{TablePreviewExportError, TablePreviewExportSession};
pub use fragment::{FragmentFit, TableCursor, TableFragmentPlan};
pub use ir::{CellParagraphComposer, ParagraphItem};
pub use session::{
    TablePreviewError, TablePreviewPage, TablePreviewPages, TablePreviewSession, TableSelection,
};
pub use text::{PreparedTextTable, TextFragment, TextFragmentFit, TextTableCursor};
pub use text_flow::{TextFlowBlock, TextFlowCell, TextFlowRow};
