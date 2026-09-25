# Stored paragraph anchor: readable visual control

This is a **separate control document**, not an excerpt of issue #6923 and not
evidence that the original document passes. It replaces the illegible synthetic
`D / after` screenshot as a maintainer review example for issue #7353.

## Input and independent reference

1. `anchor-review-input.hwpx` was authored as a small synthetic document with no
   stored LineSeg. It has a title/anchor paragraph, one 24-row table, and a following
   paragraph. Rows are visibly numbered `자료 01` through `자료 24`.
2. Hancom opened that input and saved `anchor-review-saved.hwp`. The saved file,
   including its generated LineSeg and border-fill references, is not modified.
3. Hancom printed **that saved HWP** to `anchor-review-saved-2020.pdf`, two pages.
4. Native and fresh Docker-built WASM DocumentV2 both render the same saved HWP.

Generation used the documented hwp2024 MCP CLI with engine profile `2020`.
The service reported Hancom `11.0.0.9136`, `hwp-managed-direct-dll-host`, 32-bit
worker, no input preprocessing, one-up printing (`printMethod=0`), font scope
verified with two mapped/registered fonts and zero failed registrations.
HWP job: `f0087a6c-c9b0-41e3-bd50-b87e5cbe22fe`.
PDF job: `4124bc55-bef7-496b-a39c-d4782836223b`.

| File | SHA-256 |
| --- | --- |
| authored HWPX | `9e4639cad54f7ead81e81658a15f699559b315d8c8158e9a66c9d5058faa2287` |
| Hancom-saved HWP | `03d266c02306e04d02d040fe395864b2b28c2859695000e4ba4e2f46e0390805` |
| Hancom PDF | `d9bef0481b1531f3b14ad6042acf3db679504df1679be6c2b8dc1e1999ca803d` |

## Authored properties

All dimensions below are HWP units (1/7200 inch). A5 page: 41953 × 59528;
left/right margin 3969, top/bottom margin 5669. Table width 32000, 24 rows of
2326, cell padding 283 on each side. Text is 11 pt with 160% line spacing.
The table is non-TAC, TopAndBottom, paragraph-relative top/left with vertical
offset 2835, outer top margin 283 and bottom margin 567. RowBreak is used: this
fixture exercises **between-row continuation**, not within-cell or nested cuts.

## What can be judged

- Page 1: `표 시작 위치 확인` before the table, initial table origin and outer
  edges, rows 01–19, last row within the page body.
- Page 2: rows 20–24 with no repeated/missing row; bottom border; then
  `표 종료 후 본문입니다.` outside the table.
- Compare the same full pages against the independent PDF, including standalone
  overlays. Native/WASM equality alone is not a fidelity judgment.

## Known differences — not an approved fidelity baseline

At the first preparation, V2 page 2 starts its continuation at y=75.586667 px
(96 dpi), while the PDF vertical edge starts at y=79.317708 px. V2 is about
3.731041 px / 0.987 mm above the PDF; its following paragraph is also higher.
Font width and weight differ. No alignment transform is used to hide these
differences. The offset is close to the outer top margin, but its cause and
general continuation rule are not established by this one fixture.

The test `hancom_saved_anchor_review_preserves_rows_and_following_paragraph`
protects the independently observed row ownership and body containment only.
It deliberately does not approve the continuation origin. Nested tables,
within-cell splits, original #6923 fidelity, and editor reflow are not covered.

Evidence and maintainer judgment are tracked in
[`task_m100_7353_stage19.md`](../../../mydocs/working/task_m100_7353_stage19.md).
