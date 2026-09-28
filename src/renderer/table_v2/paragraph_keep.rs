//! Paragraph protection groups the composer's real rows, not estimated heights.
use super::{GeometryError, ParagraphItem};

/// Text-only paragraphs use one acceptance transaction when keep_lines is set.
/// Internal gaps and leading paragraph space stay with their owned rows; the
/// paragraph ending remains outside, preserving the caller's cell-end policy.
pub(super) fn group(items: Vec<ParagraphItem>) -> Result<Vec<ParagraphItem>, GeometryError> {
    let mut pen: f64 = 0.0;
    let mut occupied: f64 = 0.0;
    let mut rows = Vec::new();
    for item in items {
        match item {
            ParagraphItem::Space(h) => {
                pen += h;
                occupied = occupied.max(pen);
            }
            ParagraphItem::Lines {
                height,
                advance,
                lines,
            } => {
                occupied = occupied.max(pen + height);
                rows.extend(lines.into_iter().map(|(owner, mut bounds)| {
                    bounds.y += pen;
                    (owner, bounds)
                }));
                pen += advance;
            }
            _ => return Err(GeometryError::Unsupported("non-text kept paragraph")),
        }
    }
    if rows.is_empty() {
        return Err(GeometryError::Unsupported("kept paragraph without rows"));
    }
    Ok(vec![ParagraphItem::Lines {
        height: occupied,
        advance: pen,
        lines: rows,
    }])
}
