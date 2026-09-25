//! Paragraph-local occupied end and following origin are different quantities.
//! Keep the producer's tail bands until the flow boundary; an explicit Space in
//! the surrounding cell is not a paragraph and cannot change this result.
use super::{contracts::nonnegative, GeometryError, ParagraphItem};

/// Explicit preview experiment; existing previews retain their advance policy.
/// This does not select a document engine or qualify unsupported source data.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CellEndPolicy {
    #[default]
    PreserveAdvance,
    OmitFinalLineGap,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParagraphEnd {
    occupied_end: f64,
    next_origin: f64,
    tail_spaces: Vec<f64>,
    terminal_spaces: Vec<f64>,
}

impl ParagraphEnd {
    /// Measure the already composed row recipe, not source text or declared
    /// object heights. Deferred floating anchors require their own frame first.
    pub(super) fn from_composed(
        items: &[ParagraphItem],
        tail_spaces: Vec<f64>,
        paragraph_after: f64,
    ) -> Result<Self, GeometryError> {
        let mut occupied_end: f64 = 0.0;
        let mut flow_end = 0.0;
        for item in items {
            let (height, advance) = match item {
                ParagraphItem::Space(height) => (*height, *height),
                ParagraphItem::ObjectRow { bounds, .. } => (bounds.height, bounds.height),
                ParagraphItem::Lines {
                    height, advance, ..
                }
                | ParagraphItem::InlineTables {
                    height, advance, ..
                } => (*height, *advance),
                ParagraphItem::TableControl(_) | ParagraphItem::End(_) => {
                    return Err(GeometryError::Unsupported("unresolved paragraph ending"));
                }
            };
            nonnegative(height, "paragraph row height")?;
            nonnegative(advance, "paragraph row advance")?;
            occupied_end = occupied_end.max(flow_end + height);
            flow_end += advance;
        }
        let mut end = Self::new(occupied_end, flow_end, tail_spaces)?;
        nonnegative(paragraph_after, "paragraph after spacing")?;
        // Only the next-line gap is optional. Keep explicit paragraph-after;
        // its independent cell-end policy is not decided by this experiment.
        end.terminal_spaces = vec![paragraph_after];
        Ok(end)
    }

    /// All coordinates are local to the composed paragraph content frame.
    /// `flow_end` is the pen after its rows, which may precede `occupied_end`
    /// for a TAC row with negative spacing. Tail bands are already composed;
    /// they must not be inferred from text visibility or the cell's last item.
    /// Keep their order (and separate floating-point additions) unchanged.
    pub fn new(
        occupied_end: f64,
        flow_end: f64,
        tail_spaces: Vec<f64>,
    ) -> Result<Self, GeometryError> {
        nonnegative(occupied_end, "paragraph occupied end")?;
        nonnegative(flow_end, "paragraph flow end")?;
        let mut next_origin = flow_end;
        for space in &tail_spaces {
            nonnegative(*space, "paragraph tail space")?;
            next_origin += space;
            nonnegative(next_origin, "paragraph next origin")?;
        }
        Ok(Self {
            occupied_end,
            next_origin,
            terminal_spaces: tail_spaces.clone(),
            tail_spaces,
        })
    }

    pub fn occupied_end(&self) -> f64 {
        self.occupied_end
    }

    pub fn next_origin(&self) -> f64 {
        self.next_origin
    }
}

/// Single lowering boundary for IR cells, explicit cell flows and document
/// paragraphs. This preserves the existing tail policy; it does NOT trim a
/// terminal gap or treat a blank paragraph as empty physical space.
pub(super) fn into_flow_items(items: Vec<ParagraphItem>) -> Vec<ParagraphItem> {
    into_flow_items_at_end(items, CellEndPolicy::PreserveAdvance, false)
}

pub(super) fn into_flow_items_at_end(
    items: Vec<ParagraphItem>,
    policy: CellEndPolicy,
    final_paragraph: bool,
) -> Vec<ParagraphItem> {
    let mut resolved = Vec::with_capacity(items.len());
    for item in items {
        match item {
            ParagraphItem::End(end) => {
                let spaces = if final_paragraph && policy == CellEndPolicy::OmitFinalLineGap {
                    // A negative final text gap is carried by its row advance,
                    // not a negative physical Space. Omit it at this same
                    // producer-owned boundary, retaining paragraph-after.
                    if let Some(ParagraphItem::Lines {
                        height, advance, ..
                    }) = resolved.last_mut()
                    {
                        *advance = *height;
                    }
                    end.terminal_spaces
                } else {
                    end.tail_spaces
                };
                resolved.extend(spaces.into_iter().map(ParagraphItem::Space));
            }
            item => resolved.push(item),
        }
    }
    resolved
}
