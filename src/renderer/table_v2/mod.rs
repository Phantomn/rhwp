//! Experimental table layout, developed alongside the unchanged Legacy path.
//!
//! The geometry boundary accepts **already composed**, width-bound cell content.
//! `PreparedTextTable` supplies fresh text and qualified plain stored rows using
//! the shared paragraph composer. Qualified control-only stored TAC rows retain
//! source ownership and reserve their occupied envelope once, atomically.
//! DocumentV2Session is explicitly selected;
//! neither path switches the engine in DocumentCore.
//! All coordinates use one caller-chosen unit (normally layout pixels). Querying
//! a fragment does not mutate a document, a page, or its continuation cursor.
//! Public only as an experimental consumer boundary, not a stable document API.

mod body_anchor;
mod body_excluded;
mod body_flow;
mod body_inline;
mod body_text;
mod borders;
mod cell_anchor;
mod cell_page_field;
mod content;
mod contracts;
mod decoration;
mod diagonal;
mod document;
mod document_input;
mod export;
mod fields;
mod flow;
mod fragment;
mod grid;
mod host;
mod host_absolute;
mod host_anchor;
mod host_section;
mod host_text;
pub(crate) use host_anchor::HostedAnchor;
pub use host_section::HostedSectionSession;
pub use host_text::HostedParagraphFragment;
pub(crate) use host_text::HostedParagraphPlan;
mod ir;
mod page_number;
mod paragraph_end;
mod paragraph_keep;
mod pictures;
mod session;
mod shapes;
mod source_units;
mod stored_child;
mod stored_child_frames;
mod stored_text;
mod tac;
mod tac_fresh;
mod tac_metrics;
mod tac_spaces;
mod text;
mod text_flow;
mod text_ir;
mod zones;

pub use content::TableContentPlan;
pub use contracts::{
    CellInput, CellPlacement, ComposedCell, ControlOwner, FlowBlock, FlowCellInput, FlowRowInput,
    GeometryError, InlineTableInput, Insets, LineBox, LineOwner, LinePlacement,
    NestedTablePlacement, PageArea, Rect, RowInput, SplitPolicy, TablePlacement,
};
pub use document::{DocumentV2Error, DocumentV2Session};
pub use export::{TablePreviewExportError, TablePreviewExportSession};
pub use fragment::{FragmentFit, TableCursor, TableFragmentPlan};
pub use host::{
    HostedTableError, HostedTableFit, HostedTableFragment, HostedTableProposal, HostedTableSession,
    TableHostAddress, TableHostFrame,
};
pub use ir::{CellParagraphComposer, ParagraphItem};
pub use paragraph_end::{CellEndPolicy, ParagraphEnd};
pub use session::{
    TablePreviewError, TablePreviewPage, TablePreviewPages, TablePreviewSession, TableSelection,
};
pub use tac::{stored_tac_rows, StoredTacRow};
pub use text::{PreparedTextTable, TextFragment, TextFragmentFit, TextTableCursor};
pub use text_flow::{TextFlowBlock, TextFlowCell, TextFlowRow};
