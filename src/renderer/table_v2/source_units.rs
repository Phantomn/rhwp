//! Source-unit projection at the V2 document boundary. Does not mutate IR or
//! change Legacy resolution. PreparedTextTable callers supply resolved pixels.
use super::GeometryError;
use crate::{
    model::document::Document,
    renderer::style_resolver::{resolve_styles_for_document, ResolvedStyleSet},
};

pub(super) fn resolve(document: &Document, dpi: f64) -> Result<ResolvedStyleSet, GeometryError> {
    qualify(document, dpi, resolve_styles_for_document(document, dpi))
}

pub(super) fn qualify(
    document: &Document,
    dpi: f64,
    mut styles: ResolvedStyleSet,
) -> Result<ResolvedStyleSet, GeometryError> {
    for (source, resolved) in document
        .doc_info
        .para_shapes
        .iter()
        .zip(&mut styles.para_styles)
    {
        if source.indent & 1 == 0 {
            continue;
        }
        // Plain HWPX margin values do not prove URC encoding. Its parser
        // currently does not retain the value's unit, so do not infer CHAR
        // from an odd absolute distance on that path.
        if source.hwpx_plain_para_margin {
            return Err(GeometryError::Unsupported(
                "relative indentation requires encoded URC source",
            ));
        }
        // URC bit0 selects relative characters, not a fractional HWPUNIT.
        // Hancom ParameterSetObject: value = signed data >> 1, in 1/100 ch.
        // HWP help: ch uses the Normal style's Latin character size, not this
        // paragraph's first run. Qualify the unscaled half-em basis supported
        // by independent saved controls; other bases remain explicit.
        let base = document
            .doc_info
            .styles
            .first()
            .and_then(|s| document.doc_info.char_shapes.get(s.char_shape_id as usize))
            .ok_or(GeometryError::Unsupported(
                "relative indentation requires Normal style",
            ))?;
        if base.base_size <= 0
            || base.ratios[1] != 100
            || base.relative_sizes[1] != 100
            || base.spacings[1] != 0
            || base.superscript
            || base.subscript
        {
            return Err(GeometryError::Unsupported(
                "relative indentation scaled Latin basis",
            ));
        }
        let chars = f64::from(source.indent >> 1) / 100.0;
        let latin_half_em = f64::from(base.base_size) * dpi / 7200.0 / 2.0;
        resolved.indent = chars * latin_half_em;
    }
    Ok(styles)
}
