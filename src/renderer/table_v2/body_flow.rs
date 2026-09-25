//! Story order and positioned-table reservation are different cursors. An
//! unplaced floating table must not consume its requested position as blank
//! story content or move the following prose to its eventual destination page.
use std::collections::VecDeque;

use super::{
    document::BodyPlan,
    flow::{FlowCursor, FlowFit},
    FlowCellInput, GeometryError, LinePlacement, NestedTablePlacement, Rect,
};

pub(super) struct AnchoredFlow {
    /// Story boundary after the composed host, without a fabricated spacer.
    pub boundary: usize,
    pub initial: FlowCellInput,
    /// Source offset is not repeated when the first fragment could not fit.
    pub deferred: FlowCellInput,
}

#[derive(Clone)]
struct Pending {
    anchor: usize,
    deferred: bool,
    cursor: FlowCursor,
}

#[derive(Clone, Default)]
pub(super) struct BodyCursor {
    story: FlowCursor,
    anchor: usize,
    pending: VecDeque<Pending>,
}

pub(super) struct BodyFit {
    pub next: BodyCursor,
    pub height: f64,
    pub progressed: bool,
    pub required: f64,
    pub lines: Vec<LinePlacement>,
    pub tables: Vec<NestedTablePlacement>,
}

impl BodyFit {
    fn accept(&mut self, fit: FlowFit, pen: &mut f64) -> FlowCursor {
        self.height = self.height.max(*pen + fit.height);
        *pen += fit.advance;
        self.progressed |= fit.progressed;
        self.required = fit.required;
        self.lines.extend(fit.lines);
        self.tables.extend(fit.tables);
        fit.next
    }
}

impl BodyCursor {
    pub fn complete(&self, plan: &BodyPlan) -> bool {
        self.story.block == plan.flow.blocks.len()
            && self.anchor == plan.anchors.len()
            && self.pending.is_empty()
    }

    pub fn fit(&self, plan: &BodyPlan) -> Result<BodyFit, GeometryError> {
        let mut result = BodyFit {
            next: self.clone(),
            height: 0.0,
            progressed: false,
            required: 0.0,
            lines: Vec::new(),
            tables: Vec::new(),
        };
        let mut pen = 0.0;
        let remaining = |pen: f64| Rect {
            y: plan.body.y + pen,
            height: (plan.body.height - pen).max(0.0),
            ..plan.body
        };

        // Pending reservations take priority on a new page, even if the story
        // has already ended. Reuse the exact child cut and physical tail band;
        // never restart consumed rows or infer completion from visible text.
        while let Some(mut pending) = result.next.pending.pop_front() {
            let anchor = &plan.anchors[pending.anchor];
            let flow = if pending.deferred {
                &anchor.deferred
            } else {
                &anchor.initial
            };
            let fit = pending.cursor.fit(flow, remaining(pen))?;
            if pending.deferred
                && pending.cursor.block == 0
                && fit.tables.is_empty()
                && fit.next.block != flow.blocks.len()
            {
                // A leading outer margin alone is not a table fragment. On an
                // empty page this must report DoesNotFit, not publish repeated
                // margin-only pages for an oversized atomic table.
                result.required = fit.height + fit.required;
                result.next.pending.push_front(pending);
                return Ok(result);
            }
            pending.cursor = result.accept(fit, &mut pen);
            if pending.cursor.block != flow.blocks.len() {
                result.next.pending.push_front(pending);
                return Ok(result);
            }
        }

        loop {
            let boundary = plan
                .anchors
                .get(result.next.anchor)
                .map_or(plan.flow.blocks.len(), |a| a.boundary);
            let fit = result
                .next
                .story
                .fit_until(&plan.flow, remaining(pen), boundary)?;
            result.next.story = result.accept(fit, &mut pen);
            if result.next.story.block != boundary {
                return Ok(result);
            }
            let Some(anchor) = plan.anchors.get(result.next.anchor) else {
                return Ok(result);
            };
            let fit = FlowCursor::default().fit(&anchor.initial, remaining(pen))?;
            let index = result.next.anchor;
            result.next.anchor += 1;
            if fit.tables.is_empty() && fit.next.block != anchor.initial.blocks.len() {
                // No fragment was accepted. Discard this query's initial
                // position band, NOT a real empty paragraph. Continue fitting
                // the story here; the table starts at its outer margin on the
                // next page. The query has not consumed any child unit.
                result.next.pending.push_back(Pending {
                    anchor: index,
                    deferred: true,
                    cursor: FlowCursor::default(),
                });
                continue;
            }
            let cursor = result.accept(fit, &mut pen);
            if cursor.block != anchor.initial.blocks.len() {
                result.next.pending.push_back(Pending {
                    anchor: index,
                    deferred: false,
                    cursor,
                });
                return Ok(result);
            }
        }
    }
}
