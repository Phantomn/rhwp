//! V2 fragments in host-owned document frames. The existing document flow owns
//! sections, columns, exclusions and page changes; this adapter owns only one
//! table's immutable composition and continuation. No Legacy row-cut conversion.
//!
//! This is a transport boundary, not a DocumentCore engine switch. A caller must
//! reserve the returned occupied bounds in its flow and retain the same fragment
//! for paint. The standalone document/preview APIs remain independent callers.
use std::sync::Arc;

use crate::{
    model::document::Document,
    renderer::{
        page_layout::PageLayoutInfo,
        render_tree::{PageLayoutContext, PageRenderTree, RenderNode, RenderNodeType},
    },
};

use super::{
    CellEndPolicy, GeometryError, PageArea, Rect, TablePreviewError, TableSelection, TextFragment,
    TextFragmentFit, TextTableCursor,
};

/// Identity of a host reservation. `revision` must change if prior content,
/// exclusions or page/column ownership change, even when the rectangle does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableHostAddress {
    pub section: usize,
    pub page: u32,
    /// None denotes a body-wide reservation, not an inferred column fallback.
    pub column: Option<usize>,
    pub revision: u64,
}

/// Absolute page coordinates resolved by the host, including its reservations.
/// The host passes its effective zone layout, not an unrelated default ColumnDef.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TableHostFrame {
    address: TableHostAddress,
    available: Rect,
    full_height: f64,
    page_width: f64,
    page_height: f64,
    dpi: f64,
    printed_page: Option<u32>,
}

#[derive(Debug)]
pub enum HostedTableError {
    Serialize(String),
    Source(TablePreviewError),
    Geometry(GeometryError),
    InvalidFrame,
    DifferentSectionOrDpi,
    StaleProposal,
    WrongDestination,
    RevisionOverflow,
    UnsupportedHost(&'static str),
    Paragraph {
        index: usize,
        cause: Box<HostedTableError>,
    },
    NoProgress,
}

impl std::fmt::Display for HostedTableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "table_v2 host: {self:?}")
    }
}
impl std::error::Error for HostedTableError {}
impl HostedTableError {
    pub(crate) fn in_paragraph(self, index: usize) -> Self {
        Self::Paragraph {
            index,
            cause: Box::new(self),
        }
    }
}
impl From<GeometryError> for HostedTableError {
    fn from(value: GeometryError) -> Self {
        Self::Geometry(value)
    }
}

impl TableHostFrame {
    pub fn new(
        layout: &PageLayoutInfo,
        address: TableHostAddress,
        available: Rect,
    ) -> Result<Self, HostedTableError> {
        let lane = match address.column {
            Some(index) => layout.column_areas.get(index),
            None => Some(&layout.body_area),
        }
        .ok_or(HostedTableError::InvalidFrame)?;
        // Coordinate conversion/addition may differ by a few ulps (30 versus
        // 30.000000000000004). This validates containment only; it neither
        // changes the supplied box nor grants a typesetting overflow allowance.
        let roundoff = f64::EPSILON * layout.page_width.max(layout.page_height) * 8.0;
        let valid = [
            layout.page_width,
            layout.page_height,
            layout.dpi,
            lane.width,
            lane.height,
            available.width,
        ]
        .iter()
        .all(|v| v.is_finite() && *v > 0.0)
            && [lane.x, lane.y, available.x, available.y, available.height]
                .iter()
                .all(|v| v.is_finite() && *v >= 0.0)
            && lane.x + lane.width <= layout.page_width + roundoff
            && lane.y + lane.height <= layout.page_height + roundoff
            && available.x + roundoff >= lane.x
            && available.y + roundoff >= lane.y
            && available.x + available.width <= lane.x + lane.width + roundoff
            && available.y + available.height <= lane.y + lane.height + roundoff;
        if !valid {
            return Err(HostedTableError::InvalidFrame);
        }
        Ok(Self {
            address,
            available,
            full_height: lane.height,
            page_width: layout.page_width,
            page_height: layout.page_height,
            dpi: layout.dpi,
            printed_page: None,
        })
    }

    pub fn address(&self) -> TableHostAddress {
        self.address
    }

    pub fn with_page_number(mut self, number: u32) -> Result<Self, HostedTableError> {
        if number == 0 || number > u32::from(u16::MAX) {
            return Err(HostedTableError::InvalidFrame);
        }
        self.printed_page = Some(number);
        Ok(self)
    }

    pub fn available(&self) -> Rect {
        self.available
    }
}

pub enum HostedTableFit {
    Placed(Box<HostedTableProposal>),
    DoesNotFit {
        required_width: f64,
        required_height: f64,
    },
    Complete,
}

/// Not clonable or constructible by callers. A proposal keeps its geometry,
/// payloads and continuation together through query, reservation and paint.
pub struct HostedTableProposal {
    owner: Arc<()>,
    revision: u64,
    frame: TableHostFrame,
    fragment: TextFragment,
}

impl HostedTableProposal {
    pub fn frame(&self) -> TableHostFrame {
        self.frame
    }

    /// Host flow reservation and paint use this exact occupied box. Object outer
    /// margins/anchor offsets belong to the host, not an extra inferred V2 gap.
    pub fn occupied(&self) -> Rect {
        self.fragment.geometry().placement().bounds
    }
}

/// Accepted pagination result carried intact to layout/paint. Rebuilding a page
/// may replay this immutable packet; it never advances the table cursor again.
pub struct HostedTableFragment {
    selection: TableSelection,
    frame: TableHostFrame,
    fragment: TextFragment,
    // Paint validation finishes before the host commits occupancy. The layout
    // consumer cannot fail halfway through publication or retry with Legacy.
    node: RenderNode,
    first: bool,
}

impl std::fmt::Debug for HostedTableFragment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostedTableFragment")
            .field("selection", &self.selection)
            .field("frame", &self.frame)
            .field("occupied", &self.occupied())
            .finish_non_exhaustive()
    }
}

impl HostedTableFragment {
    pub(crate) fn is_first(&self) -> bool {
        self.first
    }
    /// Original outer table address; cell/child ownership remains in the V2
    /// plan. This is not yet a Studio cursor/hit-test binding.
    pub fn selection(&self) -> TableSelection {
        self.selection
    }

    pub fn frame(&self) -> TableHostFrame {
        self.frame
    }

    pub fn occupied(&self) -> Rect {
        self.fragment.geometry().placement().bounds
    }

    pub fn append_to(&self, page: &mut PageRenderTree) -> Result<(), HostedTableError> {
        let RenderNodeType::Page(destination) = &page.root.node_type else {
            return Err(HostedTableError::WrongDestination);
        };
        if destination.page_index != self.frame.address.page
            || destination.section_index != self.frame.address.section
            || destination.width != self.frame.page_width
            || destination.height != self.frame.page_height
        {
            return Err(HostedTableError::WrongDestination);
        }
        let node = self.render_node(page.frame_mut());
        page.root.children.push(node);
        Ok(())
    }

    /// Only committed, paint-validated packets reach the existing layout loop.
    pub(crate) fn render_node(&self, context: &mut PageLayoutContext) -> RenderNode {
        let mut node = self.node.clone();
        super::text::assign_ids(&mut node, context);
        node
    }
}

/// One source table, including its descendants, pinned to the V2 path. Dropping
/// and preparing anew is the explicit invalidation boundary after an edit.
/// This object never selects a next column/page or invokes Legacy fallback.
pub struct HostedTableSession {
    owner: Arc<()>,
    revision: u64,
    cursor: TextTableCursor,
    selection: TableSelection,
    dpi: f64,
}

impl HostedTableSession {
    pub fn from_document(
        document: &Document,
        selection: TableSelection,
        dpi: f64,
        policy: CellEndPolicy,
    ) -> Result<Self, HostedTableError> {
        let prepared = super::session::prepare_selected_table(document, selection, dpi, policy)
            .map_err(HostedTableError::Source)?;
        Ok(Self {
            owner: Arc::new(()),
            revision: 0,
            cursor: prepared.start(),
            selection,
            dpi,
        })
    }

    pub fn is_complete(&self) -> bool {
        self.cursor.is_complete()
    }

    pub(super) fn from_document_with_styles(
        document: &Document,
        selection: TableSelection,
        dpi: f64,
        policy: CellEndPolicy,
        styles: &crate::renderer::style_resolver::ResolvedStyleSet,
    ) -> Result<Self, HostedTableError> {
        let prepared = super::session::prepare_selected_table_with_styles(
            document, selection, dpi, policy, styles,
        )
        .map_err(HostedTableError::Source)?;
        Ok(Self {
            owner: Arc::new(()),
            revision: 0,
            cursor: prepared.start(),
            selection,
            dpi,
        })
    }

    /// Pure query: repeated fit, non-fit and an abandoned proposal consume no
    /// content. The same snapshot supplies the bounds and eventual paint.
    pub fn query(&self, frame: TableHostFrame) -> Result<HostedTableFit, HostedTableError> {
        if frame.address.section != self.selection.section || frame.dpi != self.dpi {
            return Err(HostedTableError::DifferentSectionOrDpi);
        }
        Ok(
            match self.cursor.fit_with_page_height(
                PageArea {
                    bounds: frame.available,
                },
                Some(frame.full_height),
            )? {
                TextFragmentFit::Placed(fragment) => {
                    HostedTableFit::Placed(Box::new(HostedTableProposal {
                        owner: self.owner.clone(),
                        revision: self.revision,
                        frame,
                        fragment,
                    }))
                }
                TextFragmentFit::DoesNotFit {
                    required_width,
                    required_height,
                } => HostedTableFit::DoesNotFit {
                    required_width,
                    required_height,
                },
                TextFragmentFit::Complete => HostedTableFit::Complete,
            },
        )
    }

    /// The host must re-present its current reservation snapshot. A changed
    /// host, stale query or another session's proposal cannot consume content.
    /// The host records the returned packet AND its occupied box in the same
    /// reservation step. A later paint pass consumes this packet, never queries
    /// the table again or converts the packet into Legacy PartialTable cuts.
    pub fn commit(
        &mut self,
        proposal: HostedTableProposal,
        current_frame: TableHostFrame,
    ) -> Result<HostedTableFragment, HostedTableError> {
        if !Arc::ptr_eq(&proposal.owner, &self.owner)
            || proposal.revision != self.revision
            || proposal.frame != current_frame
        {
            return Err(HostedTableError::StaleProposal);
        }
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(HostedTableError::RevisionOverflow)?;
        let node = proposal.fragment.build_node(current_frame.printed_page)?;
        self.cursor = proposal.fragment.continuation();
        self.revision = next_revision;
        Ok(HostedTableFragment {
            selection: self.selection,
            frame: current_frame,
            fragment: proposal.fragment,
            node,
            first: proposal.revision == 0,
        })
    }
}
