//! Qualified stored picture/space rows. The row's saved physical box is
//! composed once; pagination and image paint consume the same owned snapshot.
//! This does not admit floating objects, visible mixed text or edited rows.
use super::{GeometryError, ParagraphItem, Rect};
use crate::{
    model::{
        bin_data::BinDataContent,
        control::Control,
        image::{ImageEffect, Picture},
        paragraph::Paragraph,
        shape::{DropCapStyle, SizeCriterion},
    },
    renderer::{
        layout::find_bin_data_bytes,
        render_tree::{
            BoundingBox, ImageNode, PlaceholderNode, RenderNode, RenderNodeType, TextLineNode,
        },
        style_resolver::ResolvedStyleSet,
    },
};

fn unsupported() -> GeometryError {
    GeometryError::Unsupported("V2 stored picture appearance or resource")
}

fn payload(pic: &Picture, resources: &[BinDataContent]) -> Result<RenderNodeType, GeometryError> {
    let a = &pic.shape_attr;
    // Empty HWPX binaryItemIDRef is preserved as ID0 (#1567). It owns an
    // object frame even though print output has no ink (#2225). Do not extend
    // this permission to unresolved nonzero resources or external images.
    let missing = pic.image_attr.bin_data_id == 0 && pic.image_attr.external_path.is_none();
    if pic.common.width == 0
        || pic.common.height == 0
        || pic.common.width_criterion != SizeCriterion::Absolute
        || pic.common.height_criterion != SizeCriterion::Absolute
        || pic.common.drop_cap_style != DropCapStyle::None
        || pic.caption.is_some()
        || pic.href.as_ref().is_some_and(|v| !v.is_empty())
        || pic.reverse
        || a.horz_flip
        || a.vert_flip
        || a.rotation_angle != 0
        || a.group_level != 0
        || a.render_b != 0.0
        || a.render_c != 0.0
        || a.render_tx != 0.0
        || a.render_ty != 0.0
        || !a.render_sx.is_finite()
        || !a.render_sy.is_finite()
        || a.render_sx <= 0.0
        || a.render_sy <= 0.0
        || pic.border_width != 0
        || pic.effects.shadow.is_some()
        || pic.image_attr.brightness != 0
        || pic.image_attr.contrast != 0
        || pic.image_attr.effect != ImageEffect::RealPic
        || pic.image_attr.transparency != 0
        || pic.image_attr.external_path.is_some()
        || [
            pic.padding.left,
            pic.padding.top,
            pic.padding.right,
            pic.padding.bottom,
        ]
        .iter()
        .any(|v| *v != 0)
    {
        return Err(unsupported());
    }
    // HWP 5.0 Table 83: offset_x/y are relative to an owning group, not
    // paragraph/page translation. With group_level == 0 the TAC line owns
    // the frame origin. Keep the stored fields intact, but do not apply them
    // or reject the picture because of them. Actual affine translation and
    // grouped geometry remain unsupported above; fit and paint both consume
    // the ObjectRow bounds composed below.
    if missing {
        // In this ungrouped TAC path, the row owns the frame's position.
        // Group-local source-image offsets have no pixels to transform. The
        // same row rect drives measurement and the MissingPicture backend.
        // This read-only preview does not claim editable DocumentCore IDs.
        return Ok(RenderNodeType::Placeholder(
            PlaceholderNode::missing_picture(None, None, None, None),
        ));
    }
    let data =
        find_bin_data_bytes(resources, pic.image_attr.bin_data_id).ok_or_else(unsupported)?;
    if !crate::renderer::image_resolver::is_displayable_image_data(&data) {
        return Err(unsupported());
    }
    let mut node = ImageNode::new(pic.image_attr.bin_data_id, Some(data));
    let c = &pic.crop;
    if [c.left, c.top, c.right, c.bottom].iter().any(|v| *v != 0) {
        if c.left < 0 || c.top < 0 || c.right <= c.left || c.bottom <= c.top {
            return Err(unsupported());
        }
        node.crop = Some((c.left, c.top, c.right, c.bottom));
        node.original_size_hu = pic.crop_reference_size();
    }
    // Positive source scaling is already represented by common.width/height;
    // applying it again would shrink/grow the image twice. Other transforms
    // are not admitted above. TAC ignores dormant floating-wrap settings.
    // No editable DocumentCore ownership is claimed by this preview snapshot.
    Ok(RenderNodeType::Image(node))
}

pub(super) fn compose(
    para: &Paragraph,
    width: f64,
    styles: &ResolvedStyleSet,
    dpi: f64,
    resources: &[BinDataContent],
) -> Result<(Vec<ParagraphItem>, Vec<RenderNode>), GeometryError> {
    let style = super::tac::carrier_style(para, styles)?;
    let scale = dpi / 7200.0;
    if style.vertical_alignment
        != crate::renderer::style_resolver::ParagraphVerticalAlignment::Baseline
    {
        return Err(GeometryError::Unsupported(
            "TAC paragraph vertical alignment",
        ));
    }
    let local = super::tac::physical_frame(para, width, style, dpi)?;
    let spaces = super::tac_spaces::compose(para, styles, dpi)?;
    let rows = super::tac::object_rows(&local, width / scale, style.alignment, true, &spaces)?;
    let mut items = vec![ParagraphItem::Space(style.spacing_before)];
    let mut nodes = Vec::new();
    let mut end = 0.0;
    for row in rows {
        // Lines owns physical height. Signed/overlapping row advances need the
        // general inline-content envelope path and are not quietly flattened.
        if row.top < end || row.spacing < 0.0 {
            return Err(GeometryError::Unsupported(
                "overlapping stored picture rows",
            ));
        }
        if row.top > end {
            items.push(ParagraphItem::Space((row.top - end) * scale));
        }
        let source = &local.line_segs[row.source_line];
        let bounds = Rect {
            x: f64::from(source.column_start) * scale,
            y: 0.0,
            width: f64::from(source.segment_width) * scale,
            height: row.height * scale,
        };
        let mut line = RenderNode::new(
            0,
            RenderNodeType::TextLine(TextLineNode::new(
                bounds.height,
                f64::from(source.baseline_distance) * scale,
            )),
            BoundingBox::new(bounds.x, 0.0, bounds.width, bounds.height),
        );
        let mut controls = Vec::new();
        for (si, r) in &row.spaces {
            let mut child = spaces[*si].node.clone();
            if child.bbox.height > bounds.height {
                return Err(unsupported());
            }
            child.bbox = BoundingBox::new(r.x * scale, 0.0, r.width * scale, bounds.height);
            if let RenderNodeType::TextRun(run) = &mut child.node_type {
                run.baseline = f64::from(source.baseline_distance) * scale;
            }
            line.children.push(child);
        }
        for (ci, r) in row.tables {
            let Control::Picture(pic) = &para.controls[ci] else {
                return Err(unsupported());
            };
            let payload = payload(pic, resources)?;
            line.children.push(RenderNode::new(
                0,
                payload,
                BoundingBox::new(r.x * scale, r.y * scale, r.width * scale, r.height * scale),
            ));
            controls.push(ci);
        }
        if controls.is_empty() {
            if row.spaces.is_empty() {
                items.push(ParagraphItem::Space(bounds.height));
            } else {
                items.push(ParagraphItem::Lines {
                    height: bounds.height,
                    advance: bounds.height,
                    lines: vec![(nodes.len(), bounds)],
                });
                nodes.push(line);
            }
        } else {
            items.push(ParagraphItem::ObjectRow {
                line: nodes.len(),
                bounds,
                controls,
            });
            nodes.push(line);
        }
        end = row.top + row.height;
    }
    let trailing = para.line_segs.last().ok_or_else(unsupported)?.line_spacing;
    let ending = super::ParagraphEnd::from_composed(
        &items,
        vec![f64::from(trailing) * scale + style.spacing_after],
        style.spacing_after,
    )?;
    items.push(ParagraphItem::End(ending));
    Ok((items, nodes))
}
