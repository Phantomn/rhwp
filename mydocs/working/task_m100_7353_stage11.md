# Task #7353 — V2 Native/WASM 미리보기 실행 경계

- Issue: #7353 / 선행 절편: `84744cbd6` ([stage10](task_m100_7353_stage10.md)).
- 상태: 실행 경계 구현·선택 회귀·Docker fresh WASM/Chrome 검증 완료. R3 전체 완료는 아님.
- 범위: 선택 표의 독립 V2 세션을 파일 bytes/JSON 경계와 실제 WASM export에 연결한다.
  문서 전체 엔진 전환·Studio UI·새 조판 규칙·지원 범위 확대는 하지 않는다.

## 공통 결과와 불변식

`parse_document → TablePreviewSession::from_document → 공통 V2 구성/분할 →
next_page의 동일 PageRenderTree → SVG + JSON`을 Native/WASM 양쪽에서 사용한다.
DocumentCore를 생성하지 않으므로 먼저 Legacy 표를 측정하거나 동일 문서를 두 엔진으로 섞지 않는다.
호출자가 페이지 영역·DPI·선택 주소를 명시하고 생성 이후에는 변경할 수 없다.

새 `TableV2Preview` JS 클래스는 bytes를 파싱한 독립 snapshot을 소유한다. `nextPage()` 한 번이
한 조각의 SVG와 실제 WASM tree를 함께 반환하고 종료는 undefined다. native export도 같은
구현을 호출한다. fit/직렬화 오류 전에는 후보 cursor만 진행하며 성공할 때만 세션에 반영한다.
미지원 입력은 throw/error이고 Legacy로 fallback하지 않는다. 기존 HwpDocument API는 무변경이다.

## 독립 기대값과 검증 범위

이 절편은 새 실행 경계이며 stage10의 조판 수치·baseline을 바꾸지 않는다. 합성 fresh HWPX의
고정 줄간격18px, 본문36px, 제목18px와 본문2행은 제목을 반복한2쪽이다. 중첩 표 뒤 host/after,
첫 부분 영역 건너뛰기, 페이지 제한·실패 재시도, 입력 bytes 변경 후 snapshot 고정,
불량 options/주소/저장 줄/rowspan 등의 거부를 실제 transport 호출로 확인한다.
원래 없던 export의 추가이므로 컴파일 실패를 수정 전 결함 검출로 세지 않는다.

Docker fresh WASM을 실제 Chrome에서 실행해 독립 기대 좌표·소유 순서와 native의 SVG/tree를
대조하고 양쪽 PNG를 직접 확인한다. 한컴 PDF가 없는 합성 입력을 한컴 시각 일치로 보고하지 않는다.

## 실험용 호출 계약

```js
import init, { TableV2Preview } from './pkg/rhwp.js';
await init();
const preview = new TableV2Preview(bytes, JSON.stringify({
  selection: { section: 0, paragraph: 0, control: tableControlIndex },
  dpi: 96,
  pages: {
    width: 400, height: 400,
    body: { x: 20, y: 30, width: 300, height: 36 }, first_y: 30
  },
  max_pages: 100
}));
try {
  for (;;) {
    const next = preview.nextPage();
    if (next === undefined) break;
    const { schema_version, engine, scope, page_index, svg, render_tree } = JSON.parse(next);
    // schema_version=1, engine="table_v2", scope="selected_table".
    // render_tree.root: serde RenderNode. 기존 getPageRenderTree의 간략 JSON과 다른 계약이다.
  }
} finally { preview.free(); }
```

모든 주소와 page_index는0기반이다. 선택 주소는 저장 전 임시 IR이 아닌 **파일 파싱 후 IR**의
주소다. HWPX는 구역 컨트롤을 추가할 수 있으므로 root의 control=0이 표라는 가정을 하지 않는다.
이 실험 API는 표 주소 탐색 UI·문서 전체 조판·편집·hot-patch 연결을 제공하지 않는다.
원본 bytes를 다시 열면 새 snapshot이다. 실행 중 DPI/폭/엔진 교체 API는 없으며
기존 Legacy로의 실행은 기존 HwpDocument를 별도로 만드는 명시적 호출이다.

## 검증 기록

기준 HEAD는 `84744cbd6`, 고정 policy base는 `7a95e46e025470a4d7a7b59ad68ec02958bda738`.
증적은 `output/7353/r11/`, Rust 계약 원본은 `tests/cases/issue_7353_table_v2_export.rs`다.

- 최초 실행: 2 PASS / 4 FAIL (`export-tests.log`). 테스트가 저장 전 control=0을 사용해
  InvalidSelection이었다. 실제 파싱 후 표 주소를 선택하도록 테스트만 수정했다.
- 수정 중 manifest 미준비 실행:0건 (`export-final.log`). **통과로 계산하지 않음**.
  source 크기 변경에 따른 suite 재배정 후 review worktree에서 `--prepare`를 다시 적용했다.
- 다음 실행:5 PASS / 1 FAIL (`export-final-prepared.log`). 전체0 LineSeg는 기존 parser의
  구역 단위 정규화로 제거된다(`src/parser/hwpx/section.rs`의 `HWPX_SECTION_HAS_SIZED_LINESEG`).
  파서/수용 정책은 바꾸지 않고 양의 높이를 가진 **수동 합성** 저장 줄로 검사 입력을 수정했다.
  round-trip 후 실제 LineSeg 존재/rowspan 보존을 먼저 확인한 뒤 정확한 Unsupported 오류를 검사한다.
- 최종 새 Rust 계약: **6 PASS**, `export-final-input.log`. 고정18px 줄·반복 제목·colspan의
  실제 RenderTree 좌표, 다음 내용·종료, 오류 후 count 불변, 재열기·snapshot 독립을 검사한다.

### 재현

review worktree에서 파생 suite를 준비하고 아래로 fixture/native 출력을 만든다.

```sh
node scripts/rust-test-suite-manifest.mjs --prepare
ISSUE7353_EXPORT_DIR=/home/edward/mygithub/rhwp-task-7353/output/7353/r11/fixtures \
  node scripts/run-rust-test.mjs issue_7353_table_v2_export -- \
  --cargo-profile release-test --target-dir /home/edward/mygithub/rhwp/target/pr-review --no-fail-fast
```

product worktree에서 Docker `wasm` 서비스로 `pkg/`를 생성한다. 기존 `rhwp` compose 프로젝트의
이미지·named volume을 재사용하며 해당 프로젝트의 다른 실행 container가 없음을 확인했다.
host target을 Docker target에 강제로 연결하거나 cache를 삭제하지 않았다.

```sh
docker compose --env-file .env.docker -p rhwp run --rm wasm
VISUAL_SWEEP_CHROME=/home/edward/.cache/puppeteer/chrome/linux-146.0.7680.31/chrome-linux64/chrome \
  node scripts/verify-table-v2-preview-wasm.mjs --pkg pkg \
  --fixtures output/7353/r11/fixtures --out output/7353/r11/browser \
  --dependencies-root /home/edward/mygithub/rhwp
```

브라우저 검사는 localhost의 고정 파일 allowlist만 제공하고 fixture가 없으면 실패한다.
두 backend의 exact SVG/tree 비교 외에 줄 y=30/48, colspan2, 셀 x20/w200, 본문 끝66px,
각 쪽의 title/A/B/host/after 소유 순서를 독립 검사한다. JS 객체2개의 상태 분리, 종료 반복,
페이지 제한/fit 오류 후 cursor 불변, 미지원 줄·rowspan/invalid JSON/주소/영역 거부도 실제 export로 실행한다.

### 최종 결과

| 검사 | 결과 / 로그 |
| --- | --- |
| 신규 native transport 계약 | **6 PASS**, `export-final-input.log` |
| 기존 V2 8개 case / Legacy #5301 대조군 | **78 + 3 PASS**, `selected-tests.log` |
| Rust 선택 검사 합계 | **87 PASS** — 전체 CI가 아님 |
| Native / WASM library / 신규 integration suite Clippy | **PASS**, `clippy-native.log`, `clippy-wasm.log`, `clippy-tests.log` |
| fmt / 고정 base manifest 정책 | **PASS**, `fmt.log`, `policy.log` |
| Docker WASM release + wasm-opt | **PASS**, `docker-wasm.log`, 8분 |
| 실제 Chrome WASM 실행 | **PASS**, `browser-chrome.log`, Chrome146.0.7680.31 |
| Native/WASM 실제 출력 대조 | **7쪽 exact SVG + serde tree 일치**, `browser/manifest.json` |
| 기존 frontend 선언 검사 | **1 PASS**, `bindings.log` |
| 새 JS 검사 도구 구문 / 문서2개 링크 / diff whitespace | **PASS** |

Rust lint는 고정 `/home/edward/mygithub/rhwp/target/pr-review`에서 순차로
`cargo clippy --locked -p rhwp --lib -- -D warnings`, 같은 명령의
`--target wasm32-unknown-unknown`, 새 case의 `--test regression_suite_011`을 실행했다.
이 명령들에는 모두 `--target-dir`로 위 고정 경로를 전달했다. 이어서 `cargo fmt --all -- --check`와
manifest `--check --base-ref 7a95e46e025470a4d7a7b59ad68ec02958bda738`를 실행했다.
source-side cfg(test)는 변경하지 않았다. generated suite/manifest는 review 전용·비추적 산출물이며
product 변경에 포함하지 않았다. `source.sha256`은 최종 구현/테스트/하네스16개 파일,
`pkg.sha256`은 실제 실행한 JS/WASM/TypeScript 선언을 고정한다.

처음 browser 실행은 PATH에 Chrome이 없어 시작 전 실패했다(`browser.log`). 기존 캐시의
Chrome 실행 경로를 명시해 재실행했으며 브라우저 설치나 시스템 설정 변경은 하지 않았다.
WASM SHA-256: `d1c8cad8d2ea6ae9cb8d85abf0f8918b8f5a51c735833fc7f1a2001bf643ec95`.

### 직접 판독

`browser/merged-{0,1}.review.png`, `nested-{0,1,2}.review.png`, `partial-{0,1}.review.png`
7장을 직접 열었다. 제목 반복과 A→B 이어받기, 중첩 표 종료 뒤 host/after 배치를 양쪽에서
확인했고 비교 패널·overlay에서 위치 차이, 중복, 누락을 관찰하지 않았다. 각 페이지의 독립
`*.overlay.png`, `*.native.png`, `*.wasm.png`와 원 SVG/JSON도 보존했다. overlay의 기준은
**Native이며 한컴이 아니다**. 동일 Chrome·기본 글꼴로 raster한 ASCII 합성 표에서의 backend
일치 증거로 한정하며 한글 폰트·실물 문서 피델리티·테두리 정확성은 입증하지 않는다.

`serialize` 실패 시 상태 보존은 후보 세션 후 commit하는 코드 경로로 검토했으며 실제
직렬화 실패를 주입한 검사는 없다. fit/PageLimit 오류 후 상태 보존은 Native와 WASM에서
반복 호출해 실행 검증했다. 중첩 분할 수치·알고리즘이나 기존 Legacy 경로는 변경하지 않았다.

## 남은 범위

이 출력은 여전히 borderless 선택 표 미리보기다. 실제 한컴 PDF 대조, Studio 화면·Canvas,
문서 전체 V2 선택, TAC/어울림·rowspan·테두리·저장 LineSeg 수용은 미지원/미검증이다.
전체 CI/workspace/Native Skia 검증·원격 push/PR/이슈 종료도 이번 개발 절편의 완료 주장에 포함하지 않는다.
