# Task #7353 — R2 신규 경로의 기하·분할 계약

- Issue: #7353
- 승인 계획: [구현계획](../plans/task_m100_7353_impl.md)
- 계획 승인 커밋: `fd20e37b9`
- 제품 기준: `7a95e46e025470a4d7a7b59ad68ec02958bda738`
- 상태: R2 기하 계약 기반 구현·집중 검증 완료. 문서 선택/기준 출력 등 R2 잔여 및 R3 종단 구현은 남아 있다.

## 이번 구현의 경계

`src/renderer/table_v2/`에 독립 경로를 추가했다. Legacy 함수·분기·기준값은 변경하지 않았다.
현재 진입점은 **이미 줄 구성을 끝낸 셀의 기하**를 받는 실험 Rust API다. 일반 문서 렌더링에서
V2를 선택하는 API, CLI/WASM 옵션은 아직 연결하지 않았다. 일반 문서는 그대로 Legacy를 쓴다.
`renderer::table_v2`는 향후 내부 소비자와 외부 계약 테스트를 위한 doc-hidden 실험 경계이며,
DocumentCore·파서·직렬화의 공개 계약은 변경하지 않는다. 안정 API로의 승격은 별도 검토한다.

지원: 병합 없는 직사각형 행, 상단 정렬된 셀, 확정된 안 여백과 줄 상자, 최소 물리 높이,
행 간격, 행 경계 분할/분할 금지. 같은 조각 내부의 행 간격만 계상하고 다음 조각 첫 행에는
앞 조각의 간격을 재생하지 않는다. 이는 이번 합성 기하 계약이며 한컴 실측 확정 주장이 아니다.

미지원: IR에서 셀 콘텐츠 구성, TAC/자리차지/어울림의 정책 선택, rowspan/colspan,
셀 내부·중첩 표 재귀 분할, 제목 반복, 캡션·각주, 저장 정보 검증, 실제 RenderTree/paint,
문서 세션 선택·편집 무효화·Native/WASM 연결. 중첩 표를 평탄한 줄 목록으로 바꾸어
지원하는 것처럼 처리하지 않는다. `WithinCells` 요청은 명시적인 오류다.

## 규칙·소비 경로

| 계약과 독립 기대값 | 생산·소비 위치 | 이번 검증의 의미 |
| --- | --- | --- |
| 셀 물리 높이 = 위 여백 + 콘텐츠 점유 + 아래 여백, 최소 높이 이상 | `content.rs::TableContentPlan::new` → `row_heights` | 가시 텍스트가 없어도 점유를 버리지 않음 |
| 같은 행의 셀은 동일한 행 외곽 높이 사용 | `row_heights` → `fragment.rs::TableCursor::fit`의 셀 좌표 | 옆 셀/다음 행이 다른 높이를 재계산하지 않음 |
| 예산에 수용한 행과 내부 간격만 예약 | `fit` → `TableFragmentPlan.placement.bounds.height` | `reserved_height()`와 `placement()`가 동일 필드 소비 |
| 배치 원점 = 가용 영역 원점 + 선행 행/열 + 셀 여백 + 줄 좌표 | `fit`의 `CellPlacement`/`LinePlacement` | 최종 **기하 계획** 좌표 검사. backend 출력 증거는 아님 |
| 컷은 해당 불변 계획에 속하고 실패한 조회는 내용 미소비 | `TableCursor`의 비공개 `Arc<TableContentPlan>`/행 인덱스 | 다른 계획에 과거 컷을 끼워 넣을 API가 없음 |
| 합성 겹친 줄 상자는 명시된 점유 안에 공존 | `content.rs`의 범위 검사 → `fit`의 원점 이동 | 겹침을 저장 페이지 경계로 추측하지 않음 |

기대값은 합성 입력의 명시적 여백·상자·예산에서 계산한다. 예를 들어 콘텐츠 높이 32,
위/아래 여백 3/7이면 요구 높이는 42다. 예산 41에서는 미수용, 42에서는 수용하며,
페이지 원점 (10,20)·왼쪽 여백 5이면 콘텐츠 원점은 (15,23)이다.
빈 줄 4와 뒤 문단 시작 10은 별도 입력이다. 잉크 유무로 이 간격을 삭제하지 않는다.
이 합성 입력은 정상 한컴 저장 문서로 가장하지 않으며 폰트·줄간격 계산의 정확성을 입증하지 않는다.

현재 페이지 예산을 변경하는 명령이나 paint 소비자는 없다. 따라서 생산→실제 backend까지
공유가 완료됐다고 판정하지 않는다. R3에서 이 계획을 소비하는 문서 어댑터·배치를 연결하고
동일한 기하가 후단에서 덮어써지지 않는지 실제 출력으로 확인해야 한다.

## 검증

별도 검증 worktree: `/home/edward/mygithub/rhwp-review-7353`

고정 Cargo target: `/home/edward/mygithub/rhwp/target/pr-review` (기존 경로 그대로 사용).
파생 suite 준비는 검증 worktree에서만 실행했다. 원본 `tests/cases/`만 변경 대상으로 둔다.

| 실행 | 결과 |
| --- | --- |
| 수정 전 `issue_6923_wrapper_table_stored_page_frame` / release-test nextest | 10 passed, 필터 제외 211. 한컴 시각 정답 판정은 아님 |
| 신규 `issue_7353_table_v2_geometry` | 최종 8 passed, 필터 제외 204 |
| 수정 후 Legacy `issue_6923_wrapper_table_stored_page_frame` | 최종 10 passed, 필터 제외 208 |
| 검증 worktree `cargo fmt --all -- --check` | PASS |
| Native `cargo clippy --locked -p rhwp --lib --target-dir <고정 target> -- -D warnings` | PASS |
| `cargo check --locked -p rhwp --lib --target wasm32-unknown-unknown --target-dir <고정 target>` | PASS. WASM 배포 빌드/실행 판정은 아님 |
| manifest `--check --base-ref 7a95e46e025470a4d7a7b59ad68ec02958bda738` | PASS. 1401 sources / 6020 static attrs / 48 integration targets |

명령은 `node scripts/run-rust-test.mjs <source> -- --cargo-profile release-test
--target-dir /home/edward/mygithub/rhwp/target/pr-review`다.

최종 증적: [실행 로그](../../output/7353/r2/validation.log),
[제품·테스트 SHA-256](../../output/7353/r2/source.sha256).
검증 소스는 `fd20e37b9`에 위 파일 체크섬의 작업 변경을 적용한 스냅샷이다. 작업/검증
worktree의 해당 파일 내용이 동일함을 확인했다. 신규 검사 run ID는
`6a0d13b8-f759-4719-b622-364cfe468da1`, Legacy 검사는
`b08dd90a-2ad5-459d-98d4-6027b5aa9f0f`다.

작업 worktree에는 파생 suite가 없어 최초 `cargo fmt --all`에서 missing generated 파일을
보고했다. 검증 worktree에서 준비 후 포맷 검사를 통과했다. 테스트 원본의 포맷 변경은
배정 weight도 바꾸므로 manifest를 다시 준비해 drift를 해소했다. 생성기·정책·기준값은
수정하지 않았으며 파생 파일을 source 변경에 포함하지 않는다.

nextest 설치 버전 `0.9.137`은 권장 `0.9.140`보다 낮으며 CI 관측용
`junit.report-skipped` 키 경고가 있었다. 실제 선택 검사 18개는 실행·통과했다.
이 결과를 전체 nextest 실행이나 CI 동등 검증으로 보고하지 않는다.

신규 계약은 기존 경로 결함의 수정 전 FAIL/수정 후 PASS를 주장하지 않는다. 이전에는 신규
API 자체가 없었으므로 이전 소스에서 컴파일 실패하는 것을 결함 검출 증거로 세지 않는다.
전체 회귀·Native/fresh WASM Visual Sweep·실제 문서 개선은 아직 미실행이다.

## 다음 구현

IR·문단 구성에서 이 기하 계약으로 들어오는 어댑터와 실제 배치 연결을 마련하고,
그 위에서 자식 표의 소유·컷·물리 밴드를 보존하는 재귀 분할 계약으로 확장한다.
현재 입력을 억지로 사용해 중첩 표를 원자 행 또는 평탄한 줄로 위장하지 않는다.
문서 단위 V2 선택과 명시적인 Legacy 재실행은 실제 연결이 된 뒤 검증한다.
