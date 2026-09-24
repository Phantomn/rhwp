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
