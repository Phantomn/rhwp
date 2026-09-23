# Task #7353 — 기존 회귀 계약과 V2 지원 경계 대조

- Issue: #7353
- 선행 구현 커밋: `e50d85da6` — [stage3](task_m100_7353_stage3.md)
- 범위: 기존 `tests/cases/`의 계약 분류·선택 실행, 정상 여백 계약의 V2 기하 검사 보강
- 상태: 선택 회귀 49건 통과. R3 실제 문서 종단 연결은 아직 미완료

## 기존 검사를 사용하는 원칙

기존 회귀를 새 합성 검사로 대체하지 않는다. 기존 검사는 Legacy 호출 경로에서 유지하고,
V2가 같은 기능을 지원하면 **동일 입력·독립 기대값·최종 출력 assertion**을 V2 세션에도
연결한다. 아직 문서 단위 V2 세션이 없으므로 아래 실물 검사 통과는 모두 Legacy 증거다.
미지원 기능을 제거한 파생 입력의 통과를 원본 사례 통과로 바꾸어 쓰지 않는다.

현재 `tests/cases/`의 Rust 파일은 837개다. 경로에 `table|nested|rowspan|cell|tac`가
들어간 후보는 232개이며, 이는 의미 기반 전수 분류나 테스트 함수 개수가 아니다.
[탐색 inventory](../../output/7353/r4/existing-case-inventory.json)에 경로·정적 test/ignore
속성 수·호출 표면 문자열 탐색 결과를 보존했다. 다음 기능을 연결할 때 후보를 추가로 읽으며,
이 목록에 없다는 이유로 표 관련 검증에서 제외하지 않는다.

## 우선 검증 묶음과 적용 경계

아래 파일은 모두 `tests/cases/` 기준이다. 기존 assertion·fixture·ignore는 변경하지 않았다.

| 기존 source | 독립 근거/검사 의미 | V2 연결 상태와 주의점 |
| --- | --- | --- |
| `issue_5301_nested_cell_uses_table_inner_margin.rs` | 표 기본/셀 고유 여백 선택; 실물 p66 문장 꼬리의 RenderTree 보존 | 정상 여백 선택을 V2 기하로 보강. 실물 글리프는 미연결 |
| `issue_6358_negative_cell_pad_clamp.rs` | 손상 음수 여백 복구 및 전축 0 표의 세로 여백 호환 helper | 일반 규칙으로 복제하지 않음. 적용된 음수 입력은 V2 명시적 오류 |
| `issue_7243_nested_fragment_padding.rs` | 정상 HWP/HWPX 저장 높이와 PDF; p26 조각 높이·후속 표 거리·p28 본문 경계 | V2 빈 밴드/여백 계약은 존재하나 실제 문서 구성·paint 미연결 |
| `issue_6935_row_space_start_cut_straddle.rs` | 합성 start/end 컷 분리와 실물 글자 총량·용지 경계 | rowspan 미지원. Legacy flag 표현 자체는 V2에 이식하지 않음 |
| `issue_6803_split_start_row_rowspan_end_cut.rs` | 실물 셀 문단 인덱스의 누락/중복 없는 분할·이어받기·용지 경계 | rowspan 미지원. 전체 소유 유닛 보존 assertion은 확장 시 필수 |
| `issue_6601_inline_tac_tables_share_a_line.rs` | 실물 TAC 표 둘의 줄·가로 순서; PDF 근거 좌표 범위 | TAC/병합 폭 미지원. 20px 상단 차 허용 검사를 정밀 시각 일치로 해석하지 않음 |
| `issue_7150_tac_line_owner_anchor.rs` | 실물 기준선/바깥여백·동반 표 하단 차·다른 줄 영향 | TAC 줄 구성/기준선 미지원. 이전 줄과 같은 줄을 구별할 계약 |
| `issue_5702_cell_square_nested_table_flow.rs` | 합성 HWPX 재로드 후 후속 글줄의 셀 내부 유지 | Square 미지원. 수동 LineSeg 입력이므로 정상 저장본 증거와 구분 |
| `issue_7158_square_wrap_continuation.rs` | 실물 저장 사다리/PDF와 줄 간격·이동 앵커·전체 줄 유지 | Square 가용 영역·이어받기 미지원. 전체 문서의 무넘침 검사는 아님 |
| `issue_7008_cell_tac_nested_side_by_side.rs` | 실물 같은 줄의 두 표·부모 높이·본문 첫 줄 비겹침 | TAC 미지원. 같은 줄이 확인된 경우의 점유이며 모든 TAC에 일반화 금지 |
| `issue_7066_cell_valign_nested_line_group.rs` | 3개 실물의 중첩 표 Center/Bottom 좌표 | 세로 정렬 미지원. 기존 1.4~3.7px 잔차 기록도 유지 |
| `textbox_embedded_table_span_widths.rs` | 합성 선언 폭·병합 제약으로 최종 셀 폭/개수 확인 | 글상자/병합 셀 미지원 |
| `issue_6735_textbox_table_cell_shape.rs` | 합성 글상자→표→도형의 내용·위치·같은 줄/다른 줄 배치 | 비표 컨트롤과 글상자 미지원 |

`issue_6923_wrapper_table_stored_page_frame.rs`의 10건은 동일 제품 소스에서 stage3에
통과한 증거를 재사용한다. #6923 전체의 한컴 일치나 V2 지원 완료를 의미하지 않는다.
제목 반복·세로쓰기·각주·편집/캐시 등은 이번 선택 실행에 포함하지 않았으며 여전히 미검증이다.

## 이번 검사 보강과 보류한 호환 처리

`issue_7353_table_v2_nested.rs`에 3건을 추가했다. 제품 Rust 소스는 변경하지 않았다.

1. `ir_zero_table_horizontal_margin_keeps_full_nested_cell_width`: #5301과 같은 여백
   관계인 합성 중첩 IR에서 비활성 셀 좌우 510HU 대신 표 좌우 0을 사용한다.
   실제 자식 배치 폭 36572, x=50, y=211, 자식 높이 292, 부모 예약 높이 302와 종료를 검사한다.
   기대 높이는 주입 구성기의 두 줄 10+10과 자식 여백 141+141의 합이다.
2. `ir_explicit_zero_cell_padding_does_not_fall_back_to_table`: 고유 여백 사용이 참일 때
   0을 결측으로 보지 않는다. 실제 줄 x/y/폭/높이와 예약 10, 완료를 검사한다.
3. `ir_selected_negative_padding_is_rejected_without_legacy_repair`: 선택된 음수는 오류,
   선택되지 않은 음수는 표 기본값 사용에 영향을 주지 않음을 구분한다.

값의 연결은 `from_ir_contents`의 여백 선택 → `inner_width`로 구성기 호출 →
`from_flow_rows`의 여백 공간 구성 → `TableCursor/FlowCursor` →
자식/줄 `placement`와 `reserved_height`다. 이 끝점은 아직 RenderTree/paint가 아니다.
검사는 기존 구현에서 충족하던 계약의 누락을 보강한 것이며 수정 전 FAIL/후 PASS의
결함 수정 증거로 보고하지 않는다.

초기 조사에서는 음수 fallback을 V2에도 적용할 후보로 보았으나, #6358은 깨진 저장값의
복구 계약이고 `Cell::effective_padding`에는 전축 0일 때의 수직 호환 처리도 함께 있다.
그 helper를 가져오면 선언 여백 계약 외의 동작까지 유입된다. 독립 근거를 재검토하기 전에는
V2의 명시적 거부를 유지한다. 정상 고유 여백 **0**과 손상 음수를 구별한다.

## 검증 증적

- 실행 worktree: `/home/edward/mygithub/rhwp-review-7353`
- 고정 target: `/home/edward/mygithub/rhwp/target/pr-review`
- source: `e50d85da6` + 이번 검사 3건, 제품 소스는 해당 커밋과 동일
- 실행: `node scripts/run-rust-test.mjs <위 source 이름> -- --cargo-profile release-test --target-dir /home/edward/mygithub/rhwp/target/pr-review --no-fail-fast`
- [선택 실행 로그](../../output/7353/r4/selected-tests.log), [manifest 준비](../../output/7353/r4/manifest.log)
- 결과: **49/49 PASS** — 기존 13개 source의 37건 + V2 source의 12건.
  필터로 제외된 나머지 검사는 실행하지 않았으며 전체 회귀 통과로 세지 않는다.
- [소스 SHA-256](../../output/7353/r4/source.sha256): V2 제품 6개 파일과 계약 2개 파일의
  source/review 일치를 검증했다. 이전 R2 기하 8건과 #6923 10건은 stage3 결과를 재사용하며
  이번 49건에 중복 합산하지 않는다.
- 포맷, 변경한 검사 target `regression_suite_011` Clippy(`-D warnings`),
  base `7a95e46e025470a4d7a7b59ad68ec02958bda738` 대비 manifest 검사 모두 PASS.
  [test-lint.log](../../output/7353/r4/test-lint.log)에 기록했다.
  `git diff --check` 및 이 보고서의 상대 링크 존재 검사도 PASS.

전체 nextest·PR lint 전체 묶음·Docker WASM·Visual Sweep을 실행한 단계가 아니다.
Native의 `HwpDocument` API 검사를 브라우저 WASM 실행으로 세지 않는다.

## 다음 구현에 적용

실제 문단 구성/paint 연결에는 기존 테스트의 실물 입력을 유지하고 engine만 명시적으로
선택하는 경로가 필요하다. TAC/어울림을 현재 순차 `FlowBlock::Table`로 강제 변환하지 않는다.
지원 기능을 연결할 때 위 source의 의미 기반 assertion을 재사용하고, Legacy 내부 flag를
직접 조작하는 검사만 새 소유자/조각 계약으로 번역한다. golden과 ignore는 별도 근거 없이 바꾸지 않는다.
