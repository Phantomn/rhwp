//! V2 paragraph/table reservations and their page owner. No Legacy table state.
use super::host_flow_state::PageFlow;
use super::{
    HostedAnchor, HostedParagraphPlan, HostedTableError, HostedTableFit, HostedTableSession, Rect,
    TableHostAddress, TableHostFrame,
};
use crate::{
    model::{
        page::{ColumnDef, PageDef},
        paragraph::Paragraph,
    },
    renderer::{
        pagination::{PageItem, PaginationResult},
        style_resolver::ResolvedStyleSet,
    },
};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};
pub(super) struct HostedSectionFlow {
    text: Vec<Option<HostedParagraphPlan>>,
    tables: BTreeMap<usize, HostedTableSession>,
    page_numbers: crate::renderer::page_number::PageNumberAssigner<'static>,
    assigned_numbers: Vec<u32>,
    page_offset: u32,
    revision: u64,
    pending: VecDeque<(usize, HostedAnchor, (u32, u16))>,
    hidden_empty: Option<((u32, u16), u8)>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn paginate(
    paragraphs: &[Paragraph],
    text: Vec<HostedParagraphPlan>,
    styles: &ResolvedStyleSet,
    page: &PageDef,
    columns: &ColumnDef,
    section: usize,
    hide_empty_line: bool,
    tables: BTreeMap<usize, HostedTableSession>,
    page_offset: u32,
    first_number: u32,
    dpi: f64,
) -> Result<PaginationResult, HostedTableError> {
    if text.len() != paragraphs.len() {
        return Err(HostedTableError::UnsupportedHost(
            "incomplete paragraph composition",
        ));
    }
    let mut flow = HostedSectionFlow {
        text: text.into_iter().map(Some).collect(),
        tables,
        page_numbers: crate::renderer::page_number::PageNumberAssigner::new_for_pages(
            &[],
            first_number,
        ),
        assigned_numbers: Vec::new(),
        page_offset,
        revision: 0,
        pending: VecDeque::new(),
        hidden_empty: None,
    };
    let mut state = PageFlow::new(page, columns, section, hide_empty_line, dpi);
    for (pi, source) in paragraphs.iter().enumerate() {
        use crate::model::paragraph::ColumnBreakType;
        let page_break = matches!(
            source.column_type,
            ColumnBreakType::Page | ColumnBreakType::Section
        ) || styles
            .para_styles
            .get(source.para_shape_id as usize)
            .is_some_and(|s| s.page_break_before);
        let column_break = source.column_type == ColumnBreakType::Column;
        // A source boundary follows every preceding deferred reservation. It
        // never reassigns that reservation to the following paragraph's page.
        if page_break || column_break {
            flow.finish_pending(&mut state)?;
        }
        if !state.current_items.is_empty() {
            if page_break {
                state.force_new_page();
            } else if column_break {
                state.advance_column_or_new_page();
            }
        }
        flow.place_paragraph(&mut state, pi, source)
            .map_err(|e| e.in_paragraph(pi))?;
    }
    flow.finish(&mut state)?;
    Ok(state.finish())
}

impl HostedSectionFlow {
    /// Only called after the common composition cannot fit (or its remaining
    /// budget is already negative). Hancom's normal ON/OFF saves demonstrate
    /// two suppressed empty lines per exhausted column, not per physical page.
    /// Do not substitute invisible control owners or blank multiline text.
    fn hide_overflowing_empty(
        &mut self,
        st: &mut PageFlow,
        pi: usize,
        source: &Paragraph,
        paragraph: &HostedParagraphPlan,
    ) -> Result<bool, HostedTableError> {
        if !st.hide_empty_line
            || st.current_items.is_empty()
            || !paragraph.is_unconsumed_empty_line(source)
        {
            return Ok(false);
        }
        let frame = Self::frame(st);
        let (previous, count) = self.hidden_empty.get_or_insert((frame, 0));
        if *previous != frame {
            *previous = frame;
            *count = 0;
        }
        if *count == 2 {
            return Ok(false);
        }
        *count += 1;
        // Retain source identity without emitting a Legacy FullParagraph item:
        // such an item would invoke an unrelated paint/height calculation.
        st.hide_empty_paragraph(pi);
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(HostedTableError::RevisionOverflow)?;
        Ok(true)
    }

    fn frame(st: &PageFlow) -> (u32, u16) {
        (
            st.pages.last().expect("host page ensured").page_index,
            st.current_column,
        )
    }

    fn number(&mut self, st: &PageFlow) -> u32 {
        for page in &st.pages[self.assigned_numbers.len()..] {
            self.assigned_numbers.push(self.page_numbers.assign(page));
        }
        *self.assigned_numbers.last().expect("host page numbered")
    }

    /// Deferred reservations wait for a new frame, then precede its story.
    /// Already accepted partial tables drain before following paragraphs.
    fn drain_pending(&mut self, st: &mut PageFlow) -> Result<(), HostedTableError> {
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
        st: &mut PageFlow,
        pi: usize,
        source: &Paragraph,
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
                        page: self
                            .page_offset
                            .checked_add(st.pages.last().expect("host page ensured").page_index)
                            .ok_or(HostedTableError::RevisionOverflow)?,
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
            let forced_break = paragraph.take_frame_break();
            if !forced_break
                && st.current_height > st.available_height()
                && self.hide_overflowing_empty(st, pi, source, &paragraph)?
            {
                return Ok(());
            }
            if forced_break || st.current_height > st.available_height() {
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
            } else if self.hide_overflowing_empty(st, pi, source, &paragraph)? {
                return Ok(());
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

    pub(super) fn finish_pending(&mut self, st: &mut PageFlow) -> Result<(), HostedTableError> {
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

    pub(super) fn finish(&mut self, st: &mut PageFlow) -> Result<(), HostedTableError> {
        self.finish_pending(st)?;
        if !self.tables.is_empty() || self.text.iter().any(Option::is_some) {
            return Err(HostedTableError::UnsupportedHost(
                "unconsumed section content",
            ));
        }
        Ok(())
    }
}
