# Task #7353 — A. 실물 문서 종단 연결

- 선행 커밋: `b504181da` — stage18 및 승인된 절차 개정.
- 승인: 개정 구현계획 실행 및 R5까지 과정 자동 승인. 전체 CI 상당 검증·원격 작업·기본값
  전환은 canonical의 별도 승인 경계를 유지한다.
- 상태: 구현 중. R3/A 완료나 실물 한컴 피델리티 통과가 아니다.

## 종단 목표와 독립 근거

선택 표 미리보기에서 문서 본문 흐름으로 연결한다. 용지 설정으로부터 본문 가용 영역을 정하고,
문단·표 조각의 수용 높이가 다음 내용의 원점이 되도록 한다. 문서 흐름을 가짜 표로 감싸지 않고
기존 V2 FlowCursor의 줄·자식 표 소유와 동일 fragment를 사용한다. 표 알고리즘은 Legacy와
혼합하지 않으며 기본 Studio 경로는 유지한다.

우선 원본은 `tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp`,
대응 기준은 같은 폴더의 `148738070_wrapper_table_stored_page_frame-2020.pdf`다.
PDF는 Hancom PDF 1.3.0.550/Hwp2020 표기, 실제7쪽이다. 저장된 PDF를 재사용한다.
초기 탐색의 `output/7353/r19/6923-exploratory-dump.txt`는 이전에 빌드된 release CLI를 사용한
IR 탐색 자료이며 현재 head의 조판/회귀 증거로 세지 않는다. 현재 source의 계약으로 다시 확인한다.

이 입력은 첫 문단부터 TAC5x4표·rowspan·그림과 저장 줄을 포함한다. 본문 wrapper는 문단
오프셋720HU의 RowBreak1x1표이며 내부에도 TAC/rowspan/저장 줄이 있다. 따라서 선택 표
한 개의 단순 서식 확장만으로 원본 전체를 수용할 수 없다. 기존 stage4의 의미 기반 검사와
원본을 유지하고, 기능을 지운 파생 입력을 원본 성공으로 보고하지 않는다.

## 첫 연결 시점의 작업과 경계

첫 연결은 명시적으로 선택하는 독립 V2 문서 세션이며 문단과 표의 실제 본문 점유·이월·
뒤 문단·종료를 검사한다. 저장 줄/TAC/rowspan 등의 미지원 입력은 원문 위치를 포함한 오류로
남긴다. 이 연결의 합성 계약 통과와 #6923 원본 전체 수용은 별도 판정이다.
용지 원점/본문 경계는 원본 PageDef와 공통 용지 계산에 근거하며, 변경 후 실제 RenderTree의
Body·표 외곽·TextLine 위치와 후속 문단을 검사한다. 정상 빈 줄도 공간을 소비한다.

완료까지 이 문서를 갱신한다. 내부 연결점마다 새 stage/승인 요청을 만들지 않는다.

## 구현한 연결과 소비 경로

`document_input::prepare`가 PageDef → 공통 PageLayoutInfo의 본문 사각형을 받는다.
이 공통 용지 계산은 Legacy 표 알고리즘이 아니다. 하나의 uniform section만 수용하며
저장 줄·TAC·다중 앵커·문단 insets 등 미수용 속성은 삭제하거나 fallback하지 않고 오류로 남긴다.
TopAndBottom/문단 위/offset0의 동일 수용 조건은 기존 중첩 경로와 `ir::validate_anchor`를 공유한다.

| 단계 | 생산/소비 위치와 불변식 |
| --- | --- |
| 줄 구성 | `TextComposer::compose` → ParagraphItem의 줄/간격과 같은 RenderNode payload. 빈 문단도18px 점유 |
| 표 계획 | `PreparedTextTable::prepare` → TableContentPlan/TextPaint. 같은 자식 소유·컷을 보존하며 가짜 부모 표를 만들지 않음 |
| 예산/이월 | `FlowCursor::fit` → 일반 줄의 점유와 TableCursor의 실제 reserved_height 누적. None 표가 안 맞으면 문단/자식을 소비하지 않음 |
| 배치 | `DocumentV2Session::next_page_json` → fit.lines의 원점과 fit.tables의 placement만 소비. 중앙 정렬 x와 본문 y를 다시 계산하지 않음 |
| commit | RenderTree/SVG/JSON 성공 뒤에만 cursor와 emitted 갱신. PageLimit/DoesNotFit는 상태 보존 |

SectionDef/단일 ColumnDef는 첫 문단의 구조 정보로만 수용하며 모든 텍스트·char offset·char shape·
저장 줄은 보존한다. qualified 구조 control을 제거한 문단을 일반 composer에 전달하는 것은
저장 조판 정보 제거와 다르다. 표 내부 hit-test/편집·source sidecar 연결은 아직 완료하지 않았다.
Body는 실제 용지 본문 영역이고 clip으로 overflow를 숨기지 않는다. 예산보다 큰 원자 표는
명시적 DoesNotFit이며 빈 페이지를 반복 발행하지 않는다.

## 집중 계약과 현재 결과

`tests/cases/issue_7353_table_v2_document_flow.rs`에 정식7건을 추가했다. 합성 생성본과
실물 원본의 의미를 구분한다. 새 문서 API는 선행 커밋에 없으므로 기존 결함의 red/green을
주장하지 않는다. 기존 선택 표의 미지원 거부 계약은 그대로 유지한다.

- 합성 용지400×200px, 본문(20,30,300,72), 글줄 점유18px, 실제9pt 줄 상자12px.
  이 입력값·정렬식에서 기대값을 정하며 출력이 계산한 높이를 기대값으로 복사하지 않는다.
- HWP/HWPX: before18 + CellBreak표90 → 첫 조각54(y48), 다음 조각36(y30), host/after.
- None표54: 앞 문단36 뒤의 잔여36에는 수용하지 않고 다음 쪽에서 전체 배치. host/after 보존.
- 중첩: before → 부모 빈 줄 → 자식A/B, 다음 쪽C/D·inner·tail, 마지막 쪽host·after.
  자식 x120, 부모 x70, 각 조각 외곽·테두리·셀 내용은 본문 끝102px 이내.
- 실패 재호출/독립 snapshot/종료, 잘못된 옵션·페이지·저장 줄·TAC의 명시적 거부.
- 실제 #6923은 현재 parser로 문단577(모두 stored), 표15(TAC12), rowspan셀21, 그림2를 확인.
  최초 거부는 section decoration/grid/writing direction이다. 다른 미지원 요소도 있으므로 이
  첫 guard만 풀면 된다는 의미가 아니다. `6923-admission.json`은 미수용 증거이지 피델리티 통과가 아니다.

선택 회귀158건 PASS 후 HWP 입력1건을 추가했고 최종 문서 계약7건 모두 PASS다.
중복을 제외하면159건 = V2 145 + Legacy14(#5301 3, #6311 1, #6923 10).
`output/7353/r19/selected-tests.log`, `document-final.log`에 실제 target/filter/결과를 남겼다.
처음 실행은 파생 suite 재배정과 runner의 target 계산 차이로0건 선택되어 실패했다.
생성된 suite의 실제 모듈을 확인하여 재실행했으며0건 실행은 통과 수에 포함하지 않는다.
기존 기대값·ignore·golden·래칫은 변경하지 않았다. 새로운 source-side cfg(test)는 없다.

## 재현·소스 고정

제품 worktree는 `rhwp-task-7353`, 파생 suite를 만드는 검증 overlay는 `rhwp-review-7353`다.
`b504181da` + 이 변경을 `output/7353/r19/source.sha256`으로 고정했다. 신규 파일을 포함한
V2/Rust 검증 소스는 `review-source-match.log`에서 일치하며, 그 외를 포함한 추적 중인
컴파일 입력1,067개도 `whole-review-source-match.log`에서 차이0개다. 생성 suite는 stage하지 않는다.

실제 생성 suite에서18개 case 모듈의 target을 찾고 해당 case명 filter만 nextest로 실행했다.
명령 전체·선택 모듈은 `selected-tests.log` 선두에 있다. HWP 테스트 추가 후 실행한 최종7건은
`regression_suite_015::issue_7353_table_v2_document_flow`이며 이후 manifest 재배정 시 번호는
달라질 수 있다. 외워 둔 suite 번호 대신 실제 생성 파일을 확인해야 한다.

```sh
# review overlay, --target-dir /home/edward/mygithub/rhwp/target/pr-review 공통
cargo clippy --locked -p rhwp --lib --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo clippy --locked -p rhwp --lib --target wasm32-unknown-unknown --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo clippy --locked --test regression_suite_015 --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo fmt --all -- --check
node scripts/rust-test-suite-manifest.mjs --prepare
node scripts/rust-test-suite-manifest.mjs --check --base-ref 7a95e46e025470a4d7a7b59ad68ec02958bda738
```

세 범위 Clippy(33.56/37.08/37.36초), fmt, 고정 base 정책 검사는 PASS다.
로그는 `clippy-native.log`, `clippy-wasm.log`, `clippy-tests.log`, `fmt-final.log`, `policy.log`다.
이는 개발 범위 검증이며 제출 직전 workspace all-target lint나 전체 PR CI를 대신하지 않는다.
개발 초기 제품 worktree의 cargo fmt는 없는 파생 suite 때문에 실패했으므로 review overlay에서
준비·fmt 검사했다. 제품에서 파생 suite를 생성하거나 커밋하지 않았다.

## Docker fresh WASM·직접 시각 확인

```sh
# product worktree
docker compose --env-file .env.docker -p rhwp run --rm wasm
VISUAL_SWEEP_CHROME=/home/edward/.cache/puppeteer/chrome/linux-146.0.7680.31/chrome-linux64/chrome \
  node scripts/verify-table-v2-preview-wasm.mjs --pkg pkg \
  --fixtures output/7353/r19/fixtures --out output/7353/r19/browser \
  --dependencies-root /home/edward/mygithub/rhwp \
  --solid-backgrounds --solid-borders --split-borders --split-line-property \
  --matching-table-borders --nested-alignment --cell-vertical-align --document-flow
node --test scripts/frontend-wasm-bindings.test.mjs
```

Docker 빌드는7분06초에 성공했다(`docker-wasm.log`). WASM SHA256:
`438e05e7027aac6f974cba67cfac15c96f953ad290a3f5af549d58cb4951f209`.
Chrome146.0.7680.31에서 합성 fixture92쪽(기존82 + 문서흐름10) PASS다.
Native/fresh WASM RenderTree·SVG 완전 일치, 독립 좌표·테두리·상태 분리·실패 rollback·종료를
확인했다(`browser-final.log`, `browser/manifest.json`, `pkg.sha256`). frontend bindings도1건 PASS다.

첫 브라우저 실행은 공통 용지 단위 변환의 `2250HU → 30.000000000000004px`를 기대 정수30과
strict equality로 비교하여 실패했다(`browser.log`). 수학적 기대값은 그대로 두고 신규 문서
geometry 검사만 `32 × Number.EPSILON × max(1, |기대값|)` 이내 수치 비교로 바꿨다.
대략10⁻¹²px 이하의 산술 오차이며 위치/쪽수 허용치나 golden 변경이 아니다. 실제 출력은
반올림·clamp하지 않았고 Native/WASM 상호 비교는 여전히 완전 일치 검사를 유지한다.
Rust source는 이 과정에서 바꾸지 않아 WASM을 중복 빌드하지 않았다. 최종 harness 포함 해시는
`source-final.sha256`, 원래 빌드와 동일한 Rust 해시 검사는 `compiled-source-final-check.log`다.

다음10개의 `output/7353/r19/browser/*.review.png`를 직접 열어 확인했다. 각 stem의
`.native.png`, `.wasm.png`, `.overlay.png`도 보존한다.

| stem/페이지(0-based) | 직접 확인한 의미 |
| --- | --- |
| document-split-0,1 | before → A/B/C, 다음 쪽D/E → host/after. 표 외곽·내용과 후속 문단 위치 일치 |
| document-hwp-0,1 | HWP에서도 동일 분할·후속 문단, Native/WASM 겹침·누락 차이 없음 |
| document-atomic-0,1,2 | 앞 두 문단 뒤 잔여 공간에 표를 억지로 넣지 않음. 다음 쪽 전체 표/host, 마지막 after |
| document-nested-0,1,2 | 부모 위쪽 빈 줄18px 보존, 자식 A/B→C/D, 부모 inner/tail, 표 종료 뒤 host/after |

기존82개 review PNG는 모두 이전 r18 출력과 byte-identical이다(`visual-compare.log`).
Native 첫 중첩 페이지는 WASM 빌드 전에 `document-nested-0.png`로도 직접 확인했다.
렌더링 도구는 의존성이 설치된 원래 worktree의 `rasterize-svg-webfonts.mjs`를 사용했다.
이 시각 검증의 독립 근거는 합성 입력의 용지·고정 줄간격·정렬식이다. 실제 #6923의 한컴 PDF
비교를 대신하지 않으며, Studio Canvas/편집 세션 전환 검증도 아니다.

## 다음 의존 작업 — A 미완료

원본 #6923의 저장 줄 유효성/문단 원점, TAC와 문단 줄 소속, rowspan 소유 유닛, 그림·section
부가 출력까지 종단 경로에서 수용해야 A 완료가 된다. 원본을 단순화해 대신 통과시키지 않는다.
선택 표 미리보기만의 기존 형식 지원 확장을 반복하지 않고 이 의존 순서로 이어간다.

## 후속 진행 — 일반 저장 줄의 실제 소비 경로

`978efeab1` 이후 A의 의존 작업을 이어 진행했다. 아래 기록이 저장 줄 수용 범위의 최신
상태이며 A 전체 완료를 뜻하지 않는다. Legacy나 Studio 기본 엔진은 바꾸지 않았다.

- 독립 근거: 원본 #6923 HWP의 실제 LineSeg/문자 오프셋과
  `mydocs/tech/document_ir_lineseg_standard.md`의 줄 시작·줄간격 계약.
  수동 LineSeg 입력은 분할 경계 계약으로만 쓰고 한컴 생성본으로 취급하지 않는다.
- `stored_text::localize`: 컨트롤 없는 단일 세그먼트 줄에서 저장 문자 경계·높이·기준선·
  너비를 그대로 두고 문단 원점만 뺀다. 편집으로 무효화된 partition, 구현용 합성 사다리,
  겹침/내부 페이지 리셋, 폭 변경은 명시적으로 거부한다. 겹침을 페이지 분리로 해석하지 않는다.
- `TextComposer::compose` → 공통 glyph 배치의 `physical_frame_rows` →
  `stored_text::validate_paint`: 후속 배치가 실제 줄 소속·y·높이·기준선·마지막 전진량을
  바꿨는지 검사한다. 다르면 unsupported이며 clamp나 새 측정값으로 덮지 않는다.
- 검증된 같은 RenderNode가 ParagraphItem/FlowCursor의 점유와 TextPaint/DocumentV2의
  실제 출력에 사용된다. 별도 줄 나누기나 표 알고리즘 fallback은 없다.
- 저장 컨트롤 host는 `text_ir`/`document_input`에서 계속 거부한다. 일반 문단 수용을
  TAC·TopAndBottom 저장 앵커 소유의 구현 완료로 확대 해석하지 않는다.

실제 원본에서 일반 텍스트 문단 6개의 줄 소속·높이·vpos 차이를 유지하여 V2 fragment로
출력했다. 테스트에서는 원본 문단을 바꾸지 않고 독립적인 텍스트 probe 셀에 넣는다.
이는 원본 표/문서 전체 통과가 아니다. 나머지 검사 대상 483개는 문단 inset 9개,
장식/keep 등 473개, 기타 provenance/control 1개로 명시적 미수용이다.
원본 전체의 첫 section guard와 TAC/rowspan/그림 등 잔여 경계도 그대로다.

### 집중 계약 및 비교

기존 두 `tests/cases/issue_7353_table_v2_{text,document_flow}.rs`에 3건을 추가했다.

1. 실제 원본 6문단의 문자 소속·저장 높이·상대 원점·입력 불변성.
2. 합성 12HU 줄 상자/6HU 간격/10HU 기준선: 첫 조각21HU(위 패딩3+줄12+간격6),
   다음22HU(줄12+간격6+아래 패딩4). `alpha`/`beta` 누락·중복 없이 종료하고
   겹친 줄·dirty partition·잘못된 문자 경계·합성 tag·변경 폭은 거부한다.
3. 문서 본문 before → 저장 one/two → after의 y=30/48/66/84px, 다음 쪽 next의 y=30px.
   초기 저장 vpos=1000HU는 원본에 남고 본문 원점은 FlowCursor가 소유한다.
   이 합성 입력은 명시적 왼쪽 정렬이며 한컴 저장본의 피델리티 증거가 아니다.

이전 `978efeab1`의 **TextComposer 파일만** review overlay에 대입한 대조에서는 기존7건
PASS, 신규 저장 줄2건 FAIL(원본 수용0개)을 확인했다. 전체 이전 head 재실행이나 기존
Legacy 결함 검출이라고 주장하지 않는다. 변경된 composer 복원 후에는 신규 지원이 통과한다.
`before-text-tests.log`, `text-tests.log`에 경계의 거부→수용을 보존했다.

증적 루트: `output/7353/r19/stored-text/`.
선택 회귀162건(V2 148 + Legacy14) PASS, 최종 문서 계약8건 PASS다. source-side cfg(test),
golden/ignore/기존 기대값은 바꾸지 않았다. Native/WASM library Clippy와 변경 test 대상
Clippy, fmt, 고정 base manifest 검사는 PASS다. 명령은 선행 절과 같은 target-dir/base를 쓴다.
`selected-tests.log`, `document-final.log`, `clippy-{native,wasm,tests}.log`,
`fmt-final.log`, `policy-final.log`, `source-final.sha256`에 기록했다.

테스트 마지막 주석/정렬 fixture 변경 후 파생 suite 배치가 달라져 첫 정책 검사에서 drift가
검출됐다. review overlay에서 `--prepare` 후 고정 base로 재검사하여 통과했다. 파생 파일은
제품 worktree/커밋에 넣지 않는다. 새 시각 fixture의 정렬 명시 후 문서8건도 다시 실행했다.
전체 PR CI/제출용 workspace all-target 검증을 실행한 것으로 간주하지 않는다.

### fresh WASM·시각 확인

Docker WASM 빌드7분44초 성공. WASM SHA256은
`9c76e764936417e1934d77e2c32e630897aebeea68a3334d084a7ddd8520191d`이며
`docker-wasm.log`/`pkg.sha256`에 보존했다. 위 기존 browser 명령에 `--stored-body`를 추가하고
fixtures/out을 이 증적 루트로 지정했다. Chrome146에서94쪽의 Native/WASM RenderTree·SVG
완전 일치 및 독립 좌표 검사가 PASS다(`browser.log`, `browser/manifest.json`).
기존92쪽 review PNG는 모두 이전 r19 결과와 byte-identical(`visual-compare.log`)이다.

새 `browser/document-stored-{0,1}.review.png`와 각각의 standalone overlay를 직접 열었다.
첫 쪽 before/one/two/after의 순서·18px 피치와 다음 쪽 next의 본문 시작 위치를 확인했고
누락·중복·backend 위치 차이는 보이지 않는다. 최초 합성 fixture는 기본 양쪽 정렬이라
짧은 `one`이 폭 전체로 벌어졌으며, 수동 경계 사례의 의도를 왼쪽 정렬로 명시한 뒤
문서 계약/fixture를 다시 산출했다. 엔진 좌표 보정이나 원본 문서 변경은 하지 않았다.
이는 합성 경계와 backend 비교이며 한컴 PDF와 원본 전체의 시각 통과는 아직 미검증이다.

컴파일 입력과 변경 테스트1,089개가 제품/review overlay에서 일치한다
(`review-source-match.log`). frontend bindings1건도 PASS다. Docker 실행 중 추가한
모듈/API 설명 주석은 지원 범위 문구만 갱신했으며 동작 코드는 빌드 시작 후 바꾸지 않았다.
최종 파일 해시 검사는 `source-check.log`에 보존했다.

다음 의존 범위는 저장 컨트롤 host의 TAC 줄 소속과 rowspan/section/그림 등이다.
일반 저장 줄의 제한적 수용을 이유로 원본 #6923의 미지원 guard를 제거하지 않는다.

## A 진행: 저장 TAC 줄의 공동 점유와 본문·중첩 셀 연결

2026-09-25 다음 절편 승인에 따라 같은 A 기록에서 계속한다. Legacy, Studio 기본 경로,
기존 baseline/ignore는 변경하지 않는다. 전체 PR CI나 원격 게시를 실행하는 단계가 아니다.

### 근거·적용 범위와 소비 경로

- 원본 #6923의 본문 문단24(0-based)는 문자 없는 TAC 제어 스트림이다. 저장 줄 폭48,188HU,
  높이23,793HU와 실제 표46,149×23,511HU, 바깥여백 각141HU를 원본에서 직접 검사한다.
  표 높이+위/아래 여백=저장 점유 높이이며 오른쪽 정렬 표 원점은 x=1,898HU다.
  이는 원본 메타데이터의 query 계약이지 자식 내용/원본 전체 조판 통과가 아니다.
- `tac::stored_tac_rows`가 완전한8-unit 제어 스트림과 저장 줄 시작 위치로 소속을 정한다.
  TAC라는 이유로 모든 표를 한 줄로 합치지 않는다. dirty/겹침/불완전 축/변경 폭은 거부한다.
  현재는 문자 없는 줄과 동일 점유 envelope를 가진 표들을 수용한다. 혼합 텍스트,
  서로 다른 ascent/descent의 기준선 합성, 재조판 줄 나누기는 미지원으로 남긴다.
- `tac::compose → ParagraphItem::InlineTables → ir::bind_table 또는 document_input::prepare →
  FlowBlock::InlineTables → FlowCursor::fit → TextPaint/BodyPlan`이 실제 호출 경로다.
  측정과 배치는 같은 줄 높이·표 원점·실제 ControlOwner를 소비한다. 자식 plan이 저장 크기와
  달라지면 `tac::bind`가 거부하며, shrink/clamp/Legacy fallback으로 맞추지 않는다.
- `content::from_grid_rows`는 셀 내부 경계·중복 소유·깊이를 검증한다. `FlowCursor`는 줄 전체
  높이를 한 번 예약하고 모든 자식의 완결된 조각을 지역 결과에 모은 후 함께 수용한다.
  예산이 모자라면 모든 형제를 이월한다. 시작/끝 내용 컷과 자식 continuation은 이 원자적
  줄 경로에 비해당이다. 줄 사이 빈 물리 밴드와 마지막 줄 간격은 별도 Space로 소비한다.
  한 쪽보다 큰 TAC 줄의 내부 분할은 이 제한된 경로에 포함하지 않는다.

### 독립 계약과 미지원 경계

`tests/cases/issue_7353_table_v2_document_flow.rs`에 다음3건을 추가했다.

1. 폭80px·높이36px 표2개, 네 방향 여백2px: 같은 줄 점유는40px이며80px 합산이 아니다.
   남은36px에는 표만 fit해도 여백 포함 줄은 fit하지 않는다. 두 표 모두 다음 쪽
   (x=88/172,y=32)에 배치하고4px 후행 줄간격 뒤 문단은 y=74에 둔다.
2. 별도 저장 줄의 두 표는 서로 다른 쪽에서 해당 소유를 유지한다. 같은 줄을 중첩 셀에
   넣은 경로도 부모 높이44px, 자식 원점과 뒤 문단 y=74를 실제 RenderTree에서 검사한다.
3. 불완전 stream, 제어 내부를 가리키는 줄 시작, 겹친 줄, 변경 폭과 실제 자식 내용 증가를
   거부한다. 마지막 사례는 초기 section guard가 아니라 `TAC content changed stored
   occupied box`라는 정확한 원인으로 실패해야 한다.

위 수동 작성 HWPX는 경계 계약이며 한컴 생성본/피델리티 증거가 아니다. 첫 시도에서
첫 문단에 TAC carrier를 둔 합성 입력은 직렬화기가 넣은 SectionDef/ColumnDef와 섞여
거부됐다(`structural-carrier-unsupported.log`). 이 축은 아직 미지원임을 명시하고,
서로 다른 줄 계약에서는 구조 컨트롤이 있는 선행 일반 문단과 TAC carrier를 분리했다.
원본 #6923의 구조 컨트롤이나 저장 정보를 삭제하여 수용한 것은 아니다.

변경 전035713433의 WASM에서 동일3개 시각 입력은 각각 multiple anchors/nested TAC guard로
거부됐다(`before-wasm.log`, `before-pkg.sha256`). 이는 신규 지원의 거부→수용 증거이며
기존 Legacy 결함 재현이나 한컴 일치를 의미하지 않는다.

증적 루트는 `output/7353/r19/stored-tac/`이다. 최종 검증 결과는 아래에 이어 기록한다.

최종 소스 선택 회귀는168건(V2 151 + Legacy17) PASS다(`selected-final.log`).
Native/WASM library Clippy, 변경 integration target Clippy, fmt와 고정 base
`7a95e46e025470a4d7a7b59ad68ec02958bda738` manifest 정책 검사도 PASS다.
`clippy-{native,wasm,tests}.log`, `fmt.log`, `policy.log`에 명령 결과가 남아 있다.
review overlay의 Rust 입력/manifest/변경 테스트1,068개가 제품 worktree와 일치한다
(`review-source-match.log`). 소스 해시는 `source-final.sha256`/`source-check.log`로 고정한다.
source-side test, golden/ignore 변경은 없다. 전체 PR CI·제출용 workspace all-target lint와
전체 release 회귀를 실행한 것으로 확대하지 않는다.

첫 lint에서 나머지 연산 표현의 `manual_is_multiple_of`가 검출되어 같은 의미의 표준
메서드로 고쳤다. 진행 중인 이전 Docker 빌드를 중단하고 최종 소스로 다시 시작했으며,
이후 선택 회귀·두 library lint를 다시 통과시켰다. 중단 빌드는 성공 증적으로 세지 않는다.

### fresh WASM와 직접 시각 확인

Docker 표준 빌드(`docker compose --env-file .env.docker -p rhwp run --rm wasm`)가7분42초에
완료됐다. 최종 WASM SHA256:
`05eef3074e47501691ad18c74bc7d2efc17c18b47b8053a516a04bc5753b63f8`.
`docker-wasm.log`, `pkg.sha256`, `browser/manifest.json`에 연결했다.

```bash
VISUAL_SWEEP_CHROME=/home/edward/.cache/puppeteer/chrome/linux-146.0.7680.31/chrome-linux64/chrome \
node scripts/verify-table-v2-preview-wasm.mjs --pkg pkg \
  --fixtures output/7353/r19/stored-tac/fixtures --out output/7353/r19/stored-tac/browser \
  --dependencies-root /home/edward/mygithub/rhwp \
  --solid-backgrounds --solid-borders --split-borders --split-line-property \
  --matching-table-borders --nested-alignment --cell-vertical-align \
  --document-flow --stored-body --stored-tac
```

99쪽의 Native/fresh WASM RenderTree·SVG 동일성 및 독립 좌표 검사가 PASS다.
기존94쪽 Native JSON과 review PNG는 이전 stored-text 결과와 모두 동일하다
(`prior-native-compare.log`, `visual-compare.log`). 새5쪽의 review PNG를 전부 직접 열었고,
표가 있는4쪽은 standalone overlay도 직접 확인했다. 형제 표 전체 이월, 별도 줄의 두 쪽
분배, 부모/자식 외곽, 뒤 문단과 종료에서 누락·중복·backend 차이를 보지 못했다.
대표 증적은 `browser/document-inline-1.{review,overlay}.png`,
`browser/document-inline-rows-{0,1}.{review,overlay}.png`,
`browser/document-inline-nested-0.{review,overlay}.png`다.

미지원 첫 문단 TAC 입력을 `fixtures/unsupported-structural-tac.hwpx`로 보존하고 정확한
거부 원인의 회귀 assertion도 추가했다. 이 마지막 테스트 변경 후 문서11건과 해당 target
Clippy를 재실행하여 PASS(`document-final.log`, `clippy-tests.log`), 파생 suite를 다시 준비해
최종 fmt/고정 base 정책 검사도 통과했다. 제품 source/시각 fixture는 변하지 않았다.

**A는 계속 진행 중이다.** 이 결과는 합성 경계의 제한적 지원과 backend 대조이며 한컴 PDF와
#6923 원본 전체 일치가 아니다. 저장 구조 컨트롤 혼재 축, rowspan/그림·section 속성,
일반 TAC 줄 합성과 큰 표 분할이 다음 의존 범위다. Studio 기본값/Legacy/편집 경로는 유지한다.

## A 후속 — 첫 문단 구조 슬롯과 TAC 소유 관계

2026-09-25 후속 절편 승인으로 같은 A 기록에서 진행한다. 첫 문단 구조 컨트롤을 삭제하여
표의 인덱스를 당기지 않고, 원래 HWP 문자 슬롯과 ControlOwner를 보존한다.

- 원본 #6923 p0은 secd/cold/쪽번호/표의4개 슬롯(33유닛)이다. 표 소유는 ci3이며
  선언 높이11102 + 위아래283씩 = 저장 줄11668HU다. 가운데 정렬 원점은
  `(48188 - 47488 - 566)/2 + 283 = 350HU`, y=283이다. 원본 파싱 query로 검사하며,
  음수 줄간격 -800도 관측값 그대로 보존한다. 이것은 원본 전체 렌더링 성공이 아니다.
- 생산: `tac::stored_tac_rows`의 source-only 슬롯 검증/줄 소유 → 소비:
  `tac::compose`의 공통 줄 점유 → `document_input::prepare`의 원래 ci로 자식 plan bind →
  기존 `FlowCursor` 원자적 예약 → 기존 text paint. 새 좌표 덮어쓰기·clamp는 없다.
- 본문 dispatcher가 먼저 첫 문단의 단일 SectionDef/ColumnDef를 검증한다. 구조 컨트롤은
  문자 슬롯을 차지하지만 표의 가로 폭은 차지하지 않는다. 중복·뒤쪽 선언·다단·미지원
  쪽번호를 소비 없이 버리는 경로는 거부한다. 셀 내부 구조 컨트롤 지원은 추가하지 않았다.
- signed 저장 간격은 query에서 관측할 수 있지만, 현재 flow의 음수 전진은 미지원이므로
  compose에서 명시적으로 거부한다. 이 guard를 실제 HWP 문서 세션에서 검사한다.

### 입력과 축 검증에서 발견한 제한

첫 실험의 합성 입력은 직렬화 전 구조 슬롯을 포함하지 않았다. 이후 명시적인 secd/cold
슬롯을 포함한 HWP5 계약 입력을 새로 생성했다. HWP는 raw 두 번째 줄 시작24로 소유 ci3을
보존한다. HWPX 변환은 raw 시작8과 parser shift8로 관측됐고, HWP5 기대 위치24와 다르다.
별도 ctrl/colPr와 secPr 내부 colPr의 축 처리가 같다는 초기 가정이 틀렸다.
파서/직렬화기나 공용 Paragraph accessor는 이번 절편에서 수정하지 않았다.

빈 control-only HWPX에는 실제 글자 offset이 없어 저장 시작이 이미 HWP5 축인지 별도 축인지
확정할 수 없다. 따라서 구조 shift가 있는 여러 줄은 `ambiguous structural TAC character
axis`로 거부한다. 0에서 시작하는 단일 줄은 이 모호성이 없으며 수용한다. 초기 실패 입력
`stored-tac/fixtures/unsupported-structural-tac.hwpx`는 보존하고 새 입력으로 덮어쓰지 않았다.
변환된 다중 줄도 `structural-tac/fixtures/unsupported-structural-axis.hwpx`로 남긴다.
실제 한컴 생성 HWPX 다중 줄과 축 출처 검증은 미완료 의존 항목이다.

### 계약 범위와 증적

기존 `tests/cases/issue_7353_table_v2_document_flow.rs`에2건을 추가하고 원본 query를 확장했다.
합성 입력은 한컴 피델리티 기준이 아니라 다음 독립 기하 계약이다.

- 첫 줄의80×36px 표2개, 바깥여백2px: 점유40px, centered x=88/172,y=32.
  후행4px 간격 뒤 문단 y=74. HWP/HWPX 실제 RenderTree를 대조한다.
- 서로 다른 저장 줄의 HWP 표는 각 쪽 x=130,y=32, 첫 쪽 A/a, 다음 쪽 B/b/after.
  소유 ci2/3, 외곽·후속 문단·종료와 누락/중복을 검사한다. 첫 줄 내부 컷은 비해당이다.
- 구조 슬롯 중복, 다단, 뒤 문단 ColumnDef, 쪽번호, 음수 간격과 잘못된 shift의 명시적 거부.
- 이전 cf0027c24 WASM에서 새 두 양성 입력은 `body control or multiple anchors`로 거부된다.
  `structural-tac/before-wasm.log`, `before-pkg.sha256`에 남겼다. 기존 Legacy 결함이 아니라
  신규 경로 수용 범위 확장의 변경 전/후 증거다.

증적 루트: `output/7353/r19/structural-tac/`. 검증 결과는 아래에 이어 기록한다.

최종 선택 회귀170건(V2 153 + Legacy17)이 PASS다(`selected-final.log`, 실제 suite/filter 명령
포함). Native/WASM library Clippy, 최종 파생 test target `regression_suite_007` Clippy,
fmt와 고정 base `7a95e46e025470a4d7a7b59ad68ec02958bda738` manifest 검사가 PASS다.
`clippy-{native,wasm,tests-final}.log`, `fmt-final.log`, `policy-final.log`에 연결한다.
review overlay 입력1,068개가 제품과 일치한다. 기존99쪽 Native JSON도 동일하다
(`review-source-match.log`, `prior-native-compare.log`). golden/ignore나 Legacy 코드는 바꾸지 않았다.

초기 실패들은 합성 입력의 구조 슬롯/직렬화 여백 mirror 누락, 검사 옵션 누락, 문단 정렬
가정과 HWPX 축 차이에서 발생했다. `document{,-second,-third,-fourth}.log`를 보존하며 이를
기존 엔진 결함의 수정 전 FAIL로 세지 않는다. 실제 변경 전 수용 실패 증거는 위의 동일 입력
cf0027c24 WASM 검사다. 계약 입력의 `common.margin`과 table outer margin은 양쪽 포맷에서
같은 외부 여백을 나타내므로 함께 지정했고 두 번 계상하지 않았다.

새3쪽 Native PNG를 직접 확인했다. 같은 줄의 형제 외곽과 텍스트, 별도 줄의 다음 쪽 이월,
후속 after 문단에 누락·중복·겹침을 보지 못했다. 이 합성 시각 증거는 한컴 원본 일치 판정이
아니며, fresh WASM 비교 결과는 아래에 이어 기록한다. 전체 제출용 CI는 아직 실행하지 않았다.

Docker 표준 빌드는7분31초에 완료됐다(`docker-wasm.log`). WASM SHA256은
`4cbbe08950c04ed0b97ec33f4a21a29bf150f6fd86c956649ca4be837986ac06`이다.
앞 절의 브라우저 명령에서 fixtures/out을 `structural-tac/` 아래로 바꾸고
`--structural-tac`를 추가하여102쪽의 Native/fresh WASM RenderTree·SVG 동일성과
최종 좌표 계약이 PASS했다(`browser.log`, `browser/manifest.json`). 입력별 hash는 manifest,
최종 소스 hash는 `source-final.sha256`/`post-build-source-check.log`에 남겼다.

`browser/document-inline-first-0`, `browser/document-inline-first-rows-0,1`의
review PNG3개와 standalone overlay3개를 직접 열어 확인했다. 표 형제와 텍스트의 같은 줄
배치, 서로 다른 쪽의 소유, 외곽 및 after 문단에서 backend 차이나 누락·중복을 보지 못했다.
이전99쪽 review PNG는 모두 동일하여 기존 직접 판독을 재사용했다(`visual-compare.log`).
fresh WASM의 모호한 HWPX 축 거부도 확인했다(`unsupported-axis-wasm.log`).

**A는 미완료다.** 이번 절편은 첫 문단의 제한된 구조/TAC 수용 경계까지이며 원본 #6923 전체
정답 출력이나 R5 완료가 아니다. 원본 전체의 section 속성·쪽번호·rowspan·그림, 일반 TAC
줄 합성/큰 표 분할, 구조 혼재 HWPX 다중 줄의 독립 출처 확인이 남았다. 파서·직렬화기,
Legacy/Studio 기본값, golden/ignore는 유지했다. 내부 자동 승인 경계를 유지하며 원격 게시나
전체 제출용 CI를 실행한 것으로 보고하지 않는다.

## A 후속 — 세로 병합 셀의 온전한 행 그룹

2026-09-25 후속 승인으로 #6923의 rowspan 의존 범위를 확장한다. Legacy는 변경하지 않는다.
`RowBreak`에서 병합 셀 내부를 임의로 자르거나 같은 내용을 각 행에 복제하지 않고, 병합이
가로지르는 경계들을 연결한 행 그룹을 한 번에 예약한다. `None`은 표 전체를 예약한다.
이것은 **내부 행 높이가 결정되는 온전한 셀 그룹**의 실험 경로이며 일반 rowspan 내부 분할,
한컴과의 모든 행 나눔 호환성을 완료했다는 뜻이 아니다.

### 근거·공통 결과·제한

- 입력 생성: 새 계약은 고정18px 문단과 선언 행 최소 높이를 갖는 합성 IR→HWPX→정상 parser다.
  독립 기대값은 직렬화 전 지정한 행 높이, 여백, 병합 주소, 정렬 불변식에서 정한다.
  #6923 원본은 변경 없이 파싱하여 별도 관측한다. 첫 표의 비병합 행 높이
  2282/3042/1922/3274HU와 병합 높이5324/7246/5196HU의 합 관계를 검사한다.
  그림이나 구조 컨트롤을 제거해 원본 수용으로 보고하지 않는다.
- 생산: `grid::resolve`가 각 행을 덮는 원래 셀의 격자를 검사하고 시작 주소에만 track을 만든다.
  `TableContentPlan::from_grid_rows`는 비병합 셀의 내용·최소 높이로 행 경계를 결정한다.
  병합 셀 전체 높이는 해당 행들의 합이며 전체 상자에서 Top/Center/Bottom 여백을 산출한다.
- 요구/예약/이월: `fragment::fit_rows`→`row_groups::fit_row_groups`가 연결된 그룹의 전체
  요구 높이를 비교한 뒤 같은 행 높이로 셀 배치를 만든다. 부족하면 그룹 전체를 이월한다.
  각 원래 셀의 `FlowCursor`는 한 번만 실행하며, 소비되지 않은 유닛이 남으면 오류다.
  누적 예약 높이는 수용한 그룹의 합이고 후속 문단은 그 끝에서 시작한다.
- 실제 배치: `text::render`는 원래 row/column/row_span과 최종 bounds를 소비한다.
  `CellBorders::append`는 실제 보이는 행 순서와 span 끝 경계를 사용해 T자 공유선을 합친다.
  반복 제목 뒤 source row가 건너뛰어도 같은 물리 경계를 사용한다. 좌표 clamp/별도 paint
  높이 확장은 없다. 기존 비병합/WithinCells 경로는 별도 경로로 유지하고 선택 회귀로 대조한다.
- 행 경계가 비병합 셀로 결정되지 않는 경우, 병합 내용에 맞춰 임의의 행을 늘려야 하는 경우,
  rowspan의 `CellBreak`, 제목/본문 경계를 관통하는 병합은 명시적 미지원이다.
  이번 온전한 그룹에는 시작/끝 내용 컷이 없고, padding/최소 높이 밴드는 한 번 계상한다.
  일반 셀 내부 컷·캡션·각주·그림은 이번 신규 지원의 적용 대상이 아니다.

### 검증

`tests/cases/issue_7353_table_v2_rowspan.rs`의 정식 계약은 연결된 두 병합의 그룹 이월,
여러 병합이 같은 경계에서 끝나는 경우, 예산107.5/108 및125/126px 경계, 원래 셀 소유,
T자 테두리, 세 정렬과 여백, 반복되는 병합 제목, 병합 셀 내부 자식 표, 부모 안의 그룹 분할과
host/tail, 종료 후 추가 페이지 없음, 불명확한 높이/잘못된 격자의 거부를 검사한다.
실물 관측 계약을 합성 최종 출력 계약과 분리했다.

증적 루트: `output/7353/r19/rowspan/`. 변경 전 `171c70d5a` WASM은 같은 새 입력을
`Unsupported("cell address or span")`으로 거부했다(`before-wasm.log`, `before-pkg.sha256`).
이는 Legacy 결함 재현이 아니라 V2의 기존 미지원→신규 수용 증거다.
초기 테스트 실패는 용지를 넘는 테스트 본문 높이, 단위 변환1ULP 비교, parser가 zero rowspan을
1로 정규화하는 부정 입력 기대값에서 발생했고 해당 로그를 보존했다. 제품 출력은 반올림하지
않으며 유리수 좌표 기대값 비교에 기존 방식의8ULP만 적용한다. Native 렌더의 병합 외곽과
정렬을 직접 열어 확인했다. 최종 회귀/lint/fresh WASM 결과는 아래에 기록한다.

선택 회귀180건(V2 163 + Legacy17)이 통과했다(`selected-final.log`, 실행 명령 포함).
Native/WASM library Clippy, 변경 test target Clippy, fmt, 고정 base
`7a95e46e025470a4d7a7b59ad68ec02958bda738` manifest 검사가 통과했다.
`clippy-native-final.log`, `clippy-wasm.log`, `clippy-tests-transport.log`, `fmt-final.log`,
`policy-final.log`에 연결한다. 초기 Clippy의 `manual_contains`는 동일 의미의 `contains`
호출로 수정했다. review overlay와 제품 `src/` 차이는 없다(`review-source-diff.log`).

Docker 표준 WASM 빌드는7분40초에 성공했다(`docker-wasm.log`). WASM SHA256은
`eb47e18b5efbf82248e548eec8ec60a734ecffc7f9ebd5f78df89369f9e14017`이다.
제품 소스는 빌드 전 고정했고 변경하지 않았다(`source-final.sha256`,
`post-build-source-check.log`). 최종 commit은 `verified-commit.txt`에 연결한다.

첫 브라우저 비교에서 Native 증적의120과 WASM의120.00000000000001 차이가 검출됐다.
엔진의 원문 JSON은 둘 다120.00000000000001였고, Native 테스트의 `serde_json::Value`
재파싱/재직렬화가1ULP를 잃는 원인이었다. 이전 증적과 실패 로그를 보존하고
(`rowspan-groups-reencoded.native.json`, `browser-reencoded-native.log`,
`transport-roundtrip.log`), 새 테스트 helper가 `next_page_json` 원문을 보존하도록 수정했다.
렌더러나 비교 허용치는 바꾸지 않았다. 해당10건과 변경 target Clippy·fmt·정책 검사를 다시
통과했다(`rowspan-transport.log`). 제품 소스/입력이 같아 Docker 재빌드와 전체 선택 회귀
중복 실행은 하지 않았다.

브라우저 명령은 앞 절과 같은 모든 옵션에 `--rowspan`을 추가하고 fixtures/out을
`output/7353/r19/rowspan/` 아래로 지정했다.115쪽의 **정확한** Native/fresh WASM
RenderTree·SVG 동일성과 독립 좌표/소유 계약이 PASS했다(`browser.log`, `browser/manifest.json`).
기존102쪽 Native JSON과 review PNG는 모두 동일하다(`prior-native-compare.log`,
`visual-compare.log`). 새13쪽의 standalone overlay 전체와 대표4개 review PNG
(`rowspan-groups-1`, `rowspan-align-0`, `rowspan-nested-2`, `rowspan-spanning-header-1`)를
직접 열어 병합 외곽/T자 경계, 정렬, 제목/본문 소유, host/tail과 종료를 확인했다.
합성 시각 계약의 범위에서 누락·중복·backend 차이를 보지 못했다. 한컴 PDF 일치 판정은 아니다.

**A와 R5는 미완료다.** #6923 원본577문단 전체는 아직 section/쪽번호·그림·저장 앵커 등의
미지원 경계에 걸린다. 다음 종단 의존 범위에서 이를 다루며 원본을 축소해 완료로 바꾸지 않는다.
rowspan 내부 컷과 불명확한 높이 배분도 남긴다. Legacy/Studio 기본값·golden/ignore를 유지했고,
전체 제출용 CI·원격 push·PR은 실행하지 않았다.

## A 후속 — 장식 없는 쪽 테두리 레코드 수용

2026-09-25 후속 승인으로 #6923 원본의 첫 거부 조건을 추적했다. 원본 구역에는 기본/추가2개의
`PageBorderFill`이 있고 모두 `border_fill_id=0`, 간격1417HU, attr1이다. 기존 V2는 추가 레코드가
있다는 이유만으로 거부했다. 레코드의 존재와 실제 장식 참조를 분리하여 세 레코드 모두 ID0이면
수용한다. 원본 레코드·간격·제어 슬롯은 삭제하지 않고, 어느 레코드든 비영 참조이면 계속 미지원이다.

근거는 HWP5 사양 표135의 **테두리 위치 간격과 테두리/배경 ID의 구분**, 원본의 저장값과
기준 PDF에 쪽 테두리가 없다는 관측이다. 간격을 본문 여백으로 취급하지 않는다. 생산 값은
원본 `PageDef`→`PageLayoutInfo.body_area`이며, 측정은 `BodyPlan.flow`→`FlowCursor.fit`,
실제 배치는 동일 fit의 bounds→`DocumentV2Session.next_page_json`이다. 이 경로의 원점·
행 높이·내용 컷·예약·paint는 변경하지 않는다. 장식 수용 조건만 바뀌므로 새로운 분할 알고리즘,
rowspan 내부 컷, 각주/캡션 지원은 이번 적용 범위가 아니다.

정식 `issue_7353_table_v2_document_flow` 계약에 다음을 추가했다.

- 장식 없음 레코드의 간격을 용지보다 크게 지정한 합성 HWP/HWPX에서도 앞 문단·2쪽 표 분할·
  host/뒤 문단의 **전체 RenderTree와 SVG가 무장식 대조군과 동일**해야 한다. 레코드2개와
  간격30000HU가 파싱 후에도 보존되는지 별도로 검사한다. 구현 반환 높이가 기대값이 아니다.
- 기본/추가 첫째/추가 둘째 레코드 각각에 실제 장식을 지정한 HWP/HWPX6경우는 거부한다.
- #6923 원본의 577문단 등 기존 관측을 유지하며, 구역 검사 다음 원래 문단0의 쪽번호 컨트롤에서
  거부되는 것을 고정한다. 원본 수용이나 한컴 일치로 보고하지 않는다. PDF1쪽에 `- 1 -`이
  실제 존재하므로 이 컨트롤을 지워 수용시키지 않는다.

증적은 `output/7353/r19/section/`에 보존한다. `before-prepared.log`는 수정 전14 PASS/1 FAIL,
실패 원인은 새 ID0 계약의 `Unsupported("section decoration, grid or writing direction")`다.
처음 `before.log`는 manifest 미준비로 테스트0건이므로 결함 재현 증거에서 제외했다.
수정 후 병렬 링크는 디스크 부족(os error28/ld Bus error)으로 실패했다(`after.log`). 가용3.8GiB에서
캐시 삭제 없이 `CARGO_BUILD_JOBS=1`로 실행한 `after-serial.log`는15 PASS다. 환경 실패를
제품 회귀로 분류하지 않는다. 기존 캐시·다른 작업·baseline/ignore는 변경하지 않았다.

Native 첫 쪽을 직접 열어 앞 문단과 표 조각의 위치·외곽을 확인했다. 기존 본문/분할 불변식의
합성 계약 검증이지 실물 전체 피델리티 판정이 아니다. 최종 lint와 fresh WASM 증거는 아래에 잇는다.

이번 실행의 Native 검증은 본문/수용15건과 Legacy 대조17건, 총32 PASS다(`after-serial.log`,
`legacy.log`). 이전180건 전체를 재실행한 것으로 합산하지 않는다. Native/WASM library와 변경
integration target Clippy, fmt, 고정 base manifest 검사가 통과했다(`clippy-native.log`,
`clippy-wasm.log`, `clippy-test.log`, `fmt.log`, `policy.log`). 생성 suite는 review overlay에서만
준비했으며 제품 소스와 overlay의 `src/`는 동일하다. 추가 PDF 직접 관측은 `oracle-1.png`와
`oracle-1-bbox.html`에 남겼다. 쪽번호의 표시 범위는 x283.286..311.576pt,
y788.592..798.532pt다. 이는 후속 쪽번호 구현의 독립 관측이며 현재 V2 출력 값이 아니다.

Docker fresh WASM은7분19초에 완료했다(`docker-wasm.log`). SHA256은
`edaf41a0c6d5242db35cba6d39b64d34cc26007896b712d3459194810edb18d1`이다.
기존 browser 명령의 fixtures/out을 `section/`으로 바꾸고 `--empty-page-borders`를 추가했다.
119쪽의 정확한 Native/WASM tree·SVG 대조가 통과했다(`browser.log`, `browser/manifest.json`).
변경된 본문 경로의 Native 증적은 이번15건에서 새로 생성했고, 변경하지 않은 선택 표 경로는
앞 절편의 증적을 재사용해 새 WASM과 비교했다. 기존115쪽 review PNG가 모두 동일하다
(`prior-visual-compare.log`). 전체 Native 회귀119건을 새로 실행했다는 뜻이 아니다.

`browser/document-empty-page-borders-{0,1}.review.png`와
`browser/document-empty-page-borders-hwp-{0,1}.overlay.png`를 직접 열어4쪽의 외곽/표 조각,
앞뒤 문단, 누락·중복 없음과 backend 일치를 확인했다. 각 파일의 `.native.png`, `.wasm.png`,
`.overlay.png`, `.review.png`가 같은 폴더에 있다. 합성 계약 대조이며 한컴 시각 일치율은
계산하지 않았다. `source-final.sha256`/`post-build-source-check.log`로 빌드 후 제품 소스·
테스트·하네스의 불변을 확인했고, 최종 커밋은 `verified-commit.txt`에 연결한다.

A/R5는 계속 진행 중이다. 다음 실제 원본 차단은 쪽번호 story이며 그림·저장 앵커와 기타
표 경계도 남았다. Legacy/Studio 기본값·golden/ignore는 유지한다. 전체 제출 CI·원격 push·PR은
이번 내부 절편에 포함하지 않았다. 디스크 여유가 약3.8GiB이므로 후속 큰 빌드 전 공간 상태를
확인한다. 다른 작업 캐시의 정리는 별도 승인 없이 수행하지 않는다.

## A 후속 — 원본 쪽번호와 본문 흐름 분리

2026-09-25 후속 승인으로 쪽번호를 본문/표와 다른 page story로 연결했다. `PageNumberPos`는
문단에 선언되지만 글줄의 너비·본문 높이·내용 컷을 소비하지 않는다. 첫 문단의 단일 선언,
십진 숫자, 하단 좌/중앙/우 또는 숨김만 수용한다. 상단·안/바깥·다른 형식·중간 재선언·
중첩 셀 선언은 계속 명시적으로 거부한다. Legacy 경로와 Studio 기본값은 변경하지 않았다.

독립 근거는 로컬 HWP5 사양 표147/148의 위치/장식 구분과 #6923 원본 및 대응 PDF다.
앞 절편의 `section/oracle-1-bbox.html`에서 쪽번호 잉크 범위는 y788.592..798.532pt였다.
원본 PageDef로 계산한 본문 하단771.01pt, 꼬리말 거리28.35pt, 자동 번호10pt로부터
run top=771.01+28.35/2+10/3=788.5183pt, baseline=798.5183pt를 사용한다. 글리프의 잉크
bbox와 run bbox를 동일시하지 않는다. 이 관측은 무장식 하단 번호의 근거이며, 원본 문서
전체 V2/PDF 일치 판정이 아니다. 일반 글자 스타일과 번호 문자열/폭 계산 primitive만
재사용하며 Legacy 표 배치/분할 알고리즘은 호출하지 않는다.

값 소비 경로는 `document_input.rs`의 선언 검증→`PageNumberStory::new`의 번호/용지
스냅샷→`page_number.rs::render`의 같은 TextStyle 기반 폭/원점→`document.rs`의
Body 다음 형제 노드→RenderTree/SVG다. 본문 FlowCursor/fit 결과를 수정하지 않으며,
TextLine bbox는 실제 TextRun을 감싼다. 번호·물리 용지 범위 오류 시 다음 페이지 cursor를
commit하지 않는다. TAC 경로는 `stored_tac_rows`가 원래 control index를 유지하고 기존
줄 소속/예약/배치 결과를 그대로 소비한다. 이번에는 분할 컷/rowspan/패딩 계상 규칙을
변경하지 않았다. 분할 영향 여부는 번호 없는 대조군의 완전한 Body 노드로 비교했다.

정식 테스트 `issue_7353_table_v2_page_number`의6건은 HWP/HWPX 하단3정렬, 2쪽 표 분할
뒤 번호 증가, HWP 시작9/앞뒤 장식, 숨김, 잘못된 선언, 번호 overflow와 clone/retry,
DPI72/144와 물리 용지 밖 번호 실패의 비소비를 검사한다. 합성 입력의 body는
(20,30,300,72)px이며 footer run top=102+15+40/9, 좌/중앙/우 anchor=20/170/320px다.
본문 분할 높이는18px 줄의 독립 기대값54+36px다. HWPX pageNum 저장기는 prefix/suffix를
쓰지 않고 sideChar만 쓰므로 앞뒤 장식 계약은 HWP로 검증한다. 직렬화가 제거한 속성을
V2가 수용/거부했다고 주장하지 않는다. 저장기는 이번 범위에서 변경하지 않았다.

`issue_7353_table_v2_document_flow`의 추가 계약은 secd/cold/쪽번호/TAC 두 줄을 가진
HWP의 표 소유 슬롯3/4, 기존 좌표/외곽/전체 Body 보존, 뒤 문단과 번호 증가를 검사한다.
비교 대조군에는 삽입된 제어로 이동한 표 control index만+1을 적용하며 좌표나 내용은
정규화하지 않는다. 총16건이 통과했다. 원본 #6923의 다음 거부는 예상했던 음수 간격보다
앞선 저장 줄 폭 검증이다: PageDef 본문48190HU와 LineSeg48188HU가 다르다. 두 원본 값과
명시적 미지원 결과를 검사하고, 음수 줄간격-800HU도 후속 범위로 남긴다. tolerance를 늘리거나
저장 정보를 삭제하지 않았다. A/R5 완료는 아직 아니다.

증적 폴더는 `output/7353/r19/page-number/`다. `before.log`의 기존 구현은 신규 최초4건 중
1 PASS/3 FAIL이며 PNP 명시적 거부가 실패 원인이다. 새 지원의 전후 증거이지 Legacy 결함
수정이라고 주장하지 않는다. `after.log`의2실패는 위 HWPX 장식 저장 차이를 사용한 잘못된
시험 입력이었고 수정했다. suite 재분배 후 prepare를 생략한 `after2.log`/`document.log`는
0건 실행으로 증거에서 제외했다. 준비 후 `number-final.log`6 PASS와 `document-final.log`
16 PASS가 최종 계약 결과다. `document-prepared.log`에서 검출한 소유 index 차이는 정상적인
원본 슬롯 이동으로 기대값을 수정했고, 실제 원본의 다음 거부를 폭 계약으로 바로잡았다.

Native 초기 출력 `native-number-0.png`를 직접 열어 본문 아래 분리된 번호와 여백을 확인했다.
이는 합성 경계의 직접 판독이며 한컴 실물 시각 통과로 승격하지 않는다. 최종 lint/Legacy
대조/fresh WASM 결과와 source hash는 아래에 이어 기록한다.

이번 head의 Native 실행은 쪽번호6+본문16+Legacy17=39 PASS다. Legacy 사례 목록과 실제
파생 suite 명령은 `legacy.log`에 있다. Native library/WASM library/변경 integration
suite Clippy 및 fmt, base `7a95e46e025470a4d7a7b59ad68ec02958bda738` 대비 manifest 정책
검사가 통과했다(`clippy-{native,wasm,test}.log`, `fmt.log`, `policy.log`). source-side
unit test는 변경하지 않았다. 전체 workspace/all-targets 제출 게이트 실행으로 보고하지 않는다.
검증 overlay와 제품 소스는 동일하며 파생 suite는 제품 PR 소스에 포함하지 않는다.

Docker 명령 `docker compose --env-file .env.docker -p rhwp run --rm wasm`은7분22초에
완료했다(`docker-wasm.log`). fresh WASM SHA256은
`03cc43f856b4ed5d2e3eedaf52ec3e6c49ba6a91e7fb7c04f2a6c78f352fef35`이다.
앞 절편 browser 명령의 fixtures/out을 `page-number/`으로 바꾸고 `--page-numbers`를
추가하여137쪽의 정확한 Native/WASM tree·SVG 대조가 통과했다(`browser.log`,
`browser/manifest.json`). 이번 새 Native 입력18쪽과 재생성한 본문 사례를 사용했고,
수정하지 않은 선택 표 경로의 이전 Native 증거는 재사용했다.137개 Native 회귀 테스트를
새로 실행했다는 뜻이 아니다. 기존119쪽 review PNG는 모두 동일하다
(`prior-visual-compare.log`). 실패한 중간 장식 HWPX 산출은 최종 manifest에 포함하지 않았다.

`browser/number-contact-{0,1,2}.png`에서 신규18쪽의 standalone overlay를 직접 확인했다.
좌/중앙/우 번호가 본문 아래에 분리되고 1→2, [9]→[10]으로 증가하며 숨김 입력에는 번호가 없다.
TAC 두 줄의 쪽 이월에서도 외곽·내용·뒤 문단이 유지된다.
`document-number-tac-hwp-1.review.png`와 `document-number-start-hwp-1.review.png`의
Native/fresh WASM/overlay3면도 직접 열어 위치와 표시를 대조했다. 원본 PDF에 대한 전체
시각 통과는 아니며 합성 계약과 backend 일치에 한정한다. 각 원본 `.native.png`, `.wasm.png`,
`.overlay.png`, `.review.png`도 같은 폴더에 보존했다.

검증 출발 HEAD는 `f2fdd5bb7`이며 검증한 변경 소스·정식 테스트·하네스는
`source-final.sha256`으로 고정했다. `post-build-source-check.log`가 빌드 이후 불변을 확인하고
최종 내부 커밋은 `verified-commit.txt`에 연결한다. 원본 HWP/PDF 해시는
`original-inputs.sha256`에 있다. Legacy/Studio 기본값·golden/ignore를 유지했으며 전체 제출
CI·원격 push·PR은 실행하지 않았다. 다음 A 의존 작업은 저장 폭의 출처와 수용 계약, 음수
줄간격 및 원본의 그림/앵커 등이다. 기본 엔진 전환과 R5 완료 판정은 여전히 남았다.

## A 후속 — 저장 TAC 줄 영역과 signed advance

2026-09-25 후속 승인으로 저장 줄의 정렬 영역과 음수 줄간격을 실험 V2에 연결했다.
HWP5 사양4.3.4/표62의 단 시작·줄 폭·signed 줄간격은 별도 필드다. 원본 #6923의
첫 줄은 본문48190HU 안의 줄 폭48188HU, 점유11668HU, 줄간격-800HU이며 다음 빈
문단의 vpos10868HU, 그다음 문단의 vpos12068HU다. 원본 덤프는
`output/7353/r19/6923-exploratory-dump.txt`다. 2HU 차이의 생성 원인을 반올림으로
단정하지 않고, 저장 영역이 가용 영역 안에 있을 때 저장 폭/시작 위치로 정렬한다.
원본/저장 메트릭을 삭제하거나 수치 tolerance를 확대하지 않는다.

`tac.rs::stored_tac_rows`의 줄 소유/정렬 query → `compose`의 공통
`ParagraphItem::InlineTables { height, advance, tables }` → 본문 `document_input.rs`와
셀 `ir.rs`의 같은 `FlowBlock`으로 연결했다. `height`는 온전한 외곽여백 포함 점유,
`advance`는 다음 원점이다. `content.rs::physical_extent`는 advance로 다음 원점을
진행하면서 모든 점유 끝점의 최댓값을 구한다. `flow.rs::FlowCursor::fit`은 실제 줄/표를
그 원점에 배치하고 온전한 height가 fit할 때만 수용한다. `fragment.rs::fit`은
`fit.height`를 셀/조각 예약에 사용하고 `document.rs::next_page`도 같은 결과를 소비한다.
paint에서 높이를 다시 늘리거나 별도 앵커로 덮어쓰지 않는다.

내용 컷은 여전히 source control/line owner와 block cursor다. 줄 상자 겹침을 줄 소속이나
페이지 경계로 해석하지 않는다. 원점 역행/정지, 저장 영역 밖 위치/폭, 내용 폭 초과,
불완전 슬롯/dirty cache, 측정된 자식 박스와 저장 박스 불일치는 계속 거부한다.
빈 TAC 줄의 음수 advance, 텍스트 혼재/서로 다른 기준선, oversized TAC 내부 분할은
아직 미지원이다. positive gap과 셀 padding은 물리 Space로 유지한다. 제목 재표시,
rowspan 완결 그룹, 일반 셀 분할은 기존 TableCursor를 통과하므로 대조군으로 확인한다.
새 signed TAC와 rowspan/제목의 모든 조합을 직접 검증했다는 뜻은 아니다.

정식 `tests/cases/issue_7353_table_v2_document_flow.rs`에 저장 영역3정렬,
advance39px만 들어가지만 점유40px는 안 들어가는 페이지 예산, 중첩 셀 끝의40px 보존,
서로 다른 두 저장 줄의39px 원점 간격을 추가했다. 합성 계약의 독립 기대값은
80px 자식 너비+좌우2px 여백, 두18px 텍스트 줄+상하2px 여백이다. 실제 Table/TableCell,
TextLine 좌표와 뒤 문단, 유닛 순서/종료를 검사한다. 입력은 명시한 synthetic HWP/HWPX이며
한컴 생성본으로 주장하지 않는다. 원본 query도 본문48190HU로 호출할 때 x350HU를
보존하며, 실제 전체 문서는 문단0의 `V2 source decoration effect`에서 명시적으로 멈춘다.
기존 폭/간격 제한을 넘은 것이지 원본 전체 조판 완료나 PDF 일치 판정은 아니다.

증적은 `output/7353/r19/tac-metrics/`다. `before.log`의 신규 최초3건은 수정 전
의도한 수용 제한으로 FAIL, 수정 후 PASS다. `after.log`의 이전 거부 계약2건은
새 지원 범위에 맞춰 물리 역행/비전진 경계로 교체했다. inter-row 합성 입력의 첫 실패는
HWPX 구조 슬롯 축 모호성, 다음 실패는 footer를 빠뜨린 시험 용지 예산이었다. 수용 조건을
완화하지 않고 HWP 원본 슬롯을 명시하고 용지 계산200-30-20-30=120px로 바로잡았다.
`document-final2.log`20 PASS, `focused.log`75 PASS(이 중 Legacy 대조17건)가 이번
Native 결과다. 합계95건이며 중복 이전 실행을 더하지 않는다.

`native-nested.png`와 `native-rows.png`를 직접 열어 자식 외곽/부모 높이/뒤 문단과
연속 두 TAC 행의 구분을 확인한 뒤 fresh Docker WASM을 시작했다. 기존 Legacy/default
Studio 경로·golden/ignore는 변경하지 않았다. 최종 lint·fresh WASM/시각 결과는 아래에
이어 기록한다. 출발 HEAD는 `7f463eeff`다.

공통 FlowCursor 영향 범위를 넓혀 V2 정식 계약17파일 전체176건도 실행했고 모두 통과했다
(`v2-all.log`). 최종 중복 제외 집계는 V2 176+Legacy 17=193 PASS다. 이 실행에서 Native
fixture도 다시 생성했으며 앞 절편의73개 Native JSON은 모두 byte 동일하다
(`prior-native-compare.log`). Native library/WASM library/변경 integration target의
Clippy, fmt, base `7a95e46e025470a4d7a7b59ad68ec02958bda738` 대비 manifest 검사도
통과했다(`clippy-{native,wasm,test}.log`, `fmt.log`, `policy.log`). 전체 workspace
제출 게이트 통과라고 보고하지 않는다. source-side unit test는 변경하지 않았다.
`source-final.sha256`과 `source-check.log`로 빌드 입력 불변을 확인했다.

Docker `docker compose --env-file .env.docker -p rhwp run --rm wasm`은7분23초에 완료했다
(`docker-wasm.log`). WASM SHA256은
`9fd24f8241994797fe690f17973e85db9fea896a4113e60adf8c7ed48b91b912`다.
browser 명령은 앞 절편의 fixtures/out을 `tac-metrics/`로 바꾸고 `--tac-metrics`를
추가했다. 최초 `browser.log`는 새 HWP body 높이120.00000000000001을 Native 증적의
serde_json Value 재직렬화가120으로 만든 차이를 검출했다. 허용치를 늘리지 않고 HWP
export에 renderer 원문 JSON을 보존하도록 테스트 helper만 수정했다. `document-raw.log`
20 PASS, `clippy-test-final.log`/`fmt-final.log`/`policy-final.log` PASS로 재검증했다.
제품 Rust 소스는 빌드 후 변경하지 않았으므로 같은 fresh WASM을 사용했다.
중간 Chrome 프로세스 시작 실패는 `browser-final.log`에 보존했고, 재실행
`browser-retry.log`와 `browser/manifest.json`에서144쪽의 정확한 tree·SVG 대조가 통과했다.

`browser/tac-contact.png`의 새7쪽 standalone overlay를 직접 열어 저장 영역3정렬,
예산 부족 시 두 자식의 동반 이월, 부모 셀의 전체 외곽과 뒤 문단,39px 원점 간격의
두 TAC 행을 확인했다. `document-inline-signed-budget-1.review.png`의 Native/fresh
WASM/overlay3면도 직접 대조했다. 각 `.native.png`, `.wasm.png`, `.overlay.png`,
`.review.png`는 같은 폴더에 있다. 기존137쪽 review PNG는 모두 동일하다
(`prior-visual-compare.log`). 합성 경계 및 backend 일치 증거이며 원본 #6923의
한컴 PDF 시각 일치로 승격하지 않는다.

최종 검증 파일은 `verified-source.sha256`, 빌드 직후 소스 확인은
`post-build-source-check.log`, 원본 입력은 `original-inputs.sha256`, 최종 내부 커밋은
`verified-commit.txt`에 연결한다. A/R5는 진행 중이며 원본의 글자 장식·그림/앵커와
미지원 표/문단 조합, 전체 문서 PDF 검증이 남았다. 기본 엔진 전환·전체 제출 CI·
원격 push·PR은 실행하지 않았다.

### R19 후속: 온전한 셀/표의 선형 그라데이션과 비활성 대각선 속성

출발 HEAD `d02014671`. 이전의 “글자 장식 효과” 설명을 정정한다.
`V2 source decoration effect`는 표/셀 BorderFill 검사다. #6923 원본 첫 표의 첫 셀은
BorderFill35의 선형 그라데이션(type1, angle90, step50, center50, 검정→0xEFEFEF)을
참조한다. 다른 셀의 attr0/diagonal_type1은 대각선 활성화가 아니라 저장된 선 종류다.
HWP5 표23/24의 활성 속성과 표28~30의 배경 정보를 구별한다. 기준 PDF
`tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame-2020.pdf` 1쪽을
`output/7353/r19/gradient/reference-p1.png`로 렌더링해 상단 띠의 좌검정→우밝음 방향을
직접 확인했다. 원본 전체 출력 일치 증거는 아니다.

공통 결과 경로는 `decoration::validate_source`의 원본 payload 검증 → 기존
`style_resolver::resolve_single_border_style`/`expand_gradient_steps`의 색/stop 결과 →
`text_ir::bind_paint`의 Background 보존 → `TextPaint::build_node`가 전달한 최종
TablePlacement/CellPlacement bounds → RectangleNode → 기존 SVG 선형 gradient 축이다.
측정/fit/cut/cursor에는 배경을 입력하지 않으므로 높이/원점을 재계산하거나 덮어쓰지 않는다.
기존 선형 축의 angle0 위쪽/90 오른쪽 근거는 #6845 기준 PDF 기록을 재사용하며, 새 V2
출력의 직사각형 좌표·색·stop·실제 raster 방향을 별도로 검증한다.

셀은 None/RowBreak처럼 셀 전체가 유지되는 정책, 표 전체 배경은 None 정책에서만 허용한다.
CellBreak 셀 및 분할 표의 gradient restart/이어받기는 미검증이므로 명시적으로 거부한다.
반복 제목과 colspan은 새 geometry를 만들지 않고 매번 수용된 온전한 셀의 최종 bounds를 쓴다.
활성 대각선/그림/패턴/부분 투명도, radial/conical/rectangular gradient는 계속 미지원이다.
원본 attribute 또는 저장 줄을 삭제해 수용시키지 않았다. Legacy/default Studio는 불변이다.

정식 `tests/cases/issue_7353_table_v2_export.rs`에는 두 방향과 원본 gradient payload를
옮긴 합성 row fixture, 반복 제목/colspan, 온전한 표 전체 배경, 비활성 diagonal pen,
잘못된 stops/종류/step/투명도와 분할 정책의 거부를 추가했다. 원본의 payload를 옮긴
합성 표는 한컴 생성 문서가 아니다. 고정18px 두 줄의 셀/표 좌표, 텍스트 순서, 페이지
종료와 실제 Rectangle bounds를 독립 기하 계약으로 확인한다. `before.log` 신규2건은
기존 source/resolved decoration 거부로 FAIL, `after.log`는14 PASS다.

`v2-all.log`에서178건 중177 PASS, 원본 admission 계약1건만 예상 오류 문구가 바뀌어
실패했다. 실제 다음 거부는 문단0의 `stored text requires intact single-segment rows`다.
진단 계약을 실제 경계로 갱신하되 원본은 UNSUPPORTED로 남긴다. 페이지수/golden/ignore의
허용 범위를 바꾼 것이 아니며 원본 전체 조판/PDF 비교 및 A/R5 완료를 주장하지 않는다.
최종 회귀·lint·Docker fresh WASM·시각 결과는 아래에 이어 기록한다.

최종 `final-tests.log`는196 PASS(V2 179+Legacy 17)다. fmt, Native library/WASM library/
변경 integration target Clippy 및 고정 base `7a95e46e025470a4d7a7b59ad68ec02958bda738`
대비 manifest 검사가 통과했다(`fmt.log`, `clippy-{native,wasm,test}.log`, `policy.log`).
전체 workspace 제출 CI를 실행했다는 뜻은 아니다. source-side unit test는 변경하지 않았다.
재생성한 기존 Native JSON78개는 byte 동일하다(`prior-native-compare.log`); 이전의
미사용 `document-number-start.native.json`1개는 이번 생성 대상이 아니며 HWP 대조군으로
대체된 과거 증적이다. 검증 대상은 browser 명령의 명시적 fixture 목록으로 고정한다.

Docker WASM은7분26초에 완료(`docker-wasm.log`), SHA256은
`d03aca0e00d71a62c648d2a5be6b18b1612d768f99b72ebb1e12e1f13dff6355`다.
`source-final.sha256`/`post-build-source-check.log`는 두 변경 제품 소스의 빌드 입력 불변,
`original-inputs.sha256`는 원본 HWP/PDF를 고정한다. browser 명령은 이전 절편의
fixtures/out을 `output/7353/r19/gradient/` 아래로 바꾸고 `--linear-backgrounds`를
추가했다. `browser.log`/`browser/manifest.json`:153쪽 정확한 Native/fresh WASM
tree·SVG 대조 PASS. 기존144쪽 review PNG도 byte 동일(`prior-visual-compare.log`).

새9쪽 `browser/gradient-contact.png`의 standalone overlay를 직접 열어 수평/수직 색축,
50단계 띠, 반복 제목/뒤 행의 셀 경계, 표 전체 배경, 비활성 대각선의 선 없음과 두 쪽
내용을 확인했다. `gradient-stepped-0.review.png`의 Native/fresh WASM/overlay3면도
직접 비교했다. 검정 텍스트/검정 시작 배경의 낮은 명암은 합성 입력 그대로이며 이번
검증은 텍스트 색 자동 보정을 주장하지 않는다. 각 `.native.png`, `.wasm.png`,
`.overlay.png`, `.review.png`가 같은 폴더에 있다. 이것은 SVG 출력의 backend 대조이며
Studio Canvas 실사용 또는 #6923 원본 전체 PDF 시각 통과로 확대하지 않는다.

최종 파일 지문은 `verified-source.sha256`, 내부 커밋은 `verified-commit.txt`에 연결한다.
다음 작업은 원본 첫 표의 저장 줄 수용 제한의 실제 조건을 추적하고 공통 줄 구성으로
연결하는 것이다. 분할 gradient·다른 배경 효과·중첩 gradient의 모든 조합은 미검증으로
남기며 A/R5는 진행 중이다. 기본 엔진 전환·remote push·PR은 하지 않았다.

### R19 후속: 셀 안의 저장 줄 영역과 빈 문단 캐럿

출발 HEAD `8d3d7613b`. 원본 #6923 첫 표(p0/ci3)의 첫 셀은 폭47488HU,
좌우 유효 여백141HU씩으로 내용 영역47206HU이며 저장 줄은 cs0/sw47204HU다.
거부 원인은 여러 줄 자체가 아니라 V2가 저장 줄 폭과 셀 내용 폭의 완전 일치를 요구한
조건이다. `document_ir_lineseg_standard.md`의 줄별 물리 영역 계약에 따라 포함된
cs/sw를 그대로 사용한다. 원본 HWP·LineSeg를 수정하지 않았고 2HU 보정이나 문서별
허용치를 추가하지 않았다. 기존 부동소수 변환 비교 외의 tolerance도 늘리지 않았다.

값의 경로는 `table_v2/stored_text::localize`의 포함 검사/세로 원점 이동 →
`table_v2/text::TextComposer::compose` →
`layout/paragraph_layout::layout_composed_paragraph_in_frame(physical_frame_rows=true)`의
줄별 effective_col_x/w·정렬 → `stored_text::validate_paint`의 실제 x/width/y/height/
baseline/줄 소속 확인 → 같은 TextLine 노드로 만든 Lines와 paint payload다.
페이지 fit/이어받기는 그 Lines를 소비하고 paint는 최종 배치만 평행 이동한다.
뒤 문단도 같은 점유 결과를 소비한다. 기존 dirty cache, 잘못된 partition, 겹치는 세로 줄,
여러 segment/페이지 reset, 범위 밖 줄의 거부는 유지한다. 변경 후 별도 폭 덮어쓰기를
허용하는 대신 실제 paint 폭/원점 검사도 추가했다.

폭 제한을 제거한 뒤 원본의 중앙 정렬 빈 문단에서 캐럿 TextRun의 폭이 줄 전체 폭으로
생성되어 오른쪽을 넘는 문제가 드러났다. `layout_empty_runs_line`의 **물리 줄 경로**는
빈 캐럿의 advance를0으로 생성하며 TextLine의 저장 폭·높이·간격은 보존한다.
TextComposer의 run containment를 건너뛰거나 좌표를 clamp하지 않았다. 일반
`layout_composed_paragraph`는 physical_frame_rows=false로 기존 경로를 유지한다.
공유 true 경로인 `layout_stored_inline_flow`도 영향 범위이므로 기존 #6706/#6737/#6754
계약을 추가 실행한다. Studio 편집 caret/hit-testing 전체 시나리오는 미검증이다.

`tests/cases/issue_7353_table_v2_{text,export}.rs`의 새 계약은 저장 줄의 서로 다른
가로 영역, 좌/중앙/우 정렬, 예산21px에서의 앞 줄 소비와 이어받기·뒤 문단·종료,
범위 밖/폭0 거부, 빈 중앙/우 정렬의0폭 캐럿과18px 점유, 실제 원본 첫 셀 probe다.
기대값은 저장 HU·명시한12px 줄/18px pitch·정렬 불변식에서 정했다. 합성 HWPX는
한컴 생성본이 아니며 원본 셀 probe도 원본 전체 표의 PDF 일치 증거는 아니다.

증적은 `output/7353/r19/stored-frames/`다. `before.log`는 새 text 계약2건이 기존
저장 줄 제한으로 FAIL(기존9 PASS), `focused.log`는 가로 영역 계약 통과 후 빈 캐럿
거부를 재현했다. `focused-caret.log`는47 PASS와 원본 admission 진단1 FAIL이다.
진단의 실제 다음 경계가 문단0의 `non-table cell control`로 이동해 그 **오류 기대값만**
갱신했다. 원본은 여전히 미지원이며 page-count/golden/ignore를 변경하지 않았다.
중간 `after.log`의0 tests는 suite 준비 누락으로 검증에 포함하지 않았고 이후 원본
변경마다 review worktree에서 manifest를 다시 prepare했다.

Native `native-center.png`/`native-empty.png`를 직접 열어 서로 다른 줄 영역 안의 정렬,
빈 첫 줄 뒤의 본문 위치를 확인한 뒤 제품 소스를 고정했다. 최종 회귀·lint·Docker
WASM·직접 대조 결과는 아래에 이어 기록한다.

최종 `final-tests.log`는205 PASS(V2 183+Legacy/저장 inline 대조22)다. review worktree에서
manifest prepare 후 각 case를 `resolveCasePlan`으로 연결한 generated target에
`cargo nextest run --locked --no-fail-fast -E 'test(~issue_7353_table_v2_...) | …'`
필터를 적용했고 shared `target/pr-review`, `CARGO_BUILD_JOBS=1`을 사용했다.
fmt와 Native library/WASM library/변경 integration target Clippy(-D warnings),
고정 base `7a95e46e025470a4d7a7b59ad68ec02958bda738` 대비 manifest check도 통과했다
(`fmt.log`, `clippy-{native,wasm,test}.log`, `policy.log`). 진단 오류 기대값의 rustfmt
줄바꿈만 뒤에 정리했으며 실행 의미와 제품 소스는 동일하다. 전체 workspace 제출 CI나
source-side unit test 변경은 이번 범위가 아니다.

`docker compose --env-file .env.docker -p rhwp run --rm wasm`은7분27초에 완료했다
(`docker-wasm.log`). fresh `pkg/rhwp_bg.wasm` SHA256은
`29895e351dad36dce85379026173f993d3551e9320578776d9155eca74b3ce49`다.
이전 절편의 browser 명령에서 fixtures/out을 `stored-frames/` 아래로 바꾸고
`--stored-frames`를 추가했다. `browser.log`/`browser/manifest.json`의161쪽 모두
정확한 Native/fresh WASM tree·SVG 대조를 통과했다. 기존 Native 자료7개는 빈 캐럿의
width만0으로 바뀌었고 다른 좌표·내용은 동일하다(`prior-native-compare.log`). 기존
153쪽의 review PNG는 모두 byte 동일하다(`prior-visual-compare.log`).

`browser/stored-frames-contact.png`의 새8쪽 standalone overlay와
`browser/stored-frame-center-0.review.png`의 Native/fresh WASM/overlay3면을 직접 열어
저장 영역이 다른 줄들의3정렬, 다음 쪽의 뒤 문단, 빈 첫 줄의 점유 공간을 확인했다.
각 `.native.png`, `.wasm.png`, `.overlay.png`, `.review.png`도 같은 폴더에 있다.
합성 경계와 backend 일치 증거이며 Studio Canvas 편집 검증이나 한컴 PDF 일치로
확대하지 않는다. `source-final.sha256`/`post-build-source-check.log`는 빌드 소스의
불변을, `original-inputs.sha256`는 원본 HWP/PDF를, `verified-source.sha256`와
`verified-commit.txt`는 최종 파일/내부 커밋을 고정한다.

다음 장애물은 원본 첫 표의 저장 줄 안 TAC 그림이다(원본 dump의 bin_id1, 이어서2).
`table_v2/ir.rs`의 비표 컨트롤 거부와 `text_ir.rs`의 표 전용 paint slot을 실제 그림의
줄 소속·점유·BinData paint까지 연결해야 한다. 그림을 삭제하거나 Legacy 표 조판으로
우회하지 않는다. 원본 종단 수용/PDF 비교와 A/R5는 아직 진행 중이다.
기본 엔진 전환·remote push·PR은 수행하지 않았다.

## 후속 — 셀의 저장 TAC 그림 줄과 리소스

선행 소스는 `034c0777e`다. 원본 첫 표의 cell1/bin1과 cell8/bin2는 문자 없이
그림 컨트롤을 가진 저장 줄이며 선언 크기는 각각8021×5064,10738×4617HU다.
원본의 줄/그림/리소스를 지우지 않고 V2에 연결한다. 문자 없는 완전한 제어 슬롯,
같은 줄에서 같은 점유 envelope, 양의 줄 높이와 겹치지 않는 저장 줄을 이번 경계로
한정한다. 그림별 바깥여백을 포함한 줄은 원자적이며 예산이 부족하면 통째로 이월한다.
문자 혼합·부유 배치·음수 줄간격/겹침·회전/전단·그림 효과·캡션은 명시적 미지원이다.
긍정 배율은 선언 크기에 이미 반영되어 있으므로 그림을 다시 확대/축소하지 않는다.

소비 경로는 `tac.rs::stored_object_rows`의 실제8-unit 소속/저장 cs·sw/여백 query →
`pictures.rs::compose`의 TextLine+Image와 동일 bounds →
`ir.rs::bind_table`의 ObjectRow 소유 검사/FlowBlock::Lines →
`flow.rs::FlowCursor`의 Lines 예산 검사/pen 누적·불수용 시 같은 유닛 이월 →
`text.rs::TextPaint::paint`의 동일 payload 평행 이동이다. `text_ir.rs`는 paint 순서를
같은 ObjectRow로 기록한다. 그림 리소스는 selected-table `session.rs` 및 본문
`document_input.rs`의 두 표 진입점에서 전달한다. 실제 그림 데이터는 스냅샷에 포함되며
입력 버퍼를 폐기해도 유지된다. common paint의 crop/reference-size를 보존한다.
새로운 height clamp, 별도 그림 좌표 재계산, Legacy 표 fallback은 추가하지 않았다.
rowspan/표 분할 알고리즘 자체는 이번에 바꾸지 않았다.

`tests/cases/issue_7353_table_v2_export.rs`는 좌/중앙/우 정렬, 같은 줄 그림2개와
서로 다른 저장 줄,24px 그림에23px만 남는 예산, 뒤 문단·종료, 여백/자르기,
중첩 셀/부모 높이, DocumentV2 리소스 전달, 누락 리소스/모호한 소속/효과 거부를
검사한다. 기대값은 합성 입력의 명시적인HU·12px 글줄/18px pitch,24px 그림 줄과
정렬 불변식으로 정했다.4색 PNG와 좌상단 자르기의 빨간 픽셀은 독립적인 paint oracle이다.
원본 두 그림은 원본 문단을 유지한 단일 셀 probe로 검사하고 HWPX 파생 입력으로도
재열어 크기를 확인한다. 부모 표/다른 문단을 제거한 **격리 파생본**이며 원본 전체 문서나
한컴 생성 대조군으로 주장하지 않는다.

증적 폴더는 `output/7353/r19/pictures/`다. `before.log`는 기존 소스에서 새 긍정
계약2건이 `non-table cell control`로 FAIL한 증거다. `after.log`의 실제 새 양성 계약은
통과했으나, 음성 입력을 직렬화할 때 잘못된 리소스 그림이 생략된 것과 원본 다음 거부
변경 때문에2 FAIL했다. 음성 검사는 손실 직렬화 전의 IR을 직접 검사하도록 정정했다.
`after2.log`의 추가1 FAIL은 합성 DocumentV2 입력에 PageDef가 없던 문제로, 명시적인
400×200px 용지를 입력에 추가했다. 실행되지 않은 CLI 인자 오류/컴파일 오류는
결함 재현 증거에 포함하지 않는다. 원본 admission은 문단0의
`stored text paragraph insets`로 이동했으며 그 진단 기대값만 갱신했다.
baseline/golden/ignore를 바꾸지 않았다.

`native-center.png`와 `native-source-1.png`를 직접 열어 중앙 정렬 그림 줄과 원본
로고의 표시를 확인한 뒤 제품 소스를 고정하고 Docker fresh WASM을 시작했다.
최종 회귀·lint·WASM 직접 대조 결과는 이어 기록한다. A/R5와 원본 종단 피델리티는
여전히 진행 중이며 기본 엔진/원격 작업은 변경하지 않는다.

기준 PDF1쪽(`gradient/reference-p1.png`)도 직접 확인했다. 오른쪽 OPEN 그림은 보이지만
왼쪽 첫 로고 영역은 비어 있어 격리 probe의 로고 출력과 다르다. 원본 전체 V2는 아직
수용되지 않으므로 원래 부모 셀/문서 문맥에서의 원인과 일치 여부는 미검증이다.
probe 통과로 이 차이를 해소하거나 한컴 피델리티 통과로 분류하지 않는다.

최종 `final-tests.log`는210 PASS(V2 188+Legacy/저장 inline 대조22)다. review
worktree에서 source 변경 후 manifest를 다시 prepare하고 `resolveCasePlan`으로 각
case의 generated target을 찾아 `cargo nextest run --locked --no-fail-fast`와 case
필터를 실행했다. `CARGO_BUILD_JOBS=1`, shared `target/pr-review`를 사용했다.
`fmt.log`, `clippy-{native,wasm,test}.log`는 fmt check와 Native/WASM library 및
변경 integration target의 Clippy(-D warnings) 통과 증거다. `policy.log`는 고정 base
`7a95e46e025470a4d7a7b59ad68ec02958bda738` 대비 manifest check 통과다.
source-side unit test는 변경하지 않았고 전체 workspace 제출 CI를 실행한 것은 아니다.
최초 Native Clippy가 unsigned width/height의 `<=0`을 지적해 동등한 `==0`으로 정리했다
(`clippy-native-before.log`). 진행 중 Docker를 중단한 뒤 최종 소스로 다시 빌드했으며,
위210건도 수정 후 다시 실행한 결과다. `prior-native-compare.log`의 기존89 fixture
출력은 이전 절편과 byte 동일하다.

최종 `docker compose --env-file .env.docker -p rhwp run --rm wasm`은7분23초에
완료했다(`docker-wasm.log`). fresh WASM SHA256은
`855a9531992f7d966d6017e710563f4675aa4a53334ee75934ffd2123d06699c`다.
browser 명령은 직전 stored-frames 검증과 동일한 flags에 `--pictures`를 추가하고
fixtures/out을 `pictures/` 아래로 변경했다. `browser.log`/`browser/manifest.json`은
187쪽(기존161+새 그림26)의 정확한 Native/fresh WASM tree·SVG 일치와 입력 폐기·
독립 cursor·재열기·거부·rollback·종료 검사의 통과 증거다. 새4색 그림과 crop 결과는
두 backend의 실제 PNG 내부 픽셀도 검사한다. `prior-visual-compare.log`의 기존161쪽
review PNG는 모두 이전과 byte 동일하다.

`browser/pictures-contact.png`의 새26쪽 standalone overlay와
`browser/picture-center-0.review.png`, `picture-crop-margin-0.review.png`,
`picture-source-8-0.review.png`를 직접 열었다. 정렬·여백, 그림 줄 전체 이월/다른 줄의
이어받기, 부모 셀 뒤의 after/tail 위치와 자르기 표시를 확인했다. 각 `.native.png`,
`.wasm.png`, `.overlay.png`, `.review.png`가 같은 폴더에 있다. 이 판정은 합성 규칙과
backend 일치에 대한 것으로 원본 전체 PDF 일치나 Studio Canvas 편집 검증은 아니다.
원본 첫 로고와 PDF의 차이는 앞서 기록한 미검증 상태로 남긴다.
`source-final.sha256`/`post-build-source-check.log`, `original-inputs.sha256`,
`verified-source.sha256`/`verified-commit.txt`로 빌드 소스·원본·최종 내부 커밋을 고정한다.

다음은 `text.rs::TextComposer::compose`의 저장 문단 margin/indent 제약과 실제 저장
줄 영역의 관계다. 측정·paint에서 같은 들여쓰기 결과를 소비하는지 확인하며 이어간다.
이번 절편의 지원 범위는 완료했지만 원본 문서의 종단 수용/PDF 비교와 A/R5는 미완료다.
기본 엔진 전환·push·PR은 수행하지 않았다.

### R19 후속: 저장 문단의 좌우 여백과 물리 줄 영역

직전 `1baa42de3`에서 원본 첫 표의 다음 거부는 저장 문단 여백이었다. 이번에는
`TextComposer`의 일괄 거부를 분리하고, **들여쓰기 0이며 저장 줄 전체가 문단 좌우
여백 안에 포함되는 경우**를 수용한다. `cs/sw`를 이동·축소하지 않는다. 왼쪽 여백보다
작은 cs, 오른쪽 여백을 침범하는 끝점, 양/음수 들여쓰기는 계속 명시적으로 거부한다.
들여쓰기에는 저장 bit20과 셀/본문 문맥 차이가 있어 margin과 같은 것으로 취급하지 않는다.
이 부분의 독립 근거·구현은 다음 지원 범위로 남긴다.

입력 근거는 원본 #6923 첫 표의 셀2/4/5/6/7/10/11이다. 원본 paragraph/style/LineSeg를
보존해 단일 셀 probe로 격리한다. ParaShape 좌측 여백1000의 해석은500HU이며 원본
cs도500HU다. 우측 여백·indent는0이다. 셀 폭·padding·원본 저장 sw는 그대로 유지한다.
원본 전체나 기준 PDF 일치를 주장하는 시험이 아니며 PDF1쪽의 해당 내용과 구분한다.
합성 계약은200px 셀 안 좌10/우20px, 저장 줄 [10,180]/[20,180],12px 줄/18px pitch로
독립 지정했다. 빈 줄도18px을 소비하고 뒤 문단이 이어받는다. Left/Center/Right는
각 저장 상자 안에서 정렬되며 새로 조판한 뒤 문단은 문단 여백으로 생성한170px 폭을 쓴다.

소비 경로: `stored_text::localize`가 문단 content interval에 저장 cs/sw를 검증하고
`TextComposer::compose`가 `layout_composed_paragraph_in_frame(physical_frame_rows=true)`로
물리 줄을 생성한다. 공용 paint의 `uses_stored_segment_geometry`와 여백0 분기가
저장 상자를 그대로 소비한다. `validate_paint`는 최종 bbox·baseline·줄 소속·전진량을
다시 검증한다. 같은 노드에서 `ParagraphItem`/`FlowBlock::Lines`의 요구 높이와 paint
payload를 만든다. `flow.rs::fit`은 수용한 줄만 전진시키며 예산 실패 시 줄을 남긴다.
`TextPaint::build_node`는 확정 placement와 payload 원점 차이로만 평행 이동한다.
추가 clamp/크기 재계산은 없다. 본문도 `text_flow`에서 같은 composer를 사용하지만
이번 신규 독립 시각 probe는 셀 경로다. TAC/rowspan의 컷 알고리즘은 변경하지 않는다.

검증 증적은 `output/7353/r19/margins/`에 기록한다. `before.log`의 새3검사는 모두
기존 `stored text paragraph insets` 거부로 FAIL했다(환경 실패 아님). 후속 실행 결과는
아래에 추가한다. 기본 엔진은 Legacy이며 원본 종단 A/R5는 여전히 진행 중이다.

`after.log`는 합성2 PASS와 진단2 FAIL이었다. 하나는 원본 수용 경계가 문단0의
`TAC content changed stored occupied box`로 이동한 것이고, 다른 하나는 격리 probe의
TablePageBreak를 원본 RowBreak 대신 CellBreak로 만든 오류다. probe가 원본 속성을
유지하도록 수정하고 admission의 관측 진단만 갱신했다. baseline/ignore를 바꾸지 않았다.
최종 `final-tests.log`는213 PASS(V2 191+Legacy/저장 inline 대조22),
`prior-native-compare.log`는 이전99 fixture의 Native 출력 byte 동일이다.
Native `native-center.png`, `native-source-5.png`를 직접 열어 저장 영역 안의 중앙
정렬과 원본 보도일시 셀의 두 줄이 모두 표시됨을 확인했다. 원본 probe는 부모 셀 위치를
옮긴 격리 출력으로 PDF 전체 페이지와 같은 좌표라는 주장은 하지 않는다.

다음 장애물의 관측도 남겼다(`source-cell-height-observations.jsonl`). 원본 셀5의
선언 높이3042HU=40.56px에 비해 격리 V2는45.36px이고, 차이360HU=4.8px는 마지막
줄간격과 같다. 셀2도 선언2282HU보다600HU 높다. 셀4/10은 선언 높이와 같다.
원본 PDF1쪽을 직접 열어 해당 내용이 두 줄로 배치됨을 확인했다. 이 관측은 셀 끝의
줄간격/내용 요구 높이와 TAC 저장 상자의 관계를 다음에 조사할 근거이며, 마지막
줄간격을 무조건 제거한다는 구현 결정은 아니다. 원본 종단 거부를 우회하지 않았고
이들 격리 probe를 한컴 시각 통과로 승격하지 않는다.

최종 검증은 동일 소스에서 수행했다. review worktree의 `fmt.log`,
`clippy-{native,wasm,test}.log`는 fmt check, Native root/WASM32 lib 및 변경 integration
target Clippy(-D warnings) 통과다. `policy.log`는 고정 base
`7a95e46e025470a4d7a7b59ad68ec02958bda738` 대비 manifest check 통과다. source-side
unit test는 변경하지 않았다. 전체 workspace 제출 CI를 실행한 결과는 아니다.

`docker compose --env-file .env.docker -p rhwp run --rm wasm`은7분5초에 완료했다
(`docker-wasm.log`). WASM SHA256은
`b0c55878f736fe0f2c10611ceb37181205cc13631e6354a56cfb8d1868988a78`이다.
`source-final.sha256`/`post-build-source-check.log`로 빌드 중 소스 불변을 확인했다.
Chrome 최초 실행은 시작 단계에서 종료했다(`browser-launch-failure.log`). 동일 명령의
재시도는 성공했으며 환경 실패를 조판 회귀로 집계하지 않았다.

browser 명령은 직전 pictures 검증의 flags에 `--stored-margins`를 더하고
fixtures/out을 `margins/` 아래로 지정했다. `browser.log`/`browser/manifest.json`은
206쪽(기존187+새19)의 정확한 Native/fresh WASM tree·SVG 일치, 실제 독립 좌표·내용,
입력 폐기·재열기·거부·rollback·종료 검사 통과 증거다. `prior-visual-compare.log`는
기존187쪽 review PNG가 byte 동일함을 확인한다. 수치·baseline 허용치는 완화하지 않았다.

`browser/margins-contact.png`의 새19쪽 standalone overlay와
`browser/stored-margin-source-5-0.review.png`를 직접 열었다. 원본7셀의 내용 표시,
합성 Left/Center/Right의 A/B·빈 줄·after 순서와 정렬을 확인했다. 같은 폴더의
`*.native.png`, `*.wasm.png`, `*.overlay.png`, `*.review.png`가 Native/compare/overlay
증적이다. backend 일치 검사이며 한컴 PDF 피델리티 점수를 산출한 것은 아니다.
빈 줄 점유 높이는 보이는 글자 유무가 아니라 정식 회귀의18px 예약/후속 페이지로 검사한다.

이번 저장 여백 지원 범위는 검증했으나, 양/음수 들여쓰기와 원본 종단 TAC 높이 계약은
미완료다. 다음은 `tac::bind`의 저장 상자와 실제 `TableContentPlan.height` 차이를
셀 끝 줄간격·padding·선언 높이 및 기준 PDF와 대조하는 작업이다. 기본값 전환·원격
push/PR은 수행하지 않았다. 내부 커밋 SHA는 `verified-commit.txt`에 연결한다.

### A 후속 — 셀 종료 높이 후보 조사와 원점 독립적인 여백 소비

원본 #6923 첫 표를 수정하지 않고 조사한다. 마지막 줄의 `line_spacing`은 다음 줄
원점으로의 전진이며 셀의 종료 뒤에 존재하는 줄 상자는 아니다. 빈 마지막 문단의
line_height, 명시적인 spacing_after, 셀 padding과 선언 최소 높이는 별도로 보존한다.
후보 구현에서는 저장/재조판 일반 텍스트 모두 같은 종료 규칙을 적용하고 본문의 전진 계약은
변경하지 않았다. **이 후보는 아래 영향 검사 뒤 활성 소스에서 분리해 보관했다.**

독립 수치 근거: 원본 첫 표 common.height=11102HU. 비-spanning 셀로 정한 행 최소 높이는
371/2282/3042/1922/3274HU이며 첫 빈 셀의 실제 줄300+padding282=582HU가 첫 행을
확장한다. 마지막 줄간격을 셀 점유로 더하지 않은 합은582+2282+3042+1922+3274=11102HU다.
첫 행을 저장371HU로 축소하거나 표 전체를 선언 높이로 clamp하는 처리는 하지 않는다.
셀5는1560+1200+282=3042HU, 셀6은1300+282=1582HU보다 큰 선언1922HU를 유지한다.
원본 PDF1의 표 외곽/셀 배치와 함께 검증할 대상이며 이 산술만으로 시각 통과를 주장하지 않는다.

값 경로는 TextComposer의 공용 실제 줄 노드/다음 문단 원점 → 셀 종료 문맥의
ParagraphItem → ir::bind_table/명시적 text_flow → content::physical_extent/행 높이 →
FlowCursor::fit/row_groups의 수용 및 이어받기 → TextPaint의 동일 payload 평행 이동이다.
그림/TAC carrier도 마지막 양수 줄간격과 paragraph-after를 분리한다. TAC 음수 전진은
셀 내부 후속 줄에는 유지하고 셀 종료에서는 물리 상자 끝을 사용한다. 본문에는 기존 전진을
유지한다. 공용 Legacy paragraph compositor, 기본 엔진, TAC 저장 상자 검증은 바꾸지 않는다.

증적 위치는 `output/7353/r19/terminal/`. 새 합성 검사에서 빈 마지막 줄과 spacing_after,
저장/재조판 및 종료를 검사하고 원본 binary 전체 첫 표의 선언 envelope를 별도 검사한다.
`before.log`의 합성 검사는 끝 간격 잔여로 미종료 FAIL했다. 원본 선택 표는 배치 중
InconsistentAtomicPlan으로 실패했으므로 이것은 높이 assert 실패 증거와 구별한다.

후보의 새 저장/재조판 빈 마지막 줄 검사는 PASS했고 원본의 첫 TAC 표 높이 검증도 통과해
문단1의 decoration/keep 거부까지 이동했다. 정수 HWPUNIT 좌표의 원본 선택 표는
11102HU를 정확히 산출했다. 격리7셀의 높이는 모두 원본 선언값과 일치했다. 그러나
`recheck.log`는216건 중109 PASS/107 FAIL이다. 기존 합성 계약의 끝 줄간격·정렬·분할
기대값 차이와 TAC 합성 입력의 저장 envelope 불일치 등이 함께 검출됐다. 이를107건의
실제 회귀 또는107건의 잘못된 기대값으로 일괄 판정하지 않는다. `after.log`의203건은
fmt 뒤 파생 suite가 갱신되기 전의 불완전 선택이며 최종 집계로 사용하지 않는다.
기존 기대값·baseline·ignore를 변경하지 않고 후보를 `terminal-height-prototype.patch`로
보존했다. 이 패치의 실험 결과는 현재 체크포인트의 검증 결과와 구분한다.

선행 보정은 별개의 수치 결함이다. 후보 실험 빌드의 `atomic-gdb.log`에서 원본 첫 빈 셀(행0/열0)의
7.76px 예산을 소비한 뒤 bottom padding이2.2204460492503131e-16px 남아
row_groups.rs의 완전 소비 검사가 실패함을 확인했다. 원인은 같은 물리 높이의 비교에
page y를 양변에 더하며 계산 순서가 달라진 것이다. `flow.rs::fit`의 **Space 소비 완료**
비교를 측정과 같은 로컬 좌표 `pen+left <= area.height`로 바꾼다.
실제 배치의 원점 평행 이동은 유지한다. clamp·추가 epsilon·선언 높이 보정은 없다.

정식 반례는141HU 위/아래 padding과300HU 빈 줄을 실제 변환 순서
`HU*(96/7200)`로 만든다. y=0/30/1000에서 동일한 정확 예산의 완료와 실제 줄 원점을
검사하고1HU 부족한 예산은 거부한다. 처음 작성한`HU/75`는 IEEE754 표현이 달라 수정
전에도 통과했다(`fractional-before.log`). 이를 검출 증거로 세지 않았고 실제 source
변환 순서로 고친 `fractional-before-source-scale.log`에서 해당 원인의 FAIL을 확인했다.

값/컷 경로: content::physical_extent가 padding/줄의 로컬 합으로 행 요구 높이를
생성하고 row_groups::fit_row_groups 또는 fit_rows가 동일 높이를 예산으로 넘긴다.
FlowCursor의 Space 잔여량과 Lines/InlineTables 소유 유닛은 성공한 fit에서만 소비하며,
실패한 줄은 그대로 이월한다. 실제 line/table bounds는 기존처럼 area 원점을 더해 배치한다.
이 보정은 row/rowspan 컷·제목 반복·최소 높이·종료 뒤 내용을 바꾸지 않는다.

추가 원본 계약은 같은 바이너리의7200dpi/96dpi에서 선택 표1개·셀12개·그림2개와
모든 Table/TableCell/TextLine/Image bbox의 물리 배율 및 텍스트 보존/완료를 검사한다.
이 배율 불변식은 독립 기하 계약이며 기존의 잘못된 끝 줄간격 높이를 승인하는 계약이 아니다.
기존 원본 admission은 첫 TAC 높이 불일치로 남겨두며 이슈 종단 완료를 주장하지 않는다.

중간 `checkpoint-tests.log`는215건 중214 PASS/1 FAIL이었다. Space 외에 Lines/
InlineTables까지 로컬 비교로 바꾸면 기존`fractional_page_budget_does_not_split_an_atomic_nested_table`
계약(부모1.2, 선행1.0, atomic 자식0.2)의 차감 예산0.199999...와 충돌한다. 이 검사는
그대로 보존하고 Lines/InlineTables 변경은 철회했다. 원점 오판의 확인된 원인인 Space
소비만 보정한다. 일반적인 분할 예산의 수치 표현 통일은 이번 완료 범위에 포함하지 않는다.

최종 활성 소스의 `final-tests.log`는 **215 PASS**(V2 193 + Legacy/저장 inline 대조22)다.
review overlay에서 fmt 뒤 파생 suite를 준비하고 기존 V2 전체 및 같은22건 대조군을
`cargo nextest run --locked --no-fail-fast`의 case filter로 실행했다. `prior-native-final.log`는
직전 margins 단계의 기존109개 Native export가 byte 동일함을 확인한다. 새2건은 원점별
정확 예산/부족 예산과 원본 전체 첫 표의 단위 배율·완전 소비 계약이다.

`fmt.log`, `clippy-{native,wasm,test}.log`는 fmt check, Native root/WASM32 lib 및
변경 integration target Clippy(-D warnings) 통과다. `policy.log`는 고정 base
`7a95e46e025470a4d7a7b59ad68ec02958bda738` 대비 manifest check 통과다. source-side
unit test는 변경하지 않았다. 이 내부 체크포인트는 전체 workspace 제출 CI의 완료가 아니다.
검증 소스는 HEAD `0e2e2fd36`에 이번 변경을 적용한 상태이며 네 코드/테스트/script 파일의
해시를 `checkpoint-source.sha256`에 고정했다. `post-build-source-check.log`도 모두 OK다.

`docker compose --env-file .env.docker -p rhwp run --rm wasm`은7분11초에 완료했다.
`pkg/rhwp_bg.wasm` SHA256은
`e02672c2784d0b890eed2080347bb36e69c362ffbf0b316bc7a26526ecb93daf`이다.
브라우저 명령은 직전 margins 검증의 flags에 `--fractional-cell-fit`을 추가하고
`--fixtures output/7353/r19/terminal/checkpoint-fixtures`,
`--out output/7353/r19/terminal/browser`를 사용했다. `browser.log`와
`browser/manifest.json`은 **207쪽 PASS**(기존206 + 원본 선택 표1)의 정확한 Native/fresh
WASM tree·SVG 일치와 입력 격리·거부·rollback·종료 검사 증거다.
`prior-visual-final.log`에서 기존206쪽 review PNG도 byte 동일하다.

`browser/fractional-source-0.review.png`와 `fractional-source-0.overlay.png`를 직접 열어
표 외곽,12셀, 두 그림과 텍스트의 backend 일치를 확인했다. 원본 PDF1과는 셀 끝 줄간격에
따른 높이 차이와 왼쪽 로고 표시 차이가 남아 있다. 이 출력은 선택 표의 소비 완료/수치
불변식 검증이며 원본 전체 조판이나 한컴 피델리티 통과가 아니다. 원본 종단 admission의
`TAC content changed stored occupied box` 거부도 그대로다.

기존 엔진과 기대값·baseline·ignore는 변경하지 않았다. 다음 작업은 보관한 종료 높이
후보의 실패107건을 독립 규칙/입력 근거로 분류하는 것이다. A/R5는 아직 미완료이며
기본값 전환·원격 push/PR은 하지 않았다. 이번 내부 커밋은 `verified-commit.txt`에 연결한다.

### A 후속 — 종료 높이 후보 실패 트리아지와 조합 계약

기준 제품 head는 `4f51d28158945f91c9a010804b3c316a8a0f1b20`이며 작업 시작 시 clean이다.
이 절의107건은 **비활성 후보**의 과거 실패 목록이지 현재 제품의 회귀 실패가 아니다.
기존 `recheck.log`/`prototype-failures.json`을 재사용하고 각 검사명·실패 위치·관측값·
처리 구분을 `output/7353/r19/terminal/triage-ledger.json`에 연결했다.

| 건수 | 분류와 근거 | 후속 처리 |
| --- | --- | --- |
| 96 | 기존 수치/분할 경계 assertion. 주로 font12/고정 pitch18의 끝 간격6을 셀 높이에도 포함한 합성 입력이며, 저장 줄 검사도 포함 | 미확정 판정 유지. 수치 변경과 실제 누락/중복을 구별하고 경계 테스트의 입력 의미를 보존 |
| 9 | `document_flow::inline_carrier` 계열의 수동 TAC envelope와 재조판 내용 높이 불일치 | 저장 높이 검사를 느슨하게 하지 않음. 유효한 저장 대조와 선언 최소 높이의 역할을 별도 검증 |
| 1 | 원본 첫 표의 분수 padding 완전 소비 실패 | 별도 수정 `4f51d2815` 적용 뒤 후보에서도 단위 배율/완전 소비 검사 PASS |
| 1 | 원본의 명시적 미지원 경계가 문단0 TAC 높이에서 문단1 장식/keep으로 이동 | 기능 수용의 진전으로 분류하되 원본 문서 전체/PDF 통과로 세지 않음 |

대표 경계: `issue_7353_table_v2_headers.rs::header_only_fit_is_nonfit_and_retry_does_not_consume_body`
는 기존 제목18 + 본문 글줄12 > 예산29를 전제로 한다. 후보에서는 제목12 + 본문12=24여서
29에 두 줄이 들어간다. 따라서 이 실패만으로 제목만 출력한 버그라고 판정할 수 없다.
새 종료 규칙의 정당성을 별도로 확인한 뒤 `제목 요구높이 + 최소 본문 줄 높이 - 부족량`의
경계에서 거부/재시도/내용 보존을 검사해야 한다. 단순히 실패 assertion을 삭제하지 않는다.

대표 TAC: 같은 파일이 아니라 `issue_7353_table_v2_document_flow.rs::inline_carrier`는
자식의 common.height=2700HU(36px), cell.height=0, fresh 두 문단을 수동 구성한다.
후보에서12+6+12=30px가 되므로 `tac.rs::bind`의 저장36px와 맞지 않는다. serializer
round-trip을 거쳤다는 사실은 한컴이 생성한 유효 저장 줄이라는 증거가 아니다. 이 거부를
없애려고 bind의 동등성 검사나 원본 LineSeg를 완화하지 않는다.

**후보 자체의 실제 회귀도 별도로 발견했다.** `text_flow.rs`의 후보는 마지막 vector 항목이
Paragraph일 때만 `compose_cell_end`를 선택한다. 같은 문단 뒤에 `Space(0)`을 붙이면
`compose`로 바뀌어 다음 줄 간격을 다시 포함한다. 명시적 Space는 API 주석상 추가 물리 공간이지
다음 문단/줄을 만들어내는 표지가 아니다. 따라서0은 항등이고 양수 공간은 그 높이만 추가해야 한다.
이 기대값은 후보가 계산한 끝 높이나 한컴 페이지 수가 아닌 조합의 불변식에서 정했다.

정식 검사2건은 `tests/cases/issue_7353_table_v2_nested_text.rs`에 추가했다.

- `zero_trailing_space_is_an_identity_for_cell_flow`: 같은 최종 줄의 내용/y/높이와 예약 높이 동일.
- `explicit_terminal_space_adds_only_its_own_height`: 뒤 공간2/7 추가 시 앞 줄의 내용/원점/높이를
  보존하고 예약 높이는 각각2/7만 증가. 셀 끝 줄간격 자체를12 또는18로 고정하지 않는다.

별도 review overlay에서만 보관 후보의6개 engine 파일을 적용했다. 제품 파일과 WASM은
변경하지 않았다. `triage-current.log`는 현재 구현의 관련11건 PASS다.
`triage-prototype.log`는 대표6건 중1 PASS/5 FAIL이며 새2건에서 각각
`12→18`(0 공간 추가), `+2 대신 +8`을 재현했다. 나머지3 FAIL은 위 제목 경계, TAC envelope,
원본 admission 진단이고 원본 배율/완전 소비는 PASS였다. 실험 뒤 적용한6개 파일만 복원하여
제품과 byte 동일함을 확인했다. 기존 review WIP/파생 suite의 다른 변경은 건드리지 않았다.

다음 설계에서는 `실제 줄 점유 끝`, `다음 줄/문단 원점으로의 전진`, `명시적 물리 Space`를
구분한 결과를 만들고 IR/explicit flow/그림/TAC가 같은 종료 결정을 소비해야 한다. shared
paragraph layout에도 종료 줄 분기(`paragraph_layout.rs`의 `is_cell_last_line && cell_ctx.is_some()`)
가 있지만 현재 V2는 cell_ctx=None/is_last_cell_para=false로 호출한다. bool만 바꾸면 해결된다는
가정도 성립하지 않는다. 기존 셀 측정은 끝 spacing_after를 제외하는 경로가 있어, 이를 후보처럼
항상 보존하는 판단은 마지막 line_spacing과 별도로 독립 출력 확인이 필요하다.

이번 제품 변경은 방어 계약2건과 조사 기록뿐이다. 후보/기대값/fixture/baseline/ignore를 활성
변경하지 않았으며 R5 완료로 세지 않는다. 최종 검사와 내부 커밋 결과는 아래에 연결한다.

최종 `triage-final-tests.log`는 **217 PASS**(기존215 + 새2건)다. 앞선 최종 목록의 case를
현재 manifest로 다시 배정해 실행했으며 명령 전체를 로그 첫 줄에 남겼다. `triage-fmt.log`,
`triage-clippy-test.log`(변경 case의 `regression_suite_024`, `-D warnings`),
`triage-policy.log`(기존 고정 base 대비)가 모두 통과했다. 새 test source SHA256은
`38a5f2d7e578c3d08b13de93433cf9848563e064548103a1c91911358743c42e`다.
원장107개 검사명의 유일성·source 경로·분류 합계도 확인했다.

제품 Rust 구현과 WASM 소스는 `4f51d2815` 대비 변경0이다. 따라서 이전 Native/WASM library
lint와207쪽 backend 증거를 동일 소스의 기존 근거로 재사용하며, 이번 실행 건수로 합산하지
않는다. pkg SHA256도 위 `e02672c2…ecb93daf`와 동일해 Docker/WASM을 중복 빌드하지 않았다.
전체 workspace 제출 CI/원격 작업은 미실행이다. 내부 커밋은 `triage-verified-commit.txt`에
연결한다. 다음은 종료 점유/전진 결과의 구분과 유효한 TAC/분할 경계 입력 재설계다.

### A 후속 — 편집 의미 보존과 빈 문단 경계 계약

작업지시자는 HWP/HWPX → 공통 IR → 조판에서 편집자의 의도를 저장 속성에 따라 구현하는
도메인 원칙을 장기 메모리에 기록하도록 요청했다. 승인된 Codex 메모리 확장 노트에 기록하고
구현계획의 공통 결과 절에도 반영했다. AGENTS.md나 기존 메모리 원본은 수정하지 않았다.
시작 head는 `e166bf1cf9626e5cdeaf8e6e2cc3e55606cd777b`이며 clean 상태였다.

**원인 경계와 다음 설계:** 보류 후보의 마지막 vector 항목 판정은 추가 공간과 실제 후속 줄을
혼동한다. 이를 빈 문자열 제거·0 공간 삭제로 고치지 않는다. 빈 문단은 글자/문단모양과 줄
소유자를 유지하며 TAC/어울림 영향은 별도 줄 구성에서 결정해야 한다. 마지막 줄간격과 문단
뒤 간격의 셀 끝 적용은 별도 독립 근거가 필요하다. 이번에는 잘못된 후보를 활성화하지 않고
종료 결과 분리 작업이 보존해야 할 빈 문단의 실제 좌표·이월 계약을 먼저 확정했다.

값의 실제 경로는 다음과 같다.

- 입력 `Paragraph.char_shapes`/`para_shape_id` → `TextComposer::compose`의 style 선택 →
  `composer/line_breaking.rs::layout_paragraph_in_frame_impl`의 빈 문단 font 선택 및 줄 구성.
- `layout_composed_paragraph_in_frame`이 생성한 `TextLine.bbox`와 반환 원점 → `text.rs`의
  `ParagraphItem::Lines`/줄 사이 Space → `ir.rs::bind_table` 또는 `text_flow.rs::from_flow_rows`.
- `content.rs::physical_extent`와 padding/행 최소 높이 → fit의 줄 소유 유닛·공간 소비 →
  `TextFragment::append_to`의 실제 TextLine. 테스트는 helper 반환값이 아닌 이 최종 좌표를 검사한다.
- TAC는 `text_ir.rs` → `tac.rs::compose`의 저장 줄 소속/점유·전진 경로이며, 그림은
  `pictures.rs::compose`다. 이것들을 문자 없는 빈 문단으로 취급하지 않는다. 일반 어울림과
  텍스트/TAC 혼합 재조판은 현재 명시적 미지원 경계다. 기존 거부 검사의 PASS는 구현 완료가 아니다.

`tests/cases/issue_7353_table_v2_text.rs`에 다음 2건을 추가했다. 수동 작성 fresh IR의 합성
규칙 계약이며 정상 한컴 저장본이나 피델리티 증거로 분류하지 않는다.

1. `blank_paragraph_font_percent_and_insets_set_following_line_origin`: 앞/뒤 텍스트 사이
   빈 문단 0/1/2개, 글자 크기400/1400HU(4/14pt), 156%, 문단 앞200/뒤300HU를 사용한다.
   독립 기대 피치는624/2184HU이고 문단간 전진은 각각1124/2684HU다. IR/explicit flow 양쪽에서
   빈 줄의 수·높이·원점, 뒤 텍스트 원점, 조각 내부 점유, 입력 IR 불변, 완전 종료를 검사한다.
2. `blank_line_that_does_not_fit_is_carried_before_following_text`: 첫 줄 pitch1800 +
   위 padding3 + 빈 문단 앞200 + 빈 줄400보다1HU 부족한 예산을 준다. 앞 조각에는 before만,
   다음 조각에는 빈 줄과 after가 한 번씩 나타나며 두 줄 원점 차이는624+300HU다.
   빈 문단 삭제나 줄 상자를 Space로 대체하면 성립하지 않는 계약이다.

기존 96dpi의4pt/156% 검사와 원본 #6923 저장 빈 셀 검사도 같은 focused suite에서 재실행했다.
새 검사는 현재 코드에서 PASS인 **보존 계약**이며 발견된 기존 결함의 수정 전 FAIL 증거로
주장하지 않는다. 이 결과로 끝 줄간격 포함/제외 정책을 확정하거나 기존96건 기대값을 갱신하지 않는다.

검증은 기존 review overlay에서 수행했다. 다른 WIP는 보존하고 변경 test만 제품과 동일하게
갱신했다. `output/7353/r19/terminal/`의 `domain-text-tests.log`와
`domain-control-tests.log`에 문단14건 + 기존 IR/중첩/문서 흐름40건, **총54 PASS**를 기록한다.
후자는 저장 TAC의 같은 줄/다른 줄·음수 간격·원자적 이월과 어울림 명시적 거부 대조를 포함한다.
소스 구현은 변경하지 않았고 테스트만 추가했으므로 Docker/WASM을 중복 빌드하거나 이전207쪽
시각 증거를 새 실행으로 합산하지 않았다. 새 합성 입력의 WASM/한컴 시각 검증은 미실행이다.

fmt check, 변경 integration target Clippy(-D warnings), 고정 base
`7a95e46e025470a4d7a7b59ad68ec02958bda738` 대비 manifest 검사는 각각
`domain-fmt.log`, `domain-clippy-test.log`, `domain-policy.log`에 남긴다.
이는 내부 체크포인트이며 전체 workspace 제출 CI 완료가 아니다. 다음은 빈 문단을 보존한 채
문단 구성 결과의 점유 끝/전진 원점을 분리하고, 유효한 저장 TAC와 셀 종료 근거를 연결하는 작업이다.
종료 후보 활성화·baseline/ignore 변경·Legacy 변경·원격 push/PR·R5 완료 판정은 하지 않았다.

최종 테스트 파일 SHA256은 `40972026360ae35969ba341b43d943f9eaf4154eed60aa049a16baef01c858a3`다.
제품/review의 tracked `src/`, `crates/`, Cargo 파일1346개를 byte 비교하여 차이0을 확인했다.
최종 문단 검사 명령은 review에서 `CARGO_BUILD_JOBS=1`과 공유 `CARGO_TARGET_DIR`을 지정한
`node scripts/run-rust-test.mjs issue_7353_table_v2_text -- --no-fail-fast`이며 최종 배정은
`regression_suite_007`이다. 이 target Clippy와 fmt/policy도 모두 PASS다.
도중 test source 크기 변경 뒤 파생 suite 재준비 전 실행은0건으로 실패했으며 성공으로 세지 않았다.
`--prepare` 재실행 후 위14건을 실제 실행한 결과가 최종 증거다. 다른40건 case source와
제품 구현은 그 검증 뒤 변경하지 않았다.

### A 후속 — 문단 종료 결과 분리, 기존 출력 보존

시작 head는 `660fef3ef245c74fbd8716becbce89e38931dac6`이며 제품 worktree는 clean이었다.
이번 승인에서는 보류된 종료 높이 후보를 활성화하지 않고 문단의 실제 점유 끝, 다음 원점,
문단 밖의 명시적 공간을 구분하는 구조를 구현했다. 저장 정보 수용 조건·Legacy·IR 포맷·
기준값·ignore는 변경하지 않는다. 빈 문단 삭제나 문서별 높이 보정도 추가하지 않는다.

`table_v2/paragraph_end.rs::ParagraphEnd`는 이미 구성된 줄/개체 recipe에서 물리 점유의
최댓값과 줄별 advance를 구분한다. 음수 간격 TAC에서는 물리 끝36과 pen30이 다를 수 있다.
끝 공간은 producer가 계산한 순서와 수치 그대로 보존한다. 텍스트와 그림의 결합된 끝 공간,
TAC의 양수 줄간격/문단 뒤 간격을 임의로 합치지 않아 기존 부동소수점 덧셈 순서도 유지한다.

실제 생산·소비 경로는 다음과 같다. 경로는 모두 `src/renderer/table_v2/` 기준이다.

| 생산 결과 | 변환·측정 소비 | 실제 배치·최종 원점 |
| --- | --- | --- |
| `text.rs::TextComposer::compose`의 실제 줄 상자와 끝 공간 | `ParagraphEnd::from_composed` → `ir.rs::bind_table`/`text_flow.rs::from_flow_rows`/`document_input.rs::prepare`의 `into_flow_items` | 기존 `FlowBlock` → `content.rs::physical_extent` → `flow.rs::FlowCursor::fit` → `text.rs::TextFragment::append_to` |
| `pictures.rs::compose`의 저장 ObjectRow와 끝 간격 | 동일 변환, ObjectRow 소유 줄 유지 | `text_ir.rs`의 paint slot과 기존 이미지 원점 사용. End 자체는 그릴 노드가 아님 |
| `tac.rs::compose`의 InlineTables 높이/advance와 끝 간격 | 중첩 IR/본문 모두 동일 변환. declared table height를 재계산 근거로 쓰지 않음 | `flow.rs`의 원자적 줄 fit은 height로 검사하고 advance로 pen을 전진. 각 자식의 실제 배치/종료 검사 유지 |

이번 단계에서 End 메타데이터가 fit 알고리즘을 대체한 것은 아니다. 공통 변환은 기존 끝
공간을 같은 `FlowBlock::Space`로 내려 보내며, 측정과 배치는 그 결과를 소비한다. 점유 끝과
후속 원점 getter를 추가했다는 이유만으로 셀 끝 정책이나 페이지네이션 개편이 완료됐다고
주장하지 않는다. 이후 정책 판단의 입력 경계가 마련된 상태다. 후속 자리차지 앵커 조정과
padding/수직 정렬은 기존 경로 그대로이며 이번 End로 덮어쓰지 않는다. 일반 어울림·혼합
재조판은 여전히 미지원이다.

분할 경로는 `FlowCursor::fit`의 기존 블록/space_left/child 소유 컷을 유지한다. 실제 수용한
Space만 예약하며 줄 예산 실패는 해당 줄을 이월한다. TAC height/advance 분리·자식의 원자적
fit·완료 검사도 변경하지 않는다. rowspan 특수 분기나 조각 clip을 수정하지 않았으므로 새
분할 규칙의 증거로 세지 않는다. 기존 분할·rowspan·제목 반복 대조군으로 출력 보존을 확인한다.

정식 `tests/cases/issue_7353_table_v2_nested.rs`에 2건을 추가했다.

- `paragraph_end_distinguishes_occupied_end_and_next_origin`: 12+6+2의 후속 원점20과
  물리 끝12, TAC 물리 끝36/전진30+2의 차이, 비유한/음수 공간 거부를 검사한다.
- `paragraph_end_bands_and_external_space_keep_blank_line_ownership`: 합성10높이 빈 줄의
  End(6+2) 뒤에 명시적 공간0/2/7을 넣어 최종 두 LineOwner·원점 차이18+추가 공간·
  예약 높이28+추가 공간·완전 종료를 검사한다. helper 값만 검사하지 않고 IR→fit 결과를 검사한다.

이 2건은 새 표현의 합성 계약이지 기존 사용자 결함의 수정 전 FAIL 증거가 아니다. 이전
0 공간 항등성/명시 공간 가산성, 4/14pt·156% 빈 문단/이월 검사도 함께 실행했다. 한컴
정상 생성본의 종료 간격 적용을 이 합성 결과로 대신하지 않는다.

최종 `output/7353/r19/terminal/end-tests.log`는 **221 PASS**, 미선택2549건이다. 기존 V2와
shared control 대조군의 현재 suite 배정/실행 명령은 로그 첫 줄에 남겼다. 초기 focused14건은
최종 구현 전 결과라 최종221건과 합산하지 않는다. `end-fixtures`의 Native JSON111개는 기존
`checkpoint-fixtures`111개와 전부 byte 동일하다. 이 비교는 기존 결과 보존의 증거이며
한컴 정답 일치로 해석하지 않는다. 추가 lint/fresh WASM/직접 시각 확인은 아래에 연결한다.

최종 검증은 같은 review overlay에서 순차 실행했다. `end-fmt.log`,
`end-clippy-native.log`(`cargo clippy --locked -- -D warnings`),
`end-clippy-wasm.log`(`-p rhwp --lib --target wasm32-unknown-unknown`),
`end-clippy-test.log`(`-p rhwp --test regression_suite_021`) 모두 PASS다.
`end-policy.log`의 manifest 검사는 고정 base `7a95e46e025470a4d7a7b59ad68ec02958bda738`
대비 PASS다. source-side unit test는 변경하지 않았다. 제품/review의 소스·Cargo1347개
byte 차이0과 변경 파일별 SHA256은 `end-source-comparison.json`에 기록했다.

`docker compose --env-file .env.docker -p rhwp run --rm wasm`은7분50초에 성공했다
(`end-docker-wasm.log`). fresh `pkg/rhwp_bg.wasm` SHA256은
`517e088ca4697622edefb4f2568ed3ffa895b798842b54ae30f2ecf78a6a02b8`다.
`verify-table-v2-preview-wasm.mjs`를 `--pkg pkg --fixtures output/7353/r19/terminal/end-fixtures
--out output/7353/r19/terminal/end-browser --dependencies-root /home/edward/mygithub/rhwp`와
지원하는20개 선택 flag로 실행했다: `--split-line-property --solid-backgrounds
--fractional-cell-fit --stored-margins --pictures --stored-frames --linear-backgrounds
--nested-alignment --cell-vertical-align --document-flow --stored-body --tac-metrics
--stored-tac --structural-tac --empty-page-borders --page-numbers --rowspan --solid-borders
--split-borders --matching-table-borders`. Chrome 경로는
`/home/edward/.cache/puppeteer/chrome/linux-146.0.7680.31/chrome-linux64/chrome`다.

`end-browser.log`/`end-browser/manifest.json`은 **207쪽 exact Native/fresh WASM tree·SVG
parity, isolation/rejection/rollback/termination PASS**다. 이번 Native111개와 이전
`checkpoint-fixtures`의 byte 동일성은 `end-native-comparison.json`, 이전 `browser`와
이번 review PNG207개의 byte 동일성은 `end-browser-comparison.json`에 별도 기록했다.
검증 이후 engine/test는 변경하지 않았다.

직접 연 대표 증적은 `output/7353/r19/terminal/end-browser/` 아래의 다음 파일이다.

- `fractional-source-0.review.png`와 standalone `fractional-source-0.overlay.png`:
  원본 #6923 첫 선택 표의 외곽·12셀·2그림 상대 위치가 두 backend에서 같다.
  원본은 `tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp`,
  독립 기준은 같은 stem의 `-2020.pdf` p1이다. 이번 compare의 직접 기준은 Native이며,
  앞서 기록한 한컴 PDF와의 높이/좌측 로고 차이를 해소한 것으로 주장하지 않는다.
- `document-inline-signed-budget-0.review.png`/`-1.review.png`: 앞 문단만 첫 쪽에 남고
  같은 줄의 두 TAC와 after는 다음 쪽에 함께 조판된다. 음수 간격의 후속 원점도 그대로다.
- `stored-frame-empty-center-0.review.png`: 빈 저장 줄 다음 after의 위치가 보존된다.
  빈 줄 점유 자체는 앞의 정식 좌표 계약으로 검증하며 눈에 글자가 없다는 이유로 판단하지 않는다.
- `picture-source-1-0.review.png`: 원본 그림 payload의 위치/크기가 두 backend에서 같다.

각 review는 Native/fresh WASM/overlay3열 compare이며 별도의 `.native.png`, `.wasm.png`,
`.overlay.png`도 같은 stem에 있다. `visual_accuracy_proxy_percent`는 이 backend 비교에서
산출하지 않았다. PNG byte/JSON 비교만으로 시각 통과를 선언하지 않고 위 대표 페이지를 직접
확인했다. 새 End 전용 합성 입력의 Hancom 시각 대응은 미검증이며, 이번 browser207쪽에 새
End constructor 합성 사례가 별도 fixture로 추가된 것은 아니다.

내부 커밋은 `end-verified-commit.txt`에 연결한다. 전체 workspace 제출 CI·원격 push/PR·
기본값 전환은 하지 않았다. 다음 우선순위는 이미 분리한 결과를 바탕으로 **유효한 저장 TAC의
셀 끝 간격과 선언 점유 envelope를 독립 근거에 연결하여 원본 문서 수용을 진행**하는 것이다.
보류 후보/기대값 일괄 갱신은 하지 않으며 A/R5 완료로 세지 않는다.

### 셀 끝 후행 줄간격 — 선택형 원본 검증

승인된 다음 절편은 `5bcf48178`에서 시작했다. 전체 정책 전환 전에 원본 입력으로
격리 검증하도록 선택 표 byte/JSON API에 `cell_end_policy: "omit_final_line_gap"`을
추가했다. 생략 시 `preserve_advance`가 기본이며 Legacy와 본문 V2 경로는 바꾸지 않았다.
줄 상자 자체, 빈 문단, 문단 뒤 간격, 셀 padding, 명시적 Space는 삭제하지 않는다.
비영 문단 뒤 간격의 한컴 셀 끝 의미는 아직 확정하지 않고 기존 값을 보존한다.

생산→소비: `text.rs`/`pictures.rs`/`tac.rs`의 `ParagraphEnd::from_composed`가
기존 후행 공간과 terminal 공간을 분리한다. `ir.rs::bind_table`은 셀의 마지막 문단에,
`text_flow.rs::from_flow_rows_with_end_policy`는 마지막 내용이 문단인 경우에만
`paragraph_end.rs::into_flow_items_at_end`를 적용한다. 후속 Space0/2/7은 문단 소속을
바꾸지 않으며 후속 explicit Table은 문단 끝 간격을 보존한다. IR의 중첩 표에는 같은 정책을
전달하고 이미 준비된 explicit 자식 표는 자체 plan을 보존한다. 사용자 composer의 기존
`ParagraphEnd::new` 계약은 공간을 추측해 제거하지 않는다.

변환된 동일 FlowBlock을 `content.rs`의 요구 높이와 `flow.rs::fit_cell`의 컷/예약이 소비하고,
`text.rs::TextFragment::append_to`는 그 placement를 그린다. 음수 TAC 전진과 물리 끝은
계속 분리한다. 별도 높이 clamp·paint 확대·저장 envelope 검사 완화는 없다. 기존 rowspan,
제목 반복, clipping 분기는 변경하지 않았으며 이번 종료 정책을 그 모든 경로에서 독립 검증한
것으로 주장하지 않는다. 예산 실패/이월 및 마지막 유닛 종료는 아래 합성 계약으로 검사했다.

정식 tests/cases에 추가한4건:

- `issue_7353_table_v2_text`: 재조판/수동 저장 입력, 12px 빈 줄·18px pitch·문단 뒤2px·
  padding3/4px의 두 문단은41px. Space0/2/7 가산성, 두 실제 LineOwner/원점, 34px 예산의
  빈 줄 이월과 완료를 두 adapter에서 검사한다. 후속 explicit 표가 있으면 앞 문단의6px
  간격이 유지되는 별도 검사도 추가했다. 이는 합성 계약이며 정상 한컴 저장본 증거가 아니다.
- `issue_7353_table_v2_document_flow`: 자식 최소 높이36px을 명시한 합성 TAC 입력에서
  -1/+4px 간격 모두39px 예산은 거부,40px은 수용한다. 실제 자식 y/높이·내용·완료를 검사하며,
  최소 높이 없는 수동36px envelope는 실제30px 내용과 달라 여전히 거부한다.
- `issue_7353_table_v2_export`: 변경하지 않은 원본 #6923 첫 표를96/7200dpi에서 실행한다.
  독립 원본 행 근거582(빈 줄300+padding282)+2282+3042+1922+3274=11102HU를 검사한다.
  첫 빈 줄300HU·12셀·2그림·한 조각/완료·최종 Table 높이를 확인한다.

원본은 `tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp`, 기준은
같은 stem의 `-2020.pdf` p1이다. 기본 모드12598HU와 선택 모드11102HU를 같은 원본에서
비교했다. 이전 head 자체에 새 옵션 테스트를 적용한 FAIL/PASS 실행은 하지 않았으므로
그 증거로 보고하지 않는다. 기본 결과의 이전 head 동일성은 아래111개 비교로 확인했다.

증적은 `output/7353/r19/terminal/policy-*`다. `policy-tests-final2.log`는 최종 파생 suite에서
225 PASS/3273 미선택이다(이전 `policy-tests-final.log`도225 PASS).
합성 테스트의 비공개 API/기본 Justify 설정 오류는 공개 API/명시 Left로 바로잡았고 production의
수용 조건은 바꾸지 않았다. 테스트 수정 뒤 파생 suite drift가 발생해 review worktree에서
`--prepare`를 다시 실행했다. `policy-manifest-final.log`는 고정 base
`7a95e46e025470a4d7a7b59ad68ec02958bda738` 대비 PASS다. 파생 파일은 제품에 포함하지 않는다.

lint는 review overlay에서 순차 실행했다. `policy-fmt.log`의 `cargo fmt --all -- --check`,
`policy-clippy-native.log`의 `cargo clippy --locked -- -D warnings`,
`policy-clippy-wasm.log`의 `-p rhwp --lib --target wasm32-unknown-unknown`,
`policy-clippy-tests-final.log`의 `-p rhwp --test regression_suite_014 --test regression_suite_021
--test regression_suite_022 -- -D warnings` 모두 PASS다. 공통 target-dir은
`/home/edward/mygithub/rhwp/target/pr-review`, `CARGO_BUILD_JOBS=1`을 사용했다.
source-side unit test는 변경하지 않았다. 검증 이후 Rust/테스트는 변경하지 않았다.

Docker fresh WASM은7분28초 성공(`policy-docker-wasm.log`), SHA256은
`9fe40fe35938944300279a2dedb15f26812892f50934d1f11cf458d159fa6f89`다. 이전 기록의 browser 명령에서
fixtures/out을 `policy-fixtures`/`policy-browser`로 바꾸고 `--terminal-cell-end`를 추가했다.
`policy-browser.log`는208쪽 exact tree/SVG parity·isolation/rejection/rollback/termination PASS다.
`policy-preservation.json`: 기존 Native111개와 review PNG207개는 이전 결과와 byte 동일하다.
`policy-source-comparison.json`: 제품/review의 Rust·Cargo1074개 차이0 및 변경 파일 hash를 기록했다.

직접 시각 확인: `policy-browser/terminal-source-0.review.png`의 Native/fresh WASM 동일성,
`policy-pdf-review.png`의 PDF/기존/선택 Native/선택 WASM/overlay를 확인했다.
`compare-policy.mjs`는 PDF trace의 x60.088pt, y(841−767.281)pt를 사용해96dpi에서 위치만
맞춘다. 높이/폭 변형은 없다. `policy-pdf-overlay.png`는 독립 standalone overlay다.
PDF 첫 표 높이는147.8387px, 선택 출력은148.0267px(약0.188px 차이)이며 외곽과 셀 경계가
기존167.9733px보다 가까워졌다. 좌측 로고의 표시 차이와 글꼴/글자 모양 차이는 남는다.
PDF crop 아래의 후속 문장은 selected-table preview 범위 밖이다. 원본 전체 수용·뒤 문단의
위치·전체 pagination을 이 비교로 입증하지 않으며 최종 메인테이너 시각 통과도 대신하지 않는다.
정상 대조군 `document-inline-signed-budget-1.review.png`도 직접 확인했다.

남은 일은 이 선택형 근거를 본문 TAC 수용과 연결하고 원본 전체의 다음 미지원 경계를 검증하는
것이다. 기존 baseline/golden/ignore는 수정하지 않았다. 기본값 전환·원격 push/PR·전체 CI 상당
제출 검증은 수행하지 않았으며 A/R5 완료가 아니다.

### 셀 끝 정책의 본문 연결 — 진행 기록

이번 절편 시작 head는 `2eef95686`이다. 선택 표에서 검증한 정책을 본문 옵션으로 전달하되,
셀 내부에만 적용한다. `document.rs::Options` → `document_input::prepare`의 저장 TAC 및
TopAndBottom 두 준비 호출 → `PreparedTextTable::prepare_with_end_policy` → 기존 재귀
IR/FlowBlock으로 이어진다. 본문 자체의 `paragraph_end::into_flow_items`는 그대로 두어
빈 문단·저장 TAC의 음수 전진·후속 문단 원점을 바꾸지 않는다. 준비된 표는 동일 plan으로
TAC 저장 점유 검사/FlowCursor 요구 높이·예산·이월과 document paint를 수행한다.
모든 콘텐츠 준비가 성공하기 전에는 부분 문서를 공개하지 않는 기존 계약도 유지한다.

독립 기대값: 원본 첫 표는11102HU, 본문 원점(5669,7087)HU에 저장 객체 원점(350,283)HU를
더한다. 저장 TAC 줄의 다음 전진은11668−800HU다. 첫 원본 carrier와 내부 속성을 보존하고
합성12px/18px 빈 문단·after를 붙인 파생 문서로 본문 연결/빈 줄 보존을 검사한다.
원본 전체가 아닌 파생 입력임을 구분하고 HWP 재파싱 후 첫 carrier JSON 동일성을 확인한다.
일반 자리차지 표는 합성12px/18px의 두 줄: 셀30px, 뒤 본문은18px 전진을 유지한다.
옵션 파싱만 추가하고 실제 전달을 끊은 review overlay를 음성 대조로 먼저 실행한다.

#### 연결 결과와 검증

실제 전달이 없는 음성 대조는3건 모두 의도한 원인으로 FAIL했다(`bridge-before.log`).
자리차지 표 높이36≠30px, 원본 carrier/전체는 para0의 저장 TAC 점유 불일치였다.
연결 뒤 합성 문단의 Fixed18px 입력을 올바른 HWP 단위2700으로 정정했다
(`style_resolver.rs`의 Fixed 값은2로 나누어 해석). 기대 좌표는 바꾸지 않았고3건 PASS다.
`bridge-tests-final.log`는선별228/228 PASS다. 이후 증적 저장 helper만 수정한 최종
`bridge-capture-final.log`에서3/3 재검증했다. 렌더러 JSON을 Value로 재직렬화하면서 생긴
부동소수점 끝자리 차이는 원문 JSON 보존으로 해결했고 exact parity를 완화하지 않았다.

명령/환경: review worktree, 공유 target/pr-review, CARGO_BUILD_JOBS=1.
228건 명령은 `bridge-tests-final.log` 첫 줄에, 최종 집중 명령은
`node scripts/run-rust-test.mjs issue_7353_table_v2_document_flow -- --locked document_terminal_policy`다.
최종 fmt check, native Clippy, wasm32 lib Clippy, 해당 integration target
`regression_suite_019` Clippy(`--locked`, `-D warnings`) 모두 PASS했다
(`bridge-fmt-final.log`, `bridge-clippy-{native,wasm,test}.log`).
테스트 source 수정 후 발생한 generated harness drift는 review에서 `--prepare`로 재생성해
`--check --base-ref 7a95e46e025470a4d7a7b59ad68ec02958bda738` PASS로 확인했다
(`bridge-policy-final.log`). 파생 파일은 제품 커밋 대상이 아니다. source unit test 변경은 없다.

Docker `docker compose --env-file .env.docker -p rhwp run --rm wasm`은7분28초 성공했다.
WASM SHA256: `028a077752792612442c6e2c2dd5849518c526ee79b8bc95a1ba57bc600a2dec`.
기존 browser 명령에 `--document-terminal`을 추가하고 fixtures/out을
`bridge-fixtures`/`bridge-browser`로 지정했다. `bridge-browser.log`:210쪽 exact tree/SVG
parity·isolation/rejection/rollback/termination PASS. `bridge-preservation.json`은 기존
Native113개·review PNG208개 byte 동일을 확인한다. `bridge-source-comparison.json`은
제품/review Rust·Cargo1074개 차이0, 최종 테스트 동일성·hash를 기록한다.

직접 확인한 증적은 모두 `output/7353/r19/terminal/` 아래다.
- `bridge-browser/document-terminal-{source,anchor}-0.review.png`: Native/fresh WASM 표,
  뒤 문단 배치와 overlay 확인. 원본 carrier의 저장 음수 전진과 합성 빈 줄을 보존한다.
  쪽번호 `- 1 -`는 본문 밖 footer에1회 배치되는 것을 실제 WASM 좌표로 검사한다.
- `bridge-pdf-review.png`, `bridge-pdf-{native,wasm}-overlay.png`: 원본 PDF 첫 쪽과
  파생 문서 첫 표를 같은96dpi 좌표(x72,y90,w660,h160)로 비교했다. 위치/크기 정렬 변형은
  하지 않았다(`bridge-pdf-compare.mjs`, `bridge-pdf-scope.json`). 외곽·셀 경계는 가깝지만
  로고 표시와 글꼴/글자 모양 차이는 남는다. 합성 after 및 원본 나머지 본문은 PDF 비교 밖이다.
  독립 V2 browser 경로의 직접 비교이며 일반 CLI 전체 Visual Sweep/자동 fidelity 점수는 미실행이다.

원본 전체는 첫 표를 통과해 paragraph index1에서
`Unsupported("text preview paragraph decoration or keep")`로 멈춘다.
`6923-terminal-next-source.json`: 해당 문단은 빈 문자열, 저장 높이1000HU/줄간격200HU,
`border_fill_id=1`, attr1=268이며 keep/page-break bit는 없다. `text.rs`는 비영 border ID를
일괄 거부한다. 참조 BorderFill1의 실제 테두리는 None이고 solid 배경값은0xffffffff다.
다음은 이 참조의 실제 표시 의미와 빈 문단의 저장 줄 수용을 확인하는 일이다. ID를 지우거나
빈 문단을 건너뛰어 수용시키지 않는다. 원본 전체 성공/메인테이너 시각 통과/A·R5 완료가 아니다.
기본 V2/Legacy·baseline/golden/ignore는 그대로이며 원격 push/PR·전체 CI 상당 검증은 하지 않았다.

### 비표시 문단 장식 참조와 원본 빈 줄 — 진행 기록

시작 head `b846a0cd4`. 본문 두 번째 문단은 borderFill1을 참조하지만4방향 선은None,
solid 색은0xffffffff(CLR_INVALID), 패턴은없음이다. 공통 style resolver도 fill_color=None으로
해석한다. 참조 번호 자체를 실제 표시 효과와 구분하며 원본 IR/참조를 지우지 않는다.
입력은 기존 원본 HWP와 대응 PDF를 유지한다. HWPX 합성 no-fill은 FillType::None으로
표현하며 원본 HWP sentinel은 원본 저장 속성을 그대로 보존한 별도 계약으로 검사한다.

`document_input`/표 재귀 `decoration::validate_source`가 원본 paragraph borderFill의
누락·실제 선·배경·source-only 효과를 먼저 검사한다. `TextComposer`는 resolved 참조도
비표시인지 확인한다. 원본 빈 문단의 내어쓰기는 글자 배치가 없으므로 저장 물리 줄을 보존한다.
문자가 있는 저장 들여쓰기 문단은 기존 제한 유지다. `stored_text::localize` → 공통
physical-frame paint → `validate_paint` → ParagraphEnd/FlowBlock → body/cell의 동일
payload·fragment를 소비하며 높이를0으로 만들거나 끝 원점을 재추정하지 않는다.
저장 줄 높이1000HU·간격200HU, 본문 원점7087HU+첫 TAC 전진10868HU에 따라
빈 줄 y17955/75px, 뒤 합성 문단 y19155/75px가 독립 기대값이다.

원본 첫2문단을 그대로 보존하고 합성after만 붙인 파생 문서로 연결을 확인한다. 첫2문단의
HWP 재파싱 JSON 동일성과 빈 줄 상자 x/width/y/height·후속 원점을 검사하며 원본 전체
피델리티의 증거로 확대하지 않는다. 합성 HWPX는 ID0과 비표시 ID2의 전체 출력 동일성을,
반례는 실제 선/흰 배경/3D/중심선/패턴/누락 참조를 본문과 셀에서 검사한다.
초기 합성 테스트는 BorderLine 기본값이Solid인 입력 오류가 있었다. 이를None으로 명시하고,
3D의 직렬화 원본 attr도 명시했다. 수정 전 native 기록은 이 초기 입력이므로 최종 음성 대조는
보존한 직전 WASM과 최종 두 입력으로 다시 수행했다(`noop/before-wasm.json`): 두 입력 모두
문단 장식 거부를 확인했다. 기존3D 등 반례는 해당 원본 속성이 표현되는 입력으로 확인한다.

#### 결과와 검증 범위

증적은 `output/7353/r19/noop/` 아래다. `before-wasm.log/json`은 직전 WASM
`028a077752792612442c6e2c2dd5849518c526ee79b8bc95a1ba57bc600a2dec`에 최종 입력을
넣어 두 양성 사례의 수정 전 거부를 확인한다. 새 구현은 두 입력을 수용하고 저장 빈 줄·
후속 원점 계약을 통과한다. `tests-final.log`의 선별231건 중229건 통과/2건 실패는 위 합성
입력 오류와 기존 원본 probe의 정확히6건 수용 제한이었다. 후자는225개 원본 문단의 개별
수용으로 확대됐지만 기대 수를225로 바꾸지 않았다. 비표시 참조만 제거한 테스트 대조군과
수용 여부·전체 최종 출력 동일성을 검사하고, 기존 무장식6건 및 저장 줄 좌표 검사를 유지했다.
이는 고립 문단 probe이지 원본 전체 수용이나 한컴 피델리티 판정이 아니다.
최종 `document-final.log`27/27과 `text-final.log`16/16을 재실행했다. 동일 렌더러에서 이미
통과한 나머지188건과 합쳐231건을 검증했으며, 최종231건 단일 실행으로 보고하지 않는다.

review worktree에서 `node scripts/run-rust-test.mjs`로 위 두 case를 실행했다.
`fmt-final.log`, `clippy-native.log`, `clippy-wasm.log`, `clippy-tests.log`는 fmt check,
native/WASM lib/해당 integration target Clippy(`--locked`, `-D warnings`) PASS다.
마지막 테스트 주석 정정 뒤 review의 파생 suite를 다시 준비했다. `policy-final.log`는
`--check --base-ref 7a95e46e025470a4d7a7b59ad68ec02958bda738` PASS다.
파생 suite는 stage하지 않는다. source unit test 변경은 없다. 전체 workspace 제출 게이트는
이번 내부 절편에서 실행하지 않았다. `source-comparison.json`은 제품/review Rust·Cargo
1074개 차이0과 최종 테스트 동일성, 테스트 후 주석만 정정한 범위를 기록한다.

Docker `docker compose --env-file .env.docker -p rhwp run --rm wasm` 성공(7분31초,
`docker-wasm.log`). WASM SHA256은
`3afd9baead27374bb4fc4d96cb57fb0e667f0c9ec8a6307b29b79e08b26f4304`다.
기존 browser 검증 명령에 `--noop-paragraph-border`를 추가하고 fixtures/out을
`noop/fixtures`, `noop/browser`로 지정했다. `browser.log`는213쪽 exact Native/WASM
tree·SVG 일치 및 isolation/rejection/rollback/termination PASS다. `preservation.json`은
이전 Native115개와 review PNG210개 byte 동일을 확인한다. Docker 시작 후 렌더러 변경은 없다.

`pdf-review.png`, `pdf-native-overlay.png`, `pdf-wasm-overlay.png` 및
`browser/document-noop-{source,border}-*.review.png`를 직접 확인했다. PDF 비교는 원본 첫
표 영역의 동일96dpi 좌표(x72,y90,w660,h160)이며 위치·크기 정렬 변형은 없다.
외곽/셀 경계는 가깝고 기존 로고 표시·글꼴 차이는 남는다. 합성after와 원본 나머지 본문은
PDF 비교 범위 밖이다(`pdf-scope.json`). 빈 줄의 높이는 보이지 않는 PNG만으로 추정하지 않고
최종 tree 좌표로 검사한다. 일반 CLI 전체 Visual Sweep/자동 fidelity 점수는 미실행이며
메인테이너의 최종 시각 통과를 대신하지 않는다.

원본 전체의 다음 거부는 index2의 `text preview run outside occupied line`이다.
해당 제목 문단은 저장 높이1900HU/간격380HU, 가운데 정렬이며, 다음 조사는 실제 glyph
점유와 저장 줄 상자의 관계다. 원인은 아직 확정하지 않았다. 기본 V2/Legacy·baseline·golden·
ignore는 유지하며 원격 push/PR은 수행하지 않았다. 원본 전체 수용 및 A/R5 완료가 아니다.

### 말미 공백의 논리 폭과 표시 점유 분리 — 진행 기록

시작 head `fd68bb6b3`. `run-extent/diagnostic.log`에서 원본 index2의 마지막 U+0020
런만 오른쪽을 넘는다: 줄642.506667px, 공백 시작632.764214px, 폭10.689452px,
끝643.453667px(초과0.947px). y/height/기준선·원본 저장 줄 검사는 통과한다.
제목은 원본 HWP의 저장1900HU 높이/380HU 줄간격과 실제 27자 제목+공백을 그대로 사용한다.
대응 PDF p1에는 제목의 가시 글자가 본문 안에 있고 말미 공백에는 표시 효과가 없다.

공통 문단 paint가 발행한 TextRun의 논리 advance를 줄 점유로 오인한 V2 수용 검사다.
`TextComposer`의 공통 paint → 저장 줄 일치 검사 → 표시 inline 끝 검사 → 기존 ParagraphEnd/
FlowBlock → 본문/셀 조각 paint로 이어진다. 새 검사는 backend와 같은 replay positions에서
장식 없는 말미 U+0020만 표시 끝과 구분하며, 텍스트·bbox·저장 줄·payload를 수정하지 않는다.
줄 높이/후속 원점은 계속 기존 저장 줄 결과에서 계상한다. 밑줄·취소선·음영·테두리 등의
효과가 있거나 내부 공백·가시 글자이면 기존 전체 폭 검사를 유지한다. 분할/예약 로직 변경은 없다.
합성 RIGHT 정렬 계약은 말미 공백 유무/런 분할에 무관한 가시 글자 원점과200HU 줄 폭,
12HU 높이/18HU 다음 원점을 검사한다. 수정 전 `before.log`는 의도한 범위 거부로 FAIL했다.

#### 집중 검증 결과

증적은 `output/7353/r19/run-extent/` 아래다. `before-wasm.log/json`은 직전 패키지
`3afd9baead27374bb4fc4d96cb57fb0e667f0c9ec8a6307b29b79e08b26f4304`가 원본 앞3문단과
복사 빈 문단으로 구성한 최종 파생 입력을 같은 이유로 거부함을 확인한다.
새 `document_original_title_keeps_trailing_space_and_saved_flow` 계약은 앞3문단의
HWP 재직렬화/재파싱 JSON 동일성, 제목 y19155/75·높이1900/75·폭48188/75px,
다음 빈 문단 y21435/75px와 공백 런의 원래 초과 폭 보존을 실제 최종 tree에서 검사한다.
합성 계약은 단일/분리 런, 말미 공백 없는 정렬 대조군, 밑줄/취소선/음영 및 가시 글자
초과 반례를 확인한다. 어느 경로에서도 bbox를 줄이거나 원본 공백을 삭제하지 않았다.

`after-text.log`17/17 PASS, `document-final.log`28/28 PASS에 이어
`selected.log`는233/233 PASS다. 실행 명령은 로그 첫 줄과 `run-selected.mjs`에 고정했다.
중간 `after-document.log`의1건은 원본 전체 거부 지점이 index2에서 index4로 이동한
수용 범위 계약이며, 다음 실제 거부 이유를 확인한 뒤 갱신했다. 문서 출력 기대 좌표나
baseline을 완화한 것은 아니다. 원본 나머지를 수용했다고 보고하지 않는다.
review worktree에서 fmt check, native/WASM lib/변경 integration suite Clippy를 순차로
실행해 모두 PASS했다(`fmt.log`, `clippy-{native,wasm,tests}.log`, `--locked -D warnings`).
`policy.log`는 고정 base `7a95e46e025470a4d7a7b59ad68ec02958bda738` 대비 PASS다.
파생 suite는 커밋하지 않는다. source unit test 변경은 없고 전체 workspace 제출 게이트는
이번 내부 절편에서 실행하지 않았다.

Docker `docker compose --env-file .env.docker -p rhwp run --rm wasm`은7분23초 성공했다
(`docker-wasm.log`). 컨테이너의 text.rs SHA256도 작업본과 동일했다.
최종 WASM SHA256: `156d1831f32c3201a5e48e51e03392fa892ccbb577b2f5612274bbf45c91e887`.
기존 browser 명령에 `--trailing-space`를 추가하고 fixtures/out을
`run-extent/fixtures`, `run-extent/browser`로 지정했다. 첫 시도는 Chrome 시작 단계에서
실패했고(`browser.log`), 같은 조건 재실행은 성공했다. 시작 실패의 정확한 원인은 미확정이며
제품 렌더링 실패나 통과 증거로 세지 않는다. `browser-retry.log`:214쪽 exact Native/WASM
tree·SVG 일치, isolation/rejection/rollback/termination PASS.
`preservation.json`은 기존 Native117개·review PNG213개 byte 동일을 확인한다.
`source-comparison.json`은 제품/review Rust 및 루트 Cargo1049개 차이0·테스트 동일성·패키지 hash를
기록한다. Docker 시작 후 렌더러 변경은 없다.

`pdf-review.png`를 먼저 열고 `pdf-{native,wasm}-overlay.png`,
`browser/document-trailing-source-0.review.png`를 직접 확인했다. PDF 비교는 원본 첫 표와
제목을 포함한 동일96dpi 좌표(x72,y90,w660,h196)이며 이동/축소 정렬은 하지 않았다
(`pdf-scope.json`, `compare-pdf.mjs`). 제목과 밑줄은 본문 안에 배치된다. PDF와 글꼴 굵기·
자간, 기존 로고 표시 차이는 남는다. 복사한 뒤쪽 빈 문단과 원본 후속 본문은 비교 판정 밖이다.
일반 CLI가 아닌 명시적 V2 browser 경로로 검증했으며 `visual_accuracy_proxy_percent`는
미계측이다. Native/WASM 일치·계약 통과를 한컴 피델리티 최종 통과로 바꾸지 않는다.

다음 원본 경계는 index4의 저장 줄1400HU/간격-140HU(문단90%) 수용이다. 현재 일반
텍스트의 stored_text::localize는 음수 간격을 거부한다. 이를0으로 보정하지 않고 줄 점유와
다음 전진의 계약을 조사해야 한다. 원본 전체/A·R5 완료가 아니다. Legacy/default·
baseline/golden/ignore 변경 및 원격 push/PR은 없다.

### 음수 텍스트 줄간격의 점유/전진 분리 — 진행 기록

시작 head `4bd1fcb27beb64a7db2374279ee20e24c5af40f9`. 증적은
`output/7353/r19/signed-text/` 아래다. 원본 index4는 높이1400HU와 간격-140HU를
저장하며 다음 문단의 vpos17288HU는16028+1400-140과 일치한다. 원본 HWP 앞5문단을
그대로 보존하고 뒤에 원본 빈 문단을 복사한 파생 입력을 사용했다. 재직렬화/재파싱 뒤
앞5문단 JSON 동일성을 검사한다. 대응 PDF p1의 두 부제까지가 독립 출력 비교 범위다.
뒤에 붙인 빈 문단과 원본 후속 표는 이 파생 출력의 피델리티 판정 범위가 아니다.

`stored_text::localize`는 전체 높이+간격이 양수이고 저장 원점이 해당 전진보다 앞서지
않는 줄을 수용한다. 페이지/단 재시작, 소유권 손상, 역행·비전진 줄은 여전히 거부한다.
`TextComposer::compose`의 공통 paint 노드/문단 끝 → `ParagraphItem::Lines {height, advance}`
→ `ParagraphEnd::from_composed` 및 `TableContentPlan::physical_extent`
→ `FlowCursor::fit` → 본문/셀 최종 노드로 같은 점유/전진 결과를 전달한다.
공통 paint 뒤에 높이를 축소하거나 원점을 clamp하지 않는다. 저장 정보 경로는 실제 paint의
원점·높이·기준선·줄 소유·다음 전진까지 원본과 다시 대조한다.

본문 `document_input`, IR 셀 `ir::bind_table`, 명시적 셀 `text_flow` 모두 같은 Lines
결과를 낮춘다. 각 줄은 전체 소유 유닛이다. `FlowCursor::fit`는 pen+height로 수용 여부를
판단하고, 예약 끝은 max(기존 끝, pen+height), 다음 pen은 pen+advance다. 맞지 않으면 줄의
cursor/내용을 소비하지 않고 다음 쪽에서 그대로 배치한다. 셀 끝 정책은 마지막 줄의 음수 gap만
제외할 때 advance를 그 줄의 물리 높이로 복원하며, 문단 뒤 간격은 보존한다. TAC 분기·rowspan
컷·caption/각주 지원 범위에는 변경이 없다. 일반 본문/셀 모두 같은 fit 경로를 사용한다.

합성 저장 계약은12HU 줄2개, 시작0/10HU, gap-2HU와 후속 문단을 사용한다. 예산21HU에는
첫 줄만 들어가며 둘째 줄의 전체 끝22HU를 숨겨 넣지 않는다. 이어받기/전체 배치에서 A/B/C
소유, 실제 y/높이, 종료를 검사한다. 비전진 gap과 저장 시작9HU 반례는 거부한다.
재조판 계약은16HU·75%의 정확한 -4HU gap을 사용하여 빈 문단과 문단 뒤3HU를 보존한다.
본문29px 예산에는16px 줄2개만 들어가고 세 번째는 다음 쪽으로 이동한다. 셀 끝 정책별 실제
예약46/49HU도 검사한다. 이 수치는 마지막 줄의 물리 끝과 독립적인 입력 간격에서 도출한다.
초기 합성90% 테스트는 공유 구성기의4HU 양자화를 반영하지 않아 실패했다. 이어 시도한
고정 간격도 최소 줄높이를 보장하는 기존 계약과 달랐다. 이 입력 가정을 보존된 실패 로그에
남기고, 양자화가 필요 없는75% 입력으로 검증했다. 기존 golden/허용치를 갱신하지 않았다.

`before.log`는 수정 전 저장 경로의 의도한 거부로 FAIL이다. 최초 테스트 작성 중 API 인자
오류는 빌드 오류이며 결함 재현으로 세지 않는다. `before-wasm.json/log`는 직전 패키지
`156d1831f32c3201a5e48e51e03392fa892ccbb577b2f5612274bbf45c91e887`에서 원본 파생 입력은
저장 음수 간격 거부, 재조판 파생 입력은 물리 끝 거부로 실패함을 확인한다.
최종 `focused-final.log`49/49, `selected-pass.log`237/237 PASS다. 명령은 로그 첫 줄 및
직전 절편의 `run-extent/run-selected.mjs`에 고정했다. 중간 실패 로그는 삭제하지 않는다.
원본 전체의 수용 경계 계약은 실제 index5 `stored body anchor ownership` 거부를 확인한 뒤
갱신했다. 이 변경은 전체 원본의 성공이나 출력 baseline 변경이 아니다.

review worktree의 `fmt-final.log`, `clippy-{native,wasm,tests}.log` 모두 PASS다. 변경한
integration source5개가 속한 suite를 대상으로 Clippy를 실행했다. `policy.log`는 고정 base
`7a95e46e025470a4d7a7b59ad68ec02958bda738` 대비 PASS다. 파생 suite는 커밋하지 않는다.
source unit test 변경은 없고, 전체 workspace 제출 게이트는 이번 내부 절편에서 미실행이다.

Docker `docker compose --env-file .env.docker -p rhwp run --rm wasm`7분16초 성공
(`docker-wasm.log`). WASM SHA256은
`ea2c53487a2bdaac013bdbb5e0a92d6020251169be6e3805dec47ee3b4b03703`이다.
기존 browser 명령에 `--signed-text`를 추가하고 fixtures/out을 `signed-text/fixtures`,
`signed-text/browser`로 지정했다. `browser.log`:217쪽 exact Native/WASM tree·SVG 일치,
isolation/rejection/rollback/termination PASS. `preservation.json`: 기존 Native118개와
review PNG214개 byte 동일. `source-comparison.json`: 제품/review Rust·루트 Cargo1049개
차이0, 주요 테스트와 패키지 hash를 기록한다. Docker 시작 후 렌더러 변경은 없다.

`pdf-review.png`, standalone `pdf-{native,wasm}-overlay.png`,
`browser/document-negative-{source-0,fresh-0,fresh-1}.review.png`를 직접 확인했다.
동일96dpi 원본 좌표(x72,y90,w660,h240)만 잘라 두 부제까지 대조하며 위치/크기를 맞추는
변형은 하지 않았다. 원본 저장 줄의 높이와 다음 원점은 실제 tree 계약으로 확인했고,
기존 로고 유무·글꼴 굵기/자간 차이는 남는다. 빈 줄의 점유는 PNG의 가시 글자 유무로
판정하지 않는다. CLI 전체 Visual Sweep/자동 fidelity 점수는 미실행이며 합성 좌표 및
Native/WASM 일치를 한컴 피델리티 최종 통과로 승격하지 않는다.

다음 원본 경계는 index5의1x1 자리차지 표다. 저장 host 줄600HU/간격240HU,
문단 기준 세로720HU·바깥여백283HU·RowBreak를 가진다. 이 소유/앵커 해석은 아직 미지원이다.
원본 전체/A·R5 완료가 아니다. Legacy/default·baseline/golden/ignore 및 원격 상태는 유지한다.

### 저장된 본문 자리차지 앵커 — 진행 기록

시작 head `3976eb6b847e72de807b498d7a917b7e427d6716`. 증적은
`output/7353/r19/stored-anchor/` 아래다. 원본 index5는 빈 host 줄600HU/간격240HU,
Para/Top 오프셋720HU, 바깥여백283HU, flowWithText의1x1 표다. 부모 셀은87문단과
189665HU 높이를 가지므로 common.height51339HU를 전체 표의 실제 내용 높이로 대체하지 않는다.
`before.log`는 원본 앞6문단의 기존 앵커 거부를 보존한다. 직접 내용 준비 probe에서는
저장 들여쓰기를 거부했지만, 실제 문서 경로는 재귀 decoration 검사에서 내부 표 background zones를
먼저 거부한다. 원본 전체를 수용했다고 보고하지 않는다.

#### 규칙·소비 경로와 범위

저장 host의 텍스트/LineSeg를 보존한 공통 TextComposer → `ParagraphEnd`의 occupied_end /
next_origin → `body_anchor::BodyAnchor::resolve` → 본문 `Space(before) / Table / Space(after)`
→ 기존 `FlowCursor::fit`/TableCursor → `DocumentV2Session::next_page_json`의 실제 nodes로
전달한다. 앵커 top은 문단 시작 기준 세로 오프셋+top margin이고, before는 top-next_origin이다.
common.margin과 outer_margin은 같은 원천의 IR 복제이므로 일치 여부를 검사하고 한 번만 적용한다.
측정 뒤 별도 원점 선택·좌표 clamp·내용 숨김을 추가하지 않는다.

빈 문자열도 host 줄을 제거하지 않는다. top이 host의 물리 끝 또는 다음 원점보다 앞서면
겹치는 앵커로 명시적으로 거부한다. 이번 경로는 저장된 Para/Top·Left·TopAndBottom·
flowWithText만 수용하며 TAC/어울림/다른 기준점·음수 offset·overlap는 별도 계약을 남긴다.
fresh zero-offset 본문과 중첩 셀의 기존 anchor admission은 그대로 유지한다.

시작 컷은 host 다음 물리 before 밴드의1회 소비다. 표의 요구 높이·누적 예약·실패 시 이월은
기존 child cursor의 실제 수용 조각을 사용한다. 다음 조각은 source offset을 다시 붙이지 않는다.
after 밴드는 child 완료 뒤에만 소비하고 후속 문단을 배치한다. 다중 rowspan·caption·각주
규칙은 이번에 변경하지 않았다. 바깥여백의 모든 한컴 페이지 경계 의미를 입증한 것은 아니며,
미지원 원본 전체/복합 앵커/셀 내부 stored float는 미검증으로 남긴다.

#### 입력·독립 기대값·실행 증거

합성 HWP/HWPX는12px 저장 host와18px 전진, offset18px+top6px, x offset5px+left3px,
bottom8px를 명시한다. 본문(20,30,300,72)에서 표 시작은(28,54)다.12px 줄을18px pitch로
배치하면48px 예산에 A/B/C가 들어가고, 다음 조각의 첫6px는 C 뒤 물리 간격이다. 다음 쪽의
D는 y36, 표 뒤 문단은 y62다. 실제 표/셀 높이48→24, 전체 글자 소유, 빈 host 높이,
종료와 HWP/HWPX 양 경로를 검사한다. 통째 이월 대조군은 host를 이전 쪽에 보존하고
표를 다음 쪽 y30에1회 배치한다. 겹치는 빈 host·어울림·다른 기준점 등의 반례는 거부한다.
초기 계약에서 전체18px pitch가 항상 줄의 fit 단위라고 가정한 것은 잘못이었다.
기존 공통 규칙대로12px 물리 줄과 뒤6px 밴드를 분리한 기대값으로 정정했으며,
엔진/기준값을 변경해 맞추지 않았다(`focused-retry.log`).

`stored_anchor_original_host_isolation_preserves_source_offset_and_blank`는 원본 앞5문단과
host 전체·common anchor의 HWP 재직렬화 동일성을 확인한다. **내부87문단/셀 높이는
단일 probe로 교체한 분리 입력**이며 원본 표 내용·높이·페이지 분할의 정답지가 아니다.
원본 PageDef와 host vpos로부터 x5952/75, y25378/75px를 기대하며 빈 host y24375/75,
높이8px와 실제 후속 원점까지 검사한다. PDF 원본 p1을 `pdftocairo -svg -f 1 -l 1`로
내보낸 `reference-p1.svg`의 표 상단선은 y(841-587.476562)×4/3=338.031251px,
왼쪽 세로선 x59.488281×4/3=79.317708px다. 분리 입력의 기하 원점(79.36,338.373333)과
근접하나 동일하다고 주장하지 않으며, 이 차이를 없애는 보정은 넣지 않았다.

수정 전 `before-contract.log`는 저장 앵커 미지원으로 FAIL이다. 직전 WASM
`ea2c53487a2bdaac013bdbb5e0a92d6020251169be6e3805dec47ee3b4b03703`에서도
최종 입력3개가 같은 이유로 거부됐다(`before-wasm.log/json`). 초기 diagnostic의 컴파일 오류와
stale generated suite의0-test 실행(`focused.log`)은 결함 재현/통과 증거가 아니다.
실제 generated suite를 지정해 재실행한 뒤 최종 prepare로 manifest를 동기화했다.

#### 검증 마무리

`selected.log`:241/241 PASS. 실행 명령은 로그 첫 줄과
`run-extent/run-selected.mjs`에 고정했다. 이후 음수 gap의8px 다음 원점과12px 점유 끝이
갈리는 반례(top10px)를 추가했다. 엔진 변경 없이 동일 앵커 경계4개를 다시 실행하여
`boundary-final.log`4/4 PASS다. 앞선241건 전체를 이4건에 합산하지 않는다.
`clippy-native.log`, `clippy-wasm.log`, `clippy-tests-final.log` 모두 `--locked -D warnings`
PASS, 최종 fmt check/diff check도 PASS다. format 때문에 파생 suite 지문이 달라졌던
`policy.log` 실패는 최종 prepare 후 `policy-boundary.log`에서 고정 base
`7a95e46e025470a4d7a7b59ad68ec02958bda738` 대비 PASS했다. 파생 suite는 stage하지 않는다.
source unit test 변경 및 전체 workspace 제출 게이트 실행은 없다.

Docker `docker compose --env-file .env.docker -p rhwp run --rm wasm`7분20초 성공이다.
빌드 시작 뒤 Rust 변경은 mod 선언 순서의 rustfmt 정렬뿐이며 동작 변경은 없다.
컨테이너의 새 anchor/input/mod 파일 hash는 최종 작업본과 동일함을 확인했다.
최종 WASM SHA256은
`08a5295715f5cdc6a0f1c7729dbf940e63eababb26896575f15ad2f8cd058540`이다.
기존 browser 명령에 `--stored-body-anchor`를 추가하고 fixtures/out을
`stored-anchor/fixtures`, `stored-anchor/browser`로 지정했다. `browser.log`는225쪽의
exact Native/WASM tree·SVG 일치, isolation/rejection/rollback/termination PASS다.
`preservation.json`은 기존 Native120개와 review PNG217개가 byte 동일함을 확인한다.
`source-comparison.json`은 제품/review Rust·Cargo 파일과 주요 테스트·패키지 hash를 연결한다.

Native 첫 출력 검토 후 fresh WASM의 `browser/document-anchor-{split-0,split-1,defer-1,
isolation-0}.review.png`와 `document-anchor-split-1.overlay.png`를 직접 열었다.
split의 첫 쪽 A/B/C, 이어받기 쪽 D, 뒤 문단, 통째 이월의 외곽을 확인했다. 대응
`*.compare.png`, `*.overlay.png`, `*.review.png`는 같은 browser 디렉터리에 있다.
한컴 PDF 전체와의 `visual_accuracy_proxy_percent`는 미계측이다. 원본 isolation의 probe는
원본 내용과 다르며, 동일한 두 backend의 통과를 원본 표 피델리티 통과로 승격하지 않는다.
실험 V2 세션 검증이고 Studio 기본 경로/Legacy·baseline/golden/ignore·원격 상태는 유지한다.

다음 대상은 index5 내부 background zones와 저장 들여쓰기의 독립 근거/지원 경계다.
원본 전체 수용 및 A·R5 완료는 아직 아니다.

### 저장 앵커 시각 판정 자료 철회 및 재준비

작업지시자의 지적에 따라 `stored-anchor/browser/document-anchor-split-1.review.png`를
시각 승인 요청 자료에서 철회한다. 해당 그림은 합성 A/B/C/D 입력의 Native/WASM 일치 자료이며,
원본 표의 조판 정확성을 판정할 자료가 아니다. `ff1eee900`은 합성 계약을 통과한 실험 구현으로만
분류한다. 원본의 전체 표 내용·분할·후속 문단 피델리티와 앵커 경계의 한컴 일치는 미검증이다.
앞선 실행 기록은 보존하되 시각 완료·승인의 근거로 재사용하지 않는다.

2026-09-25 재준비: 원본 index5 host와 전체 표 subtree를 보존한 발췌 입력도 실제 V2 경로에서
`V2 table background zones`로 거부됨을 재확인했다. 원본을 수용한 것처럼 Legacy 출력이나
내용 교체본을 대신 제시하지 않는다. 별도의 읽을 수 있는 대조 문서를 작성하고 한컴에서 재저장한
HWP와 그 파일의 기준 PDF로 검증한다. 이것은 #6923 원본 통과가 아닌 앵커 규칙의 독립 대조군이다.
판정 대상은 첫 표 원점, 앞뒤 문단, 분할 경계 양쪽의 문단 번호·표 외곽·누락/중복이다.

#### 교체 자료와 직접 판독 결과

판정 요청 묶음:
[`visual-replacement/REVIEW.md`](../../output/7353/r19/visual-replacement/REVIEW.md).
입력·한컴 저장본·PDF는
[`tests/fixtures/issue7353_stored_anchor_review/`](../../tests/fixtures/issue7353_stored_anchor_review/README.md)에
보존했다. 생성 절차·MCP job·세 파일의 SHA-256·정확한 입력 속성은 해당 README가 정본이다.
새 문서는 24행에 `자료 01`~`자료 24`를 넣어 각 조각의 소유 내용을 식별할 수 있게 했다.
한컴 재저장 HWP를 다시 수정하지 않고 기준 PDF와 Native/fresh WASM 양쪽에 사용했다.

| 대상 | 독립 기준 PDF / 실제 V2 관측 | 판정 |
| --- | --- | --- |
| 1쪽 | 제목 다음 표 시작, 01~19행, 표 외곽이 본문 안에 배치 | 직접 비교 확인, 메인테이너 판정 대기 |
| 2쪽 내용 | 20~24행 이어받기, 누락·중복 없이 표 종료 후 본문 | 직접 비교 확인, 메인테이너 판정 대기 |
| 2쪽 원점 | 기준 y=79.317708px, V2 y=75.586667px(96dpi) | 위치 일치 미충족: 약 0.987mm 위 |
| 글꼴 | 폭·굵기 차이 | 잔여 차이, 일치 통과 아님 |
| 중첩/셀 내부 분할, #6923 전체, 편집 후 재조판 | 이 대조 문서는 RowBreak의 행 사이 분할만 실행 | 미검증 |

PDF 원점은 `pdftocairo -svg -f 2 -l 2`의 실제 수직 테두리 좌표
`(595-535.511719)*4/3`에서 얻었다. 차이가 outer top margin(283HU)과 가깝지만
일반적인 이어받기 규칙의 원인으로 확정하지 않았다. 위치를 보정해 이미지를 맞추거나
이 좌표를 정상 baseline으로 승인하지 않았다. 이 문제를 남겨 놓고 전체 구현 승인도 요청하지 않는다.

Native 및 fresh WASM의 `*-review-{1,2}.png`와 WASM 단독 `*-overlay-{1,2}.png`를
직접 열어 판독했다. 한컴·V2·겹침을 같은 페이지/크기로 병렬 표시하고, 잔여 차이를 그림에 적었다.
전체 픽셀/ink 점수는 `rows/*-metrics-*.json`에 보존하되 자동 시각 통과 기준으로 쓰지 않는다.

#### 정상 저장 입력을 위한 최소 지원 변경과 실행 증거

한컴 저장본의 main/odd/even PageBorderFill은 ID0이 아닌 **무효과 BorderFill ID1**을
참조한다. 기존 V2는 참조 존재만으로 `section decoration, grid or writing direction`을
반환했다. `document_input::prepare`가 실제 참조된 스타일의 선·채움·효과를 검사하도록 했고,
기존 문단 검사의 무효과 판별을 `decoration::source_border_is_unpainted`로 공유했다.
원본 참조를 지우지 않았으며, 유효하지 않은 참조/보이는 효과는 계속 거부한다.
이 변경은 입력 수용 경계뿐이다. 이후 body 측정과 FlowBlock/Table 실제 배치의 원점·높이는
변경하지 않았다. PageBorderFill spacing은 테두리 배치 속성이므로 본문 예약에 넣지 않는다.

보존된 `3976eb6b8` WASM으로 **같은 최종 입력**을 실행하여 위 거부를 재현했다
(`visual-replacement/before-support.json`). 직전 `ff1eee900` 빌드로 실행한 결과는 아니며,
두 head 사이 해당 section 거부 분기는 변경되지 않았음을 코드 대조했다.
수정 후 Native 및 Docker fresh WASM은 실제 2쪽을 생성한다.

검증 source: `ff1eee90048439215a9849bf5ab90ba0ecd8089e` + 미커밋 두 Rust 파일 변경.
두 파일의 SHA-256 및 입력/PDF/WASM 해시는 `visual-replacement/rows/run.json`에 고정했다.
Native는 review worktree에서 `cargo build --locked --lib`로 빌드했다.
WASM은 `docker compose --env-file .env.docker -p rhwp run --rm wasm`으로 새로 빌드했고
7분 11초에 완료했다. `node output/7353/r19/visual-replacement/review.mjs --wasm`으로
실제 브라우저 DocumentV2를 실행했다. 두 backend SVG는 2쪽 모두 동일하다. JSON의 숫자
130곳에 최대 `5.684341886080802e-14` 차이만 있고 다른 값 차이는 없다. 초기 JSON 문자열
완전 동일 검사는 이 마지막 자리 차이로 중단되었으며, 차이 전수 분석 후 원문을 보존하고
SVG 동일성·구조·수치 차이를 분리 기록했다. 한컴 일치의 근거로 바꾸어 보고하지 않는다.

추가 `tests/cases/issue_7353_table_v2_document_flow.rs` 계약은 무효과 page-border의
main/odd/even × HWP/HWPX 최종 출력 보존과, 실제 한컴 저장본의 행 소유·후속 문단·본문 경계를
검사한다. 2쪽 원점의 일치를 주장하지 않는다. focused 결과는 후속 실행 기록에 남긴다.
이번 단계는 **대체 시각 자료 판정 대기**이며 R5 완료/원본 전체 통과/PR 제출 준비 완료가 아니다.
전체 release 회귀·Native Skia·세 Clippy 제출 gate는 이번 시각 자료 준비에서 실행하지 않았다.
Legacy/Studio 기본 경로, baseline/golden/ignore, 원격 상태는 변경하지 않았다.

focused 최종 실행은 review worktree에서 다음 명령으로 **36 passed, 0 failed**
(`visual-replacement/focused-final.log`). 최초 추가 테스트는 한컴 저장본의 글꼴별 TextRun
세 조각을 완전한 한 줄과 비교하여 실패했다(`focused.log`). TextLine별 실제 TextRun을 이어
줄 내용을 검사하도록 수정했으며 독립 PDF의 행 번호·기대 문자열은 바꾸지 않았다.
이 실패는 조판 결함이나 수정 전 재현으로 집계하지 않는다. `rustfmt --check`(변경 3파일)와
`git diff --check`도 통과했다. 위 시각 증적 후 renderer source의 추가 변경은 없다.

```sh
CARGO_BUILD_JOBS=1 cargo nextest run --locked --cargo-profile release-test \
  --test regression_suite_005 -E 'test(issue_7353_table_v2_document_flow)' \
  --target-dir /home/edward/mygithub/rhwp/target/pr-review
```

### 이어받기 위 바깥여백 — 독립 대조와 구현

2026-09-25 다음 절편 승인으로 위 2쪽 원점 차이를 우선 처리한다. 직전 대체 자료와
무효과 PageBorderFill 지원을 `50eea221d`로 로컬 커밋했다. 기존 2쪽 위치 차이의 시각
통과나 #6923 원본 전체 통과로 승인 범위를 확대하지 않는다.

`bug-hunter` 스킬/플레이북을 확인했으나 기존 CLI의 실사용 여정 전용이고 이 경로는
실험 DocumentV2 API이므로 전체 여정은 적용하지 않았다. 승인된 V2 검증 경로에서
독립 PDF 대조를 수행한다. 새 이슈/원격 변경/Legacy 전환은 없다.

독립 대조의 입력 생성·MCP job·관측값은
`tests/fixtures/issue7353_stored_anchor_review/README.md`의 continuation margin controls에
연결했다. 위 바깥여백0/283/567HU에 따라 PDF의 2쪽 표 시작이 이동하고, 문단 기준 세로
offset2835→5670HU에는 이동하지 않았다. CellBreak와 원자적 표 전체 이월도 top283HU를
반영한다. 따라서 첫 앵커의 세로 offset은 한 번만 소비하고, 새 조각의 위 바깥여백은
반복 예약한다. 합성 입력을 수동으로 고쳐 저장 캐시 수용 조건을 완화하지 않았다.

실제 소비 경로:

- `BodyAnchor::resolve`: IR common/mirror 여백 일치 검증 → before/after/restart_top.
- `document_input::prepare`: 처음 `Space(before)`는 기존대로, Table에 restart_top 전달.
- `FlowCursor::fit`: child cursor의 시작/끝 컷은 TableCursor 소유. 이미 수용한 child 또는
  예산 부족으로 이월한 child에만 prefix 적용 → 가용 높이에서 차감 → child fit의 요구 높이에
  prefix 합산. 배치 성공 시에만 prefix+reserved_height를 누적한다.
- `DocumentV2Session`: 동일 FlowFit을 본문 예약/진행과 실제 TablePlacement paint에 사용.
  paint 이후 별도 원점 보정은 없다. 마지막 child 완료 후에만 after band/후속 문단을 소비한다.
- 새 속성은 저장 본문 앵커에만 부여한다. fresh/nested adapter는0이며 TAC inline row,
  side-wrap, Legacy 경로는 비해당. nested synthetic 계약은 공유 fit의 예산 반례이지
  한컴 중첩 여백 지원 주장으로 쓰지 않는다.

실험 Rust `FlowBlock::Table` 생성자에 `restart_top` 필드가 추가되어 내부 어댑터와
테스트 생성자를 함께 갱신했다. 기존 WASM DocumentV2 옵션/입출력 스키마는 변경하지 않았다.

수정 전 라이브러리에 같은 정식 document-flow 테스트 소스를 링크하여 신규 좌표 assertion
2건 FAIL을 확인했다(`anchor-continuation/before-tests.log`): base75.586667≠79.36,
top2mm75.586667≠83.146667. 최초 진단 링크는 serde dependency 조합 오류였고 결함 재현으로
세지 않았다. 일치하는 serde_json 라이브러리로 링크한 후 실제 조판 좌표에서 실패했다.
수정 후 focused/Native/fresh WASM/시각 결과는 아래 후속 기록으로 연결한다.

#### 첫 후보 출력 확인 — 후속 재검증으로 대체

검증 source는 `50eea221d` + 이번 7개 V2 Rust 파일 변경이며 정확한 파일 해시는
`output/7353/r19/anchor-continuation/superseded-review/source.sha256`에 보존했다. review worktree와 해당 source/
test 파일이 동일함을 `cmp`로 확인했다. Native는 review의 release-test 라이브러리를 사용했다.
Docker fresh WASM 빌드가 7분23초에 성공했고 SHA-256은
`fd2e02eb75da086a83711423ca9a75260583721f8fe48ba854ae7a9f81f61975`다.

실행 명령/증거는 `output/7353/r19/anchor-continuation/` 아래에 있다.

- `docker compose --env-file .env.docker -p rhwp run --rm wasm` → `docker-wasm-first.log`.
- `probe <동일 saved HWP> review/actual render` → Native 최종 RenderTree/SVG JSON.
  진단 probe는 기존 `visual-replacement/probe.rs`를 이번 release-test 라이브러리로 링크했다.
- `node output/7353/r19/anchor-continuation/review.mjs --wasm` → 브라우저 DocumentV2,
  `review/actual/wasm.json`, 두 backend의 compare/standalone overlay/review 각1·2쪽.
- `node output/7353/r19/anchor-continuation/check-output.mjs` → `coordinates.json`.
  수정 전후 양 backend의 실제 표 원점/뒤 문단 위치/24행 단일 출현, 첫 쪽 SVG 불변을 검사.

| 검사 | 결과 |
| --- | --- |
| 2쪽 표 원점 | 수정 전75.586667 → 수정 후79.36px; 독립 PDF79.317708px |
| 2쪽 뒤 문단 원점 | 수정 전238.213333 → 수정 후241.986667px; 표 끝+567HU |
| 1쪽 보존 | 수정 전후 SVG 완전 동일 |
| Native/fresh WASM | 두 쪽 SVG 동일, JSON 숫자125곳 최대5.684e-14 차이, 다른 차이0 |
| 직접 Visual Sweep | Native/WASM review1·2 및 WASM standalone overlay1·2를 열어 표 외곽·행 연결·뒤 문단 확인 |
| 남은 차이 | 글꼴 폭/굵기. 메인테이너 시각 판정 대기 |

대표 자료는 `review/wasm-review-1.png`, `review/wasm-review-2.png`,
`review/wasm-overlay-2.png`다. 이미지 좌표 변환 없이 같은 PDF/페이지를 비교했으며,
이번 여백으로 표를 단순 이동한 것이 아니라 fit 예산/본문 점유도 같이 반영한다.

별도 `defer` 대조에서는 한컴이 후속 문단을1쪽에 남기고 표만2쪽으로 보내는 것을
PDF 직접 판독/텍스트로 확인했다. V2는 후속 문단을 표 뒤에 배치하므로 이 대조의 **전체
story 순서 일치는 미충족**이다. 이 사례를 통과 자료로 승격하지 않았고, 회귀 assertion은
표 이월 원점과 행 보존만 주장한다. 원자적 이월의 주변 문단 흐름은 후속 검토 대상이다.
원본 #6923 전체/중첩 여백/편집 후 재조판/R5 완료를 뜻하지 않는다.
전체 release 회귀, Native Skia, PR 제출용 세 Clippy gate는 이번 절편에서 실행하지 않았다.

첫 focused 실행은 `focused.log`에서 **57 passed / 1 failed / 2 not run**이었다.
기존 `fractional_page_budget_does_not_split_an_atomic_nested_table`이 실제 회귀를
검출했다. `1.2 - 1.0`으로 계산한 가용 높이와0.2를 다시 비교하면서 child fit의
수용 결과를 부모에서 뒤집었고 InconsistentAtomicPlan이 발생했다. 테스트 기대값은
유지하고 중복 reserved-height 재판정을 제거했다. prefix 자체가 본문을 넘는지만
기존 fit과 같은 절대 좌표계에서 확인하고, 자식 조각의 수용 높이는 child fit 결과를
그대로 소비한다. 이 변경 뒤 위 후보의 이미지는 `superseded-review/`에 보존하고
최종 코드의 Native/WASM 빌드·focused·시각 캡처를 다시 실행한다. 기존 캡처를 재사용하지 않는다.

최종 focused 실행은 **60 passed, 0 failed, 163 filtered out**
(`anchor-continuation/focused-final.log`)이다. 대상은 document_flow/nested/alignment이며
전체 저장소 회귀 통과를 뜻하지 않는다. 위 소수 경계 테스트와 신규 여백 예산 계약,
6개 정상 한컴 저장본(기본+5대조)의 실제 좌표/내용 검사가 통과했다.

```sh
# rhwp-review-7353; 준비된 regression_suite_005 사용
CARGO_BUILD_JOBS=2 cargo nextest run --locked --cargo-profile release-test \
  --test regression_suite_005 -E 'test(issue_7353_table_v2)' --no-fail-fast \
  --target-dir /home/edward/mygithub/rhwp/target/pr-review
```

nextest0.9.137은 저장소 권고0.9.140보다 낮다는 경고 및 관측용 report-skipped 키 경고가
있었으나 test 실행은 성공했다. Native probe도 최종 라이브러리로 다시 링크/실행했다.

최종 Docker WASM은 **7분32초 성공**, SHA-256
`3ad244d3d9bf7e2748fb0b8035a15507bdaf79f261671882631327eef2b60db1`이다.
`review.mjs --wasm`과 `check-output.mjs`를 최종 빌드로 다시 실행하고, Native/WASM의
review1·2 및 WASM standalone overlay1·2를 **새 캡처로 다시 열어 판독**했다.
최종 증적은 `review/`, 최초 후보는 `superseded-review/`로 구분한다.
표 원점79.36px, 후속 문단241.986667px, 첫 쪽 불변, 두 backend SVG 동일이라는 위 관측은
최종 코드에서도 재확인했다. 글꼴 폭·굵기와 `defer`의 후속 문단 흐름 차이는 그대로 남는다.
최종 source 해시는 `anchor-continuation/source.sha256`, 입력/WASM 식별자는
`review/run.json`, backend 차이는 `review/backend-comparison.json`이다.
변경 파일 `rustfmt --check`와 `git diff --check`도 통과했다.

이번 절편의 상태는 **이어받기 여백 구현/대상 회귀 완료, 기본 대조 문서1·2쪽 시각 판정 통과**다.
전체 R5 완료나 원본 #6923 통과를 뜻하지 않는다. 제출 gate와 남은 실제 문서 수용 작업은 유지한다.

#### 작업지시자 시각 판정 — 2026-09-26

작업지시자가 “시각 판정 통과입니다.”로 최종 대조 문서1·2쪽을 승인했다.
대상은 `anchor-continuation/review/wasm-review-1.png`, `wasm-review-2.png`로 제시한
표 시작/이어받기, 01~24행 연결, 표 외곽 및 후속 문단 위치다. 대응 WASM은 위 최종
SHA-256 `3ad244d3…2b60db1`이며, 현재7개 renderer source가 증적의 `source.sha256`과
동일함을 확인했다. 코드 변경이 없어 이미 통과한60건/빌드/시각 캡처를 반복 실행하지 않았다.
글꼴의 완전 일치, 별도 `defer` 사례의 후속 문단 흐름, 원본 #6923 전체를 승인한 것으로
확대하지 않는다. 이번 갱신은 판정 기록뿐이며 소스·기준값·ignore·원격 상태는 바꾸지 않았다.

### 다음 절편 — 이월된 floating 표와 후속 본문 흐름 분리

작업지시자의 다음 절편 승인으로 착수했다. 앞선 여백 수정/시각 승인 기록은
로컬 `3f2214e3b`에 보존했다. remote push·Legacy 변경·기본 엔진 전환은 하지 않는다.

독립 근거는 앞 절편에서 확보한 정상 한컴 `variants/defer-saved.hwp`와 그 파일의 PDF다.
원문·저장 LineSeg·PDF를 수정하거나 다시 생성하지 않았다. PDF1쪽에는 제목과 후속 문단,
2쪽에는01~03행 표만 있다. 기존 V2는 `Space(anchor.before) → Table → Space(after)`를
본문에 직렬 삽입하여 표의 예약 실패까지 본문 공백/진행으로 취급했다. 따라서 표가
2쪽으로 옮겨질 때 후속 문단도 같이 끌려갔다. 이는 총 페이지수가 아니라 페이지별
소유 내용과 실제 원점의 결함이다.

이번 규칙: 수용된 자리차지 표 조각은 실제 영역을 예약한다. 한 조각도 수용하지 못한
positioned 표의 초기 offset은 본문이 소비한 빈 문단이 아니다. 표를 다음 페이지의
대기 커서로 보내고 현재 쪽의 후속 문단은 가용 공간에서 계속 조판한다. 실제 빈 문단과
문단 간격은 본문 커서에 그대로 남는다. 표의 마지막 조각 뒤 아래 여백도 보존한다.
TAC/fresh zero-offset/중첩 셀의 기존 흐름은 이 문서용 scheduling과 구분한다.

구현 연결:

- `body_anchor::resolve`: 기존 속성→IR 검증과 x/초기 before/재시작 top/after 결과 유지.
- `document_input::prepare`: 해당 표를 본문 spacer로 삽입하지 않고 host 직후 block 경계와
  initial/deferred flow를 만든다. 표 내용 plan/paint/소유 ID는 하나를 공유한다.
- `body_flow::BodyCursor::fit`: 본문은 anchor 경계까지 공통 `FlowCursor::fit_until`로
  소비하고, 표도 같은 fit으로 조회한다. 최초 조각이 없으면 그 조회의 초기 위치 밴드만
  폐기하며 child 유닛은 소비하지 않는다. 다음 페이지에는 pending 예약을 먼저 처리한다.
  수용된 조각은 원래 child continuation과 남은 물리 밴드를 유지한다.
- `FlowFit::advance`: 누적 예약 높이와 다음 원점을 분리한다. 음수 줄간격의 실제 점유
  높이를 다음 origin으로 대신하지 않는다. `BodyFit::accept`가 그 두 값을 누적하고
  정확한 LinePlacement/TablePlacement를 모은다.
- `DocumentV2Session::next_page_json`: 동일 BodyFit의 예약 높이를 확인한 뒤 정확한
  배치 결과를 paint한다. 별도 y 재계산/clamp 없이 SVG/JSON 성공 후 두 커서를 함께
  commit한다. 본문 종료만으로 완료하지 않으며 pending 표/남은 여백도 종료 조건에 포함한다.
- 이월된 표가 새 페이지에도 안 맞으면 위 여백만으로 진전을 인정하지 않는다.
  `DoesNotFit`으로 남기고 반복 호출 때 같은 상태를 보존한다.

독립 PDF를 쓰는 `hancom_deferred_anchor_preserves_prose_on_source_page`는 수정 전
라이브러리에서 **FAIL**했다(`anchor-story/before-tests.log`):1쪽 실제는 제목만,
기대는 제목+후속 문단이다. 최초 rustc 호출의 `CARGO_MANIFEST_DIR` 누락은 환경 실패이며
결함 재현으로 세지 않았다. 해당 환경 변수를 지정한 재실행의 assertion 실패만 증거다.
기존 합성 `stored_anchor_atomic_defer...`의3쪽/후속 문단3쪽 기대는 독립 출력에 맞지
않았으므로2쪽/후속 문단1쪽으로 바꿨다. baseline·golden·ignore를 완화한 것이 아니다.

빠른 debug 단계에서 document-flow43건 통과했다. 실제 빈 줄, 다음 쪽 pending 표의
공간 점유로 후속 문단이 또 이월되는 경우, 본문이 끝나도 남는 표, 페이지 밖 offset,
여러 pending 소유자 보존, 과대 atomic 표의 무한 빈 페이지 방지와 정상 fit 대조를
포함한다. 이것은 수정 중 진단이며 최종 release-test/fresh WASM 증거를 아래에 덧붙인다.
6개 기존 한컴 대조의 범위를 자동으로 확장하지 않으며 중첩 floating/TAC/side-wrap,
다양한 후속 개체와의 상호배치 전체는 이 절편의 한컴 검증 범위가 아니다.

#### 최종 집중 회귀

최종 source는 `3f2214e3b` + 이번6개 renderer 파일(신규 `body_flow.rs` 포함) 및
document-flow 테스트 변경이며, 정확한 해시는 `output/7353/r19/anchor-story/source.sha256`다.
제품/review worktree의 `src/renderer/table_v2/` 전체가 동일함을 `diff -qr`로 확인했다.
review의 이미 준비된 `regression_suite_005`가 같은 `tests/cases/` 소스를 사용한다.
파생 suite/manifest를 source PR에 추가하지 않는다.

```sh
# rhwp-review-7353
CARGO_BUILD_JOBS=2 cargo nextest run --locked --cargo-profile release-test \
  --test regression_suite_005 -E 'test(issue_7353_table_v2)' --no-fail-fast \
  --target-dir /home/edward/mygithub/rhwp/target/pr-review
```

`anchor-story/focused-final.log`: **66 passed / 0 failed / 163 skipped**,
release-test 빌드4분56초. 대상은 document_flow/nested/alignment이며 저장소 전체 CI를
의미하지 않는다. nextest 버전/관측 설정 경고는 이전 절편과 같고 실행은 성공했다.
최종 Native probe를 이 release-test 라이브러리에 링크하여 동일 저장 HWP2종을 다시
렌더했다. 기본24행 대조군의 두 쪽 SVG는 앞선 시각 승인 코드 출력과 byte-identical이다.
이월 사례는 후속 문단이1쪽으로 이동하고2쪽의 표 원점/높이는 보존된다.
변경 Rust 파일 `rustfmt --check`, `git diff --check`, 최종 source hash 검사가 통과했다.
전체 release 회귀·PR 제출용 세 Clippy gate·원본 #6923 전체 fidelity는 미검증으로 유지한다.

#### Fresh WASM / Visual Sweep / 판정 요청

제품 worktree에서 `docker compose --env-file .env.docker -p rhwp run --rm wasm`이
**7분24초 성공**했다(`anchor-story/docker-wasm.log`). 최종 WASM SHA-256:
`927a70729ca5c013a44caaeacbff75da81ab3348b5fcf87027e9c6f6f59b2417`.
코드 변경 없이 이 빌드의 `pkg/rhwp.js`/`rhwp_bg.wasm`으로 브라우저 DocumentV2를 실행했다.

```sh
# rhwp-task-7353; probe는 위 최종 release-test 라이브러리에 링크
output/7353/r19/anchor-story/probe \
  tests/fixtures/issue7353_stored_anchor_review/variants/defer-saved.hwp \
  output/7353/r19/anchor-story/review/defer/actual render
output/7353/r19/anchor-story/probe \
  tests/fixtures/issue7353_stored_anchor_review/anchor-review-saved.hwp \
  output/7353/r19/anchor-story/review/base/actual render
# 각 대응 PDF를 pdftoppm -png -r 96으로 review/{defer,base}/hancom에 출력
node output/7353/r19/anchor-story/review.mjs --wasm
node output/7353/r19/anchor-story/review.mjs --base --wasm
node output/7353/r19/anchor-story/verify.mjs
```

`anchor-story/coordinates.json`, `review/{defer,base}/run.json` 및
`backend-comparison.json`에 원문/PDF/WASM/source 해시와 최종 결과를 남겼다.

| 검사 | 수정 전 → 최종 Native/fresh WASM |
| --- | --- |
| 후속 본문 소유 페이지/원점 |2쪽179.96px →1쪽99.053333px; 저장1100HU+660HU 글줄 간격 |
| 이월 표 |2쪽 x52.92/y79.36/폭426.666667/높이93.04px 유지 |
| 내용·종료 |1쪽 제목+후속 문단,2쪽01~03행 각1회, 불필요한3쪽 없음 |
| 편집영역 | 표와 모든 글줄이 실제 Body 안에 있음 |
| 이월 사례 backend | SVG2쪽 모두 동일; JSON 숫자24곳 최대2.842e-14, 다른 차이0 |
| 기존24행 대조 | 수정 전후 Native/WASM SVG2쪽 모두 동일; backend 숫자125곳 최대5.684e-14, 다른 차이0 |

최종 코드의 Native/fresh WASM review1·2쪽을 두 대조 모두 직접 열어 확인했고,
이월 사례 WASM standalone overlay1·2도 직접 판독했다. 원점/스케일 정렬 보정 없이
동일 PDF·동일 페이지를 비교했다. 주장은1쪽의 후속 문단 보존과2쪽 표의 내용·원점·외곽,
정상 분할 대조군의 보존이며 글꼴 폭/굵기 차이는 남는다. 자동 점수로 시각 통과를
선언하지 않는다.

판정 요청 대표 파일:

- [이월 사례1쪽 review](../../output/7353/r19/anchor-story/review/defer/wasm-review-1.png):
  제목 아래 `표 종료 후 본문입니다.`가 있고 이쪽에는 표가 없어야 한다.
- [이월 사례2쪽 review](../../output/7353/r19/anchor-story/review/defer/wasm-review-2.png):
  표01~03행과 외곽이 있어야 하며 후속 본문이 중복되지 않아야 한다.
- [2쪽 standalone overlay](../../output/7353/r19/anchor-story/review/defer/wasm-overlay-2.png).
- 입력은 [그대로의 한컴 저장 HWP](../../tests/fixtures/issue7353_stored_anchor_review/variants/defer-saved.hwp),
  기준은 [해당 HWP의 한컴 PDF](../../tests/fixtures/issue7353_stored_anchor_review/variants/defer-2020.pdf).

상태: **구현·대상 회귀·fresh WASM·직접 비교 완료, 이번 이월 사례의 메인테이너 시각 판정 대기**.
전체 R5 완료·일반 어울림/중첩 floating 수용·원본 #6923 통과를 뜻하지 않는다.

#### 다음 절편 승인과 폰트 판정 범위

작업지시자는 기존 폰트 메트릭/fallback 구현을 활용하고, 원래 폰트가 없으면 유효한
저장 LineSeg의 줄 소속·줄바꿈을 보존하는 것을 이번 조판 검증의 기준으로 명확히 했다.
대체 글꼴의 폭·굵기 외형 차이만으로 이번 절편의 잔여 조판 실패라고 분류하지 않는다.
실제 사용 폰트 대조 없이 외형 차이의 원인을 확정하지도 않는다.
이후 다음 절편 진행 승인을 받았다. 이는 별도의 명시적 시각 통과 발언과 구분한다.
검증한 소스 변경 없이 위 결과를 재사용하며, 다음 대상은 원본 #6923 문단 index5의
표 영역별 배경/테두리 수용 경계다. 원본 속성을 제거하는 우회는 하지 않는다.

### 다음 절편 — 영역 배경과 테두리의 최종 셀 좌표 적용

이전 본문/이월 절편은 검증된 코드 그대로 `b7678520e`로 로컬 체크포인트했다.
이번 절편은 `table_v2`의 영역 장식 구현이며 Legacy/default engine을 바꾸지 않는다.

#### 규칙·근거·입력

원본 #6923의 `s0p5/cell0/p26` 8×9표에는 `(1,3)..(1,3)` 영역(borderFill29)이,
`p37` 7×10표에는 borderFill2를 사용하는 두 영역이 있다. 영역을 일괄 거부하던 경계가
해제되어도 다른 source 효과와 저장 문단 수용 검사를 통과해야 한다. 현재 다음 거부는
`s0p5/cell0/p26`의 셀 borderFill31(attr8)에서 발생하는 `V2 source decoration effect`다.
원본을 수정하거나 장식 속성을 지워 통과시키지 않는다.

독립 기준은 [영역 대조 fixture](../../tests/fixtures/issue7353_zone_review/README.md)의
한컴 저장 HWP와 그 HWP의 PDF다. 생성기 HWPX에는 수동 LineSeg를 넣지 않았다.
셀 주소 `(1,0)..(23,0)`의 끝 셀은2열 병합이며,1쪽은1~19행/2쪽은20~24행이다.
한컴에서 관측한 규칙:

- 영역 면은 셀 자체 면 아래, 영역의 유효 외곽선은 셀 선 위에 적용한다.
- 끝 셀 주소는 병합 셀 전체 범위를 포함한다. 끝 열+1로 축소하지 않는다.
- 페이지의 수용된 표 조각에서 영역 면과 외곽이 이어지며2쪽 조각도 닫힌다.

별도 합성 계약에서는 영역 선 없음이 기존 셀 선을 바꾸지 않는 보존 불변식을 검사한다.
이 None 조합과 복수 영역 우선순위의 한컴 출력 대조는 이번 독립 fixture가 입증하지 않는다.

24행 대조에 앞서 높이7000HU인8행 대조도 만들었다. 한컴은7행의 빈 물리 공간을
쪽 경계에서 나누지만 V2 RowBreak는6행/2행으로 나눈다. 해당 입력·저장본·PDF를
fixture의 `diagnostic/`에 그대로 보존하며 **이 대조는 미통과**다. 통과 문서로 바꾸어
보고하지 않는다. 기존 승인2326HU 줄 프레임을 사용하는24행 대조를 별도로 생성해
장식 검증을 분리했다.8행의 영역만 제거한 진단 입력은 이전/현재 코드 SVG2쪽이 모두
동일했다(`zones/tall-nozone-before`, `tall-nozone-after`). 이는 이번 paint 변경이
행 분할을 변경하지 않았다는 증거이지, 수정된 진단 입력을 원본 fidelity로 판정한 것이 아니다.

#### 공통 결과와 실제 호출 경로

`zones.rs::Zone::prepare`는 원본 셀 주소·병합 범위와 style을 바인딩한다.
`text_ir.rs::bind_paint`가 중첩 표까지 같은 영역 paint를 준비하며,
`decoration.rs::validate_source`는 영역 borderFill도 셀/table과 동일하게 검증한다.

변경 값은 **영역의 최종 물리 사각형**이다. 기존 `TableCursor::fit`의
`TablePlacement.cells[].bounds` → `Zone::bounds` →
`TextPaint::build_node`의 배경 / `CellBorders::append`의 외곽으로 연결된다.
배경과 테두리가 같은 셀 집합의 같은 bounds를 쓰며, 선언 높이 재합산·문단 재조판·
parent 원점 재가산·clamp·cut 변경은 없다. 마지막에 원점을 덮어쓰는 분기도 없다.
`DocumentV2Session::next_page`의 build_node 후 SVG/JSON 성공 시 cursor commit과
TablePreview의 `TextFragment::append_to` 모두 이 경로를 소비한다.

분할 유닛·요구 높이·예약·이월 계산은 기존 fit 결과 그대로이며 이번 수정은 관여하지 않는다.
row/within-cell 조각의 실제 사각형을 배경/외곽에 투영한다. 중첩도 fit이 반환한 page 좌표를
한 번만 쓴다. zone/None은 선을 없애지 않고, 보이는 영역 선만 교체하여 중복 paint를 막는다.
겹치거나 맞닿는 복수 영역, 병합 셀을 가르는 영역, 분리된 물리 영역, gradient/패턴/이미지/
대각선 등은 명시적 Unsupported로 남는다. 관련 효과를 조용히 생략하지 않는다.

#### 경계 검사와 진단

`tests/cases/issue_7353_table_v2_document_flow.rs`에6건을 추가했다.

| 주장 | 실제 검사 및 독립 기대 근거 |
| --- | --- |
| 한컴 영역 적용 | 같은 저장 HWP의2쪽 면 좌표·빨간 외곽4개·노란 셀의 layer·원래 검정 외곽 제거; PDF+저장2326HU |
| 끝 병합 셀 |24행의 오른쪽 끝은32000HU 전체 폭; 각 셀 텍스트1회, 후속 본문 좌표18149HU |
| None과 비영향 경로 | 영역 없는 정상 대조와 Table/Cell/TextLine/TextRun/Line 좌표·속성 동일 |
| 중첩 영역 | 최종 child bounds와 영역 면 일치, padding 원점 중복 적용 없음 |
| 셀 내부 분할 |72px 예산/18px 줄5개 →72+18 조각,5줄 각1회, 후속 문단 보존(합성 계약) |
| 무효·미지원 | 참조 누락, 역전/범위 초과, 효과, 겹친 영역, 병합 셀 중간 컷은 거부 |

이전 release-test 라이브러리에 새 독립 HWP 검사를 링크하면
`Unsupported("V2 table background zones")`로 FAIL한다(`zones/test-before.log`).
debug 구현 후 document-flow **49 passed / 0 failed**(`zones/test-after.log`).
첫 debug 검사에서는 합성 None 선 설정의 누락과 부동소수점 exact 비교 오류2건을 바로잡았다.
구현 결함을 baseline 변경으로 감춘 것이 아니며 이전 FAIL을 결함 검출 근거로 과장하지 않는다.
Native2쪽 review를 직접 확인한 뒤 release 집중 회귀/fresh Docker WASM을 시작했다.
최종 실행 결과와 시각 판정 자료는 아래에 기록한다.

#### 최종 집중 회귀와 fresh WASM

검증 source는 `b7678520e`+이번7개 renderer 파일(신규 `zones.rs` 포함)과
document-flow 테스트다. `output/7353/r19/zones/source.sha256`로 고정했고 제품/review의
V2 소스가 동일함을 확인했다. 새 fixture의 저장 HWP도 review worktree에 복사했다.

```sh
# rhwp-review-7353: 이미 준비된 파생 suite 사용, 생성 파일은 제출하지 않음
CARGO_BUILD_JOBS=2 cargo nextest run --locked --cargo-profile release-test \
  --test regression_suite_003 --test regression_suite_004 --test regression_suite_005 \
  --test regression_suite_015 --test regression_suite_027 --test regression_suite_028 \
  -E 'test(issue_7353_table_v2)' --no-fail-fast \
  --target-dir /home/edward/mygithub/rhwp/target/pr-review
# rhwp-task-7353
docker compose --env-file .env.docker -p rhwp run --rm wasm
```

- `zones/focused-final.log`: **170 passed / 0 failed / 1122 skipped**, 빌드6분54초.
  border/split-border/text/IR/export/document/nested/alignment/geometry 대상이며 전체 CI가 아니다.
  nextest0.9.137 권장버전0.9.140/관측용 설정 경고는 기존과 같다.
- `zones/docker-wasm.log`: **7분44초 성공**. WASM SHA-256
  `efcff6e39c2e6eb26ef60c24889f9e7340352097f7beef79605dfa0256aa3074`.
- 최종 release-test 라이브러리에 Native probe를 링크해 같은 HWP를 렌더하고,
  그 뒤 코드 변경 없이 새 WASM으로 `DocumentV2`를 브라우저에서 실행했다.
  default Legacy engine은 전환하지 않았다.
- changed-file rustfmt check, `git diff --check`, source hash 검사 통과.
  전체 PR lint/회귀·원본 #6923 전체 조판은 미검증이다.

```sh
output/7353/r19/zones/probe tests/fixtures/issue7353_zone_review/zone-lines-saved.hwp \
  output/7353/r19/zones/review/actual render
output/7353/r19/zones/probe tests/fixtures/issue7353_stored_anchor_review/anchor-review-saved.hwp \
  output/7353/r19/zones/control render
# 기준 PDF를 pdftoppm -png -r96으로 동일 페이지에 출력한 뒤
node output/7353/r19/zones/review.mjs --wasm
node output/7353/r19/zones/control.mjs
```

영역 대조의 최종 Native/fresh WASM SVG는2쪽 모두 동일하다. JSON은 숫자171곳에서
최대5.684e-14 차이만 있고 다른 차이는0(`review/backend-comparison.json`).
영역 없는 정상 대조는 변경 전후 Native/WASM 각각2쪽 SVG가 byte-identical이며
backend 간에도 동일하다(`control/comparison.json`). 첫 control 브라우저 실행은
Chrome 프로세스 시작 오류로 실패했고 같은 명령 재실행은 성공했다. 이를 조판 실패로 세지 않았다.

동일 입력·PDF·페이지의 최종 Native/WASM review1·2와 WASM standalone overlay1·2를
직접 열어 확인했다. 위치 맞춤 변환 없이 면/외곽·줄 소속·24행 병합·후속 문단을 비교했다.
폰트 외형 차이 자체는 사용자 지시대로 이번 조판 실패에서 분리한다. Source/PDF/WASM 해시는
`review/run.json`에 있다. 점수나 페이지 수만으로 시각 통과를 선언하지 않는다.

판정 요청:

- [1쪽 WASM review](../../output/7353/r19/zones/review/wasm-review-1.png):2행부터 파란 영역과
  빨간 외곽,3행 왼쪽 셀의 노란 면,1~19행 내용/경계.
- [2쪽 WASM review](../../output/7353/r19/zones/review/wasm-review-2.png):20~24행,
  마지막 병합 셀 전체 폭과 표 뒤 문단. [standalone overlay](../../output/7353/r19/zones/review/wasm-overlay-2.png).
- [판정용 HWP](../../tests/fixtures/issue7353_zone_review/zone-lines-saved.hwp),
  [동일 HWP의 한컴 PDF](../../tests/fixtures/issue7353_zone_review/zone-lines-2020.pdf).

상태: **단색 영역 배경·외곽 구현/집중 회귀/fresh WASM 완료, 메인테이너 시각 판정 대기**.
미해결 큰 고정행 분할 대조, 원본의 대각선 등 source 효과, 겹친 영역 우선순위는 별도로 남는다.
이 결과를 R5 완료나 원본 #6923 전체 통과로 기록하지 않는다.

다음 절편 진행 승인을 받아 위 검증 완료분을 로컬 체크포인트로 보존한다.
진행 승인을 별도의 시각 판정 통과로 해석하지 않는다. 다음 대상은 원본의
borderFill31/29 `attr=8` 대각선 효과이며, 적용 단위와 분할 경계는 독립 출력으로 확인한다.

### 다음 절편 — 온전한 셀과 행 분할 영역의 직선 대각선

앞 절편을 `80967bc75`로 로컬 커밋하고 진행했다. Legacy 기본 경로·기존 승인된
줄 구성·글꼴 fallback/metrics·LineSeg 수용 조건은 바꾸지 않았다.

#### 독립 근거와 범위

원본 #6923의 첫 장식 차단은 `s0p5/cell0/p26`8×9 표의 borderFill31,
`attr=8`, diagonal_type1이다. 같은 표의 단일 셀 영역도 borderFill29/attr8을 참조한다.
사양 표24의 방향 비트를 확인하되 기존 Legacy helper의 모든 비트 해석을 그대로
복사하지 않았다. 새 한컴 정상 저장본에서 실제 방향과 영역 범위를 확인했다.

`tests/fixtures/issue7353_diagonal_review/create.rs` → 저장 LineSeg 없는 HWPX →
한컴 저장 HWP → 같은 HWP의 PDF 순서다. 다운로드된 HWP/PDF는 수정하지 않았다.
두 변환 job과 해시는 fixture README에 고정했다.

`diagonal-2020.pdf`2쪽에서 확인한 규칙:

- attr8은`/`,64는`\`,72는X이며, 방향만 있거나 선 종류만 있는4·5행은 선이 없다.
- 18~21행의 두 열 영역은1쪽18~19행,2쪽20~21행의 실제 영역 각각에서 대각선이 다시 시작한다.
- 마지막24행의 병합 셀은 두 열 전체 폭의 선을 가진다. 뒤 문단은 기존 위치에 남는다.
- 기대 좌표는 한컴 저장2326HU 행높이와 PDF 경계, x3969HU/폭32000HU,
  top8787HU·5952HU, 뒤 문단18149HU에서 정했다. 구현 높이를 기대값으로 재인용하지 않는다.

별도 `diagonal-cell-2020.pdf`는 긴1×1 셀40문단+대각선 반례다. 한컴은 표를2쪽으로
넘기고 용지 아래까지 출력하며, 초기 V2 진단은 내부 분할한다. **미통과 대조**로 그대로
보존한다(`diagonal/cell/actual/native.json`은 guard 추가 전 진단이며 최종 성공 자료가 아님).
셀 내부 분할의 대각선 의미가 확보되지 않아 CellBreak의 활성 셀/영역 대각선은 명시적으로
Unsupported로 남긴다. 정책을 바꾸거나 높이를 줄여 한컴 페이지 수에 맞추지 않았다.

#### 실제 소비 경로

이번 변경 값은 **대각선의 두 끝점**이다. source borderFill의 attr/pen →
style_resolver의 동일 속성 → `diagonal.rs::Diagonal::from_style`의 한정된 선 종류·방향 →
`text_ir.rs::bind_paint`의 셀 소유별 바인딩 / `zones.rs::Zone::prepare`의 영역 바인딩으로 간다.
`TableCursor::fit`가 수용한 `CellPlacement.bounds` 또는 그 셀 집합의 `Zone::bounds` →
`TextPaint::build_node` → `Diagonal::append`의 끝점으로 연결된다.
선은 배경·내용 뒤에 paint하며 parent 원점을 재가산하거나 좌표/높이를 clamp하지 않는다.
Native/WASM은 이 render tree를 소비한다. 상위 Document session과 TablePreview도
같은 build_node를 사용하고, 예외 발생 시 cursor를 commit하지 않는 경계는 그대로다.

시작/끝 컷·유닛 소유·요구 높이·예약·예산 실패 이월은 기존 fit 경로 그대로이며
대각선은 이에 관여하지 않는다. 온전한 셀, 병합 셀, 중첩된 RowBreak 자식과 행 분할 영역을
지원한다. 셀 내부 split·전체 표 배경 대각선·꺾은선·중심선·다중 ray·비실선 pen·겹친
셀/영역 대각선 우선순위는 미검증으로 거부한다. 지원 범위를 기존 Legacy의 정확성으로 대체하지 않는다.

#### 작은 검사와 수정 전후

정식 `tests/cases/issue_7353_table_v2_document_flow.rs`에5건 추가:

| 주장 | 검사 |
| --- | --- |
| 방향/영역/병합 | 같은 저장 HWP의 실제 Line 끝점·색·폭,1쪽5개/2쪽2개 선,47개 셀 text 각각1회, 뒤 문단 위치 |
| 비영향 조판 | 대각선만 제거한 명시적 진단 대조와 Table/Cell/TextLine/TextRun bbox·줄 소속 동일 |
| 중첩 | child bounds 끝점 일치, parent padding 재가산 없음, 빈 host 줄과 뒤 문단 보존 |
| 미지원 효과 | 다중 ray/꺾임/회전/비실선/3D/중심선/전체 표/중첩 우선순위 거부 |
| 내부 분할·잘못된 pen | 긴 정상 저장본은 명시적 거부, resolved API의 width255도 정확한 오류로 거부 |

새 정상 HWP 검사는 이전 release-test 코드에서 `V2 source decoration effect`로
실패했다(`diagonal/test-before.log`). 최종 debug document-flow **54 passed /0 failed**
(`diagonal/test-after.log`). 초기에 실패한3건은 원본 차단지점 갱신, 실제 빈 host 텍스트
두 개를 포함한 기대 목록, HWPX serializer가 폭255를 정상 폭으로 변환하는 합성 입력 문제였다.
폭255는 직접 resolved API 검사로 옮겼다. 조판 기대값 완화나 수치 허용치 변경은 없다.

원본 전체는 이제 index5 `stored text indentation`에서 거부된다. 장식 검증 단계를
넘어갔다는 관측이지 원본 대각선·표 전체가 최종 출력에서 검증됐다는 뜻은 아니다.
중첩 내부 분할/겹친 영역 등 후속 수용 조건도 남을 수 있다.

Native review1·2와 standalone overlay2를 직접 보고 위 대각선·외곽·후속 본문을 확인한 뒤
집중 release-test와 Docker fresh WASM 검증을 시작했다. 아래에 최종 결과를 연결한다.

검증 source는 `80967bc75`+이번 패치, 고정 목록은
`output/7353/r19/diagonal/source.sha256`이다. 제품/review V2 소스 동일, changed-file
rustfmt check와 `git diff --check`를 확인했다. 신규 source-side test나 파생 suite 변경은 없다.

```sh
# review worktree, 기존 파생 suite 재사용
CARGO_BUILD_JOBS=2 cargo nextest run --locked --cargo-profile release-test \
  --test regression_suite_003 --test regression_suite_004 --test regression_suite_005 \
  --test regression_suite_015 --test regression_suite_027 --test regression_suite_028 \
  -E 'test(issue_7353_table_v2)' --no-fail-fast \
  --target-dir /home/edward/mygithub/rhwp/target/pr-review
# product worktree
docker compose --env-file .env.docker -p rhwp run --rm wasm
```

#### 직선 대각선 최종 검증 결과

- `diagonal/focused-initial.log`:174 passed/1 failed. 기존 export의 gradient 거부 검사에
  attr8도 무조건 거부한다는 이전 지원 범위가 남아 있었다. 새 한컴 대조에서 확인한 직선8은
  별도 양성 좌표 검사로 보호하고, 기존 거부 항목은 미지원 다중 ray12로 변경했다.
  renderer는 다시 바꾸지 않았다. 이 변경은 무회귀 허용치·golden 완화가 아니다.
- 같은6개 suite의 재실행 `diagonal/focused-final.log`: **175 passed/0 failed/1122 skipped**.
  변경된 export suite만21.77초 재컴파일하고 전체175건을 실행했다. 전체 CI가 아닌 V2 집중 회귀다.
  nextest0.9.137/권장0.9.140 및 기존 관측용 설정 경고는 그대로다.
- `diagonal/docker-wasm.log`: **7분47초 성공**. WASM SHA-256
  `3a90f68db79105781091c19b0fd4461d79cdc9a7378077637b43bde4c3b93a58`.
  위 export 테스트 계약만 바뀌었으므로 같은 renderer로 WASM을 중복 빌드하지 않았다.
- 최종 source hash 검사, 제품/review V2 동일 확인, changed-file rustfmt check와
  `git diff --check` 통과. 전체 PR lint/CI, 원본 #6923 전체 출력은 미검증이다.

```sh
# 최종 release-test 라이브러리에 연결한 probe
output/7353/r19/diagonal/probe tests/fixtures/issue7353_diagonal_review/diagonal-saved.hwp \
  output/7353/r19/diagonal/review/actual render
output/7353/r19/diagonal/probe tests/fixtures/issue7353_zone_review/zone-lines-saved.hwp \
  output/7353/r19/diagonal/control render
node output/7353/r19/diagonal/review.mjs --wasm
node output/7353/r19/diagonal/control.mjs
```

새 Native/fresh WASM SVG는 두 쪽 모두 byte-identical이며 JSON은 숫자171곳 최대
5.684e-14 차이, 다른 차이0이다(`review/backend-comparison.json`). 앞 절편의 영역 배경
정상 대조는 변경 전후 Native/WASM 각각2쪽 SVG가 모두 동일하다(`control/comparison.json`).
최종 Native와 WASM의 review 및 WASM standalone overlay1·2를 직접 열어 대각선·외곽·
후속 본문을 비교했다. 폰트 외형 차이는 이번 지시대로 조판 실패와 분리한다.
source/input/PDF/WASM 고정값과 무변환 비교 조건은 `review/run.json`에 있다.

시각 판정 요청:

- [1쪽 WASM review](../../output/7353/r19/diagonal/review/wasm-review-1.png):1~3행 방향·색,
  4·5행 선 없음,18~19행 영역 대각선.
- [2쪽 WASM review](../../output/7353/r19/diagonal/review/wasm-review-2.png):20~21행 영역에서
  대각선 재시작,24행 병합 셀 전체 폭,표 뒤 문단.
- [한컴 저장 HWP](../../tests/fixtures/issue7353_diagonal_review/diagonal-saved.hwp),
  [같은 HWP의 PDF](../../tests/fixtures/issue7353_diagonal_review/diagonal-2020.pdf).

상태: **한정된 직선 대각선 구현·집중 회귀·fresh WASM 완료, 메인테이너 시각 판정 통과**.
작업지시자의 “판정 통과입니다. 다음 절편 진행을 승인합니다.”에 따라 이 범위의 판정을
기록하고 로컬 체크포인트 후 저장 문단 들여쓰기 절편으로 진행한다.
다음 종단 차단은 원본 index5의 저장 문단 들여쓰기다. 셀 내부 분할 대각선 등 미검증 범위는
계속 거부하며, 원본 전체/R5 완료 또는 한컴과의 전면 일치로 보고하지 않는다.

### 다음 절편 — 저장 문단 들여쓰기의 물리 줄 상자

직선 대각선 시각 통과분을 `105b4cbc6`으로 로컬 커밋했다. 이번 범위는 유효한 단일
저장 줄의 들여쓰기/내어쓰기다. 원본 #6923 `s0/p5/t0/c0/p1`은 indent−5928,
좌1100/우680, 저장 cs550/sw46726이며2~4줄에 bit20을 기록한다. 저장 cs는 여백만
포함하고 실제2964HU 들여쓰기는 포함하지 않는다. 원본의 값이나 줄바꿈을 수정하지 않는다.

독립 대조는 [저장 HWP](../../tests/fixtures/issue7353_indent_review/indent-saved.hwp)와
[동일 HWP의 한컴 PDF](../../tests/fixtures/issue7353_indent_review/indent-2020.pdf)다.
생성 방식·job·hash·PDF 관측치는 해당 fixture README에 있다. 수작업 LineSeg를 넣지 않고
HWPX를 한컴에서 정상 저장했다. 첫 줄1500HU 들여쓰기/후속1500HU 내어쓰기와
Left/Center/Right 정렬,16줄 소유·표 뒤 문단이 독립 기대값이다.

소비 경로: `stored_text::localize`가 저장 rows의 출처/범위/소유를 검증한 뒤 bit20이
켜진 줄에만 `cs += abs(indent)`, `sw -= abs(indent)`를 적용한 지역 사본을 만든다.
원래 IR은 그대로다. `text.rs::TextComposer`는 이 결과를 compose와
`layout_composed_paragraph_in_frame(physical_frame_rows=true)`에 전달한다.
공용 배치의 `uses_stored_segment_geometry`와 physical-frame 여백0 경로가 이 cs/sw를
소비하므로 문단 여백·들여쓰기를 다시 더하지 않는다. `validate_paint`가 최종 줄 상자,
baseline,텍스트 소속,문단 전진을 검사한 다음 같은 노드를 측정의 `ParagraphItem::Lines`와
paint payload에 보관한다. 이어받기는 기존 LineOwner/줄 단위 fit과 평행 이동만 사용한다.
TAC/rowspan/cut/높이 산식,Legacy 기본값은 변경하지 않았다.

반례는 bit20이 없는 저장 줄(스타일 값만으로 이동하면 안 됨), 첫 조각/이어받은 줄,
본문→빈 문단→뒤 문단,중첩 부모 여백,dirty 저장 정보,줄 폭을 다 먹는 inset이다.
integer-HU 줄 상자에서 표현할 수 없는 활성 소수-HU 들여쓰기는 임의 반올림하지 않고
`stored indentation precision`으로 거부한다. 빈 문단의 줄 높이를0으로 만들지 않는다.

정식 회귀는 기존 `tests/cases/issue_7353_table_v2_document_flow.rs`와
`issue_7353_table_v2_text.rs`에 추가했다. 한컴 대조 좌표/줄 소유·정렬,본문 saved flag,
이어받기/중첩 실제 원점·폭·높이·뒤 내용과 종료를 검사한다.14px 예산은 top padding3px만
수용하고 텍스트는 다음 조각에 보존한다. 기존의 무조건 indent 거부 assertion은
비활성 flag의 x불변 계약으로 교체했다. baseline/ignore/래칫은 변경하지 않았다.

작은 검증은 `output/7353/r19/indent/`에 보존한다.

- `test-before-final-fixture.log`: 최종 한컴 HWP를 이전 코드에 넣어
  `stored text indentation`으로 FAIL. 빌드 실패가 아닌 지원 제한 재현이다.
- `test-after-final.log`: document-flow56 PASS. `text-after-final.log`: text20 PASS.
- 초기 테스트 작성 오류(섹션 제어문자 위치가 없는 첫 문단 합성 저장 offset,private replay API,
  오른쪽 정렬의 끝 공백 caret 폭 포함)는 수정했다. 기대값을 구현 수치로 완화하지 않았고,
  마지막 보이는 run의 우측 정렬 및 PDF 실제 우측 끝을 구분해 검사한다.
- 원본 전체는 index5 `TAC carrier paragraph constraints`에서 명시적으로 거부된다.
  들여쓰기 제한을 넘었다는 관측이지 원본 전체/R5 통과가 아니다.

#### 별도 남은 차이와 미검증 범위

1. `tests/fixtures/issue7353_indent_review/diagnostic/`는 마지막 문단 아래 간격400HU를
   갖는 정상 저장본이다. V2는 표 하단·뒤 문단이400HU=5.333px 더 내려간다.
   `indent/terminal-spacing/review/`에 실제 비교를 보존했다. 마지막 아래 간격0 대조군을
   별도 정상 저장해 들여쓰기 검증을 분리했다. 원본 진단의 시각 통과를 주장하지 않는다.
2. LineSeg 없는 기존 재조판 경로는 폭 계산에는 indent를 쓰지만 physical-frame paint의
   cs/sw에 반영하지 못한다. 저장본 적용 규칙을 이 경로의 증거로 대신하지 않는다.
   `fresh-diagnostic.rs`의 `fresh_only_no_saved_rows`, `fresh-before.log`와
   `fresh-after.log`에서 이전/현재 모두 기대 x50 대비 x30으로 실패한다. 별도 후속 수정 대상이다.
3. 기존 `samples/lseg-04-indent.hwp`/`pdf/lseg-04-indent-hwp-2020.pdf`도 조사했다.
   한컴 PDF의 첫 줄20pt 들여쓰기는 동일 규칙을 뒷받침하지만,V2 전체는
   `text preview run outside occupied line`로 거부된다. 이 문서 전체의 시각 증거로 쓰지 않았다.
4. TAC carrier와 어울림 복수 segment,활성 소수-HU inset,원본 #6923 전체는 미검증/미지원이다.

최종 소스는 `105b4cbc6`+패치이며 `indent/source.sha256`으로 고정한다. 변경 파일 fmt와
diff check를 확인했고,집중 release-test 및 Docker fresh WASM 결과는 아래에 연결한다.

#### 저장 문단 들여쓰기 최종 검증

```sh
# review worktree: 기존 파생 suite, 변경 cases 원본만 동기화
CARGO_BUILD_JOBS=2 cargo nextest run --locked --cargo-profile release-test \
  --test regression_suite_003 --test regression_suite_004 --test regression_suite_005 \
  --test regression_suite_015 --test regression_suite_027 --test regression_suite_028 \
  -E 'test(issue_7353_table_v2)' --no-fail-fast \
  --target-dir /home/edward/mygithub/rhwp/target/pr-review
# product worktree
docker compose --env-file .env.docker -p rhwp run --rm wasm
output/7353/r19/indent/probe tests/fixtures/issue7353_indent_review/indent-saved.hwp \
  output/7353/r19/indent/review/actual render
node output/7353/r19/indent/review.mjs --wasm
node output/7353/r19/indent/control.mjs
```

- 집중 release-test **178 passed /0 failed /1122 skipped** (`indent/focused-final.log`).
  기존6개 suite의 V2 필터 결과이며 전체 CI가 아니다. nextest 버전 및 기존 관측 설정 경고는
  그대로다. 제품/review 소스·테스트 동일,최종 source hash·변경 파일 fmt·diff check 통과.
- Docker fresh WASM7분13초 성공(`indent/docker-wasm.log`). SHA-256
  `df212e8dd75a9efb4ac8bfe9223bb3a8fa96b24f3cd4f63340b05d123904f080`.
- 최종 release-test 라이브러리로 Native 재출력,새 WASM `DocumentV2`로 같은 HWP를 직접
  출력했다. SVG1쪽 byte-identical,JSON 숫자41곳 최대5.684e-14,나머지 차이0
  (`review/backend-comparison.json`).
- 이전 시각 통과 대각선 정상 대조2쪽은 Native와WASM 각각 변경 전후 SVG 동일,
  두 backend도 동일(`control/comparison.json`).
- 최종 Native/WASM review와 WASM standalone overlay를 직접 열어 들여쓰기·내어쓰기,
  저장16줄의 줄바꿈,정렬,표 외곽과 뒤 문단을 확인했다. source/input/PDF/WASM 고정값과
  무변환 비교 조건은 `review/run.json`이다. 대체 글꼴 외형은 판정 범위를 구분한다.
- 전체 PR lint/전체 CI,Studio Canvas 편집,원본 #6923 전체 출력은 미검증이다.
  기본 엔진 전환·원격 push·PR은 하지 않았다.

시각 판정 자료:

- [fresh WASM 한컴 비교·겹침](../../output/7353/r19/indent/review/wasm-review-1.png)
- [WASM standalone overlay](../../output/7353/r19/indent/review/wasm-overlay-1.png)
- [Native 비교·겹침](../../output/7353/r19/indent/review/native-review-1.png)
- [한컴 저장 HWP](../../tests/fixtures/issue7353_indent_review/indent-saved.hwp),
  [같은 HWP의 PDF](../../tests/fixtures/issue7353_indent_review/indent-2020.pdf)

이번 저장 문단 절편은 메인테이너가 시각 판정 통과하고 다음 절편을 승인했다. 별도 재조판 들여쓰기/셀 끝 아래 간격
문제와 원본의 TAC carrier 제한은 남아 있으며,A/R5 완료로 세지 않는다.

#### 후속: 저장 LineSeg 없는 재조판 들여쓰기

디스크 정리 후 중단 지점에서 재개했다. 원인은 줄 채우기에서는 들여쓰기만큼 폭을 줄이지만,
physical-frame 배치가 소비하는 LineSeg cs/sw에는 그 시작점·폭을 기록하지 않은 것이다.
수정 전 첫 줄은 기대 x50 대신 x30에 배치됐다. 저장본 경로의 통과와 별개인 결함이다.

- 생산: `composer/line_breaking.rs::layout_paragraph_in_physical_frame`에서 활성 줄의
  inset을 기존 HWPunit 격자로 변환하고, 실제 줄 채우기에 쓸 구간을 먼저 결정한다.
  동일 구간으로 terminal replay/일반 fill/kerning fill을 수행하며 이중 indent를 제거한다.
  재시도 후 실제 수용된 구간만 반환 LineSeg cs/sw에 기록한다.
- 소비: `table_v2/text.rs`의 fresh 가지가 위 결과를 받아 공통 paragraph layout으로 전달한다.
  `layout/paragraph_layout.rs`의 physical-frame 가지는 cs/sw를 최종 줄 상자로 사용하고
  스타일 들여쓰기를 다시 더하지 않는다. 같은 줄 bbox/내용이 `ParagraphItem::Lines`와
  fragment paint에 전달되며,fit/append에서 들여쓰기 원점을 별도로 추정하지 않는다.
- 기존 `layout_paragraph_in_frame`과 저장 LineSeg 경로,Legacy 기본값은 유지한다.
  exclusion 복수 구간은 신규 API에서 명시적으로 거부한다. 비유한 inset·너비를 다 먹는
  inset도 clamp하지 않는다. 페이지 컷·이어받기·높이 산식은 변경하지 않았다.

독립 시각 입력은 `tests/fixtures/issue7353_fresh_indent_review/README.md`에 출처를 기록했다.
원본 `fresh-input.hwpx`는 저장 줄이 없는 입력이고,한컴 정상 저장 `fresh-saved.hwp`와
같은 문서의 PDF는 기대값/시각 기준에만 쓴다. 저장 줄을 입력에 주입하지 않았다.
셀 최소 높이28000HU·끝 아래 간격0으로 미해결 terminal spacing과 분리했다.
초기 비지원 anchor 입력은 별도 보존했으며 원본 #6923 전체의 통과 자료가 아니다.

정식 기존 cases에4개 검사를 추가했다. 실제 최종 좌표·폭·텍스트·빈 줄·표 외곽·뒤 문단,
양수/음수/0 inset과 좌/중앙/우 정렬,너비 부족 줄바꿈,중첩 원점과 페이지 이어받기에서
줄 소유·종료를 확인한다. 기대값은 입력 속성/한컴 저장 관측/정렬 불변식에서 정했다.
한컴 segment와 선언 폭의2HU(약0.027px) 차이는 별도로 기록하며 기준값으로 숨기지 않았다.

작은 검증과 수정 전 증거는 `output/7353/r19/fresh-indent/`에 있다.

- 수정 전 `b08ed5e52`: `document-before-final.log`의2개 새 검사가 x30≠50,
  x63.36≠83.36으로 FAIL. `text-before.log`의2개 새 검사도 실제 좌표 불일치로 FAIL.
  초기 링크/테스트 작성 오류는 결함 검출 증거에서 제외했다.
- 수정 후 debug: `document-after.log` **58 PASS**, `text-after.log` **22 PASS**.
- `before-final/native-review-1.png`와 `review/native-review-1.png`를 직접 비교해
  첫 줄/후속 내어쓰기 줄의 위치 개선을 확인했다. 최종 release/fresh WASM 증거는 아래에 잇는다.
- source head는 `b08ed5e52`+현재 패치,고정 manifest는 `fresh-indent/source.sha256`이다.
  baseline/ignore는 변경하지 않았다. 원본 TAC carrier,셀 끝 아래 간격,복수 exclusion,
  Studio Canvas 편집과 전체 PR lint/CI는 이번 결과의 완료 범위가 아니다.

최종 집중 검증 명령:

```sh
# rhwp-review-7353, 기존 파생 suite에서 실행
CARGO_BUILD_JOBS=2 cargo nextest run --locked --cargo-profile release-test \
  --test regression_suite_002 --test regression_suite_003 \
  --test regression_suite_004 --test regression_suite_005 \
  --test regression_suite_015 --test regression_suite_017 \
  --test regression_suite_027 --test regression_suite_028 \
  -E 'test(issue_7353_table_v2) | test(issue_4755) | test(issue_6102) | test(issue_3128)' \
  --no-fail-fast --target-dir /home/edward/mygithub/rhwp/target/pr-review
```

`fresh-indent/focused-final.log`: **187 passed /0 failed /1532 skipped**.
nextest0.9.137 대비 권장0.9.140 및 기존 관측 설정 경고가 남는다.
최종 source manifest·변경 파일 fmt·diff check 통과. release-test 라이브러리로 최종 Native
대조군1쪽 및 기존 시각 통과 대각선2쪽/저장 들여쓰기1쪽을 다시 출력했다.

```sh
# product worktree
docker compose --env-file .env.docker -p rhwp run --rm wasm
output/7353/r19/fresh-indent/probe tests/fixtures/issue7353_fresh_indent_review/fresh-input.hwpx \
  output/7353/r19/fresh-indent/review/actual render
output/7353/r19/fresh-indent/probe tests/fixtures/issue7353_diagonal_review/diagonal-saved.hwp \
  output/7353/r19/fresh-indent/control-diagonal render
output/7353/r19/fresh-indent/probe tests/fixtures/issue7353_indent_review/indent-saved.hwp \
  output/7353/r19/fresh-indent/control-indent render
node output/7353/r19/fresh-indent/review.mjs --wasm
node output/7353/r19/fresh-indent/control.mjs
```

- Docker fresh WASM **7분47초 성공**, `fresh-indent/docker-wasm.log`.
  SHA-256 `6301762662faeec1cdbcd7d19ee9ec120c9b847302368ae4fc933ca7a52b549d`.
- Native/fresh WASM의1쪽 SVG byte-identical,JSON 숫자38곳 최대차이5.684e-14,
  그 밖의 차이0 (`review/backend-comparison.json`).
- 기존 대각선2쪽·저장 들여쓰기1쪽 모두 각 backend의 변경 전후 SVG 동일,
  Native/WASM 상호 동일 (`control-diagonal/comparison.json`, `control-indent/comparison.json`).
- 최종 Native review,WASM review,standalone overlay를 직접 열어 첫 줄15pt inset,
  내어쓰기 후속 줄,중앙/오른쪽 정렬,실제 빈 줄,표 경계와 뒤 문단 위치를 확인했다.
  대체 글꼴 외형 및2HU 폭 차이는 남겨 기록한다. 한컴 저장 줄을 주입하지 않은
  대조군 검증이며 원본 #6923 전체의 시각 통과로 확대하지 않는다.
- source/input/PDF/WASM 고정값·96dpi·무변환 비교 조건은 `review/run.json`에 있다.
  빌드 후 source manifest 재확인,fmt/diff check 통과. 원격 push/PR/기본 엔진 전환 없음.

시각 판정 요청 자료:

- [새 WASM 한컴 비교·겹침](../../output/7353/r19/fresh-indent/review/wasm-review-1.png)
- [WASM standalone overlay](../../output/7353/r19/fresh-indent/review/wasm-overlay-1.png)
- [수정 전 Native 비교](../../output/7353/r19/fresh-indent/before-final/native-review-1.png)
- [원본 무저장 HWPX](../../tests/fixtures/issue7353_fresh_indent_review/fresh-input.hwpx)
- [한컴 저장 HWP](../../tests/fixtures/issue7353_fresh_indent_review/fresh-saved.hwp),
  [한컴 PDF](../../tests/fixtures/issue7353_fresh_indent_review/fresh-2020.pdf)

메인테이너가 이번 재조판 들여쓰기의 시각 판정 통과 및 다음 절편 진행을 승인했다.
셀 끝 아래 간격과 원본 TAC carrier 지원은 남아 있으며 R5 완료로 보고하지 않는다.

#### 후속: 셀 마지막 문단 아래 간격의 명시적 정책

시각 통과한 재조판 들여쓰기는 `82b96e5ea`로 체크포인트 커밋했다. 이번에는 이전에
보존한 `tests/fixtures/issue7353_indent_review/diagnostic/indent-saved.hwp`와 대응 PDF를
그대로 사용한다. 새 한컴 변환이나 저장 줄 수정 없이 비영 끝 간격 문제를 검증한다.

독립 근거: 한컴 저장본 마지막 줄은 cell-local y27600HU,높이1100HU이며 위아래 여백은
각283HU다. 따라서 점유 끝 기준 외곽은29266HU다. 마지막 ParaShape 아래 간격800
(실제400HU)을 가진 진단본과0인 승인 대조본의 한컴 외곽/뒤 문단은 동일하다.
진단 PDF의 표 하단과 `AFTER INDENTED TABLE`,저장 뒤 문단 y32951HU를 함께 확인했다.
선언 cell.height28000HU는 최소 높이이지 실제 점유 높이29266HU가 아니다.

기존 `omit_final_line_gap`은 마지막 줄간격만 제외하고 paragraph-after를 보존하도록
의도적으로 제한한 실험 옵션이었다. 이를 조용히 재정의하거나 Legacy/default를 바꾸지 않고,
`cell_end_policy: "omit_final_paragraph_gap"`을 추가했다. 셀 끝의 **마지막 문단**에서만
다음 줄간격과 문단 뒤 간격을 제외한다. 실제 빈 줄·문단 앞 간격·문단 사이 간격·명시적
Space·padding·선언 최소 높이는 유지한다. 본문 문단에는 적용하지 않는다.

생산/소비와 적용 경계:

| 경계 | 이번 처리 |
| --- | --- |
| `text.rs`/`pictures.rs`/`tac.rs`의 `ParagraphEnd::from_composed` | 실제 줄/객체 점유와 후속 origin 외에 의미가 확인된 paragraph-after를 `Some`으로 표시 |
| public `ParagraphEnd::new`의 사용자 composer | 뒤 공간의 의미를 모르는 `None`이므로 새 옵션에서도 임의 제거하지 않음 |
| `ir.rs::bind_table` / `text_flow.rs::from_flow_rows_with_end_policy` | 실제 마지막 문단 경계에서만 `into_flow_items_at_end`를 호출. 후속 explicit Table이면 앞 문단 간격 보존; Space(0)/양수는 별도 물리 공간 |
| `paragraph_end.rs::into_flow_items_at_end` | 기존 두 정책은 보존. 새 정책은 의미가 확인된 terminal after만 제외; 음수 최종 줄 advance는 기존 line-gap 정책과 같이 물리 줄 높이까지 보존 |
| `content.rs` 물리 요구 높이 → `flow.rs::fit_cell` 예약/컷 → `TextFragment::append_to` | 같은 변환 결과를 소비. paint에서 다시 높이를 늘리거나 clamp하지 않음 |

내용 컷/rowspan/제목 반복 알고리즘 자체는 바꾸지 않았다. 영향을 받는 마지막 줄의
수용 예산·이어받기·실제 빈 줄·마지막 유닛 종료·표 뒤 문단을 검사한다. 저장/재조판 텍스트와
중첩 explicit 자식은 검증하되,그림/TAC의 비영 after·분할 rowspan의 한컴 출력까지 이번
단일 PDF로 입증했다고 주장하지 않는다.

수정 전후 작은 검증 (`output/7353/r19/cell-after/`):

- `before-selector-only.patch`는 새 옵션 이름을 기존 line-gap 동작에 연결한 진단 단계다.
  높이 산식은 수정 전과 같고,이 상태에서 최종 이름을 사용하는 같은 검사를 실행했다.
  `document-before.log`: 실제395.546667px 대비 독립 기대390.213333px로 FAIL.
  `text-before.log`: 마지막 빈 줄 수용/음수 끝 간격 검사2개 FAIL,나머지20 PASS.
  옵션이 없던 원래 `82b96e5ea`에서 새 이름을 사용할 수 있었다고 주장하지 않는다.
- 초기 diagnostic 테스트의 선언 최소 높이와 실제 높이 혼동,수동 rustc의 서로 다른 serde
  의존성 링크 실패는 정정했으며 결함 검출 근거에서 제외했다.
- 수정 후 `check.sh`: document-flow59,text23,nested16 = **98 PASS**.
  정상 한컴 진단본16줄의 최종 위치/표 높이/뒤 문단과 본문의 비영 아래 간격을 검사했다.
  합성 경계에서는 blank 보존,양/음수 끝 줄간격,39px 정확 예산,34px 분할 후 blank 이월,
  Space0/2/7 가산성,명시적 후속 표 앞 간격,60px 선언 최소 높이,opaque tail 보존을 검사했다.
- Native 선행 review를 직접 열어 기존400HU 외곽·뒤 문단 초과가 사라지고 내부 줄과
  들여쓰기/정렬은 유지됨을 확인했다. 글꼴 외형 차이는 그대로 구분한다.
- source는 `82b96e5ea`+패치이며 `cell-after/source.sha256`으로 고정한다.
  소스 수정 없이 본문 after 반례를 추가한 최종 cases를 review에 동기화했다.
  집중 release-test 및 Docker fresh WASM 결과는 아래에 연결한다.

최종 집중 회귀: 직전 절편과 같은8개 generated suite에서
`test(issue_7353_table_v2) | test(issue_4755) | test(issue_6102) | test(issue_3128)` 필터,
`--locked --cargo-profile release-test --no-fail-fast`,공유 target `rhwp/target/pr-review`로
실행했다. **190 passed /0 failed /1532 skipped** (`cell-after/focused-final.log`).
nextest 버전/기존 관측 설정 경고는 유지된다. 전체 CI/PR lint를 실행한 결과가 아니다.
`82b96e5ea`의 이전 Native probe와 수정 후 기존 옵션의 진단본 JSON도 byte-identical이며,
이전 미통과 진단 PNG의 원본 JSON과도 동일함을 확인했다. source manifest·변경 파일 fmt·
diff check 및 review source 동일성을 확인했다.

최종 fresh WASM·시각 증적:

- `docker compose --env-file .env.docker -p rhwp run --rm wasm`: 성공,7분43초
  (`cell-after/docker-wasm.log`). WASM SHA-256:
  `7d9fa78bb6655c66ef32dbf8d3ab3bee3c7497beb74d181376a8e9947ccb83dd`.
- release-test Native probe와 새 WASM의 동일 진단 HWP 결과는 SVG byte-identical이다.
  JSON 수치 차이41개는 최대 `5.684341886080802e-14`,비수치 차이0이다
  (`cell-after/review/backend-comparison.json`).
- `node output/7353/r19/cell-after/review.mjs --wasm`으로 기존 한컴 PDF와
  Native/fresh WASM compare·standalone overlay·review를 생성했다.
  최종 Native/WASM review와 overlay를 직접 확인했다. 표 내부16줄을 유지하면서
  표 하단과 `AFTER INDENTED TABLE`의 기존400HU 초과가 해소되었다.
  글꼴 외형·폭 차이는 별도로 남으며,이 결과를 원본 #6923 전체 통과로 간주하지 않는다.
- `node output/7353/r19/cell-after/control.mjs`: diagonal,저장 indent,fresh indent
  3종×기존/신규 정책6조합에서 승인 출력 보존 및 Native/WASM SVG 일치를 확인했다
  (`cell-after/controls.log`). 최초 실행의 증적 경로 오타는 출력 전용 스크립트에서
  정정 후 재실행했으며 제품 코드/fixture는 변경하지 않았다.
- source/head·입력/PDF·WASM 해시와 실행 profile은 `cell-after/review/run.json`,
  검증한 소스는 `cell-after/source.sha256`에 연결했다. 최종 해시 재검사4개 모두OK.

판정 자료:

- [fresh WASM review](../../output/7353/r19/cell-after/review/wasm-review-1.png)
- [fresh WASM standalone overlay](../../output/7353/r19/cell-after/review/wasm-overlay-1.png)
- [Native review](../../output/7353/r19/cell-after/review/native-review-1.png)
- [진단 HWP](../../tests/fixtures/issue7353_indent_review/diagnostic/indent-saved.hwp)
- [한컴 기준 PDF](../../tests/fixtures/issue7353_indent_review/diagnostic/indent-2020.pdf)

메인테이너가 시각 판정 통과 및 다음 절편 진행을 승인했다. 기존 정책과 Legacy/default를 유지하며,
그림/TAC·분할 rowspan의 추가 독립 출력 검증과 원본 #6923/R5 완료는 남아 있다.

#### 후속: TAC carrier의 비표시 문단 테두리 참조

직전 셀 끝 아래 간격 절편은 시각 판정 승인 후 `6c535e20d`에 보존했다.
이번 대상은 원본 #6923의 index5 내부 TAC carrier가 borderFill1을 가진다는 이유로
`TAC carrier paragraph constraints`에 걸리는 경계다. 원본의 이 참조는 보이는 장식이 아니다.
일반 텍스트와 다른 수용 조건을 두지 않고, 기존 `paragraph_is_unpainted` 계약을 TAC/그림의
공통 `carrier_style`에 적용한다. margin/indent·줄 소속·저장 점유 상자 검사는 완화하지 않는다.

규칙과 실제 소비 경로:

- 원본 `DocInfo → validate_paragraph_source`가 3D·채움 등 source 효과와 참조 유효성을 검사한다.
- `ResolvedStyleSet → tac::carrier_style → paragraph_is_unpainted`에서 실제 장식 없는 참조만
  수용한다. `text`와 같은 규칙이며 missing reference와 실제 선·채움은 여전히 거부한다.
- `tac::compose / pictures::compose → stored_object_rows`의 원래 줄 소속/기하를 그대로
  사용한다. `ParagraphItem → content/flow`의 요구·예약/컷과 재귀 표·그림 paint는 바꾸지 않는다.
  참조를 지우거나 좌표·height·clamp를 보정하지 않는다. 본문/중첩/그림의 실제 최종 트리를 비교한다.

독립 입력은 `tests/fixtures/issue7353_tac_noop_review/`의 정상 한컴 저장본과 대응 PDF다.
생성 출처·해시·기대 좌표는 해당 README에 한 번만 기록한다. 원본 #6923과 혼동하지 않도록
부모/자식 및 앞뒤 문단을 명시한 읽을 수 있는1쪽 대조군이다. 저장본은 수동 수정하지 않았다.
Native review/overlay를 직접 확인한 결과 표 외곽·중첩 배치·뒤 문단은 기준과 정합하고,
글꼴 외형 차이는 남는다. 메인테이너 최종 판정과 fresh WASM은 아래 결과로 연결한다.

수정 전후 작은 검증 (`output/7353/r19/tac-noop/`):

- 수정 전 `6c535e20d`와 동일한 release-test lib로 최종 정상 HWP 및 비표시 참조 반례를
  실행하여 두 검사 모두 **TAC carrier paragraph constraints**로 FAIL (`before.log`).
- 초기 검사에서 필수 max_pages 옵션 누락/BorderLine 기본값을 None으로 오인한 오류와
  fixture generator의 borrow 및 진단 필드명 오류는 수정했다. 이를 결함 검출 근거로 세지 않는다.
- body/nested × 같은 줄/다른 줄 TAC의 참조0 대조군과 비표시 참조 입력은 전체 tree/SVG가 같다.
  missing/visible 참조의 명시적 실패도 검사한다. 그림 carrier의 같은 줄/다른 줄은 별도 export
  계약으로 실제 image·분할·뒤 문단/종료의 동일성을 검사한다.
- 원본 전체의 다음 거부는 index5 `text preview run outside occupied line`으로 진전했다.
  기존 admission 검사는 이 정확한 다음 경계를 기록하도록 갱신했다. 수용 조건 해제만으로
  원본 전체나 R5 완료로 판단하지 않는다. baseline/ignore와 Legacy/default는 변경하지 않았다.

소규모 최종 document-flow 검사61건은 모두 PASS (`after-final.log`). 실제 한컴 저장본의
선언 최소 높이·저장 TAC 줄 폭/높이·앞뒤 문단 vpos로 기대값을 정해 최종 표/줄 좌표를 검사했다.
수정 전 Docker WASM(`7d9fa78b…`)으로도 같은 최종 HWP의 거부를 확인했다
(`before-wasm.json`). 제품 코드 변경은 V2의 공유 수용 조건 한 곳이며, 이전 검증한 셀 끝 정책은
그대로 유지한다. source/head는 `6c535e20d`+패치, 정확한 source/cases 해시는
`tac-noop/source.sha256`에 고정하고 review worktree와의 동일성을 확인했다.

최종 출력 검증:

- Docker `wasm` 성공, **7분28초** (`tac-noop/docker-wasm.log`), WASM SHA-256
  `1860044d9a401502bb886a4412d033318f1bc8ceecdb2b816f26e312e2ee2964`.
- `bash output/7353/r19/tac-noop/finalize.sh`로 최종 release-test Native probe와 fresh WASM을
  같은 저장본에 실행했다. SVG byte-identical, JSON 수치 차이26개/최대
  `5.684341886080802e-14`, 비수치 차이0 (`review/backend-comparison.json`).
- diagonal/저장 indent/fresh indent × 두 끝 간격 정책6조합의 기존 승인 SVG 보존과
  Native/WASM 일치를 확인했다 (`controls.log`). 이는 동일 출력 보존 검사이며 각 입력의
  모든 미지원 동작을 새로 시각 승인한 결과는 아니다.
- 최종 Native/fresh WASM review 및 standalone overlay를 직접 열어 부모·자식 표 외곽,
  셀 앞/뒤 문단과 표 뒤 본문을 확인했다. 별도의 좌표 정합 변환은 하지 않았다.
  대체 글꼴 폭·굵기 차이는 별도로 남긴다. 원본 #6923 전체는 다음 텍스트 경계에서 미수용이다.

메인테이너 판정 자료:

- [fresh WASM review](../../output/7353/r19/tac-noop/review/wasm-review-1.png)
- [standalone overlay](../../output/7353/r19/tac-noop/review/wasm-overlay-1.png)
- [Native review](../../output/7353/r19/tac-noop/review/native-review-1.png)
- [동일 HWP 샘플](../../tests/fixtures/issue7353_tac_noop_review/noop-saved.hwp)
- [한컴 기준 PDF](../../tests/fixtures/issue7353_tac_noop_review/noop-2020.pdf)

메인테이너가 이 절편의 시각 판정 통과와 다음 절편 진행을 승인했다. 전체 PR CI/lint·원격 게시·기본 엔진 전환은
수행하지 않았으며, 집중 회귀 최종 결과는 아래에 기록한다.

집중 release-test 결과: **193 passed /0 failed /1532 skipped** (`tac-noop/focused.log`).
review worktree에서 이전 절편과 동일한8개 suite(002/003/004/005/015/017/027/028)에
`test(issue_7353_table_v2) | test(issue_4755) | test(issue_6102) | test(issue_3128)` 필터,
`--locked --cargo-profile release-test --no-fail-fast --target-dir /home/edward/mygithub/rhwp/target/pr-review`
옵션으로 실행했다. nextest0.9.137/권고0.9.140 및 기존 설정 경고는 유지된다.
최종 source manifest3건·fmt·diff check를 확인했고, 검증 이후 제품 코드 변경은 없다.

#### 후속: 양쪽 정렬 말미 공백의 정밀도와 서식 소유

앞 절편의 시각 승인과 결과는 `d8c184085`에 커밋했다. 다음 원본 거부
`text preview run outside occupied line`을 추적한 결과, 정확한 대상은
`s0/p5/t0/c0/p4/t0/c6/p1`의 “○ 나머지 약 79.7%…” 첫 줄이었다.
저장 오른쪽 경계308.96px와 실제 가시 advance 끝309.0146666666667px 사이에
약0.054667px 초과가 있었다. 세로 점유나 저장 줄 소속 문제가 아니다.

공통 문단 배치는 전체 줄과 run 폭을 소수 정밀도로 측정하지만 양쪽 정렬이 제외하는 말미 공백은
정수 반올림하고, 여러 서식에 걸친 말미도 마지막 run의 서식 하나로 측정했다.
`layout/paragraph_layout.rs::justified_trailing_space_width`가 각 suffix run의 서식과
`estimate_text_width_exact`를 사용하도록 수정했다. tolerance 확대·glyph 축소·좌표 clamp나
V2 guard 해제는 하지 않았다. 기존 공백 분배 정책, 저장 LineSeg, 줄 끝 공백 자체를 보존한다.

실제 소비 경로:

- `estimate_line_run_widths`의 소수 자연 폭 → `compute_line_extra_spacing`의 말미 폭 제외와
  `extra_word_sp` → `emit_line_runs`의 같은 소수 run 측정/말미 분배 회수 → 최종 TextRun.
- `table_v2/text.rs::compose`는 이 최종 노드와 `painted_inline_ends`를 검사하고 같은 노드로
  `ParagraphItem::Lines`와 paint payload를 만든다. 이후 content/flow의 fit·컷·이어받기·SVG/WASM은
  기존 공유 결과를 소비한다. 원점/줄 높이/끝 컷을 새로 보정하는 분기는 추가하지 않았다.
- 수정은 공통 양쪽/나눔 정렬의 내부 공백 분배 경로에 적용된다. 재조판 soft-wrap은 원래
  공백 slot 계산을 유지하며, 마지막 줄·Right/Center·배분 정렬·별도 dash 종료 정책은 바꾸지 않는다.
  Legacy도 같은 helper를 소비하므로 기존 양쪽 정렬/줄 폭/재줄바꿈 회귀를 추가 검증한다.

독립 근거와 작은 검증 (`output/7353/r19/run-box/`):

- 정렬 불변식: 말미 공백을 제외한 첫 줄의 가시 끝은 저장 줄 오른쪽 경계와 일치해야 한다.
  합성 계약은 두 소수 글자 크기와 여러 suffix 서식을 검사한다. 원본 문단은 수정하지 않고
  23172HU 폭, text_start30, 내어쓰기2064HU를 그대로 최종 좌표/이어받기에서 검사한다.
- 이전 release-test lib에서 두 신규 검사 모두 실제 의도한 경계로 FAIL (`before-final.log`).
  실제 정상 한컴 저장 대조군도 이전 Native와 WASM에서 같은 run 경계로 거부된다
  (`fixture-before-final.log`, `before-wasm.json`). 빌드 실패는 재현 증거로 세지 않았다.
- 변경 후 text 계약25건, document-flow62건 PASS (`after.log`, `document-after-final.log`).
  원본 두 줄을 작은 예산으로 분할해 첫/다음 조각의 글줄 소속과 종료를 검사했다.
  새 문서 검사 작성 중 run 목록을 줄로 오인하고 page 전용 helper를 node에 호출한 오류는
  정정했다. 이를 제품 회귀나 수정 전 결함으로 보고하지 않는다.
- 원본 전체는 다음 `index5: TAC carrier paragraph constraints`에서 명시적으로 거부된다.
  이 다음 경계는 아직 미검증이며 R5/원본 전체 완료가 아니다. baseline·ignore 변경은 없다.

대조군 출처·해시·생성 명령·저장 기하와 첫 시도 보존은
`tests/fixtures/issue7353_justify_review/README.md`에 기록했다. 원본 글자·문단 서식을 가져온
독립1쪽이며 부모를 TAC로 작성해 한컴 정상 저장 후 **동일 HWP**에서 PDF를 얻었다.
첫 부유 표 대조군의 별도 앵커 차이는 `anchor-diagnostic/`에 보존했고 통과 증거로 사용하지 않는다.
그 입력의 부유 표와 뒤 본문 순서 차이는 미해결 진단으로 남는다. TAC 대조군의 통과가 그
앵커 경로를 해결하거나 검증한 의미는 아니다.

최종 시각 검증:

- Docker WASM 성공7분08초 (`docker-wasm.log`), SHA-256
  `043dba7e79491d09223479e906dc73e8e76c9038b1dbaabdb7ef32ffa694cbf8`.
- `bash output/7353/r19/run-box/finalize.sh`: debug Native와 fresh WASM의 SVG byte-identical,
  JSON 수치 차이15개/최대2.842170943040401e-14, 비수치 차이0. Native profile은 debug이며
  집중 회귀는 별도 release-test로 실행한다. source는 `d8c184085`+패치, `source.sha256`에 고정했다.
- 이전 승인 diagonal/저장 indent/fresh indent × 두 끝 간격 정책6조합에서 Native/WASM SVG
  보존 및 backend 일치를 확인했다 (`controls.log`).
- Native/fresh WASM review·standalone overlay·동일 영역 확대 review를 직접 열었다.
  “병당 ” 뒤 개행, “2.6%” 시작, 내어쓰기, 표 외곽·뒤 본문 위치를 확인했다. 글꼴 외형 차이는
  남는다. 0.055px 해소 자체는 육안 개선을 과장하지 않고 좌표 계약으로 판정한다.

메인테이너 판정 자료:

- [동일 영역 확대 review](../../output/7353/r19/run-box/review/wasm-focus-review.png)
- [fresh WASM 전체 review](../../output/7353/r19/run-box/review/wasm-review-1.png)
- [standalone overlay](../../output/7353/r19/run-box/review/wasm-overlay-1.png)
- [HWP 샘플](../../tests/fixtures/issue7353_justify_review/justify-saved.hwp)
- [한컴 기준 PDF](../../tests/fixtures/issue7353_justify_review/justify-2020.pdf)

메인테이너가 이번 절편의 시각 판정 통과와 다음 절편 진행을 승인했다. 전체 PR CI/lint·원격 게시·기본 엔진 전환은 수행하지 않았다.
최종 집중 회귀 결과는 아래에 기록한다.

최종 집중 release-test: **214 passed /0 failed /1956 skipped** (`run-box/focused-resume.log`).
review worktree에서 source/cases 해시 동일성을 확인한 뒤 다음 명령으로 검증했다:

```sh
CARGO_BUILD_JOBS=2 cargo nextest run --locked --cargo-profile release-test \
  --test regression_suite_002 --test regression_suite_003 --test regression_suite_004 \
  --test regression_suite_005 --test regression_suite_014 --test regression_suite_015 \
  --test regression_suite_017 --test regression_suite_018 --test regression_suite_027 \
  --test regression_suite_028 \
  -E 'test(issue_7353_table_v2) | test(issue_4755) | test(issue_6102) | test(issue_3128) | test(issue_7254) | test(issue_5679) | test(issue_4956)' \
  --no-fail-fast --target-dir /home/edward/mygithub/rhwp/target/pr-review
```

처음211건 실행은 review source 동기화와 경합해 신규3건을 포함하지 않아 최종 증거로 쓰지 않는다.
동기화 후 실행은 빌드 도중143(SIGTERM)로 중단되어 테스트 판정이 없었다. 메모리·디스크 여유와
OOM 기록 부재를 확인하고 캐시로 재개한 위214건 결과만 최종 결과다. nextest0.9.137/권고0.9.140 및
기존 설정 경고는 남는다. 최종 신규3건이 실제 실행 목록에 있음을 확인했다. source manifest3건,
fmt check와 diff check도 통과했다. 검증 뒤 제품 코드 변경은 없다.

### 다음 절편: 저장 TAC 줄의 실제 공백과 문단 안쪽 경계

메인테이너의 직전 시각 통과를 반영해 `362f31d61`로 checkpoint를 커밋했다.
이번 변경은 그 위의 작업 패치이며 legacy 기본 경로는 유지한다.

원본 #6923의 다음 거부 대상은 `s0/p5/t0/c0/p7`이다. 이는 빈 carrier가 아니라
offset0에 보통 공백 한 글자, offset1에 표 control이 있는 문단이다. 내어쓰기-4624HU가
있지만 첫 저장 줄에는 indentation flag가 없다. 공백을 삭제하거나 내어쓰기를 무조건
적용하는 대신 원본 소유 위치와 실제 글자 폭을 보존해야 한다.

구현과 소비 경로:

- `tac_spaces::compose`: 기존 글꼴/문자 위치 계산으로 ASCII 공백의 advance와 TextRun을
  함께 만든다. tab·NBSP·가시 문자·장식 공백은 이번 범위로 임의 수용하지 않는다.
- `tac::physical_frame` → `object_rows`: 저장 cs/sw의 문단 여백을 검증하고, bit20이 있는
  줄에만 indent를 반영한다. 공백과8unit control의 연속 source coverage, 줄 소속,
  바깥여백, 정렬을 같은 결과로 만든다. 공백 폭도 표 앞/사이/뒤 정렬 예산에 포함한다.
- `tac::compose` → `ir`/`document_input`: 같은 저장 줄 상자와 TextRun을
  `InlineTables.lines`에 바인딩한다. 공백의 no-ink 상자는 저장 줄 높이·baseline을 가진다.
- `FlowCursor`는 줄 전체 점유 높이가 예산에 맞고 자식 표가 모두 수용된 뒤에만 같은
  pen에서 표와 공백 줄을 방출한다. 요구 높이가 안 맞으면 전체 줄을 이월하며 이중 예약하지
  않는다. `text_ir`와 문서 paint는 이 소유 payload/배치를 그대로 소비한다.
- 혼합 공백의 non-final Justify 줄은 분배 결과가 없으므로 명시적으로 미지원이다.
  그림 carrier의 여백/indent는 별도 경로이므로 기존 거부 조건을 유지했다.

독립 근거/검증:

- `tests/cases/issue_7353_table_v2_document_flow.rs`에 body/nested × indentation flag
  유/무 × 같은/별도 저장 줄 × Left/Center/Right의24조합을 추가했다. 표 자체36px만 fit하고
  줄 점유40px는 fit하지 않는 경계에서 source 유닛 보존, 정확한 최종 좌표, 다음 문단을
  검사한다. source hole·가시 문자·tab·공백 포함 너비 초과·fresh·non-final Justify는 거부한다.
- 수정 전 lib에서 신규 positive 계약이 `TAC carrier paragraph constraints`로 FAIL
  (`tac-insets/test-before.log`). 최종 direct document-flow 계약은 **65 passed /0 failed**
  (`document-final.log`). 정상 한컴 저장 대조군도 이전 Native에서 같은 carrier 사유로
  거부됐다 (`before-fixture.log`). 빌드 실패를 결함 검출 증거로 세지 않는다.
- `tests/fixtures/issue7353_tac_space_review/README.md`에 생성 출처·변경점·입력/PDF 해시를
  기록했다. 원본 carrier 속성을 가져오되 읽을 수 있는1×1표와 평문 부모/뒤 본문으로
  독립 작성한 대조군을 한컴에서 정상 저장하고, **동일 저장 HWP**에서 PDF를 얻었다.
  저장 LineSeg를 수동 편집하지 않았다.
- PDF 부모/자식 왼쪽 선의 상대 간격은13.59375px, Native는13.573333px이다(96dpi).
  PDF 양자화 비교 예산은300dpi 프린터 한 dot이고, 원본 치수·공백/표 인접성은 별도 정확
  좌표 assertion으로 검사한다. 합성24조합 통과를 전부 한컴 시각 일치로 주장하지 않는다.

원본 전체는 이제 다음 자식의 `stored text requires intact single-segment rows`에서 거부된다.
해당 빈 셀의 가용 폭은1303−510−510=283HU지만 저장 줄 폭은1440HU다. 이는 별도 조사할
입력/저장 폭 경계이며 이번 대조군 통과로 원본 전체 통과나 R5 완료를 선언하지 않는다.
초기 대조군 작성 시 폭/페이지 장식/문자 baseline 제약이 발견된 입력은 output에 보존했고,
수용 조건을 완화하지 않았다. 정상 재작성 절차와 차이는 fixture README에 구분했다.

최종 검증 (`output/7353/r19/tac-insets/`):

- `focused-qualified.log`: **199 passed /0 failed /1532 skipped**. 최종 신규3건의 실행을
  확인했다. review worktree에서 동기화 완료 후 다음 명령을 실행했다.

  ```sh
  CARGO_BUILD_JOBS=2 cargo nextest run --locked --cargo-profile release-test \
    --test regression_suite_002 --test regression_suite_003 \
    --test regression_suite_004 --test regression_suite_005 \
    --test regression_suite_015 --test regression_suite_017 \
    --test regression_suite_027 --test regression_suite_028 \
    -E 'test(issue_7353_table_v2) | test(issue_4755) | test(issue_6102) | test(issue_3128)' \
    --no-fail-fast --target-dir /home/edward/mygithub/rhwp/target/pr-review
  ```

  이는 이번 집중 범위이며 전체 CI가 아니다. nextest 권고 버전/기존 설정 경고는 남는다.
- `docker compose --env-file .env.docker -p rhwp run --rm wasm`: **성공7분46초**
  (`docker-wasm-qualified.log`). fresh WASM SHA-256:
  `1bcac26bd60c47b93c4333261ca425d48e979b3dbcb82673b3b72d9534885f55`.
- final debug Native probe → `node output/7353/r19/tac-insets/review.mjs --wasm`
  → `focus.mjs native`, `focus.mjs wasm`: 동일 입력1쪽, SVG byte-identical,
  JSON 비수치 차이0, 수치 차이7개/최대2.2737367544323206e-13.
  `source.sha256`11개 source/test 해시와 `review/run.json`에 source checkpoint+패치를 고정했다.
- `node output/7353/r19/tac-insets/controls.mjs`: 기존 승인 diagonal/저장 indent/fresh
  indent/직전 justify × 끝 간격 정책2종 = **8조합**에서 기존 Native SVG 보존 및
  fresh WASM SVG 동일성 통과 (`controls.log`).
- 최종 Native/fresh WASM 전체 review·동일 영역 확대 review·standalone overlay를 직접
  열어 부모/자식 표 시작·외곽과 `AFTER CELL` 위치·보존을 확인했다. 한컴 쪽 선이 더 진하고
  글리프 외형 차이가 남는다. 픽셀 점수를 시각 통과 판정으로 대신하지 않았다.
- `cargo fmt --all -- --check`, `git diff --check`, source manifest 검증 통과.
  중간 코드 변경으로 중단한 빌드/검사는 최종 증거에 포함하지 않는다. 검증 뒤 제품 코드 변경 없음.

이번 절편 메인테이너 판정 요청:

- [같은 영역 확대 review](../../output/7353/r19/tac-insets/review/wasm-focus-review.png)
- [fresh WASM 전체 review](../../output/7353/r19/tac-insets/review/wasm-review-1.png)
- [standalone overlay](../../output/7353/r19/tac-insets/review/wasm-overlay-1.png)
- [한컴 저장 HWP 대조군](../../tests/fixtures/issue7353_tac_space_review/space-saved.hwp)
- [동일 HWP의 한컴 PDF](../../tests/fixtures/issue7353_tac_space_review/space-2020.pdf)

확인할 대상은 `부모 셀 여백 → 실제 공백1칸 → 자식 표 왼쪽`과 뒤 문단이다.
전체 PR lint/CI·원격 게시·기본 엔진 전환은 수행하지 않았으며, 이번 새 절편의 메인테이너
시각 판정은 대기한다.
메인테이너가 이번 TAC 공백/들여쓰기 절편의 시각 판정 통과와 다음 절편 진행을 승인했다.

### 후속 절편: 좁은 셀의 저장 최소 텍스트 줄 폭

직전 승인 범위를 `ce3f9a53b`로 커밋한 뒤 진행했다. 원본 #6923의
`s0/p5/t0/c0/p7/t0/c1/p0`은 빈 가운데 셀이다. 셀1303 HU−좌우 여백510 HU씩으로
물리 안쪽 폭283 HU지만 저장 줄 폭은1440 HU다. 기존 공통 구성기의
`composer.rs::cell_inner_text_width`가 같은 최소 폭을 적용한다. 주석의 과거 보고서만
근거로 수용을 넓히지 않고, 좁은 셀을 새로 작성해 한컴 정상 저장으로1440 HU를 재확인했다.
입력 생성·동일 저장 HWP의 기준 PDF·해시는
`tests/fixtures/issue7353_narrow_cell_review/README.md`에 기록했다.

구현 주장과 실제 소비 경로:

| 경계 | 생산·소비와 불변식 |
| --- | --- |
| 공통 최소 폭 | `stored_text.rs::cell_lane_width`가 기존 공통 최소 폭과 정상 저장 줄 정보를 대조한다. 관측 sw의 최댓값으로 임의 확대하지 않는다. |
| IR → 측정 | `ir.rs::bind_table`이 공통 폭으로 문단을 구성하고 `CellTrack.text_width`에 기록한다. 물리 `FlowCellInput.width`와 padding은 유지한다. |
| 콘텐츠 검증 | `content.rs::from_grid_rows`의 Lines만 저장 텍스트 폭으로 검사한다. InlineTables·자식 표의 가용 폭은 물리 폭이다. |
| 컷 → 실제 배치 | `fragment.rs::fit_rows`가 물리 셀 원점/여백을 넘기고 `flow.rs`의 Lines 분기가 이미 구성한 줄 높이와 순서로 수용/이월한다. `text.rs::TextPaint`가 그 위치로 같은 payload를 이동한다. 셀 외곽 폭은 grid에서 그대로 가져온다. |
| 적용 제한 | 저장된 평문 셀만 확장한다. 혼합 fresh/컨트롤 셀은 거부하고, 명시 Flow 입력/본문 경로는 이 IR 증거 없이 폭을 확장하지 않는다. fresh 최소 폭 재조판은 이번 검증 범위 밖이다. |

독립 기대값은 정상 저장1440 HU와 원문 셀1303 HU·여백510 HU에서 정한다. 합성 경계는
900 HU 줄 높이·450 HU 간격·위75/아래150 HU 여백을 독립 입력으로 주고 최종 좌표를
검사한다. 빈 줄은 높이를 가진 한 유닛이며, 마지막 문단 간격 정책 아래 총51px를 예약한다.
20px 예산 세 쪽의 시작/끝 컷에서 A·빈 줄·B를 한 번씩 소비하고 종료하는지 검사한다.
여기서는 rowspan·헤더·캡션·각주 경로를 수정하지 않았다.

수정 전 신규 텍스트 계약은 저장 폭 거부로 FAIL했다 (`test-before.log`). 최종 한컴A
대조군도 이전 Native에서 같은 원인으로 거부됐다 (`before-final.log`). 수정 후 직접
텍스트 계약26건은 PASS (`text-after.log`). 빌드 실패는 결함 검출 증거에 포함하지 않는다.
원본 전체는 이후 `nested anchor, TAC, wrap or outer margin`에서 멈춘다
(`original-after.log`). 이를 R5 완료나 원본 전체 통과로 바꾸어 보고하지 않는다.

초기 AB 대조군은 양쪽 엔진 모두B가 인접 셀에 가려지는 별도 현상이 있었다. 원본 산출을
보존한 채 가운데 글자만A로 바꾸어 한컴에서 다시 저장했다. 이번 시각 증적은 그 최종A
저장본/PDF를 사용한다. 기존 자료를 최종 코드의 시각 증거로 재사용하지 않는다.

최종 산출 위치는 `output/7353/r19/narrow-cell/`이다. checkpoint `ce3f9a53b`와
`source.sha256`의6개 source/test 패치를 기준으로 검증하며 `review/run.json`에 입력·PDF·
WASM 해시를 고정했다. 파생 fixture의 생성 코드는 README의 별도 절차다.

- review worktree에서 직전 절편에 기록한8개 suite·동일 nextest 필터/명령을 최종 source로
  실행: **201 passed /0 failed /1532 skipped** (`focused.log`). 신규 저장 최소 폭·정상
  한컴 대조군2건의 실행을 확인했다. 집중 범위이며 전체 CI가 아니다. nextest 권고 버전과
  기존 설정 경고는 남는다.
- `docker compose --env-file .env.docker -p rhwp run --rm wasm`: 성공7분34초
  (`docker-wasm.log`). fresh WASM SHA-256:
  `aa9a455ba588568f9ecb229912143e0f615e1b35290980c5f58530384c25e20b`.
- 최종 Native probe, `pdftocairo -png -r 96 -singlefile`, `review.mjs --wasm`,
  `focus.mjs native`, `focus.mjs wasm`: 동일 입력1쪽, Native/fresh WASM SVG 동일.
  JSON 비수치 차이0, 수치22개/최대2.2737367544323206e-13.
- `controls.mjs`: 기존 승인 diagonal/저장 indent/fresh indent/justify/TAC 공백
  × 끝 간격 정책2종 **10조합**에서 기존 Native SVG 보존과 fresh WASM 동일성 통과.
- Native/fresh WASM 전체 review·같은 영역 확대 review·standalone overlay를 직접 열어
  A·좁은 셀/이웃 셀 경계·부모 외곽·뒤 문단을 비교했다. 한컴 선이 더 진하고 글리프
  외형 차이는 남는다. 픽셀 점수나 백엔드 동일성으로 메인테이너 시각 판정을 대신하지 않는다.
- `cargo fmt --all -- --check`, `git diff --check`, source manifest 검증과 review worktree의
 6개 변경 source/test 동일성 확인 통과. 검증 후 제품 코드 변경 없음.

이번 절편 시각 판정 대상:

- [가운데A·좁은 셀 경계 확대](../../output/7353/r19/narrow-cell/review/wasm-focus-review.png)
- [전체 WASM review 및 뒤 문단](../../output/7353/r19/narrow-cell/review/wasm-review-1.png)
- [standalone overlay](../../output/7353/r19/narrow-cell/review/wasm-overlay-1.png)
- [한컴 저장 HWP](../../tests/fixtures/issue7353_narrow_cell_review/narrow-saved.hwp)
- [동일 저장 HWP의 한컴 PDF](../../tests/fixtures/issue7353_narrow_cell_review/narrow-2020.pdf)

메인테이너 시각 판정 대기. 전체 PR lint/CI, 원격 게시, 기본 엔진 전환은 수행하지 않았다.
메인테이너가 좁은 셀 절편의 시각 판정 통과와 다음 절편 진행을 승인했다.

### 후속 절편: 셀 내부 자리차지 표의 폭0 소유 줄과 후속 빈 문단

직전 승인 범위를 `3a264c57e`로 커밋했다. 원본 #6923의 다음 미지원 대상은
`s0/p5/t0/c0/p26/t0`이다. 빈 소유 문단에 폭0/양수 높이의 저장 줄과, 문단 기준
자리차지 표가 있다. 표의 가로offset1980 HU·바깥여백141 HU씩·세로offset0을 보존해야 한다.
소유 줄을 지우거나 자식 표와 세로로 합산하지 않고 같은 원점의 점유로 처리한다.
그 뒤 편집자가 만든 빈 문단은 독립된 줄이다.

원본 속성을 복사한 읽을 수 있는 대조군을 HWPX로 생성하고 한컴2020에서 정상 저장했다.
동일 저장 HWP의 PDF를 사용하며 생성·작업 ID·해시는
`tests/fixtures/issue7353_cell_anchor_review/README.md`에 연결했다. 저장 정보의 독립 기대값은
소유 줄 높이1500/간격644 HU, 자식 높이5000 HU, 후속 빈 문단vpos5282 HU,
`AFTER ANCHOR` vpos7426 HU다. 저장 LineSeg나 PDF 좌표를 수동 수정하지 않았다.

| 실제 호출 경로 | 생산 결과와 소비 계약 |
| --- | --- |
| `cell_anchor.rs::compose` → `text_ir.rs` | 유효한 폭0 저장 소유 줄과 자식 위치를 `ParagraphItem::ExcludedTable`로 구성한다. 동일 구성에서 소유 줄 paint payload도 만든다. |
| `ir.rs::bind_table` → `content.rs` | 실제 자식 plan과 좌우 가용 폭을 검사한 `FlowBlock::AnchoredTable`로 바꾼다. 높이는 소유 줄과 자식/여백의 점유 합집합이며 다음 원점에는 소유 줄 간격도 반영한다. |
| `fragment.rs::fit_rows` → `flow.rs::FlowCursor::fit` | 셀의 실제 페이지 예산에서 소유 줄과 첫 자식 조각을 함께 수용/이월한다. 시작/끝 내용 컷은 자식 cursor가 소유한다. 첫 조각에서 소유 줄은 한 번만 출력하며 누적 예약은 수용된 조각의 실제 높이다. |
| 이어받기 → 실제 배치 | 자식 continuation은 이미 소비한 내용을 재시작하지 않는다. 내용이 끝나고 남은 물리 아래 여백은 별도 tail로 이월한다. `TextPaint`는 같은 소유 줄/자식 순서와 최종 배치 좌표를 소비한다. 뒤의 빈 문단과 후속 본문은 유지한다. |

저장 경로의 빈 소유 줄·문단 기준·세로offset0·자리차지 자식 하나만 이번 수용 범위다.
fresh 재조판·가시 텍스트 혼재·어울림·여러 앵커·비영 세로offset은 확장하지 않았다.
일반 빈 문단/TAC 경로는 이 소유 줄 규칙을 적용하지 않는다. rowspan·반복 헤더·캡션·각주
처리는 수정하지 않았다. 조각마다 위 여백을 예약하는 분할 경로는 아래 합성 계약으로만
검증했으며, 해당 다쪽 배치의 한컴 시각 일치는 미검증이다.

정식 `tests/cases/issue_7353_table_v2_document_flow.rs`에 정상 저장본 최종 좌표 검사와
작은 예산의 분할 계약2건을 추가했다. 합성 계약은 복제 IR의 분할 정책·여백을 변경한 것임을
명시한다.20px에서는 소유 줄만 fit하더라도 자식 첫 조각이 안 맞아 전체 보류하고,
30/40/50px에서는 호스트·자식·후속 빈 줄/본문의 무누락·무중복·예약/경계·종료를 검사한다.
가시 텍스트, 어울림, 잘못된 저장 폭, 여백 불일치는 거부하는 대조군이다.

수정 전 새 계약은 `nested anchor, TAC, wrap or outer margin`으로 FAIL
(`output/7353/r19/next-anchor/test-before.log`), 수정 후 문서 경로68건은 PASS
(`test-after.log`)다. 빌드/테스트 작성 중 오류는 결함 재현 증거로 세지 않는다.
원본 전체는 다음 `rowspan height needs redistribution`에서 명시적으로 멈춘다.
전체 원본의 조판이나 R5 완료를 주장하지 않는다.

검증 산출은 `output/7353/r19/next-anchor/`에 모았다. checkpoint `3a264c57e`와
11개 변경 Rust source/test의 `source.sha256`로 검증 코드를 고정한다.
초기 `focused.log`는 source 동기화 전의201건 실행이므로 이번 최종 검증에서 제외한다.
최종 동기화 후 실행은 `focused-final.log`다.

- review worktree에서 `CARGO_BUILD_JOBS=2 cargo nextest run --locked --cargo-profile
  release-test --test regression_suite_002 --test regression_suite_003 --test
  regression_suite_004 --test regression_suite_005 --test regression_suite_015 --test
  regression_suite_017 --test regression_suite_027 --test regression_suite_028 -E
  'test(issue_7353_table_v2) | test(issue_4755) | test(issue_6102) | test(issue_3128)'
  --no-fail-fast --target-dir /home/edward/mygithub/rhwp/target/pr-review`:
  **203 passed /0 failed /1532 skipped**. 신규2건이 실제 실행됐다. nextest 권고 버전과
  기존 설정 경고는 남는다. 전체 CI가 아닌 집중 범위다.
- `docker compose --env-file .env.docker -p rhwp run --rm wasm`: **성공7분41초**.
  fresh WASM SHA-256: `6465386ae0195698fa1e2146d456760a7f6fee6e167c66ff524f92f4cca83c4a`.
- Native probe와 `pdftocairo -png -r 96 -singlefile`로 동일 저장 HWP/PDF를 산출한 뒤
  `review.mjs --wasm`, `focus.mjs native`, `focus.mjs wasm`을 실행했다.
  양쪽1쪽/SVG 동일, JSON 비수치 차이0, 수치12개/최대2.2737367544323206e-13.
  source·입력·PDF·WASM 해시는 `review/run.json`으로 연결했다.
- `controls.mjs`: 기존 승인 diagonal/저장 indent/fresh indent/justify/TAC 공백/좁은 셀
  × 끝 간격 정책2종 **12조합**의 Native SVG 보존과 fresh WASM 동일성 통과.
- Native/fresh WASM 같은 영역 확대와 전체 review·standalone overlay를 직접 열어
  자식 표 위치, 의도된 빈 한 줄, `AFTER ANCHOR`, 부모 외곽과 `AFTER CELL`을 비교했다.
  한컴 선이 더 진하고 글리프 외형 차이는 남는다. 백엔드 동일성이나 픽셀 점수를
  메인테이너 시각 판정으로 대신하지 않는다.
- `cargo fmt --all -- --check`, `git diff --check`, source manifest 검사와 검증용
  worktree의 최종 source/test 동일성 확인 통과. 검증 뒤 제품 코드 변경 없음.

이번 절편 시각 판정 대상:

- [자리차지 표·후속 빈 줄 확대 비교](../../output/7353/r19/next-anchor/review/wasm-focus-review.png)
- [fresh WASM 전체 review](../../output/7353/r19/next-anchor/review/wasm-review-1.png)
- [standalone overlay](../../output/7353/r19/next-anchor/review/wasm-overlay-1.png)
- [한컴 정상 저장 HWP](../../tests/fixtures/issue7353_cell_anchor_review/anchor-saved.hwp)
- [동일 HWP의 한컴 PDF](../../tests/fixtures/issue7353_cell_anchor_review/anchor-2020.pdf)

메인테이너 시각 판정 대기. 전체 PR lint/CI·원격 게시·기본 엔진 전환은 수행하지 않았다.
메인테이너가 셀 내부 자리차지 표·후속 빈 문단 절편의 시각 판정 통과와 다음 절편 진행을 승인했다.

### 후속 절편: 병합 셀 최소 높이의 부동소수점 오판 제거

직전 승인 범위를 `893e8bc99`로 커밋했다. #6923의 다음 실패는 동일 자식
`s0/p5/t0/c0/p26/t0`의 `rowspan height needs redistribution`이었다.
최초에는 선언 행 높이1382 HU를 기준으로 재분배가 필요하다고 추정했으나, 실제 실행에서
각 행은 저장 줄1200 HU+위·아래 여백141 HU씩으로1482 HU였다. 첫3행의 합4446 HU와
`구 분` 병합 셀 최소4446 HU는 같은 높이다. 재분배 필요라는 진단을 철회한다.

`content.rs::TableContentPlan::from_grid_rows`에서 실제 합은59.27999999999999403px,
최소 높이는59.28000000000000114px였다. 최소 높이 비교에만 합산 항 수와
`f64::EPSILON`에 비례하는 연산 오차 범위를 적용했다. 행 높이·내용 높이·배치 좌표는
변경하지 않으며, 물리 내용의 초과와 페이지 예산 비교는 여전히 엄격하다.
실제 부족분을 행 축소나 임의 배분으로 통과시키는 구현이 아니다.

| 실제 호출 경로 | 이번 변경과 경계 검증 |
| --- | --- |
| 저장 셀·줄 메트릭 → `text_ir.rs`/`ir.rs` → `content.rs::from_grid_rows` | 기존 구성 결과로 비병합 행의 높이를 먼저 확정한다. 병합 셀 선언 최소와 그 행 합의 수치 비교만 변경한다. |
| `fragment.rs::fit_rows` → `fragment/row_groups.rs::fit_row_groups` | 확정된 행 높이로 연결된 병합 그룹의 요구 높이와 누적 예약을 계산한다.4445 HU에서는 이월하고4446 HU에서는 첫 그룹을 수용한다. 컷·내용/물리 밴드·padding 계산은 변경하지 않는다. |
| `TextFragmentPlan::append_to` → `TextPaint::build_node` | 같은 placement의 실제 셀 y/height와 표 높이를 검사한다. 각 셀 소유가 한 번씩 보존되고 마지막 조각 뒤 Complete가 되는지 확인한다. |
| 다른 경로 | 실제 최소 높이1/100/10000 HU 추가는 기존 오류로 거부한다. 셀 내부 줄 분할·반복 헤더·캡션/각주 정책은 수정하지 않았다. |

새 정식 계약은 `tests/cases/issue_7353_rowspan_roundoff.rs`다. 승인 head 라이브러리에서는
의도한 최소 높이 오류로 FAIL(`test-before.log`), 수정 후 PASS(`test-after.log`)를 확인했다.
원본을 수정하지 않고 읽으며 최종 셀 경계·예약12139 HU·모든 셀 보존과 종료를 검사한다.
원본 전체의 admission 검사는 다음 `nested anchor, TAC, wrap or outer margin`을 보고한다.
이는 원본 전체 성공이 아니라 다음 미지원 경계로 진행한 것이다.

독립 시각 대조군은 원본 자식 표를 TAC 단독 표로 분리하고 뒤 문단을 추가한 정상 한컴
저장본이다. 생성 변경점·한컴 HWP/PDF job·해시는
`tests/fixtures/issue7353_rowspan_roundoff_review/README.md`에 기록했다.
Native에서 병합 셀·행 경계·표 외곽·뒤 문단을 직접 확인했다. 선 농도와 글리프 외형 차이는
남는다. 검증 코드는 checkpoint `893e8bc99`+`output/7353/r19/rowspan-height/source.sha256`다.
fresh WASM·집중 회귀 검증 결과는 아래에 이어 기록한다.

최종 검증 산출: `output/7353/r19/rowspan-height/`.

- review worktree에 최종 source/test를 먼저 동기화하고
  `node scripts/rust-test-suite-manifest.mjs --prepare`로 파생 suite를 갱신했다.
  `rg -l 'issue_7353_table_v2|issue_7353_rowspan_roundoff|issue_4755|issue_6102|issue_3128'
  tests/generated/regression_suite_*.rs`로 해당15개 target을 선택했다. 직전 절편의8개
  target 목록을 재사용하지 않았다. 파생 파일은 source PR 대상이 아니다.
- `CARGO_BUILD_JOBS=2 cargo nextest run --locked --cargo-profile release-test
  <위15개 --test target> -E 'test(issue_7353_table_v2) | test(issue_7353_rowspan_roundoff) |
  test(issue_4755) | test(issue_6102) | test(issue_3128)' --no-fail-fast
  --target-dir /home/edward/mygithub/rhwp/target/pr-review`:
  **269 passed /0 failed /3041 skipped**. 새 연산 오차 계약도 실제 실행됐다.
  `focused.log`에 빌드9분07초와 실행 결과를 보존했다. 기존 nextest 버전/설정 경고가 있다.
- `docker compose --env-file .env.docker -p rhwp run --rm wasm`: **성공7분49초**.
  fresh WASM SHA-256: `139198e352972c76cfa74a65c23834f74c7e09ade73d8f1f6a4d659b57343c8d`.
- `review.mjs --wasm`, `focus.mjs native`, `focus.mjs wasm`: Native/fresh WASM 모두1쪽,
  SVG·RenderTree JSON 동일(수치 차이0). `review/run.json`에 입력·PDF·WASM·source 해시를 연결했다.
- `controls.mjs`: diagonal/저장 indent/fresh indent/justify/TAC 공백/좁은 셀/셀 앵커
  × 끝 간격 정책2종 **14조합**에서 승인 Native SVG 보존과 fresh WASM 동일성을 확인했다.
- 같은 페이지·영역의 Native/fresh WASM 확대·전체 review와 standalone overlay를 직접 열어
  병합 셀·행 경계·표 외곽·뒤 문단과 페이지 하단을 확인했다. 대각선2개도 출력에 존재한다.
  한컴과 선 농도·글리프 외형 차이는 남으며, 내용 픽셀 자동 보조 일치율은42.2193%,
  전체 픽셀 일치율은97.6851%다. 수치를 사람의 판정 정확도나 통과로 해석하지 않는다.
- `cargo fmt --all -- --check`, `git diff --check`,3개 Rust source/test 해시 검사 통과.
  제품 source 트리 전체가 review worktree와 동일함도 확인했다. 검증 뒤 제품 코드 변경 없음.

시각 판정 대상은 **분리 대조군1쪽**이다. 원본 전체의 앵커와 다쪽 조판은 미완료다.

- [병합 셀·행 경계 확대 review](../../output/7353/r19/rowspan-height/review/wasm-focus-review.png)
- [전체 compare](../../output/7353/r19/rowspan-height/review/wasm-compare-1.png)
- [standalone overlay](../../output/7353/r19/rowspan-height/review/wasm-overlay-1.png)
- [전체 review](../../output/7353/r19/rowspan-height/review/wasm-review-1.png)
- [한컴 정상 저장 HWP](../../tests/fixtures/issue7353_rowspan_roundoff_review/rowspan-saved.hwp)
- [동일 HWP의 한컴 PDF](../../tests/fixtures/issue7353_rowspan_roundoff_review/rowspan-2020.pdf)

메인테이너 시각 판정 대기. 전체 PR lint/CI·원격 게시·기본 엔진 전환은 수행하지 않았다.
메인테이너가 병합 셀 높이 연산 오차 절편의 시각 판정 통과와 다음 절편 진행을 승인했다.

### 후속 절편 — 자리차지 표 뒤의 저장 호스트 줄 보존

직전 승인 절편은 `96532f132`로 커밋했다. 이번 대상은 #6923 원본의
`s0/p5/t0/c0/p37/t0`다.67개 공백 문자가 있는 문단에 Para/Top, 세로0,
TopAndBottom 표가 연결되어 있다. 원본을 수정하지 않았다.

독립 근거: 앞 문단의 다음 원점22540HU에 자식 높이15107과 위/아래283HU를
더하면38213HU로, 저장 호스트 LineSeg vpos와 정확히 같다. 호스트 높이1400,
줄간격-280(80%)을 적용하면 다음 빈 문단39333HU, 그 뒤 문단40533HU다.
즉 표가 줄을 대체하거나 공백이 사라지는 것이 아니라 **표 제외 영역 뒤에 실제 줄이 이어진다**.

원본 속성을 분리한 두 HWPX를 한컴에서 정상 HWP로 저장하고 그 HWP로 PDF를 만들었다.
한 대조군은 공백 문자를 보존하고, 다른 대조군은 같은 속성의 `HOST TEXT`를 사용한다.
자식 표 내용은1x1 표로 단순화했다. 정상 생성본의 저장 좌표와 PDF 모두 같은 배치 순서를
확인했다. 생성 절차·변경점·job·해시는
`tests/fixtures/issue7353_following_anchor_review/README.md`에 있다.
이는 원본 전체 일치의 증거가 아니다.

| 실제 호출 경로 | 규칙·소비 결과 |
| --- | --- |
| `cell_anchor.rs::compose_following` → `text_ir.rs::compose_items` | Para/Top 세로0 자리차지 표의 x·바깥여백을 생성하고, 저장 호스트는 기존 `TextComposer`가 줄 구성·signed gap을 보존한다. 텍스트 존재를 줄 높이로 대체하지 않는다. |
| `ir.rs::bind_table` → `FlowBlock::AnchoredTable` | 자식의 실제 plan과 margin을 바인딩한다. 이번 경로는 공유 원점의 가짜 줄을 만들지 않고 `host: None`; 실제 호스트 Lines가 뒤따른다. 기존 폭0 exclusion은 `Some(host)`로 유지된다. |
| `content.rs` → `flow.rs::FlowCursor` | 측정도 fit도 같은 child plan과 top/bottom을 소비한다. 첫 child가 안 맞으면 top만 먼저 소비하지 않는다. 자식 내용 컷·남은 선언 높이·tail은 기존 재개 상태를 공유한다. tail 소진 뒤에만 호스트 줄로 이동한다. |
| `TextFragment::append_to` → `TextPaint` | 확정 placement를 사용한다. 뒤에서 child 원점을 다시 선택하거나 줄을 숨기는 보정은 없다. 최종 부모/자식 외곽·줄 원점·뒤 본문 위치를 회귀 계약으로 검사한다. |

정식 계약2건을 `tests/cases/issue_7353_table_v2_document_flow.rs`에 추가했다.
직전 승인 라이브러리에서는 둘 다 기존 미지원 앵커 오류로 FAIL, 변경 후 PASS다.
정상 저장본은 한컴에서 관측한6766/7886/9086HU 줄 원점과10568HU 부모 외곽을 검사한다.
별도의 합성 분할 계약은 top alignment/CellBreak/패딩0으로 바꾼 입력에서20/30/40px 예산과
전체 배치를 대조한다. 내용 순서·무중복, 자식 물리 높이5000HU의 완전 소비, 실제 줄/외곽의
페이지 영역 준수, 첫 child를 못 넣을 때 top-only 소비 없음, 마지막 Complete를 확인한다.
이 분할 입력을 한컴 정상 페이지네이션 증거로 주장하지 않는다.

다른 세로 앵커·어울림·불일치 margin은 계속 거부한다. 여러 저장 줄, 완전히 빈 문자열의
positive-width control carrier는 독립 근거 미확보로 미검증/미지원이다.
저장 줄이 없는 기존 fresh 경로는 수정하지 않았으며, 이번 확장의 fresh 재조판은 미완료다.
원본 전체는 다음 `text preview stored rows or controls`(자식 문단의 탭 등)로 진행했다.
전체 admission 계약은 이 다음 미지원 경계를 기록하도록 바꿨으며 성공이나 fallback으로
바꾸지 않았다. rowspan·반복 헤더·캡션·각주 알고리즘은 이번에 변경하지 않았다.

검증 산출: `output/7353/r19/next-anchor2/`.
코드 기준은 `96532f132`+`source.sha256`의 working patch다.
Native 문서 흐름70건 통과. 두 대조군의 Native compare/overlay/review를 직접 열어
표 위치·외곽·호스트 뒤 빈 줄과 AFTER HOST/CELL을 확인했다. 선 농도·글리프 외형 차이는 남는다.
집중 release-test 회귀와 fresh Docker WASM 검증 결과는 아래에 이어 기록한다.

최종 결과:

- review worktree의 실제 source/test 동기화 후 파생 suite 준비.
  `cargo nextest run --locked --cargo-profile release-test <해당 --test targets>
  -E 'test(issue_7353_table_v2) | test(issue_7353_rowspan_roundoff) | test(issue_4755) |
  test(issue_6102) | test(issue_3128)' --no-fail-fast
  --target-dir /home/edward/mygithub/rhwp/target/pr-review`: **271 passed /0 failed**,
  3057 skipped. 현재 nextest 권장 버전/설정 경고는 기존과 같다. `focused.log` 참조.
- 새 계약은 수정 전2FAIL(`test-before.log`), 수정 후 문서 흐름70PASS
  (`test-after.log`). 정식 집중 suite에서도 두 계약을 실행했다.
- `docker compose --env-file .env.docker -p rhwp run --rm wasm`: **성공7분50초**.
  WASM SHA-256: `893ba1d048bc87f5c1da1309bc95904990d001adefa09acd5e33a411430e741b`.
- `review.mjs --wasm`, `visible.mjs --wasm`: 두 정상 저장 대조군 각1쪽에서
  Native/fresh WASM SVG·RenderTree 동일, 수치 차이0. 각 `run.json`과
  `backend-comparison.json`에 source/input/PDF/WASM 연결을 기록했다.
- `controls.mjs`: 기존 diagonal/저장 indent/fresh indent/justify/TAC 공백/좁은 셀/
  폭0 셀 앵커 × 끝 간격 정책2종, **14조합**의 승인 SVG 보존 및 Native/WASM 동일성 통과.
- `focus.mjs native`, `focus.mjs wasm`(review/visible 각각) 생성 후 확대 review와
  standalone overlay를 직접 확인했다. 표·호스트·뒤 문단 원점은 맞으며 기존 선 농도·글리프
  외형 차이는 남는다. 위치 보정 없이 같은 영역을 비교했다. 자동 점수를 시각 통과로 해석하지 않는다.
- `cargo fmt --all -- --check`, `git diff --check`, source/test SHA 검사 통과.
  검증 이후 제품 코드 변경 없음. 전체 PR lint/CI·push·기본 엔진 전환은 수행하지 않았다.

메인테이너 판정 대상은 아래 두 **분리 대조군1쪽**이다. 원본 전체 다쪽 조판은 미완료다.

- [공백 호스트 확대 review](../../output/7353/r19/next-anchor2/review/wasm-focus-review.png)
- [실제 글자 호스트 확대 review](../../output/7353/r19/next-anchor2/visible/wasm-focus-review.png)
- [공백 호스트 전체 compare](../../output/7353/r19/next-anchor2/review/wasm-compare-1.png)
- [공백 호스트 standalone overlay](../../output/7353/r19/next-anchor2/review/wasm-overlay-1.png)
- [실제 글자 호스트 standalone overlay](../../output/7353/r19/next-anchor2/visible/wasm-overlay-1.png)
- [정상 한컴 저장 HWP·PDF 및 생성 기록](../../tests/fixtures/issue7353_following_anchor_review/README.md)

메인테이너 시각 판정 대기. 다음 원본 차단점은 자식 표 내부의 탭/저장 텍스트 처리다.
메인테이너가 두 대조군의 시각 판정 통과와 다음 절편 진행을 승인했다.

### 후속 절편 — 단일 저장 LEFT 탭의 공통 텍스트 재생

직전 승인 절편은 `dd33300ab`로 커밋했다. 이번 원본 대상은 #6923의
`s0/p5/t0/c0/p37/t0/c0/p0`, ` 창원공장\t  ` 문단이다. 원본은 변경하지 않았다.
LineSeg 폭22220HU, 높이1200, baseline1020, gap720이며 탭 확장에
폭2924HU/type0x0100(LEFT, leader없음)이 저장되어 있다. char_offsets는
`[0,1,2,3,4,5,13,14]`로 탭의8-unit 소유 범위를 보존한다.

위반 경계는 `TextComposer::compose`와 `stored_text::localize`의 제어문자 일괄 거부다.
공통 문단 엔진에는 저장 LEFT 폭을 쓰는 경로가 이미 있다. 새 tab 폭 계산기나 문자 대체를
추가하지 않고 검증된 입력만 그 경로로 허용했다. 기본 Legacy·편집 재조판은 변경하지 않았다.

독립 근거 확보 과정에서 원본 자식 표 전체를 분리해 정상 저장한 HWP는
겹치는 zone 장식 미지원으로 거부됐다(`output/7353/r19/tabs/full-table/`). 이를 숨기거나
이번에 장식 우선순위를 추측하지 않았다. 원본 탭 문단과 눈으로 간격을 확인할 수 있는
`LEFT\tRIGHT` 문단을2행1열 표로 분리하고 한컴에서 정상 HWP 저장한 뒤 그 HWP로 PDF를
생성했다. 첫 탭2924HU는 보존됐고 두 번째는 한컴이1308HU로 저장했다.
원본 reserved extension words는0, 정상 저장본은32이므로 이를 의미 속성으로 제한하지 않았다.
생성 변경점·job·해시는 `tests/fixtures/issue7353_stored_tab_review/README.md`에 기록했다.
이 대조군은 원본 전체 조판이나 일반 페이지네이션의 한컴 일치를 입증하지 않는다.

| 값의 실제 호출 경로 | 소비와 검사 |
| --- | --- |
| 입력 tab_extended·char_offsets → `stored_text::validate_tabs/localize` | intact 저장 줄1개, LEFT 탭1개, 양수 low-word 폭, leader없음,96dpi만 허용. 줄 경계·원래 메트릭을 보존한다. |
| `compose_paragraph` → `layout_composed_paragraph_in_frame` → `paragraph_layout::emit_line_runs` | 같은 저장 확장을 `TextStyle.inline_tabs`로 전달한다. `text_measurement::compute_char_positions_walk`가 실제 탭 advance를 소비한다. |
| `stored_text::validate_paint` → `ParagraphItem::Text` → `FlowCursor` | 공통 엔진의 최종 줄 상자·다음 원점으로 높이를 결정한다. 저장 줄 소속·원점·baseline을 다시 검사하며 별도 탭 기반 높이를 추측하지 않는다. |
| `TextPaint` → SVG/fresh WASM | 확정 run과 좌표를 재생한다. 실제 SVG의 L→R 위치 차이에서 LEFT의 폰트 advance를 뺀 값이1308/75px인지 검사한다. 표 외곽6000/75px·줄 중심·뒤 AFTER TABLE도 검사한다. |

반례에서 공통 엔진의 한 문단 여러 저장 줄에 다른 탭 폭을 넣으면 뒤 줄이 첫 확장 폭을
다시 소비함을 확인했다(두 번째2250HU 대신1500HU). 일반 지원으로 확장하지 않고
다중 줄·다중 탭을 명시적 미지원으로 남겼다. RIGHT/CENTER/DECIMAL, leader,
placeholder, high-word 폭, non96dpi, fresh/recomputed 탭도 미검증/미지원이다.
이는 잘못된 배치를 허용하는 높이 보정이 아니라 V2 preview admission의 한계다.

정식 `tests/cases/`에4개 계약을 추가했다. 정상 저장본의 실제 SVG/외곽/뒤 본문,
합성 두 문단의 서로 다른 탭 폭과13px 분할 예산에서의 줄/유닛 보존·종료,
탭 앞뒤 텍스트·공백·글자모양 run 경계, 미지원 입력 거부를 검사한다.
분할/rowspan/반복 헤더/캡션/각주 알고리즘은 수정하지 않았으며 기존 줄 단위 fit과
동일 payload의 continuation을 소비한다. 새 합성 분할 사례는 한컴 출력 증거와 구분한다.
새 정상 저장본 계약은 수정 전 라이브러리에서 `text preview stored rows or controls`로
FAIL(`test-before.log`), 수정 후 PASS다. 원본 전체 admission은 다음
`stored TAC carrier requires unambiguous intact rows`로 이동했고 계속 명시적으로 거부한다.

산출은 `output/7353/r19/tabs/`, source는 `dd33300ab`+`source.sha256`의 working patch다.
Native 문서 흐름71건, 텍스트29건 통과. 같은 영역의 Native compare·standalone overlay·
확대 review를 직접 확인했다. 탭 간격·셀/표 외곽·뒤 문단은 보존되며 선 농도·글리프 외형
차이는 남는다. 집중 회귀/fresh Docker WASM 결과와 판정 링크는 아래에 이어 기록한다.

최종 검증:

- review worktree에 이번 source/test/fixture만 동기화하고 `node
  scripts/rust-test-suite-manifest.mjs --prepare` 실행. `cargo nextest run --locked
  --cargo-profile release-test <선택된14개 --test targets> -E
  'test(issue_7353_table_v2) | test(issue_7353_rowspan_roundoff) | test(issue_4755) |
  test(issue_6102) | test(issue_3128)' --no-fail-fast --target-dir
  /home/edward/mygithub/rhwp/target/pr-review`: **275 passed /0 failed**,
  2757 skipped, 빌드8분53초. `focused.log`/`prepare.log` 참조.
- `docker compose --env-file .env.docker -p rhwp run --rm wasm`: **성공7분43초**.
  WASM SHA-256 `5f0cda59a5076af84c98bd723cf23c5be427bf6a0fe99cb14190d40737319dfe`.
- `review.mjs --wasm`: 정상 저장 HWP의1쪽 Native/fresh WASM SVG·RenderTree 동일,
  수치 차이0. `review/run.json`과`backend-comparison.json`에 입력/PDF/source/WASM 연결.
- `focus.mjs wasm`: 첫 브라우저 시작 오류 뒤 캡처만 재시도해 성공했다. 코드/빌드 재실행이나
  과거 캡처 재사용은 없었다. Native와fresh WASM의 확대 review·standalone overlay를 직접 열어
  제목과 LEFT/RIGHT 간격, 셀 경계, 뒤 본문을 확인했다. 자동 픽셀 점수로 판정을 대체하지 않았다.
- `controls.mjs`: 기존 diagonal/indent/fresh indent/justify/TAC 공백/좁은 셀/폭0 셀 앵커 ×
  두 끝 간격 정책 **14조합**에서 승인 SVG 보존 및 Native/WASM 동일성 통과(`controls.log`).
- `cargo fmt --all -- --check`, `git diff --check`, `source.sha256` 및 review worktree와의
  source/test 해시 일치 확인. 검증 후 제품 코드 변경 없음. 전체 PR lint/CI·remote push·
  기본 엔진 전환은 수행하지 않았다.

메인테이너 시각 판정 대상은 **단일 저장 탭을 분리한 정상 한컴 대조군1쪽**이다.

- [fresh WASM 확대 review](../../output/7353/r19/tabs/review/wasm-focus-review.png)
- [전체 compare](../../output/7353/r19/tabs/review/wasm-compare-1.png)
- [standalone overlay](../../output/7353/r19/tabs/review/wasm-overlay-1.png)
- [정상 저장 HWP](../../tests/fixtures/issue7353_stored_tab_review/tab-saved.hwp)
- [동일 HWP의 한컴 PDF](../../tests/fixtures/issue7353_stored_tab_review/tab-2020.pdf)

메인테이너가 위치 일치로 판정하고 다음 절편 진행을 승인했다. 글리프 외형의 완전 일치나
원본 전체 통과로 확대 해석하지 않는다. 원본 #6923 전체는 다음 저장 TAC 줄 소속 경계로
진행했으며 아직 미완료다.

### 다음 절편: 저장 공백 줄과 뒤 TAC 표의 줄 소속

승인된 탭 절편은 `68059d15f`로 커밋했다. 원본의 prefix 진단에서 p68까지의 다음 실패는
`V2 cell border style`, p69를 포함하면 저장 TAC 거부로 바뀐다. p69는 공백60개가 첫 줄을
차지하고, source offset60의 표가 둘째 줄에 있다. 첫 줄1400HU와 다음 원점2116HU를
표가 없다는 이유로 지우면 편집자의 줄 구성과 뒤 표 위치를 훼손한다.

원본 p69를 기존 검증용 부모1×1 표에 분리하고 저장 줄을 제거한 HWPX를 한컴에서 정상
저장했다. 문단/자식 표의 원본 속성은 보존했고 부모 높이26000HU·독립적인 뒤 문단을 사용한다.
입력 XML의 음수 바깥여백-1HU를 한컴은0으로 다시 저장했으며 둘째 줄 높이도14845에서
14847HU로 바뀌었다. 따라서 이 저장본을 원본 음수 여백의 배치 근거로 대신하지 않는다.

이번 구현 대상은 **정상 저장본의 공백 전용 첫 줄 보존**이다. `tac::object_rows`가 줄 소유와
공백 advance를 확정하고 `compose`는 그 실제 줄 상자를 `ParagraphItem::Lines`로 전달한다.
`ir::bind_table`/`FlowBlock::Lines`의 기존 fit·continuation과 같은 payload의 TextPaint가
측정/실제 배치를 소비한다. 공백을 Space(0)으로 바꾸거나 표 줄에 합치지 않는다.
음수 여백은 명시적 미지원으로 유지한다. 원본과 정상 저장본은 따로 추적한다.

전체 자식 내용을 보존한 분리본은 수정 뒤 `text preview run outside occupied line`에서
거부된다. 이 별도 경계를 숨기지 않고 원본/분리본을 output에 보존했다. 줄 소속의 시각
검증용 대조군은 같은 문단·공백60개·자식 표45360×14847HU를 유지하고 자식 내부만
읽을 수 있는1×1 `TABLE ON SECOND LINE`로 재작성해 다시 한컴 정상 저장했다.
`tests/fixtures/issue7353_tac_blank_row_review/README.md`에 변경점·생성 job·해시를 기록했다.

실제 적용 경로와 경계:

| 경계 | 이번 결과 및 소비 경로 |
| --- | --- |
| 저장 줄 소속 → 높이/원점 | `tac::object_rows`가 공백 전용 줄에도 source offset의 공백 payload를 귀속한다. `tac::compose`는1400HU 줄과716HU 다음 간격, 둘째 줄의1000HU indent를 보존한다. |
| 컷/예약 → 이월 | `ir.rs`의 Lines 변환 → `flow.rs`의 Lines 분기는 전체1400HU 상자가 fit할 때만 소유 줄을 소비한다. 공간만 fit하면 기존 Space 규칙으로 패딩만 소비하며 공백 줄은 남는다. 이어 TAC InlineTables 분기는14847HU 표 전체를 원자적으로 예약한다. |
| 실제 배치 | `TextPaint::build_node`의 소유 key와 확정 LinePlacement가 같은 payload를 평행이동한다. 뒤 표 원점은 별도 텍스트 가시성으로 추측하지 않는다. Native/fresh WASM의 실제 출력 비교를 수행한다. |
| 합성 분할 경계 | 정상 저장본의 부모 break 정책만 CellBreak로 바꾸고 공백 줄까지1HU 부족/표까지1HU 부족 예산을 실행한다. 전자는 패딩만 소비 후 공백 줄 보존, 후자는 공백60개1회 출력 후 표만 이어받기, 최종 종료를 검사한다. 한컴 분할 일치 증거로 부르지 않는다. |
| 비해당 | rowspan·반복 제목·각주·캡션·일반 어울림·음수 여백 정책은 바꾸지 않았다. Legacy/default와 편집 재조판도 변경하지 않았다. |

새 정상 저장본 계약은 수정 전 release-test 라이브러리에서
`TAC spaces without occupied object row`로 FAIL(`contract-before.log`), 수정 후 Native에서
PASS다. 문서 흐름72건·텍스트30건 통과. 같은 HWP의 PDF에서 추출한 표 외곽 간 변위도
별도로 검사한다: x=(74.960938−56.609375)pt, y=(770.156250−746.183594)pt,
300dpi1dot 허용. 저장 메트릭 기반 실제 좌표 검사는 기존대로 정확한 값이다.

Native compare·standalone overlay를 직접 확인했다. 부모 상단에서 자식 표까지 공백 줄이
보존되고 자식/부모 외곽·뒤 문단 위치가 대응한다. 선 농도와 폴백 글리프 외형 차이는 남는다.
최종 집중 회귀와 fresh WASM 결과는 아래에 이어 기록한다.

최종 backend 검증(source `68059d15f`+`output/7353/r19/tac-next/final-source-tests.sha256`):

- Docker `docker compose --env-file .env.docker -p rhwp run --rm wasm` 성공7분27초.
  WASM SHA-256 `5ec3c08a5358eee9f97ab7ac75997eb8e5e33673515611176c7af4f9a4fc1288`.
- 최적화 완료 후 `review.mjs --wasm` 재캡처. Native/fresh WASM의 SVG·RenderTree 동일,
  numeric diff0. `review/run.json`에 source/input/PDF/WASM 해시 고정.
  같은 영역의 WASM review와 standalone overlay를 직접 확인했다.
- `controls.mjs`: 기존 승인 대조군7개×끝 간격 정책2개=14조합의 SVG 유지와 Native/WASM
  동일성 통과. 자동 비교는 기존 출력 무회귀 근거이지 새 한컴 시각 판정의 대체가 아니다.
- review worktree의 V2 source와 현재 작업 source가 모두 같음을 확인했다. 첫 nextest 실행은
  이전 release-test 라이브러리의 거부로 신규2건이 실패했다. 그 결과를 수정 후 결과로
  인정하지 않고 `cargo build --locked --profile release-test --lib --target-dir
  /home/edward/mygithub/rhwp/target/pr-review`로 라이브러리를 명시적으로 갱신했다.
  갱신된 release-test probe의 전체 출력이 Native debug와 동일함을 확인했다.
- `cargo fmt --all -- --check`, `git diff --check`, source 해시 검사 통과.
  전체 PR lint/CI·원격 게시·기본 엔진 전환은 수행하지 않았다.

명시적 library 갱신 후 동일 집중 nextest 필터의 최종 결과는 **277 passed /0 failed**,
2565 skipped,13개 suite binary다(`output/7353/r19/tac-next/focused-final.log`). 필터는
`test(issue_7353_table_v2) | test(issue_7353_rowspan_roundoff) | test(issue_4755) |
test(issue_6102) | test(issue_3128)`이며 release-test profile과 고정 review target을 썼다.
최종 source/test와 review worktree 입력의 일치, 포맷·diff 검사를 다시 확인했다.

메인테이너 판정 대상은 **공백 전용 첫 줄 + 둘째 줄 TAC 표의 정상 저장 분리 대조군1쪽**:

- [fresh WASM review](../../output/7353/r19/tac-next/review/wasm-review-1.png)
- [standalone overlay](../../output/7353/r19/tac-next/review/wasm-overlay-1.png)
- [정상 저장 HWP](../../tests/fixtures/issue7353_tac_blank_row_review/space-row-saved.hwp)
- [동일 HWP의 한컴 PDF](../../tests/fixtures/issue7353_tac_blank_row_review/space-row-2020.pdf)

메인테이너가 위치 일치로 판정하고 다음 절편 진행을 승인했다. 승인 범위는 부모 상단과 자식
표 사이 공백 줄, 둘째 줄 들여쓰기, 양쪽 표 외곽과 AFTER CELL 위치다. 원본 전체 통과나
음수 여백 지원, 글리프 외형의 완전 일치를 뜻하지 않는다.

### 전체 진행 현황 — 위치 일치 판정 후

승인된 R2~R5 전체 범위에 대한 거친 진행 추정은 약45%(40~50% 범위)다. 절편·커밋·테스트
개수의 비율이나 측정된 일정이 아니다. R2의 공통 계획/소유권/경로 격리 기반은 확보했고,
R3는 본문·중첩 표·저장 줄·Native/WASM 연결을 구현했지만 #6923 원본 전체 수용과 기준
출력 대조는 미완료다. R4의 정렬·제목 반복·그림/서식은 일부 확보했으나 일반 어울림,
복합 분할·각주·캡션·편집/캐시/커서·세로쓰기 등은 남았다. R5의 최신 devel 통합·전체 검증·
전환 판단도 남아 있다. 반 이상 잔여가 있다고 보고하며, 지원 경계를 숨긴 완료율로 쓰지 않는다.

다음은 원본 p69의 음수 바깥여백과 저장 TAC 점유 높이 관계를 독립 근거로 추적한다.
정상 HWPX 재저장으로 여백이0이 된 대조군은 이 원본 규칙의 근거로 대신하지 않는다.

### 다음 묶음: 원본 차단점 분리와 자식 내용을 보존한 법령 표

앞 위치 판정 절편은 `e266850af`로 커밋했다. 음수 여백은 사양의 HWPUNIT16(INT16)과
`parser/control.rs`의 i16 파싱을 통해 IR에 그대로 들어온 값이다. 원본 p69 표는
14847−1−1=14845HU, 본문 p29 표도11156−1−1=11154HU로 저장 줄 높이에 대응한다.
단순 파서 오류·한 문서에만 붙일 예외가 아니다. 현재 `tac::object_rows`의 음수 거부 뒤에는
`content.rs`의 child.y≥0/child 끝≤행 높이, `flow.rs`의 행 전체 예약, `ParagraphEnd`의
점유 끝 계산이 연결된다. guard만 풀면 물리 표와 저장 줄 경계가 어긋난다. 위쪽 돌출과
다음 줄 전진·페이지 경계의 계약을 함께 구현해야 하며, 이번 묶음에서는 음수 지원을
완료로 바꾸지 않는다. 독립 기존 PDF는 그대로 보존했고 재생성으로 원본을 대체하지 않았다.

동시에 자식 내용을 그대로 보존한 정상 저장 분리본의 다음 거부를 셀/문단으로 좁혔다.
`output/7353/r19/tac-next/diagnose.rs`의1×1 분리 probe는 위치 일치 증거가 아니라 원인
분류용이다. `c2/p1`, `c3/p1`의 줄 끝 밑줄 공백이 대상이다. 전자의 line 끝277.44px에
run 논리 끝284.6px, 후자의 line 끝300.1066667px에 run 끝307.2666667px가 관측됐다.
둘 다 실제 출력의 soft-wrap 공백 제외와 달리 V2 guard가 장식된 논리 공백을 표시 폭으로
판정했다. 임시 진단 출력은 제거했고 원본·정상 저장 분리본을 변경하지 않았다.

독립 규칙은 기존 #6028/#6117의 한컴 출력과 공통 paint 계약이다. 마지막 가시 run의
자동 줄바꿈 구분 공백만 장식선에서 제외하며, 문단 끝/강제 개행/별도 공백-only run의
작성자 공백은 보존한다. `TextRunNode::soft_wrap_decoration_trim`으로 기존 판정을 추출해
SVG·WebCanvas·LayerBuilder와 V2 표시 폭 검사가 공유한다. 줄·논리 텍스트·font 크기·
저장 partition은 바꾸지 않는다. 공용 출력 경로의 기존 동작은 동일하며 회귀 검사를 수행한다.

이후 같은 분리본에서 자식 표 첫 행만 배치한 뒤 `InconsistentAtomicPlan`을 확인했다.
두 행 높이는1765/75+13082/75=197.96px로 전체 높이와 같다. 그런데 원점126.48px을 더하면
`126.48+23.533333333333335+174.42666666666668=324.44000000000005`,
`126.48+197.96=324.44`라 전체 행 수용 검사가 둘째 행을 이월했다. `fragment::fit_rows`의
행 수용은 공통 계획과 같은 로컬 좌표 `offset+row_height <= budget`으로 바꿨다.
epsilon·높이 축소·추가 허용치 없이 원점 이동 불변식을 지킨다. 시작/끝 컷·rowspan 전용
분기는 수정하지 않았고, 수용된 행의 기존 내용/최소 높이/paint 경로는 유지한다.

정식 계약은 `tests/cases/issue_7353_table_v2_text.rs`의 soft-wrap 장식 및 exact-height
원점 이동 계약, `issue_7353_table_v2_document_flow.rs`의 정상 분리본 종단 계약이다.
수정 전 라이브러리에서 전자는 기존 표시 폭 거부로 FAIL, 후자는 y126.48에서 컷 변경으로
FAIL했다(`output/7353/r19/underline/before-text.log`, `before-atomic.log`). 빌드 오류는
이 red 결과에 포함하지 않는다. 수정 후 문서73건·텍스트32건 PASS이며 Native에서는
자식 네 셀·두 행·뒤 본문까지1쪽으로 출력된다.1HU 부족 예산은 여전히 둘째 행을 이월한다.
실제 LayerBuilder의 trim3과 SVG 장식선 끝225px도 독립 지정한200HU 줄 폭으로 검사한다.

정상 저장 HWP/PDF는 `tests/fixtures/issue7353_stored_underline_review/`에 보존한다.
이전 --simple 대조군과 달리 원래 자식 표 내용을 교체하지 않았다. 다만 원본을 분리하고
한컴에서 재저장한 입력이며 음수 여백은0으로 바뀌었다. 원본 전체 통과 자료가 아니다.
Native review를 직접 열어 동일 줄바꿈·두 행 외곽·공백 줄·뒤 본문을 확인했다.
폴백 글꼴 외형/굵기와 선 농도 차이는 남는다. 집중 회귀 및 Docker fresh WASM을 진행한다.

#### 법령 표 묶음의 초기 검증 — 추가 경계 보완 전

- source는 `e266850af` 위 작업 diff이며 `output/7353/r19/underline/source.sha256`으로
  변경 Rust6개와 정식 검사2개를 고정했다. 검증 worktree의 전체 `src`도 작업 소스와 같다.
- `docker compose --env-file .env.docker -p rhwp run --rm wasm` 완료:7분36초,
  컨테이너 exit0. 실행 CLI 연결은 종료됐지만 컨테이너의 최적화는 계속됐으므로 재빌드하지
  않고 `docker wait`와 `docker logs --follow`로 실제 완료를 확인했다.
  `docker-completion.log`, `docker-container-exit.log`에 보존했다.
- WASM SHA-256은 `cd7609258bc7564d5b23843136f3ec9aa5b2f4c8dd28cd23ac58be52f2e83036`.
  `node output/7353/r19/underline/review.mjs --wasm`의 브라우저 DocumentV2 출력은
  Native와 JSON 숫자·기타 필드 차이0, SVG 동일, 각각1쪽이다. `review/run.json`에 입력·
  기준 PDF·source·WASM 해시가 연결된다.
- `node output/7353/r19/underline/controls.mjs`: 기존 승인7개×끝 간격 정책2개=14조합
  모두 기존 SVG 유지 및 Native/WASM 일치. 새 한컴 피델리티 판정의 대체 근거는 아니다.
- `cargo clippy --locked -p rhwp --lib --target-dir /home/edward/mygithub/rhwp/target/pr-review
  -- -D warnings`와 같은 명령의 `--target wasm32-unknown-unknown` 모두 통과했다.
  전체 workspace/all-targets CI lint나 전체 회귀를 실행한 것으로 보고하지 않는다.
- 최종 WASM review와 standalone overlay를 직접 열어 법령 두 셀의 줄바꿈, 자식 표 두 행·
  외곽, 부모 공백 줄과 뒤 본문 위치를 확인했다. 폴백 글꼴 굵기/외형 및 테두리 농도 차이는
  남는다. 새 묶음의 메인테이너 시각 판정은 대기이며 앞 묶음의 위치 승인을 재사용하지 않는다.

시각 판정 자료(동일 정상 저장 분리본1쪽, 원본 전체 수용 자료 아님):

- [fresh WASM review](../../output/7353/r19/underline/review/wasm-review-1.png)
- [standalone overlay](../../output/7353/r19/underline/review/wasm-overlay-1.png)
- [정상 저장 HWP](../../tests/fixtures/issue7353_stored_underline_review/carrier-saved.hwp)
- [동일 HWP의 한컴 PDF](../../tests/fixtures/issue7353_stored_underline_review/carrier-2020.pdf)

집중 release-test 결과는285건 중284PASS/1FAIL이었다(`underline/focused.log`). 기존
`fractional_page_budget_does_not_split_an_atomic_nested_table`가 `InconsistentAtomicPlan`으로
실패했으므로 이 시점의 완료·시각 판정 요청은 보류했다. 위 초기 WASM 해시와 manifest는
`underline/run-before-budget.json`에 보존하며 최종 소스 증거로 재사용하지 않는다.

추가 원인은 부모1.2 예산에서 앞 내용1.0을 빼면 자식0.2보다 작은 값이 전달되는 데 있다.
절대 좌표의 반올림에 의존하던 원래 초기 fit 검사도 로컬 폭/높이 비교로 바꾸고,
`flow::child_budget`은 부모 로컬 좌표에서 `pen + (prefix + child_plan.height) <= total`이
입증된 경우 이미 수용 가능한 계획 높이를 자식 query에 보존한다. 실제 초과는 남은 예산만
전달하며 epsilon이나 선언 높이 축소로 통과시키지 않는다. 일반 Table와 AnchoredTable가
같은 함수를 사용하고 InlineTables는 이미 계획 높이를 직접 전달한다. 자식의 실제 예약은
기존 `fragment.reserved_height()`를 통해 부모 pen/occupied end/placement로 이어진다.
rowspan 전용 컷·헤더 반복 경로는 변경하지 않았다.

기존 실패 계약을 일반/앵커 자식과 원점0/20/126.48로 확장했다. 독립 예산1.2는 자식0.2와
앞 내용1.0을 완전히 수용하며, 바로 아래의 표현 가능한 f64 예산은 부모 원점과 무관하게
거부한다. 중첩 계약16건과 정상 분리본1쪽은 수정 후 통과했다. 최종 집중 검증은 빠른
경계 재검증을 위해 **dev profile**을 사용하며 release-test 결과와 혼동하지 않는다.
현재 source7개/정식 검사3개로 manifest를 갱신했고 Docker fresh WASM과 시각 캡처도
다시 생성한다.

보완 후 집중 검사 결과: `underline/focused-final.log`의 dev profile
nextest는 **285PASS/0FAIL**,3244 skipped,16개 suite binary다. 필터는
`test(issue_7353_table_v2) | test(issue_7353_rowspan_roundoff) | test(issue_4755) |
test(issue_6102) | test(issue_3128) | test(issue_6028) | test(issue_6117) | test(issue_6451)`.
초기 release-test의1건 실패와 분리해 기록하며 전체 CI 통과로 보고하지 않는다.

보완 후 최종 검증도 완료했다. Native/WASM lib Clippy는 각각
`clippy-native-final.log`, `clippy-wasm-final.log`에서 통과했고 포맷·diff·source 해시와
검증 worktree 전체 source 일치를 다시 확인했다. Docker 빌드는7분37초, exit0이며
`docker-final-completion.log`와 `docker-final-exit.log`에 보존했다. 최종 WASM SHA-256은
`bd2a3cce4e82853e01809e8a2850b87072b25ae4b2b63b232d8a1c6d1c6ed97a`다.

최종 `review.mjs --wasm` 실행에서 Native/WASM 각각1쪽, JSON 숫자·기타 필드 차이0,
SVG 동일을 확인했다. `controls-final.log`의 기존 승인 대조군14조합도 모두 유지됐다.
최종 source로 갱신한 위 **WASM review와 standalone overlay를 다시 직접 열었다**.
두 법령 셀의 줄 구성과 표/뒤 본문 위치는 유지되며 글꼴 외형·선 농도 차이는 남는다.
새 묶음은 메인테이너 시각 판정 대기다. 원본 음수 여백·원본 전체 수용은 계속 미완료이며,
전체 CI·기본 경로 전환·원격 게시를 수행하지 않았다.

메인테이너가 위 분리본을 통과로 판정했다. 부모26000HU의 남는 공간은 생성기에서 정한
검증용 높이이며 자식2행4셀이 조기 분할된 결과가 아님을 확인한 뒤의 승인이다. 승인 범위는
해당 분리본의 줄바꿈·밑줄·표/뒤 문단 위치이며 원본 전체 페이지네이션 통과로 확대하지 않는다.
전체 진행 추정은 약45%(40~50% 범위)로 유지한다. 다음은 원본 음수 바깥여백에서
줄 전진과 실제 표 점유의 공통 계약 및 실제 원본 다음 차단점을 추적한다.

#### 승인 후 다음 묶음 — 음수 표 바깥여백의 점유 계약 (진행 중)

앞 승인분은 `4af4f2232`로 커밋했다. 전체 범위 기반 추정은 여전히 약45%다.
R2 기반은 마련됐지만 R3 원본 종단 연결, R4 복합/편집 경계, R5 통합·전환은 남아 있다.
새 검사 건수나 원본의 거부 지점 이동만으로 전체 완료율을 올리지 않는다.

근거는 원본 HWP와 HWP 5.0 사양의 signed HWPUNIT16 바깥여백이다. 원본
`s0/p5/t0/c0/p69/t0`는 높이14847HU, 위/아래-1HU, 저장 줄 높이14845HU이며
`s0/p29/t0`도11156-1-1=11154HU다. 이전 한컴 재저장 분리본은 여백이0으로
정규화됐으므로 이 음수 여백의 기준 출력으로 사용하지 않았다.

공통 결과의 실제 경로:

- `tac::object_rows`는 저장 줄 높이·전진량과 signed margin을 포함한 원본 표 rect를
  보존한다. `tac::compose`는 줄 전진과 별도로 실제 표 하단을 포함하는 physical height를
  만든다. 그림의 음수 여백은 이번 수용 대상이 아니다.
- 본문 `document_input`과 셀 `ir::bind_table` 모두 같은 InlineTables 결과를 바인딩한다.
  `ParagraphEnd::from_composed`와 `content::physical_extent`는 각각 이 height/advance를
  소비한다. 표의 선언 높이를 줄 높이로 축소하지 않는다.
- `FlowCursor::fit_until`은 로컬 `pen+height`로 예산을 검사하고 자식을 완전한 계획 높이로
  예약한다. 실제 child origin은 `area.y+pen+child.y`, 후속 원점은 `pen+advance`다.
  줄 전체가 fit하지 않으면 컷을 소비하지 않는다. rowspan 전용 컷은 변경하지 않았다.
- 음수 top이 앞 밴드 안에 들어오는 경우만 배치한다. 이어받은 조각의 시작 위로 돌출하면
  `inline table extends above fragment origin`으로 명시적 거부한다. 첫 줄의 음수 여백
  원점·수평 부모 밖 돌출은 아직 미지원이며 임의 padding/clamp/clip으로 감추지 않았다.

`tests/cases/issue_7353_table_v2_nested.rs`에 독립 예산 계약을 추가했다. 앞 공간10px,
자식50px, top/bottom-1px이면 논리 끝58px과 실제 끝59px을 구별한다. 58px 예산에서는
자식을 수용하지 않고59px에서는 전체 수용한다. 뒤 문단의58px 원점과 다음 조각에서의
내용 보존, 원점 이동, 앞 공간 없는 이어받기의 명시적 거부를 검사한다. 수정 전 라이브러리는
`InvalidNumber("inline table y")`로 FAIL, 수정 후 PASS했다(`signed-margin/before.log`).
초기 테스트 작성 중 컴파일 오류·원점 기대값 보완은 이 red 증거에 포함하지 않았다.

`issue_7353_table_v2_document_flow.rs`는 원본 속성의 산술 관계와 별도의 합성 HWP
종단 계약을 구별한다. 같은 줄의 두 표 및 뒤 본문 좌표, top0/bottom-2px에서 논리 줄만
fit하는52px 페이지 예산의 실제 이월을 검사한다. 합성 계약을 한컴 일치로 보고하지 않는다.

Native dev 라이브러리 빌드 후 정식 `tests/cases/issue_7353*.rs`18개를 rustc test harness로
실행해 **277PASS/0FAIL**했다. 파일별 build/run 로그는 `output/7353/r19/signed-margin/`.
export/image와 split-line/zip 의존성 누락으로 처음 컴파일하지 못한2개도 해당 extern을
명시한 뒤 재실행했다. Cargo generated-suite/전체 CI 실행과 구분한다. Native lib Clippy는
통과했다. 이번 변경의 fresh WASM 런타임·직접 시각 비교는 아직 미실행이며 완료로 세지 않는다.

원본 terminal-policy 실행은 음수 여백 거부를 넘어 `s0/p5`의 `V2 cell border style`에
도달했다(`signed-margin/original.log`). 입력을 바꾸지 않았으며 실제 원본 페이지는 아직
출력되지 않는다. 최초 미지원 선 종류는 `s0/p5/t0/c0/p7/t0`의 borderFill47/46에 있는
Double이다(`signed-margin/next-blocker.log`). 원본 수용 경계 진단 assertion만 새 거부
항목으로 갱신했으며 페이지 수·golden·피델리티 허용치를 바꾸지 않았다.
다음 종단 의존 작업은 이중선과 공유 경계의 출력이며, 기존 승인 PNG를 새 원본 판정용으로
재사용하거나 현재 합성 계약만으로 새 시각 승인을 요청하지 않는다.

이중선 후속 조사의 독립 자료로 원본 한컴 PDF4쪽을 `pdftocairo -f 4 -l 4 -svg`로
추출했다(`signed-margin/reference-p4.svg`). 제목 숫자 셀의 평행선은 stroke0.36pt,
중심 간격 약1.08pt로 관측된다. 기존 Legacy의 최소3px 합성이나 SVG 일반 도형의
30/40/30 비율을 표 이중선의 정답으로 곧바로 재사용하지 않고, 저장 굵기·600dpi 격자·
공유 경계/모서리를 함께 대조해야 한다. 이번 절편에서는 이중선 구현을 변경하지 않았다.

이후 backend 검증을 완료했다. Native/WASM lib Clippy 모두 통과했고 Docker 표준 빌드는
7분13초, exit0으로 완료했다(`signed-margin/docker.log`). WASM SHA-256:
`337954b7ec128a2c2a4275d9ec1cf5895d35a342623113cef6eea009d1b6eaa4`.
소스는 `4af4f2232` 위 diff이며 `signed-margin/source.sha256`의3개 Rust/2개 검사 파일로
고정했다. 기본 Legacy 경로는 변경하지 않았다.

- `controls.mjs`: 승인된7개 대조군×2끝 간격 정책=14조합 모두 기존 SVG 유지,
  Native/fresh WASM SVG 동일(`controls.log`).
- `boundaries.mjs`: 합성 HWP의 signed-inline1쪽/signed-bottom2쪽 모두 Native와 SVG
  동일. 전자는 본문 컨테이너 높이112와112.00000000000001의1 ULP 차이만 있고,
  표·문단 좌표는 같다. 후자는 JSON도 동일하다. `*-comparison.json`에 차이를 보존했다.
  처음 전체 JSON exact equality 실패는 `boundaries-exact-json-failed.log`에 보존했다.
  엔진 좌표나 정식 회귀 허용치는 바꾸지 않았고, 진단 harness는 numeric 차이를 기록하고
  machine-relative precision 이하인지 확인한다. 문자열/SVG/소유권은 정확히 비교한다.
- 브라우저 launch 실패1회는 별도 `boundaries-launch-failed.log`로 남겼으며 재시도 성공했다.
  빌드를 반복하지 않았다. harness의 JSON 객체 key 순서 비교도 의미상의 key 집합 비교로
  바로잡았다. 이 두 항목은 엔진 결함이나 회귀 실패 건수에 포함하지 않는다.
- 기존 승인 법령 분리본의 새 Native/fresh WASM 비교·standalone overlay를 직접 열어
  줄바꿈·두 행 외곽·부모 빈 공간·AFTER CELL 위치가 유지됨을 확인했다. JSON 차이0,
  SVG 동일이며 폴백 글꼴 외형·선 농도 차이는 이전과 같다.
  [새 대조군 review](../../output/7353/r19/signed-margin/review/wasm-review-1.png),
  [standalone overlay](../../output/7353/r19/signed-margin/review/wasm-overlay-1.png).
  새 음수 여백 합성 출력3쪽도 직접 확인했다. 이것은 계약/무회귀 증거이며 원본의 한컴
  피델리티 통과 자료가 아니다. 원본 전체는 이중선에서 계속 미지원이며 R3/R5 완료나
  전체 CI 통과로 보고하지 않는다.

#### 승인 후 다음 묶음 — 표 이중선과 공유 경계

사용자가 다음 절편을 승인했다. 변경 소스는 `3c35762b1` 위 diff이며
`output/7353/r19/double/source.sha256`으로 고정했다. Legacy 경로와 기본값은 유지한다.

독립 근거는 원본 #6923 PDF4쪽 제목 셀과 새 한컴 정상 저장2×2 대조군4개다.
입력 생성 방식·HWP/PDF job·한계는
`tests/fixtures/issue7353_double_review/README.md`에 기록했다. 원본 제목 추출본은
table/cell outline 불일치가 있어 미지원이며, 이 사실을 바꾸지 않고 별도 동일선 대조군을
작성했다. PDF의 굵기ID0/3/7/11 pen은 각각0.12/0.12/0.36/약1.079pt,
중심 간격은 약3pen이다. 600dpi 격자에서 선언 폭을4등분한 pen과1:2:1비율에 해당한다.

공통 경로는 `CellBorders::prepare`/`resolve_edges`의 저장 borderFill → 조각의 실제 행/셀 경계 →
`CellBorders::append`의 공유 span union/outline/zone 적용 → `borders::double::append`의
동일 기하에 두 pen 배치 → Single LineNode/ink_bbox → 공통 SVG·WASM export다.
측정·컷·예약 높이는 변경하지 않는다. 공유 경계는 한 번만 만들고 L/T/교차점의 안쪽 선을
접점에서 끝내 빈 간격을 가로지르지 않게 한다. 서로 다른 굵기/색/선 종류 접점과
Double zone perimeter는 독립 검증 전까지 명시적으로 거부한다.

정상 저장 대조군 실행에서 별도의 수평 폭 산술 오류가 검출됐다.
저장 줄13980HU/75와 `(15000-510-510)/75`의 계산 순서 차이로
186.40000000000001과186.39999999999998이 비교되어 ContentBounds가 발생했다.
`content::exceeds_text_lane`은 machine-relative4EPSILON 범위의 산술 오차만 구별한다.
실제 줄 상자·배치 원점은 바꾸지 않으며 .01HU/1HU의 실제 넘침은 계속 거부한다.
수직 예산, TAC/rowspan 컷의 허용치는 바꾸지 않았다. 실제 저장 입력을 수정하지 않았다.

정식 회귀 검사는 `tests/cases/`에 두었다. 이중선 추가 전에는 border style 거부,
폭 산술 보완 전에는 정상 저장 grid의 ContentBounds로 실패한 로그를 각각
`double/before.log`, `double/roundoff-before.log`에 보존했다. 현재18개 집중 harness는
**284PASS/0FAIL**이다. 최종좌표·이중선 간격·교차점·공유선 중복·네 셀 글자와AFTER CELL을
검사하며, 별도 합성 계약으로 제목 반복/셀 분할/빈 물리 꼬리에서 기존Solid와 같은 내용·
점유 경계·종료를 확인했다. 한컴 기준의 없는 분할Double 출력은 이 합성 계약과 구분한다.
Native/WASM lib Clippy와 cargo fmt check는 통과했다. 전체CI·workspace lint는 이번
내부 절편에서 실행하지 않았다. 테스트 수 증가를 전체 타스크 진척률로 환산하지 않는다.

원본 #6923은 입력 변경 없이 `V2 overlapping zone decorations`까지 진행했다.
정상 저장 제목 추출본은 `V2 table/cell outline disagreement`에서 멈춘다.
원본 전체 페이지/총 페이지 수의 통과 자료는 아직 없다. 다음 원본 차단점은 이 두 장식
우선순위 규칙이며, 이번Double 대조군만으로 R3/R5 완료를 선언하지 않는다.

최종 backend 검증: `docker compose --env-file .env.docker -p rhwp run --rm wasm`는
7분18초/exit0으로 완료했다(`double/docker.log`). WASM SHA-256은
`4e31fbe3d1c1eb873aa9a2f845b795fe68e3e6a489d551be7bc2ad470a19abee`다.
`DOUBLE_CASE=grid-{0,3,7,11} node output/7353/r19/double/review.mjs --wasm`로
네 정상 저장 입력의 Native/fresh WASM JSON차이0·SVG동일을 확인했다.
`double/controls.mjs`의 기존 승인7대조군×2정책=14조합도 모두 기존SVG 유지·backend동일이다.
Native review 일괄실행 중 Chrome launch가1회 실패했고 단독 재실행은 성공했다.
빌드/엔진 실패로 세지 않았다. 입력·PDF·소스·WASM hash는 각grid `run.json`에 있다.

Native/fresh WASM review와 대표 standalone overlay를 직접 열어 네 셀·공유선·교차점과
AFTER CELL 보존을 확인했다. 96dpi에서는 가는 선의 농도/antialias 차이와 폴백 글꼴 차이가
남는다. 굵은 선의384dpi확대에서는 두 pen·빈간격·L/T/십자 형태를 직접 확인했다.
PDF의600dpi끝점 반올림과 연속좌표의 미세 차이는 완전한 픽셀 일치로 보고하지 않는다.
최종 메인테이너 시각 판정은 다음 자료로 요청한다.

- [이중선 전체/후속 문단 비교](../../output/7353/r19/double/grid-11/wasm-review-1.png)
- [표 네 셀 전체384dpi확대](../../output/7353/r19/double/grid-11/wasm-highdpi-review.png)
- [standalone overlay](../../output/7353/r19/double/grid-11/wasm-overlay-1.png)
- 입력: `tests/fixtures/issue7353_double_review/grid-11-saved.hwp`
- 기준: `tests/fixtures/issue7353_double_review/grid-11-2020.pdf`, 물리1쪽

이 자료는 원본 전체가 아닌 명시된Double 규칙의 대조군이다. 원본 수용 경계와
전체타스크 남은 범위는 위 제한을 유지한다. 전체진척 추정은 약45%(40~50%)이며,
이번 좁은 선 출력 지원만으로 전체완료율을 올려 보고하지 않는다.

#### 이중선 승인 후 — 중첩 구역 장식·셀 테두리 소유

사용자가 앞 절편의 이중선 시각 판정을 통과시키고 다음 진행을 승인했다.
이번 변경은 `9f2196f4d` 위 diff이며 `decoration-priority/source.sha256`으로 소스를
고정한다. 기본 Legacy와 페이지네이션/측정은 변경하지 않는다.

원본 #6923의 차단 구역은 s0/p5/t0/c0/p37/t0의7×10표다. 구역(r3,c0)..(r3,c0)과
(r0,c0)..(r5,c9)는 모두 fill2의 동일 검정 실선/배경 없음이다. 서로 다른 장식의
선언 순서 우선순위가 아니라 같은 효과의 합성이 필요했다. 제목 추출본은 표 전체
실선 선언과 셀별 Double/Solid/None이 공존하며, 한컴 PDF는 셀별 선을 사용한다.

독립 대조군의 생성·원본/정상 저장본/PDF·job과 제한은
`tests/fixtures/issue7353_decoration_priority_review/README.md`에 있다.
첫 outline 진단 입력은 모든 borderFill을 변경하여 페이지/문단까지 테두리가 생겼다.
그 입력·출력은 output에 보존하고 판정에서 제외했다. 셀 fill만 복제하는 별도의
outline-clean 대조군을 정상 저장했다. 구역 대조군도 저장 LineSeg를 수동 조정하지
않고 한컴에서 다시 계산했다. 기존 title-saved.hwp/PDF는 변경 없이 재사용한다.

적용 경로는 `text_ir::prepare`의 `Zone::prepare`/`CellBorders::prepare` →
기존 측정·분할이 수용한 `TablePlacement` → `TextPaint::build_node`
(`text.rs`)의 실제 cell bounds → `Zone::bounds`/`append_background`와
`CellBorders::append`의 topology/공유선 union → zone perimeter 적용 →
Double/Solid LineNode → 공통 SVG/WASM이다. 줄·개체 원점과 예약 높이는 그대로이며
측정에 장식 높이를 재추가하지 않는다. 재귀 자식과 일반/반복제목/셀내 분할 조각도
동일 build_node를 소비한다. 컷·요구 높이·예산 실패 이월·소비 유닛 변경은 비해당이다.

- 같은 불투명 단색/동일 선이고 대각선이 없는 중첩 구역만 합성한다. borderFill ID가
  달라도 지원하는 배경/선 속성이 같으면 수용한다. 서로 다른 효과·중첩 대각선·Double
  zone은 계속 미지원이다. 실제 조각의 zone span을 먼저 union하여 반대쪽 선끼리의
  충돌까지 검사한 뒤 cell edge를 대체한다. 같은 borderFill에서도 왼쪽/오른쪽 색이
  다르면 인접 구역 경계에서 충돌할 수 있기 때문이다. `zone-junction-before.log`는
  첫 구현이 이 충돌을 덮어쓴 반례의 FAIL이며, 선언 역순/실패 시 cursor 불변까지
  정식 검사에 포함했다. 배경/선의 선언 순서로 충돌의 승자를 정하지 않는다.
- 셀의 명시적 borderFill은 all-None까지 테두리 소유로 보존한다. 표 전체 선을
  덧그려 셀의 빈 구간을 채우지 않는다. 참조0은 all-None과 다르며, 참조 없는 외곽
  셀의 table-outline fallback은 아직 미지원이다. 인접 셀 선 충돌 규칙은 유지한다.

수정 전 기존 라이브러리에서 새 계약은 outline disagreement/overlapping zones로
실패했다(`borders-before.log`, `flow-before.log`). 그 뒤 강화한 실제 출력 검사는
outline-clean의 두 열린 구간/검정 선/30000×8000HU 크기, title의8개 Double pen과
4개 Solid edge/공백 셀 가로선 없음/AFTER CELL, zones의행4위·행7아래 빨강선/두 쪽
내용과 뒤 문단을 확인한다. 기존 실행 바이너리로 실제 세 입력도 같은 원인으로 실패했다
(`before-{outline-clean,zones,title}.log`). 빌드 시 누락된 roxmltree extern 오류는
환경 오류이며 결함 재현 수치에 포함하지 않았다. 새 검사 width ID8의 초기 예상값
오기는 선언0.6mm의600dpi 반올림14unit 근거로 정정했다(엔진/허용치 변경 없음).

원본 전체는 입력을 바꾸지 않고 문단22의 `stored text requires intact single-segment
rows`까지 진행한다(`original.log`). 이는 다음 수용 차단점 진단이지 전체 조판 통과가
아니다. 전체 R3/R5 완료와 원본 페이지 수 일치, 실제 Studio 수동 검증은 미검증이다.
이번 자료는 V2 Document API의 좁은 장식 규칙 검증으로 한정한다.

최종 Native 검증은 `decoration-priority/tests.sh`의 기존18개 #7353 harness에서
**289PASS/0FAIL**이다(`tests-final-summary.log`). Native/WASM lib Clippy는 각각
`clippy-{native,wasm}-final.log`, fmt는 `fmt-final.log`에 남긴다. 전체workspace lint,
전체CI/원격 검증은 내부 절편에서 실행하지 않았으며 이 결과를 CI 통과로 보고하지 않는다.
첫 Docker 빌드7분25초 뒤 zone 접점 반례를 보완했으므로 그 빌드는 최종 증적이 아니다.
최종 소스로 Native를 다시 출력하고 Docker WASM도 다시 빌드한다(`docker-final.log`).

최종 `docker compose --env-file .env.docker -p rhwp run --rm wasm`는7분20초/exit0으로
완료했다. WASM SHA-256은
`4771b46f65e44048a3fd3b3460bf75c315bc24ec19d0d4dcbb7ad63628a26a43`이다.
`REVIEW_CASE={outline-clean,zones,title} node output/7353/r19/decoration-priority/review.mjs
--wasm`의3입력4페이지는 JSON수치/기타 차이0, SVG동일이다. 각 run.json에 입력·기준PDF·
소스·WASM hash가 있다. `controls.mjs`의 기존7대조군+Double4대조군×2정책=22조합은
모두 이전SVG 보존/Native-WASM동일이다(`controls-final.log`). 최종 정상 저장 속성 검사를
보강한 document-flow harness도77PASS로 재확인했다(`flow-final.log`, 총계289유지).

최종 소스의 Native/fresh WASM review·standalone overlay와 제목384dpi 확대를 직접
열어 셀 사이 열린 구간, 이중선/실선, 구역 안쪽/바깥쪽 테두리, 노랑 셀 배경, 두 쪽
연속 내용과 AFTER문단을 확인했다. 폴백 글꼴 외형·PDF 끝점 양자화/antialias 농도 차이는
남으며 이를 완전 픽셀 일치로 보고하지 않는다. 다음 자료의 메인테이너 판정은 대기한다.

- [원본 제목 추출본 확대](../../output/7353/r19/decoration-priority/title/wasm-highdpi-review.png):
  `tests/fixtures/issue7353_double_review/title-saved.hwp`/대응PDF1쪽.
  번호 이중선과 제목 실선 사이의 빈 셀에 가로 테두리가 생기지 않아야 한다.
- [명시적 선 없음 대조군](../../output/7353/r19/decoration-priority/outline-clean/wasm-review-1.png):
  `tests/fixtures/issue7353_decoration_priority_review/outline-clean-saved.hwp`/대응PDF1쪽.
  좌상단 위/우하단 아래가 열린 상태이며 빨강 표 전체 외곽선은 표시하지 않는다.
- [중첩 구역1쪽](../../output/7353/r19/decoration-priority/zones/wasm-review-1.png),
  [2쪽·후속 문단](../../output/7353/r19/decoration-priority/zones/wasm-review-2.png):
  `tests/fixtures/issue7353_decoration_priority_review/zones-saved.hwp`/대응PDF1·2쪽.
  행4위/행7아래 빨강선과 기존 배경·분할 위치·AFTER ZONE TABLE을 함께 비교한다.
- standalone overlay는 같은 디렉터리의 `wasm-overlay-{1,2}.png`, 전체 비교는
  `wasm-compare-{1,2}.png`다. 원본 전체의 시각 통과 자료로 확대 해석하지 않는다.

다음 대상은 원본 문단22의 저장 줄 구성 경로다. 이번 절편에서는 그 수용 조건이나
페이지 수 기준을 변경하지 않았고, push/PR/원격 갱신도 수행하지 않았다.

### 2026-09-26 — 원본 단위 문단의 0% 줄간격: 점유 높이와 전진량 분리

작업지시자가 앞 절편(장식 우선순위)의 시각 판정 통과와 다음 진행을 승인했다.
그 판정은 위의 장식 대조군에 기록하며 원본 전체 조판 통과로 확대하지 않는다.
이번 소스 시작점은 `4139473e6`, 증적은 `output/7353/r19/stored-body-rows/`다.

원본 #6923 PDF 물리6쪽의 본문 문단22(0-based)는 `(단위 : kl, %)`이며,
ParaShape134의 Percent0, 저장 높이1000HU/간격−1000HU로 **전진량이0**이다.
이는 높이0인 공백이 아니다. 다음 빈 문단23은 같은 vpos19852HU에서 높이1300HU/
간격−260HU를 점유하고, 문단24의 표는20892HU에서 시작한다. 원본 문서의 저장값과
대응 PDF의 위치를 대조했다. 거부 사유의 `single-segment rows` 문구와 달리 실제
차단 조건은 `height + spacing <= 0`이었다. 문서 ID·특정 문자열 분기는 넣지 않았다.

공통 결과의 생산·소비 경로:

| 단계 | 실제 경로와 계약 |
| --- | --- |
| 수용/공통 배치 생산 | `stored_text.rs:109 localize`는0전진을 허용하되 음수 전진·비양수 줄 높이를 계속 거부. `text.rs:431–491`의 공유 문단 배치와 `validate_paint`가 저장 줄·실제 bbox·끝점을 대조 |
| 공통 결과 | `text.rs:498–580`: 실제 TextLine bbox에서 `height`와 다음 원점까지의 `advance`를 분리. advance0도 Lines/LineOwner를 보존하며 Space(0)으로 바꾸지 않음 |
| 측정 | `content.rs:78 physical_extent`와 `paragraph_end.rs:from_composed`: max(pen+height)는 물리 점유, pen+=advance는 후속 원점. 기존 공통 측정을 그대로 소비 |
| 문서/셀 수용 | `document_input.rs`/IR cell lowering → `FlowBlock::Lines`; `flow.rs:193–213`은 height 전부가 예산에 맞아야 수용. 실패하면 컷을 소비하지 않고 이월. 수용 시 소유 블록 인덱스를 전진시켜0pitch도 정확히 한 번 소비 |
| 실제 출력 | 같은 fit의 LinePlacement → document payload translate / TextPaint 셀 배치. height를 다시 축소하거나 좌표를 clamp하지 않음. 셀 마지막 줄의 기존 명시적 end-policy는 그대로 유지 |

TAC/anchor/rowspan 예약 알고리즘은 바꾸지 않았다. 일반 텍스트 본문 및 셀 경로의
같은 Lines 결과를 사용하며, inline TAC의 non-forward advance 제한을 풀지 않는다.
행 소유 단위를 바꾸거나 본문 높이·페이지 수를 맞추는 보정도 추가하지 않았다.

독립 대조군은 `tests/fixtures/issue7353_zero_pitch_review/`의 생성 코드·README로
재현한다. 원본 문단21/22/23/25(제목·단위·빈 줄·자료출처)를 추출하고 큰 표24는
**생략한 문단 간격 대조군**이다. 표 조판 개선 증거로 제출하지 않는다. HWPX의 저장
LineSeg는 모두 지운 뒤 MCP 한컴으로 HWP를 정상 저장하고 그 HWP에서 PDF를 생성했다.
`zero`는 원래0%, `normal`은 단위 문단만100%로 변경했다. 생성된 vpos는 각각
`[0,2340,2340,3380]` / `[0,2340,3340,4380]`HU다. PDF 자료출처 yMin은
93.042318pt /102.991445pt(양자화 포함)로 이동하고 제목·단위 위치는 같다.
이 정상 저장본은 손으로 만든 LineSeg 수용 근거가 아니다.

수정 전 `original-before.log`와 `saved-before.log`는 기존 실행파일로 원본 추출본과
한컴 정상 저장본 모두 해당0pitch를 거부한 결과다. `before.log`의 fresh 경계 계약은
`non-progressing text line`으로 실패했다. 초기 테스트 작성 중 타입 오류와 serializer의
section-control/마지막 문단 bit 차이를 잘못 전수 비교한 실패는 조판 결함 재현에서 제외했다.
현재 검사는 원본의 텍스트·스타일·저장 줄 메트릭 보존과 실제 최종 좌표를 직접 검사한다.

정식 `tests/cases/`에는 원본0pitch, 정상 한컴0/100% 위치 대조,20px 본문에서
12px 줄이 남은2px에 들어가지 않는 경계, 빈/보이는 줄의 각1회 소비, 셀11/12px
예산과3개 terminal정책, 한 문단의 동일 원점 저장2줄을 포함한다. 기존 negative-gap
반례는 이제 유효한−12 대신 역방향 전진이 되는−13을 검사한다. 허용치/페이지 수
baseline은 바꾸지 않았다. 원본 전체의 다음 진단은 문단24의
`shared text paint changes stored metrics`이며 아직 전체 V2 출력은 미검증이다.

Native에서 정상 저장 두 문서의 review와192dpi 동일 영역 확대를 직접 열어 제목·단위
위치와 자료출처의10pt 차이를 확인했다. 대체 글꼴 외형 차이는 남는다. 최종 fresh WASM,
대조군 무회귀와 메인테이너 시각 판정 결과는 아래에 이어 기록한다.

최종 집중 회귀는 기존18 harness에서 **294PASS/0FAIL**이다
(`tests-final-summary.log`, document-flow80/text34 포함). Native/WASM lib Clippy는
`clippy-native.log`/`clippy-wasm.log` 모두 exit0, 최종 fmt는 `fmt-final.log`다.
이는 내부 절편 검증이며 전체 workspace/CI 상당 검증이나 실제 Studio 수동 판정은
수행하지 않았다. Docker fresh WASM은7분20초/exit0(`docker.log`), SHA-256은
`d36ac12829eaff93c0df71e5b7c35600908126e4706c94e9a7d676917a7ba8c3`이다.

`REVIEW_CASE={zero,normal} node output/7353/r19/stored-body-rows/review.mjs --wasm`
결과 두 문서는 각각1쪽이며 Native/WASM JSON수치/기타 차이0, SVG동일이다.
각 run.json의 입력·PDF·WASM hash와 `source.sha256`으로 최종 제품 소스를 고정했다.
제품 코드 동결 후 추가한 테스트/문서만 변경했으며 빌드 후 제품 소스 hash도 확인했다.

동일 영역192dpi 확대와 Native/fresh WASM review, 대표 standalone overlay를 직접
열어 제목·단위 표시·자료출처의 위치를 확인했다.0%와100%의 후속 원점 차이는
1000HU(10pt)이며 빈 줄은 양쪽 모두 실제 줄로 남는다. 글꼴 굵기/폭과antialias 차이는
남아 full-page 내용 픽셀 proxy는0%10.39%,100%10.64%다. 자동 점수를 위치 판정이나
사람의 정확도로 해석하지 않는다. 표24는 자료에서 제외했으므로 표/전체 문서 통과를
의미하지 않는다. 메인테이너 시각 판정은 다음 자료로 요청한다.

- [두 간격 비교 확대](../../output/7353/r19/stored-body-rows/wasm-spacing-review.png):
  위0%/아래100%, 왼쪽한컴/오른쪽fresh WASM. 제목·단위가 같은 원점인 상태에서
  자료출처가 아래 대조군에서10pt 내려가는지 확인한다.
- [0% review](../../output/7353/r19/stored-body-rows/zero/wasm-review-1.png),
  [compare](../../output/7353/r19/stored-body-rows/zero/wasm-compare-1.png),
  [overlay](../../output/7353/r19/stored-body-rows/zero/wasm-overlay-1.png).
- [100% review](../../output/7353/r19/stored-body-rows/normal/wasm-review-1.png),
  [compare](../../output/7353/r19/stored-body-rows/normal/wasm-compare-1.png),
  [overlay](../../output/7353/r19/stored-body-rows/normal/wasm-overlay-1.png).
- 실제 입력/기준PDF는 `tests/fixtures/issue7353_zero_pitch_review/{zero,normal}-saved.hwp`,
  `{zero,normal}-2020.pdf`. Native 자료는 같은 output의 `native-*`다.

마지막 `controls.mjs`는 기존7대조군+Double4+직전장식3 ×2 cell-end정책 =28조합에서
이전 SVG 보존과 Native/WASM동일을 모두 확인했다(`controls-final.log`). 기본 엔진은
Legacy로 유지하며 push/PR/원격 상태를 변경하지 않았다. 다음 종단 차단점은 위에
기록한 원본 문단24의 저장 paint 메트릭 불일치다.

### CENTER 글줄 기준점 해석 — 원본 문단24의 다음 경계

작업지시자는 직전0% 줄간격 절편에 **시각 판정 통과**와 다음 진행을 승인했다.
기존 §5.1 실행 묶음 안에서 조사·구현·집중 검증을 계속한다. 아래 결과는 원본 전체나
R5 완료를 의미하지 않는다. 증적 디렉터리는 `output/7353/r19/stored-paint/`다.

원본 문단24의13×8 시장점유율 표에서 첫 셀 `순위`의 저장 줄은
height/textheight1200HU, baseline600HU다. `ParaShape.attr1`의20..21비트가2,
즉 HWPX `align@vertical=CENTER`다. 가로 가운데 정렬이나 셀 수직정렬과 다른 속성이다.
파서/IR/직렬화기는 보존하지만 `ResolvedParaStyle`은 이 값을 전달하지 않았다.
공통 배치의 `ensure_min_baseline`이600HU(8px)를12.8px로 올리면서, V2의
`stored_text::validate_paint`가 저장 결과와의 불일치를 검출했다. bbox/폭/줄 소속이
아니라 기준선 하나의 불일치였으며, `before-input.log`/`center-before.log`에 남겼다.

독립 근거를 먼저 확보했다. 원본 그대로 한컴 HWP 재저장 및 저장 LineSeg를 모두 지운
HWPX의 정상 재조판 저장본에도600HU가 나왔다. 단순 stale-cache로 볼 수 없다.
한컴 정상 저장 CENTER/BASELINE 대조군은 동일12pt 제목 셀에서600/1020HU를 각각
기록하지만 PDF 제목 셀 글자 위치는 같다.600은 글꼴 ascent가 아니라 가운데 정렬의
기준점이다. 그대로 paint한 임시 진단은 글자를 위로 올렸으므로 폐기했다. 이 진단용
`paragraph_layout.rs` 변경과 출력 logging은 제품 코드에 남기지 않았다.

| 값의 경로 | 이번 처리와 검증 |
| --- | --- |
| `ParaShape.attr1[20..21] → style_resolver` | `ParagraphVerticalAlignment`로 보존. Legacy 소비 동작은 바꾸지 않음 |
| `stored_text::localize → resolve_vertical_alignment` | 줄 partition/점유 높이/간격/원점은 보존. 동등한 resolved em의 CENTER 기준점을 공통 `frame_metrics_for_line`의 glyph baseline으로 해석. 원본 IR은 불변 |
| `TextComposer::compose → shared physical-frame paint → validate_paint` | 해석한 줄 메트릭을 실제 paint도 그대로 사용했는지 검사. 기존 불일치 검사를 완화하거나 생략하지 않음 |
| paint nodes → `ParagraphItem::Text → FlowCursor → fragment append` | 같은 최종 노드의 점유 높이와 원점을 측정/배치가 소비. 표/셀/후속 문단의 최종 bbox를 검사 |

이는 텍스트 기준점 지원이며 컷·rowspan·예약/이월·clipping·종료 알고리즘은 변경하지
않았다. 혼합 resolved em/첨자, TOP/BOTTOM, em과 맞지 않는 저장 CENTER 기준점은
명시적 미지원이다. fresh uniform-em 경로는 공통 재조판이 이미 glyph baseline을 만들므로
저장 CENTER 참조 변환을 이중 적용하지 않는다. 일반 BASELINE의 잘못된 낮은 값을
CENTER로 추정하여 고치는 예외는 추가하지 않았다.

시각 대조군은 `tests/fixtures/issue7353_center_reference_review/`에 생성 코드와
출처를 함께 보존했다. 원본p6 문단21~25(제목/단위/빈 줄/표 전체/자료출처)를 추출하되,
**모든 표·셀 테두리를 동일 실선으로 변경하고 zone을 비운 대조군**이다. 원본은 인접
테두리 충돌 때문에 실제 paint가 아직 거부된다. 이를 원본 통과나 테두리 해결로
보고하지 않는다. 두 대조군의 차이는 셀 문단 세로 CENTER/BASELINE뿐이다. 모든
LineSeg는 한컴에서 다시 만들었으며 HWP→PDF도 그 정상 저장본에서 생성했다.
MCP engine2020 / Hancom11.0.0.9136 / direct32bit / preprocessing none,
job 및 생성법은 fixture README와 `*-status.json`에 연결했다.

정식 테스트는 같은12pt 제목의 실제 glyph baseline1020HU, 전체 줄/표/셀 bbox와
뒤 자료출처, 텍스트 보존을 독립 정렬 불변식으로 검사한다. BASELINE의 낮은값은
거부, CENTER는 수용, 잘못된 CENTER 참조는 거부, source IR불변, fresh uniform-em
동등성과 mixed-em 미지원을 함께 잠갔다. 수정 전 실행파일은 CENTER를 명시적으로
거부하고 BASELINE은1쪽을 출력했다. 새 지원의 전/후 증거이지 이전에 그려진 문서의
회귀 복구 주장과 혼동하지 않는다. 초기 단독 rustc 명령의 roxmltree 인자 누락은
환경/하네스 실패이며 조판 결함 재현으로 세지 않는다.

18개 집중 하네스 중 새 계약은 통과했고, 유일한 실패는 다음 차단점을24로 기대하던
원본 admission 진단이었다. 생성 단계의 관측은 이제 문단35
`TAC row exceeds stored width`다. 진단을 관측값으로 갱신한 document-flow81PASS와
나머지 하네스 결과를 합하면 **297PASS/0FAIL**이다(`tests-summary.log`,
`document-final.log`). 기준 페이지수/허용치/ignore를 변경한 것이 아니다.
원본 문단24만 추출한 실제 paint의 인접 테두리 충돌도 별도로 남는다.

Native compare/review와192dpi 확대를 직접 확인했다. 셀 글자가 위쪽으로 올라가던
진단과 달리, 정렬 해석 후 제목/단위/표 각 행/자료출처의 위치를 비교할 수 있다.
대체 글꼴 폭·굵기와 기존 상대크기 처리는 이번 해결 주장에 포함하지 않는다. 특히
`하이트`는 독립 PDF의 CENTER/BASELINE간 yMin 차이가0.359607pt이며, 현재 공통
폰트 투영의 상대크기 차이가 남아 있다. 이 부분을 glyph 완전 일치로 판정하지 않는다.
전체 workspace/CI 상당 검증 및 Studio 수동 검증은 아직 수행하지 않았다.

최종 검증은 동일 source manifest(`source.sha256`, base `f73bd4c02`+이번 patch)로
수행했다. fmt, Native lib Clippy, WASM lib Clippy는 모두 통과했다. 표준 Docker
WASM 빌드는7분13초에 완료했고 `pkg/rhwp_bg.wasm` SHA-256은
`b5431433b9422d2ada4ddcab1f831e7bb4dce09370cee09a38ab67dfd2b31135`다.
`REVIEW_CASE=center|baseline node output/7353/r19/stored-paint/review.mjs --wasm`로
새 바이너리를 브라우저에서 실행했다. 두 대조군 모두1쪽이며 Native/fresh WASM
JSON 수치 차이0, 기타 차이0, SVG 동일이다(`*/backend-comparison.json`, `*/run.json`).
`controls.mjs`의 기존14대조군×2종 셀 끝 간격 정책 **28조합**도 이전 Native SVG를
보존하고 현재 Native/WASM SVG 및 예상 페이지수를 모두 일치시켰다(`controls-final.log`).
이는 집중 무회귀 증거이며 전체 CI 통과로 보고하지 않는다.

fresh WASM review·standalone overlay·확대 비교와 BASELINE 정상 대조군을 직접
열어 표 외곽/모든 행/뒤 자료출처의 위치와 누락 여부를 확인했다. 판정 대상은
글줄 세로 CENTER 기준점 해석이며 글꼴 폭·굵기·상대크기의 완전 일치가 아니다.

- 확대 비교: `output/7353/r19/stored-paint/wasm-center-detail.png`
- compare: `output/7353/r19/stored-paint/center/wasm-compare-1.png`
- standalone overlay: `output/7353/r19/stored-paint/center/wasm-overlay-1.png`
- review: `output/7353/r19/stored-paint/center/wasm-review-1.png`
- 정상 대조군 review: `output/7353/r19/stored-paint/baseline/wasm-review-1.png`

코멘트: 내용 픽셀 중심 자동 일치율 보조값 = 약14.89%.
높을수록 기준 PDF와 rhwp PNG가 더 비슷합니다.
낮은 값은 잉크 위치나 형태 차이의 검토 신호입니다.
사람의 판정 정확도가 아닌 자동 보조값입니다.

이번 절편의 사람 시각 판정은 대기다. Legacy 기본 경로·페이지수 golden·ignore는
변경하지 않았고 원격 push/PR도 하지 않는다. 다음 원본 경계는 표24의 공유 테두리
충돌과 문단35의 TAC 저장 폭 제한이며, 이번 대조군 통과로 해결된 것으로 간주하지 않는다.

### 공유 실선의 굵기 결합 — 원본 표24의 paint 경계

작업지시자는 CENTER 기준점 절편(`2134c34ca`)의 시각 판정을 통과시켰다.
다음 대상은 같은 표의 테두리다. `CellBorders::append`가 셀 소유 경계를 물리 조각의
topology interval로 수집한 뒤 `union`에서 스타일이 다르면 전부 거부한다. 원본 표는
제목 아래7개 구간에서 같은 검정 Solid의 width0/2(0.10/0.15mm)가 맞닿는다.
기존 한컴 정상 재저장 분리본 `stored-paint/table-saved.hwp`와 대응 PDF를 재사용한다.
이번에는 테두리를 통일하지 않는다. PDF trace(`shared-borders/reference-trace.xml`)의
y=671.386pt 경계에서0.24pt와0.48pt 검정 실선이 같은 중심선에 겹쳐 그려진다.
왼쪽 순위 열은0.24pt만 유지되고 제조사 열부터0.48pt가 겹친다. 따라서 동색 불투명
실선의 점유 합집합을 구간별 큰 굵기 하나로 표현할 수 있다. 다른 색/복선/서로 다른
선종의 우선순위까지 이 근거로 추정하지 않는다.

변경 값의 경로는 셀 BorderFill → `CellBorders::append`의 span → `union`의 구간별
스타일 → 공통 LineNode/ink bbox → SVG/Canvas다. 줄·표 측정과 pagination은 선 굵기를
소비하지 않으며 셀/표/뒤 문단 bbox를 바꾸지 않는다. 분할 시에도 기존 accepted physical
rectangles의 동일한 중심선에 적용한다. 컷·요구/예약 높이·이월·종료는 변경하지 않는다.
반례는 부분적으로만 겹치는 병합 셀 경계, 선언 순서 반전, 분할 조각, 색/선종 충돌이다.
정식 `same_color_solid_shared_edges_union_width_per_interval`는 수정 전 정상 컴파일 후
`conflicting shared V2 cell borders`로 실패했다(`shared-borders/before.log`).

#### 입력 유효성과 판정 범위

실제 출력에서 추출본 `table-input.hwp`와 그 입력을 직접 출력한 한컴 PDF 사이에 큰
위치 차이가 보였다. 이를 통과 이미지로 제시하지 않는다. 실패 출력은
`shared-borders/raw-review/`에, 입력/직접 PDF는
`tests/fixtures/issue7353_shared_border_review/`에 보존했다. 직접 HWP 재저장본도
host 저장 폭42520HU < 표46149HU로 별도의 TAC 수용 오류를 낸다. 이번 실선 수정으로
두 문제를 해결했다고 간주하지 않는다.

최종 판정 입력은 같은 표의 테두리·배경·문자/문단 속성을 유지하고 저장 LineSeg만
비운 HWPX를 한컴으로 정상 재저장한 `reflow-saved.hwp`와 대응 `reflow-2020.pdf`다.
이전 CENTER 조사에서 생성한 정상 대조군을 재사용했다. 생성 절차·MCP job·파일 해시와
HWPX 경유에 따른 한계는 fixture README에 기록했다. 원본 저장 줄 입력의 성공이나
원본 전체 문서 통과로 대체하지 않는다. 원본 PageDef와 정상 대조군은 같지만
host vpos20892→3380HU, 직접 HWP 저장/정상 대조군의 폭42520/48188HU 차이가 있다.
이 위치/폭 문제는 다음 진단 대상으로 남긴다.

#### 실행 증거

검증 source는 `2134c34ca` + 이번 patch이며, 생산 코드
`src/renderer/table_v2/borders.rs` SHA-256은
`d7ccc9b15ee6ae3482ab76b0f61bbd4c60049a9d3db93b097a02e35c0b671f35`다.
`shared-borders/source.sha256`와 fresh WASM `review/run.json`에 고정했다.

| 검증 | 결과 / 증거 |
| --- | --- |
| 수정 전→후 | 정식 공유선 구간 계약이 이전 코드에서 거부 오류로 FAIL, 수정 후 PASS (`before.log`, borders harness log) |
| 18개 정식 harness | 최초299 PASS 후 document harness를81→83건으로 확장·재실행하여 **301 PASS / 0 FAIL** (`tests-summary.log`, `document-final.log`) |
| 셀/분할/zone 경로 | 부분 공유선 굵기·선 끝점·잉크 bbox·선언 순서 반전·분할 뒤 내용 보존 검사. 다른 색 충돌은 여전히 거부 |
| 정상 한컴 저장본 | 13×8 표의 원래 공유선 구간0.32/0.64px와 뒤 자료출처 보존, 정상 종료 검사 |
| 포맷·lint | fmt check, Native/WASM32 library Clippy `-D warnings` PASS (`fmt.log`, `clippy-{native,wasm}.log`) |
| fresh WASM | Docker 빌드7분15초, WASM SHA `a66177444ede1194bac9868c1fdbf42dbf907cb1b41cd3bbf3de5929969794f8` (`docker.log`) |
| 대상 backend 비교 | SVG 동일, 구조 차이 없음, 좌표 부동소수 최대차2.28e-13 (`review/backend-comparison.json`) |
| 기존 대조군 | 14입력×2종 terminal policy=28조합32페이지, 기존 Native SVG 유지 및 fresh WASM SVG 동일 (`controls-final.log`) |

실제 호출 경로에 zone perimeter도 같은 `union`을 소비한다. 동일한 zone 스타일의
좌/우 실선 굵기가 서로 다른 인접 zone을 합성 계약으로 추가해 최종 공유선의 길이·굵기와
선언 순서 무관성을 검사했다. 배경 node의 선언 순서/ID가 아닌 실제 geometry/style을
비교한다. 이는 실선 합집합의 계약이며 한컴의 서로 다른 zone 스타일 우선순위 증거는 아니다.
기존 `zones::prepare`의 서로 다른 장식 충돌과 복선 junction 제한은 유지한다.

Native와 fresh WASM의 review·standalone overlay·확대 비교를 직접 열어 제목/단위,
공유선 구간, 모든 행/회색 배경, 외곽과 뒤 자료출처를 확인했다. 글꼴 외형·굵기·상대크기
차이는 남아 있다. 이번 사람 판정 대상은 **원래 테두리를 보존한 정상 대조군의 공유선**이다.

- 확대 비교: `output/7353/r19/shared-borders/wasm-border-detail.png`
- compare: `output/7353/r19/shared-borders/review/wasm-compare-1.png`
- standalone overlay: `output/7353/r19/shared-borders/review/wasm-overlay-1.png`
- review: `output/7353/r19/shared-borders/review/wasm-review-1.png`

코멘트: 내용 픽셀 중심 자동 일치율 보조값 = 약38.34%.
높을수록 기준 PDF와 rhwp PNG가 더 비슷합니다.
낮은 값은 잉크 위치나 형태 차이의 검토 신호입니다.
사람의 판정 정확도가 아닌 자동 보조값입니다.

이번 공유선 절편은 사람 시각 판정 대기다. 전체 CI/workspace all-targets와 Studio 수동
조작은 이번에 실행하지 않았다. Legacy 기본 경로·golden·ignore 변경, 원격 push/PR은 없다.

### 저장된 단일 TAC 객체의 줄 폭 초과 — 원본 문단35

작업지시자는 공유 실선 절편(`6e640239d`)을 시각 통과시켰다. 다음 원본 수용 경계는
문단35의 단일 TAC 표다. 표48083HU + 좌/우 바깥여백141HU씩 =48365HU가 저장 줄48188HU보다
크며, 표 뒤 UTF-16 위치8에는 편집자 공백이 남아 있다. 원본 PDF p7은 표를 축소하거나
지우지 않고 줄 시작에 배치한다(x≈58.05pt). 기존 `tac::object_rows`는 음수 free를
무조건 거부하므로 정상 저장된 단일 원자 객체를 수용하지 못한다.

같은 의미를 한컴으로 정상 저장한 대조군은 원본 문단33..36의 HWPX 분리본이다. 저장
LineSeg·PageDef·표/내용을 수동 보정하지 않았으며, 한컴 HWP 재저장 후에도 폭/높이와
후행 공백이 유지된다(`tac-overflow/sales-saved.hwp`). 원본 PDF p7과 이 저장본의
PDF가 독립 근거다. 앞 절편 HWP 직접 추출본의 위치 차이는 별개다: 실제 한컴 재저장 시
PageDef 좌우 여백이5669→8504HU로 바뀌어 본문 폭이 달라졌다. 원본의 저장 vpos도
분리 후 재계산되므로 원본 전체 배치를 이 추출본 위치에 맞추지 않는다.

변경 경로는 저장 줄의 `segment_width`와 단일 객체의 바깥여백 포함 advance →
`object_rows`의 정렬 offset → `compose`의 InlineTables 공통 rectangle → body FlowCursor의
동일 원점 → TableCursor의 자식 선언 폭 → paint다. 단일 객체 자체가 줄보다 큰 경우에만
남는 정렬 공간이 없다고 처리하며, 객체 폭/여백·후행 공백·세로 점유를 줄이지 않는다.
여러 객체나 선행 공백 때문에만 초과한 줄, 그림, 미검증 정렬은 종전처럼 거부한다.
중첩 셀의 물리 가용 폭 검사는 `TableContentPlan`에 별도로 남아 있어 본문 overhang 수용을
셀 밖 출력 허용으로 확대하지 않는다. pagination/세로 예약/종료는 변경하지 않는다.

#### 결과와 다음 미지원 경계

원본 문단35 및 `sales-saved.hwp`는 폭 검사를 통과한 뒤 셀72 문단0의
`Field(Formula, =SUM(ABOVE)??%g,;;100)`에서 거부된다. 필드를 삭제하거나 결과 문자열로
치환하지 않았다. 원본 전체/저도주 표의 최종 배치는 **미검증**이다. 정상 저장 추출본과
독립 PDF, 생성 절차·MCP job·해시는 `tests/fixtures/issue7353_tac_overflow_review/README.md`에
보존했다. 이 fixture의 테스트는 미지원 경계 검사이며 정상 조판 통과로 합산하지 않는다.

이번 실제 시각 판정 입력은 기존 `issue7353_shared_border_review/table-saved.hwp`와
`table-2020.pdf`다. 앞 절편의 `reflow-saved.hwp`와 다른 파일이다. Right 문단의 표46149HU와
좌우 여백282HU가 저장 줄42520HU보다 크지만 PDF는 x≈86.473pt에서 선언 폭을 보존한다.
이번 V2는1쪽을 출력하며 표 원점(8645,13441)HU, 크기(46149,23511)HU, 뒤 자료출처
원점 y37593HU를 검사한다. 마지막 값은 본문 시작9920+저장 줄 시작3380+높이23793+간격500이다.

실제 소비 경로: `tac.rs:object_rows` → `tac.rs:compose`의 InlineTables rectangle →
`body_flow`/`flow::FlowCursor`의 동일 객체 box → `TableCursor` → paint.
중첩은 `content.rs`의 InlineTables 물리 경계 검사도 소비하며, 세 정렬 모두 실제
`ContentBounds { row:0,column:0 }`를 검사한다. 일반 흐름과 중첩의 적용 범위를 혼동하지 않는다.
시작/끝 컷·예약 높이·이월·종료는 이 절편에서 변경하지 않았다.

검증 source는 `6e640239d` + 이번 patch. `tac.rs` SHA-256:
`48e23fad07cd8d788a64c850b32a5e363b5f9d2413f17bc2c6552b53d2eb9677`.
이하 로그/산출물의 공통 경로는 `output/7353/r19/tac-overflow/`다.

| 검증 | 결과 / 증거 |
| --- | --- |
| 수정 전→후 | 같은 정상 저장 시장 표가 이전 코드에서 폭 거부로 FAIL, 수정 후 실제 배치 계약 PASS (`before.log`, `document-final.log`) |
| 정식 focused 검사 | 17 harness218건 + 최종 document harness88건 = **306 PASS / 0 FAIL**. 전체 CI가 아니다 (`tests-summary.log`, `document-final.log`) |
| 경계 | 단일 Left/Justify/Right·후행 공백의 소유와 실제 위치·뒤 문단 원점·중첩 물리 경계. 여러 객체/선행 공백/후행 공백만의 초과/Center는 미지원 유지 |
| 포맷·lint | fmt check 및 Native/WASM32 library Clippy `-D warnings` PASS (`fmt.log`, `clippy-{native,wasm}.log`) |
| fresh WASM | Docker 빌드7분15초, SHA `766670ec83a1c1a82a5ee2fa49f691e4dae447b03819775f0aa5308b94c2fd4d` (`docker.log`) |
| 대상 backend | Native/fresh WASM SVG 동일, 구조 차이0, 부동소수 최대차2.28e-13 (`market/backend-comparison.json`) |
| 기존 대조군 | 14입력×2종 terminal policy=28조합32페이지. 이전 Native SVG 유지 및 fresh WASM SVG 동일 (`controls-final.log`) |

후행 공백의 합성 계약은 HWP 직렬화 경로를 쓴다. HWPX의 선두 control-slot 축 변환 및
HWP 첫 문단의 SectionDef 삽입을 별개 입력 조건으로 확인하고, 별도 선행 문단을 둔
유효한 계약으로 구성했다. 문자열 소유 축이나 terminal 정책을 생산 코드에서 완화하지 않았다.
이는 합성 계약이지 조합별 한컴 출력 일치 증거가 아니다.

Native/fresh WASM의 compare·review·standalone overlay·확대 PNG를 직접 열어
왼쪽 시작, 전체 폭, 모든 행/외곽, 뒤 자료출처를 확인했다. 표를 축소하거나 잘라 숨기지 않는다.
글꼴 외형·굵기·자폭의 기존 차이는 남아 있다.

- 확대 비교: `tac-overflow/wasm-width-detail.png`
- compare: `tac-overflow/market/wasm-compare-1.png`
- standalone overlay: `tac-overflow/market/wasm-overlay-1.png`
- review: `tac-overflow/market/wasm-review-1.png`

코멘트: 내용 픽셀 중심 자동 일치율 보조값 = 약40.44%.
높을수록 기준 PDF와 rhwp PNG가 더 비슷합니다.
낮은 값은 잉크 위치나 형태 차이의 검토 신호입니다.
사람의 판정 정확도가 아닌 자동 보조값입니다.

전체 CI/workspace all-targets와 Studio 수동 조작은 미실행이다. Legacy 기본 경로·golden·ignore
변경과 push/PR은 없다. 사람 시각 판정 대상은 정상 저장 시장 표이며, 다음 구현 대상은
원본35번 문단의 수식 필드 보존이다. 부분 수용 개선을 A/R5 완료로 판정하지 않는다.

### 저장 수식 필드 결과의 셀 조판

작업지시자는 `af88cb78f`의 시각 판정을 통과시켰다. 다음 대상은 동일 원본35번 문단 및
정상 저장 `sales-saved.hwp`의 셀72(행8/열8)다. IR text는 `100.0`, 필드 범위는
문자0..3(`100`), 후행 `.0`은 필드 밖 일반 텍스트다. 원본 PDF p7과 이미 획득한
`sales-2020.pdf`의 같은 셀이 독립 표시 근거이며 다시 변환할 필요가 없다.

위반 규칙은 저장된 계산 결과를 가진 문단을 객체 배치가 필요한 컨트롤처럼 거부하는 것이다.
수식 계산/편집 갱신은 이번 범위가 아니다. 저장 LineSeg·문자 축·짝이 맞는 비중첩 필드 범위를
검증한 결과만 공통 TextComposer에 그대로 전달한다. 필드/범위를 삭제하지 않고 같은
paragraph/줄 구성의 `TextLine`을 측정·FlowCursor·paint가 소비한다. 필드 마커는 폭을
갖는 개체가 아니며 명령 문자열을 표시 문자열로 바꾸지 않는다.
반례는 누락/빈 결과, 고아 종료, 잘못된 참조·중첩/겹침, 슬롯 축 불일치, 다른 필드 타입,
편집으로 무효화된 저장 줄이다. 새로운 pagination·행 높이/clip 예외는 추가하지 않는다.

구현은 `table_v2/fields.rs::stored_formula_result`가 저장 필드의 짝과 UTF-16/8칸 제어
슬롯 축을 검증하고 `ir::bind_table`·`IrTextComposer::compose`·`TextComposer::compose`가
이를 수용하는 형태다. 대상은 비중첩 Formula 필드만 있는 유효한 저장 문단이다.
다른 필드, 객체와 섞인 문단, 편집 후 재계산은 수용을 확장하지 않는다.

실제 소비 경로는 `text.rs:compose`의 저장 줄 localize → 공통 `compose_paragraph` →
`layout_composed_paragraph_in_frame`의 최종 TextLine → `ParagraphItem::Lines`와 paint payload
→ 기존 flow/fragment → `TextPaint::append`의 동일 payload 이동이다. Formula 마커가 별도
개체 폭이나 paint를 만들지 않는 공통 composer/layout 경로를 확인했다. 필드 범위·명령을
지워서 텍스트로 위장하지 않으며 측정 뒤 다른 필드 원점을 선택하는 분기도 추가하지 않았다.
시작/끝 컷·예약 높이·이월·clip·이어받기는 이번 변경 대상이 아니다.

정상 저장본의 독립 값은 셀4434×2048HU, 표 공통 좌우 여백510HU, 저장 줄 폭3412HU,
줄 높이1200HU다. 문단은 **가로 Right**, 셀은 **세로 Center**이므로 실제 TextRun의
오른쪽 끝 = 셀 왼쪽+(510+3412)/75, y = 셀 위+(2048-1200)/150으로 검사한다.
글꼴 advance를 기대값으로 고정하지 않는다. 필드 결과100과 후행문자.0의 보존, 모든 셀,
자료출처가 표 아래에 한 번만 배치되는 것, 계산 명령 미출력과 마지막 종료도 검사한다.

검증 source는 `af88cb78f` + 이번 patch이며 `fields.rs` SHA-256은
`d3186220f7543db575c7fbc95ddbeba8a0ef8b4ad94850a5b092d0b71b7af3f6`이다.
전체 table_v2 source manifest와 입력/PDF/WASM 해시는 `formula/review/run.json`에 있다.
이하 경로의 공통 prefix는 `output/7353/r19/`다.

| 검증 | 결과 / 증거 |
| --- | --- |
| 수정 전→후 | 동일 정상 저장 sales 입력이 기존 코드의 paragraph2 non-table control로 FAIL, 수정 후 실제 문서 배치 PASS (`formula/before.log`, `formula/document-final.log`) |
| focused | 17 harness218건 + 최종 document92건 = **310 PASS / 0 FAIL** (`formula/tests-summary.log`, `formula/document-final.log`). 초기 테스트 코드 컴파일/잘못된 가로 Center 가정은 원문 Right 속성 확인 후 바로잡았으며 검출 증거로 세지 않는다 |
| 반례 | 13개 손상/편집 입력이 정확한 필드 검증 오류로 거부, ClickHere 미지원 유지. 여러 Formula+일반문자 합성 축 계약은 실제 preview 출력과 종료 검사. 합성 계약은 한컴 출력 근거가 아님 |
| 포맷·lint | fmt check 및 Native/WASM32 library Clippy `-D warnings` PASS (`formula/fmt.log`, `formula/clippy-{native,wasm}.log`) |
| fresh WASM | Docker7분12초, SHA `0e0391387d8e77a1b9452ede5f9cb5f12994f44da68353ca80d2214effd3bf2b` (`formula/docker.log`) |
| 대상 backend | Native/fresh WASM SVG 동일, 구조 차이0, float 최대차2.28e-13 (`formula/review/backend-comparison.json`) |
| 기존 대조군 | 14입력×2정책=28조합32페이지 + 직전 시장 표2정책2페이지. 이전 Native SVG 보존 및 fresh WASM SVG 동일 (`formula/controls-final.log`, `formula/market-control.log`) |

재현 명령은 `node formula/tests.mjs`, 최종 document harness 직접 컴파일·실행,
`docker compose --env-file .env.docker -p rhwp run --rm wasm`,
`pdftoppm -r 96 -png -singlefile <sales-2020.pdf> <formula/review/hancom-1>`,
`node formula/review.mjs --wasm`, `node formula/detail.mjs`,
`node formula/controls.mjs`, `node formula/market-control.mjs`다
(`formula/`는 위 공통 prefix 아래). 브라우저 동시 실행 중 추가 시장 대조군 launch가
한 번 실패해 기존 대조군 종료 뒤 단독 재실행하여 통과했다. 제품 실패로 합산하지 않는다.

Native와 fresh WASM의 review·standalone overlay·확대 비교를 직접 열어 마지막 행100.0,
표 외곽, 앞 제목과 뒤 자료출처를 확인했다. 글꼴 외형·굵기 차이는 남아 있다.

- 확대 비교: `formula/review/wasm-table-detail.png`
- compare: `formula/review/wasm-compare-1.png`
- standalone overlay: `formula/review/wasm-overlay-1.png`
- review: `formula/review/wasm-review-1.png`

코멘트: 내용 픽셀 중심 자동 일치율 보조값 = 약11.20%.
높을수록 기준 PDF와 rhwp PNG가 더 비슷합니다.
낮은 값은 잉크 위치나 형태 차이의 검토 신호입니다.
사람의 판정 정확도가 아닌 자동 보조값입니다.

**원본 전체는 미완료다.** 원본 #6923의 준비는 성공하지만 첫 physical page를 출력한 후
두 번째 physical page(0기반 `page:1`)에서 `DoesNotFit`, 요구 높이4148.68px가 발생한다.
`formula/admission/6923-terminal-admission.txt`가 증거다. 기존 거부 검사 갱신은 수용 경계
진단의 갱신이지 page-count golden 변경이나 전체 시각 통과가 아니다. 다음 대상은 이 배치
실패의 실제 컷·요구 높이·가용 예산 소비 경로다. 이번 판매현황 표의 사람 시각 판정은 대기한다.
전체 CI/workspace all-targets, Studio 수동 조작과 수식 편집 후 재계산은 미실행이다.
Legacy 기본 경로·golden·ignore 변경 및 push/PR은 없으며 R5 완료로 판정하지 않는다.
