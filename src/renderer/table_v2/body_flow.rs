//! Story order and positioned-table reservation are different cursors. An
//! unplaced floating table must not consume its requested position as blank
//! story content or move the following prose to its eventual destination page.
use std::collections::VecDeque;

use super::{
    document::BodyPlan,
    flow::{FlowCursor, FlowFit},
    FlowCellInput, GeometryError, Insets, LinePlacement, NestedTablePlacement, Rect,
};

pub(super) struct AnchoredFlow {
    /// Story boundary after the composed host, without a fabricated spacer.
    pub boundary: usize,
    pub host_paragraph: usize,
    pub initial: FlowCellInput,
    /// Source offset is not repeated when the first fragment could not fit.
    pub deferred: FlowCellInput,
    /// Outer bottom margin belongs to every accepted physical fragment.
    pub bottom_margin: f64,
    /// Qualified saved-page compatibility clearance; never stretches a box.
    pub nonterminal_clearance: f64,
    pub side_wrap: Option<Insets>,
    pub host_end_offset: f64,
}

impl AnchoredFlow {
    fn fit(
        &self,
        cursor: &FlowCursor,
        flow: &FlowCellInput,
        area: Rect,
        page_height: f64,
    ) -> Result<FlowFit, GeometryError> {
        let query = |inset: f64| {
            cursor.fit_with_page_height(
                flow,
                Rect {
                    width: if self.side_wrap.is_some() {
                        flow.width
                    } else {
                        area.width
                    },
                    height: (area.height - inset).max(0.0),
                    ..area
                },
                Some((page_height - inset).max(0.0)),
            )
        };
        // Query without committing: a complete table needs its real margin,
        // not the compatibility clearance of an unfinished saved page frame.
        let mut inset = self.bottom_margin;
        let mut fit = query(inset)?;
        if fit.next.block != flow.blocks.len() && self.nonterminal_clearance > 0.0 {
            inset += self.nonterminal_clearance;
            fit = query(inset)?;
        }
        if !fit.tables.is_empty() {
            let budget = (area.height - inset).max(0.0);
            if inset > area.height {
                // Even an empty table cannot commit a margin outside the body.
                return Ok(FlowFit {
                    next: cursor.clone(),
                    height: 0.0,
                    advance: 0.0,
                    progressed: false,
                    required: fit.height + inset,
                    lines: Vec::new(),
                    tables: Vec::new(),
                    body_inline_overflow: false,
                });
            }
            fit.height = if fit.height == budget {
                area.height
            } else {
                fit.height + inset
            };
            fit.advance = if fit.advance == budget {
                area.height
            } else {
                fit.advance + inset
            };
        } else if fit.next.block != flow.blocks.len() {
            fit.required += inset;
        }
        Ok(fit)
    }
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
    page_break: usize,
    pending: VecDeque<Pending>,
}

pub(super) struct BodyFit {
    pub next: BodyCursor,
    pub height: f64,
    pub progressed: bool,
    pub required: f64,
    pub lines: Vec<LinePlacement>,
    pub tables: Vec<NestedTablePlacement>,
    pub body_inline_overflow: bool,
}

impl BodyFit {
    fn accept(&mut self, fit: FlowFit, pen: &mut f64, capacity: f64) -> FlowCursor {
        let remaining = capacity - *pen;
        // Reuse the caller's exact boundary when the child consumed its whole
        // budget. Re-adding the subtracted prefix can otherwise gain one bit.
        self.height = self.height.max(if fit.height == remaining {
            capacity
        } else {
            *pen + fit.height
        });
        *pen = if fit.advance == remaining {
            capacity
        } else {
            *pen + fit.advance
        };
        self.progressed |= fit.progressed;
        self.body_inline_overflow |= fit.body_inline_overflow;
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
            && self.page_break == plan.page_breaks.len()
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
            body_inline_overflow: false,
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
            let fit = anchor.fit(&pending.cursor, flow, remaining(pen), plan.body.height)?;
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
            pending.cursor = result.accept(fit, &mut pen, plan.body.height);
            if pending.cursor.block != flow.blocks.len() {
                result.next.pending.push_front(pending);
                return Ok(result);
            }
        }

        loop {
            let anchor_boundary = plan
                .anchors
                .get(result.next.anchor)
                .map_or(plan.flow.blocks.len(), |a| a.boundary);
            let next_break = plan.page_breaks.get(result.next.page_break).copied();
            let boundary =
                next_break.map_or(anchor_boundary, |(block, _)| block.min(anchor_boundary));
            let fit = result.next.story.fit_body_until(
                &plan.flow,
                remaining(pen),
                boundary,
                plan.body.height,
            )?;
            result.next.story = result.accept(fit, &mut pen, plan.body.height);
            if result.next.story.block != boundary {
                return Ok(result);
            }
            if next_break
                .is_some_and(|(block, anchors)| block == boundary && result.next.anchor == anchors)
            {
                // A deferred predecessor belongs before this boundary. Drain
                // its exact continuation first; do not let the next paragraph
                // escape ahead of it or restart a consumed table unit.
                if result.next.pending.is_empty() {
                    result.next.page_break += 1;
                    result.progressed = true;
                }
                return Ok(result);
            }
            let Some(anchor) = plan.anchors.get(result.next.anchor) else {
                return Ok(result);
            };
            // Use the accepted host geometry, shared with paint. Following-line
            // spacing may end at the page boundary, so subtracting an uncut
            // source tail from the story pen would move the table upwards.
            // The initial band is relative to the composed occupied end.
            // If a physical paragraph-after band carried past the host page,
            // the host is absent here and the ordinary deferred frame applies.
            let host_end = result
                .lines
                .iter()
                .filter(|line| line.owner.paragraph == anchor.host_paragraph)
                .map(|line| line.bounds.y + line.bounds.height - plan.body.y)
                .reduce(f64::max);
            let deferred = host_end.is_none();
            if anchor.side_wrap.is_some() {
                // A paragraph-local offset needs this page's complete host,
                // not just the last rows of a paragraph split across pages.
                let planned = plan
                    .lines
                    .keys()
                    .filter(|l| l.paragraph == anchor.host_paragraph)
                    .count();
                let placed = result
                    .lines
                    .iter()
                    .filter(|l| l.owner.paragraph == anchor.host_paragraph)
                    .count();
                if planned == 0 || placed != planned {
                    return Err(GeometryError::Unsupported(
                        "stored body side-wrap across pages",
                    ));
                }
            }
            let (flow, mut reservation_pen) = match host_end {
                Some(end) => (&anchor.initial, end + anchor.host_end_offset),
                None => (&anchor.deferred, pen),
            };
            let fit = anchor.fit(
                &FlowCursor::default(),
                flow,
                remaining(reservation_pen),
                plan.body.height,
            )?;
            if anchor.side_wrap.is_some() {
                // Only an intact, same-page stored exclusion is qualified.
                // Never defer the object alone while keeping its narrow saved
                // text lanes on the old page, or publish a partial wrap box.
                if deferred || fit.next.block != flow.blocks.len() || fit.tables.len() != 1 {
                    return Err(GeometryError::Unsupported(
                        "stored body side-wrap across pages",
                    ));
                }
                result.next.anchor += 1;
                result.accept(fit, &mut reservation_pen, plan.body.height);
                // Occupancy advances result.height but not the prose pen.
                // Following stored lines retain their actual left/right lanes.
                continue;
            }
            let index = result.next.anchor;
            result.next.anchor += 1;
            if fit.tables.is_empty() && fit.next.block != flow.blocks.len() {
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
            let cursor = result.accept(fit, &mut reservation_pen, plan.body.height);
            pen = pen.max(reservation_pen);
            if cursor.block != flow.blocks.len() {
                result.next.pending.push_back(Pending {
                    anchor: index,
                    deferred,
                    cursor,
                });
                return Ok(result);
            }
        }
    }
}

/// Validate the final coordinates consumed by paint, after every story/anchor
/// branch. Stored widths alone are not proof of a valid wrap after placement.
pub(super) fn validate_side_wraps(plan: &BodyPlan, fit: &BodyFit) -> Result<(), GeometryError> {
    let less = |a: f64, b: f64| a < b && !super::tac::same(a, b);
    let intersects = |a: Rect, b: Rect| {
        a.width > 0.0
            && a.height > 0.0
            && b.width > 0.0
            && b.height > 0.0
            && less(a.x, b.x + b.width)
            && less(b.x, a.x + a.width)
            && less(a.y, b.y + b.height)
            && less(b.y, a.y + a.height)
    };
    for anchor in &plan.anchors {
        let Some(margin) = anchor.side_wrap else {
            continue;
        };
        let Some(table) = fit
            .tables
            .iter()
            .find(|t| t.owner.paragraph == anchor.host_paragraph)
        else {
            continue;
        };
        let b = table.placement.bounds;
        let exclusion = Rect {
            x: b.x - margin.left,
            y: b.y - margin.top,
            width: b.width + margin.left + margin.right,
            height: b.height + margin.top + margin.bottom,
        };
        if fit
            .lines
            .iter()
            .any(|line| intersects(exclusion, line.bounds))
            || fit.tables.iter().any(|other| {
                other.owner != table.owner && intersects(exclusion, other.placement.bounds)
            })
        {
            return Err(GeometryError::Unsupported(
                "stored body side-wrap intersects flow",
            ));
        }
    }
    Ok(())
}
