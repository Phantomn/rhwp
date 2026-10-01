//! Assemble a page from accepted V2 fragments. Placement, source ownership and
//! continuation cuts belong to those fragments; this layer does not reflow them.
use super::HostedTableError;
use crate::{
    model::paragraph::Paragraph,
    renderer::{
        pagination::{PageContent, PageItem},
        paint_resources::layout_rect_to_bbox,
        render_tree::{PageBackgroundNode, PageRenderTree, RenderNode, RenderNodeType},
        style_resolver::ResolvedStyleSet,
    },
};

pub(super) fn build(
    page: &PageContent,
    paragraphs: &[Paragraph],
    styles: &ResolvedStyleSet,
) -> Result<PageRenderTree, HostedTableError> {
    let layout = &page.layout;
    let mut tree = PageRenderTree::new(page.page_index, layout.page_width, layout.page_height);
    if let RenderNodeType::Page(root) = &mut tree.root.node_type {
        root.section_index = page.section_index;
    }
    // Section admission currently allows only an unpainted page BorderFill.
    // The paper background remains present independently of content/hide flags.
    let background = RenderNode::new(
        tree.next_id(),
        RenderNodeType::PageBackground(PageBackgroundNode {
            background_color: Some(0x00ff_ffff),
            border_color: None,
            border_width: 0.,
            gradient: None,
            image: None,
        }),
        tree.root.bbox,
    );
    tree.root.children.push(background);
    let mut body = RenderNode::new(
        tree.next_id(),
        RenderNodeType::Body { clip_rect: None },
        layout_rect_to_bbox(&layout.body_area),
    );
    let mut used_bottom = layout.body_area.y;
    for column in &page.column_contents {
        let column_layout = column.zone_layout.as_ref().unwrap_or(layout);
        let area = column_layout
            .column_areas
            .get(column.column_index as usize)
            .ok_or(HostedTableError::WrongDestination)?;
        let mut node = RenderNode::new(
            tree.next_id(),
            RenderNodeType::Column(column.column_index),
            layout_rect_to_bbox(area),
        );
        for item in &column.items {
            match item {
                PageItem::HostedParagraph {
                    fragment,
                    para_index,
                } => {
                    super::host_border::append(
                        fragment,
                        *para_index,
                        paragraphs,
                        styles,
                        layout.dpi,
                        tree.frame_mut(),
                        &mut node,
                    );
                    node.children
                        .extend(fragment.render_nodes(tree.frame_mut()));
                    used_bottom = used_bottom.max(fragment.next_y());
                }
                PageItem::HostedTable { fragment, .. } => {
                    let mut table = fragment.render_node(tree.frame_mut());
                    if let RenderNodeType::Table(t) = &mut table.node_type {
                        let selection = fragment.selection();
                        t.section_index = Some(selection.section);
                        t.para_index = Some(selection.paragraph);
                        t.control_index = Some(selection.control);
                    }
                    node.children.push(table);
                    let bounds = fragment.occupied();
                    used_bottom = used_bottom.max(bounds.y + bounds.height);
                }
                _ => {
                    return Err(HostedTableError::UnsupportedHost(
                        "V2 page contains an unprepared layout item",
                    ))
                }
            }
        }
        body.children.push(node);
    }
    crate::renderer::page_paint::append_column_separators(
        tree.frame_mut(),
        &mut body,
        layout,
        layout.body_area.y,
        used_bottom,
    );
    crate::renderer::page_paint::set_body_clip(&mut body, layout.page_height);
    tree.root.children.push(body);
    tree.apply_legacy_hancom_product_display_projection();
    crate::renderer::page_paint::lift_cell_anchored_objects_above_text(&mut tree.root);
    Ok(tree)
}
