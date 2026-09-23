# Task #7353 — V2 중첩 표의 반복 제목행

- Issue: #7353. 선행 절편: `ac8d99335` — [stage8](task_m100_7353_stage8.md).
- 범위: fresh IR 독립 표 미리보기의 연속 상단 제목행을 재귀 분할·예약·출력에 연결한다.
- 상태: 제한된 제목행 반복 구현·선택 71건·범위 lint 통과. Legacy 기본 경로와 문서 전체 엔진 선택은 변경하지 않는다.

## 규칙과 지원 경계

[HWP 5.0 표 76](../tech/한글문서파일형식_5.0_revision1.3.md)의 제목 줄 자동 반복과
IR의 `Table.repeat_header` / `Cell.is_header`를 입력으로 사용한다. 제목 반복은 본문 내용의
재소비가 아니라 각 조각의 물리 공간 예약이다. 제목은 한 묶음으로 배치하고 이어지는 본문의
수용 결과까지 확인한 뒤 조각을 확정한다. 본문이 전혀 진행하지 못하면 제목만 출력하지 않는다.
편집된 빈 줄·여백·최소 높이의 남은 물리 밴드는 본문 진행이며, 글자가 없다는 이유로 버리지 않는다.

이번 수용 범위는 병합 없는 grid에서 모든 셀이 제목으로 표시된 연속 상단 행이다. 일부 셀만 제목인
행과 중간의 산재한 제목 표시는 미지원으로 거부한다. 기존 `leading_header_rows`의 넓은 해석을
일반 규칙으로 복제하지 않는다. 전체가 제목인 표는 원자적으로 한 번만 출력하고 종료한다.
`repeat_header=false`, 제목 표시 없음, `TablePageBreak::None`은 재출력 없이 기존 흐름을 따른다.
TAC/어울림·저장 줄·rowspan·테두리 등 stage8 미지원 경계는 그대로다.

원본 입력은 고정 18px 줄 전진과 12px 글자 상자를 지정한 합성 fresh IR다. 사양은 반복 속성의
존재를 뒷받침하지만 모든 경계의 한컴 실제 동작을 보증하지 않는다. prefix 원자성과 수용 경계는
이번 신규 V2의 명시적 제한이며 실제 한컴 출력과의 피델리티는 별도 검증 대상이다.

## 생산 → 분할/예약 → 실제 출력

- `ir.rs::bind_table`: grid와 제목 표시를 확인하여 불변 `TableContentPlan.header_rows` 생성.
- `fragment.rs::fit_with_header`: 같은 plan의 prefix를 `fit_rows(..., atomic=true)`로 구성하고,
  실제 수용 높이만큼 본문 가용 영역을 줄인다. 본문은 원래 continuation에서 진행한다.
- `fit_rows`: 기존 FlowCursor로 줄/공백/자식 조각을 소비한다. 반복 제목과 본문 모두 같은
  재귀 자식 결과, 패딩과 최소 높이를 사용하며 별도 제목 paint 높이를 계산하지 않는다.
- 본문 예산 실패는 prefix+본문 요구 높이를 반환한다. 성공 시 두 final placement를 결합한다.
  continuation과 `rows()`는 원본 본문 소비 위치를 나타내며 반복 prefix로 되감지 않는다.
- `flow.rs`: 자식의 증가한 실제 `reserved_height`를 부모 예약에 반영한다. 부모와 자식이 모두
  제목을 반복해도 한 예산에서 수용하며, 내용 종료 뒤 남은 host·뒤 문단을 보존한다.
- `text.rs::TextPaint::build_node`: 원래 소유 키의 불변 payload를 새 final 좌표에 출력한다.
  별도 제목 전용 paint 경로·clamp·클리핑·Legacy fallback은 없다.

`Never`는 기존 전체 표 fit, `BetweenRows`는 제목+원자 본문 행, `WithinCells`는 제목+본문 컷을
사용한다. spans/각주/캡션은 수용되지 않으므로 해당 컷 검증을 완료했다고 보고하지 않는다.

## 독립 기대값과 검증 계획

정식 source: `tests/cases/issue_7353_table_v2_headers.rs`.

- 제목18 + 본문3×18: 예산36에서 각36, 제목 y30/본문 y48. 본문 A/B/C는 각각 한 번.
- 예산29는 제목18 뒤 글자 상자12가 안 들어간다. 요구30으로 실패하고 같은 세션 재시도도
  실패한다. 페이지/본문 cursor는 진행하지 않는다.
- 제목2×18 + 원자 행18: 예산53은 실패,54는 제목2행+본문1행을 수용한다.
- 제목 위2/아래3, 최소30 + 본문 위1/2×18/아래4: 예산53에서 조각49/52.
- 부모/자식 제목18 각각, before/본문3줄/host/after: 예산72에서72/72/54.
- 예산 sweep, 제목 내부 중첩 표, 첫 부분 페이지 이월, 전체 제목 종료, 미지원 표시를 검사한다.
- 최종 RenderTree의 표·셀·줄 포함 관계와 좌표를 검사하고 실제 SVG/PNG를 직접 확인한다.

## 검증 기록

제품 기준은 `ac8d99335` + 이번 변경, 검증은 `/home/edward/mygithub/rhwp-review-7353`에서
고정 target `/home/edward/mygithub/rhwp/target/pr-review`를 사용했다. 새 검사는 `tests/cases/`에만
추가하며 파생 suite를 제품 브랜치에 포함하지 않는다.

- 최초 [before.log](../../output/7353/r9/before.log)는 검사 코드의 `unwrap_err`가 출력 타입의
  Debug를 요구한 **빌드 실패**다. 재현으로 세지 않는다. 검사만 수정한
  [before-executed.log](../../output/7353/r9/before-executed.log)에서 7건이 기존의
  `Unsupported("header, caption or cell spacing")`로 실패했다. 이는 기존 V2 미지원 기능의
  확인이지 기존 Legacy 조판 결함 7건이라는 뜻이 아니다.
- 첫 구현 [7건 PASS](../../output/7353/r9/after.log), 경계 보강
  [12건 PASS](../../output/7353/r9/after-final.log), 두 열과 저장·재파싱 포함 최종
  [14건 PASS](../../output/7353/r9/after-final14.log). 보강 7건의 before 실행은 하지 않았다.
- 기존 기하8/중첩12/텍스트7/중첩 출력9/세션9/fresh IR9 및 Legacy #5301의3건:
  [57건 PASS](../../output/7353/r9/selected-tests.log). 신규 포함 **71건 PASS**.
- 명령: `node scripts/run-rust-test.mjs <case> -- --cargo-profile release-test
  --target-dir /home/edward/mygithub/rhwp/target/pr-review --no-fail-fast`.
- [source.sha256](../../output/7353/r9/source.sha256)의 소스/검사18개 파일을 제품과 review
  양쪽에서 대조했다. 제품 코드는 최초 after 실행 이후 바뀌지 않았으며 검사만 보강했다.
- `cargo clippy --locked -p rhwp --lib --target-dir <고정 target> -- -D warnings`:
  [Native PASS](../../output/7353/r9/native-clippy.log). 이 단계 뒤 명령 묶음이 exit143으로
  종료되어, 완료한 검사는 반복하지 않고 후속 명령만 별도로 실행했다.
- 동일 lib lint에 `--target wasm32-unknown-unknown`을 추가:
  [WASM lib PASS](../../output/7353/r9/wasm-clippy.log).
- `cargo clippy --locked --test regression_suite_003 --target-dir <고정 target>
  -- -D warnings`: [최종 검사 target PASS](../../output/7353/r9/test-clippy.log).
- `cargo fmt --all -- --check`: [PASS](../../output/7353/r9/fmt.log).
  review worktree에서 manifest 준비 후 `node scripts/rust-test-suite-manifest.mjs --check
  --base-ref 7a95e46e025470a4d7a7b59ad68ec02958bda738`:
  [정책 PASS](../../output/7353/r9/policy-check.log).
- 변경 문서 2개 [상대 링크 PASS](../../output/7353/r9/doc-links.log), `git diff --check` PASS.
  source-side test module 변경은 없다. nextest 버전·`report-skipped` 경고는 로그에 보존했다.

전체 CI/workspace lint·전체 회귀와 Docker WASM build는 실행하지 않았다. WASM lib lint를
fresh WASM 실행 증거로 세지 않으며, 원격 push/PR/이슈 변경도 수행하지 않았다.

### 실제 합성 출력

최종 검사에 `ISSUE7353_PREVIEW_DIR=.../output/7353/r9/preview`를 지정해 SVG와 합성 HWPX를
내보냈다. `rsvg-convert -b white <svg> -o <png>`로 래스터화한 6개 PNG를 직접 열어 읽었다.
단일 제목의 alpha/beta/gamma와 부모·자식 동시 제목의 before/A, B/C, host/after 순서를 확인했다.
글자 간격의 기존 fresh preview 차이는 남아 있으며, 테두리 없는 출력의 정렬·포함 관계는 정식
RenderTree assertion으로 검사한다. 이 자료는 한컴 글꼴/외곽선 시각 통과 증거가 아니다.

![부모/자식 첫 제목](../../output/7353/r9/preview/nested-header-0.png)

![부모/자식 반복 제목과 본문 이어받기](../../output/7353/r9/preview/nested-header-1.png)

![자식 종료 후 host와 후속 문단](../../output/7353/r9/preview/nested-header-2.png)

[출력 hash](../../output/7353/r9/preview.sha256),
[합성 HWPX](../../output/7353/r9/preview/fresh-headers.hwpx)를 보존한다.
HWPX 재파싱 검사는 합성 파일의 소유·반복 속성과 좌표 연결 검사이지 정상 한컴 생성본 검증이 아니다.

## 남은 범위

실물 기준 PDF, Native/fresh WASM Visual Sweep, 문서 전체 선택·Studio 연결은 미검증이다.
저장 줄·TAC/어울림·rowspan·테두리·각주 및 전체 CI 검증은 이번 절편 결과에 포함하지 않는다.
