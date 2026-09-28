# RowBreak spanning-cell continuation

Scope: the first14 paragraphs of `samples/86712_regulatory_analysis.hwp`,
normally saved by Hancom; not acceptance of the complete original document.
Section0/paragraph13 is a31×17 RowBreak table. PDF page2 ends after source
row9; page3 starts at row10. The row8/column0 and column1 spanning titles occur
only on page2. These observations—not the prior V2 page count—define the cut.

## Source and independent reference

On2026-09-27, the existing `output/7353/r19/page-break/input.hwpx` (original
first14 paragraphs, no invented line metrics) was normally saved to HWP using
the Hancom conversion service, engine2020 /11.0.0.9136, then exported to PDF.
`prefix14-saved.hwp` is that saved document and `prefix14-2020.pdf` its3-page
reference. Save job: `552f9045-3f99-46aa-a97f-1acef381936f`;
PDF job: `f4216727-2879-4ced-86bb-3338e07b2627`.
Earlier provenance remains in stage19's page-break/cell-break records.

Two controlled variants establish the physical frame behavior independently
of V2. `generate.rs` changes only the indicated source properties and keeps
the SectionDef control mirror synchronized. Both HWPX variants were normally
saved by Hancom before PDF export; synthetic metadata is not used directly as
an oracle. HWP save can normalize additional fields, so the returned saved HWP
is the input compared with its own PDF.

| Variant | Source change | HWP save job | PDF job |
| --- | --- | --- | --- |
| margin | Paper bottom margin +600HU |981c22ec-38c7-41fa-8c29-4e710414a8b0|ea8834c7-0947-458e-bd21-32231c5f0d32|
| no-bottom | Main table bottom outer margin141HU→0 |40085191-de0c-4f6b-9f8c-82ec859dec1e|7b2e6383-d968-4c2e-8440-1edf41987991|

Original service job/status/download evidence and submitted HWPX files are in
`output/7353/r19/rowbreak-span/`. All three references have3pages and the same
row9→10 cut. `mutool draw -F trace` reports page2's bottom stroke at PDF y58.976,
64.969 and57.657 respectively, with the `1 0 0 -1 0 841` transform.
At96dpi those ends are1042.6987,1034.7080 and1044.4573px. Thus the independent
frame-margin delta is7.9907px and outer-margin delta1.7587px. Source-HU
invariants are8px and1.88px; PDF printer quantization is not a layout constant.

## Contract and limitations

`tests/cases/issue_7353_rowbreak_span_continuation.rs` checks actual final
Table/TableCell/TextLine geometry, row ownership, single title emission,
continued cells, physical frame reservation and end-of-document. Before this
change the normal source test fails because row9/column2 is missing from page2
(`output/7353/r19/rowbreak-span/test-before.log`). Margin variants protect the
additional physical-band rule, not the initial whole-group defect alone.

This slice keeps spanning-cell content atomic in its first fragment; it does
not claim cell-internal content splitting for rowspan. WithinCells rowspan
remains explicitly unsupported. Never and repeated-header atomicity remain.
PDF versus V2 still differs by about2.3px at page2's bottom edge and by fallback
font appearance; no coordinate clamp or reference-baseline relaxation hides it.
Maintainer visual judgment is required; matching3pages alone is insufficient.

Reproduction: compile `generate.rs` against the local rhwp library, pass a new
output directory, submit each HWPX to the documented Hancom MCP save operation
(`engine=2020,target=hwp`), then submit the returned HWP for PDF export. Do not
replace reference files with V2 output. See stage19 for exact native/WASM
commands, source hashes and the current review/standalone-overlay assets.
