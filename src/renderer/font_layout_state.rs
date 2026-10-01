//! Font registration and measurement snapshots shared by both engines.
//! No pagination, table layout, or render-tree caches are owned here.

#[derive(Clone, Default)]
pub(crate) struct FontLayoutState {
    exact_font_sources: crate::renderer::kerning::ExactFontSourceRegistry,
    horizontal_shaping_instance_requests:
        crate::renderer::shaping_context::HorizontalShapingInstanceRequestRegistry,
}

impl FontLayoutState {
    pub(crate) fn register_exact_font_source(
        &mut self,
        slot: crate::renderer::kerning::ExactFontSlot,
        bytes: &[u8],
        face_index: u32,
    ) -> Result<
        crate::renderer::kerning::ExactFontRegistryRegistration,
        crate::renderer::kerning::ExactFontRegistryError,
    > {
        self.exact_font_sources.register(
            slot,
            crate::renderer::kerning::ExactFontSource { bytes, face_index },
        )
    }

    pub(crate) fn clear_exact_font_sources(&mut self) -> bool {
        let sources_cleared = self.exact_font_sources.clear();
        let requests_cleared = self.horizontal_shaping_instance_requests.clear();
        sources_cleared || requests_cleared
    }

    /// Q3-D internal command owner. The public native/WASM surface remains
    /// unopened until the activation matrix is approved. Validation and
    /// canonicalization complete before this mutates the request snapshot.
    #[allow(dead_code)]
    pub(crate) fn set_horizontal_shaping_instance_request_dormant(
        &mut self,
        slot: crate::renderer::kerning::ExactFontSlot,
        variations: &[crate::renderer::shaping::ShapingVariation],
    ) -> Result<
        crate::renderer::shaping_context::HorizontalShapingInstanceRequestRegistration,
        crate::renderer::shaping_context::HorizontalShapingInstanceRequestError,
    > {
        self.horizontal_shaping_instance_requests.set_verified(
            &self.exact_font_sources,
            slot,
            variations,
        )
    }

    pub(crate) fn clear_horizontal_shaping_instance_request(
        &mut self,
        slot: crate::renderer::kerning::ExactFontSlot,
    ) -> bool {
        self.horizontal_shaping_instance_requests.remove(slot)
    }

    pub(crate) fn horizontal_shaping_instance_request(
        &self,
        slot: crate::renderer::kerning::ExactFontSlot,
    ) -> Option<&[crate::renderer::shaping::ShapingVariation]> {
        self.horizontal_shaping_instance_requests
            .request_slice_for_slot(slot)
    }

    pub(crate) fn horizontal_shaping_instance_request_counts(&self) -> (usize, u64) {
        (
            self.horizontal_shaping_instance_requests.request_count(),
            self.horizontal_shaping_instance_requests.generation(),
        )
    }

    pub(crate) fn exact_font_source_handle(
        &self,
        slot: crate::renderer::kerning::ExactFontSlot,
    ) -> Option<&crate::renderer::kerning::ExactFontSourceHandle> {
        self.exact_font_sources.handle_for_slot(slot)
    }

    pub(crate) fn exact_font_layout_session(
        &self,
    ) -> crate::renderer::kerning::KerningLayoutSession<'_> {
        crate::renderer::kerning::KerningLayoutSession::new(&self.exact_font_sources)
    }

    /// HeightMeasurer, TypesetEngine, page-tree LayoutEngine, edit reflow가 한
    /// transaction에서 같은 slot/source 결정을 읽도록 immutable snapshot을 만든다.
    /// Source payload는 Arc라 복제되지 않는다.
    pub(crate) fn exact_font_measurement_context_snapshots(
        &self,
    ) -> (
        Option<std::sync::Arc<crate::renderer::kerning::KerningMeasurementContext>>,
        Option<std::sync::Arc<crate::renderer::shaping_context::HorizontalShapingContext>>,
    ) {
        if self.exact_font_sources.slot_count() == 0 {
            return (None, None);
        }
        let registry = self.exact_font_sources.clone();
        (
            Some(std::sync::Arc::new(
                crate::renderer::kerning::KerningMeasurementContext::new(registry.clone()),
            )),
            Some(std::sync::Arc::new(
                crate::renderer::shaping_context::HorizontalShapingContext::with_instance_requests(
                    registry,
                    self.horizontal_shaping_instance_requests.clone(),
                ),
            )),
        )
    }

    /// Q4-D2 vertical table-cell activation snapshot. The registry clone keeps
    /// immutable font bytes in Arc storage and cannot observe later host
    /// registration changes during the page transaction.
    pub(crate) fn vertical_shaping_context_snapshot(
        &self,
    ) -> Option<crate::renderer::shaping_vertical::VerticalShapingContext> {
        if self.exact_font_sources.slot_count() == 0 {
            None
        } else {
            Some(
                crate::renderer::shaping_vertical::VerticalShapingContext::new(
                    self.exact_font_sources.clone(),
                ),
            )
        }
    }

    pub(crate) fn exact_font_source_registry_counts(&self) -> (usize, usize, usize, u64) {
        (
            self.exact_font_sources.slot_count(),
            self.exact_font_sources.source_count(),
            self.exact_font_sources.total_source_bytes(),
            self.exact_font_sources.generation(),
        )
    }

    pub(crate) fn exact_font_source_bytes_for_resource_key(
        &self,
        key: &str,
    ) -> Option<std::sync::Arc<[u8]>> {
        let (byte_len, digest) = crate::paint::parse_font_blob_resource_key(key)?;
        if byte_len > crate::paint::MAX_PORTABLE_FONT_BLOB_BYTES {
            return None;
        }
        self.exact_font_sources
            .source_arc_matching(byte_len, |bytes| {
                crate::paint::resource_digest_hex(bytes) == digest
            })
    }
}
