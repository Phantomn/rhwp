# Task #7353 — 문단 소유 연결과 재귀 이어받기

- Issue: #7353
- 선행 커밋: `7360ba58c` — [R2 계약 기록](task_m100_7353_stage2.md)
- 범위: 승인된 R3의 IR 소유 연결·재귀 기하 분할. 실제 문서 종단 렌더링 완료가 아니다.
- 상태: IR 소유 연결·재귀 기하 구현 및 집중 검증 완료. R3 종단 연결은 진행 중.

## 구현과 지원 경계

기존 `table_v2`의 행 단위 커서를 셀별 콘텐츠 커서와 자식 표 커서로 확장했다.
R2의 이미 구성된 셀 입력도 같은 경로에 들어간다. 기존 표 조판 알고리즘으로 fallback하거나,
자식 표를 부모의 평탄한 줄 목록으로 바꾸지 않는다. Legacy 소스와 기본 실행 경로는 변경하지 않았다.

| 책임 | 구현 |
| --- | --- |
| 입력·기하·소유자 계약 | `table_v2/contracts.rs` — `FlowBlock`, `ControlOwner`, 재귀 `NestedTablePlacement` |
| 불변 콘텐츠·전체 물리 높이 | `table_v2/content.rs` — `from_flow_rows`, 부모/자식 폭 검증 |
| 셀별 내용 소비 | `table_v2/flow.rs` — 줄 그룹·공간·자식 표의 서로 다른 이어받기 상태 |
| 행/표 조각·공통 예약 기하 | `table_v2/fragment.rs` — 각 셀 소비 높이로 공통 행 외곽 확정 |
| 공통 Document IR의 문단·컨트롤 소유 | `table_v2/ir.rs` — `from_ir_contents`, `CellParagraphComposer` |

문단 구성기가 반환한 `TableControl(ci)`는 해당 `Paragraph.controls[ci]`와 결합된다.
그 자리에서 자식 표 계획을 재귀로 만들고 원본 문단/컨트롤 인덱스를 보존한다. 슬롯 누락·중복,
다른 종류의 컨트롤, 병합 셀, 지원하지 않는 앵커는 오류로 반환한다.
IR 연결은 셀 개별 안 여백 적용 여부에 따라 표 기본 여백 또는 셀 여백을 선택한다.
음수 여백이나 다른 폭에서 만든 구성을 조용히 보정·축소하지 않는다.

현재 IR 경계는 병합 없는 가로/상단 정렬 셀, 절대 크기, 셀 간격 0,
영점 오프셋·문단 기준·왼쪽/위 정렬인 비-TAC TopAndBottom 자식 표에 한정한다.
제목 반복·캡션·상대 크기·명시적 쪽/단 나눔·TAC·Square·바깥여백 등은 거부한다.
최외곽 표의 앵커는 이 **표 내부 구성 API**가 처리하지 않는다. 문서 흐름 어댑터에서 연결해야 한다.
`cell.height`를 최소 물리 높이로 받는 현재 IR 계약도 실제 문서 출력 대조 전의 실험 경계다.

`CellParagraphComposer`는 문단과 실제 가용 폭을 받는 주입 인터페이스다. 이번에는 합성 검사
구성기로 연결을 검증했으며, 실제 폰트/서식·저장 LineSeg 검증·편집 재조판 구성기는 아직 없다.
따라서 임의 LineSeg를 유효 저장본으로 승인하거나 HWP/HWPX 샘플의 시각 개선을 주장하지 않는다.

## 분할과 물리 공간 계약

- `Lines`는 구성기가 확정한 원자적 줄 그룹이다. 좌표 겹침으로 새 페이지 컷을 추측하지 않는다.
- `Space`는 소비한 높이와 남은 높이를 갖는다. 위/아래 안 여백도 같은 물리 공간으로 한 번 계상한다.
- `Table`은 자식 계획과 `TableCursor`를 유지한다. 다음 페이지에서 자식의 원래 행 0부터 다시 시작하지 않는다.
- 각 셀은 독립적으로 전진하며, 이번 조각에서 실제 수용한 높이들의 최댓값으로 행 외곽을 정한다.
- 최소 행 높이의 남은 밴드는 실제 예약 높이만큼 감소한다. 내용이 끝났다고 남은 밴드를 버리지 않는다.
  반대로 막힌 첫 줄 앞에서 최소 높이만으로 빈 진행을 만들어내지 않는다.
- `Never`/`BetweenRows`는 전체 표/현재 행의 필요 공간이 먼저 충족되어야 한다.
  `WithinCells`만 구성된 유닛 경계와 자식 표의 분할 정책을 사용한다. TAC 여부를 대신 쓰지 않는다.
- 실패한 조회는 원래 커서를 바꾸지 않는다. 0 여백을 지나갔다는 사실만으로 높이 0의 조각을 방출하지 않는다.
- 소수 단위 예산은 실제 배치 좌표의 끝점으로 fit을 검사한다. 예를 들어 IEEE 부동소수에서
  `1.2 - 1.0 < 0.2`여도 `1.0 + 0.2 == 1.2`다. 차감 예산 비교만으로 분할 금지 표의
  일부만 배치하지 않도록 실제 끝점 비교를 사용한다. epsilon 증액이나 좌표 clamp는 하지 않는다.
  원자 행의 전체 구성과 fit이 모순되면 부분 조각을 성공으로 내보내지 않고 계약 오류를 반환한다.
- 자식 계획 구성 및 IR 탐색의 최대 깊이 64는 재귀 자원 제한이다. 문서 ID나 조판 결과를 맞추는 조건이 아니다.

실제 호출 경로:

`from_ir_contents → 문단 구성 결과/자식 계획 → from_flow_rows → TableCursor::fit →
FlowCursor::fit → 자식 TableCursor::fit → 확정된 자식 placement/continuation → 부모 행 외곽`

`reserved_height()`와 `placement()`는 동일한 조각 높이를 읽는다. 자식 높이는 반환된 조각의
예약값을 부모에서 그대로 합산하며 다시 원래 선언 높이로 교체하지 않는다. **아직 RenderTree/paint
소비자는 없으므로 최종 backend까지 공통 결과가 연결되었다고 주장하지 않는다.**

## 독립 기대값과 검사

`tests/cases/issue_7353_table_v2_nested.rs`는 합성 입력이다.
상단 여백 3, 앞 문단 10, 자식의 행 20/30, 뒤 문단 5, 아래 여백 7이면 총 물리 높이는 75다.
예산 40에서 첫 조각은 `3+10+20=33`, 다음 조각은 `30+5+7=42`다.
두 번째 조각에는 자식 행 1과 뒤 문단만 있어야 하며 앞 문단/자식 행 0이 반복되면 실패다.
자식이 분할 금지이면 첫 조각은 13, 다음은 `50+5+7=62`여야 한다.

추가 경계: 3단 중첩의 원점/컷, 남은 안 여백·최소 물리 밴드, 0/부족/소수 예산,
서로 다른 속도로 전진하는 병렬 셀, IR 여백 선택, 컨트롤 슬롯 누락, 미지원 입력 거부.
rowspan은 입력부터 거부하므로 여러 rowspan 종료 경계는 이번 검증에 비해당이다.
실제 glyph·테두리·뒤 문단 RenderTree 위치와 사용자 편집 의도 판정은 여전히 미검증이다.

| 검사 | 결과 |
| --- | --- |
| 신규 재귀/IR 계약 | 최종 소스 9/9 PASS |
| R2 기하 계약 | 최종 소스 8/8 PASS |
| Legacy #6923 집중 검사 | 10/10 PASS — V2 개선 증거와 구분 |
| Native Clippy | `cargo clippy --locked -p rhwp --lib -- -D warnings` PASS |
| 포맷 | `cargo fmt --all -- --check` PASS |
| WASM compile | `cargo check --locked -p rhwp --lib --target wasm32-unknown-unknown` PASS |
| manifest base 비교 | `7a95e46e025470a4d7a7b59ad68ec02958bda738` 대비 PASS |

2026-09-23, 선행 HEAD `7360ba58c`에 이번 미커밋 소스 변경을 적용한 상태에서 검증했다.
`/home/edward/mygithub/rhwp-review-7353`에 같은 소스를 반영하고 파생 suite를 준비했다.
Cargo target은 `/home/edward/mygithub/rhwp/target/pr-review`, 테스트 profile은 `release-test`다.
세 테스트는 `node scripts/run-rust-test.mjs <source-name> -- --cargo-profile release-test
--target-dir /home/edward/mygithub/rhwp/target/pr-review`로 각각 실행했다.
총 27건 모두 통과했으며 필터로 제외된 다른 검사는 전체 회귀 실행으로 세지 않는다.

최종 로그: [validation.log](../../output/7353/r3/validation.log).
정확한 소스 스냅샷: [source.sha256](../../output/7353/r3/source.sha256).
검증 후 `table_v2/*.rs`와 두 계약 테스트의 source/review 파일 일치를 확인했다.
`git diff --check`도 통과했다. nextest 설치 버전 0.9.137/권장 0.9.140 및
`report-skipped` 설정 경고는 남아 있으나 실행 실패는 없었다.

신규 API가 이전 커밋에 없는 컴파일 실패는 결함 검출의 수정 전 FAIL로 세지 않는다.
첫 빌드의 부동소수 타입 추론 오류는 타입을 명시해 수정했다. 이를 조판 결함 재현으로 기록하지 않는다.
전체 CI·Visual Sweep·WASM 배포 빌드·Studio 연결은 아직 수행하지 않았다.

## 다음 연결

실제 폰트/서식 기반 문단 구성 결과와 paint payload를 함께 보존해 기하 계획의 줄 소유자를
최종 RenderTree와 연결해야 한다. 측정과 paint에서 문단을 각각 다시 구성해서는 안 된다.
그 후 문서 단위 명시적 V2 선택·독립 세션·Legacy 재실행을 연결하고 실제 샘플로 시각 검증한다.
TAC/어울림은 이 순차 블록 모델에 강제로 끼워 넣지 않고 문단 줄 구성·가용 영역 계약을 확장한다.
