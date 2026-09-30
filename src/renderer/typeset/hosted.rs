//! V2 placement policy for the existing section driver. This module has no
//! source-order loop or separate page finalization: run_section owns both.
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};

use super::{
    ColumnDef, EndnoteDeferral, PageDef, PageItem, PaginationResult, Paragraph, ResolvedStyleSet,
    TypesetEngine, TypesetState,
};
use crate::renderer::table_v2::{
    HostedAnchor, HostedParagraphPlan, HostedTableError, HostedTableFit, HostedTableSession, Rect,
    TableHostAddress, TableHostFrame,
};

pub(super) struct HostedSectionFlow {
    text: Vec<Option<HostedParagraphPlan>>,
    tables: BTreeMap<usize, HostedTableSession>,
    page_numbers: crate::renderer::page_number::PageNumberAssigner<'static>,
    assigned_numbers: Vec<u32>,
    revision: u64,
    pending: VecDeque<(usize, HostedAnchor, (u32, u16))>,
}

impl TypesetEngine {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn typeset_hosted_section(
        &self,
        paragraphs: &[Paragraph],
        text: Vec<HostedParagraphPlan>,
        styles: &ResolvedStyleSet,
        page: &PageDef,
        columns: &ColumnDef,
        section: usize,
        tables: BTreeMap<usize, HostedTableSession>,
    ) -> Result<PaginationResult, HostedTableError> {
        if text.len() != paragraphs.len() {
            return Err(HostedTableError::UnsupportedHost(
                "incomplete paragraph composition",
            ));
        }
        let mut flow = HostedSectionFlow {
            text: text.into_iter().map(Some).collect(),
            tables,
            // Admission excludes NewNumber and nondefault section start numbers.
            // Number each physical page, not each column, as in finalization.
            page_numbers: crate::renderer::page_number::PageNumberAssigner::new_for_pages(&[], 1),
            assigned_numbers: Vec::new(),
            revision: 0,
            pending: VecDeque::new(),
        };
        self.run_section(
            paragraphs,
            &[],
            styles,
            page,
            columns,
            section,
            &[],
            false,
            Default::default(),
            false,
            false,
            None,
            None,
            &Default::default(),
            EndnoteDeferral::None,
            Some(&mut flow),
        )
    }
}

impl HostedSectionFlow {
    fn frame(st: &TypesetState) -> (u32, u16) {
        (
            st.pages.last().expect("host page ensured").page_index,
            st.current_column,
        )
    }

    fn number(&mut self, st: &TypesetState) -> u32 {
        for page in &st.pages[self.assigned_numbers.len()..] {
            self.assigned_numbers.push(self.page_numbers.assign(page));
        }
        *self.assigned_numbers.last().expect("host page numbered")
    }

    /// Deferred reservations wait for a new frame, then precede its story.
    /// Already accepted partial tables drain before following paragraphs.
    fn drain_pending(&mut self, st: &mut TypesetState) -> Result<(), HostedTableError> {
        while let Some((pi, mut anchor, blocked)) = self.pending.pop_front() {
            if blocked == Self::frame(st) {
                self.pending.push_front((pi, anchor, blocked));
                break;
            }
            while !anchor.complete() {
                let number = self.number(st);
                let lane = st.layout.column_areas[usize::from(st.current_column)];
                let available = st.available_height() - st.current_height;
                let packet = if available >= 0.0 {
                    anchor.fit(
                        Rect {
                            x: lane.x,
                            y: lane.y + st.current_height,
                            width: lane.width,
                            height: available,
                        },
                        st.available_height(),
                        st.section_index,
                        pi,
                        number,
                    )?
                } else {
                    None
                };
                if let Some(packet) = packet {
                    let next = packet.next_y() - lane.y;
                    st.append_item(PageItem::HostedParagraph {
                        para_index: pi,
                        fragment: Arc::new(packet),
                    });
                    st.align_flow_to(next);
                } else if st.current_items.is_empty() && st.current_height == 0.0 {
                    return Err(HostedTableError::NoProgress);
                }
                if !anchor.complete() {
                    st.advance_column_or_new_page();
                }
            }
        }
        Ok(())
    }

    pub(super) fn place_paragraph(
        &mut self,
        st: &mut TypesetState,
        pi: usize,
    ) -> Result<(), HostedTableError> {
        self.drain_pending(st)?;
        let mut last_line = None;
        let mut host_lines = None;
        let mut paragraph = self.text.get_mut(pi).and_then(Option::take).ok_or(
            HostedTableError::UnsupportedHost("paragraph already consumed"),
        )?;
        if let Some(mut table) = self.tables.remove(&pi) {
            while !table.is_complete() {
                for page in &st.pages[self.assigned_numbers.len()..] {
                    self.assigned_numbers.push(self.page_numbers.assign(page));
                }
                let lane = st.layout.column_areas[usize::from(st.current_column)];
                let available = st.available_height() - st.current_height;
                if available < 0.0 {
                    st.advance_column_or_new_page();
                    self.drain_pending(st)?;
                    continue;
                }
                // The owner line and first table fragment share this origin.
                // Check their joint budget before either cursor can advance.
                if paragraph.anchor_required().is_some_and(|h| h > available) {
                    if st.current_items.is_empty() && st.current_height == 0.0 {
                        return Err(HostedTableError::NoProgress);
                    }
                    st.advance_column_or_new_page();
                    self.drain_pending(st)?;
                    self.revision = self
                        .revision
                        .checked_add(1)
                        .ok_or(HostedTableError::RevisionOverflow)?;
                    continue;
                }
                let frame = TableHostFrame::new(
                    &st.layout,
                    TableHostAddress {
                        section: st.section_index,
                        page: st.pages.last().expect("host page ensured").page_index,
                        column: Some(usize::from(st.current_column)),
                        revision: self.revision,
                    },
                    Rect {
                        x: lane.x,
                        y: lane.y + st.current_height,
                        width: lane.width,
                        height: available,
                    },
                )?
                .with_page_number(*self.assigned_numbers.last().expect("host page numbered"))?;
                match table.query(frame)? {
                    HostedTableFit::Placed(proposal) => {
                        // Validate paint and commit continuation BEFORE reserving.
                        // The exact occupied bottom advances the host once.
                        let packet = Arc::new(table.commit(*proposal, frame)?);
                        let bounds = packet.occupied();
                        let mut occupied_end = bounds.y + bounds.height;
                        if paragraph.anchor_required().is_some() {
                            let owner = paragraph
                                .fit(
                                    frame.available(),
                                    st.section_index,
                                    pi,
                                    st.available_height(),
                                    *self.assigned_numbers.last().expect("host page numbered"),
                                )?
                                .ok_or(HostedTableError::NoProgress)?;
                            occupied_end = occupied_end
                                .max(owner.next_y())
                                .max(owner.occupied().y + owner.occupied().height);
                            st.append_item(PageItem::HostedParagraph {
                                para_index: pi,
                                fragment: Arc::new(owner),
                            });
                        }
                        st.append_item(PageItem::HostedTable {
                            para_index: pi,
                            fragment: packet,
                        });
                        st.align_flow_to(occupied_end - lane.y);
                    }
                    HostedTableFit::DoesNotFit { .. } => {
                        // A failed query never consumes a content unit. Retrying
                        // an empty equal-width column cannot make progress.
                        if st.current_items.is_empty() && st.current_height == 0.0 {
                            return Err(HostedTableError::NoProgress);
                        }
                        st.advance_column_or_new_page();
                        self.drain_pending(st)?;
                    }
                    HostedTableFit::Complete => break,
                }
                self.revision = self
                    .revision
                    .checked_add(1)
                    .ok_or(HostedTableError::RevisionOverflow)?;
            }
        }
        // A free owner line follows the table. An occluded saved owner line was
        // already placed at the first fragment's anchor, not dropped or doubled.
        while !paragraph.complete() {
            if paragraph.take_frame_break() || st.current_height > st.available_height() {
                st.advance_column_or_new_page();
                self.drain_pending(st)?;
            }
            for page in &st.pages[self.assigned_numbers.len()..] {
                self.assigned_numbers.push(self.page_numbers.assign(page));
            }
            let lane = st.layout.column_areas[usize::from(st.current_column)];
            let area = Rect {
                x: lane.x,
                y: lane.y + st.current_height,
                width: lane.width,
                height: st.available_height() - st.current_height,
            };
            if let Some(packet) = paragraph.fit(
                area,
                st.section_index,
                pi,
                st.available_height(),
                *self.assigned_numbers.last().expect("host page numbered"),
            )? {
                if let Some(end) = packet.line_end() {
                    last_line = Some((Self::frame(st), end));
                    let (frame, count) = host_lines.get_or_insert((Self::frame(st), 0));
                    if *frame != Self::frame(st) {
                        *frame = Self::frame(st);
                        *count = 0;
                    }
                    *count += packet.line_count();
                }
                let next = packet.next_y() - lane.y;
                st.append_item(PageItem::HostedParagraph {
                    para_index: pi,
                    fragment: Arc::new(packet),
                });
                st.align_flow_to(next);
            } else if st.current_items.is_empty() && st.current_height == 0.0 {
                return Err(HostedTableError::NoProgress);
            } else {
                st.advance_column_or_new_page();
                self.drain_pending(st)?;
            }
            self.revision = self
                .revision
                .checked_add(1)
                .ok_or(HostedTableError::RevisionOverflow)?;
        }

        if let Some(mut anchor) = paragraph.take_anchor() {
            let frame = Self::frame(st);
            let side_wrap = anchor.is_side_wrap();
            if side_wrap && host_lines != Some((frame, paragraph.planned_line_count())) {
                return Err(crate::renderer::table_v2::GeometryError::Unsupported(
                    "stored body side-wrap across pages",
                )
                .into());
            }
            let lane = st.layout.column_areas[usize::from(st.current_column)];
            let host_end = last_line
                .filter(|(owner_frame, _)| *owner_frame == frame)
                .map(|(_, end)| end);
            if host_end.is_none() {
                anchor.defer();
            }
            let y = host_end.unwrap_or(lane.y + st.current_height);
            let remaining = (lane.y + st.available_height() - y).max(0.0);
            let number = self.number(st);
            if let Some(packet) = anchor.fit(
                Rect {
                    x: lane.x,
                    y,
                    width: lane.width,
                    height: remaining,
                },
                st.available_height(),
                st.section_index,
                pi,
                number,
            )? {
                let next = (packet.next_y() - lane.y).max(st.current_height);
                st.append_item(PageItem::HostedParagraph {
                    para_index: pi,
                    fragment: Arc::new(packet),
                });
                if !side_wrap {
                    st.align_flow_to(next);
                }
                if !anchor.complete() {
                    self.pending.push_back((pi, anchor, frame));
                    st.advance_column_or_new_page();
                    self.drain_pending(st)?;
                }
            } else {
                anchor.defer();
                self.pending.push_back((pi, anchor, frame));
            }
        }
        Ok(())
    }

    pub(super) fn finish_pending(&mut self, st: &mut TypesetState) -> Result<(), HostedTableError> {
        if !self.pending.is_empty() {
            if self
                .pending
                .front()
                .is_some_and(|(_, _, frame)| *frame == Self::frame(st))
            {
                st.advance_column_or_new_page();
            }
            self.drain_pending(st)?;
        }
        Ok(())
    }

    pub(super) fn finish(&mut self, st: &mut TypesetState) -> Result<(), HostedTableError> {
        self.finish_pending(st)?;
        if !self.tables.is_empty() || self.text.iter().any(Option::is_some) {
            return Err(HostedTableError::UnsupportedHost(
                "unconsumed section content",
            ));
        }
        Ok(())
    }
}
