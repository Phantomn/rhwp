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
