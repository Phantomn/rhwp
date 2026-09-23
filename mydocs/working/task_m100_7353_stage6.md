# Task #7353 — 중첩 표의 공유 fragment와 실제 출력 연결

- 선행 절편: `acf43598f` — [stage5](task_m100_7353_stage5.md).
- 범위: 명시적으로 구성된 셀 흐름의 재귀 paint 연결. Legacy 기본 경로는 변경하지 않는다.
- 상태: R3 진행 중. 문서 전체 엔진 선택이나 TAC/어울림 지원 완료가 아니다.

## 구현 경계

`PreparedTextTable::from_flow_rows`가 새로운 `TextFlowRow/Cell/Block` 입력을 받는다.
이 입력은 문서 IR의 대체물이 아니라 **앵커·줄 소유 관계가 이미 결정된 순차 셀 흐름**의
임시 소비 경계다. 문단은 실제 공통 글줄 구성기로 구성하고, 자식 표는 준비된 내용 계획과
출력 snapshot을 함께 연결한다. 임의 RenderNode를 외부에서 주입하지 않는다.

`prepare(&Table, ...)`의 IR 수용 범위를 임의로 넓히지 않았다. 해당 API는 여전히 개체가 없는
fresh plain-text 표만 수용한다. 자리차지 표의 host 문단을 자식 앞에 무조건 추가하거나 빈
host 줄을 무조건 삭제하면 앵커 원점과 문단 점유가 달라진다. 그런 규칙을 추측해 연결하지 않는다.
저장 LineSeg, TAC, Square/어울림의 문단 소유·위치 해석은 후속 문서 연결에서 별도로 구현한다.

## 생산 결과부터 출력까지

| 값/소유 | 생산·측정·예약 | 실제 배치 |
| --- | --- | --- |
| 문단의 실제 줄/물리 간격 | `TextComposer::compose` → 동일 TextLine 상자를 FlowBlock으로 구성 | 보관한 동일 payload를 확정된 줄 좌표로 평행 이동 |
| 자식 내용과 출력 데이터 | `PreparedTextTable`의 plan/paint 쌍 → `FlowBlock::Table` | 셀 주소 + 문단/컨트롤 소유자로 자식 paint 선택 |
| 시작·끝 컷과 요구 높이 | `FlowCursor::fit`의 자식 cursor → `TableCursor::fit` | `NestedTablePlacement`의 최종 페이지 좌표를 재귀 사용 |
| 누적 예약·이월 | 실제 수용한 child fragment 높이만 누적, 미완료 cursor 보존 | `TextPaint::build_node`는 높이를 다시 계산하지 않음 |
| 뒤 문단·여백·최소 높이 | 자식 완료 뒤 후속 block; 여백/남은 물리 밴드는 기존 V2 cursor | 확정 부모 외곽과 실제 뒤 문단 위치를 함께 출력 |

재귀 결과는 이미 페이지 좌표이므로 부모 원점을 다시 더하지 않는다. 부모 줄과 자식 표는
payload의 흐름 순서로 결합한다. 줄들을 전부 출력한 뒤 자식 표를 몰아서 출력하지 않는다.
같은 문단/컨트롤 번호도 서로 다른 셀·표의 snapshot에서 구별한다. 전체 노드를 준비한 다음에만
페이지 ID를 할당하고 append하여 오류 중간 상태를 페이지에 남기지 않는다.

호출 위치: `text_flow.rs:48`(구성 진입) / `text_flow.rs:128`(자식 snapshot 연결) →
`flow.rs:70`(자식 fit) / `flow.rs:86`(수용 높이 누적) → `text.rs:178`(실제 재귀 노드 구성) →
`text.rs:168`(페이지 append). 기존 `fragment.rs`와 `flow.rs`의 기하 분할 규칙 자체는 수정하지 않았다.

자식 표의 DPI가 부모와 다르면 명시적으로 거부한다. 다른 DPI의 pixel 계획을 자동 축소하지
않는다. 중복 소유자, 명시적 페이지 나눔 등 미지원 입력도 오류로 반환한다.
rowspan·반복 제목·캡션·각주·문서 앵커는 이 입력에 표현되지 않으며 해당 경로 검증으로 세지 않는다.

## 독립 기대값과 정식 계약

검사 source: `tests/cases/issue_7353_table_v2_nested_text.rs`.
수동 생성한 fresh 문단과 명시적 흐름이며 정상 저장 문서·한컴 PDF를 대신하지 않는다.
12px 글자, 고정 18px 줄간격, DPI 7200에서 1HU=1px를 입력으로 선언한다.
글줄 상자는 12px이고 별도 간격6px와 합쳐 전진18px가 된다. 상자 높이와 줄 전진을 구별한다.

- 자식: 상2 + alpha18 + beta18 + 하3 = 41.
- 부모: 상3 + before18 + 자식41 + after18 + 하4 = 84.
- 첫 페이지 예산43: 부모 상3 + before18 + 자식 상2 + alpha18 = 예약41.
  원점(20,30)에서 before(25,33), alpha(27,53), 자식 외곽 y51/h20.
- 이어받기 예산43: beta18 + 자식 하3 + after18 + 부모 하4 = 예약43.
  새 원점(40,80)에서 beta(47,80), after(45,101), 자식 외곽 y80/h21.
- 자식 Never: 첫 예산60에는 부모21만 수용, 다음 예산40은 요구41로 비수용.
  예산63 재질의에서 자식41 + after18 + 하4를 출력하며 소유 내용은 소실되지 않는다.
- 3단 중첩은 바깥 상1/하2와 x 여백4를 추가하여 총87. 각 페이지 실제 좌표와 종료를 검사한다.
- 자식 뒤 빈 문단18 및 최소 높이70의 나머지11 밴드를 보존한다. 글자가 없다는 이유로
  빈 문단/남은 셀 높이를 소비 완료로 취급하지 않는다.
- 16가지 페이지 예산(18~100, 43.5 포함)에서 총84, 각 글자 정확히 한 번, 출력 외곽 이내,
  정상 종료를 검사한다. 동일 소유 번호의 형제 셀, ID 유일성, flat IR snapshot의 중첩도 검사한다.
- 행 단위 분할은 두 자식 행 각각 상2 + 줄 전진18 + 하3 = 23으로 구성한다. 부모의
  두 조각44/45와 실제 다음 행 payload·y를 검사하여 동일 문단 번호의 행 간 혼동을 검출한다.

기존 API에는 이번 실제 중첩 출력 입력이 없었다. API 부재로 인한 컴파일 실패를 수정 전 결함
재현으로 세지 않는다. 기존 `tests/cases/`의 fixture/assertion/baseline/ignore는 변경하지 않는다.

## 검증

- 소스: `acf43598f` + 이번 변경. [source.sha256](../../output/7353/r6/source.sha256)의
  제품/검사13개 파일이 source/review worktree와 바이트 단위로 일치함을 확인했다.
- 실행 worktree `/home/edward/mygithub/rhwp-review-7353`, 고정 target
  `/home/edward/mygithub/rhwp/target/pr-review`.
- `node scripts/run-rust-test.mjs <case> -- --cargo-profile release-test --target-dir
  /home/edward/mygithub/rhwp/target/pr-review --no-fail-fast`: **39건 PASS**.
  신규 중첩 출력9건은 [최종 로그](../../output/7353/r6/nested-text-tests-complete.log),
  기존 V2 기하8/중첩12/텍스트7과 Legacy #5301의3건은
  [선택 회귀 로그](../../output/7353/r6/selected-tests.log)에 기록했다.
  마지막 추가는 테스트1건뿐이며 동일 제품 소스의 기존30건을 중복 실행/합산하지 않았다.
- `cargo clippy --locked -p rhwp --lib --target-dir <고정 target> -- -D warnings`:
  [Native PASS](../../output/7353/r6/native-clippy.log).
- 동일 명령에 `--target wasm32-unknown-unknown`을 추가한 lib lint:
  [WASM PASS](../../output/7353/r6/wasm-clippy.log).
- `cargo clippy --locked --test regression_suite_023 --target-dir <고정 target>
  -- -D warnings`: [최종 검사 target PASS](../../output/7353/r6/test-clippy-complete.log).
- `cargo fmt --all -- --check` PASS. manifest 준비 후
  `node scripts/rust-test-suite-manifest.mjs --check --base-ref
  7a95e46e025470a4d7a7b59ad68ec02958bda738`:
  [정책 PASS](../../output/7353/r6/policy-check.log).
  파생 harness는 review worktree에서만 사용하며 제품 변경에 포함하지 않는다.

첫 실행의 2건 실패는 새 테스트가 고정 줄간격18을 글줄 상자 높이로 잘못 사용한 결과다.
공통 구성기의 `frame_metrics_for_line` / `compute_line_spacing_hwp(Fixed)`와 입력 글자12px를
대조하여 **상자12 + 간격6**으로 기대값을 바로잡았다. 제품 코드는 이 실패 때문에 변경하지 않았다.
[첫 실행 로그](../../output/7353/r6/nested-text-tests.log)는 보존하며 제품 결함 수정 전 FAIL로 세지 않는다.

### 합성 출력의 직접 확인

최종9건 실행 시 `ISSUE7353_PREVIEW_DIR=.../output/7353/r6/preview`로 동일 fragment의
SVG를 내보내고, `rsvg-convert -b white <svg> -o <png>`로 rasterize했다.
[출력 hash](../../output/7353/r6/preview.sha256)를 보존한다. 첫 조각의 before/alpha,
다음 조각의 beta/after가 각각 독립적인 위치에 표시되는 것을 직접 확인했다.
테두리 없는 합성 입력이므로 아래 PNG로 표 외곽선의 한컴 일치를 주장하지 않는다.
표/셀 외곽 좌표와 뒤 문단 위치는 위 정식 RenderTree 계약에서 직접 검사했다.

![합성 첫 조각](../../output/7353/r6/preview/nested-first.png)

![합성 이어받기 조각](../../output/7353/r6/preview/nested-continuation.png)

Native/fresh WASM compare·overlay·review와 실물 한컴 PDF 기준 Visual Sweep은 **미검증**이다.
현재 V2 표 내부 API가 문서/WASM 세션에 연결되지 않아 해당 증적을 아직 만들 수 없다.
위 합성 raster 확인이나 WASM lib lint를 그 증거로 대신하지 않는다. 전체 workspace CI lint와
전체 회귀도 이번 선택 검증 결과에 포함하지 않는다. 원격 push/PR 작업은 수행하지 않았다.

## 남은 연결

문서 IR에서 유효한 저장 줄과 새 줄 구성의 소유·앵커를 확정하여 이 공통 결과로 내려보내는 경로,
문서 세션의 V2 선택, 편집/캐시, Native/fresh WASM 출력 및 실물 기준 PDF Visual Sweep은 남아 있다.
따라서 R3 완료나 rhwp-studio 중첩 표 개선 완료로 보고하지 않는다.
