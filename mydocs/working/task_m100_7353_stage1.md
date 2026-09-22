# Task #7353 R1 — 표 조판 책임·의존 관계 조사

- Issue: #7353
- 기준 제품/테스트 SHA: `7a95e46e025470a4d7a7b59ad68ec02958bda738`
- 브랜치/worktree: `task_m100_7353`, `/home/edward/mygithub/rhwp-task-7353`
- 승인 범위: 수행계획의 R1 조사·설계. 조판 수정·원격 push·PR 변경 없음
- 판정: 정적 책임 조사와 정책 검사 완료. 동적 회귀·Native/WASM 시각 기준선은 미실행

## 1. 조사 결론

`table_layout.rs`만 나누면 해결되지 않는다. 셀 유닛·컷 계산은 LayoutEngine 메서드인데,
typeset이 임시 LayoutEngine을 만들어 호출하며 실제 배치와 편집/커서 경로도 이를 사용한다.
HeightMeasurer에는 별도의 표 높이 계산이 있다. 따라서 공통 계산의 소유 위치와
세션 상태·무효화 계약을 함께 분리해야 한다.

이 기록은 기준 devel의 정적 코드 조사다. #7195 통합 브랜치의 줄 번호·검증 결과를
이 기준으로 전용하지 않았으며, 모든 기존 조건의 정당성을 입증한 것도 아니다.

## 2. 기준 규모와 책임 지도

`wc -l` 결과이며 주석·기존 테스트 포함 물리 줄 수다. CC나 실행 비용은 아니다.

| 기준 경로 | 줄 수 | 주요 책임과 관측 위치 |
| --- | ---: | --- |
| `layout/table_layout.rs` | 19,856 | `layout_table:2602`, 열 폭 `3725`, 행 높이 `3838/4128`, 가로 셀 배치 `5525`, 유닛 생성 `10403`, 행 컷 `14128` |
| `layout/table_partial.rs` | 5,275 | 컷 창 `570`, 커서 probe `831`, 셀 조각 배치 `1038`, 부분 표 `3786/3893` |
| `layout/table_cell_content.rs` | 1,456 | 세로 셀 `62`, 셀 도형 `650`, 글상자 내부 표 `804` |
| `height_measurer.rs` | 5,380 | 본문/셀/표 사전 측정. 전부 표 코드인 것은 아님. `measure_table_impl:2068` |

위 경로의 접두사는 `src/renderer/`다. 다음은 실제 연결 지점이다.

| 책임 | 생산/호출 → 소비 | 목표 소유자 |
| --- | --- | --- |
| 줄·유닛 구성 | `table_layout::cell_units_uncached:10403` → `cell_units:10122` → 행/블록 컷 및 부분 셀 배치 | `renderer/table/content` |
| 컷과 조각 계약 | `RowCut/RowCutResult:1671/1675`, `NestedTableCut/Split:1892/1904` → typeset 및 layout | 공통 table 계약. vec 인덱스 의미 유지 |
| 기하 계산 | `resolve_column_widths:3725`, `resolve_row_heights_with_common_fit:4128`, `resolve_cell_padding:4802` | 공통 table 측정. 기존 HeightMeasurer 계산과 무조건 병합하지 않음 |
| 분할 요구 높이 | `advance_row_cut_inner:14128`, `row_cut_content_height:16862`, mixed reserve `17301` | 공통 table fragment 계산 |
| 페이지 결정 | `typeset/table/block/prepare:71` → `scan/runner/row_step:366` → continuation | 기존 `typeset/table` 유지, 계산 서비스 소비자로 변경 |
| 상태 확정 | `continuation/fragment/emit:285`의 PartialTable, `304`의 flow advance | 기존 TypesetState·continuation 커서 유지 |
| 실제 배치 | `layout.rs::layout_partial_table_item` → `table_partial:3786/3893` → `1038` | `layout/table/partial`, 셀 배치 모듈 |
| 편집 무효화 | `document_core/commands/text_editing.rs:1975/2309` → `invalidate_cell_units_after_text_edit:10059` | 기존 외부 진입점 보존, 공통 계산 캐시로 위임 |

## 3. 컷에서 실제 높이까지의 연결

대표 ordinary row 경로를 다음과 같이 대조했다.

1. `typeset/table/block/prepare.rs:71`: LayoutEngine 생성 후 profile과 render normalization을 설정한다.
2. `table_layout.rs:10403`: 문단/줄/자식 표를 CellUnit으로 구성한다. 소유 문단,
   가시 줄 범위, 저장 reset, nested cut 및 물리 예약 정보를 포함한다.
3. `table_layout.rs:17301`: budget으로 컷을 구하고 mixed nested reserve를 계산해 재시도한다.
4. `typeset/table/scan/runner/row_step.rs:366` 이후: source-frame tail 후보로 컷·예산을
   바꿀 수 있다. 공통 helper 호출만으로 최종 컷이 확정됐다고 볼 수 없다.
5. `continuation/fragment/emit.rs:69`: `consumed + header_overhead`로 partial_height 계산.
   `285`에서 컷·행 도메인·물리 높이 override를 PartialTable에 보존하고 `304`에서 흐름 전진.
6. `pagination.rs:589`: 시작 컷의 행/블록 공간, nested row domain,
   start/end row height override는 서로 다른 계약이다. bool 또는 height 하나로 합치지 않는다.
7. `table_partial.rs:4280/4320`: 컷에서 행 높이를 다시 구한다. `4375`의 시작 밴드,
   `4398` 부근의 걸친 셀 요구, `4434`의 마지막 행 override와 이후 frame 보정도 소비한다.
8. `table_partial.rs:1468/2021`: 줄 범위·중첩 조각을 복원해 실제 셀 내용을 배치한다.

rowspan은 `advance_row_block_cut:14594`, `advance_row_block_cut_with_row_offsets:14826`
등 별도 경로다. 위 ordinary row 조사만으로 모든 rowspan 경계를 검증했다고 하지 않는다.
전체 표 배치와 `layout_embedded_table`도 부분 표와 별개로 유지하며 이동 검증한다.

## 4. 상태와 의존 위험

- `layout.rs:3157`의 CellUnit 캐시는 포인터 키 → `Arc<Vec<CellUnit>>`다.
  `table_nested_text_flag_cache`, overflow 판정 캐시, profile, normalization도 관련된다.
- `table_layout.rs:10403` 이후의 저장 frame 판단은 `current_body_area`도 읽는다.
  따라서 DPI만 받는 순수 함수로 바꾸거나 profile/body 설정 시점을 바꾸면 동작 보존이 아니다.
- 캐시 조회는 `&self`여도 내부 상태를 바꾼다. CQRS의 Query와 수학적 순수 함수를 구분한다.
- `layout.rs:3289`의 전체 clear, `document_core/queries/rendering.rs:7290`의 호출,
  편집 후 선택적 무효화와 fingerprint `10032`를 함께 보존한다.
- Typeset 임시 계산 세션과 실제 paint 세션은 현재 별도다. 리팩토링을 이유로 같은 캐시 인스턴스로
  합치지 않는다. 기존 수명·Send/Arc 계약과 edit/reopen invalidation을 먼저 고정한다.
- 셀 유닛 생성이 자식 표의 분할 계산을 다시 부르는 재귀는 도메인 내부 재귀다.
  이를 무조건 없애지 않는다. 제거할 의존은 계산 계층 → RenderTree/페이지 가변 상태다.
- `table_partial`의 lazy `CellComposedStore:289`와 커서 probe는 성능·편집 계약이다.
  전체 eager compose로 바꾸어 코드를 단순화하지 않는다.

## 5. 기존 규칙·예외의 취급

저장 reset의 hard-break 승격/흡수, 선언 높이 신뢰·padding 축소, 투명 1×1 wrapper,
재귀 fragment projection, 부분 표 frame/clip 보정은 이동 시 조건·상수·순서를 보존한다.
원본/독립 출력과 대조하지 않은 조건은 일반 사양으로 재승인하지 않는다.

`table_layout_rules.md`는 일부 옛 호출명(`pagination/engine.rs` 등)을 포함한다.
의미 규칙의 참고와 현재 호출 위치를 구분하며 #7280의 `typesetting_architecture.md`와
실제 코드를 우선 대조했다. 현재 canonical 문서를 아직 수정하지 않았다.

## 6. 검증 대상 선정 및 실행 상태

| 축 | 기존 정식 검사/증거 후보 | R1 상태 |
| --- | --- | --- |
| wrapper·저장 frame | `issue_6923_wrapper_table_stored_page_frame`, `issue_6923_wrapper_fragment_page_frame` | source 확인, 실제 fixture/PDF 참조 확인. Rust 실행·시각 판독 미실행 |
| 행/블록 소유권 | `issue_6935_row_space_start_cut_straddle`, `issue_6803_split_start_row_rowspan_end_cut` | source/경로 확인, 실행 미실행 |
| TAC 줄 소속 | `issue_6601_inline_tac_tables_share_a_line`, `issue_7150_tac_line_owner_anchor` | source/경로 확인, 실행 미실행 |
| 어울림 | `issue_7158_square_wrap_continuation`, `issue_6128_wraparound_para_flow_advance`, `issue_5702_cell_square_nested_table_flow` | 실행 후보, 전체 assertion 감사 미완료 |
| 글상자·세로 셀 | `textbox_embedded_table_span_widths`, `issue_6029_vertical_cell_line_budget` | 실행 후보 |
| 캐시·편집 | 기존 source의 #2214/#4167 테스트, document_core text-edit/cursor 경로 | 이동 전 실행·정식 경계 검사 필요 |

실행한 명령:

```sh
node scripts/rust-unit-test-tiers.mjs --check --base-ref 7a95e46e025470a4d7a7b59ad68ec02958bda738
node scripts/run-rust-test.mjs --print issue_6923_wrapper_table_stored_page_frame
git diff --stat 7a95e46e025470a4d7a7b59ad68ec02958bda738 -- src tests Cargo.toml Cargo.lock
```

- unit-tier 정책 PASS: 4,205 tests / 298 modules / ready 0 / support 87 / white-box 4,114 /
  cfg support items 28. **테스트 본문 4,205건을 실행한 결과가 아니다.**
- test source는 현재 manifest의 `regression_suite_011`로 해석됨. `--print`는 실행하지 않는다.
- 제품·테스트·Cargo diff 없음. 기존 #7222 worktree의 시험 결과를 재사용하지 않았다.
- 전체 회귀, 빌드, Clippy, fresh WASM, Visual Sweep은 아직 실행하지 않았다.
  비용이 큰 전체 PR 검증은 관련 focused/시각 결과 공유 후 별도 승인 절차를 따른다.

## 7. 다음 단계 조건

구현계획에서 함수 이동뿐 아니라 cache/context와 테스트의 이동 방법을 확정한다.
구현 승인 후 **첫 제품 코드 이동 전에** 고정 기준의 focused 계약 및 대표 페이지 기준 출력을 확보한다.
R1의 정적 조사 완료를 실행 기준선 완료로 바꾸어 보고하지 않는다.
범위는 R2/R3의 논리적 묶음으로 유지하며 함수별 승인 절편을 늘리지 않는다.
