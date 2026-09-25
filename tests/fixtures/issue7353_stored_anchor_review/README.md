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

The initial result above is preserved as a before observation. The follow-up
below establishes the top-margin rule independently; the tests now also check
the unrounded source-based continuation origin. Font differences remain.
Nested tables, within-cell cuts in the reference documents, original #6923
fidelity, and editor reflow are not covered by these PDF comparisons.

## Continuation margin controls

`variants/` contains authored HWPX, untouched Hancom saves, and PDFs printed
from those saves using the same service/version/settings above. Starting from
`anchor-review-input.hwpx`, only these properties were changed before Hancom
opened them (never manually change the saved LineSeg):

| Control | Change (HU) | PDF page2 border top at96dpi | HWP job / PDF job |
| --- | --- | --- | --- |
| top0 | common/mirror outer top0 |75.479167px|`95da1b1b-ef23-488b-9ba8-1a5863366d76` / `d7cb577f-a1e7-4e31-91c7-ad2daef53653`|
| top2mm | common/mirror outer top567 |82.994792px|`89f4276d-5f32-4cf0-93f1-9965e99c758b` / `90da853d-e05b-4332-be55-6f32a04536f2`|
| offset20mm | vertical offset5670; top283 unchanged |79.317708px|`63bf6bfd-143a-444c-95b3-ccdd9fa65f02` / `7fa3b173-2875-4b20-9077-822aa481bd93`|
| cellbreak | page_break CellBreak |79.317708px|`fdcd0028-51fb-4fb7-9805-0b9c4b160b02` / `a522d42b-1377-434b-9c52-e0475cd0e5d8`|
| defer | page_break None; offset43000; first3rows only, common height6978 |79.317708px|`1360bce6-1915-439b-9459-f73a0c6f68b8` / `2a6bff09-d021-4c69-afe6-7603ef1fa828`|

For `defer`, truncate cells/row_sizes to3, set row_count3 and clear the derived
cell_grid before serialization. All other source properties are unchanged.
Each reference is two pages. `pdftocairo -svg -f 2 -l 2` exposes the left
vertical border's start: y538.390625/532.753906/535.511719pt under the transform
`matrix(1,0,0,-1,0,595)`. Pixel top is `(595-y)*4/3`.

The page2 top changes with outer top margin, not the original vertical offset;
atomic deferral also keeps that margin. The engine contract therefore uses
`(5669 + outer_top)/75`px, not the PDF printer's rounding (difference <0.16px).
The changed fit path must reserve that margin before accepting a child and pass
the resulting origin directly to paint. This does not authorize TAC, side-wrap,
or nested-anchor rules. A separate synthetic WithinCells budget contract tests
transactional rejection when the child fits but child+margin does not.

`defer` PDF에서는 후속 문단이1쪽에 남고 표만2쪽으로 이월된다. 여백 절편 당시 V2는
직렬 본문 흐름 때문에 후속 문단을 표 뒤에 배치했다. 그 당시 결과는 이월 원점만
검증했으며 문서 전체 fidelity를 입증한 것이 아니다.

후속 본문 흐름 절편은 표의 위치 예약과 본문 소비를 분리한다. 수정하지 않은
`defer-saved.hwp`가 정식 회귀 입력이다.1쪽에는 제목과 후속 문단,2쪽에는01~03행이
각각 한 번만 있어야 하며 문단 중복이나1쪽의 표가 없어야 한다. 원점 기대값은
1쪽 글줄5669/75 및(5669+1100+660)/75px,2쪽 표(5669+283)/75px와 높이6978/75px다.
구현 결과를 복사한 값이 아니라 저장 메트릭과 독립 PDF의 페이지 소유에 근거한다.
기본24행 대조군은 여전히2쪽의 최종 조각 뒤에 후속 문단을 배치해야 한다.
별도의 합성 빈 줄·과대 표·복수 대기 표·본문 종료 계약은 커서 불변식 검사이며,
추가적인 한컴 fidelity 증거가 아니다. 새 Native/fresh WASM 비교와 메인테이너
시각 판정 상태는 아래 작업 기록에서 확인한다.

Evidence and maintainer judgment are tracked in
[`task_m100_7353_stage19.md`](../../../mydocs/working/task_m100_7353_stage19.md).
