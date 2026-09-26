# CENTER reference versus glyph baseline

Original: `../issue6923/148738070_wrapper_table_stored_page_frame.hwp`, physical
PDF p6, paragraphs21–25. The market-share table has13 rows/8 columns including
merged headers, followed by the source note. Its text paragraphs set
`ParaShape.attr1[20..21]=2` (HWPX `align@vertical="CENTER"`). This is neither
horizontal justification nor table-cell vertical alignment.

The original, an unchanged Hancom HWP re-save, and a HWPX reflow with all stored
rows removed independently produce height1200HU/reference600HU in the first
cell. BASELINE alignment instead produces1020HU. Hancom PDF places the uniform
12pt header words at the same positions in both controls. Consequently600HU is
an alignment reference, not a glyph ascent that should be clamped or painted
literally. Shared normal em metrics give1020HU for the glyph baseline.

`create.rs` preserves the table's content, dimensions, merged cells, title,
units, blank paragraph and following note. **It changes all table/cell borders
to uniform solid edges and clears zones**, isolating text alignment from the
original's unsupported conflicting adjacent edges. It clears all LineSeg and
creates CENTER/BASELINE variants, differing only in cell-paragraph vertical
alignment. These are explicit controls, not original-document fidelity passes.

Conversion: async HWP2024 MCP client, explicit engine2020, Hancom11.0.0.9136,
32-bit direct worker, preprocessing none, 2026-09-26. HWPX→normal HWP save→PDF
from that same HWP. Jobs:

- CENTER HWP: `81349c0d-fa51-4097-9614-a2cbfe2835da`
- CENTER PDF: `6a2a564c-2ea7-4bd0-bc8e-02697cc2c134`
- BASELINE HWP: `b683844b-c3f9-49b1-9af3-d730cd81d3cb`
- BASELINE PDF: `fce70f33-58cd-4e39-a889-65aa7c7f6aa9`

Evidence: `output/7353/r19/stored-paint/*-status.json`, PDF bbox exports,
Native/fresh WASM compare/overlay/review. PDF text bboxes coincide except
`하이트`, whose relative-size styling changes its yMin by0.359607pt. The
existing shared font projection does not reproduce that relative-size detail;
it remains a limitation, not a tested claim of exact glyph equivalence.
Fallback glyph width/weight differences also remain.

The formal document-flow test checks final line boxes, glyph baseline, complete
text, table/cell boxes and the following note against the independent alignment
invariant. Text tests distinguish a CENTER reference from a malformed BASELINE
value, preserve the input IR, and cover fresh uniform-em text. Mixed resolved
em sizes/scripts and TOP/BOTTOM remain explicit unsupported V2 paths. No
Legacy behavior, default-engine selection or page-count baseline is changed.
