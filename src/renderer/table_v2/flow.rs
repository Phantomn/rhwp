//! Cell-local consumption. Physical spaces and recursive child cursors are distinct.
use std::sync::Arc;

use super::{
    FlowBlock, FlowCellInput, FragmentFit, GeometryError, LinePlacement, NestedTablePlacement,
    PageArea, Rect, TableCursor,
};

#[derive(Debug, Clone, Default)]
pub(super) struct FlowCursor {
    pub block: usize,
    space_left: Option<f64>,
    child: Option<Box<TableCursor>>,
}

pub(super) struct FlowFit {
    pub next: FlowCursor,
    pub height: f64,
    pub progressed: bool,
    pub required: f64,
    pub lines: Vec<LinePlacement>,
    pub tables: Vec<NestedTablePlacement>,
}

impl FlowCursor {
    pub fn fit(&self, cell: &FlowCellInput, area: Rect) -> Result<FlowFit, GeometryError> {
        let mut result = FlowFit {
            next: self.clone(),
            height: 0.0,
            progressed: false,
            required: 0.0,
            lines: Vec::new(),
            tables: Vec::new(),
        };
        while let Some(block) = cell.blocks.get(result.next.block) {
            let available = (area.height - result.height).max(0.0);
            match block {
                FlowBlock::Space(height) => {
                    let left = result.next.space_left.unwrap_or(*height);
                    let taken = if area.y + result.height + left <= area.y + area.height {
                        left
                    } else {
                        left.min(available)
                    };
                    result.height += taken;
                    if left > taken {
                        result.next.space_left = Some(left - taken);
                        result.progressed |= taken > 0.0;
                        result.required = left - taken;
                        break;
                    }
                    result.next.space_left = None;
                }
                FlowBlock::Lines { height, lines } => {
                    if area.y + result.height + height > area.y + area.height {
                        result.required = *height;
                        break;
                    }
                    result.lines.extend(lines.iter().map(|line| LinePlacement {
                        owner: line.owner,
                        bounds: Rect {
                            x: area.x + line.bounds.x,
                            y: area.y + result.height + line.bounds.y,
                            width: line.bounds.width,
                            height: line.bounds.height,
                        },
                    }));
                    result.height += height;
                }
                FlowBlock::Table { owner, plan } => {
                    let cursor = result
                        .next
                        .child
                        .as_deref()
                        .cloned()
                        .unwrap_or_else(|| TableCursor::new(Arc::clone(plan)));
                    match cursor.fit(PageArea {
                        bounds: Rect {
                            x: area.x,
                            y: area.y + result.height,
                            width: area.width,
                            height: available,
                        },
                    })? {
                        FragmentFit::Placed(fragment) => {
                            result.height += fragment.reserved_height();
                            result.tables.push(NestedTablePlacement {
                                owner: *owner,
                                placement: Box::new(fragment.placement().clone()),
                            });
                            result.progressed = true;
                            let next = fragment.continuation();
                            if !next.is_complete() {
                                result.next.child = Some(Box::new(next));
                                break;
                            }
                            result.next.child = None;
                        }
                        FragmentFit::DoesNotFit {
                            required_height, ..
                        } => {
                            result.required = required_height;
                            break;
                        }
                        FragmentFit::Complete => {
                            result.next.child = None;
                        }
                    }
                }
            }
            result.next.block += 1;
            // Zero padding is bookkeeping, not enough progress to emit a blank
            // fragment in front of a blocked line/table.
            result.progressed |= !matches!(block, FlowBlock::Space(h) if *h == 0.0);
        }
        Ok(result)
    }
}
