//! Page stories do not own body space or advance the table/paragraph cursor.
//! Paragraph-entry decimal footer declarations; document flow activates each
//! on the first page that accepts its host, not on preceding pages. A position
//! change (including disabled) does not restart the section's number sequence.
use crate::{
    model::{
        control::{Control, PageNumberPos},
        document::SectionDef,
        paragraph::Paragraph,
        table::Table,
    },
    renderer::{
        layout::{estimate_text_width, format_page_number},
        page_layout::PageLayoutInfo,
        render_tree::{
            BoundingBox, FieldMarkerType, RenderNode, RenderNodeType, TextLineNode, TextRunNode,
        },
        TextStyle,
    },
};

use super::{ControlOwner, GeometryError, LinePlacement, NestedTablePlacement};

/// Qualify the unchanged UTF-16 control slot, shared by standalone and hosted
/// body composition. A declaration after visible content is not paragraph entry.
pub(super) fn validate_body_entry(para: &Paragraph, control: usize) -> Result<(), GeometryError> {
    if para.controls[..control]
        .iter()
        .any(|c| !matches!(c, Control::SectionDef(_) | Control::ColumnDef(_)))
        || para.control_utf16_positions().get(control).copied() != Some((control * 8) as u32)
    {
        return Err(GeometryError::Unsupported(
            "page-number declaration within paragraph content",
        ));
    }
    Ok(())
}

/// Source ownership, not coordinates: a declaration in a later cell fragment
/// must not be activated merely because its enclosing table has started.
pub(super) struct PageNumberHost {
    parents: Vec<CellHost>,
    paragraph: usize,
    line: Option<usize>,
}

#[derive(Clone)]
struct CellHost {
    table: ControlOwner,
    row: usize,
    column: usize,
}

impl PageNumberHost {
    pub fn body(paragraph: usize) -> Self {
        Self {
            parents: Vec::new(),
            paragraph,
            line: None,
        }
    }

    pub fn accepted(&self, lines: &[LinePlacement], tables: &[NestedTablePlacement]) -> bool {
        let mut lines = lines;
        let mut tables = tables;
        for parent in &self.parents {
            let Some(table) = tables.iter().find(|t| t.owner == parent.table) else {
                return false;
            };
            let Some(cell) = table
                .placement
                .cells
                .iter()
                .find(|c| c.row == parent.row && c.column == parent.column)
            else {
                return false;
            };
            lines = &cell.lines;
            tables = &cell.tables;
        }
        lines.iter().any(|l| {
            l.owner.paragraph == self.paragraph && self.line.is_none_or(|line| l.owner.line == line)
        }) || (self.line.is_none() && tables.iter().any(|t| t.owner.paragraph == self.paragraph))
    }
}

/// A standalone page-story declaration is not an inline cell occupant. Keep
/// its UTF-16 source slot and saved lines when projecting it to text composition.
/// Other control combinations need their own ordered composition support.
pub(super) fn cell_declaration(para: &Paragraph) -> Result<bool, GeometryError> {
    if !para
        .controls
        .iter()
        .any(|c| matches!(c, Control::PageNumberPos(_)))
    {
        return Ok(false);
    }
    if !matches!(para.controls.as_slice(), [Control::PageNumberPos(_)]) {
        return Err(GeometryError::Unsupported(
            "cell page-number declaration within paragraph content",
        ));
    }
    cell_line(para)?;
    Ok(true)
}

fn cell_line(para: &Paragraph) -> Result<usize, GeometryError> {
    let position = para
        .control_utf16_positions()
        .first()
        .copied()
        .ok_or(GeometryError::InconsistentAtomicPlan)?;
    if para.line_segs.is_empty() {
        return if position == 0 {
            Ok(0)
        } else {
            Err(GeometryError::Unsupported(
                "fresh cell page-number within paragraph content",
            ))
        };
    }
    if para.stored_text_partition_is_dirty() {
        return Err(GeometryError::Unsupported(
            "dirty cell page-number line ownership",
        ));
    }
    // The normal #7158 save puts pgnp after the title text, still in its first
    // saved line. A later saved line must wait for that exact accepted line.
    // Text composition independently validates this unchanged line partition.
    (0..para.line_segs.len())
        .rfind(|&i| para.line_seg_text_start(i) <= position)
        .ok_or(GeometryError::Unsupported(
            "cell page-number has no saved host line",
        ))
}

pub(super) fn collect_cell_stories(
    table: &Table,
    owner: ControlOwner,
    section: &SectionDef,
    layout: &PageLayoutInfo,
    stories: &mut Vec<(PageNumberHost, PageNumberStory)>,
) -> Result<(), GeometryError> {
    fn visit(
        table: &Table,
        owner: ControlOwner,
        parents: &mut Vec<CellHost>,
        section: &SectionDef,
        layout: &PageLayoutInfo,
        stories: &mut Vec<(PageNumberHost, PageNumberStory)>,
    ) -> Result<(), GeometryError> {
        if parents.len() >= 64 {
            return Err(GeometryError::Unsupported("table nesting resource limit"));
        }
        for cell in &table.cells {
            parents.push(CellHost {
                table: owner,
                row: cell.row.into(),
                column: cell.col.into(),
            });
            for (pi, para) in cell.paragraphs.iter().enumerate() {
                cell_declaration(para)?;
                for (ci, control) in para.controls.iter().enumerate() {
                    match control {
                        Control::PageNumberPos(value) => stories.push((
                            PageNumberHost {
                                parents: parents.clone(),
                                paragraph: pi,
                                line: Some(cell_line(para)?),
                            },
                            PageNumberStory::new(value, section, layout)?,
                        )),
                        Control::Table(child) => visit(
                            child,
                            ControlOwner {
                                paragraph: pi,
                                control: ci,
                            },
                            parents,
                            section,
                            layout,
                            stories,
                        )?,
                        _ => {}
                    }
                }
            }
            parents.pop();
        }
        Ok(())
    }
    visit(table, owner, &mut Vec::new(), section, layout, stories)
}

pub(super) struct PageNumberStory {
    declaration: PageNumberPos,
    layout: PageLayoutInfo,
    first: u32,
    line_bottom: f64,
}

impl PageNumberStory {
    pub fn new(
        declaration: &PageNumberPos,
        section: &SectionDef,
        layout: &PageLayoutInfo,
    ) -> Result<Self, GeometryError> {
        if declaration.format != 0
            || !matches!(declaration.position, 0 | 4..=6)
            || section.page_num_type != 0
            || [
                declaration.prefix_char,
                declaration.suffix_char,
                declaration.dash_char,
            ]
            .iter()
            .any(|c| *c != '\0' && c.is_control())
        {
            return Err(GeometryError::Unsupported("page-number format or position"));
        }
        // Default automatic footer, independently varied in the Hancom margin
        // matrix (tests/fixtures/issue7353/footer-position). A footer allocation
        // places the number just above the bottom paper margin; with no footer,
        // its line ends halfway through that margin. Test the source value, not
        // a floating-point subtraction of layout coordinates for zero.
        let bottom_margin = f64::from(section.page_def.margin_bottom) * layout.dpi / 7200.0;
        let line_bottom = layout.page_height
            - bottom_margin
                * if section.page_def.margin_footer == 0 {
                    0.5
                } else {
                    1.0
                };
        Ok(Self {
            declaration: declaration.clone(),
            layout: layout.clone(),
            first: u32::from(section.page_num).max(1),
            line_bottom,
        })
    }

    pub fn render(&self, page_index: u32) -> Result<Option<RenderNode>, GeometryError> {
        if self.declaration.position == 0 {
            return Ok(None);
        }
        let number = self
            .first
            .checked_add(page_index)
            .filter(|n| *n <= u32::from(u16::MAX))
            .ok_or(GeometryError::Unsupported("page-number range"))?;
        self.render_number(number)
    }

    /// The product document host already resolved the section start/continuation.
    /// Do not add the source section's first number a second time.
    pub(super) fn render_number(&self, number: u32) -> Result<Option<RenderNode>, GeometryError> {
        if self.declaration.position == 0 {
            return Ok(None);
        }
        if number == 0 || number > u32::from(u16::MAX) {
            return Err(GeometryError::Unsupported("page-number range"));
        }
        let p = &self.declaration;
        let text = format_page_number(number, p.format, p.prefix_char, p.suffix_char, p.dash_char);
        // 10pt is the observed automatic page-number size, not the host's style.
        // Use the same style for measuring and painting; no character-count width.
        let size = 10.0 * self.layout.dpi / 72.0;
        let style = TextStyle {
            font_family: "바탕".into(),
            font_size: size,
            color: 0,
            ..Default::default()
        };
        let width = estimate_text_width(&text, &style);
        let area = self.layout.footer_area;
        let free = area.width - width;
        if !free.is_finite() || free < 0.0 {
            return Err(GeometryError::Unsupported("page-number width"));
        }
        let x = area.x
            + match p.position {
                4 => 0.0,
                5 => free / 2.0,
                6 => free,
                _ => unreachable!(),
            };
        // The default10pt HWP line has an850HU baseline in a1000HU line.
        // PDF text origins, rather than glyph ink bounds, qualify this metric.
        // Story placement and painting consume the same top/baseline. No body
        // fit mutation or clamping, even if the author's footer is very small.
        let top = self.line_bottom - size;
        let height = size;
        let baseline = size * 0.85;
        if !top.is_finite() || top < 0.0 || top + height > self.layout.page_height {
            return Err(GeometryError::Unsupported("page-number outside page"));
        }
        let mut line = RenderNode::new(
            0,
            RenderNodeType::TextLine(TextLineNode::new(height, baseline)),
            BoundingBox::new(x, top, width, height),
        );
        line.children.push(RenderNode::new(
            0,
            RenderNodeType::TextRun(TextRunNode {
                text,
                style,
                char_shape_id: None,
                para_shape_id: None,
                section_index: None,
                para_index: None,
                char_start: None,
                cell_context: None,
                is_para_end: true,
                is_line_break_end: false,
                rotation: 0.0,
                is_vertical: false,
                char_overlap: None,
                border_fill_id: 0,
                baseline,
                field_marker: FieldMarkerType::None,
                layout_positions: None,
                display_text: None,
            }),
            BoundingBox::new(x, top, width, size),
        ));
        Ok(Some(line))
    }
}
