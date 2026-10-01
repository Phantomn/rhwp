//! Bind explicit fragment ownership to product query metadata. Geometry is not
//! inspected or changed: row/cell/paragraph/control owners come from the plan.
use crate::renderer::{
    cell_context::{CellContext, CellPathEntry},
    render_tree::{RenderNode, RenderNodeType},
};

pub(super) fn bind(node: &mut RenderNode, section: usize, context: Option<CellContext>) {
    let mut context = context;
    match &mut node.node_type {
        RenderNodeType::Table(table) => {
            if let (Some(paragraph), Some(control)) = (table.para_index, table.control_index) {
                let mut owner = context.unwrap_or(CellContext {
                    in_textbox: false,
                    parent_para_index: paragraph,
                    path: Vec::new(),
                });
                if let Some(parent) = owner.path.last_mut() {
                    parent.cell_para_index = paragraph;
                }
                owner.path.push(CellPathEntry {
                    control_index: control,
                    cell_index: 0,
                    cell_para_index: 0,
                    text_direction: 0,
                });
                table.section_index = Some(section);
                context = Some(owner);
            }
        }
        RenderNodeType::TableCell(cell) => {
            if let (Some(owner), Some(index)) = (&mut context, cell.model_cell_index) {
                if let Some(entry) = owner.path.last_mut() {
                    entry.cell_index = index as usize;
                    entry.text_direction = cell.text_direction;
                }
            }
        }
        RenderNodeType::TextLine(line) => {
            if let (Some(owner), Some(paragraph)) = (&mut context, line.para_index) {
                if let Some(entry) = owner.path.last_mut() {
                    entry.cell_para_index = paragraph;
                }
                line.section_index = Some(section);
            }
        }
        RenderNodeType::TextRun(run) => {
            if let Some(owner) = &context {
                run.section_index = Some(section);
                run.para_index = owner.path.last().map(|entry| entry.cell_para_index);
                run.cell_context = Some(owner.clone());
            }
        }
        _ => {}
    }
    for child in &mut node.children {
        bind(child, section, context.clone());
    }
}
