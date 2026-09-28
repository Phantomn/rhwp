# Stored Shift+Enter rows — #7353

`prefix.hwpx` keeps the first 15 top-level paragraphs of
`samples/issue6601/36331407_side_by_side_tac_tables.hwpx`.
Run `python3 tests/fixtures/issue7353_stored_break_review/create.py` from the repository root.
Only trailing top-level paragraphs are removed. Text, styles, LineSeg records,
fields, tables and the cover are preserved. This is not a hand-authored LineSeg fixture.

Paragraphs 8 and 13 (zero-based) contain `hp:lineBreak` in the original XML.
The parsed LF is a structural boundary, not an unsupported printable character.
Paragraph 9 is a separate, intentional empty paragraph with a 500HU line box.

## Independent reference

`prefix-2020.pdf` was generated from this exact `prefix.hwpx` by the Hancom MCP
conversion service, not by rhwp: job `44d60f06-c26d-4653-bb9f-270afb4207bd`,
engine2020 / Hancom11.0.0.9136 / managed direct DLL / 32bit / preprocess none,
2026-09-28 KST, two pages.

- Input SHA256: `8cd2d2782a2cc839058c413bb45c276c4190879593e72a854d800e4dd2b0c918`
- PDF SHA256: `ee654f020e3b2e68661fa9548ac9ef6d1ed18c67f20720b4230374747c2e7088`
- Conversion receipts: `output/7353/r19/stored-break/pdf-{start,status,download}.json`.

## What to inspect

On page 2, the reference number following the regional self-sufficiency center
notice starts a new, indented row. The empty paragraph before `□ 점검 개요` occupies
its saved height. The facility list is followed by the smaller `※` note on its
own indented row, then by `자활근로사업단 : 10개소`. Check these rows, the two preceding
tables and the cover; do not infer correctness from the page count alone.

The formal case
`saved_body_forced_breaks_keep_indented_rows_blank_and_successor` checks actual
DocumentV2 output, source row membership, original HU-based geometry, break
markers, empty-line height and following content. The separate synthetic cell
test exercises leading/consecutive/trailing LF and continuation ownership; it
does not claim Hancom reference coverage for those synthetic combinations.

Native and fresh-WASM visual commands and results are recorded in
`mydocs/working/task_m100_7353_stage19.md`. Font-outline differences are separate
from this stored-partition contract. This excerpt does not certify the original
four-page document or all of V2.
