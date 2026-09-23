# Task #7353 — 자리차지 중첩 표의 fresh IR·출력 연결

- Issue: #7353. 선행 절편: `c90c1000e` — [stage7](task_m100_7353_stage7.md).
- 범위: 기존 독립 V2 세션에서 fresh 문단 IR의 자식 표를 재귀 측정·분할·출력한다.
- 상태: 제한된 IR 연결 구현·선택57건·범위 lint 통과. Legacy 기본 경로와 문서 전체 엔진 선택은 변경하지 않는다.

## 규칙과 수용 경계

[표 조판 규칙](../tech/table_layout_rules.md)의 `TopAndBottom + treat_as_char=false`는
문단 기준 원점에 자리차지하고 본문을 위/아래로 밀어내는 계약이다. 이번에는 **문단 위/왼쪽 기준,
오프셋과 바깥여백 0인 자식 표 하나**만 연결한다. 자식이 문단 시작점부터 전체 폭을 배제하므로
호스트 글줄은 자식 뒤에 배치한다. 글자가 없는 호스트도 공통 문단 구성기의 물리 줄을 보존한다.
표 선언 위치의 UTF-16 슬롯과 문단의 배치 기준 원점은 구별한다.

이는 모든 컨트롤을 배열 순서로 쌓는 규칙이 아니다. TAC·Square·페이지/용지 기준·0이 아닌
오프셋·여러 동시 앵커는 기존 수용 경계 또는 새 명시적 오류로 거부한다. host의 좌우 여백,
들여쓰기, 위 간격은 문단 원점을 바꾸므로 이 절편에서는 거부하며 표 뒤로 잘못 옮기지 않는다.
아래 간격은 원래 문단 구성 결과로 유지한다. 저장 LineSeg는 지우거나 느슨하게 수용하지 않는다.

입력은 합성 fresh Document IR 및 그것을 직렬화한 입력이며 정상 한컴 생성본이 아니다.
위 정본의 일반 의미를 제한된 수용 영역의 구현 계약으로 사용하지만, 기존 helper나 합성 검사의
통과를 한컴 피델리티 입증으로 승격하지 않는다. 빈 호스트와 실제 한컴 출력의 차이는 실물 검증 때
별도로 판독해야 한다. 미지원 조합을 성공시키기 위해 Legacy로 fallback하지 않는다.

## 공통 결과와 실제 소비 경로

`text.rs::prepare` → `text_ir.rs::prepare` → `ir.rs::bind_table`의 grid/앵커/깊이 검사 →
`IrTextComposer`의 순서 있는 `ParagraphItem` → 기존 `TableContentPlan`/`FlowCursor` →
`TableFragmentPlan` → 동일 소유 순서를 보관한 `TextPaint::build_node` → 세션의 성공 commit.

새 `text_ir.rs`는 문단·컨트롤 소유 연결만 맡는다. 기존 `TextComposer`는 fresh plain-text
구성과 payload 생성을 맡고, `flow.rs`/`fragment.rs`는 그대로 컷·예약·재개를 계산한다.
`text_flow.rs`의 명시적 순차 흐름 API도 유지하며 거기에 IR 앵커 추정을 섞지 않는다.

- `text_ir.rs:62`에서 생산한 순서 중 줄/표 슬롯을 그대로 paint recipe에도 보관한다.
  paint 단계가 `controls` 배열에서 순서를 다시 추측하지 않는다.
- `ir.rs:170`은 같은 항목을 측정 흐름으로 바꾸고, 자식 슬롯에서 재귀한다.
  전체 grid 순회 순서와 문단 payload 선행 기록 순서를 맞춰 셀/문단/컨트롤 소유자를 연결한다.
- `flow.rs`가 자식 컷의 실제 요구·수용 높이를 누적하고 실패/미완료 cursor를 보존한다.
  빈 밴드·상하 여백과 최소 높이도 기존 V2 소비 모델을 따른다.
- paint는 확정된 부모/자식 페이지 좌표만 사용한다. 추가 높이 보정이나 클리핑으로 덮지 않는다.
  안쪽 표의 미지원 테두리 역시 무시하지 않고 전체 준비를 실패시킨다.

원본 문단의 controls를 변경하지 않는다. 텍스트 전용 임시 복사에서는 이미 별도 소유된 non-inline
컨트롤만 분리하고, UTF-16 offset·글자모양 참조·저장 줄 정보는 유지한다. 저장 줄 검사를 우회하는
fresh화가 아니며, 새 API를 공개하거나 Legacy 표 계산을 호출하지 않는다.

## 독립 기대값과 검사

정식 source: `tests/cases/issue_7353_table_v2_ir_text.rs`.
96 DPI, 글자900 HU=12px, 고정 줄간격2700의 IR 단위=18px 전진을 입력으로 선언한다.

- 자식: 위2 + alpha18 + beta18 + 아래3 =41.
- 부모: 위3 + before18 + 자식41 + host18 + after18 + 아래4 =102.
- 예산43: 조각41/39/22. 실제 before(25,33), alpha(27,53), 다음 beta(27,30),
  host(25,51), 마지막 after(25,30). 자식 외곽은 첫 y51/h20, 다음 y30/h21이다.
- 빈 host에도18을 예약하고 아래 문단 간격5를 더하면 전체107이며 after y115다.
- 원자 자식은 예산60에서 먼저 부모21만 수용하고 다음 자식41+host18, 마지막22로 진행한다.
- 3단 중첩과 역순 저장된 형제 셀, 같은 문단/컨트롤 번호의 지역 소유를 실제 출력으로 검사한다.
- 글자모양 UTF-16 위치와 명시적 개행, 10가지 페이지 예산, source 비변경,
  TAC/어울림/offset/바깥여백/저장 줄/복수 앵커/host inset의 비적용 조건을 검사한다.
- HWPX 직렬화/재파싱은 동일 합성 입력의 파서 연결 검사이며 정상 한컴 생성본 증거는 아니다.

## 검증 기록

검증 소스는 `c90c1000e` + 이번 변경, review worktree는
`/home/edward/mygithub/rhwp-review-7353`, 고정 target은
`/home/edward/mygithub/rhwp/target/pr-review`다. 파생 suite는 제품 브랜치에 포함하지 않는다.

첫 before 시도는 새 검사가 private API·비직렬화 Document·잘못된 font 필드명을 사용한 빌드 실패였다.
[로그](../../output/7353/r8/before-tests.log)를 보존하며 결함 재현으로 세지 않는다. 검사만 수정한
[실제 before 실행](../../output/7353/r8/before-tests-executed.log)에서는 양성6건이 기존의
`text preview stored rows or controls` 거부로 실패했다. 추가1건의 실패는 새 상세 오류 문자열
계약의 차이이며 양성 입력 결함 재현으로 합산하지 않는다. 깊은 미지원 입력 거부1건은 이미 통과했다.

- [수정 후 최종9건](../../output/7353/r8/after-tests-final.log) PASS. 저장·재파싱 검사1건은
  최초 before 실행 이후 보강했으므로 그 검사의 before 실패를 실행했다고 보고하지 않는다.
- 기존 V2 기하8/중첩12/텍스트7/중첩 출력9/세션9 및 Legacy #5301의3건:
  [48건 PASS](../../output/7353/r8/selected-tests.log). 신규 포함 **57건 PASS**.
- 실행 명령: `node scripts/run-rust-test.mjs <case> -- --cargo-profile release-test
  --target-dir /home/edward/mygithub/rhwp/target/pr-review --no-fail-fast`.
- [source.sha256](../../output/7353/r8/source.sha256)의 제품/검사17개 파일을 product/review
  양쪽에서 대조했다. 최종9건 추가는 테스트만의 변경이며 제품 소스는 첫 after8건 실행과 같다.
- `cargo clippy --locked -p rhwp --lib --target-dir <고정 target> -- -D warnings`:
  [Native PASS](../../output/7353/r8/native-clippy.log).
- 동일 명령에 `--target wasm32-unknown-unknown`을 추가한 lib lint:
  [WASM PASS](../../output/7353/r8/wasm-clippy.log).
- `cargo clippy --locked --test regression_suite_003 --target-dir <고정 target>
  -- -D warnings`: [최종 검사 target PASS](../../output/7353/r8/test-clippy.log).
- `cargo fmt --all -- --check`: [PASS](../../output/7353/r8/fmt.log).
  manifest 준비 뒤 `node scripts/rust-test-suite-manifest.mjs --check --base-ref
  7a95e46e025470a4d7a7b59ad68ec02958bda738`: [정책 PASS](../../output/7353/r8/policy-check.log).
- 변경 Markdown 두 파일의 상대 링크 검사 및 `git diff --check` PASS.

source-side test module 변경은 없고 새 검사는 `tests/cases/`에만 추가했다. nextest 설치0.9.137 /
권장0.9.140 및 `report-skipped` 미지원 경고는 보존하며 도구 버전을 변경하지 않았다.

### 합성 출력 직접 확인

최종 검사에서 `ISSUE7353_PREVIEW_DIR=.../output/7353/r8/preview`로 실제 SVG를 기록한다.
`rsvg-convert -b white <svg> -o <png>`로 rasterize하여 before/alpha, beta/host, after가
각 조각에서 구분되어 출력되는 것을 직접 확인했다. 테두리 없는 합성 출력이므로 외곽선 피델리티나
글꼴/자간의 한컴 일치 판정은 아니다. 표/셀 외곽과 내용 위치는 정식 RenderTree assertion으로
검사했다. [출력 hash](../../output/7353/r8/preview.sha256)와 합성 HWPX를 보존한다.

![첫 조각](../../output/7353/r8/preview/ir-nested-0.png)

![이어받기와 호스트](../../output/7353/r8/preview/ir-nested-1.png)

![뒤 문단](../../output/7353/r8/preview/ir-nested-2.png)

실물 한컴 PDF와의 Native/fresh WASM Visual Sweep은 **미검증**이다. 문서 전체 선택 및 WASM
preview 연결이 아직 없어 기존 Studio 실행을 이번 V2 경로의 증거로 사용할 수 없다.
전체 CI/workspace lint·전체 회귀도 이번 결과에 포함하지 않으며, 원격 push/PR은 수행하지 않았다.

## 남은 범위

이번 결과는 제한된 fresh IR 자리차지 중첩 표의 종단 연결이다. 저장 줄의 적격성, TAC 줄 소속,
어울림 가용 영역, 0이 아닌 offset/문단 inset, 반복 제목·rowspan·테두리·각주와 실제 문서 엔진
선택/Native·WASM 비교는 남아 있다. 독립 미리보기 지원을 R3 전체 완료로 보고하지 않는다.
