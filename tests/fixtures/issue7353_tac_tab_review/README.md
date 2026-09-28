# Stored LEFT tab between two TAC tables

This is an isolated carrier, not a fidelity claim for the complete parent document.

## Provenance

Source: `samples/issue2470/36382471_masked.hwpx`, section 0, paragraph 0,
control 2 (parent table), first cell, first paragraph. Clone this paragraph into
the section body, retain the source DocInfo and section properties, and serialize
as `carrier.hwpx`. Its two TAC controls and stored 799 HU LEFT tab are retained.
Moving the cell carrier into the body changes the available frame; the raw
extraction is not a valid stored body-line oracle.

Hancom 2020 MCP opened that extraction and saved `carrier-saved.hwp`
(job `b5330cc0-cf8a-4ca8-9e9f-61009ebd198a`). The same saved HWP was converted
to `carrier-2020.pdf` with Hancom 2020
(job `000c3921-634f-4602-9707-68f8f640bcf8`). Tests and visual comparison use
this HWP/PDF pair, not the raw extraction. The original whole document is unchanged.

SHA256:

- Original: `43572dad5e17395aa02d1b0000b736b8467278931086604776ef30393dd0f54b`
- `carrier.hwpx`: `2f037957c2a8c79dc8c49615e1ad0ef2cf1ef447bdf696c74191c5eeaf7976fe`
- `carrier-saved.hwp`: `39091766efd4c8bc579c24d26b01f55d50750cb11ce20a10d7d37c02eb8e90d9`
- `carrier-2020.pdf`: `21a760aa703a3d0032ef5d652cb136eb5cd2686f1620a1ace3f7356536fe83e8`

## Independent expectations

At 96 DPI, 75 HU = 1 px. The saved tab's resolved advance is 799 HU, not
one ordinary space. Its source-stream extent is 8 units. Structural controls
occupy 16 source units, first table 8, tab 8, then the second table starts at 32.
Body left is 5385 HU; each table has 140 HU left/right outer margin; first
table width is 18921 HU. Table left edges are therefore 73.666667 and
340.333333 px. Inspect the PDF and review image for both outlines, their gap,
baseline alignment and all cells, not merely the page count.

`tests/cases/issue_7353_table_v2_document_flow.rs` protects source ownership,
table positions and cell preservation. Synthetic HWP/HWPX contracts additionally
cover body/cell carriers and left/center/right/final-line justify alignment;
these contracts are not independent Hancom visual examples.

## Limits

One qualified stored LEFT tab on one stored line is admitted. Missing/zero
advance, RIGHT tab and leader variants remain unsupported; this is not a new
fresh-layout tab-stop resolver. Whole-source preflight stops at an unrelated
body field end, and the original parent-prefix frame remains unsupported.
Neither is counted as full-document acceptance. Diagnostic inputs and failures
remain under `output/7353/r19/tac-tabs/`.
