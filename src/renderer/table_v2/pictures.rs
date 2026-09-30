//! Qualified stored picture/space rows. The row's saved physical box is
//! composed once; pagination and image paint consume the same owned snapshot.
//! Cell-local full-width exclusions use a separate positioned composition;
//! visible mixed text, side wrapping and edited picture hosts remain unsupported.
use super::{GeometryError, ParagraphItem, Rect};
use crate::{
    model::{
        bin_data::BinDataContent,
        control::Control,
        image::{ImageEffect, Picture},
        paragraph::{LineSeg, Paragraph},
        shape::{
            DropCapStyle, HorzAlign, HorzRelTo, SizeCriterion, TextWrap, VertAlign, VertRelTo,
        },
    },
    renderer::{
        layout::find_bin_data_bytes,
        render_tree::{
            BoundingBox, GroupNode, ImageNode, PlaceholderNode, RenderNode, RenderNodeType,
            TextLineNode,
        },
        style_resolver::ResolvedStyleSet,
    },
};

fn unsupported() -> GeometryError {
    GeometryError::Unsupported("V2 stored picture appearance or resource")
}

/// A zero-width saved host records a full-lane exclusion, not a TAC line.
/// The cell adapter has rejected multi-column stories before reaching here.
/// In that cell-local lane Column/Left and Para/Left coincide only with the
/// zero paragraph insets qualified below; this is not a document-column origin.
pub(super) fn excluded_cell_candidate(para: &Paragraph) -> bool {
    matches!(para.controls.as_slice(), [Control::Picture(p)] if !p.common.treat_as_char)
}

pub(super) fn compose_excluded_cell(
    para: &Paragraph,
    width: f64,
    styles: &ResolvedStyleSet,
    dpi: f64,
    resources: &[BinDataContent],
) -> Result<(Vec<ParagraphItem>, Vec<RenderNode>), GeometryError> {
    let fail = || GeometryError::Unsupported("stored cell picture exclusion");
    let [Control::Picture(pic)] = para.controls.as_slice() else {
        return Err(fail());
    };
    let [row] = para.line_segs.as_slice() else {
        return Err(fail());
    };
    let style = super::tac::carrier_style(para, styles)?;
    let a = &pic.common;
    if !para.text.is_empty()
        || !para.char_offsets.is_empty()
        || para.char_count != 9
        || para.stored_text_partition_is_dirty()
        || para.source_line_seg_vertical_pos.is_some()
        || para.layout_only_fill_lines != 0
        || para.hwpx_axis_shift != 0
        || !para.field_ranges.is_empty()
        || !para.orphan_field_ends.is_empty()
        || !para.range_tags.is_empty()
        || !para.title_marks.is_empty()
        || !para.markpen_marks.is_empty()
        || row.text_start != 0
        || row.vertical_pos < 0
        || row.segment_width != 0
        || row.column_start != 0
        || row.line_height <= 0
        || row.text_height != row.line_height
        || row.baseline_distance < 0
        || row.baseline_distance > row.line_height
        || row.line_spacing < 0
        || row.tag != LineSeg::TAG_SINGLE_SEGMENT_LINE
        || a.treat_as_char
        || a.text_wrap != TextWrap::TopAndBottom
        || a.vert_rel_to != VertRelTo::Para
        || a.vert_align != VertAlign::Top
        || !matches!(a.horz_rel_to, HorzRelTo::Para | HorzRelTo::Column)
        || a.horz_align != HorzAlign::Left
        || (a.horizontal_offset as i32) < 0
        || (a.vertical_offset as i32) < 0
        || !a.flow_with_text
        || a.allow_overlap
        || a.prevent_page_break != 0
        || [
            style.margin_left,
            style.margin_right,
            style.indent,
            style.spacing_before,
            style.spacing_after,
        ]
        .iter()
        .any(|v| *v != 0.0)
        || [a.margin.left, a.margin.right, a.margin.top, a.margin.bottom]
            .iter()
            .any(|v| *v < 0)
    {
        return Err(fail());
    }
    let scale = dpi / 7200.0;
    let x = (f64::from(a.horizontal_offset) + f64::from(a.margin.left)) * scale;
    let y = (f64::from(a.vertical_offset) + f64::from(a.margin.top)) * scale;
    let w = f64::from(a.width) * scale;
    let h = f64::from(a.height) * scale;
    if !width.is_finite() || width <= 0.0 || x + w + f64::from(a.margin.right) * scale > width {
        return Err(fail());
    }
    let host_height = f64::from(row.line_height) * scale;
    let host_advance = host_height + f64::from(row.line_spacing) * scale;
    let occupied = host_advance.max(y + h + f64::from(a.margin.bottom) * scale);
    // One atomic occupied envelope, with distinct host and image rectangles.
    // The host retains its own line height/baseline, never the image's height.
    // IR binding, cell alignment, fragment fitting and paint consume this box.
    let bounds = Rect {
        x: 0.0,
        y: 0.0,
        width,
        height: occupied,
    };
    let mut node = RenderNode::new(
        0,
        RenderNodeType::Group(GroupNode {
            section_index: None,
            para_index: None,
            control_index: None,
        }),
        BoundingBox::new(0.0, 0.0, width, occupied),
    );
    node.children.push(RenderNode::new(
        0,
        RenderNodeType::TextLine(TextLineNode::new(
            host_height,
            f64::from(row.baseline_distance) * scale,
        )),
        BoundingBox::new(0.0, 0.0, 0.0, host_height),
    ));
    node.children.push(RenderNode::new(
        0,
        payload(pic, resources)?,
        BoundingBox::new(x, y, w, h),
    ));
    let mut items = vec![ParagraphItem::ObjectRow {
        line: 0,
        bounds,
        controls: vec![0],
    }];
    items.push(ParagraphItem::End(super::ParagraphEnd::from_composed(
        &items,
        vec![],
        0.0,
    )?));
    Ok((items, vec![node]))
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
    let rows = super::tac::object_rows(
        &local,
        width / scale,
        style.alignment,
        style.vertical_alignment,
        true,
        &spaces,
    )?;
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
            let bounds =
                BoundingBox::new(r.x * scale, r.y * scale, r.width * scale, r.height * scale);
            let child = match &para.controls[ci] {
                Control::Picture(pic) => RenderNode::new(0, payload(pic, resources)?, bounds),
                Control::Shape(shape) => {
                    super::shapes::node(shape, bounds, styles, dpi, resources)?
                }
                _ => return Err(unsupported()),
            };
            line.children.push(child);
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
