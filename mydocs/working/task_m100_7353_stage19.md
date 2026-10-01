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

### 원본 부모 셀의 페이지 초과와 이어받기 — 진행 중

작업지시자는 `50823731a`의 판매현황 표 시각 판정을 통과시켰다. 다음 대상은 원본 #6923
문단5/control0의 1×1 부모 표다. TAC가 아닌 TopAndBottom/RowBreak이며 셀의 87개 문단과
자식 표를 구성하면 4144.91px, 바깥여백 포함 요구량은 4148.68px다. 기존은 BetweenRows를
항상 셀 내부 분할 금지로 처리하여 두 번째 physical page에서 수용하지 못한다.
원본 대응 `148738070_wrapper_table_stored_page_frame-2020.pdf`의 실제 1·2쪽을 열어,
같은 부모 셀이 페이지를 넘어 이어지는 독립 출력 근거를 확인했다. PDF는 총7쪽이다.

후보 규칙은 새 페이지에도 들어가지 않는 Top 정렬 단일 행만 구성된 줄/개체 유닛으로
분할하는 것이다. 현재 남은 예산과 새 페이지 용량을 구분한다. 새 페이지에는 들어가는 행은
기존 통째 이월을 유지한다. Never, rowspan 연결 그룹과 TAC 한 줄 그룹의 원자성은 유지한다.
RowBreak enum/파서나 Legacy 규칙은 변경하지 않는다. 일반성/시각 검증 전 후보 상태다.

소비 경로: `BodyCursor::fit`이 본문 fresh-page 높이를 전달 →
`FlowCursor::fit_with_page_height`가 반복되는 자식 상단 밴드만 차감 →
`TableCursor::fit_rows`에서 행 점유 높이와 fresh capacity를 비교 → 셀 FlowCursor가
실제 수용한 줄·자식 조각을 반환 → 그 조각 높이를 부모 예약에 반영 → `TextPaint::build_node`
는 동일 셀/자식 bounds를 사용한다. padding/빈 밴드는 기존 FlowBlock과 minimum_left로
한 번 소비하고, paint에서 컷이나 높이를 재계산하지 않는다. 반복 제목과 Top 이외 세로 정렬은
이번 내부 분할 확장에서 제외하여 기존 원자적 계약을 유지한다. rowspan 연결 그룹도 내부
분할 미지원이므로 기존 원자적 경로로 남는다.

첫 실행은 내부 TAC 표의 마지막 행에서 `InconsistentAtomicPlan`을 드러냈다. 자식 높이는
444.96px인데 group합에 페이지 원점을 먼저 더한 비교가 마지막 그룹을 거부하여273.55px만
반환했다. `row_groups`는 같은 시작 행부터의 local prefix를 측정·수용·최종 예약에 공유하도록
수정 중이다. 허용 오차나 높이 clamp를 추가하지 않는다. 진단 출력은 제거했다.
증적 경로는 `output/7353/r19/inline-split/`이며 아직 완료/시각 판정 자료가 아니다.

후속 Native 진단에서는7쪽 전체 종료가 가능해졌다. 그러나 비교 이미지를 직접 열어2쪽의
첫 제목이 위로 당겨지고,3쪽에 한컴4쪽의 소제목이 먼저 놓이는 차이를 확인했다. 원본 부모
셀 문단6은 공백 두 글자, 저장 vpos0·높이1500HU·줄간격316HU이며, 앞 문단5의 마지막
줄은 vpos48210HU·높이1100HU다. 현재 연속 흐름은 이 공백 문단을1쪽 끝에 수용한다.
원본 PDF2쪽에서는 이 공백 뒤 제목이 시작한다. 명시적 raw_break_type은0이므로 vpos
되감김만으로 강제 쪽나눔을 만들지 않는다. 저장 프레임의 유효성/공백 소유를 별도 확인해야 한다.
이는 글꼴 외형 차이가 아니며,7쪽 종료를 독립 PDF의 피델리티 통과로 승격하지 않는다.

부분 셀의 gradient/대각선을 온전한 셀처럼 다시 늘려 그리지 않도록 실제 조각의 partial
상태를 paint까지 전달하고 미지원 장식은 실패로 유지한다. 반복 제목54px 계약과 세로 정렬
89px 부족 계약을 처음 확장이 바꾼 것은 focused 검사에서 검출했으며, 기대값을 바꾸지 않고
확장 범위를 위 미검증 경로에서 제외했다. 작은 일반 행의 이월, 초과 행의 줄 소유/최소 물리
밴드 보존, Never/용량 미지정 경로, 장식 거부와 재시도의 상태 보존을 정식 검사에 추가했다.

`roundoff-before.log`는 row_groups 수정만 되돌린 통제 실행에서 정상 원본 내부 표의
최종 행 누락을 검사한 FAIL이다(빌드 성공 뒤 assertion 실패). 같은 테스트는1HU 부족 대조군도
포함한다. 원본 부모 행의 수정 전 명시적 거부 증거는 기존 `formula/admission/6923-terminal-admission.txt`
를 재사용한다. 무변경 Legacy, baseline/golden/ignore와 source 샘플은 그대로다.

현재 후보 검증(source `50823731a` + 작업 diff):

| 항목 | 결과 / 증거 (`output/7353/r19/inline-split/` 아래) |
| --- | --- |
| focused 정식 cases | 19 harness **315 PASS / 0 FAIL**, `tests-final-summary.log` |
| 내부 행 누락 반례 | 수정 전 FAIL `roundoff-before.log` → 수정 후 PASS `issue_7353_rowspan_roundoff.log`;1HU 부족·여러 원점 대조 포함 |
| Native 원본 | 변경하지 않은 #6923 입력 **7쪽 출력 후 종료**, `original-final.log`, `review/actual/native.json`. 각 부모 조각의 본문 경계·셀 내 줄 경계 검사 PASS; PDF 위치 일치 판정은 아님 |
| lint | fmt check, Native/WASM32 library Clippy `-D warnings` PASS, `fmt-final.log`, `clippy-{native,wasm}-final.log` |
| Native 비교 | 최종 코드 재출력/재캡처 `review/native-{compare,overlay,review}-{1..7}.png`.2쪽 위치 차이를 다시 직접 확인; 판정 보류 |

최종 `fragment.rs` SHA-256은 `4e00546e1e33963e5eccc6c8331a4f85cc476d7992b43892812d62ae3ad2c1e2`,
`fragment/row_groups.rs`는 `8f0faf8c2ec6e7778a718955106e1286decf4ea7a42090175349b6aa0234bdf1`다.
전체 source manifest·입력/PDF 해시는 `source.sha256`, `review/run.json`에 있다.
실행 명령은 `node output/7353/r19/inline-split/tests.mjs`, library build/Clippy,
`inline-split/probe <원본HWP> inline-split/review/actual`,
`pdftoppm -r 96 -png <대응PDF> inline-split/review/hancom`,
`node output/7353/r19/inline-split/review.mjs`다.

**시각 완료 후보가 아니므로 fresh WASM/Docker·전체 CI·새 사람 승인 요청은 보류**한다.
기존 `pkg`는 직전 승인된 수식 절편 빌드이며 이번 후보와 같다고 보고하지 않는다.
다음 장애물은 저장 프레임 경계에서 공백 문단/후속 제목의 소유다. 겹치는 줄 상자의 끝만
비교해 페이지 경계를 추정했던 Legacy 예외를 복제하지 않고, 정상 저장 프레임의 유효성을
독립 출력 및 반례와 함께 검증해야 한다. 아직 커밋하지 않은 내부 작업 diff로 유지한다.

### 저장 셀 프레임 경계와 빈 문단 소유 — 시각 확인 후보

작업지시자의 다음 절편 승인으로 위 장애물을 처리한다. 입력은 수정하지 않은 원본 #6923
HWP이며 독립 기준은 같은 fixture의 `148738070_wrapper_table_stored_page_frame-2020.pdf`다.
PDF2쪽의 제목 앞 공백과 원본 셀 문단6의 저장 메트릭을 근거로 한다. 문단6의 공백 두 글자는
1쪽에서 먼저 소비할 내용이 아니라2쪽의 첫 줄이다. 높이1500HU(20px), 줄간격316HU
(4.213333px), 뒤 제목의 상단 여백141HU를 그대로 보존한다. PDF 페이지 수7 자체를
분할 조건이나 성공 기준으로 사용하지 않는다.

규칙과 소비 경로:

- `stored_text::cell_frame_starts` → `ir::bind_table`: 저장 줄 구성이 유지되는 셀 story에서
  문단 첫 줄 원점이0으로 되돌아가는 경계를 다음 문단 앞 `StoredFrameStart`로 보존한다.
  줄의 **끝점 겹침**은 경계가 아니다. 모든 원점이0인 문단들도 경계로 만들지 않는다.
  실제 원본에서 해당하는 셀 문단은6·29·48·71이다.
- 누락 LineSeg, dirty text partition, 원본 vpos 투영 snapshot, layout-only suffix가 있으면
  저장 프레임을 재사용하지 않는다. custom/reflow composer도 기본적으로 재사용하지 않는다.
  각 문단의 기존 저장 폭·줄 분할·메트릭 admission은 유지한다. 비0 되감김과 문단 내부
  되감김은 이번 범위에서 명시적 미지원이며 authored page break로 변환하지 않는다.
- `TableCursor::fit_rows` → `FlowCursor::fit_cell_until`: 셀 내부 분할 경로만 프레임 경계를
  소비한다. 앞 내용 뒤 경계를 만나면 marker와 후속 빈줄을 다음 continuation에 남긴다.
  새 조각의 시작 marker 자체는0높이 메타데이터이며 빈줄을 대신하지 않는다.
  Never·통째 수용되는 일반 RowBreak 행은 저장 경계로 강제 분할하지 않는다.
- 앞 프레임의 남은 물리 높이는 현 조각에 예약한다. 첫 후보처럼 외곽을 ink 끝으로 줄이지
  않는다. 자식이 전달받은 예산 전체를 쓴 경우 Flow/Table/Body fit이 같은 예산 끝점을
  전달하여 뺄셈·재합산 roundoff를 제거한다. 허용 오차 확대나 paint clamp는 추가하지 않는다.
- `TextPaint::build_node`는 fit의 셀 bounds와 실제 줄 소유·위치를 소비한다. 빈줄을1쪽에
  복제하거나2쪽에서 생략하지 않으며 paint에서 프레임/높이를 다시 추정하지 않는다.

정식 검사 `tests/cases/issue_7353_stored_cell_frames.rs`는 원본의 빈줄이2쪽에 정확히 한 번
존재하는지,20px 높이와 제목까지의 저장 간격,1~4쪽 부모 표가 본문 하단까지 점유하는지를
실제 render tree에서 검사한다. 추가 합성 계약은 경계에서 빈줄/후속 줄 소유, 부족 예산에서
재시도 불변성, 정확한 예산 끝점, 마지막 유닛 뒤 종료, intact Never/RowBreak 비적용,
음수 줄간격에 의한 상자 겹침, local0 원점 반복, fresh composer 비적용을 확인한다.
합성 계약은 추가 한컴 문서와의 일치 증거가 아니다.

통제 수정 전 실행은 IR marker 연결만 끈 상태로 빌드 후 원본 빈줄의 page index가0으로
검출되어 FAIL(`contract-before.log`)했다. 복원한 수정 후 같은 소유 검사는 PASS이며,
그 후 물리 하단 예약 assertion과 fresh composer 대조까지 포함한 최종5개 검사가 PASS다.
다른 focused cases315개도 PASS다. 전체 CI/workspace all-targets 및 Studio 수동 편집은
미실행이며 테스트/golden/ignore의 기대 기준을 완화하지 않았다.

검증 source는 `50823731a` + 현재 작업 diff이며 증적은 `output/7353/r19/cell-frames/`다.
`source.sha256`과 `review/run.json`에 source manifest·원본 HWP·기준 PDF hash를 고정했다.
Native 원본은7쪽 출력 후 정상 종료했고 Native/WASM32 library Clippy `-D warnings`와
fmt check가 통과했다. focused20 harness319 PASS 뒤 같은 최종 production source에
fresh composer 대조1개를 추가하여 해당 harness5개를 재실행했다(총 고유320 PASS).
`tests-final-summary.log`, `contract-after.log`, `native-final.log`,
`clippy-{native,wasm}-final.log`가 실행 증거다.

실행은 `node output/7353/r19/cell-frames/tests.mjs`, 공유 target의 library build/Clippy,
`cell-frames/probe <원본HWP> cell-frames/review/actual`,
`node output/7353/r19/cell-frames/review.mjs`다. 실험 DocumentV2용 기존 직접 Chrome
capture/comparison 경로로 Native compare·standalone overlay·review를 새로 생성했다.
기본 CLI/Studio의 Legacy 출력으로 V2 증거를 대체하지 않았다.1~7쪽을 열어2쪽 빈줄 뒤 제목 위치와
페이지별 내용 소속 개선을 직접 확인했다. 글꼴 외형·테두리 굵기·1쪽 로고의 기준 PDF 차이는
남으며,4쪽 부모 외곽 하단은 기준 PDF보다 아래에 있다. 이번 경계/빈줄 소유 개선을
표 외곽까지 포함한 PDF 완전 일치로 판정하지 않는다. 변경 전후 TextRun1098개의 텍스트
다중집합은 동일하다(페이지별 위치/순서 일치의 증거는 아님).6·7쪽도 앞 흐름 변경의 영향을
받았으므로 비교 이미지를 직접 확인했으며 변경 없음으로 분류하지 않는다.

Docker fresh WASM 빌드/직접 Chrome 출력 비교까지 완료했다. 기본 compose는 네트워크
pool 부족으로 시작하지 못하여 기존 `rhwp` project network/cache를 사용했다. 물리 예약
수정 전 후보 빌드는 중단했으며 최종 코드는 `docker-final.log`의 별도 빌드로 검증했다.
`docker compose --env-file .env.docker -p rhwp run --rm wasm`는7분10초에 성공했다.
그 뒤 `node output/7353/r19/cell-frames/review.mjs --wasm`으로 실제 Chrome의 새
`DocumentV2`에서7쪽을 출력하고 정상 종료했다. Native와7쪽 SVG가 모두 동일하며 render
tree의 비수치 차이0, 최대 수치 차이는2.274e-13이다(`review/backend-comparison.json`).
새 WASM2쪽 review와 standalone overlay를 직접 열어 첫 공백 및 제목 위치를 확인했다.
한컴 대비2쪽 잉크 영역 자동 일치율22.32%는 글꼴·테두리 차이도 포함한 보조값이지 사람의
최종 판정 정확도가 아니다. 대표 자료는 `review/wasm-{compare,overlay,review}-2.png`이고
1~7쪽 모두 생성했다. WASM SHA-256:
`6dd21d40c61791c76242e367c78765286873d26fb8dad8f81b6a1925c45667ec`.
2쪽 첫 공백 소유/저장 간격의 사람 판정을 요청하며,4쪽 외곽 하단 차이는 남은 검토로 유지한다.
Legacy/Studio 기본 엔진 전환·원격 push·PR·R5 전체 완료는 이번 절편에 포함하지 않는다.

### 부모 셀 첫 빈 문단 원점 재검증 — 앞선 해결 주장 정정

작업지시자는 하단이 아니라 부모 표 시작의 첫 빈 문단 문제임을 지적하고 추적·검증 보정을
승인했다. 앞선 빈줄/제목 사이 상대 간격과 Native/WASM 일치만으로 해결됐다는 주장을 철회한다.
빈줄의 존재와 높이가 맞더라도 셀 기준 원점이 잘못되면 뒤 내용까지 함께 이동한다.

이번에는 원본 `section0/paragraph5/control0/cell(row0,col0)`를 먼저 고정하고 물리1쪽의
첫 문단0과 물리2쪽의 첫 문단6을 모두 대조했다. 문단0은 빈 문자열·높이1400HU·간격0이며,
현재도 부모 셀 상단+141HU에서 시작한다. 문단6은 공백 두 글자·높이1500HU·간격316HU다.
문단6 자체가 사라진 것이 아니라 **저장 프레임 이어받기 때 셀 상단 안여백141HU가 빠진다**.
원본/서식은 변경하지 않았다.

| 물리2쪽 기준,96dpi | 수정 전 | 수정 후 | 근거 |
| --- | ---: | ---: | --- |
| 부모 셀 상단 |98.266667|98.266667|동일 최종 cell bounds |
| 첫 빈줄 상단 |98.266667|100.146667|셀 안여백141HU=1.88px |
| 빈줄 높이 |20|20|저장 LineSeg1500HU |
| 다음 제목 carrier 줄 상단 |122.48|124.36|빈줄 높이+줄간격316HU |
| 제목 자식 표 상단 |124.36|126.24|carrier 원점+개체 상단여백141HU |

독립 PDF2쪽 vector path는 부모 상단98.292px, 분홍 제목 상단126.103px다.
`mutool draw -F trace -o output/7353/r19/leading-blank/reference-trace.xml <대응PDF> 1-2`
후 path transform을96dpi로 환산했다. 텍스트 bbox를 빈줄 bbox로 간주하지 않았다.
PDF gap27.811px와 원본 속성 합 `(141+1500+316+141)/75=27.973333px`를 대조하여
프레임 상단 여백 누락을 판별했다. PDF좌표에 맞춘 상수 보정은 추가하지 않는다.

실제 경로는 `content::from_flow_grid`가 최초 셀 padding을 Space로 한 번 구성 →
`ir::bind_table`의 저장 프레임 marker → `FlowCursor::fit_cell_until`의 marker 이어받기다.
기존은 최초 padding Space를 이미 소비한 상태에서 marker만 건너뛰어 첫 줄을 셀 상단에 놓았다.
수정은 저장 프레임 시작에서만 **동일 cell.padding.top**을 물리 예산과 pen에 포함한다.
`fragment::fit_rows`의 content_origin도 같은 시작 상태를 사용한다. 실제 paint는 fit이 반환한
line.bounds를 translate하므로 paint 단계에서 보정하거나 빈줄을 추가하지 않는다.
일반 용량 컷은 새 저장 프레임이 아니므로 padding/빈줄을 재생하지 않는다.

첫 유닛을 수용하지 못하면 marker/inset을 함께 미소비 상태로 유지하고 요구 높이에 여백을
포함한다. 여백만 소비한 빈 페이지를 내보내지 않는다. 이 실패 시 복원을 모든0진행 경로에
적용한 첫 후보는 기존0높이 행의 종료/테두리 거부 계약을 깨뜨렸다. focused에서 검출하여
복원을 실제 저장 프레임 시작 경로에 한정했고 기대값은 바꾸지 않았다.

추적 실패의 원인은 검사의 기준점 누락이다. 기존 `original_blank_line...`은 서식과 텍스트로
문단6을 찾고 제목까지의 상대 간격을 검사했으나 **부모 셀 상단→첫 줄 상단**은 검사하지 않았다.
V2 paint가 셀 내부 TextLine의 section/para 인덱스를 지우는 것도 조사 가시성의 제약이다.
이번에는 부모 Table 경로와 첫 TableCell/첫 TextLine을 구조적으로 함께 검사하고,
`leading-blank/detail.mjs`로 원본 경로·빈줄/후속 줄/자식 표의 전후 좌표를 기록한다.
이는 이번 검사의 보강이며 범용 IR→render tree 소유 추적 API를 완성했다고 주장하지 않는다.

검증 증적은 `output/7353/r19/leading-blank/`다. 원본 검사에 부모 기준 원점을 추가한
`before.log`는 변경 전 코드에서2쪽 cell.y=line.y로 FAIL했고, 수정 후 정식6개 경계 검사에서
PASS했다.5px inset 합성 대조는0·4·14px 예산에서 요구15px로 실패,38px 예산에서 정확 수용,
18px 뒤의 일반 용량 컷에서 padding 미반복·유닛 보존·최종 종료를 확인한다.
20 focused harness **321 PASS /0 FAIL**(`tests-final.log`), 원본7쪽 정상 종료
(`native-final.log`)를 확인했다. 합성 대조의 통과는 별도 한컴 입력의 피델리티 증거가 아니다.
최종 source는 `50823731a`+기존 WIP 포함 현재 diff이며 `source.sha256`으로 고정한다.

Native compare/overlay/review를 재생성하고2쪽을 직접 확인했다. 원점 보정은 제목 앞 공백부터
후속 내용에 적용되며1쪽 첫 빈 문단은 변경하지 않는다. 확인 영역을 명확히 하기 위해
`first-blank-native.png`에는 같은2쪽 상단의 PDF/수정 전/수정 후 확대와 rhwp 실제 빈줄 상자
주석을 분리했다. PDF에 rhwp의 추정 빈줄 상자를 덧씌우지 않는다.
최종 Native/WASM32 library Clippy와 fmt/diff 검사를 통과했다. Docker fresh WASM은
`docker compose --env-file .env.docker -p rhwp run --rm wasm`으로7분11초에 성공했다
(`docker-final.log`). `node output/7353/r19/leading-blank/review.mjs --wasm`으로 실제
Chrome의 새 DocumentV2에서7쪽을 출력했고 Native와7쪽 SVG가 모두 동일했다.
render tree 비수치 차이0, 최대 수치 차이2.274e-13이다(`review/backend-comparison.json`).
WASM SHA-256: `5a770be7976821324221bdd61b9515a3f52bae2c7bd624e57e3870200657570c`.
source.sha256도 재대조하여 테스트·lint·시각 출력 사이 production source 변경이 없음을 확인했다.

Native 영향3~5쪽 review와 fresh WASM2쪽 review/standalone overlay/확대 자료를 직접 열었다.
`node output/7353/r19/leading-blank/detail.mjs --wasm`의 `trace-wasm.json`과
`first-blank-wasm.png`는 셀 상단→빈줄→제목의 최종 좌표와 동일 영역 전후 출력을 연결한다.
2쪽 자동 잉크 일치율24.11%는 보조값이며 사람 판정 정확도가 아니다.
1쪽 첫 빈 문단은 이미 올바른 원점이므로 변경하지 않았다. Native1·6·7쪽 SVG는 이전
cell-frames 후보와 byte 동일하고,7쪽 전체의 페이지별 텍스트 순서도 유지된다.
3·4쪽 부모 외곽 하단 등 다른 차이와 전체 CI는 이번 첫 빈 문단 원점 검사로 해결/완료
처리하지 않는다. 이번 자료는 해당 상단 원점에 대한 사람 시각 판정 대기이며,
Legacy/Studio 기본 엔진 전환·원격 push·PR·R5 전체 완료를 수행하거나 주장하지 않는다.

### 2026-09-27 — 상단 빈 문단 승인, 부모 조각 하단의 독립 재검증

작업지시자가 직전 `leading-blank/first-blank-wasm.png`의 시각 판정을 통과시키고 다음
절편을 승인했다. 승인 범위는 부모 셀 상단→첫 빈줄→제목의 원점이며, 앞서 명시한 하단
외곽 차이나 전체 R5 완료로 확대하지 않는다. 같은 stage19에서 계속한다.

다음 대상은 저장 프레임 종료 시 부모 조각의 물리 하단이다. `bug-hunter`의 독립 기준과
실제 호출 경로 대조 절차를 적용하되, 일반 CLI는 Legacy이므로 승인된 실험 DocumentV2
probe/Chrome 출력의 기존 정확한 source 증거를 사용한다. 새 CLI나 Legacy 대체 출력을
만들지 않는다. 원격 이슈 생성은 하지 않고 발견을 #7353의 이 기록에 연결한다.

`output/7353/r19/frame-extent/audit.mjs`가 직전 source manifest와 원본/PDF 해시를
현재 파일과 대조한 뒤 동일 source의 실제 WASM render tree와 PDF vector path를 비교했다.
production source는 바뀌지 않았으므로 직전 fresh WASM을 재사용하며 재빌드를 반복하지 않는다.
`mutool draw -F trace <대응PDF> 1-5`의 실제 transform을96dpi로 환산했고, 원본 부모 폭
638.68px에 대응하는 수평 외곽만 골랐다. 결과는 `audit.json`, 재현 가능한 원 trace는
`reference.xml`이다. PDF Creator `Hwp 2020 0.0.0.0`, Producer `Hancom PDF 1.3.0.550`,
7쪽/A4이며 폰트 목록에는 embedded Gulim/H2gtrM/H2gtrE 및 Type3가 포함된다.
기존 대응 PDF를 재사용했고 이번에 새 한컴 출력이나 폰트 환경 동등성을 확보했다고 쓰지 않는다.

| 물리 쪽 | PDF 부모 하단(px) | 현재 V2 부모 하단(px) | 아래쪽 차이(px) |
| --- | ---: | ---: | ---: |
| 1 |1021.923|1028.013|6.091|
| 2 |1021.923|1028.013|6.091|
| 3 |1016.968|1028.013|11.045|
| 4 |1010.255|1028.013|17.759|
| 5(부모 종료) |832.369|833.173|0.804|

실제 경로:

- `stored_text.rs:162`의 `cell_frame_starts`는 되감김 위치만 반환한다.
  `ir.rs:227`은 높이가 없는 `FlowBlock::StoredFrameStart`를 넣는다.
- `flow.rs:98-104`는 경계를 만나면 **pen과height를 모두area.height로 덮어쓴다**.
  이는 프레임 소유를 보존하지만, 원본에 없는 "표 외곽도 남은 예산 전체를 채운다"는
  높이 가정을 동시에 추가한다. 미소비 페이지 공간과 실제 셀 외곽이 구별되지 않는다.
- `fragment.rs:281`의 `used`가 이 height를 받고, `:319`의 CellPlacement bounds 및
  TablePlacement/reserved_height로 전달된다. `body_flow.rs:45`는 같은 예약량을 본문
  높이/pen으로 수용한다. paint는 확정 bounds를 그리므로 뒤에서 임의 clamp할 문제가 아니다.
- 일반 용량 컷·마지막 프레임에는 이 marker 덮어쓰기가 없고, Never/온전한 RowBreak는
  stored frame으로 분할하지 않는다. 해당 미적용 경로까지 전부 같은 결함이라고 하지 않는다.

**기존 검사 해석 정정:** `issue_7353_stored_cell_frames.rs:70`의
`bottom(parent)==bottom(body)`는 현재 구현을 고정한 계약이지 PDF1~4쪽의 독립적인
정답이 아니다. "대응 PDF에서 본문 경계까지 채운다"는 주석/앞선 보고의 근거가 부족하다.
321개 PASS 중 이 assertion의 성공을 하단 피델리티 충족으로 세지 않는다. 이번 절편에서는
대신 더 느슨한 assertion으로 통과시키거나 baseline을 변경하지 않았다.

기각한 대안: 마지막 실제 내용 상자+저장 마지막 줄간격+하단 padding으로 단순 종료시키면
1쪽 하단1003.547px(PDF보다18.376px 짧음),4쪽1020.400px(PDF보다10.145px 김)이 된다.
따라서 남는 공간을 통째로 제거하거나 특정 쪽에서 마지막 gap을 버리는 보정도 채택하지 않는다.
원본 common.height51339HU는 단일 선언값, cell.height189665HU는 단일 셀 값이며,
이를 페이지별 외곽 높이 목록으로 해석할 근거는 아직 없다. raw extra에도 알려진 페이지별
높이 목록은 파싱되어 있지 않다. 파서에서 데이터가 소실됐다고 단정하지 않는다.

현재 판정은 **외곽 차이와 무조건 예산 전체 예약 경로 확인 / 정확한 대체 높이 규칙 미검증**이다.
`frame-extent/parent-bottom-review.png`를 직접 열어1·4쪽의 같은 하단 영역을 확인했다.
이 그림은 기존 승인 WASM의 조사용 재구성이며 수정 후 산출물이나 새 시각 통과 후보가 아니다.
원본의 마지막 빈 문단은 계속 보존해야 하며 잉크 끝에 맞춘 외곽 축소는 금지한다.

후속 수정의 설계 경계는 `내용 컷/다음 페이지 소유`와 `표 조각의 물리 외곽/본문 예약`의 분리다.
외곽을 결정할 독립 근거를 먼저 확보해야 한다. 정상 한컴 저장 대조군에서 마지막 빈 줄·
줄간격·셀 하단 여백·표 바깥 여백을 하나씩 바꾼 전후 입력/PDF로 종료 규칙을 분리하고,
그 규칙에 따라 정식 FAIL/PASS 계약과 공통 fragment 결과를 구현한다. 단일 원본의 관측
좌표를 상수로 옮기거나 테스트 통과를 목적으로 기대값을 바꾸지 않는다. 당시 이름이 노출된
도구 목록만 확인하여 새 한컴 대조군 획득을 미실행으로 남겼다. 아래 후속에서 문서화된
MCP CLI 경로를 확인했으므로 "변환 경로가 없다"는 판단은 정정한다. 이미 승인된 상단
수정은 유지하며, 이 미검증 하단 규칙을 구현 완료로 처리하지 않는다.

### 2026-09-27 — 정상 한컴 저장 대조군으로 프레임 하단 규칙 분리

다음 절편 승인에 따라 `bug-hunter`의 독립 기준 확보 절차를 계속했다. 이름이 노출된 도구가
없더라도 `mydocs/manual/mcp_hwp2024Convert_usage.md`의 archive CLI를 사용할 수 있었다.
비공개 env 파일을 인자로 전달하는 `start → status → succeeded 확인 → download`로
새 대조군을 획득했다. 인증값/endpoint는 이 문서나 명령 증적에 기록하지 않는다.

`output/7353/r19/frame-extent/`의 `create.rs`는 원본에서 스타일/표 틀만 가져온 별도
작성 대조군을 만든다. 65개 단문, 14pt, 한 셀의 RowBreak 표이며 원본 #6923 내용과의
피델리티 대체물이 아니다. **LineSeg 없는 작성 HWPX → 한컴 HWP 저장 → 그 HWP에서 PDF**
순서이고, 한컴 저장본의 줄 정보는 수동 수정하지 않았다. 최종 비교군은 다음 6종이다.

| 대조군 | 분리한 조건 | 한컴 저장 첫 조각 높이(px) | 현재 V2 높이(px) | 진단 |
| --- | --- | ---: | ---: | --- |
| `base2` | 줄간격100%, 안 여백141HU |899.760|906.853|FAIL|
| `blank2` | 내부41번째 문단을 빈줄로 교체 |899.760|906.853|FAIL|
| `blankend3` | 첫 쪽 마지막48번째 문단을 빈줄로 교체 |899.760|906.853|FAIL|
| `padding2` | 셀 하단 여백141→750HU |889.213|906.853|FAIL|
| `outer2` | 표 바깥 하단 여백0→750HU |881.093|906.853|FAIL|
| `gap3` | 셀 내부 줄간격160%, 본문 host는100% 유지 |888.560|906.853|FAIL|

표의 기대 높이는 **한컴이 새로 저장한 common.height**이다. 이 대조군에서는 첫 조각의
마지막 저장 줄 `vpos + line_height + 셀 상하 여백`과 정확히 같고, PDF vector의 실제 외곽도
함께 관찰했다(`controls.json`, 각 `*-trace.xml`). PDF 첫 조각 실측 높이는 base2=898.696,
padding2=888.148, outer2=880.156, gap3=887.508px이다. 저장 수치 대비 약1px의 인쇄 출력
차이는 별도로 남기며 좌표를 이동/확대해 숨기거나 PDF와 수치상 완전 일치라고 하지 않는다.

이 대조군이 입증한 범위:

- 마지막 빈 문단을 지우지 않아도 올바른 외곽을 계산할 수 있다. blankend3의48번째 줄은
  한컴 저장본에서1400HU 높이를 가지며, 기본 문서와 같은 프레임 높이/쪽 소유를 유지한다.
- 160%의 마지막 줄간격840HU는 다음 줄 원점용이며 해당 조각 외곽에 더해지지 않는다.
  중간 줄간격은 유지된다. 마지막 줄의 높이 자체를 버린다는 뜻이 아니다.
- 셀 하단 여백은 물리 외곽에 포함된다. 여백 때문에 첫 쪽 수용 줄 수가48→47로 바뀌므로
  더 큰 여백에서 첫 조각의 총 높이가 오히려 작아질 수 있다.
- 표 바깥 여백도 수용 줄 수에 영향을 주지만 셀 외곽 높이에 합쳐 그리지 않는다.
- 현재 V2는 이 차이를 무시하고 `StoredFrameStart`에서 동일한 남은 페이지 예산을
  조각 높이로 만든다. `before-controls.log`의6건 FAIL은 현재 결함 재현이며 전체 CI 실행이나
  이번 변경 때문에 새로 생긴 회귀6건이라는 뜻이 아니다. CI 기대값/ignore는 바꾸지 않았다.

중간 작성본도 보존했다. offset0 작성본은 한컴이0폭 host 줄을 저장하여 V2가 거부했다.
`gap2`는 host까지160%로 만들어 host 흐름2240HU와 표 offset2000HU가 겹쳤다.
그 입력을 수용하도록 엔진 조건을 완화하지 않았다. 최종군은 정상 작성 속성으로 host 공간을
확보하고 다시 한컴에서 저장했으며, 이전 입력의 성공으로 바꾸어 보고하지 않는다.
최종 `create.rs`는6종의 의미상 재생성 절차이며 기존 산출물 덮어쓰기를 거부한다.

변환 provenance: profile2020, Hancom11.0.0.9136,32-bit managed DLL host,
input_preprocess=none, PDF one-up(print_method0), font scope verified(2등록/0실패).
각 `*-hwp-job/status/download.json`, `*-pdf-job/status/download.json`에 job과 결과가 있다.
대표 base2 HWP job=`9b442820-11f0-44de-8bf3-5026c0698b61`, PDF job=
`ca903f95-8fd4-42ee-bcce-b3af31703ca0`이다. 원본도 별도 `original-resaved.hwp`로 저장해
확인했지만 원본을 교체하지 않았다. 첫 조각 common.height51339HU와 주요 프레임 경계는
유지되었고 일부 저장 vpos에2HU 차이가 있었다. 이를 원본 전체 재조판 일치 증거로 쓰지 않는다.

재현 명령은 `node output/7353/r19/frame-extent/controls.mjs base2 blank2 padding2 outer2 gap3 blankend3`,
`check-controls.mjs`의 같은 인자, `capture.mjs <base2|blankend3|gap3> --wasm`이다.
Native6종을 실행했고 기본/끝 빈줄/160%의 Native와 실제 Chrome WASM 총7쪽을 비교했다.
7개 SVG byte 동일, render tree 비수치 차이0, 최대 수치 차이2.274e-13이다.
코드 source manifest 전체가 이전 fresh 빌드와 일치하므로 같은 WASM을 사용했으며
새 빌드를 했다고 주장하지 않는다. WASM SHA는 위 상단 수정 검증과 동일하다.
각 `*-review/run.json`과 `before-controls.json`에 입력/PDF/source/WASM 해시를 연결했다.

`base2-review/wasm-review-1.png`와 `controls-before-review.png`의 실제 하단을 직접 확인했다.
compare/standalone overlay/review는 각 대조군 `*-review/`에 있다. 이미지에 명시했듯
**수정 전 결함 재현 자료이지 시각 통과 요청이 아니다.** 기존 상단 승인도 취소하지 않는다.

이번 절편은 독립 대조군과 실패 검출 근거 확보까지다. 일반 텍스트 대조군의 종료 규칙을
원본 중첩 표에 무조건 대입하면 앞선1쪽/3쪽의 물리 밴드 차이는 아직 설명되지 않는다.
후속 구현은 줄/개체 점유 끝, 문단 tail, 셀 inset, 바깥 여백, 다음 프레임 소유를 분리해
공통 fragment 결과로 전달해야 한다. 정상 예산/부족 예산의 마지막 유닛 원자 수용,
일반 용량 컷과 저장 프레임 컷, intact/Never의 비적용, 뒤 문단을 정식 `tests/cases/`에서
전후 검증한 뒤 원본과 fresh WASM 시각 자료를 다시 만든다. 이번 조사만으로 구현 완료,
회귀 PASS, R5 완료를 선언하지 않으며 production Rust/test source는 변경하지 않았다.

### 2026-09-27 — 저장 프레임 종료 구현 후보와 원본 물리 밴드 보류

다음 절편 승인에 따라 위 대조군으로 재현한 `StoredFrameStart`의 남은 페이지 예산
전체 점유를 수정했다. 기본 엔진은 여전히 Legacy이며 이 절편은 실험 V2 경로다.
**일반 대조군은 해결했지만 원본 #6923 전체 시각 통과는 보류한다.**

적용 규칙과 소비 경로:

- `ir.rs::bind_table`은 저장 프레임 경계 직전의 `ParagraphEnd`를 일반 Space로
  일찍 없애지 않고 `paragraph_end.rs::into_stored_frame_tail`을 통해 typed tail로
  보존한다. 외부 composer의 의미를 모르는 tail은 여전히 명시적 Space로 보존한다.
- `content.rs::physical_extent`의 intact 측정과 `flow.rs::fit_cell_until`의 intact/
  Never 배치는 기존 tail을 같은 순서로 더한다. 저장 프레임 분할 경로만 마지막
  following-line gap 대신 점유 끝 + 문단 뒤 간격 + 셀 하단 inset을 소비한다.
  마지막 빈 문단의 줄 상자는 삭제하지 않는다.
- `flow.rs`는 마지막 유닛 fit 전에 하단 inset/문단 뒤 간격을 예산에서 예약한다.
  `StoredFrameStart` marker는 다음 프레임 소유로 남기지만 현재 조각 높이를 페이지
  전체 예산으로 바꾸지 않는다. 초기/이어받기 프레임의 상단 inset도 첫 실제 유닛과
  함께 수용한다. 상단 여백만 있는 빈 조각을 허용하던 부족 예산 반례도 재현·수정했다.
- `fragment.rs::fit_rows`의 `used = max(content_offset + fit.height)`가 셀 bounds,
  표 placement와 reserved height에 그대로 전달된다. 뒤 본문은 `FlowBlock::Table`과
  `body_flow.rs::BodyFit::accept`가 같은 조각의 점유/advance를 소비한다. paint에서
  외곽을 다시 늘리거나 clamp하지 않는다. 일반 용량 컷에는 저장 프레임 inset을 재생하지 않는다.
- rowspan 전용 `row_groups.rs`는 `FlowCursor::fit`의 intact 쿼리를 사용하여
  `stored_frames=false`로 전달된다. 이번 종료 규칙을 적용하지 않으며 저장 프레임
  분할 피델리티는 미검증이다. 이 경로를 지원 완료로 확대하지 않는다.
  intact/Never 비적용 계약도 유지했다.

정식 테스트는 `tests/cases/issue_7353_stored_frame_end.rs`다. 입력6종의 정상 저장 HWP,
작성 HWPX와 독립 PDF를 `tests/fixtures/issue7353/stored-frame-end/`에 보존했다.
기대 높이는 위 독립 한컴 저장값이며 구현 결과의 재인용이 아니다. 실제 셀 bounds,
첫 조각의48/47/30줄, 마지막 빈줄 포함65줄의 순서·중복·누락, 다음 쪽 시작,
마지막 AFTER TABLE의 표 밖 위치와 추가 빈 페이지 부재를 검사한다.
합성 경계는 점유20/advance15 또는20, 문단 뒤3, 하단5의 독립 합28로 검사한다.
27px 예산은 거절,28px는 수용하며 intact 경로는 기존48px를 유지한다.
상단5를 더한33px 경계도4/5/25/32px 거절과33px 수용을 확인한다.

전후 증거는 `output/7353/r19/frame-end/`에 둔다. 기존 수정 전 Native 실행 파일
`leading-blank/probe`로 동일 입력을 실행한 결과에 정식 테스트의 동일 좌표 assertion을
적용한 `formal-before-replay.rs/log`는 FAIL이다. 전체 과거 source를 다시 빌드한 것이
아니라 수정 전 실행 파일의 실제 출력에 assertion을 적용한 경로임을 구분한다.
6종 각각의 수정 전 FAIL은 기존 `frame-extent/before-controls.json`에 있다.
추가 상단 여백 반례의 수정 전 FAIL은 `contracts-padding-before.log`이며,
최종21 harness **325 PASS / 0 FAIL / 0 ignored**는 `tests-summary-final.log`에 있다.
이는 #7353 관련 소범위 검사이지 전체 CI/전체 corpus 통과가 아니다.
fmt, Native library와 wasm32 library Clippy도 `*-final.log`에 기록한다.

기존 `issue_7353_stored_cell_frames`의 “원본 PDF 첫4쪽이 본문 하단과 같다”는 assertion은
앞 절편의 PDF vector 관찰로 전제가 틀렸음이 확인되어 본문 초과 여부로 정정했다.
높이의 정확성은 새 독립 대조군 계약으로 검사한다. 이 정정으로 원본의 남은 외곽 차이를
통과시킨다는 의미는 아니다. 아래 미충족 상태와 수치를 별도로 유지한다.

원본은7쪽이고 수정 전후 각 쪽의 TextRun 순서가 모두 동일하다. 하지만 독립 PDF와
부모 표 하단을 직접 비교하면 다음 차이가 남는다(`original-comparison.json`).

| 물리 쪽 | 후보 하단 - PDF 하단(px) | 판정 |
| --- | ---: | --- |
| 1 | -22.323 | 미충족: 외곽 아래 물리 밴드를 아직 보존하지 못함 |
| 2 | -3.349 | 미충족: 잔여 외곽 차이 |
| 3 | -7.035 | 미충족: 잔여 외곽 차이 |
| 4 | +0.972 | 약1px 인쇄 출력 차이 범위, 사용자 시각 판정 전 |
| 5 | +0.804 | 이번 종료 변경 전후 동일 |

특히1쪽은 기존 +6.091px보다 오차가 커졌다. 이를 개선 완료나 기존 차이라는 이유로
통과 처리하지 않는다. 원본의 common.height51339HU와 줄 점유 끝의 차이1747HU는
후속 조사 대상이며, 이 수치를 문서별 여백 상수로 추가하지 않는다. 정상 대조군은
후속 내용 누락 없이 해결했지만 **원본 물리 밴드 규칙이 확인되기 전 후보를 최종 승격하지 않는다.**
이번 작업의 비교 자료는 대조군 부분 검증과 원본 미충족을 구분하는 자료이며 R5 완료,
전체 시각 승인, PR/push 승인 요청이 아니다.

최종 후보 source는 HEAD `50823731a` + WIP이며 `frame-end/source.sha256`로 고정했다.
상단 여백 경계 보정 전에 시작했던 첫 Docker 산출물은 최종 증거로 사용하지 않았다.
보정 후 `docker compose --env-file .env.docker -p rhwp run --rm wasm`을 다시 완료했다
(`docker-verified.log`,7분8초). 최종 WASM SHA256은
`da653c776d1de9d25d8a06b11d20c1341d8e3fb46c5198838f6fb9c697474ff6`이다.
`capture.mjs <base2|blankend3|gap3|original> --wasm`으로 실제 Chrome의 DocumentV2를
실행했다. 기본/끝 빈줄/160% 대조군7쪽과 원본7쪽, 합14쪽의 Native/fresh WASM SVG가
byte 동일하고 render tree 비수치 차이0, 최대 수치 차이2.274e-13이다.
`summary.json`, 각 `*-review/run.json`과 `backend-comparison.json`에 source/input/PDF/
WASM 해시를 연결했다. 실험 DocumentV2를 호출하는 캡처이므로 기본 Legacy CLI sweep을
실행했다고 보고하지 않는다. 같은 페이지의 compare·standalone overlay·review를 산출했다.

직접 판독한 대표 자료는 `controls-review.png`(동일 좌표 하단 영역),
`blankend3-review/wasm-review-2.png`, `gap3-review/wasm-review-3.png`,
`original-review/wasm-review-4.png`, `original-review/wasm-overlay-1.png`다.
대조군의 마지막 빈줄 공간과 다음 쪽49번째 줄/AFTER TABLE은 유지되고 하단 외곽은
약1px 차이다. 원본1쪽에서는 표 아래 물리 공간이 짧아진 것을 overlay에서도 확인했다.
글꼴 외형 차이·원본 PDF 상단 로고 차이는 이번 하단 규칙의 해결 주장에 포함하지 않는다.
**자동 계약 통과와 별개로 원본 시각 미충족이 남으므로 최종 승인 요청을 보류한다.**

### 2026-09-27 — 원본 하단 물리 공간의 원인 분리: 셀 최소 높이 배분

다음 절편 승인에 따라 원본의 부족한1747HU를 정상 한컴 저장 대조군으로 추적했다.
**원인은 빈 문단 누락이 아니라, 분할 도중 셀 최소 높이의 남은 물리 공간을 배분하지
않는 V2 경로다.** 이번 절편은 원인과 반례를 확정했으며 production Rust는 수정하지
않았다. 위 후보의 원본 시각 미충족 상태를 유지한다.

입력 생성과 독립 근거는 `output/7353/r19/frame-band/`에 보존했다.
`create.rs`는 앞 절편의65줄 무캐시 작성 HWPX에서 표 높이만90000HU로 바꾼
`declared-tall`과 셀 높이만200000HU로 바꾼 `cell-tall`을 만든다.
`original-controls.rs`는 원본 부모 셀 높이를189665/1000HU로 달리한 두 HWPX를
만든다. 두 입력 모두 부모 셀 LineSeg를 동일하게 제거하여 한컴이 다시 조판하도록
했고, 그 입력 캐시를 증거로 사용하지 않았다. 네 문서 모두 MCP `start → status →
download`로 한컴2020 profile에서 정상 HWP 저장한 뒤 **그 저장 HWP를 PDF로 출력**했다.
원본과 기존 기준 PDF는 변경하지 않았다. 변환 job/status/download JSON과 input/HWP/PDF,
`analyze.mjs`, `analysis.json`이 재현 근거다.

| 정상 저장 대조군 | 입력 차이 | 한컴 저장 첫 조각 높이(HU) | 관찰 |
| --- | --- | ---: | --- |
| declared-tall | 표 common.height만1000→90000 | 67482 | 기존 base2와 동일. 선언 표 높이를 무조건 최소 높이로 쓸 수 없음 |
| cell-tall | 셀 height만1000→200000 | 67913 | 첫 쪽부터 물리 공간을 소비하고, 내용 종료 후에도3쪽까지 남은 셀 공간 보존 |
| original-minimum | 원본 셀 height189665 유지 | 51339 | 원본 기준 PDF의1~5쪽 하단과 동일 |
| original-small | 부모 셀 height만1000 | 49592 | 첫 하단이1747HU 줄고,2·3쪽도 줄지만4·5쪽은 불변 |

원본 변형 두 정상 저장본의 부모 셀 **전체 paragraphs JSON이 동일**하다. 부모 표의
semantic 차이는 `cells[0].height`와 한컴이 다시 계산한 `common.height`뿐이며,
raw control bytes16/17도 이 common.height의 복제다. 줄 소속·줄 상자·빈 문단·중첩 표의
내용을 바꾸지 않아도 외곽 하단이 달라졌으므로,1747HU를 빈 줄이나 line gap으로
추가하는 처리는 잘못이다. `common.height`도 이 사례에서는 원인이 아니라 저장된
첫 조각 크기라는 결과다.

PDF의 동일 페이지에서 부모 외곽 하단을96dpi 좌표로 비교한 값:

| 물리 쪽 | 셀189665HU PDF | 셀1000HU PDF | 현재 V2 원본 |
| --- | ---: | ---: | ---: |
| 1 | 1021.923 | 998.588 | 999.600 |
| 2 | 1021.923 | 1017.607 | 1018.573 |
| 3 | 1016.968 | 1008.976 | 1009.933 |
| 4 | 1010.255 | 1010.255 | 1011.227 |
| 5 | 832.369 | 832.369 | 833.173 |

현재 V2는 원래 셀 높이가 있어도 작은 셀 높이의 한컴 대조군과 같은 하단을 낸다
(약1px 인쇄 출력 차이는 별도). 페이지 수는 두 PDF 모두7쪽이다. 페이지 수만으로는
이 결함을 검출할 수 없다. 단순 cell-tall도 V2는3쪽을 만들지만 첫 조각899.760px가
한컴 저장905.507px보다 짧고, 마지막 조각이 그만큼 길어진다. 총 높이 보존만으로
조각별 높이 배분의 정확성을 입증할 수 없다.

실제 소비 경로:

1. `ir.rs::bind_table`이 `cell.height * scale`을 `FlowCellInput.minimum_height`로
   전달하고 `fragment.rs::TableCursor::reset_row`가 `minimum_left`로 보존한다.
2. `fragment.rs::fit_rows`의 `used`는 실제 수용한 줄/자식 조각 높이다.
   **289행의 `if all_done` 안에서만** 남은 최소 높이를 고려한다. 저장 프레임 경계에서
   끊긴 비끝 조각은 이 처리를 건너뛴다.
3. 같은 `used`가 셀 bounds·표 조각 높이·본문 예약 높이로 전달되고,331행에서
   `minimum_left`를 차감한다. 따라서 paint의 별도 축소가 아니라 fit 결과의 물리
   공간 배분 누락이며, paint에서 외곽만 늘려 해결하면 본문 예약과 다시 불일치한다.
4. Legacy `float_placement.rs` / `table_partial.rs`에는1x1 비끝 조각을 본문 하단−
   바깥 아래 여백−100HU로 고정하는 별도 규칙이 있다. 이번 작은 셀 높이 대조군과
   원본4쪽은 같은1x1 RowBreak 형상이지만 그 위치까지 차지하지 않는다. 이 예외를
   V2에 그대로 복제하지 않는다. 남은 최소 높이 소비와 쪽 경계 예산을 구분해야 한다.

다음 구현은 `common.height` 반복 강제나 문서별1747HU 보정이 아니라, 내용 컷과
**남은 물리 최소 높이의 분할 소비 결과**를 함께 만드는 방향이다. 내용 유닛이 전혀
fit하지 않는 경우의 빈 조각 금지, 최소 높이를 모두 소비한 뒤 내용 높이로 종료,
작은 최소 높이의 기존6종 대조군, 실제 후속 문단, 적용되는 rowspan/중첩 경로를
검증해야 한다. 쪽 경계의 바깥 여백·기존100HU 규칙의 적용 근거는 별도 확인하여
최소 높이 배분 문제와 섞지 않는다. 이 부분은 아직 구현/최종 검증 완료가 아니다.

직접 확인한 `frame-band/diagnostic-review.png`는 같은1쪽 하단 영역의
“원래 셀 높이 한컴 / 작은 셀 높이 한컴 / 현재 V2” 비교이며 **원인 진단 자료이지
수정본 승인 요청이 아니다**. V2 캡처는 위 frame-end 결과를 재사용했다. production
Rust30개 파일이 그 source.sha256과 모두 동일함을 `frame-band/source-check.log`로
확인했기 때문에 재사용한 것이며, 새 빌드나 수정 후PASS로 보고하지 않는다.
이번 절편에서는 source 변경·재빌드·전체 회귀·새 WASM·commit/push를 수행하지 않았다.

### 셀 최소 높이의 분할 소비 구현 — 부분 충족, 하단 예산은 미해결

다음 절편 승인에 따라 experimental V2의 `fragment.rs::fit_rows`를 수정했다.
Legacy/default 전환은 하지 않았다. 검증 소스는 HEAD `50823731a` 위 working patch이며,
production Rust30개 파일은 `output/7353/r19/minimum-band/source-final.sha256`으로 고정했다.
기존 WIP를 보존했다. 다음 내용은 앞 절의 진단 결과를 대체하는 구현 결과다.

- 실제 수용한 내용 컷마다 남은 셀 최소 높이를 가용 공간 안에서 함께 소비한다.
  내용이 끝난 뒤에만 최소 높이를 붙이던 지연 배분을 제거했다. 최소 높이가 소진된 뒤에는
  내용 점유 높이만 사용하며, 내용 종료 후 실제 남은 물리 공간은 유지한다.
- 첫 내용이 들어가지 않는 경우 물리 공간만으로 진행을 만들지 않는다. `from_flow_rows`가
  삽입한 상단 여백 block0만 소비한 경우도 이에 포함한다. 다른 셀의 실제 내용이 들어가면
  같은 행 조각에서 여백을 소비할 수 있다. 편집자가 입력한 빈 문단을 제거하는 조건은 아니다.
- 내용 컷/소유는 `FlowCursor::fit_cell_until`의 cursor와 lines/tables를 그대로 유지한다.
  `fragment.rs:269` 요구 높이 → `:286` 여백만 진행한 경계 판정 → `:300` 실패 시 컷 보류 →
  `:310` 최소 공간 예약 → `:328` 동일 used의 CellPlacement → 조각 높이와 minimum_left 차감으로
  이어진다. `flow.rs`의 Table 분기와 `body_flow.rs::BodyFit::accept`가 이 조각 높이를 소비한다.
  paint에서 외곽만 늘리는 보정은 추가하지 않았다. rowspan은 `fit_row_groups` 별도 경로로,
  이번 셀 내부 컷 변경 비대상이며 기존 그룹 높이 계약을 함께 실행했다.

독립 기대값과 재현:

| 검사 | 근거 / 수정 전 | 최종 결과 |
| --- | --- | --- |
| 최소120px, 줄3개 각20px, 예산50px | 각 컷의 물리 공간50+50+20. 이전20+20+50과 잔여 꼬리 | PASS |
| 첫 줄이 들어가지 않는 예산 | 컷 진행 금지, required에 상단 여백 포함. padding-before.log에서 FAIL 재현 | PASS |
| 작은/소진 최소 높이, 내용 종료 후 진짜 꼬리 | 내용 높이와 물리 공간을 구별하고 총 높이·유닛 순서 보존 | PASS |
| 중첩·여러 셀 | 부모가 자식 조각 높이를 재추측하지 않음. 다른 셀의 실제 진행 대조 | PASS |
| 정상 한컴 base2/cell-tall | 앞 절의 독립 HWP/PDF.65줄 순서와 총 높이, 첫 조각 공간 증가 | PASS; PDF 하단 정확 일치는 미충족 |

정식 원본은 `tests/cases/issue_7353_fragment_minimum_band.rs`9개이며,
정상 저장 대조군을 `tests/fixtures/issue7353/minimum-band/`에 보존했다.
초기5개를 수정 전 library에 실행한 `before.log`는4FAIL/1PASS였다.
상단 여백 경계는 첫 수정 후보에서 별도로 FAIL을 재현한 뒤 보정했다.
기존 oversize-row의 정확한 배분 기대값을20/40/50에서25/50/35로 변경했다.
이것은 같은110px 및1/2/1줄 컷을 유지하면서 호출 예산25/50/50에 최소 공간을 앞 조각부터
소비하는 계약 변경이다. 정상 한컴 대조군의 앞 조각 공간 증가가 독립 근거이며 ignore나
허용 오차 확대는 하지 않았다.

최종 로컬 실행:

- `cargo build --locked -p rhwp --lib --target-dir /home/edward/mygithub/rhwp/target/pr-review` PASS.
- `cargo fmt --all -- --check`, Native 및 wasm32 라이브러리 Clippy `-- -D warnings` PASS.
  `build-final.log`, `fmt-final.log`, `clippy-native-final.log`, `clippy-wasm-final.log` 참조.
- 기존 `frame-end/tests.sh`와 같은 standalone rustc harness로 #7353의22개 원본을 실행:
  **334PASS / 0FAIL / 0ignored**. `final-tests/`, `tests-summary-final.log` 참조.
  이는 국소 검증이며 generated CI suite·workspace all-targets·전체 CI 통과 주장이 아니다.
- `node output/7353/r19/minimum-band/capture.mjs original|base2|cell-tall`을 각각 실행하여
  최종 Native7+2+3쪽을 캡처했다. 실제 명령은 인자를 하나씩 전달한다.
  두 대조군의 최초 브라우저 실행 실패는 순차 재실행으로 해결했다. 각 `*-final-review/run.json`에
  입력/PDF hash와 소스 manifest를 보존했다. Legacy CLI가 아닌 실제 V2 probe 출력이다.
- `check.mjs` / `comparison.json`: 원본7쪽의 페이지별 TextRun 순서 전부 유지.
  원본2·3쪽 review와1쪽 standalone overlay, base2 첫쪽과 cell-tall 마지막쪽을 직접 판독했다.
  마지막 물리 꼬리와 AFTER TABLE은 보존된다. 폰트 외형 차이는 별도다.

원본 부모 외곽 하단의 기준 PDF 대비96dpi 차이는1·2쪽 **+6.091px**,3쪽 **−7.035px**,
4쪽+0.972px,5쪽+0.804px다.1쪽의 짧은 외곽은 없어졌지만 과대해졌으며,
**2쪽은 수정 전−3.349px보다 절대 오차가 커졌다**.3쪽은 변하지 않았다.
따라서 최소 높이 분할 계약은 충족했지만 원본 시각 개선 완료는 아니다.
대표 진단 자료는 `minimum-band/original-final-review/native-compare-2.png`,
`native-overlay-2.png`, `native-review-2.png`이며 승인 요청용 통과 자료로 제출하지 않는다.

다음 대상은 쪽별 하단 가용 높이/바깥여백의 정의와 최소 공간 예약의 연결이다.
Legacy100HU 보정 또는 모든1x1 비끝 조각의 본문 하단 고정을 그대로 가져오지 않는다.
현재 Native에서 차이가 확인되므로 이 후보의 fresh Docker WASM·전체 회귀는 아직 실행하지
않았다. wasm32 Clippy 통과를 WASM 실행 검증으로 대체하지 않는다. commit/push도 하지 않았다.

### 조각 하단 가용 높이 예약 — Native 및 fresh WASM 검증

다음 절편 승인에 따라 `document_input.rs`와 `body_flow.rs`를 수정했다. 앞 절의 최소 높이
분할 결과는 유지하고, 셀 높이에 바깥여백을 섞지 않았다. `body_anchor.rs`가 산출한 아래
여백을 기존의 **마지막 조각 뒤 Space** 대신 각 accepted 조각의 예산과 본문 예약에 반영한다.
분할·기하 계산 후 paint에서 외곽을 clamp하거나 본문 바닥까지 강제로 늘리지 않는다.

독립 근거와 적용 범위:

- 아래 바깥여백은 표 외곽과 본문 경계 사이의 공간이다. 첫/계속/마지막 조각에서 표와 함께
  수용하고, 여백만 뒤 페이지로 내보내지 않는다. 실제 위치 여유가 부족하면 같은 컷으로 이월한다.
- 추가100HU는 사양 필드나 일반 조판 규칙이 아니라 **저장 HWP 프레임의 경험적 호환 값**이다.
  기존 #7095 독립 한컴 PDF·여백/테두리/쪽 기하 변형 근거
  (`mydocs/report/fragment-geometry-7095/README.md`, 기존 #7095 회귀의 근거 주석)와
  이번 정상 저장 cell-tall/original-minimum 대조를 확인했다. 기존 Legacy 술어/paint 고정을
  호출하지 않고, 실제 V2 바인딩 결과에 StoredFrameStart가 있는 미편집 native HWP5의
  본문1x1 RowBreak anchor에서만 비끝 조각의 가용 높이로 사용한다. TAC·HWPX·저장 프레임이
  없는 재조판에는 적용하지 않는다. 단순1x1 형상만으로는 적용 근거가 되지 않는다.
- 최종 조각인지 먼저 순수 fit 질의로 확인한다. 끝나지 않는 경우에만 여유를 포함한 예산으로
  다시 질의하고 **재질의 결과의 컷·높이·continuation**을 수용한다. 두 질의 모두 cursor를
  변경하지 않으며 최종 결과만 commit한다. 마지막 조각 뒤에는 실제 바깥여백만 예약한다.

실제 소비 연결:

`document_input.rs:342` profile/저장 프레임 확인 → `AnchoredFlow.bottom_margin` 및
`nonterminal_clearance` → `body_flow.rs:32` 현재·새 쪽 예산 차감 → `flow.rs` Table 분기의
같은 child budget → `fragment.rs::fit_rows` 내용 컷/남은 최소 높이/CellPlacement →
`AnchoredFlow::fit`에서 바깥 공간 예약 → `BodyFit::accept` → `document.rs`의 placement와
`text.rs:195`의 동일 bounds paint다. 첫 배치와 pending/deferral이 같은 fit을 호출한다.
여백 예산 실패 시 원래 cursor를 유지하며, 여백만 소비한 deferred 경계는 기존의 명시적
DoesNotFit를 유지한다. nested cell·rowspan 내부 컷 알고리즘은 수정하지 않았으며 기존
관련 계약을 함께 실행했다.

검증은 `output/7353/r19/frame-budget/`에 보존한다. HEAD `50823731a` 위 working patch의
Rust30개 파일은 `source-final.sha256`으로 고정했다.

| 관측 | 결과와 범위 |
| --- | --- |
| 원본 PDF 외곽 회귀 | 수정 전1쪽1028.013px로 FAIL → 수정 후1~5쪽 모두 독립 PDF 축척 정규화 오차0.25px 이내 PASS |
| 큰 셀 대조군 | 기존906.853px →905.520px. PDF 벡터 높이 오차0.25px 이내; 한컴 저장67913HU와는1HU 차이 잔존 |
| 기본·선언 높이만 큰 대조군 |2쪽 유지. declared-tall의 두 쪽 SVG가 base2와 동일 |
| 저장 HWP 호환 값 비적용 | 동일 저장 내용을 HWPX로 직렬화한 source-boundary 계약 PASS. 한컴 HWPX fidelity 증거는 아님 |
| 바깥여백 포함 원자 표 |72px 본문에6+18+60px 요구를 거부, 반복 질의도 같은84px 요구/페이지 진행 없음 PASS |
| 국소 회귀 | #7353의22개 standalone 원본 harness **338PASS,0FAIL,0ignored**. 전체 CI 결과가 아님 |
| 정적/빌드 | 최종 Native library build, fmt check, Native/wasm32 library Clippy PASS. workspace all-targets/전체 CI 미실행 |

`before.log`의 원본 회귀는 실제 좌표 실패이며 빌드 실패가 아니다. 최초 큰 셀 검사는
저장 common.height와 소수점까지 동일해야 한다고 요구해 수정 후에도1HU 차이로 실패했다.
이것을 숨기지 않고 정확 일치 주장을 철회했다. 최종 검사는 원본과 **같은 독립 PDF 벡터
좌표0.25px 기준**을 사용한다. 이 변경은 저장 필드를 일치시킨 증거가 아니며100HU의 정확한
내부 반올림 규칙은 미확정이다. 큰 셀의 이전906.853px와 PDF 높이 차이는 이 기준도 위반한다.

기존 `stored_anchor_reserves_offset_once_and_preserves_host_and_following_rows`의 기대값은
본문 끝102px/표 시작54px에48px 높이를 허용하여8px 바깥여백을 침범했다. 독립 입력의
여백을 반영한 table budget94px에 따라 첫쪽 A/B, 다음쪽 C/D로 수정했다. 기존3/1줄 기대를
유지하려고 여백 적용을 제외하지 않았다. 새 원자 표 경계와 HWPX 비적용 검사는 최종 소스의
PASS이며, 이 둘을 별도 수정 전 FAIL 실행 증거로 과장하지 않는다.

Native 원본7쪽의 페이지별 TextRun 순서는 모두 유지된다(`comparison.json`).1~5쪽 하단의
PDF 원좌표 대비 차이는 각각+0.984/+0.984/+0.992/+0.972/+0.804px다. 한컴 출력의
84188HU→841pt 인쇄 축척을 정규화하면 모두0.25px 이내다. 캡처 이미지에는 이 정규화나
맞춤 이동을 적용하지 않았으므로 아래쪽에 약1px 프린지가 남는다. 원본1·3쪽 review를
직접 확인했으며 기존 글꼴/로고 차이와 이번 표 외곽 판정을 구분한다.

추가 대조군 `frame-band/original-small-saved.hwp` 전체는 paragraph29의
`unequal TAC occupied envelopes` 지원 제한으로 DocumentV2가 거부했다. 이 정상 재저장
대조군 전체의 무회귀는 미검증이며, 기본/큰 셀 대조군 통과로 대신하지 않는다.

fresh WASM은 표준 Docker 경로로 완료했다. 최초 기본 project의 신규 network 생성은
address pool 소진으로 실패했고, 이전부터 사용하던 `-p rhwp` network/cache로 재시작했다.
네트워크·볼륨을 삭제하거나 native WASM으로 우회하지 않았다.

- `docker compose --env-file .env.docker -p rhwp run --rm wasm`: **7분15초, PASS**.
  `docker-wasm-final.log`, `wasm.sha256`에 결과와 패키지 해시를 보존했다.
- `node output/7353/r19/frame-budget/capture.mjs original --wasm`, `base2 --wasm`,
  `cell-tall --wasm`을 순차 실행했다. 실제 브라우저 DocumentV2 출력으로 총 **12쪽**을
  비교했으며 Native/fresh WASM의 SVG가 모두 동일하다. JSON의 비수치 차이는 없고
  최대 수치 차이는2.28e-13 미만이다. 각 `*-final-review/run.json`과
  `backend-comparison.json`에 입력·기준 PDF·소스·패키지와 실행 결과를 연결했다.
- `geometry-comparison.json`: 수정 전후 원본7쪽의 TextRun/TextLine/Image 내용과 실제
  bounds가 모두 보존된다. 부모 표1~5쪽 하단의 PDF 인쇄 축척 정규화 오차는 최대0.086px다.
  이는 이미지의 임의 위치 정렬이나 픽셀 수정이 아니라 독립 PDF 좌표의 단위 비교다.
- 최종 원본3쪽 WASM review/standalone overlay와 cell-tall 마지막쪽 review를 직접 확인했다.
  부모 표 하단, 큰 셀의 물리 꼬리 및 AFTER TABLE을 확인했다. 원본3쪽 자동 잉크 일치율
  보조값18.78%는 글꼴 등 기존 차이를 포함하며, 사람의 시각 판정 정확도가 아니다.

시각 판정 대상은 원본1~3쪽 부모 표 하단이다. 증적 루트는
`output/7353/r19/frame-budget/original-final-review/`이며 대표 파일은
`wasm-compare-3.png`, `wasm-overlay-3.png`, `wasm-review-3.png`다.
1·2쪽은 같은 디렉터리의 `wasm-review-1.png`, `wasm-review-2.png`에서 확인한다.
소스 manifest를 최종 검증했으며 이후 production 변경은 없다.
이 절편은 작업지시자가 시각 판정 통과를 확인하고 다음 절편을 승인했다. 전체 CI, 기본 엔진 전환, commit/push는
수행하지 않았고, 앞서 명시한1HU 차이와 original-small 대조군 미검증은 남아 있다.

### 재저장 대조군 TAC 거부 — 입력 계보와 저장 줄 일관성 조사

하단 경계 시각 판정 통과 후 `original-small` 전체 수용을 막는 문단29를 조사했다.
이번 절편에는 production 코드를 변경하지 않았다. `tac-envelope/inspect.rs`를 최종
Native 라이브러리에 링크하여 원본과 두 재저장본을 직접 파싱·준비했다. 과거 JSON dump를
원본의 현재 파싱 결과로 대신하지 않았다.

| 입력 | 표 높이HU | 위/아래 바깥여백HU | 저장 줄 높이HU | 관측 |
| --- | ---: | --- | ---: | --- |
| #6923 원본 HWP |11156|−1 / −1|11154|V2 문서 준비 수용 |
| original-small-input HWPX |11156|−1 / −1|11154|문단29 속성 보존; 부모 캐시 제거 입력이므로 문서 전체 수용 증거 아님 |
| original-small/minimum 정상 변환 HWP |11156|0 / 0|11154|문단29 `unequal TAC occupied envelopes` 거부 |
| 문단29 LineSeg만 제거 후 한컴 변환 HWP |11156|0 / 0|없음|`stored TAC carrier requires unambiguous intact rows` 거부 |

원본은 `11156 − 1 − 1 = 11154`로 일치한다. child 단독 준비 높이는 원본과 재저장본
모두148.7466667px이며, 거부는 child 행 높이 계산 이전의 `tac.rs::object_rows`에서 발생한다.
`tac.rs:217`의 저장 줄/객체 점유 일치 검사 → `compose`의 줄 원점·advance/물리 점유 →
`document_input.rs`의 `tac::bind` → FlowCursor의 InlineTables 수용 → 동일 placement paint
경로다. 여기서 단순2HU tolerance를 추가하면 변환 중 달라진 여백과 오래된 줄 정보의
불일치를 검증하지 않은 채 수용하게 된다. 일반적인 서로 다른 높이의 TAC 합성 규칙도
입증하지 못한다. 이 조사만으로 guard를 완화하지 않았다.

HWPX ZIP의 `outMargin`은 실제로−1을 보존했다. serializer의
`src/serializer/hwpx/table.rs::write_out_margin`도 signed 값을 그대로 기록한다.
따라서 이번 관측의−1→0 변경은 rhwp serializer의 clamp가 아니라 **이 한컴 변환 과정**에서
발생했다. 왜 해당 변환이 저장 LineSeg를 갱신하지 않았는지는 미확정이다. 모든 한컴 편집기나
포맷 전체의 동작으로 일반화하지 않는다.

독립 재생성 시도는 원본 실패 파일을 보존한 채 `tac-envelope/create.rs`로 수행했다.
`original-small-input.hwpx`를 파싱해 문단29의 LineSeg만 제거하고 별도
`refreshed-input.hwpx`로 저장했다. MCP `start → status → download`, engine2020,
한컴11.0.0.9136, input_preprocess none으로 `refreshed-saved.hwp`를 얻었다.
job `2de06ae8-4829-41c3-8690-289cf50b7bdd`는8초에 성공했지만 **LineSeg는 여전히 없다**.
파일 변환 성공을 캐시 재생성이나 정상 조판 증거로 승격하지 않는다.

증거는 `output/7353/r19/tac-envelope/`의 `inspection.jsonl`,
`input-inspection.jsonl`, `refreshed-inspection.jsonl`, `hwp-{job,status,download}.json`,
`inspect.rs`, `create.rs`와 새 입력/저장본이다. production30개 파일은 이전
`frame-budget/source-final.sha256`과 동일함을 재확인했다. 코드 변경이 없어 이전
Native/WASM 시각 통과 증거를 재사용하며 전체 회귀·Docker 빌드를 반복하지 않았다.

결론: 원본 하단 경계의 승인과 별개로, 이전 대조군은 문단29에서 저장 정보의 상호 일관성이
깨져 있으므로 V2 수용 조건 완화의 근거로 쓰지 않는다. 부모 표 외곽에 관한 기존 PDF 관측은
보존하지만 이 문서 전체의 무회귀는 여전히 미검증이다. 다음 구현 경계는 **저장 줄 없는 TAC
문단의 텍스트/공백·객체 순서, 기준선, 바깥여백, 가용 폭을 반영한 공통 줄 구성**이다.
미지원 재조판과 저장 정보 불일치를 구분하며, 독립적인 정상 생성 문서/출력으로 이 경계의
기대값을 확보한 뒤 측정과 paint에 같은 결과를 연결한다.

### 저장 줄 없는 TAC 재조판 — 줄 소유·공백·가용 폭의 공통 결과

다음 절편 승인으로 `tac_fresh.rs`를 추가했다. 유효한 저장 LineSeg가 있으면 기존
`tac.rs` 경로를 그대로 쓰며, 저장 줄이 없는 경우만 새 composer를 호출한다.
임의 LineSeg 생성, 저장 높이 tolerance 완화, Legacy 표 측정 호출은 하지 않는다.

입력의 raw UTF-16 순서 → 공백/명시적 개행/TAC 객체 토큰 → 가용 폭에 따른 줄 구성 →
`ParagraphItem::{InlineTables,Lines,Space,End}`가 생산 결과다. 글자 크기·줄간격의 기본
메트릭은 기존 `layout_paragraph_in_physical_frame`의 빈 글줄 결과를 사용한다.
TAC 줄은 바깥여백을 포함한 점유와 advance를 구분한다. 본문과 셀 내부가 같은 결과를
소비하며, 개행으로 생긴 빈 줄도 소유 LineBox로 유지한다.

| 실제 호출 경로 | 소비 및 적용 |
| --- | --- |
| `tac.rs::compose → tac_fresh::compose` | 캐시가 없는 경우에만 원점·높이·줄 소유를 구성 |
| `document_input.rs`의 TAC 분기 | 본문 InlineTables와 빈 Lines를 FlowBlock으로 전달 |
| `text_ir.rs::compose → ir.rs::bind_table` | 셀 내부도 동일 ParagraphItem을 FlowBlock으로 전달 |
| `tac.rs::bind` | 실제 자식 계획과 선언 객체 상자의 폭·높이가 다르면 명시적 거부; resize하지 않음 |
| `flow.rs`의 InlineTables 수용 | 줄 전체 요구 높이를 예산과 비교; 부족하면 해당 줄을 소비하지 않고 이월; 형제 표 전부를 transactional fit |
| `document.rs` 및 기존 text paint | 같은 placement의 원점/자식 조각을 사용; 별도 원점 추측·clamp 없음 |

범위는 비음수 바깥여백, 들여쓰기 없는 공백·TAC·개행 문단이다. 같은 줄의 TAC는 같은
점유 높이 envelope에 한정한다. 서로 다른 높이의 공통 기준선, 가시 텍스트 혼합, 비최종
양쪽 정렬 줄의 공백 배분, lane보다 큰 객체, 편집으로 선언 상자보다 커진 자식 내용은
아직 미지원이다. 범위를 일반 TAC 재조판 완료나 R5 완료로 보고하지 않는다.
rowspan/cell 분할 알고리즘·clipping·후속 캡션 규칙은 변경하지 않았으며, 이번 TAC 줄의
완전 수용/이월에서 동일 객체를 중복 소비하거나 누락하지 않는지를 검사했다.

#### 독립 기준 및 계약

- 기준은 `tests/fixtures/issue7353_tac_noop_review/README.md`에 출처가 있는 정상 한컴
  저장본과 PDF다. 파생 입력은 **중첩 TAC 호스트의 LineSeg만 제거**해서 저장한다.
  원본 그대로의 저장 줄 일치 검사와 이 재조판 검사는 별개다.
- 부모 y8787HU, 셀 padding283HU, 앞 줄 advance1760HU, 자식 높이5000HU,
  CELL AFTER vpos7420HU 및 표 뒤 본문 vpos21685HU를 독립 기대값으로 사용했다.
  실제 부모/자식 외곽과 뒤 문단까지 검사한다.
- 재조판 내부 폭은 `32000−2×283=31434HU`다. 저장 줄 폭31432HU와 달라 가운데
  정렬 x는 저장 기준보다1HU(약0.013px) 오른쪽이다. 허용치로 숨기지 않고 실제 가용
  폭에서 좌우 여유가 같은 정렬 불변식을 검사한다. y·높이는 독립 기준과 일치한다.
- `tests/cases/issue_7353_table_v2_document_flow.rs`에5개 정식 계약을 추가했다.
  같은 줄의 두 표, 너비 부족에 따른 두 줄/페이지, 명시적 개행, 선두 빈 줄,
  앞·사이 공백의 순서/너비, 뒤 문단 위치 및 미지원 반례를 검사한다.
- 첫 합성 입력의 저장 과정에서 자동 구역 컨트롤이 개행 앞 객체 슬롯을 소비했다.
  합성 입력에 구역 슬롯을 명시하고 재파싱 char_offsets 보존을 추가 검사했다.
  또한 자식 내용의 종료 간격 정책과 선언 높이가 달랐던 합성 입력은 의도한 셀 최소
  높이2700HU를 명시했다. 두 수정은 테스트 입력의 계약을 분명히 한 것이며 생산
  수용 조건을 완화한 것이 아니다.
- `fresh-tac/before.log`의 독립 중첩 TAC 검사는 수정 전 실제 실행에서 저장 줄
  미지원으로 FAIL, 동일 파생 입력의 최종 검사는 PASS다. 초기 합성 행 검사도 FAIL이었으나
  이후 입력 슬롯·최소 높이를 정정했으므로 최종 모든 합성 변형의 수정 전 검출 증거로
  대신하지 않는다. 정상 대조군인 기존 저장 줄·일반 fresh 문단 검사는 보존한다.

#### 검증 상태

검증 소스는 HEAD `50823731a` + 작업 변경이며 `output/7353/r19/fresh-tac/source.sha256`
으로 생산31개 파일을 고정했다. `after-final.log`는 최종 문서 흐름99건 PASS,
`suite.log` 및 `tests/*.log`는 #7353의22개 source/343건 PASS다. 마지막 추가 좌표
assertion 후 영향 source99건을 다시 실행했다. Native/WASM library Clippy 각각 PASS,
fmt 및 diff whitespace 검사 PASS다. 이는 절편 검사이며 workspace/all-target CI 묶음의
완료를 주장하지 않는다.

기존 #6923 원본은7쪽이며 `original-control/native.json`을 이전 승인본
`frame-budget/original-final-review/actual/native.json`과 `cmp`하여 전체 렌더 트리·SVG의
동일함을 확인했다. `tac-envelope/refreshed-saved.hwp`도7쪽 출력까지 진행한다
(`refreshed.log`). 후자는 수용 진단이며 문서 전체 시각 통과로 승격하지 않는다.

Native 비교 명령은 `node output/7353/r19/fresh-tac/capture.mjs`다.
`review/native-{compare,overlay,review}-1.png`에서 자식 표 위치·높이, CELL AFTER,
부모 외곽과 AFTER PARENT TABLE을 직접 확인했다. 픽셀 자동 일치율 보조값37.72%는
글꼴 등 차이를 포함하며 사람 시각 판정 정확도가 아니다. 입력·기준 PDF·source hash는
`review/run.json`에 연결했다. fresh Docker WASM 최종 비교 결과는 아래에 이어 기록한다.

`docker compose --env-file .env.docker -p rhwp run --rm wasm`은7분17초에 PASS했다.
`node output/7353/r19/fresh-tac/capture.mjs --wasm`으로 새 패키지를 브라우저에서
실행했다. `review/backend-comparison.json`: Native/fresh WASM SVG 동일, 비수치
차이0건, 최대 수치 차이5.69e-14 미만이다. `wasm.sha256`에 패키지를 고정했고
`source-check.log`에서31개 생산 파일이 검증 중 변경되지 않았음을 확인했다.
최종 WASM review와 standalone overlay를 직접 판독하여 자식 외곽과 뒤 문단·부모
외곽/뒤 본문의 보존을 확인했다. 작업지시자가 시각 판정 통과 및 다음 절편 진행을 승인했다.

판정 자료: `output/7353/r19/fresh-tac/review/wasm-review-1.png`,
`wasm-compare-1.png`, `wasm-overlay-1.png`. 시각 입력은 같은 루트 상위의
`fresh-nested.hwp`, 기준은 위 fixture의 `noop-2020.pdf`다. 이 결과를 혼합 텍스트·
서로 다른 높이의 TAC·재저장 원본 전체의 피델리티 증거로 확대하지 않는다.
전체 CI, 기본 엔진 전환, commit/push는 수행하지 않았다.

### 승인 후 후속 검증 — fresh TAC 재저장 파생 문서 전체

앞 절편의 시각 통과를 기록하고, 수용 진단만 남았던
`output/7353/r19/tac-envelope/refreshed-saved.hwp` 전체를 검증했다. 생산 코드와
Rust test는 추가 변경하지 않았다. HEAD `50823731a` + 기존 WIP의
`fresh-tac/source.sha256`, `wasm.sha256` 일치를 확인하여 앞 절편의 Docker WASM을
재사용했다. 동일 소스의 빌드·343건 검사를 반복하지 않았으며, 이번 실행 결과로
workspace CI 통과를 새로 주장하지 않는다.

#### 입력과 독립 기준

입력은 #6923 원본 그 자체가 아니다. 부모 셀 최소 높이를189665→1000HU로 낮추고
부모 셀의 저장 줄을 제거한 파생본을 한컴에서 재저장했으며, 본문 p29 TAC의 LineSeg도
제거한 재조판 대조군이다. `tac-envelope/create.rs`에 생성 과정이 있다. 이 결과를
원본 전체의 한컴 일치나 일반 fresh TAC 지원 완료로 확대하지 않는다.

같은 `refreshed-saved.hwp`를 MCP 변환 클라이언트의 `start --target pdf --engine 2020`
→ `status` → `download`로 한컴 PDF화했다. 작업 ID는
`4d0b9fd5-d49f-442a-b163-3b5b104bb08f`이며, 입력·PDF hash와 소스 manifest는
`output/7353/r19/fresh-tac-full/review/run.json`에 고정했다.
기준 PDF는 `fresh-tac-full/refreshed-2020.pdf`이고7쪽이다. 정상 변환된 독립 출력이며
Native SVG나 브라우저 인쇄로 만든 정답지가 아니다.

#### 실행과 직접 판독

- `node output/7353/r19/fresh-tac-full/capture.mjs --wasm`: Native/fresh WASM 모두7쪽,
  7쪽 모두 SVG 동일, 비수치 차이0건, 트리의 최대 수치 차이2.274e-13px 미만.
- 전체7쪽 WASM review와3·6쪽 standalone overlay를 직접 확인했다. 부모 표1–5쪽의
  외곽, 이어지는 자식 표와 본문,6쪽 하단의 저장 줄 없는 TAC 및7쪽 후속 내용을 확인했다.
  이것은 직접 판독 범위이며 모든 문자열·유닛의 완전 보존을 전수 입증했다는 뜻은 아니다.
- `node output/7353/r19/fresh-tac-full/geometry.mjs`: PDF stroke와 V2 부모 하단을
  대조했다. PDF 용지841pt와 HWP84188HU의 비율을 적용한 **진단용 좌표 환산** 후
  1–5쪽 하단 차이는 각각−0.033/−0.098/−0.098/−0.058/−0.067px였다.
  이미지에는 이 환산이나 위치 보정을 적용하지 않았고, 테스트 허용치를 바꾸지 않았다.
- 기존 승인 원본과 비교할 때1–3쪽 부모 하단은 작은 최소 높이에 맞게 줄었다.
  자식 상자는4쪽 마지막 표를 제외하고 동일하다.4쪽 마지막 표와6쪽 본문 p29 표는
  x/y가 각각+1HU다. `inspect.rs`와 `original-ir.jsonl`/`refreshed-ir.jsonl`에서
  해당 표의 바깥여백이 한컴 재저장 중−1→0HU로 바뀐 것을 확인했다.4쪽 호스트 저장
  줄 높이도14845→14847HU이며,6쪽 p29는 저장 줄이 없는 입력이다.
- 축소 review2를 처음 보고 큰 가로 이동을 의심했으나 PDF stroke 직접 측정으로
  정정했다. 해당 자식 표 왼쪽은 PDF109.381px, V2 109.520px이며 V2 상자는 기존
  승인 원본과 동일하다. 큰 이동 결함으로 등록하거나 이를 맞추는 보정을 하지 않았다.

대표 판정 자료는 `fresh-tac-full/review/wasm-review-3.png`, `wasm-overlay-3.png`,
`wasm-review-6.png`, `wasm-overlay-6.png`다. 내용 픽셀 자동 일치율 보조값은3쪽19.73%,
6쪽46.81%이며 사람 판정 정확도가 아니다. 글꼴 폭·굵기 차이와1쪽 로고 출력 차이는
남아 있으며 이번 TAC/부모 외곽 검증으로 해결했다고 보고하지 않는다. 작업지시자가
시각 판정 통과와 다음 절편 진행을 승인했다. 전체 R5 완료·기본 엔진 전환·commit/push는
수행하지 않았다.

### 승인 결과의 정식 회귀 계약 고정 및 다음 실물 진입점 확인

생산 코드는 바꾸지 않고 승인된 파생 입력·독립 PDF·생성 HWPX를
`tests/fixtures/issue7353/fresh-tac-full/`에 보존했다. README에 원본과의 차이,
한컴 정상 저장/PDF job, hash, 독립 기대값과 판정 한계를 기록했다.
`samples/` 추가나 기존 baseline·ignore·허용치의 변경은 없다.

새 `tests/cases/issue_7353_fresh_tac_full_document.rs`의 계약은 실제
`DocumentV2Session::from_bytes` → 페이지 RenderTree를 검사한다.

- 부모 조각은 PDF와 같이1–5쪽에만 존재한다. PDF stroke에서 얻은 하단을 용지 단위
  차이만 환산하여96dpi 출력1픽셀 이내로 비교한다. 이는 새 검사의 독립 판정 해상도이며
  관측 잔차에 맞춘 subpixel 허용치나 기존 검사의 완화가 아니다.
- 부모 하단은 본문 안에 있고 자식 표 상자는 부모 조각 안에 들어간다.
- 저장 줄 없는 본문29번 TAC는6쪽에 정확히 한 번 있으며 IR 선언 폭·높이와 같다.
  뒤 문단은 같은 쪽의 표 아래에 남는다. 마지막 쪽 뒤 재호출도 종료 상태다.
- 모든 쪽의 Body TextRun에서 비공백·비제어 문자별 개수를 세어 source IR의 재귀
  문단 전체와 비교한다. 본문 누락·중복의 문자 개수 축을 보호하며, 같은 문자의
  교환이나 문단 전체 순서를 보증하지 않는다. 위치·후속 문단 순서는 별도 assertion이다.

검증 소스는 HEAD `50823731a` + 앞 절편과 동일한 생산 WIP + 새 test다.
기존 `fresh-tac/source.sha256`·`wasm.sha256`의 동일성을 확인했고, 새 source는
`rustc --edition=2021 --test ... -D warnings`로 현재 library에 연결하여 실행했다.
`output/7353/r19/fresh-tac-contract/after.log`:1건 PASS,
`document-control.log`:기존 문서 흐름99건 PASS. fmt/diff whitespace도 확인했다.
직접 rustc 집중 실행은 Cargo generated suite·workspace Clippy·전체 CI를 대신하지 않으며
새 source의 manifest 준비와 제출 전 필수 lint는 통합 검증 때 남아 있다.

구현 전 보존 바이너리 `output/7353/r19/frame-budget/probe`로 동일 보존 HWP를
실행하면 문단29의 `stored TAC carrier requires unambiguous intact rows`로 거부된다
(`fresh-tac-contract/before.log`). 신규 지원의 이전 거부→현재 수용 증거다.
새 geometry assertion 전체가 이전 오조판을 검출했다고 주장하지 않는다.
생산 코드와 출력이 같으므로 앞 절편에서 승인된7쪽 시각 자료와 fresh WASM을 재사용하며
같은 이미지의 재승인 요청이나 Docker 재빌드는 하지 않는다.

다음 실물 대조군은 기존 계획/stage4의 #7243 계약에 연결된
`samples/86712_regulatory_analysis.hwp` 및
`samples/issue1891/86712_regulatory_analysis.hwpx`다. 현재 V2 실행은 두 입력 모두
**네 번째 본문 문단(인덱스3)**의 `PageNumberPos`에서 명시적 거부다.3쪽 오류가 아니다.
IR 값은 format0/position5/dash `-`이며 위치·서식 자체는 `page_number.rs::new`의
지원 범위다. 최초 원인은 `document_input.rs`가 쪽번호 선언을 `pi == 0`으로만 허용하는
진입 제한이다. `7243-hwp.log`, `7243-hwpx.log`, `7243-ir.jsonl`에 보존했다.

후속은 쪽번호 선언의 실제 적용 페이지와 호스트 빈 문단의 줄 점유를 구분하는 공통
story 소유 계약이다. 첫 문단 제한을 무조건 삭제하여 이전 페이지에까지 적용하거나,
컨트롤/빈 문단을 제거해 원본 수용을 위장하지 않는다. 이 진입 제한 뒤의 다른 미지원
기능은 아직 실행 확인하지 않았으며 #7243 표 분할 해결이나 R5 완료로 보고하지 않는다.

### 뒤 문단의 쪽번호 선언 활성화 — 계약 구현, 세로 위치 차이 보류

이번 승인 범위는 위 #7243의 최초 진입 제한이다. 기준 HEAD `50823731a`와 기존 WIP를
유지했고 Legacy/기본 엔진/표 분할·높이 규칙은 변경하지 않았다.

독립 대조군 `tests/fixtures/issue7353/late-page-number/`는 기존 정상 저장본의 용지·스타일을
재사용하되 본문을50개 ROW 문단으로 교체한 작성 입력이다. 선언 호스트25/40번은 빈 문단이다.
HWPX→한컴2020 정상 HWP 저장→그 HWP의 PDF 순서와 job/hash는 fixture README에 기록했다.
합성 LineSeg를 만들지 않았다. 첫 대조군의25번은 예상과 달리1쪽에 있으므로, 실제2쪽 선언
대조군40번을 추가했다. 기준 PDF는 각각1·2쪽 모두 번호, 1쪽 번호 없음·2쪽 `- 2 -`이다.
따라서 구역 전체 소급 적용이나 선언 시1번으로 재시작하는 구현은 하지 않는다.

값 소비 경로:

- `document_input.rs`가 단일 선언의 서식과 문단 진입 슬롯을 검증하고 `(paragraph, story)`를
  저장한다. 첫 문단만 허용하던 제한을 제거하되 글자/객체 뒤 선언·복수 선언은 미지원이다.
- `document.rs::next_page_json`이 실제 수용된 `fit.lines/fit.tables`의 소유 문단으로 활성화를
  결정한다. 번호는 구역 시작 번호+물리 페이지 index이고, 활성 상태는 SVG/JSON 성공 후
  cursor와 함께 commit한다. 실패/retry/clone에서 먼저 소비하지 않는다.
- 호스트 문단은 기존 줄 구성 결과를 그대로 사용한다. 정상 저장1000HU 줄+500HU 간격은
  ROW40과ROW42 사이의 실제 빈 줄로 남는다. story는 Body 뒤 형제이며 본문 예산을 밀지 않는다.
  분할 컷/rowspan/패딩/예약 변경은 비해당이며, story 없는 대조군과 Body 전체 equality를 검사한다.

`issue_7353_table_v2_page_number.rs`의 신규 계약은 정상 HWP와 그 IR의 HWPX 직렬화,
선언 전/후 쪽번호, 빈 줄·뒤 문단, clone/overflow/retry, 문단 중간 선언의 명시적 거부다.
HWPX 직렬화는 별도 한컴 정상 생성본이라고 주장하지 않는다. 기존 합성 테스트의 secd 슬롯
누락 때문에 HWP 저장기가 pgnp를 글자 뒤로 이동시키던 문제도 입력 쪽에서 고쳤다.
내장 SectionDef와 구역 정의를 함께 변경해 stale 정의를 저장하지 않도록 했다.
기존 입력/실패 로그는 `old-ir.jsonl`, `before.log`, 초기 `after.log`에 보존했다.
`before.log`의 정상 대조군은 이전 코드에서 paragraph25 미지원으로 실패한다. 신규 지원의
거부→수용 증거이지 번호 y 위치 수정의 FAIL→PASS 증거가 아니다.

증적 루트는 `output/7353/r19/late-page-number/`다. 초기 문서 흐름 재검사98/99 PASS의
1실패는 기존 잘못된 서식 검증보다 새 위치 검증이 먼저 실행되어 오류 이유가 바뀐 것이었다.
기대값을 완화하지 않고 서식 검증 우선순위를 보존했다. 직접 rustc 첫 시도에서 빠진
`--extern roxmltree` 때문에 발생한 컴파일 실패는 회귀 실패로 세지 않는다.

**시각 미충족:** `second-review/native-review-2.png`와 standalone overlay를 직접 확인했다.
호스트 빈 줄과 후속 ROW의 세로 위치는 보존되지만, 자동 번호는 한컴보다 약18px 위에 있다.
PDF 번호 잉크 y556.592669..566.537319pt이며 V2의 기존 run top은
본문 하단538.59pt+10/3pt다. 잉크 bbox와 run bbox는 구분한다. 내용 픽셀 보조값10.62%는
사람 판정 정확도가 아니며, 이 이미지를 시각 통과 승인 대상으로 제시하지 않는다.

추가 `margins.rs` 대조군은 margin_bottom/footer를 바꾸고 정상 한컴 저장·PDF로 확인했다.
아래 값은 PDF 잉크 top(pt)이며 저장 후 원본 여백 값이 유지됐다. 생성 HWPX 직접 PDF와
정상 저장 HWP의 PDF 관측값도 같았다.

| 아래 여백/꼬리말(HU) | PDF 잉크 top |
|---|---:|
|5669/0|556.592669|
|5669/1417|528.288009|
|5669/2835|528.288009|
|2835/0|570.744999|

기존 `footer_distance/2 + em/3`를 일반 규칙으로 주장할 수 없다. #6923 단일 관측에서
만든 호환 공식이라는 한계를 코드 주석에 명시했고, 새 관측에 맞춘 상수·조건을 추가하지 않았다.
현재 속성/문서 스타일과 한컴 번호 배치 사이의 일반 관계는 **미검증**이다. 이 위치 차이가
해소되기 전에는 시각 완료·fresh WASM 판정 준비로 보고하지 않는다. Docker/전체 CI 재실행도
하지 않았으며 이전 WASM을 현재 source의 검증으로 재사용하지 않는다.

최종 검증은 `*-final.log`에 보존했다. Native library build와 Native/WASM lib Clippy
`-D warnings`, fmt check, diff whitespace 모두 PASS다. 최종 라이브러리에 다시 연결한
정식 source 직접 harness는 쪽번호9건/문서 흐름99건/승인된7쪽 전체 문서1건, 총109건 PASS다.
이는 선택 회귀이며 generated suite manifest·workspace/all-targets Clippy·전체 CI를
대체하지 않는다. 최종 source 전체 hash는 `source.sha256`이고 같은 라이브러리로 Native
캡처를 다시 생성했다. 번호 y 차이는 그대로 재현되어 미충족 상태다.

#7243 HWP/HWPX 원본은 이제 본문 인덱스5의 `non-table cell control`로 거부된다.
`7243-controls.jsonl`에서 최초 대상은 `s0/p5/control0/cell0(0,0)/p0`의 `Field`이며
“노후계획도시 정비 및 지원에 관한 특별법 시행령” 문단이다. 필드를 삭제하거나 표 높이를
줄여 통과시키지 않았다. 다음 우선순위는 번호 y 배치 근거 확립 후 이 실물 Field의 역할과
저장 결과 경계를 연결하는 것이다. 이번 기록은 부분 구현/추가 결함 발견 보고이며
시각 승인·WASM 완료·#7243 종단 완료·R5 완료가 아니다. commit/push는 하지 않았다.

### 기본 쪽번호 하단 앵커/기준선 교정 — 독립 여백 행렬

앞 절편의18px 차이를 우선 처리했다. 기존 #6923 한 건에서 근사한 `footer_distance/2+em/3`
공식을 다른 여백에도 일반화한 것이 원인이었다. 이번에는 용지 속성→번호 줄 하단→줄 상단/
기준선→SVG TextRun의 실제 y를 연결한다. 본문 fit/표 분할과 별개인 story만 변경했다.

독립 증거는 `tests/fixtures/issue7353/footer-position/`의 작성 HWPX11개와 한컴2020 PDF다.
이전 정상 저장본의 용지·스타일을 재사용하되 본문은 한 문단으로 교체했고 저장 줄은 수동
작성하지 않았다. 아래/꼬리말 여백11조합(0,1HU,1/2/5/10/15mm 포함)과 보이지 않는 쪽
테두리 Paper/Body 기준을 대조했다. README와 `conversion.json`에 값·job·SHA를 보존했다.
별도의 정상 저장 HWP→PDF 대조는 앞 절편의3건을 재사용했다. 같은 검증을 새 실행으로 합산하지 않는다.

PDF의 잉크 bbox가 아니라 `mutool draw -F trace`의 글자 기준선 좌표를 사용했다.
A5 PDF y변환0.119935에 원점4712/4476/4594/4948을 곱하면 각각
565.13372/536.82906/550.98139/593.43838pt다. 글자10pt의 기본1000HU 줄과850HU 기준선,
용지/device 반올림을 함께 대조하면 다음 관계가 모든 대조군에서96dpi1픽셀 이내다.

- 꼬리말 여백>0: 줄 하단=용지 하단−아래 여백.
- 꼬리말 여백=0: 줄 하단=용지 하단−아래 여백/2.
- 줄 상단=줄 하단−10pt, 기준선=줄 상단+8.5pt.

이는 HWP 사양에 명시된 수식이라는 주장이 아니라 **기본10pt 경로의 독립 출력 호환 규칙**이다.
[한컴 공식 도움말](https://help.hancom.com/hoffice_mac/2022/ko-KR/hwp/format/pagenumber.htm)은
자동 번호가 영문 이름 `Page Number` 스타일의 글자 모양을 따른다고 설명한다. 새 대조군에는
그 스타일이 없고 #6923은10pt 기본 크기임을 확인했다. 사용자 정의 크기/서식 지원은 이 수정에
포함하지 않았다. 도움말의 구역 적용 설명과 수동 배치 pgnp의 적용 페이지도 같은 근거라고
섞지 않는다. UI 생성 단계에서 컨트롤을 어디에 삽입하는지 이번에는 검증하지 않았다.

생산/소비 경로는 `PageNumberStory::new`가 PageDef의 정수 여백으로 `line_bottom`을 만들고,
`render`가 같은 값으로 TextLine/TextRun top·height·baseline을 만드는 구조다.
0 여부는 부동소수점 좌표 차이로 추측하지 않는다. 기본 줄 높이는10pt이며 뒤 내용이 없는
story에 임의의1.2배 bbox를 남기지 않는다. `document.rs`는 이 노드를 Body 다음에 그대로
추가하고 `SvgRenderer`가 그 기준선을 소비한다. 이후 앵커 덮어쓰기/clamp는 없다.
작은 양수 꼬리말 여백에서 번호가 본문 영역과 겹칠 수 있어도 본문을 강제로 밀지 않는다.
0여백도 번호가 용지 안에 들어가는 입력이다. 분할 컷/rowspan/패딩/flow 예약 변경은 비해당이다.

정식 `issue_7353_table_v2_page_number.rs`에11개 독립 PDF 기준선 행렬 검사를 추가했다.
수정 전 `before.log`: b20f0 실제735.8978px 대 PDF753.5116px로 FAIL.
수정 후 `final.log`: 번호10건 PASS. 합성 geometry 계약은 위 독립 관계에 따라 갱신했으며
baseline/ignore를 완화하지 않았다. 기존 “0여백이면 용지 밖” 테스트의 잘못된 전제는 b0f0
PDF로 반증했다. 대신10pt 번호보다 물리 용지가 작고1pt 본문은 들어가는 입력에서
거부/retry의 비소비를 검증한다. 실험 중 `after.log`는 Native build 완료 전에 링크된
이전 라이브러리의 결과이므로 최종 검증에서 제외하고 빌드 종료 후 다시 연결했다.

최종 Native library build, Native/WASM lib Clippy `-D warnings`, fmt/diff check는 PASS다.
같은 라이브러리의 문서 흐름99건/승인된 전체 문서1건도 PASS, 총110건 선택 회귀다.
직접 rustc harness이며 전체 CI·generated suite 정책·workspace all-targets 제출 검증은 아니다.
2쪽 대조군의 모든 Body와 기존 승인7쪽 문서의 모든 Body를 수정 전 JSON과 직접 비교해
완전 동일함을 확인했다.7쪽 문서는 번호 baseline만−0.87778px 이동한다.

증적은 `output/7353/r19/footer-position/`에 모았다. Native `second-review`2쪽과
`b20f5-review`1쪽의 review를 직접 열어 세로 차이 해소를 확인했다. 자동 잉크 보조값은
각각10.62%,13.68%이며 글꼴 폭/줄표 glyph 차이가 남아 있고 사람 판정 정확도가 아니다.
동일 본문 출력이라도 번호가 달라졌으므로 fresh Docker WASM을 새로 빌드하여 backend
비교를 진행한다. 이 기록 시점에는 최종 WASM/메인테이너 시각 판정을 완료로 세지 않는다.

후속 실행: Docker `docker compose --env-file .env.docker -p rhwp run --rm wasm`은
7분24초에 exit0으로 완료됐다(`docker-wasm.log`). 최종 WASM SHA256은
`ba3bf72309abc4585434c9229e96e5efafd94083d3cc69bcc467913bca388a38`이며,
code head `50823731a`+WIP의 `source.sha256`과 일치하는 소스로 빌드했다.
`node output/7353/r19/footer-position/capture.mjs --wasm` 및 `--matrix --wasm`,
`--full --wasm`을 실행했다. 각 `run.json`에 입력/PDF/WASM SHA와 소스 manifest가 있다.

| 브라우저 대조 | 페이지 | SVG | RenderTree 최대 수치 차이 | 비수치 차이 |
|---|---:|---|---:|---:|
| second-review |2|전부 동일|2.22e-16|0|
| b20f5-review |1|동일|1.42e-14|0|
| full-review |7|전부 동일|2.28e-13 미만|0|

최종 포맷 이후 번호 테스트를 다시 링크/실행해10 PASS를 확인했다(`final.log`).
fresh WASM의 `second-review/wasm-review-2.png`, `b20f5-review/wasm-review-1.png`,
`full-review/wasm-review-1.png`와 각각의 standalone `wasm-overlay-N.png`를 직접 열었다.
2쪽의 기존18px 세로 차이는 사라졌고 ROW40/42 사이 빈 줄은 유지된다. 글꼴 폭과 번호
양옆 줄표의 외형 차이는 남는다. 전체7쪽 대조군1쪽에는 기존 로고 표시·글꼴·외곽선 차이도
보이며 이번 번호 수정으로 해소했다고 주장하지 않는다. 수정 전후 Body JSON은 동일하다.
전체1쪽 잉크 자동 보조값은36.79%이며 사람의 시각 판정 정확도가 아니다.

이번 절편은 기본 쪽번호 위치의 구현/선택 검증/WASM 증적 준비까지 완료했다.
메인테이너에게 2쪽 번호 기준선과 빈 줄 보존을 시각 판정 요청한다. 원본 #7243의 Field
지원, 사용자 정의 번호 스타일, R5 전체 완료는 아직 미완료다. 전체 CI·commit·push 및
기본 엔진 전환은 수행하지 않았다.

### 입력된 누름틀의 저장 표시 결과 — 착수

작업지시자가 앞 절편의 쪽번호 위치 시각 판정을 통과시켰다. 다음은 #7243 원본 첫 제목
셀의 ClickHere 필드다. HWP/HWPX 모두 properties=32768, 범위0..26, 시작/끝8 UTF-16단위 슬롯,
저장 두 줄(text_start0/25), 표시 문자열은 “노후계획도시 정비 및 지원에 관한 특별법 시행령”이다.
HWP 사양 표153의 bit15(내용 수정됨), 기존 `Field::is_dirty`, 원본 한컴 PDF에 제목이
인쇄된 사실을 독립 근거로 삼는다. 현재 계산식 전용 `fields::stored_formula_result`가
이 값을 거부한다. Field를 삭제하거나 안내문을 제목 대신 출력하지 않는다.

이번 범위는 입력된 비어 있지 않은 누름틀의 저장 결과 재생이다. 유효한 범위/슬롯/LineSeg를
공통 검증한 뒤 같은 텍스트 구성 결과를 측정과 paint가 소비한다. 편집 후 재조판, 초기 안내문,
빈 누름틀, 하이퍼링크, 중첩/문단을 넘는 필드의 의미는 무조건 수용하지 않는다.
원본에는 입력된 누름틀59개, 초기 누름틀3개, 하이퍼링크117개가 있어 이번 결과를 원본 전체
수용으로 세지 않는다. 원본 앞6문단과 제목 표를 보존한 분리본을 정상 한컴 저장/출력해
지원한 제목의 줄바꿈·좌표·표 경계 증거로 사용한다. 분리본과 원본 전체의 주장은 구분한다.

추가 피드백: 공공기관의 누름틀 범위에 웹기안 SDK로 다른 문서 조각이 채워질 수 있다는
작업지시자의 관찰을 고려한다. 누름틀은 문자열만 담는 상자로 일반화하지 않으며, 필드 범위와
내부 문단/표/그림의 소유·저장 줄·페이지 분할은 별도 축이다. 현재 파일로 SDK 생성 출처를
단정하지 않는다. 이번 단순 저장 텍스트 경로는 내부 객체/중첩 슬롯/다른 문단의 끝 마커를
명시적으로 거부하고 원본을 보존한다. 향후 복합 조각 지원을 필드 삭제·평탄화로 대신하지 않는다.

구현은 `fields.rs::stored_result`에서 기존 계산식과 입력된 ClickHere의 적격성/범위/UTF-16
슬롯을 검증하고, `ir.rs`가 이를 비기하 marker로 소유한다. `text_ir.rs`→`text.rs`의 기존
저장 줄 구성 결과가 FlowLine의 점유와 TextLine/TextRun paint를 함께 만든다. marker를
지우거나 텍스트 오프셋을 다시 만들지 않으며 뒤에서 별도 높이/원점을 보정하지 않는다.
본문 직접 필드, 복합 객체 혼재, 편집 세션 지원을 확장한 것이 아니다. 분할 컷/rowspan/
예약/이어받기 알고리즘 변경은 비해당이다.

정식 `tests/cases/issue_7353_filled_field.rs`의 정상 저장 분리본 문서 및 원본 HWP/HWPX
선택 제목 셀은 수정 전 라이브러리에서 `non-table cell control`로 거부됐다(`before.log`).
새 지원의 이전 거부→현재 수용 증거이며 초기 필드 가드의 오류 문자열 차이를 결함 재현으로
세지 않는다. 첫 테스트 작성의 Section 직렬화 컴파일 오류도 재현 증거에서 제외했다.
문서에 상단 표가 하나 더 있으므로 검사 대상은 제목 셀로 명확히 지정했다.

수정 후5건은 두 줄 문자열/48px 원점 간격/40px 줄 높이/34px 기준선, 실제 셀 포함,
원본 IR 보존, 종단 반복 호출, 초기 상태·빈 값·저장 정보 무효화·슬롯 오류·고아 끝·중첩
객체의 거부를 검사한다. 분리본 문서 경로의 절대 글자 x/y 및 셀 상자는 독립 PDF trace의
좌표를96dpi로 환산하여1px 이내를 검사한다. 선택 preview의 임의 원점을 PDF 위치로
주장하지 않는다. 앞6문단 외 원본 내용은 분리본에 없다는 한계도 README에 명시했다.

Native build, Native/WASM lib Clippy `-D warnings` PASS. 최종 선택 검사는
filled_field5 + document_flow99 + page_number10 + fresh_tac_full_document1 =115 PASS다.
기존 계산식의 다른 종류 거부 반례는 이제 지원하는 ClickHere 대신 여전히 미지원인
Hyperlink로 옮겼고, 초기 ClickHere 거부는 새 검사에서 보존한다. 기존 baseline/ignore는
변경하지 않았다. 직접 rustc harness이며 전체 CI/generated suite/workspace 검증은 아니다.

Native 비교 PNG에서 제목의 “관/한” 줄 경계, 앞 빈 문단, 셀 상자가 보존됨을 직접 확인했다.
글꼴 외형/획/줄표 차이는 남는다. 자동 잉크 보조값36.14%는 사람 판정 정확도가 아니다.
원본 HWP/HWPX를 재실행하면 p5의 필드는 수용하고 p8에서
`stored text requires intact single-segment rows`로 명시적 거부된다(physical8쪽이 아님).
`output/7353/r19/field-result/`의 원본 로그와 source manifest로 연결하며 원본 전체 완료는 아니다.
Docker fresh WASM 빌드는7분18초에 완료했다(`docker-wasm.log`). 이어
`node output/7353/r19/field-result/capture.mjs --wasm`으로 같은 정상 저장 HWP를
브라우저 DocumentV2 API에서 실행했다. Native/WASM SVG는 동일하고 비수치 차이는0,
최대 수치 차이는2.274e-13이다(`review/backend-comparison.json`). 전체 Studio 편집
세션을 검증한 것은 아니다. source HEAD는50823731a+WIP이며 빌드 전후
`source.sha256` 일치를 확인했다. 입력/PDF/WASM 해시는 `review/run.json`에 고정했다.

fresh WASM `review/wasm-review-1.png`와 `review/wasm-overlay-1.png`를 직접 열어
한컴 PDF 대비 제목2줄의 관/한 경계, 앞 공백 영역과 두 표 외곽을 확인했다. 글꼴 외형/획과
쪽번호 줄표 차이는 남으며 자동 잉크 보조값36.14%를 사람 판정으로 승격하지 않는다.
작업지시자가 이번 제목 절편의 시각 판정을 통과시켰다. 복합 문서 조각/원본 전체는 미지원 또는
미검증으로 유지하고 필드 내용을 삭제·평탄화하지 않는다. 전체 CI, commit/push,
기본 엔진 전환은 수행하지 않았다.

### 본문 자리차지 표의 너비0 저장 선언 줄

작업지시자가 입력된 누름틀 제목 절편의 시각 판정을 통과시켰다. 다음 원본 p8은
TopAndBottom/ParaTop/offset0 표와 너비0의 저장 선언 줄이다. source 줄은 높이1600HU,
줄간격320HU, vpos25995이며 뒤의 독립 빈 문단 p9는 vpos42719다. 표의 내부8문단은
1600HU 높이+480HU 간격으로 이어진다. 표 높이16442HU와 상하 바깥여백 각141HU의
합16724HU가 p8→p9 원점 차이와 같다. 선언 줄의 높이를 없애는 것이 아니라 표와 같은
원점의 점유로 보존하고, p9의 실제 빈 줄은 별도로 전진시켜야 한다.

이전 document_input은 표 컨트롤을 분리한 뒤 일반 TextComposer로 너비0 줄을 보내 거부했다.
셀 경로의 cell_anchor::compose→ExcludedTable→FlowBlock::AnchoredTable 계약을 본문에 연결했다.
일반 저장 텍스트의 너비 검사를 완화하거나 별도 높이 예외를 만들지 않는다. source IR은 보존한다.
어울림/TAC/양수 너비/세로offset이 있는 다른 소유 경로는 이 계약에 넣지 않는다.

실제 경로: `cell_anchor.rs:99`에서 저장 선언 줄의 높이/advance/여백을 생산하고,
`document_input.rs:282`가 prepared plan과 함께 AnchoredTable로 결합한다.
`body_flow.rs::BodyCursor::fit`→`flow.rs:168`은 host와 첫 자식 조각을 함께 수용하고
자식 컷/요구높이/예약을 `TableCursor::fit_with_page_height`에서 받는다. 실패하면 host도
소비하지 않는다(:253). 수용된 조각 높이와 host 점유의 합집합만 전진하고(:209), 마지막
물리 여백은 anchor_tail로 이어받는다(:268). `document.rs:159`는 확정 placement를
paint.build_node에 전달한다. paint에서 다시 컷/원점을 선택하지 않는다. 기존 일반 본문
anchor와 TAC 경로는 변경하지 않았으며 이번 경로의 rowspan 변경은 비해당이다.

독립 증거는 원본 첫10문단을 보존한 HWPX를 정상 한컴 저장한 HWP와 같은 HWP의 PDF다.
추가 `marked-*` 대조군은 그 뒤에 “목차 표와 빈 문단 다음 위치 확인”을 새 문단으로 넣어
정상 저장/출력했다. 보이지 않는 빈 영역만 제시하지 않고 뒤 빈 문단의 효과를 확인하기 위함이다.
이 문장은 원본 내용이 아니며 원본 전체 통과의 증거도 아니다. 생성 명령/receipt/입력·PDF·
hash는 `output/7353/r19/body-exclusion/` 및 fixture README에 연결한다.

`tests/cases/issue_7353_body_exclusion.rs`5건은 실제 문서 경로의 두 줄 원점 차이,
셀 외곽/8개 줄(5개 빈 줄 포함)/독립 PDF 글자 기준선, 원본 HWP/HWPX 첫10문단,
뒤 확인 문단과1920HU 빈 줄 전진, 합성 작은 용지의 이어받기/host1회/종료와
잘못된 폭·어울림·offset 등 거부를 검사한다. 수정 전3개 수용 경로가 p8에서 의도한
stored-row 거부로 실패했고 수정 후 통과한다(`before-final.log`, `after.log`). 초기
테스트의 Box 할당 컴파일 오류, CellBreak+Center 미지원 조합, 컨트롤 제거 뒤 serializer의
빈 문단 정상화는 대상 결함 재현에 포함하지 않는다. 합성 분할 대조군은 CellBreak/Top을
명시했으며 한컴 동일 페이지 수의 증거로 쓰지 않는다.

선택 검사120 PASS(body_exclusion5/filled_field5/document_flow99/page_number10/fresh_tac1),
Native build와 Native/WASM lib Clippy `-D warnings` PASS. 직접 rustc harness이며 전체CI/
generated suite/workspace 제출 검증이 아니다. Native compare와 overlay에서 목차 줄 위치,
뒤 확인 문단 위치를 직접 확인했다. 확인 문단의 PDF glyph 시작 x≈177.99px와 V2≈174.97px의
약3.02px 차이는 남는다. 이 문단은 CENTER이며 글자 폭이 다른 대체 글꼴의 시작 위치까지
일치하는 계약은 성립하지 않았다. 임계치를 늘리지 않고 수평 일치 주장을 철회하여 정렬 중심과
독립 PDF 세로 기준선(1px 이내)을 분리 검사했다. 기존 목차의 왼쪽 원점/PDF 좌표 검사는
그대로 통과한다.

fresh Docker WASM은 `docker compose --env-file .env.docker -p rhwp run --rm wasm`으로
7분12초에 완료했다. WASM SHA256은
`d46b92bb50bce11f40f5c7969c1b1b2c83e2cd57bdf8748efe7b1c7a099bcbc3`이다.
`node output/7353/r19/body-exclusion/capture.mjs --wasm` 및 같은 명령의
`--marked --wasm`으로 정상 저장 입력 두 건을 fresh 브라우저 DocumentV2에서 실행했다.
두 입력 모두 Native/WASM SVG가 동일하고 비수치 차이는0, 최대 수치 차이는
2.274e-13이다. source HEAD50823731a+WIP와 입력/PDF/WASM 해시는 각 `run.json`에
기록했으며 빌드 후 `source.sha256`31개 항목 일치와 `git diff --check`를 확인했다.

`output/7353/r19/body-exclusion/review-contents/` 및 `review-marked/`의
`wasm-review-1.png`, `wasm-overlay-1.png`를 직접 열어 목차3줄과 뒤 확인 문단의
세로 위치를 대조했다. 내용 픽셀 중심 자동 일치율 보조값은 각각28.36%,25.91%이며
사람 판정 정확도가 아니다. 글꼴 외형과 위의 가운데 정렬 글자 시작점 차이는 남는다.
marked 첫 캡처는 Chrome 시작 중 종료되어 `capture-wasm-marked.log`에 보존했고,
동일 코드·입력의 재실행은 통과했다(`capture-wasm-marked-retry.log`). 이를 조판 실패나
수정 전 결함 재현으로 세지 않는다. 전체 Studio 편집 세션 검증은 아니다.
이번 절편의 작업지시자 시각 판정은 통과했다. 전체 CI, commit/push, 기본 엔진 전환은
수행하지 않았다.
원본 전체는 HWP/HWPX 모두 p10에서 `inconsistent grid boundary`로 멈춘다. p10은 본문
문단 인덱스이며 물리10쪽이 아니다. 이 경로의 원인은 아직 미조사이며 본 절편 완료에 포함하지 않는다.

### 작성자 표의 공유 오른쪽 경계와 말미 공백 측정

직전 본문 자리차지 선언 줄 절편의 시각 판정을 작업지시자가 통과시켰다.
다음 원본 p10(물리10쪽 아님)의6행11열 작성자 표는 앞5행 너비 합47,953HU,
마지막 행47,958HU, 표 선언 너비47,958HU다. HWP/HWPX와 첫12문단의 한컴 정상
재저장본에서 모두 같았다. 정수 경계 완전 일치라는 V2의 가정이 지원을 막았다.
한컴 PDF는 오른쪽 외곽이 연결된다.5HU를 화면 기반의 임의 tolerance로 허용하지 않는다.

`grid.rs::resolve`는 내부 경계를 그대로 검사하고, 서로 다른 행 끝은 표 선언 너비와
최대 너비의 완전 행이 일치할 때만 공유 오른쪽 경계로 확정한다. 짧은 행의 마지막 셀
잔여를 CellTrack.width에 반영한다. 선언 너비가 없거나 완전 행과 모순되거나 내부 경계가
충돌하면 거부한다. 모든 행의 너비가 같은 기존 경로는 바꾸지 않는다. 원본 Cell.width를
수정하거나 열을 비례 축소/확대하지 않으며, 병합 내부 미관측 열도 만들어 내지 않는다.

소비 경로: grid의 CellTrack.width→`ir.rs`의 inner_width/cell_lane_width와 composer→
`TableContentPlan::from_grid_rows`→TableCursor의 요구 폭/CellPlacement→TextPaint의
확정 셀 외곽과 텍스트 배치다. 그 뒤 별도 열 폭 재계산은 없다. 기존 flow/row_groups의
컷·예약·이어받기 알고리즘은 변경하지 않았다. 저장 줄 측정 및 fresh 줄 구성은 같은
확정 폭을 받는다. source IR·Legacy 표 기하 경로는 그대로다.

grid 수용 후 실제 paint에서 “ 정책책임자 직위 : ”의 가시 advance가 줄 끝287.146667px를
0.5px 초과했다. 전체 run은 소수 폭인데 오른쪽/가운데 정렬의 말미 공백 helper는 정수로
반올림했다. 공통 `paragraph_layout.rs::trailing_space_width_after_last_inline_object`가
`estimate_text_width_exact`를 사용하도록 변경했다. 스타일별 suffix/마지막 TAC 이후 범위/
가운데 정렬의 밑줄 공백 제외 조건은 유지한다. 이 helper는 Legacy 문단·글상자에서도
소비하므로 V2만의 변경으로 주장하지 않는다. 정렬 offset→emit_line_runs의 소수 측정→
V2 painted_inline_ends/각 backend replay가 같은 advance를 사용한다. clamp나 guard 완화,
폰트 교체, 저장 줄바꿈 변경은 없다.

독립 대조군은 `tests/fixtures/issue7353/grid-terminal/README.md`에 생성 절차·정상 저장
MCP job·hash·PDF 좌표를 기록했다. 원본 첫12문단 이외를 포함하지 않는다. 수정 전
grid 계약2건은 의도한 경계 거부로 실패하고 guard1건은 통과했다(`grid-boundary/before.log`).
grid만 수정한 뒤 문서 paint 계약은 정렬 오류로 실패했다(`before-text.log`). 진단 로그의
변수명 오타로 발생한 일시적 컴파일 실패는 결함 재현으로 세지 않았고 임시 진단 출력은 제거했다.

최종 검사는 실제 셀 좌표/PDF 경계·25개 셀 소유·문서 종료, IR 원본 보존, 내부 경계
충돌/선언 누락/모순 거부, 소수 글자 크기와 혼합 글자모양 suffix의 정렬 불변식을 포함한다.
Native compare/standalone overlay에서 작성자 표·작성일·정책책임자 행의 위치를 직접 확인했다.
자동 잉크 일치율20.97%는 사람 판정 정확도가 아니다. 글꼴 외형/굵기/자폭 차이는 남는다.
원본 전체는 HWP/HWPX 모두 본문 p12의 `explicit body page/column break`에서 멈춘다.
현재 절편은 원본 전체 수용이나 R5 완료가 아니다.

`bash output/7353/r19/grid-boundary/tests.sh`의 최종 선택 회귀 검사는177 PASS다
(`tests-final-summary.log`). grid_terminal4/colspan10/rowspan_roundoff2/text37,
기존 정렬 회귀 #7081(2)/#5820(1)/#6173(1), body_exclusion5/filled_field5/
document_flow99/page_number10/fresh_tac1을 포함한다. 직접 rustc harness이며 전체 CI나
generated suite/workspace 제출 검증으로 보고하지 않는다. Native build, Native/WASM lib
Clippy `-D warnings`, `cargo fmt --all -- --check`, `git diff --check` 모두 통과했다.
HEAD50823731a+WIP의 `source.sha256`32개 항목(table_v2와 공통 paragraph_layout)을
최종 코드와 대조해 모두 일치했다.

fresh Docker WASM은 `docker compose --env-file .env.docker -p rhwp run --rm wasm`으로
7분15초에 완료했다(`grid-boundary/docker-wasm.log`). WASM SHA256은
`282c1ae8bc4387f79a902a215aedba507dde142097b31794f1f9f3f3b4d4e6ce`다.
`node output/7353/r19/grid-boundary/capture.mjs --wasm`으로 같은 정상 저장 HWP를
브라우저 DocumentV2에서 실행했다. Native/WASM 모두1쪽, SVG 동일, render tree의 수치/
비수치 차이0이다(`review/backend-comparison.json`). source HEAD+WIP/입력/PDF/WASM
hash는 `review/run.json`에 연결한다.

`review/wasm-review-1.png`와 `review/wasm-overlay-1.png`를 직접 열어 작성자 표의
오른쪽 외곽·내부 열·작성일·정책책임자 행 및 앞 목차와의 간격을 대조했다. 자동 잉크
일치율20.97%이며 글꼴 외형 차이는 남는다. 이번 절편은 작업지시자 시각 판정 통과다.
원본 전체·Studio 편집 세션·전체 CI 검증, commit/push, 기본 엔진 전환은 수행하지 않았다.

### 본문 문단 앞 명시적 쪽 나누기

작성자 표 절편의 시각 판정 통과와 다음 진행 승인을 받았다. 원본 HWP/HWPX의 p12
`< 규제 개요 >`는 `ColumnBreakType::Page`, 첫 저장 줄 y=0이다. HWP5 사양 표59의
쪽 나누기와 정상 한컴 저장/PDF2쪽의 문단 시작을 독립 근거로 삼는다. 기존 V2는 본문
나누기를 일괄 거부했으므로 기존 오배치 수정이 아닌 명시적 거부 경로의 신규 지원이다.

`document_input::prepare`가 문단 진입의 `(flow block, 앞 anchor 수)`를 BodyPlan에
기록한다. 로컬 문단 composer에만 나누기를 소비한 clone을 전달하며 원본 IR·LineSeg를
변경하지 않는다. 일반 문단, TAC, zero-width 자리차지 선언 줄 모두 같은 진입 경계다.
`BodyCursor::fit`은 다음 anchor와 쪽 경계 중 앞쪽까지만 `FlowCursor::fit_until`로
측정/예약한다. 같은 block의 선행 anchor는 먼저 처리하고 pending 표는 기존 컷으로
이어받은 뒤 경계를 소비한다. 다음 문단은 다음 fit의 본문 원점에서 시작한다.
`DocumentV2Session::next_page_json`은 이 fit의 lines/tables를 그대로 paint하고 성공 후
cursor를 commit한다. paint에서 쪽을 다시 추측하거나 좌표를 clamp하지 않는다.

분할 경로는 기존 일반 Table/AnchoredTable/TAC의 소유 컷·요구 높이·누적 예약을 그대로
소비하며 flow/row_groups의 표 분할 알고리즘은 바꾸지 않았다. 달라지는 것은 body fit의
종료 경계다. 앞 표가 현재 공간에 안 맞으면 pending에 남고 앞 표의 마지막 조각이 끝난
쪽에서 나누기를1회 소비한다. 원래 높이/부분 컷/rowspan 요구 높이 자체는 비변경이다.
첫 문단 Page와 본문 단·다단·구역, 셀 내부 나누기는 명시적 거부를 유지한다.

정상 저장 대조군은 `tests/fixtures/issue7353/page-break/README.md`에 생성·MCP job·
해시·독립 PDF 좌표를 기록했다. 첫14문단 시도는 p13 표 내부의 명시적 나누기 거부로
멈췄다(`probe.log`). 이 입력/정상 저장본/변환 영수증을 보존하고, 성공 대조는 원본
첫13문단으로 명시했다. p13의 속성을 삭제해서 원본 전체를 통과시키지 않았다. 현재
원본 HWP/HWPX 모두 p13에서 같은 미지원 이유로 멈춘다(본문 문단 인덱스이며 물리13쪽 아님).

기존 바이너리로 이 정상 저장본의 p12 거부를 재현했다(`before-native.log`). 작은 계약
3건도 수정 전 같은 거부로 실패했다(`before.log`). 임시 rustc 의존성 누락과 테스트의
빈 TextRun/누적 emitted_pages 가정 오류, 지원 밖 기본 Justify TAC 입력, source 바깥여백을
빠뜨린 기대값은 결함 재현으로 세지 않는다. HWPX로 직렬화되지 않는 구역/다단 반례는
HWP로 인코딩하고 재파싱 속성 보존을 assert한 뒤 검사한다.

Native2쪽에서 제목의 실제 원점/기준선과 쪽 번호를 직접 확인했다. 첫 쪽 작성자 표와
공백 문단은 보존된다. 자동 잉크 일치율은1쪽20.97%,2쪽8.77%이며 글꼴 차이가 남는다.
2쪽은 제목과 쪽 번호만 있으므로 흰 영역 픽셀 일치율을 전체 조판의 증거로 사용하지 않는다.
`bash output/7353/r19/page-break/tests.sh` 최종 선택 회귀186건 통과
(`tests-final-summary.log`): document_flow108(신규 경계9), grid_terminal4/colspan10/
rowspan_roundoff2/text37/기존 정렬4/body_exclusion5/filled_field5/page_number10/fresh_tac1.
Native build, Native/WASM lib Clippy `-D warnings`, fmt/diff 검사도 통과했다. 직접 rustc
harness이며 generated suite/workspace/전체 CI 검증으로 보고하지 않는다. 전체 CI·commit/push·
기본값 전환은 이 절편의 수행 범위가 아니다.

fresh Docker WASM은 `docker compose --env-file .env.docker -p rhwp run --rm wasm`으로
7분24초에 완료했다(`page-break/docker-wasm.log`). WASM SHA256은
`87ee2351736983cdcaab5a3ddb247248664683c814f1d687b47654f91f1c0ba4`다.
`node output/7353/r19/page-break/capture.mjs --wasm`으로 같은 정상 저장본을 실행했다.
Native/WASM 모두2쪽, 두 SVG 동일, render tree의 수치/비수치 차이0이다.
source HEAD50823731a+WIP와 입력/PDF/WASM hash는 `review/run.json`, 소스32개 hash는
`source.sha256`에 보존했다. 빌드 뒤 소스 일치 및 diff 검사를 다시 확인했다.

`review/wasm-boundary-review.png`는1쪽 하단 y640..1000px와2쪽 상단 y40..180px를
세 backend 패널에 똑같이 잘라 나란히 배치한 추가 판정 자료다. 숨은 좌표 정합이나 확대는
없으며 `boundary.mjs`에 절차가 있다. 이 이미지와2쪽 `wasm-review-2.png`,1·2쪽
`wasm-overlay-{1,2}.png`를 직접 확인했다. 전체 페이지의
`wasm-compare-{1,2}.png`/`wasm-review-{1,2}.png`도 같은 폴더에 보존했다.
제목의 새 쪽 원점, 앞 표와 공백 문단의 보존, 쪽 번호를 확인하는 제한된 시각 자료이며
뒤 규제 개요 표의 정상화 증거가 아니다. 이번 절편은 작업지시자 시각 판정 통과다.

### 셀 첫 문단의 일반 1단 정의

본문 쪽 나누기의 시각 판정 통과와 다음 진행 승인을 받았다. 다음 p13 거부를 추적한
결과 실제 속성은 Page가 아니라 r6c2/p0와 r30c1/p0의 `MultiColumn`이었다.
둘 다 첫 컨트롤은 일반 1단·동일 너비·간격0·구분선 없음의 `ColumnDef`다.
원본과 한컴 재저장본에서 같으며, 첫14문단 PDF2쪽의 자식 표와3쪽의 규제정비 셀은
앞뒤 내용과 같은 셀 흐름 안에 놓인다(`cell-break/source.log`, `break-2020.pdf`).
따라서 쪽 경계를 추가하지 않고 셀 story의 초기 단일 영역 선언을 처리한다.

`ir.rs::initial_cell_column`(80행)은 첫 문단/첫 source slot의 정상 단일 영역만
수용한다. 실제 다단·폭/간격/구분선 변경·중간 선언·Page/Column/Section은 거부한다.
`bind_table`266행은 로컬 clone의 `column_type`만 소비하고 원본 IR/문자축/LineSeg/
컨트롤 배열은 유지한다. 289/308행은 구조 슬롯의 소비를 기록하며 표 소유자로 취급하지
않는다. `text_ir.rs`138행의 일반 텍스트는 구조 컨트롤만 없는 로컬 view로 기존 composer에
전달한다. TAC는169행에서 구조 슬롯을 포함한 원본 배열을 기존 `tac::compose`에 전달한다.
따라서 `ColumnDef=ci0`, 자식 `Table=ci1`의 소유 번호가 보존된다.

측정은 기존 ParagraphItem/FlowBlock을 소비하고, `bind_paint`는 동일 ParagraphPaint
slot recipe로 원본 ci1의 자식 paint를 바인딩한다. 구조 정의가 추가 줄·Space·페이지
경계·표 높이를 만들지 않는다. fit/cut/reservation/continuation/rowspan 알고리즘과
최종 원점 보정은 이번 절편에서 변경하지 않았다. 이 지원을 일반 다단이나 중첩 표
페이지네이션 전체 지원으로 해석하지 않는다.

독립 대조군은 `tests/fixtures/issue7353/cell-column/README.md`에 생성·MCP job·
hash·PDF 좌표를 기록했다. 원본 두 셀을 각각1x1 부모로 분리하고 설명 제목을 추가했으며
셀 내용/단 정의/자식 표/글자모양은 유지했다. 저장 줄 캐시를 제거한 뒤 한컴에서 정상
재조판·저장했다. 최초 생성본은 제목의 raw Page 비트를 남기는 생성 오류가 있어 보존하고
v2에서 바로잡았다. 이 생성 오류와 테스트의 의존성 누락/노드 traversal/고정 간격 가정
오류는 제품 결함 재현으로 세지 않는다.

기존 Native 바이너리는 최종 정상 저장 fixture에서 의도한
`explicit paragraph page/column break`로 거부한다(`before-v2.log`). 변경 후에는1쪽이다.
`tests/cases/issue_7353_table_v2_document_flow.rs`의 신규 `initial_cell_*`4건은
실제 부모/자식 표의 최종 bbox, 뒤 제목/문단의 기준선, 자식 텍스트 누락·중복, 빈 첫 문단의
점유를 검사한다. PDF path/글자 기준선은0.5px 이내 비교하고, fresh 셀 재조판은 암묵적
단일 영역 대조와 같은 paint 좌표를 내는지 별도로 검사한다. 변형 계약은 한컴 일치 자료와
구별한다. Native review/standalone overlay를 직접 열어 표 외곽·두 행·뒤 제목·규제정비
문단을 확인했다. 잉크 중심 보조값50.81%이며 폰트 외형 차이는 남는다.

`bash output/7353/r19/cell-break/tests.sh`: 선택201건 PASS
(document_flow112, nested_text11, grid_terminal4, colspan10, rowspan_roundoff2,
text37, 기존 정렬4, body_exclusion5, filled_field5, page_number10, fresh_tac1).
Native build 및 Native/WASM lib Clippy `-D warnings`도 통과했다. 직접 rustc harness이며
전체 CI/generated suite/workspace 검사로 보고하지 않는다. 소스는 HEAD50823731a+WIP다.

원본 첫14문단은 다음 제한인 `stored body anchor outside body`에서 멈춘다
(`full-prefix.log`). 한컴 저장 전 HWPX 대조군도 기존 anchor 조건에서 거부된다
(`fresh-hwpx.log`). 정상 저장 HWP와 fresh HWP 계약의 통과를 이 입력들 또는 원본 전체
통과로 바꾸어 보고하지 않는다. 기본 엔진 전환·commit/push는 수행하지 않았다.

fresh Docker WASM은 `docker compose --env-file .env.docker -p rhwp run --rm wasm`으로
7분20초에 완료했다(`cell-break/docker-wasm.log`). WASM SHA256은
`f690e79cda0ea830832057d585f8f48963e9d0f67eceb1c6bff979f0210171ec`다.
`node output/7353/r19/cell-break/capture.mjs --wasm`으로 같은 정상 저장 HWP를 브라우저
DocumentV2에서 실행했다. Native/WASM 모두1쪽, SVG 동일, 수치/비수치 차이0이다.
`review/run.json`에 source HEAD+WIP, 입력/PDF/WASM hash를 보존했다. 소스31개 manifest
`source.sha256`와 실제 소스의 일치, fmt/diff 검사도 확인했다.

`review/wasm-detail-review.png`는 같은1쪽 x60..560/y60..310px 영역을 Native와 같은
크기로 보여주는 판정 자료다(`crop.mjs --wasm`, 이동 보정·확대 없음). 이 자료와
`review/wasm-overlay-1.png`를 직접 열어 자식 표 원점·두 행·부모 외곽·뒤 제목·하단 문단의
보존을 확인했다. 전체 `wasm-compare-1.png`/`wasm-review-1.png`도 보존했다.
잉크 보조값50.81%이며 글꼴 굵기/외형 차이는 남는다. 이번 절편은 작업지시자 시각 판정 통과다.

### 본문 너비보다 넓은 저장 자리차지 표

셀 초기 일반1단 정의의 시각 판정 통과 후 다음 거부를 조사했다.
`stored body anchor outside body`는 세로 분할이 아니라 가로 수용 조건이었다.
원본과 첫14문단 정상 재저장본은 본문48190HU, 표49204HU, 좌우 바깥여백141HU다.
96dpi에서 본문642.533px, 표656.053px다. 원본 대응 PDF2쪽에서도 오른쪽 외곽은
본문 밖·용지 안에 있다. 표 축소나 본문 글줄 폭 확대 없이 이 절대 너비를 유지한다.

독립 대조는 원본 첫 네 행의 내용·17열 그리드·4행 병합·폭·여백을 보존하고 뒤 문단을
추가한 정상 한컴 저장 HWP/PDF다. 생성기·MCP job·PDF path 및 뒤 문단 기준선·hash는
`tests/fixtures/issue7353/body-width/README.md`에 기록했다. 원본31행 전체의 정상화
증거가 아니며, 원본 첫14문단은 다음 미지원 `V2 mixed double-border junction`에서
멈춘다(`output/7353/r19/body-width/full.log`). 이 거부를 숨기거나 표를 간략화한
대조군의 통과로 원본 통과를 선언하지 않는다.

가로 값의 소비 연결:

- `document_input.rs`320–364행: 저장 exclusion host가 선언한 x와 준비된 표 폭으로
  용지 안 바깥여백 포함 envelope를 검증하고 `AnchoredTable.available_width`를
  확정한다. 본문 `FlowCellInput.width`와 앞뒤 LineSeg 폭은 변경하지 않는다.
- `ir.rs`의 두 셀 anchor 경로는 기존 content box 및 오른쪽 여백 검사를 유지한다.
  `content.rs`217행의 lane 검증도 부모 content box 초과를 거부한다. 본문용 배치
  가능 폭을 셀 overflow 허용으로 전파하지 않는다.
- `flow.rs`168–270행: 최초/이월/이어받기 모두 동일 anchor lane을 child fit에
  전달한다. 세로는 기존 `child_budget → TableCursor::fit_with_page_height →
  reserved_height/continuation`을 소비한다. host는 최초 수용 때만, 하단 여백은
  마지막 조각 뒤에서만 소비한다. 실패 시 아직 소비하지 않은 host/child를 이월한다.
- `document.rs`156행은 그 TablePlacement를 `paint.build_node`로 전달한다.
  paint의 별도 너비 축소·다른 원점 선택·clamp는 추가하지 않았다. 일반 body anchor,
  TAC·어울림·셀 내부 분할 규칙·rowspan 컷 알고리즘은 변경하지 않았다.

용지 경계의4회 양수 부동소수점 연산 오차만 구분한다(4×EPSILON×좌표 크기).
정확한 맞춤과1HU 부족 반례를 함께 실행해 실제 부족분을 허용치로 숨기지 않았다.
표 원점·폭 자체는 변경하지 않는다. 용지 밖 개체와 nonzero vertical offset의 별도
본문 anchor 경로는 미지원이며 이번 지원 범위에 포함하지 않았다.

수정 전 보존 Native 바이너리는 동일 정상 HWP를 p1의 `stored body anchor outside body`로
거부했다(`before.log`). 수정 후 동일 HWP는1쪽이며 최종 bbox와 뒤 문단 기준선이 독립
PDF 좌표0.6px 이내다. 신규 정식 계약은 `issue_7353_body_exclusion.rs`의5건이다:
정상 저장본의 최종 표/후속 문단, 용지/바깥여백 경계, 셀 overflow 거부, RowBreak
이어받기, 병합 셀의 통째 이월. 원본4행 병합+RowBreak는 통째 이동해야 하므로
이어받기 계약에서만 첫 열을 비병합 셀로 바꾸고 빈 문단/원래 글자의 소유를 명시했다.
이 변형은 한컴 일치 자료가 아니다. 초기 변형의 빈 셀 글자모양 누락과 기존 rowspan
cell-internal cuts 미지원은 입력/범위 오류로 구분하고 제품 결함 재현으로 세지 않았다.

선택 회귀224건 PASS(본문10, nested18, grid_terminal4, colspan10,
rowspan_roundoff2, text37, 기존 정렬4, filled_field5, document_flow112,
page_number10, fresh_tac1, nested_text11). `body-width/tests.sh` 실행 후 추가한
통째 이월 계약은 본문 harness10건을 다시 실행했다. 각 최종 harness log를 증거로 삼는다.
Native build, Native/WASM lib Clippy `-D warnings`, fmt/diff 검사도 통과했다.
직접 rustc 선택 harness이며 전체 CI/generated suite/workspace 검사로 보고하지 않는다.

최종 소스 Native의 compare·standalone overlay·detail review를 직접 확인했다.
표 외곽·행 높이·뒤 문단 위치가 대응하며 글꼴 굵기/외형 차이는 남는다. 잉크 중심
자동 보조값18.67%를 위치 판정이나 사람 판정 정확도로 사용하지 않는다.
fresh Docker WASM은 `docker compose --env-file .env.docker -p rhwp run --rm wasm`으로
7분10초에 완료했다(`body-width/docker-wasm.log`). WASM SHA256은
`54a485d1073a07db9e659341bf4c5542aa340f99e29459a1e356bf9fd18d1b9a`다.
`node output/7353/r19/body-width/capture.mjs --wasm`과 `crop.mjs --wasm`으로
같은 정상 HWP를 새 브라우저 WASM에서 실행했다. Native/WASM 모두1쪽, SVG 동일,
수치/비수치 차이0이다(`review/backend-comparison.json`). source HEAD50823731a+WIP,
입력/PDF/WASM hash 및 소스31개 hash는 `review/run.json`과 `source.sha256`에 보존했다.

최종 `review/wasm-detail-review.png`와 `review/wasm-overlay-1.png`를 직접 열어
동일 영역의 표 외곽·네 행·뒤 문단을 확인했다. detail은 x50..750/y60..345px의 같은
영역만 잘라 보여주며 이동 보정·확대는 없다. 전체 compare/review도 같은 폴더에 있다.
추가된 테스트2파일의 hash는 `tests.sha256`, 전체 선택 결과는 각 harness의 최종 log에
보존했다. 본문 이어받기의 내용 기대값은 분할 전 V2 출력이 아니라 입력 셀 소유 문자열로
검사한다. 소스/테스트 hash 일치와 fmt/diff를 재확인했다.
이번 절편은 작업지시자 시각 판정 통과다. 승인 범위는 본문보다 넓은 표의 원래 폭,
네 행의 외곽과 후속 문단 배치이며 원본 전체 통과를 뜻하지 않는다.
기본 엔진 전환·commit/push는 없다.

### 전체 진행 현황과 다음 접합 절편

승인된 전체 범위에 대한 거친 진행 추정은 약50% 전후다. 절편·테스트 개수의 비율이
아니며, 이전45% 추정 이후 본문 연결과 정상 저장 입력의 지원 경계가 확장된 점을
반영한다. R2 기반과 R3 Native/WASM 연결은 확보했으나 원본 전체 수용은 미완료다.
R3/R4 복합 분할·일반 어울림·각주/캡션·편집/캐시/커서·세로쓰기 등의 검증과 R5
최신 devel 통합·전체 CI·전환/복귀 판단이 남아 절반가량의 잔여 범위로 보고한다.

다음은 첫14문단의 `V2 mixed double-border junction` 거부다. 독립 PDF와 입력의
실제 접합 스타일을 확인한 후 지원 규칙을 정하며, 기존 guard 삭제만으로 진행하지 않는다.

### 실선–이중선 내부 접합

원본 s0/p13/r28–29/c4의0.5mm 세로 이중선은 양쪽으로 이어지는0.12mm 가로
실선과 만난다. 첫14문단 정상 저장 PDF3쪽 `14.비용감축제`의 path에서 이중선
두 pen(0.36pt, 원본 중심284.127/285.207pt)이 실선
가장자리에서 끝나는 것을 확인했다. 숫자는 문서 위치일 뿐 구현 조건으로 쓰지 않는다.
독립 정상 저장 가로/세로 대조군은 `tests/fixtures/issue7353/mixed-borders/README.md`에
생성 방식·job·원문/PDF hash·지원 경계를 기록했다.

최종 값의 경로는 `CellBorders::append`가 확정된 `TablePlacement`의 xs/ys와 공유
Span을 구성 → `borders::double::Junction::inset`이 두 방향의 동일 색/동일 실선을
확인 → 공통 실선 pen 폭의 절반을 반환 → `double::append`가 최종 LineNode
끝점과 ink bbox를 생성 → 동일 RenderTree를 Native SVG와 WASM이 소비하는 것이다.
실선은 기존 한 개의 연속 Span 그대로다. 두 pen의 간격이나 표/셀 사각형은 바꾸지 않는다.
측정·예약·컷·이어받기 생산 경로 변경은 비해당이며, geometry 불변 대조로 확인한다.

바깥 모서리까지 일괄 수용하지 않았다. 최초 all-vertical/all-horizontal double 그리드의
정상 저장 PDF는 한쪽만 만나는 접합이 내부와 다름을 보여 준다. 이 진단 입력/PDF도
`output/7353/r19/mixed-borders/{vertical,horizontal}-*`에 보존했다. 정식 inner
대조군은 모든 외곽을 실선으로 두고 중앙 경계만 이중선으로 선언해 실제 원본에 해당하는
T/십자 접점을 확인한다. 색 충돌·한쪽 접점·동일하지 않은 incident 실선은 기존 거부를 유지한다.

정식 회귀 `normal_saved_mixed_inner_junctions_stop_at_solid_ink_and_preserve_content`는
최종 두 pen의 시작/끝,0.48px 폭/1.44px 중심 간격, 실선의 연속성, 독립 PDF 원점,
30000×8000HU 그리드, 각 셀 글자와 AFTER CELL의 단일 소유/후속 배치를 검사한다.
추가 접합 분기만 제외한 수정 전 라이브러리에 같은 테스트를 링크해
`V2 mixed double-border junction`으로 FAIL한 뒤 복원 코드로 PASS했다.
`mixed-borders/test-before.log`는 빌드 실패가 아니라 해당 거부의 재현이다.
반복 제목·분할과 색 충돌의 계약도 추가했다. 최초 테스트에서 의도적으로 바꾼
border_fill_id까지 같다고 비교한 실수는 장식 참조만 비교에서 빼고 bbox/소유/텍스트를
유지하도록 정정했다. 선택 harness의 roxmltree 링크 누락도 제품 결함이 아니라 실행
구성 오류로 분리했다.

#### 새로 출력 가능한 원본 첫14문단의 분할 차이

거부 해소 후 V2는3쪽을 출력하지만 동일 PDF와2→3쪽 컷이 다르다.
V22쪽은 r0..7,3쪽은 r8부터이며 PDF2쪽에는 r8/9까지 있다. `fit_row_groups`의
`group_end(8)`은 r8/c0의7행 병합을 포함한 그룹 끝15까지 묶는다. 원래 남은 공간에
r8/9를 배치하고 rowspan을 이어받는 처리는 현재 이 RowBreak 경로에서 하지 않는다.
페이지3 후속 내용은 약63.87px 늦어지며 직접 `full/native-review-3.png`에서 관측했다.

원본 테두리의 Double만 Solid로 치환한 진단 사본은 수정 전/후 출력이 완전히 동일하다.
이 진단 출력과 원래 Double 입력의 Table/TableCell/TextLine/TextRun도 bbox와 내용이
동일하다(`geometry-check.json`). 따라서 이번 paint 변경으로 생긴 분할 회귀가 아니다.
사본은 진단용이며 한컴 일치 증거로 쓰지 않는다. 원본 첫14문단 통과도 선언하지 않는다.
다음 실제 작업 대상은 이 RowBreak 병합 그룹의 경계/소유/이어받기 규칙이다. 이번 접합
절편에 분할 규칙 변경을 섞거나3쪽이라는 총수만으로 정상화하지 않는다.

#### 최종 검증과 판정 자료

`bash output/7353/r19/mixed-borders/tests.sh` 최종227건 PASS
(borders25, split_borders11, headers14, colspan10, rowspan10, nested18,
geometry9, vertical_alignment8, body_exclusion10, document_flow112).
Native build 및 Native/WASM lib Clippy `-D warnings` 통과, fmt/diff 통과다.
직접 rustc 선택 harness이며 전체 CI/generated suite/workspace 검사로 보고하지 않는다.
production은 `borders/double.rs`의 내부 실선 접점 분기10줄 추가뿐이다.

`docker compose --env-file .env.docker -p rhwp run --rm wasm`은7분21초에 완료했다.
새 WASM SHA256은 `2b456e033f504ba20071de4a13da1cb43ae7e6d3f7dde0e92208b8a6870758b7`다.
`MIXED_CASE=<vertical-inner|horizontal-inner|full> node
output/7353/r19/mixed-borders/capture.mjs --wasm`으로 실제 브라우저 DocumentV2를
실행했다. 두 대조군은 각각1쪽, 첫14문단은3쪽이며 모든 쪽의 Native/WASM SVG 동일,
수치/비수치 차이0이다. 각 `run.json`에 HEAD50823731a+WIP, 입력/PDF/WASM hash를,
`source.sha256`에는 변경한 borders 하위 모듈을 포함한 소스32개 hash를 남겼다.
`tests-fixtures.sha256`도 현재 파일과 대조했다.

Native/fresh WASM의 직접 확인 자료:

- [세로 이중선 384dpi 접점](../../output/7353/r19/mixed-borders/vertical-inner/wasm-junction-review.png)
- [가로 이중선 384dpi 접점](../../output/7353/r19/mixed-borders/horizontal-inner/wasm-junction-review.png)
- [세로형 전체 review](../../output/7353/r19/mixed-borders/vertical-inner/wasm-review-1.png),
  [standalone overlay](../../output/7353/r19/mixed-borders/vertical-inner/wasm-overlay-1.png)
- [가로형 전체 review](../../output/7353/r19/mixed-borders/horizontal-inner/wasm-review-1.png),
  [standalone overlay](../../output/7353/r19/mixed-borders/horizontal-inner/wasm-overlay-1.png)
- [미해결 원본3쪽 비교](../../output/7353/r19/mixed-borders/full/wasm-review-3.png),
  [standalone overlay](../../output/7353/r19/mixed-borders/full/wasm-overlay-3.png)

384dpi 접점은 같은 물리 영역의 start T/cross/end T를 같은 크기로 잘라 놓은 자료다.
개별 영상 이동·정렬 보정 없이 직접 판독했다. 전체 페이지 잉크 자동 보조값은 세로형93.20%,
가로형93.00%이며 폰트 외형과 미세 PDF 격자 차이는 남는다. 원본3쪽22.19%는 분할
차이의 진단값이지 통과 근거가 아니다. 접합 규칙 대조군은 작업지시자 시각 판정 통과이며
원본 분할은 다음 절편의 미해결 대상이다. 기본 엔진 전환·commit/push는 수행하지 않았다.

### RowBreak 병합 셀의 행 경계 이어받기 착수

동일 정상 저장 첫14문단과 한컴 PDF2/3쪽을 재사용한다. PDF2쪽은 r8/9를 포함하고,
병합 제목은 앞 조각에 한 번만 표시되며 PDF3쪽의 이어받은 좌측 영역은 빈 셀로 남는다.
현재 `group_end`의 병합 연결 그룹 원자성은 이 관측과 다르다. 행 경계 분할과 셀 내부
내용 분할을 구별하여, 셀 내용 전체가 앞 조각에 수용되는 행 경계 컷부터 구현한다.
내용이 앞 조각에 안 들어가는 컷은 수용하지 않으며 Never/반복 제목의 원자성은 유지한다.

추적 값은 물리 행 범위다. `TableContentPlan`의 행 높이/내용 점유 → row-group fit의
컷 후보와 요구 높이 → `TableFragmentPlan`의 예약 높이/실제 셀 조각 → `TextPaint`의
내용 소유 및 `CellBorders`의 물리 경계를 연결한다. 원본 셀 주소와 조각 행 범위를
분리하여 이어받기 테두리가 원래 시작 행을 다시 생성하지 않게 한다. CellBreak rowspan
내부 컷은 기존 명시적 미지원으로 남긴다. 글꼴과 전체 원본 수용은 이번 판정 범위가 아니다.

#### RowBreak 구현 및 독립 경계 확인

수정 전 정식 `tests/cases/issue_7353_rowbreak_span_continuation.rs`는2쪽의
r9/c2가 없다는 원인으로 FAIL했다(`output/7353/r19/rowbreak-span/test-before.log`).
수정 후에는2쪽 r0..9,3쪽 r10..30이며 r8/c0,c1의 원본 주소는 유지하되
`visible_rows`로 각 조각의 실제 행 범위를 별도로 전달한다. 두 병합 제목은 첫 조각에만
소유되고 다음 조각은 빈 물리 셀로 이어진다. 제목이 잘리는 대신 셀 내용 전체가 첫 조각에
들어가는 컷만 수용한다. 셀 내부 rowspan 내용 분할을 구현한 것으로 보고하지 않는다.

첫 후보는 행 컷을 맞췄지만2쪽 마지막 행 높이가 짧았다. 이를 paint에서 늘리지 않고
쪽 여백 대조본으로 분할 단계의 물리 예약을 검증했다. 원본 첫14문단 정상 저장 HWP에서
① 쪽 아래 여백만600HU 늘린 HWPX,② 주 표 아래 바깥여백만141HU→0으로 바꾼 HWPX를
각각 한컴으로 정상 저장한 뒤 해당 HWP의 PDF를 획득했다. 행/줄 메타데이터는 수동으로
바꾸지 않았다. 생성 코드·입력·반환본·PDF·job ID는
[`tests/fixtures/issue7353/rowbreak-span/README.md`](../../tests/fixtures/issue7353/rowbreak-span/README.md)에 연결했다.
재현 생성기의 두 `section0.xml`은 실제 제출 입력과 동일한 것도 확인했다.

독립 PDF2쪽의 마지막 테두리 끝은 원본1042.6987px,쪽 여백 증가본1034.7080px,
표 여백 제거본1044.4573px다. 같은 r9→10 컷에서 물리 표 끝이 가용 영역/바깥여백에
따라 움직인다. 이에 따라 rowspan을 가로지르는 컷은 마지막 물리 행에 남은 빈 밴드를
예약하고 후속 행의 원래 최소 높이/내용은 소비하지 않는다. 최종 조각은 남은 실제 높이만
사용한다. source HU 델타8px/1.88px를 적용하며 PDF 출력의 양자화 차이를 상수로
엔진에 넣지 않았다. 현재 원본2쪽 외곽 끝은1045.0267px로 PDF와 약2.33px 차이가 남는다.

실제 호출/소비 경로:

| 단계 | 코드와 계약 |
| --- | --- |
| 내용 점유 생산 | `content.rs:388`의 패딩 포함 `content_height`; 기존 전체 셀 정렬 결과와 분리 |
| 컷·요구 높이 | `fragment.rs:209`→`fragment/row_groups.rs:15`의 `row_cut_required`; 앞 조각이 소유할 전체 내용까지 요구 |
| 예약·이월 | 같은 파일 `fit_row_groups`가 컷을 먼저 확정하고 남는 물리 밴드를 마지막 행에 예약; `next.row=end`로 내용 컷과 구별 |
| 바깥여백 | `flow.rs:202`의 RowBreak 앵커 자식 예산에서 아래 바깥여백 선예약; CellBreak는 기존 별도 tail 소비 계약 유지 |
| 실제 배치 | `TextPaint::build_node` (`text.rs:203` 이후)가 원본 주소로 payload를 찾고 전달된 줄/자식 원점을 사용; 내용 재측정/축소 없음 |
| 외곽·배경 | `borders.rs:85` 이후가 `visible_rows`와 실제 bbox 사용; gradient/diagonal 조각은 기존 명시적 미지원 유지 |

검증 중 바깥여백을 CellBreak까지 적용했을 때 기존
`following_anchor_cuts_preserve_negative_gap_host_and_child_tail`의20px 예산이 실패했다.
이 검사를 완화하지 않고, 독립 출력으로 확인한 RowBreak 경로로 적용 범위를 수정하여
기존20/30/40/1000px CellBreak 검사 모두 복구했다. CellBreak 여백의 한컴 일치는 이번
근거로 주장하지 않는다.

작은 경계 계약은36px 행에54px 내용이 있는 경우를53px 예산에서 거부하고54px에서
온전히 수용하는 것, 같은 컷에서 여러 셀이 끝나는 경우, 두 병합 셀의3개 조각,
첫 조각 Center/Bottom 정렬, 이후 제목 비반복, 마지막 행 후 종료, Never/반복 제목,
중첩 자식 뒤의 문단 보존을 확인했다. #6923의1HU 부족 검사는 세 번째 행을 수용하지
않는 계약을 유지하고, 새 물리 닫힘 밴드와 원래 행 높이 합을 따로 검사한다. 이전 그룹
전체 원자성 기대값은 RowBreak와 맞지 않아 독립 PDF와 위 규칙에 따라 변경했고, 전체
원자성 대조군은 명시적 Never로 유지했다. 단순 페이지 수/golden 완화가 아니다.

#### RowBreak 최종 검증 및 시각 판정 준비

선택 harness `bash output/7353/r19/rowbreak-span/tests.sh`:237건 PASS
(normal/여백2, borders25, split_borders11, headers14, colspan10, rowspan13,
nested18, geometry9, vertical_alignment8, body_exclusion10, document_flow112,
rowspan_roundoff2, oversize_row3). Native build와 Native/WASM lib Clippy
`-D warnings`, fmt/diff 확인. 이는 직접 rustc 선택 검사이며 전체 CI/generated suite/
workspace 검증을 대신하지 않는다. 신규 테스트 원본만 `tests/cases/`에 두었다.

Native에서 원본2/3쪽 review와 두 여백 대조본2쪽을 직접 판독했다. 원본3쪽의 후속 표
외곽과 행 위치가 개선됐으며, 원본2쪽 하단 약2.33px 및 대체 글꼴 외형 차이는 남는다.
`docker compose --env-file .env.docker -p rhwp run --rm wasm`은7분28초에 완료했다.
fresh WASM SHA256:
`d0ff783b99898082ebc52daa1eeb371453d89741e36edf8c37316cc9f87a9800`.
`SPAN_CASE=<prefix14|margin|no-bottom> node output/7353/r19/rowbreak-span/capture.mjs --wasm`
으로 실제 브라우저 DocumentV2를 실행했다. 입력3종×각3쪽 모두 Native/WASM 수치·비수치
차이0, SVG 동일이다. 각 `run.json`에 HEAD50823731af+WIP, 입력/PDF/WASM SHA256과
소스32개 manifest를 기록했다. `source.sha256`, `tests-fixtures.sha256` 최종 대조 통과.

판정 자료(각 review는 한컴/새 WASM/overlay를 같은 쪽·크기로 표시):

- [원본 첫14문단2쪽 review](../../output/7353/r19/rowbreak-span/review/wasm-review-2.png),
  [compare](../../output/7353/r19/rowbreak-span/review/wasm-compare-2.png),
  [standalone overlay](../../output/7353/r19/rowbreak-span/review/wasm-overlay-2.png)
- [원본 첫14문단3쪽 review](../../output/7353/r19/rowbreak-span/review/wasm-review-3.png),
  [compare](../../output/7353/r19/rowbreak-span/review/wasm-compare-3.png),
  [standalone overlay](../../output/7353/r19/rowbreak-span/review/wasm-overlay-3.png)
- [쪽 여백 증가본2쪽 review](../../output/7353/r19/rowbreak-span/margin-review/wasm-review-2.png)
- [표 바깥여백 제거본2쪽 review](../../output/7353/r19/rowbreak-span/no-bottom-review/wasm-review-2.png)

fresh WASM2쪽 review와3쪽 standalone overlay도 직접 열어 행 경계·병합 제목 소유·
후속 내용·외곽을 확인했다. 원본2/3쪽 잉크 자동 보조값20.06%/50.46%는 사람이 판정한
정확도가 아니다. 약2.33px 테두리 잔차와 글꼴 외형 차이를 공개한 자료에 대해
**작업지시자 시각 판정 통과**와 다음 절편 진행 승인을 받았다. 기본 엔진 전환,
전체 원본 통과 선언,commit/push는 하지 않았다.

#### 다음 원본 진입점: p15 단 기준 앵커와 셀 문단 내부 이어받기

원본 `86712_regulatory_analysis.hwp`의 p15(문단 index,쪽 번호 아님)는 가로 Column,
세로 Para/Top,offset0,폭0 host를 가진 RowBreak2×2표다. 첫17문단을 보존한 HWPX를
한컴 정상 저장 후 반환 HWP의 PDF10쪽을 확보했다. 생성과 job 근거는
`tests/fixtures/issue7353/cell-line-continuation/README.md`에 연결한다.

수정 전 정식 `issue_7353_cell_line_continuation`은 p15의
`stored excluded cell anchor` 미지원으로 FAIL했다. 메모리상의 Para 치환 진단에서
`unqualified stored cell frame reset`도 확인했다. PDF4→5쪽은 오른쪽 셀의 같은 문단
p7/line0→line1 이어받기이며, 원본 저장 y가0으로 돌아간다. 문단 분해나 속성 치환으로
입력을 우회하지 않고 본문 Column 원점과 셀 내부 줄 컷을 명시적으로 구별한다.

추가 확인된 경로는 반복 제목이다. `fragment::fit_with_header`가 본문 fit에 fresh-page
capacity를 전달하지 않아 초과 높이 RowBreak행이 원자적으로 거부됐다. 제목 예약은
원자적으로 유지하고 제목을 뺀 현재 조각 예산과 호출자가 준 fresh-page capacity를
분리해 기존 셀 줄 fit을 호출한다. 제목 높이를 초과 행 판정의 fresh capacity에서도
빼는 첫 후보는 기존 `multirow_prefix`에서 간격만 남는 제목 전용 마지막 쪽을 만들었다.
이 잘못된 가정을 제거하여 기존 53px 거부/54px 수용 계약을 유지한다. 합성 새 경계는
본문40px/전체 fresh45px에서는 원자성 유지, 본문60px/fresh50px에서는 제목10px을
실제 예산에서 예약한 뒤20px줄 단위로 분할하는 것을 검사한다.
추적: source(pi,li) 리셋 → 같은 UTF-16 분할의 연속 문단 메트릭 → 공통 TextComposer
노드 → IR의 StoredFrameTail/Start → FlowCursor 컷·패딩 예약 → 원래 line owner의 paint.
본문의 일반 LineSeg 수용 조건 및 셀 안 Column 앵커 수용은 변경하지 않는다.

#### 셀 문단 내부 이어받기 검증

현재 절편의 실제 소비 경로는 `stored_text.rs:162`의 `(pi,li)` 컷 → 같은 파일201의
연속 story 변환 → `ir.rs:310`의 동일 composer 입력 →457의 원래 line owner 경계에
`StoredFrameTail/Start` 삽입 → `fragment.rs:120,158`의 제목+본문 트랜잭션 →
`flow.rs:109,135,154`의 끝 패딩 선예약/시작 패딩/간격 처리 → `text.rs:232`의 같은
line payload 배치다. 글자와 UTF-16 분할은 바꾸지 않는다. Never/intact는 원래 줄간격을
유지하고, 분할 시에만 해당 저장 프레임 끝의 다음 줄 간격을 소비하지 않는다.

`bash output/7353/r19/cell-anchor-next/tests.sh`:17개 선택 harness289건 PASS.
신규5건은 정상 저장본의 양쪽 셀 모든 줄(빈줄 포함) 순서·횟수·셀 내부 수용, 반복 제목,
4→5쪽 이어받기와10쪽 후속 문단,1HU 부족 시 거부와 재시도,첫/끝 패딩 및 Never pitch,
제목만의 진행 금지,비정상 nonzero 리셋 거부,본문 Column을 셀 기준으로 오인하지 않는
반례를 검사한다. 기존 제목14건은 기대값 변경 없이 통과했다. 수정 전 첫 거부와
중간 후보에서 검출한 실패도 같은 output 폴더에 보존했다.

Native build,Native/WASM lib Clippy `-D warnings`,fmt/diff PASS. 전체 CI나
generated suite/workspace 검증을 대신하지 않는 선택 검사다. 원본 전체는 현재 p20의
`stored indentation precision`에서 명시적 미지원이며, 첫17문단의 수용을 전체 원본
완료로 보고하지 않는다.

최종 Native4/5쪽 review,6/7/8/9쪽 standalone overlay,10쪽 후속 내용까지 직접 판독했다.
`reference-trace.xml`의 독립 PDF 선 끝을96dpi로 환산하면4/5/9쪽은
1036.9453/1038.5440/635.7840px다. Native끝은1037.9200/1039.5200/636.3200px로
약0.98/0.98/0.54px 차이가 남는다. 엔진에 그 잔차를 보정 상수로 넣지 않았다.
대체 글꼴 외형과 이 잔차는 판정 자료에서 공개한다. 총10쪽이라는 사실만으로
시각 통과를 선언하지 않는다.

#### 최종 WASM 판정 준비 — 셀 문단 내부 이어받기

`docker compose --env-file .env.docker -p rhwp run --rm wasm`:7분27초 PASS.
첫 후보 빌드는 경계 수정으로 중지했으며 최종 코드로 다시 빌드했다.
fresh WASM SHA256:`4aa9ace4b3ffe261fc2377beae3c673e77a923c18f19d0dc9a59ca7b61b05acb`.
`node output/7353/r19/cell-anchor-next/capture.mjs --wasm`으로 실제 브라우저 DocumentV2
실행:10쪽 전체 Native/WASM 수치 차이0,기타 차이0,SVG 동일.
`review/run.json`에 HEAD50823731af+WIP의 source manifest,입력/PDF/WASM SHA를 기록했고
마지막 source manifest 대조도 통과했다. 최종 WASM4/5쪽 review를 직접 열어 확인했다.

판정 대상은 한컴 정상 저장 첫17문단의4→5쪽 문단 내부 이어받기,제목 반복,
9쪽의 표 종료다. 대체 글꼴과 약1px 이내의 표 하단 잔차는 남아 있다.

- [4쪽 review](../../output/7353/r19/cell-anchor-next/review/wasm-review-4.png)
- [5쪽 review](../../output/7353/r19/cell-anchor-next/review/wasm-review-5.png)
- [9쪽 review](../../output/7353/r19/cell-anchor-next/review/wasm-review-9.png)
- [4쪽 compare](../../output/7353/r19/cell-anchor-next/review/wasm-compare-4.png) /
  [standalone overlay](../../output/7353/r19/cell-anchor-next/review/wasm-overlay-4.png)
- [5쪽 compare](../../output/7353/r19/cell-anchor-next/review/wasm-compare-5.png) /
  [standalone overlay](../../output/7353/r19/cell-anchor-next/review/wasm-overlay-5.png)
- [검토용 HWP](../../tests/fixtures/issue7353/cell-line-continuation/prefix17-saved.hwp) /
  [독립 기준 PDF](../../tests/fixtures/issue7353/cell-line-continuation/prefix17-2020.pdf)

4/5쪽 잉크 자동 보조값16.12%/14.97%는 사람이 판정한 정확도가 아니다.
**에이전트 시각 검토: 이번 절편의 기능 범위 충족**. 작업지시자의 “확인은 당신도 할 수
있습니다” 지시에 따라 직접 판독 결과로 이 절편의 확인을 마친다. 작업지시자가 직접
시각 통과를 선언한 것으로 기록하지 않으며, PR 최종 승인과도 구분한다.

판독 근거: WASM4쪽 말미의 문단이5쪽 반복 제목 아래로 이어지고,6~8쪽의 저장 줄 구성과
왼쪽 빈줄 배치,9쪽 마지막 문장/표 종료,10쪽 후속 문단이 보존된다. Native4~10쪽 직접
판독과 최종 WASM4/5쪽 review·9쪽 standalone overlay를 확인했으며,10쪽 전체의
Native/WASM 출력 동일성도 대조했다. 이번 확인 시 source manifest는 변경되지 않았다.
누락·중복은 정식 줄 보존 검사와 함께 확인했다. 앞서 공개한 글꼴 차이와0.54~0.98px
하단 잔차까지 완전 일치한다고 주장하지 않는다. 동일 자료의 확인을 다시 요청하거나
빌드·회귀를 중복 실행하지 않는다. 기본 엔진은 전환하지 않았고 commit/push도 하지 않았다.

#### 메인터너 판정 및 다음 절편 — URC 글자 단위 내어쓰기

작업지시자가 위 셀 문단 내부 이어받기에 대해 “메인테이너의 시각 판정도 통과”를
명시하고 다음 절편 진행을 승인했다. 앞 절편의 증적을 재사용한다.

원본 `86712_regulatory_analysis.hwp`의 본문 p20은 ParaShape indent=-3001이다.
저장 LineSeg는 cs=0/sw=48188이고 첫 줄을 제외한6개 줄에 bit20이 있다.
**초기 가설 정정:** 홀수를 half-HU로 해석한 후보는 합성 계약을 통과했지만 실제
한컴PDF10쪽과 약80px 시작점 차이를 만들었다. 자동 검사로 시각 통과를 선언하지 않고
직접 비교에서 검출했다. 후보의10쪽 좌표는 `half-unit-indent/diagnostic/native.json`,
수정 전 수용 거부는 `saved-before.log`에 보존했다. 이 후보의 테스트 통과는 올바른
조판 근거가 아니다. output/fixture/test의 `half-unit` 이름은 초기 조사 이력이다.

[한컴 URC 설명](https://forum.developer.hancom.com/t/topic/2209)의 bit0 단위 구분과
signed `data >> 1`에 따르면 -3001은 **-15.01ch**다. [첫 줄 모양 도움말](https://help.hancom.com/hoffice/multi/ko_kr/hwp/format/paragraph/paragraph%28indenting%29.htm)은
ch의 기준을 바탕글 스타일의 영문 크기로 명시한다. 이 문서는 바탕글10pt로,
영문 반각5pt 기준의 내어쓰기는75.05pt다. PDF의 첫 줄/다음 줄 시작 차이는 약74.959pt로
프린터 좌표 양자화 잔차가 남는다. 본문 run의14/15pt를 기준으로 삼지 않는다.

추적: ParaShape URC→V2 source_units의 문서 기준 단위 해소→stored_text::localize의물리구간
→TextComposer→layout_composed_paragraph_in_frame의effective_col_x/w
→실제TextLine/TextRun→같은노드를셀fit과paint가소비. 물리 구간 경로에서는 기존
margin/indent 중복 적용을 하지 않는다. 다른 Legacy 호출은None, 기존inline flow의
정수 줄 상자는 원래 값을 물리 구간으로 변환해 같은 경로를 사용한다.
기본 Legacy style resolver는 그대로 두고 V2 Document/선택 TablePreview 진입점에 같은
단위 해소를 적용한다. PreparedTextTable의 직접 호출자는 기존대로 해소된px를 제공한다.
이번 범위는 indent의URC CHAR이며, 다른 문단 여백·위아래 간격의CHAR 해소는 미구현이다.
장평/상대크기100%,자간0인 바탕글 영문 기준을 검증했으며 다른 기준은 명시적으로 거부한다.

기준 출력은 원본 첫21문단을 무스타일변경으로 추출한 뒤 한컴 정상 저장한HWP와 그PDF다.
바탕글만12pt로 바꾸고 본문run크기는 유지한 별도 대조군도 정상 저장/PDF출력했다.
한컴이 좁아진 가용 폭으로 갱신한 대조군의LineSeg를 그대로 검사하며 원본 줄바꿈을
강요하지 않는다. 두 파일 모두10쪽이지만 판정 근거는10쪽 들여쓰기와각저장줄소속이다.
첫21문단을 원본 전체 지원으로 보고하지 않는다.

#### URC 내어쓰기 검증과 적용 경계

`half-unit-indent/tests.sh`의7개 harness192건과 정상 저장본 신규3건,
공통 물리 줄 상자를 소비하는 `issue_6812_square_picture_tac_table`20건이 통과했다
(총215건). `test-summary-final.log`, `test-saved-final.log`, `test-6812.log`에 결과를
보존했다. Native build, Native/WASM lib Clippy `-D warnings`, fmt를 통과했다.
전체 CI/workspace gate를 수행했다는 의미는 아니다.

기존 precision 거부는 `test-before.log`/`saved-before.log`에 보존했으며, 최종 검사는
원본/바탕글12pt 정상 저장본의 실제 줄 상자·글자 원점·저장 줄 소속을 확인한다.
셀 경로의 양수 URC 합성 대조군은 `AB` 두 글자 carrier로 구분한다. 한컴 생성본으로
주장하지 않는다. 원본의 긴 첫 줄을 양수 들여쓰기한 후 좁은 폭에 그대로 강요하지 않는다.
단위가 보존되지 않은 plain HWPX의 홀수 여백은 URC로 추정하지 않고 명시적으로 거부한다.
Normal 기준 누락과 scaled Latin 기준도 미지원 경계를 검사했다.

앞서 승인한 첫17문단을 최종 Native로 다시 출력해 기존 증적과 대조했다.
10쪽 모두 내용/구조 차이0, 좌표 최대 차이1.1369e-13px다. 부동소수점 연산 순서로
4~8쪽 SVG 문자열 차이는 있지만 의미 있는 배치 차이는 없다
(`previous-control/comparison.json`). 원본 첫21문단의10쪽 다음 줄 x는
PDF175.42493px / V2175.65333px, Normal12 대조군은 PDF195.574px / V2195.66667px다.
잔차 약0.23px /0.09px와 대체 글꼴 외형을 공개하며, 임의 위치 보정은 추가하지 않았다.

#### URC 최종 fresh WASM 및 직접 시각 확인

`docker compose --env-file .env.docker -p rhwp run --rm wasm`:7분19초 PASS.
최종 WASM SHA256:`aaeaee52467204ca63236fc41decc845670dfce54ee7ea6bc5f0d60e61965a02`.
`node output/7353/r19/half-unit-indent/capture.mjs --wasm`와 같은 명령의
`--normal12 --wasm`으로 실제 브라우저 DocumentV2를 실행했다. 두 입력 각각10쪽 모두
Native/WASM 수치 차이0, 기타 차이0, SVG 동일이다. `review/run.json`과
`control-review/run.json`에 HEAD50823731af6050c60ec3cfcc898abf36daa6b35a+WIP의
source manifest/입력/PDF/WASM SHA를 보존했다. 검증 후 source manifest 대조 PASS.

최종 두 입력10쪽의 WASM review와 standalone overlay를 직접 열어 확인했다.
원본의 내어쓴6줄과 대조군에서 한컴이 다시 나눈6줄은 각 입력의 저장 줄 소속과
시작점에 대응한다. 앞 제목/빈줄 위치와 마지막 줄도 보존된다.
에이전트 판정은 **이번 내어쓰기/줄 구성 범위 충족**이다. 이번 새 증적에 대한
메인터너 판정을 대신 기록하지 않으며 원본 전체/R5 완료로 확대하지 않는다.
자동 잉크 보조값11.65%/11.35%는 사람 판정 정확도가 아니고 글꼴 차이의 영향을 받는다.

- [원본10쪽 review](../../output/7353/r19/half-unit-indent/review/wasm-review-10.png) /
  [compare](../../output/7353/r19/half-unit-indent/review/wasm-compare-10.png) /
  [standalone overlay](../../output/7353/r19/half-unit-indent/review/wasm-overlay-10.png)
- [바탕글12pt 대조군10쪽 review](../../output/7353/r19/half-unit-indent/control-review/wasm-review-10.png) /
  [compare](../../output/7353/r19/half-unit-indent/control-review/wasm-compare-10.png) /
  [standalone overlay](../../output/7353/r19/half-unit-indent/control-review/wasm-overlay-10.png)
- [검토용 HWP](../../tests/fixtures/issue7353/half-unit-indent/prefix21-saved.hwp) /
  [독립 PDF](../../tests/fixtures/issue7353/half-unit-indent/prefix21-2020.pdf)

최종 Native로 원본 전체를 재실행한 다음 미지원은 p30의 `V2 cell border style`이다
(`full-original-final.log`). 다음 절편은 해당 테두리 속성의 IR/측정/paint 적용 경로를
조사한다. 기본 Legacy 경로 전환, 전체 CI, commit/push는 수행하지 않았다.

#### 메인터너 판정 및 다음 절편 — 셀 Dash 테두리

작업지시자는 URC 내어쓰기 증적의 시각 판정 통과와 다음 절편을 승인했다.
다음 거부는 원본p30/c0,4행3열 표의 borderFill15/16에 저장된 Dash다.
파서→IR에는 정상 보존되지만 V2 `borders::resolve_edges`가 Solid/Double만 수용했다.
원본 첫31문단을 속성 변경 없이 추출→한컴 정상 저장→같은HWP의PDF를 만들었고,
정상 저장본도 수정 전 p30에서 `V2 cell border style`로 거부됐다(`before.log`).
정식 `issue_7353_cell_dash`의 첫 검사도 같은 원인으로 FAIL했다(`test-before.log`).

독립 PDF10쪽의 점선은 약2.398pt 획/1.439pt 공백이다. 일반 도형의6px/3px 패턴을
그대로 적용하지 않는다. 원본 표를 이용해 굵기4종 및16종 대조군을 정상 저장하고
수평/수직 stroke trace를 확보했다. 16개 표준 굵기에 대응하는600dpi pen 사전을 사용하며
문서·페이지·특정 셀 크기 조건은 없다. 출처·변경점·job은
[fixture 기록](../../tests/fixtures/issue7353/cell-dash/README.md)에 연결했다.
원본/대조군의 저장 LineSeg는 수동 수정하지 않았다.

추적: BorderFill→ResolvedStyleSet→`text_ir.rs:241 CellBorders::prepare`→
`borders.rs:280 resolve_edges`의 검증/보존→`text.rs:130 fit`의 기존 공통 조각 geometry→
`text.rs:181 build_node`→269의 `borders.append`→위상 경계별union→`borders/dash.rs`의
물리 stroke 목록→공통 RenderNode Line→Native/WASM SVG다. Dash를 짧은 실선 노드들로
표현해 backend별 독자 대시 추정을 없앴다. 셀 측정 높이·예약·컷·원점은 변경하지 않았다.
마지막 pen을 해당 경계 끝에서 끝내는 것은 paint 패턴의 부분 획이며 셀/내용 좌표 clamp가 아니다.
Zone Dash 수용과 혼합 Double 접점은 미검증이므로 기존 거부 경계를 유지한다.
공유 Dash의 색/굵기/선종류 충돌도 임의 우선순위로 덮지 않는다.

정식 선택 검사5개 harness173건 PASS: borders28,cell_dash2,document_flow112,rowspan13,
nested18. 새 검사는 실제 정상 저장본의 두 점선 경계·연속 주기, 두 DPI의16종 최종
endpoint/부분 획/끝 공백, 동일 공유 경계의 중복 제거, 충돌 시 세션 미전진,
반복 제목·부모/자식 분할·뒤 문단의 기존 bbox/텍스트 보존을 확인한다.
`test-summary-final.log`에 실행 결과를 남겼다. Native build,Native/WASM lib Clippy,
fmt/diff도 PASS. 전체 CI/workspace 검증을 대신하지 않는다.

16종 catalog의 문서 전체는 한컴 정상 저장 후 host paragraph inset이 있어 V2가 거부한다
(`capture-catalog.log`). 이를 없애려고 문서 속성/수용 조건을 바꾸지 않았다.
catalog의 독립PDF에서 pen을 관측하고, 선택 TablePreview의16개 실제 표를 각2개 DPI로
검사했다. catalog 전체 V2 조판 통과 또는 전체 시각 일치로 보고하지 않는다.

Native10쪽 review/standalone overlay 직접 확인: 점선 경계·저장 줄바꿈·표 외곽이 대응한다.
원본 표 y/height는 V2851.6133/173.12px, PDF는 약850.7493/172.9307px다.
약0.86px 시작점·0.19px 높이 잔차와 대체 글꼴 외형 차이가 남는다.
자동 잉크 보조값11.37%는 사람 판정 정확도가 아니다. 임의 보정은 추가하지 않았다.
원본 전체의 다음 미지원은 p34 `anchored host paragraph insets`다(`full-original.log`).

#### 셀 Dash 최종 fresh WASM 및 직접 시각 확인

Docker 표준 빌드 `docker compose --env-file .env.docker -p rhwp run --rm wasm`는
7분16초 PASS. WASM SHA256은
`19e80fa32d9838f64b4c0490c664adee572b30eae9d28b677bc41aef36b89a8d`다.
`node output/7353/r19/cell-dash/capture.mjs --wasm`으로 정상 저장본10쪽을 실제
브라우저 DocumentV2에서 출력했다. Native/WASM 수치·기타 차이0,10쪽 SVG 모두 동일이다
(`review/backend-comparison.json`). `review/run.json`은 HEAD50823731af+WIP,
source manifest·입력·기준PDF·WASM 해시를 연결한다. 검증 후 source manifest 대조 PASS.

최종 WASM10쪽 review와 standalone overlay를 직접 열었다. 하단 표의 두 점선 경계,
열 사이 연속성, 셀 줄바꿈과 외곽 배치를 확인했다. 이번 Dash 처리 범위의 에이전트
판정은 충족이다. 대체 글꼴과 위에 기록한 약0.86px/0.19px 잔차는 남아 있으며,
자동 잉크 보조값11.37%를 사람 판정 정확도로 해석하지 않는다. 이번 증적에 대한
메인터너 시각 판정이나 원본 전체/R5 완료를 대신 선언하지 않는다.

기존 승인한 첫21문단 정상 저장본도 최종 Native로 재실행했다.
`cell-dash/previous-control/native.json`과 `half-unit-indent/review/actual/native.json`을
`cmp`로 비교해10쪽 전체 바이트 동일을 확인했다. 기존 대조군 무변경이며,
다음 대상은 p34의 앵커 문단 안쪽 여백이다. 전체 CI·기본 엔진 전환·commit/push는 하지 않았다.

- [최종10쪽 review](../../output/7353/r19/cell-dash/review/wasm-review-10.png) /
  [compare](../../output/7353/r19/cell-dash/review/wasm-compare-10.png) /
  [standalone overlay](../../output/7353/r19/cell-dash/review/wasm-overlay-10.png)
- [검토 HWP](../../tests/fixtures/issue7353/cell-dash/prefix31-saved.hwp) /
  [독립 PDF](../../tests/fixtures/issue7353/cell-dash/prefix31-2020.pdf)

#### 메인터너 판정 및 다음 절편 — 저장 호스트 내어쓰기·줄간격과 표 원점

작업지시자는 셀 Dash 증적의 시각 판정 통과와 다음 절편 진행을 승인했다.
p34의 `anchored host paragraph insets`는 내어쓰기-2500URC만 있어도 앵커 문단을
거부하던 조건이다. 빈 호스트의 실제 저장 줄은400HU+뒤 간격200HU이며, 표 top은
문단 상대448HU+바깥 위141HU=589HU다. 줄 상자를 침범하지 않으면서 뒤 간격의
11HU를 점유한다. 기존 BodyAnchor는 실제 점유 끝과 다음 원점을 모두 침범 기준으로
사용했다. 입력의 빈 문자열을 줄 높이0으로 해석하거나 표 좌표를 다음 원점으로
clamp하지 않는다. 저장 줄 내어쓰기는 기존 TextComposer/localize에서만 적용한다.

첫35문단을 속성 변경 없이 추출→한컴 정상 저장→같은 반환HWP의 독립PDF11쪽을
확보했다. 원본과 정상 저장본 모두 p34 속성이 유지된다. 수정 전 셀Dash source의
기존 실행 바이너리로 정상 저장본이 같은 p34 거부를 내는 것을 보존했다(`before.log`).
이는 새 지원의 수정 전 명시적 거부 증거이며, 새 테스트를 옛 source에 실행한
FAIL로 과장하지 않는다. [fixture 출처와 독립 관측](../../tests/fixtures/issue7353/host-insets/README.md).

실행 경로: 입력 LineSeg/ParaShape→`TextComposer::compose`→`ParagraphEnd`의
occupied_end/next_origin→`BodyAnchor::resolve`의 before/tail_overlap→
`document_input::prepare`의 AnchoredFlow→`BodyCursor::fit`의 실제 예약 시작점→
`FlowCursor`/TableCursor 공통 조각→실제 table/text paint다. 저장 문단의 좌우 여백과
문단 앞 간격, fresh 앵커 문단의 inset 지원은 확대하지 않았다. 저장 줄 메트릭이 유효한
경로의 들여쓰기/내어쓰기만 텍스트와 앵커의 책임으로 분리했다.

일반 before>=0 예약은 기존 경로를 유지한다. tail_overlap>0은 실제 호스트 줄이
현재 페이지에 있는 경우에만 해당 뒤 간격에서 예약한다. fit 실패 시 이야기 흐름의
원점을 되돌리지 않고 표만 deferred로 이월한다. 호스트 뒤 공백 자체가 다음 페이지로
이어진 경우 source offset을 재적용하지 않고 deferred 원점을 사용한다. 수용된 조각의
끝과 원래 호스트 다음 원점의 합집합으로 다음 흐름을 정한다. 공백은 내용 컷과 별도로
보존되고, 부분 소비된 자식의 줄간격도 다음 조각으로 이어받는다.

이 문서의 다음 거부는 자식 표의 오른쪽 회피 여백이었다. 실제 border box는
부모 셀 안이지만 그 여백까지 추가 너비로 요구했다. TopAndBottom의 ExcludedTable/
PositionedTable 모두 `ir::bind_table`에서 실제 x+plan.width를 검사하고 같은 plan과
available_width를 `content` 검증·`flow` fit·paint에 전달한다. 전체 폭을 이미 회피하는
right margin으로 table ink를 늘리지 않는다. 자식의 실제1HU overflow는 계속 거부한다.
rowspan/컷 알고리즘·cell padding·표의 선언 최소 높이는 수정하지 않았다.

Native11쪽 review 직접 확인: 첫 빈 문단, 제목 아래 부모/자식 표 위치·줄바꿈·외곽이
기준PDF에 대응한다. 자식 표 위/높이 PDF137.1293/99.892px 대 V2137.24/99.9067px.
글꼴 외형과 미세 잔차를 별도 기록하며 자동 잉크 보조값25.91%는 사람 판정 정확도가 아니다.
앞서 승인한 첫31문단·10쪽은 최종 Native의 `previous-control/native.json`과
셀Dash의 `review/actual/native.json`을 `cmp`해 바이트 동일이다.
원본 전체는 p43 `unqualified stored field result`까지 전진했다(`full-original.log`).

#### 저장 호스트 원점 절편 — 최종 검증

`bash output/7353/r19/host-insets/tests.sh`의 정식 `tests/cases/` 선택 검사165건 PASS:
host_anchor_gap4, document_flow116, ir_text9, nested18, rowspan13, cell_dash2,
half_unit_indent3. 최종 결과는 `test-summary-final.log`다. 실제 빈 줄 상자 침범의 거부,
뒤 간격 내부 원점, 첫 조각 예산/원자 표 이월, 호스트 공백의 다음 페이지 이어받기,
자식 실제1HU 너비 초과 거부, 텍스트 들여쓰기와 표 원점 분리를 검사했다.
원본 source를 이용한 build와 Native/WASM lib Clippy, fmt/diff 검사도 PASS다.
전체 CI/workspace-all-targets 검증을 실행한 것으로 확대하지 않는다.
문단 뒤 간격만으로 tail_overlap이 생기는 경우의 독립 한컴 시각 근거는 이번 범위에 없다.

Docker 표준 WASM 빌드는7분20초 PASS. WASM SHA256은
`7fcbbb7343bc6d2919211023a1ad37a87055eee79b7c1537e23bd3b4035efc1a`다.
`node output/7353/r19/host-insets/capture.mjs --wasm`으로 같은 정상 저장 HWP를
실제 브라우저에서 실행했다. Native/WASM11쪽 수치 차이0, 기타 차이0, 모든 SVG 동일
(`review/backend-comparison.json`). `review/run.json`은 HEAD50823731af+WIP,
source manifest, 입력/기준PDF/WASM 해시를 연결한다. 빌드 이후 source manifest 대조 PASS.

최종 WASM11쪽 review와 standalone overlay를 직접 확인했다. 첫 빈 문단의 점유,
제목과 표 시작점, 자식 표 외곽·줄바꿈은 해당 기준PDF에 대응한다. 에이전트의 이번
지원 범위 판정은 충족이며, 글꼴 외형·농도 및 위에 기록한 미세 좌표 잔차는 남는다.
자동 잉크 보조값25.91%는 사람 판정 정확도가 아니다. 이 새 증적의 메인터너 판정은
아직 받지 않았으며 원본 전체/R5 완료를 선언하지 않는다. 다음 대상은 p43의 저장
field result 자격 검증이다. 기본 엔진 변경·commit/push는 하지 않았다.

- [최종11쪽 review](../../output/7353/r19/host-insets/review/wasm-review-11.png) /
  [compare](../../output/7353/r19/host-insets/review/wasm-compare-11.png) /
  [standalone overlay](../../output/7353/r19/host-insets/review/wasm-overlay-11.png)
- [검토 HWP](../../tests/fixtures/issue7353/host-insets/prefix35-saved.hwp) /
  [독립 PDF](../../tests/fixtures/issue7353/host-insets/prefix35-2020.pdf)

#### 메인터너 판정 및 다음 절편 — 열린 누름틀 시작 마커의 저장 본문

작업지시자가 위11쪽 시각 판정 통과와 다음 진행을 승인했다. p43 부모표 첫 셀의
첫 문단에는 ClickHere 시작2개(16UTF-16슬롯), 실제 본문152자, 문단 끝1유닛이 있다.
field_ranges와 orphan_field_ends는 없고, 셀의 후속12문단에도 종료 마커가 없다.
한 필드는 dirty=1, 다른 필드는 dirty=0이지만 둘의 안내문과 본문은 다르다.
문단 내부에서 반드시 닫힌 필드만 허용한 `fields::stored_result`가 이를 거부했다.

원본 첫44문단을 속성 변경 없이 추출→HWPX 직렬화→한컴 정상 저장한 HWP에서도
위 구조/본문/오프셋을 보존했다. 반환 HWP로 만든 독립 PDF는12쪽이며 실제 본문을
출력하고 안내문을 출력하지 않는다. 생성 job/환경은 fixture README에 연결한다.
기존 실행 binary의 p43 거부 및 정식 새 테스트의 같은 거부 FAIL을
`output/7353/r19/field-boundary/before.log`, `before-test.log`에 보존했다.

규칙: 완결 범위가 없는 시작 마커를 임의의 결과 범위로 바꾸지 않는다. ClickHere만
있는 정확한 시작 슬롯 뒤의 저장 리터럴을 보존하고 실제 줄 구성은 TextComposer가
소비한다. dirty=0만으로 뒤 본문을 안내문으로 지우지 않는다. 알려진 안내문과
구별되지 않는 텍스트, 잔재·편집·미확인 슬롯·혼합 객체·종료 마커는 여전히 거부한다.
이 절편은 다문단 필드의 명령 실행/편집/종료 의미 지원이 아니다.

경로: 파서 Paragraph의 controls/char_offsets/char_count/LineSeg→
`fields::stored_result`의 open_prefix_text 검증→`ir::bind_table`의 필드 소유 확인과
`text_ir`의 같은 검증→기존 TextComposer 저장 줄 결과→FlowBlock 예약/분할→공통
RenderNode→Native/WASM이다. 입력 IR·마커·본문은 삭제/변경하지 않으며 별도 원점
보정·높이 예외를 추가하지 않는다. 아래 실제 출력 확인에서 추가로 발견한 RowBreak
진입 조건을 수정했으며, 기존 FlowCursor의 예약/끝 컷 계산은 재사용한다.

#### 열린 누름틀 절편 — 실제 출력에서 발견한 저장 프레임 이어받기

필드 수용만 구현한 중간 Native 출력은 표 전체를12쪽으로 이월했다. 독립 PDF는
11쪽에서 시작하여12쪽으로 이어받으므로 페이지 수12가 같아도 시각 충족이 아니다.
RowBreak 행이 fresh page에 들어간다는 이유로 부분 페이지에서 저장 프레임 컷을
무시한 것이 원인이다. 정상 저장본 p8의 vpos35572→0 재시작이 실제 독립 근거다.

`fragment::fit_rows`의 분할 진입은 상단 정렬 RowBreak 행에 저장 프레임 경계가
있고 현재 예산을 넘거나 이미 이어받기 중인 경우에도 기존 cell cursor를 사용한다.
전체가 현재 예산에 들어가는 미시작 행은 기존 원자 배치를 유지한다. Never,
반복 제목의 원자 처리, rowspan group 및 비상단 정렬은 지원을 확대하지 않았다.

실제 경로는 `ir::stored_text::cell_frame_starts`→저장 문단의 공통 FlowBlock
(`StoredFrameStart`/tail)→TableContentPlan 높이→`fragment::fit_rows`의 현재 예산과
분할 진입→`FlowCursor::fit_cell_until`의 소유 컷/안 여백→같은 TablePlacement의
paint다. 소비된 내용 컷과 남은 물리 높이를 새로 혼합하지 않는다. 요구 줄이 안 맞는
예산에서 패딩만 소비하지 않고, 수용한 줄/패딩은 실제 조각 높이와 continuation으로
이어진다. 반복 제목/rowspan의 별도 경로는 이번 조건의 적용 대상이 아니다.

선택 정식 검사174건 PASS(`field-boundary/test-summary-final.log`):
open_field_markers3, filled_field5, stored_cell_frames6, stored_frame_end4,
rowbreak_span_continuation2, oversize_row3, document_flow116, nested18,
rowspan13, host_anchor_gap4. 작은 경계는 패딩만 fit하는 예산 거부, 첫 컷·다음 시작,
마지막 줄/패딩 종료, Never 및 경계 없는 RowBreak 대조군을 검사했다. 정상 저장
입력은 두 조각의 최종 좌표/높이, 빈 줄, 본문의 정확히 한 번 보존을 검사한다.

Native/WASM lib Clippy와 Native build PASS. 앞서 승인한 첫35문단 대조군의11쪽
전체 JSON은 동일한 원시 직렬화 probe로 `cmp`하여 바이트 동일하다. JSON을 재파싱해
출력하는 진단 probe의 부동소수점 직렬화 차이는 엔진 좌표 변화와 구분했다.
원본 전체의 다음 거부는 p89 `stored excluded cell anchor`이며 전체 문서/R5 완료가 아니다.
Native11·12쪽 review와 standalone overlay를 직접 확인했다. 저장 줄바꿈·표 시작과
이어받기는 기준PDF에 대응하며 글꼴 외형/농도 잔차는 남는다. 잉크 픽셀 일치율 보조값
14.86%/11.48%는 사람 판정 정확도가 아니다. fresh WASM 결과는 아래에 연결한다.

#### 열린 누름틀 절편 — fresh WASM 최종 증적

Docker 표준 빌드7분20초 PASS. WASM SHA256:
`d929e8d7c7d8134aa6915b8204f542e463a7659ec3374953f2e02a501eec836e`.
`node output/7353/r19/field-boundary/capture.mjs --wasm` PASS.
`review/backend-comparison.json`: Native/WASM12쪽 수치 차이0, 기타 차이0,
모든 SVG 동일. `review/run.json`에 HEAD50823731af+WIP, source manifest,
입력/기준PDF/WASM 해시를 기록했다. 빌드 이후 source manifest 대조도 PASS다.
최종 테스트는 대상 셀 전체 문자열의 동등성까지 검사하여174건 PASS이며 fmt/diff PASS.
전체 CI/workspace-all-targets 검증이나 기본 엔진 전환은 실행하지 않았다.

fresh WASM11·12쪽 review와 standalone overlay를 직접 확인했다. 실제 본문 시작,
11쪽 마지막 줄→12쪽 첫 줄 이어받기, 저장 줄바꿈과 문단 간격은 독립 PDF에 대응한다.
에이전트의 이번 지원 범위 판정은 충족이다. 글꼴 외형·농도 및 미세 프린터 잔차가
남고 잉크 픽셀 일치율14.86%/11.48%는 사람 판정 정확도가 아니다. 이 새 증적의
메인터너 판정은 대기이며 원본 전체/R5 완료로 확대하지 않는다. commit/push 없음.

- [11쪽 review](../../output/7353/r19/field-boundary/review/wasm-review-11.png) /
  [compare](../../output/7353/r19/field-boundary/review/wasm-compare-11.png) /
  [standalone overlay](../../output/7353/r19/field-boundary/review/wasm-overlay-11.png)
- [12쪽 review](../../output/7353/r19/field-boundary/review/wasm-review-12.png) /
  [compare](../../output/7353/r19/field-boundary/review/wasm-compare-12.png) /
  [standalone overlay](../../output/7353/r19/field-boundary/review/wasm-overlay-12.png)
- [검토 HWP](../../tests/fixtures/issue7353/field-boundary/prefix44-saved.hwp) /
  [독립 PDF](../../tests/fixtures/issue7353/field-boundary/prefix44-2020.pdf) /
  [생성 근거](../../tests/fixtures/issue7353/field-boundary/README.md)

#### 메인터너 판정 및 다음 절편 — 빈 호스트의 문단 상대 세로 위치

작업지시자가 위11·12쪽 시각 판정 통과와 다음 절편을 승인했다. 다음 p89는
빈 문자열/폭0 저장 줄(높이1000HU, 간격600HU)에 RowBreak 자리차지 표가 선언된
경우다. 문단 상대 세로784HU, 가로1238HU, 바깥여백141HU이며 기존 cell_anchor는
세로0만 허용하여 거부했다. 정상 저장한 첫90문단에서도 이 속성과 vpos4640HU가
보존되고 독립 PDF17쪽의 표 시작점은75.6+(4640+784+141)/75=149.8px에 대응한다.
입력 속성/LineSeg를 수동 변경하지 않았으며 생성 결과와 기존 거부는
`output/7353/r19/anchor-offset/`에 보존했다.

규칙은 빈 호스트 줄을 삭제하지 않고 문단 상대 위치와 물리 바깥여백을 구별하는 것이다.
`cell_anchor`→ParagraphItem::ExcludedTable→document_input/ir의 FlowBlock::AnchoredTable
공통 offset_y가 측정 `content::height`와 실제 `flow::fit_cell_until`의 최초 예산/원점에
사용된다. 첫 수용 시에만 offset_y를 소비하고 continuation은 위 여백만 재사용한다.
실패한 query는 호스트/offset을 소비하지 않는다. host 줄과 child 점유의 합집합으로
흐름을 전진시켜 빈 문단을0높이로 만들지 않는다. paint는 반환된 placement를 그대로
사용한다. 음수 offset, side-wrap, 가시 텍스트 호스트는 이 수용 조건에서 확대하지 않는다.
rowspan/끝 컷/행 높이는 수정하지 않으며 공통 자식 cursor의 실제 조각을 예약한다.

#### 빈 호스트 세로 위치 — 집중 검증

`bash output/7353/r19/anchor-offset/tests.sh` 선택 정식 검사189건 PASS:
excluded_anchor_offset5, body_exclusion10, open_field_markers3, filled_field5,
stored_cell_frames6, stored_frame_end4, rowbreak_span_continuation2, oversize_row3,
document_flow116, nested18, rowspan13, host_anchor_gap4. 로그는
`test-summary-final.log` 및 각 검사 `.log`다. 실물 기대값의 용지 왼쪽 원점은
5669HU이며 위 본문 원점5670HU와 구분한다. 처음에 둘을 같은75.6px로 기록한
테스트 기대값 오류를 원문 값으로 정정했고 파서 원문 값 assertion도 추가했다.

작은 합성 계약은 첫 조각에서 원래 여백+줄만 fit하고 offset 포함 시 실패하는 예산,
재시도·이어받기의 위 여백, 호스트 한 번 소비, 뒤 문단/마지막 종료를 검사했다.
RowBreak의 컷 조각은 아래 여백도 함께 예약하는 별도 실제 경로를 검사했다.
셀 adapter는 기존 fixture의 속성 변형으로 host 원점 불변, 자식 원점/부모 점유의
동일 이동, 음수 거부를 검사했다. 비영 offset의 실제 다쪽 한컴 출력은 이번 입력에
없으며 continuation은 합성 계약이다. 이를 실물 시각 증거로 확대하지 않는다.

Native build와 Native/WASM lib Clippy PASS, fmt/diff PASS. 전체 CI/workspace-all-targets
게이트는 별도 승인 전이므로 미실행이다. 승인된 첫44문단12쪽 대조군의 전체 JSON은
`previous-control/native.json`과 이전 `field-boundary/review/actual/native.json`을
`cmp`해 바이트 동일이다. 원본 전체는 p89를 통과하고 p101 `non-table cell control`에서
명시적으로 거부한다(`full-original.log`). 아직 원본 전체/R5 완료가 아니다.

Native17쪽 review/standalone overlay 직접 판독: 표 시작·줄바꿈·행 경계는 독립 PDF에
대응한다. PDF 위149.756px/높이780.5867px 대 V2위149.8px/높이781.4933px로,
하단은 약0.95px 잔차가 있다. 글꼴 외형·농도와 프린터 잔차는 남으며 임의 보정하지
않는다. 자동 잉크 픽셀 일치율20.13%는 사람 판정 정확도가 아니다.

#### 빈 호스트 세로 위치 — fresh WASM 최종 증적

Docker 표준 WASM 빌드7분27초 PASS. WASM SHA256:
`4a7e846bd93cf83e92fa18884b8e096616810039090f26834be632ed7ef23c3e`.
`node output/7353/r19/anchor-offset/capture.mjs --wasm` PASS.
`review/backend-comparison.json`에서 Native/WASM17쪽의 수치 차이0, 기타 차이0,
모든 SVG 동일을 확인했다. `review/run.json`은 HEAD50823731af+WIP,
source manifest, 입력/PDF/WASM 해시를 연결한다. 빌드 후 source 해시 대조 PASS.

fresh WASM17쪽 review와 standalone overlay를 직접 열어 제목 뒤 빈 문단·표 시작점,
행 경계·셀 줄바꿈·하단을 확인했다. 이번 지원 범위의 에이전트 판정은 충족이며,
글꼴 외형/농도와 앞서 명시한 미세 잔차는 남는다. 이 새 증적의 메인터너 판정은
대기다. 원본 전체 및 R5 완료, 전체 CI 통과로 확대하지 않는다. 기본 엔진 변경,
commit/push는 하지 않았다. 다음 대상은 p101의 셀 내부 비표 컨트롤이다.

- [17쪽 review](../../output/7353/r19/anchor-offset/review/wasm-review-17.png) /
  [compare](../../output/7353/r19/anchor-offset/review/wasm-compare-17.png) /
  [standalone overlay](../../output/7353/r19/anchor-offset/review/wasm-overlay-17.png)
- [검토 HWP](../../tests/fixtures/issue7353/anchor-offset/prefix90-saved.hwp) /
  [독립 PDF](../../tests/fixtures/issue7353/anchor-offset/prefix90-2020.pdf) /
  [생성 근거](../../tests/fixtures/issue7353/anchor-offset/README.md)

### 셀의 저장 하이퍼링크 본문과 이어받기

사용자가 앞 절편17쪽 fresh WASM 시각 판정 통과와 다음 진행을 승인했다.
p101의 다음 거부 지점을 조사했다. 셀 문단p34부터 하이퍼링크 필드가 있고,
문단p69는 저장 vpos가 문단 안에서0으로 재시작하며, p75 두 번째 줄은
첫 가시 글자136이 아닌 시작 표식128에서 시작한다. 정상 저장에서도 동일하다.
원본 첫102문단→HWPX 직렬화→한컴 HWP 정상 저장→그 HWP의 PDF 생성 절차와
해시는 [fixture 근거](../../tests/fixtures/issue7353/cell-control/README.md)에 있다.
속성·LineSeg를 임의로 고친 입력이 아니다. 독립 PDF21쪽을 얻었다.

구현 전 `anchor-offset/probe`는 같은 새 HWP를 p101 `non-table cell control`로
거부했다(`cell-control/before.log`). 이번 변경은 기존 기본 엔진의 결함 수정이
아니라 V2의 명시적 미지원 범위 확장이다. 수용 뒤 드러난 같은 표의 두 경계를
함께 처리했다. 링크 명령은 해석/실행하지 않고 저장 표시 문구와 글자모양만 사용한다.

- `fields::stored_result`의 범위·슬롯·저장 유효성 검증에 닫힌 Hyperlink를 포함.
  열린 링크·잘못된 범위/오프셋은 계속 거부한다. ClickHere 안내문 규칙은 그대로다.
- `stored_text::continuous_cell_paragraph`는 검증된 필드의 원래 글자 위치·범위를
  그대로 두고 행 원점만 연속 좌표로 만든다. `fields::is_row_start`는8슬롯 표식의
  정확한 경계를 수용하며 표식 내부 위치를 수용하지 않는다.
- p101 셀은 Center이지만 선언 높이282HU보다 내용이 커서 실제 정렬 밴드는0이다.
  `content::from_grid_rows`가 이미 만든 `content_offset_y`를 `fragment::fit_rows`의
  RowBreak 분할 수용과 실제 배치가 함께 사용한다. enum만 보고 분할을 금지하던
  조건을 제거했다. 양수 정렬 밴드, WithinCells의 새 Center 지원은 확대하지 않는다.

실제 호출 연결은 `ir::bind_table`의 저장 프레임 시작/끝→공통 text 구성→FlowBlock의
저장 컷과 Lines→`fit_rows`의 수용 높이/`fit_cell_until`의 내용 컷→CellPlacement→
TextPaint다. 바깥 `document_input`의 AnchoredTable→flow의 첫/이어받기 예산과
예약 높이→DocumentV2 후속 흐름도 실행했다. 표식은 소스 위치만 차지하고 별도
높이/폭을 만들지 않는다. 실제 빈 줄은 Lines로 보존한다. 양수 정렬 여백·Never는
분할하지 않고, rowspan/repeated header는 이번 변경의 별도 분할 경로가 아니며
기존 경로와 집중 대조군을 유지했다. paint 후 높이 확대나 좌표 덮어쓰기는 추가하지 않았다.

새 `issue_7353_stored_hyperlink`4건은 전체 셀의 모든 빈/가시 줄 순서·문구·높이와
셀 내부 좌표,20→21쪽의 문단 이어받기, 마지막 종료를 검사한다. 속성 변형은
실제 정렬 밴드/원래 높이만 fit하는 예산의 거부, Never,0여백 Top/Center/Bottom의
동일 출력, 불완전한 필드와 임의 표식 내부 컷 거부를 검사한다. 수정 전 명시적 거부
로그와 수정 후21쪽/4건 PASS를 연결하며, 합성 변형을 한컴 시각 근거로 세지 않는다.

기존 filled-field와 document-flow의 Hyperlink 무조건 거부 검사2건은 새 정상 저장
근거와 충돌하여 실패했다. 해당 검사는 Date 등 미지원 필드 종류 거부로 갱신하고, Hyperlink의
범위/표식 오류 거부는 새 정식 검사로 보호했다. 페이지수 baseline을 완화한 것이 아니다.
초기 Clippy `manual_is_multiple_of` 경고를 수정하고 Native/WASM lib Clippy 재실행
PASS. 전체 CI/workspace-all-targets는 별도 승인 전 미실행이다.

Native19~21쪽 review 직접 판독: 법령 문구/줄바꿈,20쪽 마지막 두 줄 뒤21쪽 첫 `다`,
표 하단을 확인했다. 대체 글꼴 외형/농도와 프린터 잔차는 남는다. 자동 잉크 일치율은
19쪽11.98%,20쪽11.26%,21쪽12.91%로 사람 판정 정확도가 아니다.
원본 전체는 p129 `body control or multiple anchors`에서 명시적 거부한다.
새 부분본의21쪽 통과를 원본 전체 또는 R5 완료로 확대하지 않는다.

#### 하이퍼링크 절편 — 최종 집중 검증

최종 소스의 Native lib 재빌드 뒤 `bash output/7353/r19/cell-control/tests.sh`
재검증201건 PASS (`test-summary-final.log`). 구성은 신규4, anchor-offset5,
body-exclusion10, open-field3, filled-field5, stored-cell-frames6, frame-end4,
rowbreak-span2, oversize-row3, vertical-alignment8, document-flow116, nested18,
rowspan13, host-anchor-gap4다. 신규 정식 검사는 본문 아래 경계 및 독립 PDF21쪽의
상하 표 외곽(프린터 양자화1px 이내)도 검사한다. 기준값을 현 출력으로 갱신하지 않았다.

최종 source로 이전 승인 prefix90의17쪽 전체 Native JSON을 다시 내보내
이전 절편과 `cmp` 결과 바이트 동일을 확인했다(`cell-control/previous-control/`).
fmt/diff PASS. 최종 Native19~21쪽 직접 판독에 앞서 Native18쪽 review와
20/21쪽 standalone overlay도 직접 판독했다.

#### 하이퍼링크 절편 — fresh WASM 최종 증적

Docker 표준 WASM 빌드7분20초 PASS. 새 WASM SHA256:
`0df55d8b05e08a4961618a3457914bfb001a78228285d49ae854a5cad3709112`.
`node output/7353/r19/cell-control/capture.mjs --wasm` PASS.
`review/backend-comparison.json`:21쪽 Native/WASM 수치 차이0, 기타 차이0,
모든 SVG 동일. 최종 `review/run.json`은 HEAD50823731af+WIP, source manifest,
입력/PDF/WASM 해시를 연결한다. 빌드 뒤 source 해시 대조 PASS.

fresh WASM19/20/21쪽 review와20/21쪽 standalone overlay를 직접 열어
링크 문구·줄바꿈·이어받기 및 셀 하단을 판독했다. 해당 지원 범위의 에이전트
판정은 충족이다. 글꼴 외형/농도·프린터 잔차는 남고 메인터너 시각 판정은 대기다.
PDF의 비활성 javascript 링크 대상에 대해 raster 도구가 `invalid link destination`
경고를 출력했지만 변환/캡처는 종료 코드0이며, 링크를 실행하지 않았다.
commit/push·기본 엔진 전환·전체 CI는 수행하지 않았다. 다음 대상은p129다.

- [19쪽 링크 본문 review](../../output/7353/r19/cell-control/review/wasm-review-19.png)
- [20쪽 review](../../output/7353/r19/cell-control/review/wasm-review-20.png) /
  [standalone overlay](../../output/7353/r19/cell-control/review/wasm-overlay-20.png)
- [21쪽 review](../../output/7353/r19/cell-control/review/wasm-review-21.png) /
  [standalone overlay](../../output/7353/r19/cell-control/review/wasm-overlay-21.png)
- [검토 HWP](../../tests/fixtures/issue7353/cell-control/prefix102-saved.hwp) /
  [독립 PDF](../../tests/fixtures/issue7353/cell-control/prefix102-2020.pdf)

#### 하이퍼링크 절편 승인 / 다음 쪽 번호 재선언 조사

메인터너가 위19~21쪽 시각 판정을 통과시키고 다음 절편을 승인했다.
원본p129는 빈 문단에 `PageNumberPos(position=5,dash='-')`가 들어 있는 경우다.
원인은 `document_input::prepare`의 구역당 최초 선언만 허용하는 조건이다.
이 빈 문단의 저장1500HU 줄 높이와752HU 간격을 없애는 수정은 하지 않는다.
입력 선언→BodyPlan→fit에서 수용한 host owner→페이지 footer→직렬화 성공 뒤
session 상태 확정 경로를 조사한다. 선언 재등장과 위치 변경/해제, 같은 페이지 내
순서를 정상 한컴 저장/출력 대조군으로 확인한 뒤 적용 범위를 결정한다.
원본133문단 부분본은 속성·줄 정보를 수동 수정하지 않고 정상 한컴 저장을 거친다.

#### 쪽 번호 재선언 — 독립 근거와 구현

`tests/fixtures/issue7353/page-number-timeline/README.md`에 정상 저장/PDF 출처와
입력·결과 해시를 기록했다. 원본133문단 부분본은23쪽, 별도 timeline은6쪽이다.
한컴 timeline은 가운데1→왼쪽2→왼쪽3→오른쪽4→번호 해제→오른쪽6이다.
같은4쪽의 가운데/오른쪽 선언 중 뒤 선언이 적용됨을 PDF로 확인했다.
PDF trace 기준선565.13372pt와 bbox 정렬 edge/center를 정식 테스트에 연결했다.

`document_input.rs:118–185`의 단일 선언 제한을 소스 순서의 목록으로 바꿨다.
`document.rs:186–210`은 fit에서 실제 수용한 host 줄/표 owner로 마지막 선언을
선택한다. 이전 커밋 상태보다 오래된 owner의 이어받기는 상태를 되돌리지 않는다.
footer는 해당 story의 기존 `PageNumberStory::render` 결과를 그대로 소비한다.
직렬화 성공 뒤 cursor/활성 선언/출력쪽 수를 함께 확정한다. 실패·대기 중인
host는 앞 페이지에 영향을 주지 않는다. 쪽 번호 변경으로 번호를 재시작하지 않는다.

본문 측정·예약·분할 컷·paint 원점은 변경하지 않았다. 선언을 담은 p129 빈 줄은
최종 render tree에서1500HU 높이와 직전 줄부터2252HU 간격으로 확인했다.
번호0 변형과 본문 tree의 완전 동일성도 검사했다. 행 분할/rowspan/클리핑의
알고리즘 변경은 비해당이며 기존 document/TAC/중첩 경로를 집중 대조했다.

수정 전 보존 Native probe(`cell-control/probe`)에서 동일 입력2건을 실행했다.
`body-next/before-prefix.log`는p129, `before-timeline.log`는p1에서 같은
`body control or multiple anchors` 오류를 기록했다. probe SHA256은
`effb5f520b80bfc13d5f9326c7394db2ce7d930cf95abad7e03f4aff08d4bb93`이며
직전 절편의 source manifest와 연결된다. 빌드 실패가 아니라 실제 DocumentV2
공개 진입의 거부다. 변경 후 동일 입력은 각각23/6쪽을 출력한다.
신규5건 전체를 이전 라이브러리로 실행한 것은 아니며, 신규 결함의 수정 전
실행 증거는 위2건의 공개 경로 거부다. 경계 변형들은 변경 후 보호 계약으로 구분한다.

`bash output/7353/r19/body-next/tests.sh`:7 suite158건 PASS.
신규5, document-flow116, stored-hyperlink4, body-exclusion10,
host-anchor-gap4, nested18, fresh-TAC-full-document1이다.
신규 정식 검사에 페이지 수/오류 이유 확인을 보강한 뒤 `test-final.log`5건 PASS.
Native/WASM lib Clippy, fmt/diff PASS. 이전 승인 prefix102의21쪽 Native JSON은
이전 산출과 `cmp` 바이트 동일(`body-next/previous-control/`).

Native22/23쪽 review,22쪽 standalone overlay 및 timeline4쪽 review를 직접
판독했다. 줄바꿈·문단 원점·번호의 정렬 기준을 확인했다. p128 마지막 줄의
PDF x126.531425pt(96dpi168.70857px)와 V2 x168.78667px도 대조했다.
대체 글꼴 농도/폭과 프린터 양자화 차이는 남는다. 원본 전체의 다음 명시적
거부는p161 `nested anchor, TAC, wrap or outer margin`이며 별도 다음 대상이다.

기존 `issue_7353_table_v2_page_number`를 추가 실행하자9통과/1실패였다.
실패는 `unsupported_stories_are_not_silently_discarded`의case4(뒤 본문 문단에
두 번째 쪽 번호 선언 자체를 거부)다. 독립 한컴 timeline과 충돌하는 과거 지원
제한이므로 뒤 선언의 미지원 숫자 format 거부로 변경했다. 새 일반 재선언 검사는
그대로 두고, 분할 표 뒤 문단의 재선언에서 HWP/HWPX 모두 앞 페이지 가운데/
수용 페이지 왼쪽과 본문 동일성을 보호하는 정식 검사도 추가했다.
`test-page-number-final.log`:11건 PASS. 최종 집중 검증 합계8 suite169건 PASS.
쪽 수 baseline/좌표 허용치/ignore를 바꾼 것이 아니다. 새 입력은 정식
`tests/cases/`와 fixtures에만 추가했으며 파생 suite를 PR source로 만들지 않았다.

#### 쪽 번호 재선언 — fresh WASM 및 시각 판정 요청

Docker 표준 빌드7분17초 PASS. WASM SHA256:
`ce3b4d74a7fef360166e88e6899e505d0dd4c7b78dbd34de589f8d9cabb1ffff`.
`node output/7353/r19/body-next/capture.mjs --wasm` 및
`--timeline --wasm` 모두 PASS. 부분본23쪽/대조군6쪽 각각 Native와 WASM의
수치 차이0, 기타 차이0, 모든 SVG 동일(`review`/`timeline-review`의
`backend-comparison.json`). run manifest는 HEAD50823731af+WIP의 source
해시와 입력/PDF/WASM 해시를 연결한다. 빌드 후 source manifest 대조 PASS.

새 WASM22/23쪽 review,22쪽 standalone overlay, timeline2/4/5/6쪽 review와
4쪽 standalone overlay를 직접 판독했다. 본문과 번호 위치·동일 페이지 최종 선언·
해제/재개를 확인했다. 에이전트 판정은 해당 범위 충족, 메인터너 판정은 대기다.
자동 잉크 일치율은22쪽12.37%,23쪽13.56%,timeline2쪽15.25%,4쪽10.18%다.
이는 대체 글꼴 농도/외형과 인쇄 잔차를 포함한 보조값이지 사람 판정 정확도가 아니다.
본문 줄바꿈/원점/높이가 유지되는 근거는 정식 좌표 검사와 직접 판독으로 분리했다.

- [22쪽 WASM review](../../output/7353/r19/body-next/review/wasm-review-22.png) /
  [standalone overlay](../../output/7353/r19/body-next/review/wasm-overlay-22.png)
- [23쪽 WASM review](../../output/7353/r19/body-next/review/wasm-review-23.png)
- 위치 변경: [왼쪽2](../../output/7353/r19/body-next/timeline-review/wasm-review-2.png) /
  [오른쪽4](../../output/7353/r19/body-next/timeline-review/wasm-review-4.png)
- [해제5](../../output/7353/r19/body-next/timeline-review/wasm-review-5.png) /
  [재개6](../../output/7353/r19/body-next/timeline-review/wasm-review-6.png)
- [부분본 HWP](../../tests/fixtures/issue7353/page-number-timeline/prefix133-saved.hwp) /
  [한컴 PDF](../../tests/fixtures/issue7353/page-number-timeline/prefix133-2020.pdf)

원본 전체/R5 완료로 확대하지 않는다. 다음 대상은p161의 중첩 표 속성 거부다.
commit/push·기본 엔진 전환·전체 CI는 이번 절편에서 수행하지 않았다.

### 다음 승인: 셀 초기 단 정의와 중첩 저장 프레임의 연결

메인터너가 직전 쪽 번호 재선언 절편의 시각 판정 통과와 다음 진행을 승인했다.
기존22~23쪽 판정은 통과로 확정한다. 이번 대상은 원본p161의
`cell13/p0/c1` 자리차지 자식 표다. 근거 자료는
`tests/fixtures/issue7353/local-column-anchor/README.md`에 연결한다.

입력의 첫 셀 문단에는 일반1단 `ColumnDef`가control0, 비TAC/TopAndBottom/
ParaTop/ColumnLeft 표가control1로 선언되어 있다. 저장 host는 폭0, 높이1500HU,
간격300HU인 실제 줄이다. 자식 마지막 문단의 저장 두 줄은v37000→0으로
다음 프레임에 이어진다. 원본 첫163문단을 속성 삭제 없이 HWPX로 내보낸 뒤
한컴2020profile에서 정상 HWP 저장하고 그 HWP의26쪽 PDF를 생성했다.
원본 자체와 재저장본의 차이를 혼동하지 않는다.

수정 전 `body-next/probe`는 동일 정상 HWP를p161에서
`nested anchor, TAC, wrap or outer margin`으로 거부했다(`nested-next/before.log`).
초기 단의 수용만 연결한 중간본은26쪽을 만들었지만 부모 마지막 행 전체가
26쪽으로 이월됐다. Native 직접 판독으로 이를 검출했고
`nested-next/anchor-only-review/`와 `anchor-only-probe`를 보존했다.
이 중간 산출을 시각 통과 자료로 제출하지 않았다.

적용 규칙과 실제 소비 경로:

- `ir::initial_cell_column`에서 셀 첫 문단의 단일 영역을 검증한다.
  `cell_anchor::excluded_slot/compose_in_frame`은 구조 슬롯을 삭제하지 않고
  control1의 `ExcludedTable`을 생산한다. 명시적 단 정의 없는 Column 참조,
  다단·중간 선언은 수용하지 않는다.
- `ir::bind_table`은 동일 item의 host/offset/owner와 자식 plan을
  `FlowBlock::AnchoredTable`로 연결한다. `text_ir`은 그 item의 control1을
  paint 슬롯으로 저장하고 원본 control1에 바인딩한다. host와 자식은 같은
  문단 원점을 공유하며 host의1500HU 점유를 삭제하거나 별도 빈 줄로 더하지 않는다.
- `FlowBlock::has_stored_frame_cut`은 분할 가능한 비inline 자식의 저장 컷
  존재를 부모에 전달한다. 실제 컷 소유권은 자식 cursor에 남는다.
  `TableCursor::fit_rows`의 부모 분할 결정 → `FlowCursor::fit_cell_until` →
  자식 `fit_in_frame`까지 동일 프레임 소비 모드를 전달한다. 자식 전체 높이가
  남은 예산에 들어가도 부모의 분할 프레임 안에서는 저장 컷을 건너뛰지 않는다.
- 자식 continuation이 저장 프레임에서 시작하면 부모 셀의 아래 여백을
  해당 조각 예산에 먼저 확보하고 다음 조각의 위 여백을 한 번 적용한다.
  `CellPlacement.content_origin`과 자식의 최종 bbox가 이 결과를 소비한다.
  여백만 fit하는 실패는 커서를 확정하지 않는다. 이미 소비한 host/제목은
  반복하지 않고 마지막 자식 줄 뒤의1300HU 빈 문단과 표 외곽도 보존한다.
- 저장 컷을 가지지 않는 완전한 동반 셀은 첫 수용 조각의 물리 높이로
  Center/Bottom 정렬한다. 수용 예산 안에 내용 전체가 들어가야 하며, 실제
  placement를 그 원점에서 다시 질의한다. paint만 이동하거나 원래 작은
  높이를 수용한 뒤 외곽을 늘리지 않는다. 컷을 가진 셀의 비0 정렬 band는
  여전히 미지원이다.
- Never/inline 자식은 원자적이며 descendant 컷을 전파하지 않는다.
  rowspan 조각은 기존 row-group 경로이므로 새 컷 전파 대상에서 제외한다.
  header의 atomic prefix는 저장 프레임 강제 소비를 하지 않고 body만 이어받는다.

신규 정식 검사 `tests/cases/issue_7353_local_column_anchor.rs`는 정상 저장본의
25/26쪽 최종 좌표·부모/자식 외곽·빈 host·후속 빈 문단·원문 유닛 전체 보존,
단 좌표 반례, 1/2단계 자식의 경계 예산과 원자적 재시도, Never 대조군,
Center/Bottom 동반 셀의 첫 조각 정렬과 중복 방지를 검사한다.
변형 입력의 다단 거부 검사는 typed IR에서 실행한다. 최초 HWP 재직렬화 변형은
기존 raw 단 속성이 유지되어 의도한2단 입력이 아니었고, 그 실행은 구현의
다단 수용 증거가 아니다. 실제 변형의 qualification 검사는 별도로 통과했다.

검증 및 Docker WASM 시각 결과는 아래에 이어 기록한다.

최종 source는 HEAD50823731af+기존WIP이며 `nested-next/source.sha256`으로 고정했다.
`tests-final.log`14suite239건, `tests-extra-final.log`4suite38건이 통과했다.
신규6번째 동반 셀 경계와 원본 슬롯/직렬화 변형 검증을 추가한
`tests-boundary-final.log`6건까지 반영하면18suite 총278건 PASS다.
전체 회귀/CI를 실행한 수치가 아니다. Native/WASM lib Clippy는 각각
`clippy-native-final.log`/`clippy-wasm-final.log`에서 PASS다.
이전 승인 prefix133의23쪽 Native JSON은 `previous-control/`에서 직전 산출과
`cmp` 바이트 동일이었다. 새 기능의 수정 전 증거는 동일 정상 입력의 공개 진입
거부(`before.log`)와 중간본의 잘못된 페이지 소유/시각 출력이며, 신규6개 테스트
전체를 이전 라이브러리로 실행했다고 주장하지 않는다.

원본 전체 재검사(`nested-next/full.log`)는 여전히p161의
`unqualified stored cell frame reset`에서 멈춘다. 원본 자식p9 첫 줄은v100으로
재시작하는 반면 한컴 정상 재저장본의 마지막 문단 둘째 줄은v0으로 재시작한다.
현재 프레임 수용은0만 허용하므로 이것이 다음 원본 입력 조사 대상이다.
숫자만0으로 바꾸거나 재저장본 성공을 원본 성공으로 보고하지 않는다.

최종 Docker WASM과 직접 시각 판독:

- `docker compose --env-file .env.docker -p rhwp run --rm wasm` 성공
  (`nested-next/wasm-build.log`). WASM SHA256은
  `ddcfb01decd1c1aed4142621c82523718040f94a0c5d4b33d76110d980dda745`다.
  빌드 뒤 `sha256sum -c nested-next/source.sha256`에 해당하는 저장소 루트 검사를
  통과했고 이후 production source 변경은 없다.
- `node output/7353/r19/nested-next/capture.mjs --wasm` 성공.
  동일 정상 저장본의 Native/fresh WASM은 모두26쪽이며 render tree 수치 차이0,
  그 외 차이0, 전체26쪽 SVG 동일이다(`review/backend-comparison.json`).
  입력·PDF·source manifest·WASM 해시는 `review/run.json`에 고정했다.
- `cargo fmt --all -- --check`와 `git diff --check` PASS.
- Native와 fresh WASM의25~26쪽 review 및 standalone overlay를 직접 판독했다.
  25쪽 부모 마지막 `근거설명` 행이 유지되고 자식 마지막 문단은 `지정 전까지`에서
  이어받는다. 26쪽에는 `각 주민대표단별로 약 1년간 운영하는 것으로 가정` 한 줄과
  후속 빈 문단의 점유가 보존되며 부모/자식 외곽이 끝난다. 제목 셀의 반복은 없다.
  글꼴 외형·농도와 인쇄 배율/양자화 잔차는 남는다. 에이전트 판정은 이번 범위
  충족이며 메인터너 시각 판정은 대기다. 원본 전체 또는 R5 완료 판정은 아니다.

판정 자료:

- [25쪽 WASM review](../../output/7353/r19/nested-next/review/wasm-review-25.png) /
  [standalone overlay](../../output/7353/r19/nested-next/review/wasm-overlay-25.png)
- [26쪽 WASM review](../../output/7353/r19/nested-next/review/wasm-review-26.png) /
  [standalone overlay](../../output/7353/r19/nested-next/review/wasm-overlay-26.png)
- [검토용 정상 재저장 HWP](../../tests/fixtures/issue7353/local-column-anchor/prefix163-saved.hwp) /
  [동일 입력 한컴 PDF](../../tests/fixtures/issue7353/local-column-anchor/prefix163-2020.pdf)

commit/push·기본 엔진 전환·전체 CI는 수행하지 않았다.

### 다음 승인: 문단 앞 간격을 포함한 저장 셀 프레임 시작

메인터너가 직전 셀 초기 단/자식 저장 프레임 절편의25~26쪽 시각 판정을 통과시키고
다음 진행을 승인했다. 이번 대상은 원본p161 자식p9의v100 재시작 거부다.

원인과 독립 근거:

- 원본 해당 문단의 ParaShape.spacing_before는200이며 공통 style resolver는
  URC2배 스케일을 적용해100HU로 해석한다. 이전 수용은 무조건v0만 허용했다.
- 직전 정상 저장 부분본에 같은 서식의 실제 빈 문단1개만 마지막 문단 앞에 삽입하고,
  HWPX→한컴 정상 HWP 저장→동일 HWP의PDF 순서로 대조군을 만들었다.
  LineSeg의v100을 손으로 쓰지 않았다. 정상 저장본에서도 마지막 문단p10/l0가
  정확히100HU로 다음 프레임을 시작했다. PDF25쪽에는 추가 빈 문단,26쪽에는
  마지막 문단 두 줄이 나타난다. 생성 절차·job·해시·독립 좌표는
  `tests/fixtures/issue7353/frame-origin/README.md`에 기록했다.
- 같은 정상 입력과 정식 신규 검사 모두 수정 전 공개 DocumentV2 진입에서
  `unqualified stored cell frame reset`으로FAIL이다. 빌드 실패를 재현으로 세지 않았다.

실제 생산/소비 경로:

- `stored_text::cell_frame_starts`는 실제 줄 원점 감소를 검사하되 문단 첫 줄에
  한해 해석된 spacing_before와 일치하는 비영 원점을 수용한다. 문단 내부 컷은
  기존0 원점 계약을 유지한다. 특정100HU나 임의의 작은 값 범위는 사용하지 않는다.
- `CellParagraphComposer`의 boolean 프레임 보존 응답을 style에 근거한 컷 목록
  query `stored_frame_starts`로 바꿨다. `TextComposer`와`IrTextComposer`가 동일
  qualification을 사용하고 custom/fresh composer의 기본 반환은 빈 목록이다.
- `ir::bind_table`은 문단 앞에 `StoredFrameStart`만 추가한다. 원본v100을 별도의
  Space로 더하지 않는다. `stored_text::localize`가 저장 문단 원점을 빼고 공통
  문단 배치가 같은 resolved spacing_before를 한 번 적용한다. 그 최종 노드에서
  `TextComposer::compose`가 Space/Lines를 생산해 fit과 paint가 함께 소비한다.
- 이전 절편의 `FlowCursor::fit_cell_until`/자식 `fit_in_frame`과
  `CellPlacement.content_origin` 경로는 그대로다. 프레임 여백 예약·예산 실패
  rollback·자식 소유 유닛·조각 종료를 바꾸거나 paint 원점만 보정하지 않았다.
  문단 앞 간격은 새 프레임에서 줄과 함께 fit되어야 하며 이미 소비한 제목은 반복하지 않는다.

정식 신규 검사 `issue_7353_stored_frame_origin`:

- 정상 한컴 저장본96/144dpi:25쪽 빈 줄,26쪽 두 줄/앞 간격1회, 부모·자식 외곽,
  후속1300HU 빈 문단, 모든 원문 유닛1회 보존 및 종료를 검사한다.
  실제 첫 글줄 기준선과 부모 외곽은 독립 PDF trace 좌표에도 대조한다.
- 비영 값1/99/101/1200을 지정한 typed IR 반례는 거부한다.
- 내부 이어받기 줄의v100은 문단 앞 간격을 재적용할 근거가 없으므로 거부한다.
- 수정 전1FAIL/2PASS(`frame-origin/test-before.log`), 수정 후3PASS
  (`test-final.log`). 최초 테스트 소스 API명 오타로 인한 컴파일 실패는 결함 재현에
  포함하지 않았고, 올바른 API로 같은 이전 라이브러리를 실행한 실패만 근거로 삼았다.

검증: 새 검사와 local-column-anchor/document-flow/nested/text/stored-cell-frames/
stored-frame-end/fragment-minimum-band의8suite 총202건 PASS(`frame-origin/tests.log`).
Native/WASM lib Clippy PASS. 전체 CI 게이트를 수행한 결과는 아니다.
Native25~26쪽 compare/review와26쪽 standalone overlay를 직접 열어 빈 줄·원점·
줄바꿈·외곽 및 후속 내용을 확인했다. 대체 글꼴 외형/농도와 인쇄 잔차는 남는다.
fresh Docker WASM 검증 결과는 아래에 이어 기록한다.

원본 전체는 이제p161을 통과해p172의 `unqualified stored cell frame reset`에서
멈춘다(`frame-origin/original.log`). 해당 위치는cell80/p0/c1/cell0/p21/l0이며
v9213, 앞 간격은100HU다(`original-resets.log`). 이번 규칙과 일치하지 않아
수용하지 않았으며 다음 독립 조사 대상으로 남긴다. 원본 전체 또는R5 완료가 아니다.

최종 Docker WASM 검증:

- `docker compose --env-file .env.docker -p rhwp run --rm wasm` 성공,7분14초.
  WASM SHA256 `faae4c55624a2384538fe0aa32fde600a34b62f1630c84d47c8624eb2d5620b4`.
- HEAD50823731af+기존WIP의 production source를 `frame-origin/source.sha256`으로
  고정하고 빌드 뒤 검증했다(`source-verify.log`). production source 후속 변경 없음.
- `node output/7353/r19/frame-origin/capture.mjs --wasm` 성공.
  Native/fresh WASM26쪽 전체 render tree 차이0, SVG 동일
  (`review/backend-comparison.json`). source/입력/PDF/WASM 해시는 `review/run.json`.
- fresh WASM25/26쪽 review 및26쪽 standalone overlay를 직접 판독했다.
  25쪽 빈 문단과26쪽 앞 간격·두 줄·외곽·후속 빈 문단을 확인했다.
  해당 범위 에이전트 판정은 충족, 메인터너 시각 판정은 대기다.
- 직전 승인 prefix163의 Native26쪽은 이전 산출과 `cmp` 바이트 동일
  (`previous-control/`). fmt check 및 `git diff --check` PASS.

판정 자료:

- [25쪽 WASM review](../../output/7353/r19/frame-origin/review/wasm-review-25.png)
- [26쪽 WASM review](../../output/7353/r19/frame-origin/review/wasm-review-26.png) /
  [standalone overlay](../../output/7353/r19/frame-origin/review/wasm-overlay-26.png)
- [빈 문단을 추가한 정상 저장 대조군 HWP](../../tests/fixtures/issue7353/frame-origin/paragraph-start-saved.hwp) /
  [동일 입력 한컴 PDF](../../tests/fixtures/issue7353/frame-origin/paragraph-start-2020.pdf)

기본 Legacy 엔진 전환·commit/push·전체 CI는 수행하지 않았다.

### 후속 절편: p172의 셀 단 기준 앵커와 문단 간격 (진행 중)

직전 `frame-origin` 25~26쪽은 메인터너 시각 판정 통과. 다음 절편 진행 승인에 따라
p172를 조사한다. 원본의 v9213 감소를 곧바로 새 프레임으로 허용하지 않는다.
원본 첫174문단을 속성 변경 없이 HWPX로 직렬화하고 한컴2020 정상 저장한 대조군은
같은 후속 문단을 v100(문단 앞 간격)으로 다시 저장한다. 원본과 재생성본은 구분한다.

- 정상 저장 job `e08822d9-de04-477d-9d27-c5a23bd8e34e`, PDF job
  `1d5d2277-2234-4597-a085-de801f095cab`, PDF28쪽.
- 입력/진단/기준: `output/7353/r19/next-172/`. 수정 전 정상본은 p172에서
  `stored excluded cell anchor`로 거부된다(`before.log`).
- 깊은 자식 이야기 p20/p23의 표는 단 왼쪽 기준, 앞 간격100HU,
  문단 좌우 여백50HU, 저장 host column_start50HU다. 단 정의는 같은 셀 p0에 있다.
  PDF27쪽의 3x12 표 및28쪽 5x4 표를 직접 열어 확인했다.
- 변경값의 경로: `ir::bind_table`의 셀 최초 단 정의 → composer의 셀 단 문맥 →
  `cell_anchor`의 host 원점/offset_y/host_advance → `FlowBlock::height`와
  `FlowCursor::fit_cell_until`의 예산 및 실제 LinePlacement → TextPaint.
  host.bounds의 x/y를 마지막 배치에서 덮어쓰던 지점도 함께 변경한다.
- 전체 부모 표는 앵커 준비를 통과한 후28쪽에서 `DoesNotFit`(요구높이1245.6133px)로
  멈춘다. 이 부모 분할 문제와 앵커 해석의 검증을 섞지 않기 위해 같은 원문 셀80의
  1x1 내부 표를 별도 정상 저장 대조군으로 준비한다. 원본 전체 일치/완료 주장이 아니다.

구현·검증 결과(같은 절편):

- `CellParagraphComposer::compose_in_cell`에 adapter가 검증한 셀1단 문맥을 전달한다.
  최초 p0의 단 정의를 같은 셀 뒤 문단에서 사용하며 다른 셀로 전파하지 않는다.
  단 정의 부재/다단/중간 단 선언은 이번 지원으로 바꾸지 않는다.
- `cell_anchor::compose`는 Column/Left 표의 x를 문단 왼쪽 여백과 분리한다.
  host는 저장 column_start와 resolved margin_left의 일치를 확인한다.
  앞 간격은 host의 y/advance에만 적용하고, Para/Top 표는 간격 전 문단 원점을 쓴다.
  초기 구현에서 표에도 앞 간격을 더한 것은2쪽 독립 PDF 좌표 검사로 발견해 제거했다.
- 같은 결과의 소비: `FlowBlock::height`는 host의 y+height를 점유 끝으로 보고,
  fit은 그 끝까지 예산을 예약하며 실제 LinePlacement도 host의 x/y를 보존한다.
  host 예산 실패 시 자식 컷과 host는 소비되지 않는다. 내용 끝/여백 이어받기는
  기존 anchor_tail을 사용하고 paint에서 별도 원점을 덮어쓰지 않는다.
- 정상 분리본 `tests/fixtures/issue7353/cell-column/cell-column-saved.hwp`와 동일 입력
  한컴PDF2쪽으로 검증. 분리 편집·초기 생성 실패·정상 저장 job·해시·독립 좌표는
  같은 폴더README의 후속 문단 절에 기록했다. 원본 전체의 대체 정답지가 아니다.
- 수정 전 보존 Native probe: 같은 정상 분리본이 `stored excluded cell anchor` FAIL.
  중간 단 문맥 연결본: 신규 정식검사2FAIL/1PASS(`test-initial-anchor.log`),
  최종3PASS. 이 중간본을 작업 시작 전 전체 코드라고 부르지 않는다.
- 최종12개 focused suite 총224PASS: 새 cell-column-insets와 기존 stored-frame-origin,
  local-column-anchor, document-flow, nested, text, stored-cell-frames, stored-frame-end,
  fragment-minimum-band, excluded-anchor-offset, host-anchor-gap, body-exclusion.
  최종 Native/WASM lib Clippy PASS, fmt check/diff check PASS. 전체CI 실행 아님.
- 신규 정식검사는96/144dpi의 host/표 최종 좌표, 독립PDF 외곽, 모든 원문 내용1회,
  뒤 문단 및2쪽 종료를 검사한다. 합성 작은 예산은31px 거부/32px 수용 뒤
  host간격3px와 다음10px줄을 이어받는 실제 fit 경계를 검사한다.
- Native1/2쪽 review 및2쪽 standalone overlay 직접 판독: 두 표 위치·내용·줄바꿈과
  후속 문단 보존 확인. 폰트 외형/농도와 인쇄 배율 잔차는 남는다.
  직전 승인 frame-origin26쪽 전체 Native JSON은 `cmp` 바이트 동일.
- 최초 WASM 빌드는 PDF 대조로 원점 수정이 필요해 중단했다. Native 최종 좌표검사 후
  Docker 재빌드 중이며 완료 전 fresh WASM 판정으로 보고하지 않는다.

남은 범위: 최종 코드에서 첫174문단 정상 저장본은28쪽에서 요구높이1242.9467px의
`DoesNotFit`로 멈춘다(`full-after-final.log`). 원본 전체는 p172의 v9213에 대한
`unqualified stored cell frame reset`을 그대로 거부한다(`original-final.log`).
이 실패를 page-count 예외나 임의 저장 좌표 허용으로 숨기지 않았다. 이번에 판정할 것은
분리한 정상 저장본의 셀 단 기준/문단 원점 규칙이며 전체 부모의 분할 완료가 아니다.

최종 fresh WASM 및 판정 자료:

- Docker 빌드 성공,7분17초(`next-172/wasm-build-final.log`). WASM SHA256
  `0bca78c8aec99d4da91e9f32bcf05071d23ccba2529cf94eefc5fc539019808f`.
- `node output/7353/r19/next-172/capture.mjs --wasm` 성공.
  Native/fresh WASM2쪽 전체 render tree 차이0, SVG2쪽 모두 동일
  (`column-review/backend-comparison.json`). HEAD50823731af+WIP이며
  `source.sha256`은 빌드 후 재확인했다. 입력/PDF/WASM/source는 `column-review/run.json`.
- fresh WASM1/2쪽 review와 standalone overlay를 직접 판독했다. 두 자식 표의
  위치·줄바꿈·후속 문단 보존은 에이전트 판정 충족. 폰트 외형/농도 및 인쇄 잔차는
  남으며 메인터너 시각 판정은 대기다. 전체 원본과 전체CI는 통과 주장하지 않는다.
- [1쪽 건설공사비지수 표 review](../../output/7353/r19/next-172/column-review/wasm-review-1.png) /
  [standalone overlay](../../output/7353/r19/next-172/column-review/wasm-overlay-1.png)
- [2쪽 건축비 표 review](../../output/7353/r19/next-172/column-review/wasm-review-2.png) /
  [standalone overlay](../../output/7353/r19/next-172/column-review/wasm-overlay-2.png)
- [정상 저장 HWP](../../tests/fixtures/issue7353/cell-column/cell-column-saved.hwp) /
  [동일 입력 한컴 PDF](../../tests/fixtures/issue7353/cell-column/cell-column-2020.pdf).

기본 Legacy 경로 전환·commit/push는 수행하지 않았다.

### 후속 절편: 세로 병합과 비병합 셀의 저장 분할 공존 (진행 중)

직전 `cell-column`2쪽은 메인터너 시각 판정 통과. 다음 절편 승인에 따라 정상
저장 prefix174 전체 부모 표의 실패를 처리한다. 원본 p172의 v9213을 임의로
새 프레임으로 허용하는 변경은 하지 않는다.

원인: `TableCursor::fit_rows`는 표 어디에든 rowspan이 있으면 모든 행을
`fit_row_groups`로 보냈다. 이 경로는 첫 조각에 셀 내용을 통째로 소비하므로,
p172의 r26c2(비병합)의3줄/1줄 저장 컷과 r27c1(가로 병합만 있음)의 자식 컷도
무시했다. 앞 행이26쪽에서 통째로 밀렸고 마지막 행은28쪽에서
`DoesNotFit(required_height=1242.9467px)`로 실패했다. 특정 문서 번호가 아닌
**현재 내용 소유 셀의 rowspan 여부**와 공통 분할 결과로 경로를 구분한다.

실제 생산·소비 경로:

| 구간 | 규칙과 호출 경로 |
| --- | --- |
| 원문→컷 | 정상 한컴 저장본의 `stored_frame_starts` → `ir::bind_table` → `StoredFrameStart`. 중첩 비병합 행의 컷은 `FlowBlock::has_stored_frame_cut`으로 조상에게 전달한다. |
| 병합 소유자 앞부분 | `fit_mixed_rows` → `fit_row_groups`의 `row_cut_required`. 이미 소비한 병합 내용은 계속 원자적이며 내용을 새로 분할하지 않는다. 뒤 행을 같은 조각에 붙이는 중간 경계는 페이지 끝으로 늘이지 않는다. |
| 비병합 셀/자식 | 같은 `fit_plain_rows` → `FlowCursor::fit_cell_until` → 자식 `fit_in_frame`. host·패딩·물리 최소 밴드를 포함한 요구 높이와 확정 컷을 그대로 소비한다. |
| 예산 실패 | 비병합 첫 유닛이 안 맞으면 해당 컷을 소비하지 않는다. 앞 병합 행만 수용 가능하면 그 조각을 종료하고 다음 페이지에서 재시도한다. Never/제목 원자 구간은 기존 경로를 유지한다. |
| 물리 이어받기 | 이미 내용이 소비된 incoming rowspan의 외곽은 실제 수용한 행 조각의 끝까지 이어진다. 서로 다른 종료 행의 span을 다른 span의 높이로 늘이지 않는다. 같은 페이지 안의 내부 경로 전환은 같은 셀 외곽 하나로 합친다. |
| 최종 배치 | 완성된 조각 높이에서 첫 병합 셀의 세로 정렬을 다시 query하고 동일 `CellPlacement`의 lines/tables/origin을 paint가 사용한다. 이어받은 병합 셀에는 내용을 다시 넣지 않는다. |

단순 행 경계의 rowspan 이어받기와 **저장 셀 내부 컷**을 구분한다. 초기 후보는
후자도 페이지 바닥까지 외곽을 늘였으나26쪽 직접 overlay 및 PDF 하단 좌표와
불일치했다. 셀 내부 컷은 실제 수용 줄·패딩 envelope에서 닫도록 수정했고,
기존 rowbreak-span 대조군의 물리 밴드 계약은 유지한다.

정식 신규 `issue_7353_mixed_row_frames`6건은 정상 저장본28쪽 종료,26쪽3줄/
27쪽1줄(`이자율`), 독립 PDF26쪽 하단,27/28쪽3x12·5x4 자식 표, 내부 원문
단위 전부1회/순서 보존, 부모 외곽·본문 경계 및 후속p173 빈 문단을 검사한다.
작은 합성 계약은 원래 작은 높이만 들어가는39px와 실제 첫 줄까지 들어가는45px,
컷 재시도·Never, 동일/상이한 행에서 종료되는 여러span, 후속 새span 및 같은
페이지의 외곽 중복을 검사한다. 합성 기대값은10/20px 줄 점유와 셀 소유 관계이며
한컴 일치 주장이 아니다. 입력/PDF provenance와 독립 좌표는
`tests/fixtures/issue7353/mixed-row-frames/README.md`에 기록했다.

후속 span 시작 경계를 추가 검사하던 중 같은 incoming 셀을 한 페이지에 두 노드로
발행하는 후보 결함을 발견했다. 첫 WASM 빌드는 중단하고, 내부 경로 경계를
실제 페이지 경계로 오인하지 않도록 동일 소유 셀을 합쳤다. 해당 반례6번째 검사는
수정 전2개/수정 후1개의 실제 셀을 확인한다. 중단 빌드를 검증 성공으로 세지 않는다.

Native26~28쪽 review와26쪽 overlay 직접 판독에서 줄 이어받기·중첩 표·뒤 문단을
확인했다. 폰트 외형/농도·프린터 배율의 미세 차이는 남는다. 전체 원본/모든
rowspan 셀 내부 분할/R5 완료가 아니며, 최종 회귀·fresh WASM 결과는 아래 이어 기록한다.

최종 Native 검증:

- 위6건을 포함한16 focused suite 총248PASS. Native/WASM **lib** Clippy,
  fmt check 및 diff check PASS. 전체 CI/워크스페이스 검증은 수행하지 않았다.
- 최종 정식 검사 소스로 보존된 직전 라이브러리 실행:5FAIL/1PASS. 정상본의
 28쪽 실패와 비병합 셀 컷 누락이 재현된다(`tests-before.log`, `before.log`).
  수정 전 부분 출력27쪽도 `before/native-review-27.png`로 직접 확인했다.
- 수정 전/후 Native JSON 비교:1~25쪽 동일, 기존 쪽 중 변경은26/27쪽이며
  수정 후28쪽까지 종료한다(`before-after-pages.json`).
- 28쪽 부모 표 하단은 Native386.2933px, PDF384.6987px로 약1.59px 차이가
  남는다. 전부 폰트/프린터 문제라고 단정하지 않고 **기하 잔차·판정 보류**로
  남긴다. 신규 검사는26쪽 독립 하단 좌표를 검사하며28쪽의 완전한 외곽 일치를
  주장하지 않는다. 자식 두 표와 뒤 문단은 보존된다.
- 원본 전체 재실행은 여전히p172의 `unqualified stored cell frame reset`에서
  거부한다(`original.log`). 이를 정상 재저장 대조군 통과와 구별한다.

최종 fresh WASM 및 시각 판정 준비:

- `docker compose --env-file .env.docker -p rhwp run --rm wasm` 성공,7분12초
  (`mixed-row-frames/wasm-build-final.log`). WASM SHA256
  `319eb20f39ba2d8bca36d3d62aa393a9d11ac156582e6b831e586e632fc99b90`.
- `node output/7353/r19/mixed-row-frames/capture.mjs --wasm` 성공.
  Native/fresh WASM28쪽 전체 render tree 차이0, SVG28쪽 모두 동일
  (`review/backend-comparison.json`). HEAD50823731af+WIP의 코드 manifest와
  입력·PDF·WASM 해시는 `review/run.json`에 고정했고 빌드 후 source를 재확인했다.
- fresh WASM26/27/28쪽 review 및26/28쪽 standalone overlay 직접 판독:
  26쪽 마지막 셀3줄→27쪽 `이자율`1줄,27쪽 건설공사비지수 표,
  28쪽 건축비 표의 순서·내용·분할 보존을 확인했다. 부모 외곽의 누락·중복은
  발견하지 않았다. 28쪽 부모 하단의 약1.59px 기하 잔차는 앞서 기록한 대로
  보류하며, 폰트 외형/농도 차이와 분리한다. 메인터너 시각 판정은 대기다.
- [26쪽 review](../../output/7353/r19/mixed-row-frames/review/wasm-review-26.png) /
  [overlay](../../output/7353/r19/mixed-row-frames/review/wasm-overlay-26.png)
- [27쪽 review](../../output/7353/r19/mixed-row-frames/review/wasm-review-27.png) /
  [overlay](../../output/7353/r19/mixed-row-frames/review/wasm-overlay-27.png)
- [28쪽 review](../../output/7353/r19/mixed-row-frames/review/wasm-review-28.png) /
  [overlay](../../output/7353/r19/mixed-row-frames/review/wasm-overlay-28.png)
- [정상 저장 HWP](../../tests/fixtures/issue7353/mixed-row-frames/prefix174-saved.hwp) /
  [동일 입력 한컴 PDF](../../tests/fixtures/issue7353/mixed-row-frames/prefix174-2020.pdf).

기본 Legacy 경로 전환·commit/push는 수행하지 않았다. 전체 원본·전체CI·R5 완료를
주장하지 않으며, 이번 제출 범위는 정상 저장 대조군의 혼합 행/중첩 표 분할이다.

### 후속 절편: 초기 셀 단 정의와 중첩 누름틀

메인터너가 앞선26~28쪽 시각 판정을 통과시키고 다음 절편을 승인했다.
원본 p172의 비영점 저장 프레임 reset을 정상화하는 조건부터 추가하지 않고,
원본 직접 한컴2024 PDF와 정상 재저장본의 입력·출력을 분리해 확인했다.
원본은 `unqualified stored cell frame reset` 거부를 유지한다. 원본의 p21
vpos9213과 재저장본100은 다르며, 이 차이를 임의로 페이지 시작0으로 치환하지 않았다.
원본 직접 PDF job은 `9b7241cb-b7f9-4172-9940-a9378e4f44ee`다.

정상 전체 재저장본은 다음 p222에서 `non-table cell control`로 막혔다.
첫 셀 문단의 ColumnDef 하나와 중첩 ClickHere 두 개가 원인이었다. 이전 필드
검사는 제어 전체를 필드로 요구하고 중첩 범위를 거부했다. 입력은 수동 캐시가
아닌 정상 한컴 저장본이며, 필드 범위 `[0,183)`/`[3,182)`와 source slot을
파서 정의대로 보존하면 저장6줄을 기존 공통 배치로 전달할 수 있다.

`table_v2/fields.rs`에서 초기 단 슬롯과 필드 슬롯을 구분하고, 잘 구성된 중첩
범위만 허용했다. 각 begin/end는8 source unit이며 추가 줄·공백을 만들지 않는다.
교차 범위·중첩 수 불일치·누락/중복 슬롯·다단·안내문·편집 후 무효 캐시는 거부한다.
문서 ID·페이지 수 조건은 없고, 명령 실행이나 Legacy 변경도 없다.

실제 소비 경로:

- `ir.rs:initial_cell_column`/`bind_table`이 셀 첫 문단의 단 정의를 검증하고
  `fields::stored_result`가 범위와 정확한 문자 오프셋을 검증한다.
- `text_ir.rs:compose_in_cell`→`text.rs:compose`가 동일 저장 줄로 실제
  TextLine payload와 ParagraphItem 높이/advance를 함께 만든다.
- 기존 분할 흐름은 이 줄 유닛을 소비하고 `TextPaint::build_node`가 같은
  payload를 배치 원점으로 이동한다. 필드용 별도 높이·앵커·clamp는 추가하지 않았다.
- 이번 변경은 줄 수용 자격이며 pagination/rowspan/컷 계산 변경은 없다.
  80px 합성 예산으로 모든 저장 줄이 실제 분할을 거쳐 한 번씩 셀 안에 배치되는지
  검사했다. 필드 편집 후 재조판과 문단 사이 범위는 미구현/미검증으로 남긴다.

증적은 `output/7353/r19/child-frame-tail/`이다. 정상 대조군 생성·해시·MCP job과
독립 PDF 기준선은 `tests/fixtures/issue7353/nested-field/README.md`에 기록했다.
정식 신규 `issue_7353_nested_field`3건은 저장6줄/문자 순서와 보존, 실제 셀 범위,
PDF36쪽 기준선, 제한 예산 분할, 잘못된 범위 거부를 검사한다.
동일 검사 소스를 직전 라이브러리에서 실행하면2FAIL/1PASS이며 예상한 미지원
제어 오류로 실패한다(`tests-before.log`). 수정 후3PASS이며 독립 PDF 좌표 검사를
추가한 최종 검사도3PASS다(`tests-final.log`).

기존 필드·저장 프레임·단 정의·문서·텍스트·중첩 표 대조군 포함11 suite 총204PASS
(`tests-all.log`, 신규3건 최종 재검증은 중복 합산하지 않음).
정상 전체 재저장본은 Native64쪽까지 종료했고 동일 입력 한컴 PDF도64쪽이다.
이는 원본 통과 또는64쪽 전체 시각 일치 판정이 아니다. 직접 판독한36쪽에서는
중첩 필드의6줄/줄바꿈을,64쪽에서는 마지막 표·내용·종료를 확인했다.
폰트 외형·농도 차이가 남는다. fresh WASM 대조와 최종 검증은 아래 이어 기록한다.

최종 검증/시각 판정 준비:

- Native/WASM **lib** Clippy, fmt 및 diff check PASS. 신규 검사도 rustfmt 및
  직접 rustc `-D warnings`로 검증했다. 전체 CI/통합 suite 등록 검사는 미실행이다.
- Docker WASM 빌드 성공(7분18초). SHA256:
  `b2772a4e82341b9c7964715b9936373ea5bd5e2db3eacf1208cedd695c3cc051`.
- `node output/7353/r19/child-frame-tail/capture.mjs --full --wasm` 및
  `--wasm`으로 전체본64쪽/대조군36쪽을 검증했다. 각 Native/fresh WASM
  render tree 차이0, SVG 전부 동일이다. `full-review/run.json` 및
  `review/run.json`에 HEAD50823731af+WIP source manifest와 입력/PDF/WASM 해시를
  고정했다. 빌드 후 source manifest 재검사 PASS.
- 정상 전체본의 한컴 PDF job `4baee058-752d-4897-a246-908928bed006`,
  SHA256 `b8c0a434ff2672c7df2cf77bd5dd1b1a1224250b37d1a62344ec265dd933cb4f`.
- fresh WASM36쪽 review/standalone overlay 직접 판독: 주민대표단 문단의
  6줄과 뒤 `3. 규제목표` 문단 배치 보존.64쪽 review에서는 마지막 표/내용과
  문서 종료를 확인했다.28쪽 대조에서도 자식 표·뒤 표가 보존되며, 앞 절편에서
  기록한 부모 하단의 작은 기하 잔차는 남는다. 전체64쪽 시각 판정은 미실행이다.
- 원본 현재 코드 재실행(`original-after.log`)은 여전히p172의 저장 프레임
  reset에서 거부한다. 정상 재저장본의64쪽 성공으로 원본 통과를 대체하지 않는다.
- [36쪽 review](../../output/7353/r19/child-frame-tail/full-review/wasm-review-36.png) /
  [overlay](../../output/7353/r19/child-frame-tail/full-review/wasm-overlay-36.png)
- [64쪽 review](../../output/7353/r19/child-frame-tail/full-review/wasm-review-64.png)
- [정상 전체 저장 HWP](../../output/7353/r19/child-frame-tail/full-saved.hwp) /
  [동일 입력 한컴 PDF](../../output/7353/r19/child-frame-tail/full-2024.pdf).

이번 절편의 메인터너 시각 판정은 대기다. 기본 Legacy 변경·commit/push는 없으며,
원본 미지원 경계와 전체 통합 검증이 남아 있어 R5 완료로 보고하지 않는다.

### 다음 절편 조사: 원본의 자식 표 뒤 저장 프레임

메인터너가 중첩 누름틀 절편의36/64쪽 시각 판정을 통과시켰다. 공공기관의
웹기안기에서는 누름틀 범위에 다른 문서 조각을 삽입할 수 있다는 도메인 지침도
확인했다. 따라서 필드 경계를 단순 문자열로 평탄화하지 않으며, 문단·표·그림·
중첩 필드의 소유를 보존해야 한다. 앞 절편의 한 문단/저장 텍스트 검증을
문단 간 복합 문서 조각의 지원 증거로 확대하지 않는다.

이번 조사는 원본 p172의 미지원 원인을 좁혔다. 원본·정상 재저장본을 재귀 비교한
`output/7353/r19/frame-after-child/compare.rs`와 `compare.log`를 보존했다.
비교한 문단의 텍스트·char_offsets·field_ranges·char_shapes 참조·문단 모양 ID·
문단/컨트롤 수에는 차이가 없었다. 글자모양은 raw bytes/언어별 폰트 ID를 제외하고
실제 참조 폰트 이름으로 비교했을 때 차이가 없었다. DocInfo 전체는 동일하지 않으며
폰트 목록 추가 등은 `original-doc-info.txt`/`saved-doc-info.txt`에 남겼다.
이를 모든 문서 속성이 같다는 주장으로 확대하지 않는다.

차이는 reset 한 곳에만 있지 않았다. 경로는
`s0/p172/t0/c80/p0/t1/c0`이다.

| 항목 | 원본 저장값 | 한컴 정상 재저장값 |
| --- | --- | --- |
| p6 줄 수 | 4 | 3 |
| p8 줄 수 | 5 | 4 |
| p10 줄 수 | 2 | 1 |
| p15 줄 수 | 2 | 1 |
| p20 자식 표 호스트 vpos | 66300 | 59100 |
| p20 자식 표 common.height | 1463 | 10212 |
| p21 자료출처 문단 vpos | 9213 | 100 |

줄바꿈 text_start도 p3부터 달라진다. 누적4줄×1800HU=7200HU가 자식 표 시작
차이에 연결된다. 따라서 `9213→100` 보정만으로 현재 한컴 출력의 재조판을
대신할 수 없다. 이것만으로 원본 캐시가 불법이라고 단정하지도 않는다. 원래
작성 환경의 저장 줄 구성과 현재 한컴의 재계산 결과를 분리해야 한다.

진단용 복제 IR에만 두 반사실 실험을 수행했다(`probe.rs`). 원본은 변경하지 않았다.

- `intact`: 무수정 HWPX 직렬화 대조도 p172의 동일 미지원 오류.
- `carry`: p21 이후 vpos에70000HU를 더해 reset을 없앤 변형은27쪽 출력 뒤
  `DoesNotFit { page:27, required_height:1358.64 }`로 중단.
- `reset`: p21 이후 vpos에서9113HU를 뺀 변형은65쪽 종료. 독립 기준 PDF64쪽과
  다르며, 좌표를 바꾼 합성 결과이므로 원본 지원/수정 성공 증거가 아니다.

위 변형은 다른 소비 지점의 좌표 차이에도 영향을 줄 수 있다. 따라서 carry 실패를
단순히 StoredFrameStart 제거의 결과라고 해석하지 않는다. 원본의 비영점 reset
수용 규칙을 만들 근거로도 사용하지 않는다.

독립 대조군 후보도 생성했다. 정상 전체본에서 본문p172~173을 분리하고 용지 높이를
2500HU 줄인 `carrier.hwpx`를 한컴2024로 정상 저장했다(job
`59e949a4-19e4-4079-b00c-bfe60ac1b31d`). 캐시 값을 직접 수정하지 않았다.
그러나 이 후보의 reset은 p2/p23의100HU이며, 목표인 자식 뒤 비영점 잔여 높이를
재현하지 않았다. 또한 현재 V2는 별도의 저장 단일 줄 자격 검사에서 거부한다.
이를 목표 반례/시각 판정 자료로 제출하지 않는다.

판정: 원본의 reset 단독 완화는 근거 부족으로 채택하지 않았다. 이번 절편에서는
production Rust·WASM·baseline을 변경하지 않았고, 앞 절편 검증을 새 검증으로
합산하지 않는다. 다음 구현 전 해결할 계약은 **저장 줄 구성 재사용과 현재 환경
재조판의 구분**, 그리고 **자식 표가 이미 넘긴 프레임과 후속 문단의 프레임 소유**다.
원본 저장 줄을 유지해야 하는 경로에 정상 재저장본의 줄 수/좌표를 강제하지 않는다.

### 후속 조사/후보 검증: 자식이 소유하는 프레임의 정상 재현

다음 절편 승인에 따라 위 미재현 경계를 정상 한컴 저장본에서 재현했다.
`frame-after-child/normal_candidates.rs`는 정상 전체본의p172~173을 분리하고
앞 빈 문단과 용지 높이만 바꾼다. 74000/76000/78000HU 후보를 각각 한컴2024로
저장했다. 78000HU에서는 자식 호스트v59100 뒤 자료출처v4448이 저장된다.
직접 대응 PDF3쪽의2쪽은 자식0/1행,3쪽은2행·자료출처·뒤 표다.
입력/PDF와 생성 job·해시는 `tests/fixtures/issue7353/child-frame-tail/README.md`에
보존했다. 기준 캐시를 수동 수정한 앞선 반사실 실험과 구분한다.

검토한 생산→소비 경로는 `stored_text::cell_frame_starts`의 reset 분류 →
`ir::bind_table`의 공통 자식 plan → `FlowBlock::has_stored_frame_cut` →
부모 `fit_mixed_rows`/`fit_plain_rows` → `FlowCursor::fit_cell_until`의 여백 예약 →
자식 `TableCursor::fit_in_frame`의 컷과 최종 TablePlacement다.

후보는 비영점 감소를 무조건 수용하지 않고, 완성된 자식 plan의 유일한 행 suffix와
바깥여백이 후속 저장 원점에 대응할 때만 자식 행 경계로 소유를 옮겼다.
반복 제목·rowspan·내부 저장 컷이 있는 자식은 확장하지 않았다. 후보가 따로
준비한 높이나 Legacy 측정을 paint에 재사용하지는 않는다. 그러나 아래 실제
실행에서 실패했으므로 이 방식의 일반성·완료를 확정하지 않는다.

| 실행 | 관측 |
| --- | --- |
| 직전 코드 + 정상78000HU 저장본 | 동일 `unqualified stored cell frame reset` |
| 후보 코드 + 같은 입력 |4쪽 종료, 한컴3쪽과 다른 자식 행 소유 |
| 직전 코드 +76000HU 저장 대조군 |3쪽 종료. 시각 일치 검증은 아님 |
| 직전 코드 +74000HU 저장 대조군 |별도 단일 저장 줄 자격에서 미지원 |

`trace-78000.log`에서 자식 최초 예산81.12px인데 처음 두 행의 공통 요구 높이는
20.8667+63.5333=84.4px다. 따라서0행만2쪽에 수용하고1행을3쪽으로 넘긴 뒤,
새로 설정한 저장 컷 때문에2행을4쪽으로 넘겼다. 실제 Native2/3/4쪽과
PDF2/3쪽을 직접 열어 판독했으며, 페이지 숫자만으로 실패를 분류하지 않았다.

추가 원인: IR에 선언된1행 높이4765HU와 PDF에서 관측한 약4207HU가 다르다.
PDF의 가로 경계109.521→67.453pt는42.068pt다. 단순 여백 중복이라고 아직
확정할 수 없다. 저장 프레임 소유뿐 아니라 분할 조각에서 선언 높이/물리 밴드를
어떻게 소비하는지 함께 확인해야 한다. 마지막 행의 공통 높이3882HU+바깥여백566HU는
저장4448HU와 대응하지만 이 수치 일치만으로 앞 조각의 정확성을 보장하지 못한다.
원본의9213HU도 공통 suffix8647HU+566HU와 대응하며, 이것만으로 원본 조판이
정상이라는 결론을 내리지 않는다.

후보는 `output/7353/r19/frame-after-child/candidate.patch`와 `candidate-source/`에
진단 trace를 포함해 분리 보존했다. 이번에 추가한 제품 코드만 되돌렸고 이전 WIP는
보존했다. 직전 `child-frame-tail/source.sha256`와 제품 source 전체 일치 확인.
기존 WASM은 변경하지 않았으며 후보의 WASM/전체CI는 실행하지 않았다.
비교 캡처 도구는 예상하지 못한4번째 쪽의 기준PNG 부재로 종료했다. Native1~4쪽과
1~3쪽 비교 PNG는 진단 자료이지 성공한 sweep/판정 요청 자료가 아니다.

회귀 계약 초안은 `frame-after-child/pending_contract.rs`로 보존했다. 수정 전 라이브러리에서
미지원 오류로1FAIL, 후보에서4/3쪽 불일치로1FAIL이며 뒤의 좌표 assertion까지 통과한
것이 아니다. 수정 후 PASS 계약이나 formal suite 통과로 보고하지 않는다.
다음 대상은 위 정상 입력의 자식1행 높이 차이와 실제 물리 높이 소비 규칙이다.

### 2026-09-27 후속: 선언 높이 소비와 분할 정책의 반대 매핑 확인

승인 범위의 원인 추적을 수행했다. 제품 소스는 직전 승인 WIP 그대로 유지했다.
`child-frame-tail/source.sha256` 전 항목 일치를 확인했으며, 새 WASM/전체CI를
실행하거나 과거 시각 판정을 이번 미해결 원본의 통과로 바꾸지 않았다.

**4,765HU 생산→소비 경로**: `parser/control.rs:375`의 LIST_HEADER 셀 높이 →
`table_v2/ir.rs:614`의 `minimum_height` → `content.rs`의
`max(physical, minimum_height)` 공통 행 높이 → `fragment.rs::fit_plain_rows`의
비분할 행 사전 fit → `minimum_left` 예약/차감 → `CellPlacement.bounds`.
따라서 이 숫자는 V2가 만든 보정값도 패딩을 한 번 더 더한 값도 아니다.
문제는 어떤 분할 정책에서 이 온전한 높이를 요구하고, 내용/물리 공간을 어디서
이월/종료하느냐에 있다.

용지 높이만 다른 정상 한컴 저장/PDF 대조군을 추가했다. 모든 PDF는3쪽이지만
내용과 행 경계가 다르다. `frame-after-child/trace_rows.mjs`는 PDF clip/가로 경계의
pt 차이를 HU로 환산한다. 약12HU 수준의 PDF 출력 격자 차이는 정수 원본값과
구분하며 이를 구현 허용치로 사용하지 않는다.

| 용지HU | child common.height | 1행(0기준) PDF 높이 | 마지막 행의 내용 소유 |
| --- | --- | --- | --- |
| 76000 |10212 |4757.9HU |표 전체가3쪽 |
| 78000 |5781 |4206.8HU |마지막 행 전체가3쪽 |
| 79000 |6330 |4764.2HU |마지막 행 전체가3쪽 |
| 80000 |9681 |4769.8HU |`년간`/`상승률`은2쪽, `(%)`는3쪽 |

80000HU의 마지막 행 PDF 조각은3343.7HU/1282.4HU다. 이 입력의 저장 셀 높이는
3448HU이므로 선언 높이가 모든 분할 조각 높이의 합과 같다는 가정도 성립하지 않는다.
78000HU의 값만 보고1행 높이를 전역 축소하거나 common.height를 전체 높이로
재사용하면 위 정상 대조군을 깨뜨린다. 정확한 물리 밴드 처리 계약은 아직 미완료다.

더 근본적인 오류를 확인했다. 독립 기준은
[한컴 공식 도움말](https://help.hancom.com/hoffice130_assistant/ko-KR/Hwp/table/tableattribute/table%28table%29.htm)과
[PageBreak 값에 대한 개발자 답변](https://forum.developer.hancom.com/t/topic/2004)이며,
동일 입력의 목표 표 분할 속성 한 곳만 바꾼 정상 저장/PDF 쌍으로 확인했다.

| HWP 값 / IR 이름 | 독립 규칙 | 현재 V2 매핑 |
| --- | --- | --- |
| 2 / RowBreak |나눔: 셀 안 줄 단위 분할 |BetweenRows: 행 전체 fit 요구 |
| 1 / CellBreak |셀 단위로 나눔: 셀 전체 이월 |WithinCells: 셀 내부 분할 허용 |
| 0 / None |나누지 않음 |Never |

파서의 숫자 보존 문제가 아니라 IR 의미를 분할 정책으로 연결하는 해석 문제다.
`model/table.rs`의 주석도 반대 의미로 설명한다. 기존 local 스펙 Markdown의 값2는
‘나누지 않음’으로 중복되어 있어 그것만으로 의미를 확정하지 않았다. 공식 도움말,
API 설명, 실제 한컴 출력이 일치하는 의미를 기준으로 삼았다.

정상 쌍은 `tests/fixtures/issue7353/child-frame-tail/`의 `line-split-*`,
`whole-cell-*`에 보존했다. 생성/해시/job은 같은 README에 기록했다.
HWPX 입력diff는 목표 표의 `pageBreak=CELL/TABLE` 한 곳뿐이다.
전자는 PDF2쪽의 `년간`/`상승률` 뒤3쪽의 `(%)`로 이어지고, 후자는 같은 셀 전체와
수치 행이3쪽에 모인다. 해당 PDF PNG2/3쪽을 직접 열어 확인했다.

현재 라이브러리로 `frame-after-child/policy_contract.rs`를 rustc --test 실행:
**1PASS / 2FAIL** (`policy-before.log`). 두 실패는 실제 fit 결과가 독립 규칙과
반대이기 때문이며 빌드 실패가 아니다. 뒤의 좌표/유닛 보존 assertion은 아직
실행되지 않았으므로 그 부분은 미검증이다. 이 RED 초안은 정식 성공 회귀에
등록하지 않았고, 수정 후 PASS도 주장하지 않는다.

수정은 `ir.rs` 두 줄만 뒤집고 끝낼 수 없다. 같은 의미의 소비 위치는
`text_ir.rs:268,282` 대각선/셀 배경 수용, `zones.rs:62` 영역 대각선,
`flow.rs:221` continuation 바깥여백 예약, `fragment.rs` Center/Bottom 분할 및
`fragment/row_groups.rs` rowspan 컷, `document_input.rs:476` 정상 저장 프레임
clearance다. 특히 기존 BetweenRows 경로에 붙은 저장 컷 지원을 올바른 줄 단위
정책에서도 공통 소비하도록 정리해야 한다. 원본의 비영점 reset은 여전히 별도
미지원이며 정책 매핑 오류 하나가 모든 증상을 설명한다고 주장하지 않는다.

다음 구현 우선순위는 (1) 독립 규칙 기반 정책 어댑터와 소비 분기 정리,
(2) 동일한 줄/객체 소유 컷을 사용하는 자식 continuation과 부모 후속 원점,
(3) 위4개 예산 대조군에서 물리 조각·정렬·내용 보존이다. 이전 suffix 행 수치
일치 후보에 예외를 추가하는 방향은 채택하지 않는다. 제품 변경/수정 후 PASS/
fresh WASM 시각 판정은 아직 남아 있다.

### 2026-09-27 후속: 분할 정책 어댑터와 실제 조각 소비 경계 수정

사용자의 다음 절편 승인으로 위 정책 해석 오류를 수정했다. 작업 기준은
`50823731af6050c60ec3cfcc898abf36daa6b35a` + 기존 stage19 WIP다. Legacy 엔진,
파서의 숫자 보존, 기본 엔진 선택은 바꾸지 않았다. 아래 결과는 R5 전체 완료나
`carrier-saved.hwp`의 미해결 reset/물리 밴드 조판 통과를 뜻하지 않는다.

구현과 실제 소비 경로:

- `table_v2/ir.rs::From<TablePageBreak>`에서 raw2/`RowBreak`는 `WithinCells`,
  raw1/`CellBreak`는 `BetweenRows`, raw0은 `Never`로 단일 변환한다.
  `bind_table` → 공통 `TableContentPlan` → `fragment::fit_plain_rows` /
  `fragment/row_groups::fit_mixed_rows` → `CellPlacement` → `text::append_cells`
  순서로 같은 정책과 수용 조각을 소비한다. 역사적인 enum 이름은 유지하고
  `model/table.rs`의 반대 설명만 정정했다.
- 분할 **허용**만으로 모든 Center/Bottom·rowspan 셀을 준비 단계에서 거부하지 않는다.
  온전한 행은 기존 정렬 높이로 배치하며, 실제 내부 컷에서 정렬 내용이 불완전하면
  해당 prefix를 소비하지 않는다. rowspan 내부 내용의 일반 분할은 여전히 미구현이다.
  새 제목 셀 전체 분할·일반 rowspan 내부 컷 지원을 주장하지 않는다.
- 셀 대각선·그라데이션의 수용은 준비 시 정책 이름이 아니라 최종 `cell.partial`에
  따라 판단한다. `text.rs`의 실제 partial 검사와 `zones.rs::bounds`의 partial
  검사로 미지원 조각을 paint 전에 거부한다. 온전한 셀은 정책과 무관하게 기존
  장식을 보존한다. 원래 미통과였던 긴 대각선 셀을 성공으로 바꾸지 않았다.
- `flow.rs`의 AnchoredTable은 줄/셀 정책 모두 동일한 하단 바깥여백을
  `child_budget`에서 먼저 예약한다. 그 예산으로 수용한 자식 조각 높이와 후속
  원점이 함께 전진한다. 실제 소스의 첫 줄1200HU와 위/아래 여백283HU씩은
  1766HU(약23.5467px)를 요구한다. 기존 합성20px 예산은 앞 문단만 소비하고
  자식은 이월해야 하며, 이를 검사하도록 계약을 바로잡았다.
- 정상 #6923에서 온전한13행 자식의 마지막 minimum이 뺄셈 반올림으로
  `1.7763568394002505e-14px` 남는 후속 오류를 검출했다.
  `fit_mixed_rows`의 후보 선택과 `fit_plain_rows`의 온전한 행/물리 최소 높이
  수용은 측정과 같은 prefix+height 누적합을 사용하도록 수정했다. 허용 오차나
  좌표 clamp를 추가하지 않았다. 원본 전체의 빈 문단/프레임 계약과1HU 부족
  대조 검사가 다시 통과한다.

기대값은 앞 절의 공식 도움말·PageBreak API·정상 한컴 저장/PDF 쌍에서 구한
정책 의미, 원본 저장 메트릭, 명시적인 composer 컷이다. 기존 합성 테스트 중
내부 줄 분할을 `CellBreak`로, 셀 전체 분할을 `RowBreak`로 지정했던 입력은
그 의도에 맞는 속성으로 정정했다. 정상 HWP/PDF fixture와 좌표 golden은
변경하지 않았다. 저장 컷 뒤에 여유 공간이 있어도 다음 줄의 소유를 앞쪽으로
합치지 않는 반례도 검사한다. 일반 줄/셀/여백의 기대값을 페이지수로 조정하지 않았다.

별도로 기존 장식 negative-control이 이미 지원된 `Dash`를 미지원으로 기대하는
실패를 발견했다. 변경 전 라이브러리+HEAD의 원래 테스트에서도 동일한 `case 0`
실패를 확인했다(`policy-export-before.log`). 아직 미지원인 `Dot`으로 해당
negative-control을 교체했고, 이미 있는 `dash_fragment_edges`와 두 DPI pen
catalog 계약은 그대로 통과한다. 제품의 장식 지원 범위를 넓힌 수정은 아니다.

검증 자료는 `output/7353/r19/frame-after-child/`에 모았다.

| 검사 | 결과 / 증거 |
| --- | --- |
| 정식 `issue_7353_table_v2_split_policy.rs` | 변경 전1PASS/2FAIL → 변경 후3PASS. `policy-before-formal.log`, `final-formatted-issue_7353_table_v2_split_policy.log`. 좌표와 줄 owner의 단일 소비까지 검사 |
| 관련25개 `tests/cases` | 총389PASS/0FAIL. `policy-final-summary.log`의 초기 확대 결과에 `policy-final-contracts.log`와 `policy-split-property.log`의 최종 해당 suite 결과를 대체 적용 |
| Native lib build / Clippy | `policy-formatted-build.log`, `policy-clippy.log` PASS (`cargo clippy --locked -p rhwp --lib --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings`) |
| fmt / diff | `cargo fmt --all -- --check`, `git diff --check` PASS |
| Docker fresh WASM | `policy-wasm-build.log`, 약7분26초, SHA256 `dd67a55ebc5fa8b5a7a0b03dea2cf5ee75d950974926dd3f46206ada59e94507` |
| 정상 저장 Native 전후 | `rowbreak-span/prefix14-saved.hwp` 전체3쪽 JSON/SVG가 변경 전후 동일. `policy-review/before/native.json` / `policy-review/actual/native.json` |

명령은 `run-policies.sh <phase> <suite...>`로 각 정식 원본을 같은 debug rlib에
`rustc --test -D warnings` 연결했다. zip 사용 suite는 `policy-split-property`로
명시적인 zip extern을 추가해 검사했다. 이는 선택 범위 검증이며 생성 integration
전체 suite·전체 CI·workspace all-target/WASM Clippy 완료 주장과 구분한다.
빌드 중 시작한 `consumed-policy-*` 결과는 구/신 산출 혼재 가능성 때문에 폐기했고,
빌드 성공 후 재실행한 결과만 사용했다. 진단 `POLICY_DIAG` 출력은 제품 코드에서
모두 제거했다.

Native 시각 대조는 정상 `prefix14-saved.hwp`와 대응 `prefix14-2020.pdf`를
같은2/3쪽에서 직접 열어 확인했다. 표의 row9→10 컷, 병합 셀 제목의 단일 소비,
뒤 셀 내용과 외곽은 유지된다. 기존 대체 글꼴 외형/약2px 수준 외곽선 차이는
남아 있다. 대표 자료: `policy-review/native-review-2.png`,
`native-review-3.png`, 각각의 `native-overlay-*.png`.
기존 캡처를 재사용하지 않고 이번 Native 산출로 다시 생성했다.

소스 고정은 `policy-source.sha256`, 입력/기준 PDF/명령·backend 정보는
`policy-review/run.json` 및 `policy-review.mjs`에 기록한다. 이 정상 대조군은
정책 수정의 **무회귀 대조**이며, 미해결 carrier의 시각 개선을 대신하는 자료가 아니다.
원본 비영점 reset, 자식 분할 후 후속 문단 원점, 76000/78000/79000/80000HU
대조군의 실제 물리 밴드 소비는 다음 구현 범위로 남는다.

fresh WASM 후 `node output/7353/r19/frame-after-child/policy-review.mjs --wasm`
완료. `backend-comparison.json`에서3쪽 전체 Native/WASM JSON의 수치·비수치
차이0, SVG3/3동일을 확인했다. `wasm-review-2.png`와 독립
`wasm-overlay-3.png`도 직접 열어 컷·제목 중복 없음·후속 셀과 외곽을 판독했다.
이번 WASM 결과에서도 기존 글꼴/작은 외곽 차이는 남아 있으며, 미해결 carrier에
대한 새 시각 판정 요청이나 통과 선언은 하지 않는다.

### 2026-09-27 후속: 온전한 행 경계의 자식 표 이어받기

직전 정책 수정에 이어 비영점 저장 원점 감소를 자식 표의 소유 프레임으로
연결했다. Legacy 기본값과 기존 WIP는 유지하며 commit/push는 하지 않았다.

이번 지원 범위는 첫 조각과 마지막 조각이 모두 **온전한 행의 집합**인 정상
저장 입력이다. `whole-cell-saved.hwp`(80000HU/셀 단위 분할),
`whole-row-saved.hwp`(79000HU/줄 분할 허용)는 정책이 달라도 실제 저장 컷이
0·1행→2행이다. 첫 조각6330HU와 후속 문단 원점4448HU를 양쪽에서 대조하면
동일한 유일한 경계가 나온다. 이 조건으로 지원 범위를 한정한 이유는 임의의
vpos 감소로 줄이나 셀의 소유 페이지를 추정하지 않기 위해서다.

반대로 `carrier-saved.hwp`(78000HU)는 첫 조각5781HU가 온전한 행의 합과
다르다. 이 경우에는 `unqualified stored child frame geometry`로 명시적으로
거부한다. 셀 내부 컷을 가진 `line-split-saved.hwp`도 아직 미해결이다.
따라서 원본 전체 문서 또는 모든 파생 대조군을 해결한 절편이 아니다.

#### 생산 결과와 실제 소비 경로

- `stored_text.rs::cell_frames`는 뒤 문단의 감소를 직전 excluded host에 연결한
  후보로 분리한다. fresh/dirty 입력에 저장 컷을 부여하지 않는다.
- `ir.rs::bind_table`의 ExcludedTable 분기에서 `stored_child.rs::qualify`가
  정상 저장된 자식의 첫 높이와 마지막 행+바깥여백을 동일한 composed plan에
  대조한다. 반복 제목, rowspan, 내부 컷, 자식 내부 컨트롤은 이 계약의 비적용
  경로다. 선언 높이를 clamp하거나 행 높이를 줄이는 처리는 없다.
- `TableContentPlan::stored_row_starts`는 자식 안의 컷이다. 부모의 뒤 문단 앞에
  StoredFrameStart를 삽입하지 않는다. `FlowBlock::has_stored_frame_cut`를 통해
  조상에 프레임 존재만 전달한다.
- `fragment.rs::fit_plain_rows`는 해당 경계에서 수용한 조각을 끝내며 실제
  `reserved_height`와 continuation을 반환한다. `starts_stored_frame`는 해당
  행이 미소비 상태일 때만 참이다. 예산 부족은 기존 fit/DoesNotFit 경로를 따른다.
- `flow.rs` AnchoredTable의 `fit_in_frame`→reserved_height→anchor_tail 경로가
  동일 조각의 점유와 여백을 소비한다. 조상 셀 inset은 terminal_inset에서
  예약한다. 마지막 조각 뒤 바깥여백을 소비한 후 다음 문단의100HU 앞 간격을
  적용한다. paint에는 별도의 원점 재계산이나 높이 덮어쓰기를 추가하지 않았다.

#### 계약과 확인 결과

`tests/cases/issue_7353_stored_child_tail.rs`에 정상 저장2종의 양성 계약과
짧아진 첫 조각의 음성 계약을 추가했다. 양성 계약은3쪽 종료,2/3쪽 행 소속,
각 셀 전체 문자열의 단일 보존, 글줄의 셀 내부 배치, 자식 마지막 행3882HU,
후속 문단과5×4표의 순서 및 본문 영역 내부를 검사한다.3882HU는 원문3개 줄
1000HU·두 줄 사이 간격300HU·셀 위아래 여백141HU에서 독립적으로 정했다.
후속 문단 간격 기대값에 처음100HU를 빠뜨린 오류를 원문 문단모양 확인 후
바로잡았으며 실제 좌표를 맞추는 제품 코드 수정은 하지 않았다.

- 이전 정책 수정 직후 바이너리로 같은 입력은 `unqualified stored cell frame reset`
  FAIL (`tail-current-before.log`). 이전 보존 rlib에 정식 계약을 연결한 실행도
  같은 원인으로 FAIL (`tail-contract-before.log`); 두 산출의 시점은 구분한다.
- 새 코드: 관련10개 suite **191PASS/0FAIL** (`tail-final-summary.log`). 그중 새
  계약3건은 셀 전체 내용/좌표 검사를 보강한 뒤 다시3PASS
  (`tail-content-issue_7353_stored_child_tail.log`). ignore/golden/래칫 변경 없음.
- Native build: `tail-final-build.log`. 명령은 동일 공유target에
  `cargo build --locked -p rhwp --lib --target-dir /home/edward/mygithub/rhwp/target/pr-review`.
- Native 2/3쪽 review와 독립 overlay를 직접 확인했다. 목표 자식의 행 분할,
  자료출처와 뒤 표가 보존된다. 대체 글꼴 외형 및 약2~4px의 위치·외곽
  차이가 남아 있으므로 PDF 픽셀 완전 일치라고 보고하지 않는다.

증적은 `output/7353/r19/frame-after-child/tail-review/`, 생성 명령은
`tail-review.mjs`, 현재 source manifest는 `tail-source.sha256`에 있다.
기준 head `50823731af6050c60ec3cfcc898abf36daa6b35a` 위 작업중 변경이며,
입력/PDF 해시는 fixture README 및 run.json으로 연결한다. 이 선택 범위 검증을
전체 CI 또는 R5 완료로 보고하지 않는다. fresh WASM 후 최종 결과를 아래에 추가한다.

최종 확인:

- Native lib Clippy `-D warnings` PASS (`tail-clippy.log`,54.18초),
  `cargo fmt --all -- --check` / `git diff --check` PASS.
- Docker fresh WASM 성공 (`tail-wasm-build.log`,7분13초). SHA256
  `01e052be0627ed07768ae2d169a754b0828a743ab0e2faef8140e5622533a7d6`.
- `node output/7353/r19/frame-after-child/tail-review.mjs --wasm` 및
  같은 명령의 `--whole-row` 실행 성공. 두 정상 입력 모두3쪽이며, 각각 전체3쪽
  Native/WASM JSON 수치·비수치 차이0, SVG3/3동일이다. 각 review 폴더의
  `backend-comparison.json`과 `run.json`에 기록했다.
- `tail-row-review/wasm-review-2.png`, `wasm-review-3.png`와
  `tail-review/wasm-review-2.png`, `wasm-overlay-3.png`를 직접 판독했다.
  온전한 자식 행의 컷과 다음 쪽 후속 문단·표가 유지된다. 글꼴/작은 위치 차이는
  위와 같다. 이후 작업지시자의 “시각 판정 통과입니다” 피드백으로 이번에 제시한
  정상 대조군2/3쪽의 온전한 행 경계 이어받기 시각 판정을 통과로 기록했다.
  대상 증적과 WASM 해시는 위와 동일하며, 이 판정으로 `carrier-*`의 끝 행 높이나
  `line-split-*`의 셀 내부 분할까지 통과 처리하지 않는다.
- 새 계약 양성2건의 이전 보존 rlib 결과는 모두 같은 미지원 원인으로 FAIL이다.
  최종191건 결과 중 새3건은 강화된 전체 셀 문자열/좌표 검사까지 재실행 PASS다.
  전체workspace/all-target/WASM Clippy 및 전체 CI는 이번 선택 검증 범위에 포함하지 않았다.

다음 구현 대상은 `carrier-*`의 짧아지는 페이지 끝 행과 `line-split-*`의 셀 내부
컷이다. 첫 조각과 내용 소유 컷을 분리해 물리 밴드를 소비하는 계약이 필요하며,
현재 온전한 행 경계의 통과를 그 범위의 완료로 확장하지 않는다.

### 2026-09-27 후속: 페이지 끝 행의 저장 물리 높이

이번 절편은 위 두 대상 중 `carrier-*`의 행 경계와 실제 높이를 구현했다.
`line-split-*`의 내부 줄 컷은 서로 다른 조각의 물리 높이·정렬 계약이 필요하므로
미지원 상태를 유지한다. 이 구분을 정식 음성 계약으로 보호한다.

#### 근거와 적용 경로

입력은 fixture README에 기록한 한컴 정상 저장본 그대로이며, 새로운 기준 PDF나
LineSeg 보정본을 만들지 않았다. 첫 조각5781HU, 앞 행1565HU, 다음 쪽 마지막
행3882HU와 다음 문단 원점4448HU를 함께 사용한다. 독립 PDF의 끝 행은 약4207HU
(인쇄 스케일 포함)이며, 저장 첫 조각에서 계산한 물리 밴드는4216HU다.
기존4765HU는 이 저장 조각의 무조건적인 최소 높이가 아니었다.

- 생산: `stored_child::qualify`가 뒤 문단 원점을 만족하는 유일한 행 경계를 찾고,
  첫 조각에서 앞 행들을 제외해 끝 행의 물리 밴드를 확정한다. 모든 셀의 실제 줄·
  패딩이 그 안에 들어가야 한다. 더 큰 임의 밴드나 내용보다 작은 밴드는 거부한다.
- 측정: 부모 bind 전 자식 `row_heights/height`와 각 셀의 `content_offset_y`를
  동일 저장 밴드로 확정한다. 원본 IR·호출자 `minimum_height`는 수정하지 않고
  별도 `stored_row_bands`에 qualified 결과를 보존한다. 일반 재조판 및 직접 구성한
  API 최소 높이 계약에는 이 저장 정보가 없다.
- 수용/컷: `TableCursor::reset_row`는 qualified 밴드를 소비 잔량으로 사용한다.
  `fit_plain_rows`는 같은 행 높이로 예산을 검사하고 같은 offset으로 줄을 배치한다.
  저장 행 경계에서 자식이 멈추고, 다음 쪽에는 마지막 행부터 이어진다. 예산이
  부족하면 기존 DoesNotFit/이월 계약을 따르며 작은 높이를 수용한 뒤 paint에서
  다시 늘리지 않는다. 내부 줄 컷·rowspan·반복 제목은 이번 qualification 비적용이다.
- 누적 예약/최종 배치: 조각 `reserved_height`를 AnchoredTable의 바깥여백 및
  조상 셀 inset과 함께 소비한다. 별도 paint clamp나 좌표 덮어쓰기는 추가하지 않았다.
  마지막 행 뒤 자료출처와 후속5×4표까지 동일3쪽에 남고 EOF에서 종료한다.

#### 실행 검증

- 수정 전 `band-before-issue_7353_stored_child_tail.log`: 기존 정상2건 PASS,
  새 carrier 계약은 `unqualified stored child frame geometry`로 FAIL. 빌드 실패가 아니다.
- 최종 관련10 suite **193PASS/0FAIL** (`band-final-summary.log`). 새 계약은5건이며
  양성3종, 직렬화 정상 대조 뒤 모순된 높이2종 거부, 내부 컷 미지원 경계다.
  첫 밴드가 맞아도 뒤 문단 원점만1HU 바꾼 음성 입력은 거부하는 검사를 추가한 뒤
  새5건을 다시 실행해 통과했다(`band-final-contract-issue_7353_stored_child_tail.log`).
  전체 셀 내용의 단일 보존·셀 내부 좌표·CENTER 대칭·후속 문단/표·3쪽 종료를 검사한다.
  기존 물리 최소 밴드12건·중첩/rowspan/제목 계약도 통과했다. ignore/golden 변경 없음.
- Native 최종 빌드 `band-final-build.log` 성공. Native compare/overlay/review는
  `output/7353/r19/frame-after-child/carrier-band-review/`에 생성했다.
  2/3쪽을 직접 판독해 목표 끝 행 및 다음 쪽 내용 보존을 확인했다. 글꼴 외형과
  작은 기존 위치 차이는 남아 있으며 PDF 픽셀 완전 일치를 주장하지 않는다.

기준 head `50823731af6050c60ec3cfcc898abf36daa6b35a` 위 WIP이다. 최종 소스는
`output/7353/r19/frame-after-child/band-source.sha256`, 입력/PDF 해시는 fixture
README와 review `run.json`에 연결한다. 생성 명령은 `band-review.mjs`다.
이 기록은 선택 범위의 구현/검증이며 전체 CI·R5 완료 또는 메인테이너 시각 통과가 아니다.
Docker fresh WASM과 최종 직접 판독 결과는 아래에 추가한다.

- Native lib Clippy `-D warnings` PASS (`band-clippy.log`,29.04초), Cargo fmt 및
  별도 integration source rustfmt check, `git diff --check` PASS.
- 기존 승인 whole-cell/whole-row의 새 Native JSON은 이전 `tail-review/actual` 및
  `tail-row-review/actual` 산출과 각각 `cmp`로 바이트 동일함을 확인했다.
- Docker fresh WASM 성공 (`band-wasm-build.log`,7분14초). SHA256
  `bead6c215e71ef2544d39ab9da44f25a581fd369a51b4027ea9ce9503a28a1d8`.
  `band-review.mjs --wasm` 및 `--whole-cell`·`--whole-row` 조합을 실행했다.
  carrier와 정상 대조군2종 모두3쪽, Native/WASM JSON 수치·비수치 차이0,
  SVG 각각3/3동일이다. 각 `*-band-review/backend-comparison.json` 및
  `run.json`이 소스 manifest·입력/PDF·WASM 해시를 보존한다.
- 최종 `carrier-band-review/native-review-2.png`, `wasm-review-2.png`,
  `wasm-review-3.png`, 독립 `wasm-overlay-3.png`를 직접 판독했다.
  2쪽 마지막 `주거용 건물` 행의 높이/가운데 정렬, 3쪽 마지막 행 및 자료출처·
  후속 표 연결을 확인했다. 작은 위치/대체 글꼴 차이는 위와 동일하다.
  이 두 쪽을 메인테이너 시각 판정 대상으로 제시한다.

남은 대상은 `line-split-*`의 셀 내부 줄 컷이다. 이번 저장 물리 밴드 계약을
그대로 내부 줄 분할에 적용하거나 미지원 검사를 삭제하지 않는다. 해당 조각별
패딩·정렬·소비 잔량을 독립 PDF/저장 줄 소유와 함께 검증해야 한다.
전체 CI/all-target/WASM Clippy는 이번 내부 절편의 실행 범위가 아니며,
커밋·push·PR·기본 엔진 전환은 수행하지 않았다.

작업지시자의 후속 “시각 판정 통과입니다. 다음 절편 진행을 승인합니다”에 따라
위 carrier 2/3쪽의 저장 끝 행 높이·정렬 판정을 통과로 기록한다. 다음 작업은
`line-split-*`의 내부 줄 컷이며 이 승인으로 해당 미지원 범위까지 통과 처리하지 않는다.

### 후속 절편: 자식 셀 내부 저장 줄 컷과 조각별 정렬

정상 저장 `tests/fixtures/issue7353/child-frame-tail/line-split-saved.hwp`와
대응 `line-split-2024.pdf`를 사용했다. 생성 job·해시는 fixture README에
기록되어 있으며 이 절편에서 원본 LineSeg를 수정하거나 PDF를 재생성하지 않았다.
독립 기준은 마지막 행의 `년간·상승률`이2쪽, `(%)`가3쪽인 소유 관계다.
숫자 셀은2쪽에 한 번만 나타나고 자료출처·후속5×4표는3쪽에서 이어진다.

- 입력→공통 결과: `ir.rs::bind_table`의 AnchoredTable 분기에서 다음 문단의
  저장 원점과 자식 common.height를 `stored_child::qualify`에 전달한다.
  한 행 안에 저장 컷이 있으면 `stored_child_frames::StoredRowFrames::qualify`가
  같은 `FlowCursor::fit_cell_until`로 셀별 시작 cursor·끝 block·점유·정렬을
  확정한다. 첫 행 조각3351HU는9681−6330, 마지막1282HU는1848−283×2이다.
  실제 저장 컷에 도달해야 하며, 마지막 줄과 패딩이 마지막 밴드를 정확히
  설명해야 한다. 내용보다 작은 공간이나 임의 꼬리 높이를 수용하지 않는다.
- 측정→예산: bind 전에 row_heights/height를 두 물리 조각의 합으로 확정한다.
  `fragment.rs::TableCursor::fit_plain_rows`는 해당 저장 조각 전체 높이를
  검사한다. 실패하면 소비하지 않고 DoesNotFit/이월한다. 충분하면 해당 조각의
  실제 높이를 누적하고 cursor와 remaining band를 함께 전진시킨다.
- 실제 배치: `StoredRowFrame::place`가 qualification과 같은 cell flow를
  실제 원점에서 소비한다. CENTER는 각 조각의 내용+패딩을 기준으로 적용한다.
  `FlowCursor`의 AnchoredTable/상위 Table 소비 경로는 반환 reserved_height와
  바깥여백·셀 inset을 예약한다. `TextPaint::build_node`는 이 placement를 그린다.
  별도 clipping·paint 높이 확장·Legacy fallback은 추가하지 않았다.
- 종료/비적용: 두 번째 조각을 끝내면 다음 행으로 이동한다. 이미 끝난 숫자
  셀의 content는 다시 내보내지 않는다. 한 plain-text 행의 두 저장 프레임에
  한정하며 rowspan·반복 제목·다중 컷 행·세 프레임 이상·편집 후 재조판은
  이번 qualification 대상이 아니다. 해당 범위까지 지원/시각 일치로 주장하지 않는다.

수정 전 `split-before-issue_7353_stored_child_tail.log`에서 새 정상 내부 컷
계약만 `unqualified stored child frame geometry`로 FAIL(4PASS/1FAIL)했다.
수정 후 정식 계약7건 및 관련10 suite는 **195PASS/0FAIL**이다.
`output/7353/r19/frame-after-child/split-final-summary.log`와 개별 로그에
연결한다. 정상 입력/직렬화 대조, 작은/큰 첫 밴드·모순된 tail·셀 단위 정책
음성 입력, API 부족 예산에서 미소비 후 재시도, 실제 셀 좌표·단일 내용 소유·
CENTER·뒤 문단/표·EOF를 검사했다. 45/50/54px는 내용 줄은 들어가지만
3351HU 조각+283×2HU 바깥여백+141HU 부모 끝 inset은 들어가지 않는 예산이다.
재시도는 충분한 공간의 직접 결과와 실제 placement가 동일하다.
API 예산/수동 모순 입력은 PDF 일치 근거가 아닌 경계 계약으로 구분했다.

Native 2/3쪽을 직접 판독해 내부 줄 소속·숫자 중복 없음·후속 표 연결을 확인했다.
작은 위치·대체 글꼴 외형 차이는 남는다. 이 입력은 수정 전 미지원이므로 그 차이를
‘기존 차이’로 단정하지 않으며 PDF 픽셀 완전 일치를 주장하지 않는다.
증적은 `output/7353/r19/frame-after-child/line-split-split-review/`, 명령은
`split-review.mjs --wasm`이다. 기준 HEAD `50823731af6050c60ec3cfcc898abf36daa6b35a`
위 WIP이며 `split-source.sha256`·`split-test-source.sha256`으로 구현/계약을 고정한다.
Docker fresh WASM·lint·최종 직접 판독 결과는 아래에 이어 기록한다.

- Native lib Clippy `-D warnings` PASS (`split-clippy.log`,30.24초).
  Cargo fmt check·integration source rustfmt check·`git diff --check` PASS.
  전체 CI/all-target/WASM Clippy를 수행했다는 의미는 아니다.
- 기존 carrier/whole-cell/whole-row의 새 Native JSON은 각 이전 band-review
  Native JSON과 `cmp`로 바이트 동일하다. 승인된 대조군 배치를 바꾸지 않았다.
- Docker fresh WASM 성공 (`split-wasm-build.log`,7분14초), SHA256
  `3a2c9082849ba008f66d042bed2e6e739cc139466bebb3e873d88f8ea6b7b95b`.
  `split-review.mjs --wasm` 및 `--carrier`·`--whole-cell`·`--whole-row`를 실행했다.
  네 입력 모두3쪽이며 Native/WASM JSON 수치·비수치 차이0, SVG 각각3/3동일이다.
  `*-split-review/backend-comparison.json`과 `run.json`이 입력/PDF/소스/WASM을 고정한다.
- 최종 `line-split-split-review/wasm-review-2.png`, `wasm-review-3.png`,
  standalone `wasm-overlay-3.png`를 직접 판독했다. 목표 줄 소속·숫자 미반복·
  조각별 정렬·자료출처와 후속 표 보존을 확인했다. 앞서 기록한 작은 위치·
  글꼴 차이는 남아 있다. PDF 래스터화의 xref 복구 경고는 로그에 보존했으며,
  출력 원본을 바꾸거나 이 경고를 fidelity 통과 근거로 사용하지 않았다.

현재 절편은 **구현/선택 범위 검증 완료, 메인테이너 2/3쪽 시각 판정 대기**다.
R5 전체 완료·원본 전체 문서 통과·미지원 다중 프레임 지원을 의미하지 않는다.
커밋·push·PR·기본 엔진 전환은 수행하지 않았다.

#### 메인테이너 피드백: 2쪽 내부 셀 시작 원점 미충족

“이번 변경이 오히려 2쪽 내부 셀 시작위치가 더 차이가 난다”는 피드백으로
현재 절편의 시각 판정은 **미충족**이다. 위 테스트 PASS는 유지하되 셀 상단
원점의 정확성까지 검증했다는 뜻으로 사용하지 않는다. 다음 절편으로 넘기지 않는다.

- 실제2쪽 부모 표와 내부1×1표 시작Y가 모두77.48px다. 부모 유효 위 여백은
  223HU(2.973333px)인데 내부 표 시작 원점에 재적용되지 않았다.
- 내부 첫 글줄 `편익의 종류`는Y79.76px, baseline13.6px → 기준선93.36px.
  같은 입력의 한컴 PDF trace는608×0.12pt =72.96pt →96dpi 기준97.28px로,
  rhwp가3.92px 위다. 이는 글꼴 굵기 문제가 아니라 원점 차이다.
- 입력 내부 문단0/1의 첫 저장vpos는 둘 다100HU이다. `stored_text.rs::cell_frames`
  는 `line.vertical_pos < previous`만 검사하므로 이 쪽 경계를 표시하지 않는다.
  `flow.rs::fit_cell_until`은 자식이 `starts_stored_frame()`일 때만 조상 위 여백을
  재계상한다. 이 경우 일반 예산 분할로 처리되어 부모223HU와 자식141HU의
  프레임 재시작 대신 이전 문단 줄간격의 잔량을 소비한다.
- 진단 `start-cursor.rs`는 실제1쪽 내부 표 높이26.2666667px 예산에서
  `space_left=0.9466667px`(71HU)를 보존하고 다음 첫 줄을2.28px에 둔다.
  이는 잔량71HU + 다음 문단 앞 간격100HU다. 정상 재시작에서 필요한
  부모223HU + 자식141HU + 문단 앞100HU와 비교하면293HU(3.906667px)가
  부족하여 PDF의3.92px 차이와 대응한다(나머지는 PDF 출력 좌표 반올림 범위).
  근거: `output/7353/r19/frame-after-child/start-origin.log`, `start-cursor.log`,
  `control-80000-trace-2.xml`, 기존 `line-split-split-review/actual/native.json`.
- 이전 승인 입력whole-cell(같은 용지80000HU)의 `whole-cell-band-review`에서도
  동일한 내부 표Y77.48/글줄Y79.76이다. 이전carrier(78000HU)는 내부 표Y80.453333이다.
  따라서 기존 출력 간 차이는 확인되지만, 다른 입력인carrier와 비교한 것을 이번
  줄 분할 코드의 신규 회귀라고 단정하지 않는다. 같은line-split 입력의 수정 전은
  미지원이어서 전체 출력 전후 비교가 불가능하다.

현재 원인 조사를 위해 진단 산출물만 추가했으며 production 코드는 수정하지 않았다.
후속 보정은 단순 `<`→`<=` 변경이나Y 보정이 아니다. 같은 원점을 갖는 겹침 문단과
실제 새 프레임을 독립된 저장 조각/수용 경계 근거로 구분하고, 부모·자식 여백과
문단 간격을 한 번씩 계상해야 한다. 회귀 계약에 부모→내부 표 시작점과 첫 글줄
기준선의 독립 기대값을 추가하고, 1쪽 끝/2쪽 처음/2쪽 끝을 함께 재검증한다.

#### 승인된 보정: 같은 양수 저장 원점의 문단 프레임 시작

메인테이너의 수정 승인 후 `stored_text.rs::cell_frames`의 판정을 보강했다.
단순 `<=` 치환은 하지 않았다. 현재/직전 저장 글줄 원점이 같고 양수이며,
현재 문단 앞 간격과 그 원점이 일치하는 일반 텍스트 문단에서만 판단한다.
직전 줄 높이가 양수이고 음수 줄간격·문단 뒤 간격이 없어 정상 전진과 같은
원점 배치가 모순되는 경우 저장 프레임 시작을 보존한다. 직전 문단의 단 설정은
비점유 메타데이터로 허용하되, 유효한 최초 단 설정인지는 기존 IR binder가 검증한다.
0 원점·앞 간격과 다른 원점·음수 줄간격의 겹침 가능 사례는 새 판정에서 제외했다.
이 범위를 임의 편집 LineSeg나 모든 겹침 문단의 해석 완료로 확대하지 않는다.

실제 경로는 `cell_frames → ir.rs::bind_table`의 `StoredFrameStart` 생성 →
`flow.rs::fit_cell_until`의 terminal inset 사전 예약/마커 경계 종료 →
자식 continuation의 `starts_stored_frame` → 같은 fit의 부모 위 여백 계상 →
`fragment.rs::fit_plain_rows`의 실제 셀/글줄 배치다. paint에서Y를 옮기지 않았다.
일반 예산 분할의 잔여 Space를 새 저장 프레임의 첫 문단 앞 간격으로 섞지 않는다.
기존 자식 내부 컷·최소 물리 밴드·숫자 동반 셀·마지막 유닛 뒤 종료 경로는 유지했다.

독립 근거는 앞 절과 동일한 정상 저장 `line-split-saved.hwp`와 대응 PDF다.
API 반례는 이 입력을 복제해 원점/줄간격만 바꾼 합성 계약이며 PDF 근거와 구분했다.

| 관측값, 96dpi | 수정 전 | 수정 후 | 독립 기준/판정 |
| --- | ---: | ---: | --- |
| 2쪽 부모→내부 표 시작 간격 | 0 | 2.973333px | 부모 유효 위 여백223HU |
| 내부 표→첫 글줄 간격 | 2.28px | 3.213333px | 내부 위 여백141HU + 문단 앞100HU |
| 첫 글줄 기준선 | 93.36px | 97.266667px | PDF97.28px, 반올림 범위 내 |
| 1쪽 부모 표 하단 | 986.213333px | 989.186667px | PDF985.285333px, **약3.9px 차이 남음** |

1쪽 하단 변화도 숨기지 않는다. 저장 컷이 인식되면서 기존 물리 밴드 예약 후
부모 아래 여백223HU가 계상된다. 본문 하단991.066667px 안에는 있지만 PDF와
외곽선이 정확히 같지는 않다. 위 여백 보정을 근거로 1쪽 외곽 일치까지 선언하지 않는다.
`native-review-1.png`에서 이 차이를 직접 확인했고, 2쪽 상단의 개선과 별도 판정한다.
3쪽 JSON/SVG는 수정 전과 바이트 동일하며 기존 자식 마지막 줄·자료출처·뒤 표를 보존한다.
carrier/whole-row는 세 쪽 모두 수정 전과 동일하다. whole-cell은 동일한 양수 원점을
가진 1/2쪽에 같은 보정이 적용되고 3쪽은 그대로다.

검증 명령/로그는 `output/7353/r19/frame-after-child/`에 보존했다.

- `origin-before-issue_7353_stored_child_tail.log`: 새 2쪽 원점 검사 수정 전 FAIL
  (7 PASS/1 FAIL, 빌드 오류가 아닌 실제 좌표 assertion). 보정 후 같은 원점 검사 PASS.
  이후 1쪽 아래 여백/본문 수용 검사도 추가해 `origin-final-summary.log` 9/9 PASS.
- `run-policies.sh origin-related ...`: 관련10 suite **197/197 PASS**.
  `origin-boundaries-summary.log`: 기존 원점·프레임 끝·채워진/중첩/열린 누름틀
  5 suite **18/18 PASS**. 총 선택 범위215건이며 전체 CI 실행을 의미하지 않는다.
  기존 내부 줄 컷의 예산 부족/재시도 계약은 새로 인정한 앞 문단 프레임을 먼저
  이어받도록 수정했고,3351HU 물리 밴드의 required height와 소유 유닛 검사는 유지했다.
- Native build/Clippy `-D warnings` PASS (`origin-build.log`, `origin-clippy.log`).
  Cargo fmt·integration source rustfmt·`git diff --check` 확인.
- Native 직접 판독: `line-split-origin-review/native-review-{1,2}.png`.
  기준 HEAD는 앞 절과 같은 `50823731af6050c60ec3cfcc898abf36daa6b35a` 위 WIP다.
  `origin-source.sha256`·`origin-test-source.sha256`으로 최종 소스/계약을 고정했다.

현재 2쪽 시작 원점 개선의 구현/선택 검증 결과이며, 메인테이너 시각 통과를
대신 선언하지 않는다. Docker fresh WASM 결과는 아래에 이어 기록한다.

- Docker fresh WASM PASS,7분12초 (`origin-wasm-build.log`). WASM SHA256:
  `4a2edde6df5c5fb58d455f9931f201c1b25df2306881cf9b5850dc37476a77fd`.
- `origin-review.mjs --wasm` 및 `--carrier`·`--whole-cell`·`--whole-row` 실행.
  네 입력 모두 Native/fresh WASM JSON 수치·비수치 차이0, 각 SVG3/3 동일.
  `*-origin-review/run.json`, `backend-comparison.json`에 입력/PDF/소스/WASM 연결을 남겼다.
- `line-split-origin-review/wasm-review-2.png`, standalone `wasm-overlay-2.png`,
  `wasm-origin-detail.png`를 직접 확인했다. 마지막 자료는 같은 원본 좌표의
  PDF/수정 전 WASM/수정 후 fresh WASM을 표시하며 위치를 맞춰 이동하지 않았다.
  2쪽 상단의 개선과 **1쪽 하단 차이 증가**를 함께 보존했다. PDF 래스터화의
  xref 복구 경고는 로그에 남겼으며 입력/PDF 원본은 변경하지 않았다.

상태: **2쪽 원점 보정 후보 구현/선택 검증 완료, 전체 시각 통과 아님**.
1쪽 하단 차이를 포함한 메인테이너 판정 대기이며 다음 절편으로 넘기지 않았다.
커밋·push·PR·기본 엔진 전환·전체 CI 검증은 수행하지 않았다.

#### 메인테이너 판정과 원본 전체 문서 종단 확인 (2026-09-27)

메인테이너가 앞의 원점 보정 자료에 **시각 판정 통과, 다음 진행 승인**을 주었다.
앞서 공개한1쪽 하단 약3.9px 차이도 기록에 유지한다. 이 승인을 모든 입력의 PDF
외곽선 완전 일치나 R5 전환 승인으로 확대하지 않는다.

다음 대상은 추가 속성 구현이 아니라 `samples/86712_regulatory_analysis.hwp`의
무수정 원본 종단 실행이다. production source는 앞의 `origin-source.sha256`과
전부 같음을 재확인했다. 같은 소스의 Docker fresh WASM을 재사용했으며 재빌드하지 않았다.

- `origin-probe samples/86712_regulatory_analysis.hwp output/7353/r19/after-origin/original`:
  이전 문단172의 미지원 경계를 넘어 **64쪽을 출력하고 정상 종료**.
- 정상 한컴 재저장 대조군 `output/7353/r19/child-frame-tail/full-saved.hwp`도64쪽 종료.
  원본을 이 입력으로 교체하지 않았다.
- `node output/7353/r19/after-origin/capture.mjs --wasm` 및 `--resaved --wasm`:
  두 입력 각각 Native/WASM JSON 수치·비수치 차이0, **SVG64/64 동일**.
  원본 증적은 `after-origin/`, 재저장 증적은 `after-origin/resaved/`에 분리했다.
  각각의 `run.json`에 input/PDF/source/WASM 해시를 고정했다.

**원본의 현재 한컴 PDF와의 시각 차이는 남는다.** 원본27쪽에는 건설공사비지수
자식 표의 첫 행만,28쪽에는 나머지 두 행과 자료출처·후속 표가 나온다. PDF27쪽에는
세 행 전부가 있다. 원본29쪽의 앞 문단도 PDF와 다르다. 같은 총64쪽 또는 backend
일치만으로 이 차이를 통과시키지 않는다.

기존 `frame-after-child/compare.log`의 실제 입력 비교와 이번 최종 출력 검사를 연결했다.

| 원본 IR 위치 | 무수정 원본 저장값 | 정상 한컴 재저장값 | 출력에서 확인한 의미 |
| --- | --- | --- | --- |
| p172/parent.cell77/p0 | vpos `[0,1560,0,1560]` | `[0,1560,3120,0]` | 원본은 마지막2줄이 다음 프레임, 재저장은 마지막1줄만 이월 |
| parent.cell80/p0/control1/cell0의 p6·p8·p10·p15 | 재저장보다 각1줄 많음 | 총4줄 적음 | 자식 표 이전 누적 저장 위치 차이7200HU(96px) |
| 같은 셀 p20의3×12 자식 표 | 저장 첫 높이1463HU | 10212HU | 원본 첫 조각은 제목 행, 재저장은 전체3행 |

문자/char_offsets/필드/스타일 참조의 기존 비교 결과와 달리, 실제 줄 분할과 저장
높이는 서로 다르다. 재저장본27·28쪽의 Native/fresh WASM review 및29쪽 standalone
overlay를 직접 판독하면 표 분할·자료출처·후속 표 위치가 PDF와 가깝게 대응한다.
글꼴 폭·굵기와 작은 외곽 차이는 남으며 자동 점수를 시각 판정으로 사용하지 않았다.
원본1·27·28·29·36·64쪽 review와27쪽 overlay도 직접 확인했다. 원본1·36·64쪽의
주요 배치와 종료 내용은 보존되지만, 이는64쪽 전수 시각 판독을 뜻하지 않는다.
재저장 대조군 결과를 원본의 PDF 일치로 보고하지 않는다.

`tests/cases/issue_7353_stored_child_tail.rs`에 무수정 원본 계약을 추가했다.
페이지 수64를 정답으로 고정하지 않고 아래 실제 출력 의미를 검사한다.

- 원본 cell77의 저장4줄이 실제 같은 셀에서 정확히 한 번씩 보존되고, 저장 원점
  재시작 전후2줄씩 서로 인접한 페이지에 속함.
- p172의 부모/중첩 표 조각이 본문 안에 있고,3×12 자식 첫 조각 높이가 원본의
  독립 저장 높이1463HU이며, 모든 자식 셀의 내용이 누락·중복 없이 보존됨.
- 자식 마지막 조각 뒤 자료출처가 같은 페이지에 배치되고 문서가 안전 상한 전에
  정상 종료하며, 종료 후 재호출에도 추가 페이지가 없음.

명령 `bash output/7353/r19/frame-after-child/run-policies.sh original-contract issue_7353_stored_child_tail`:
**10/10 PASS**, rustfmt 및 `git diff --check` PASS.
테스트 SHA256 `d2efafb771ce77351a912c8f33fb5f507658caffb51d3f0a69e62552bbd6317a`.
처음 작성 시 전체 표의 순회 순서를 셀 읽기 순서로 사용한 검사 오류를 수정하여 해당
소유 셀만 검사했다. 이 초기 assertion 실패는 production 결함의 수정 전 FAIL 증거가 아니다.
기존215건은 이전 동일 production source 검증이며 이번 실행 건수와 중복 합산하지 않는다.

상태: **원본 종단 실행/저장 소유 계약 충족, 원본과 현재 한컴 재출력의 시각 일치 미충족**.
남은 판정은 저장 줄 정보 보존과 현재 한컴 재조판 차이의 구분이다. 이 차이를 지우려고
원본 LineSeg를 변경하거나 문서별 예외를 추가하지 않았다. A 전체 시각 승인, B/C 잔여
기능, D 통합/기본값 전환은 완료로 올리지 않는다. 이번에는 테스트/증적/기록만 추가했으며
전체 CI·추가 production 수정·커밋·push·PR·기본 엔진 변경은 하지 않았다.

#### 절편 종료 — 메인테이너의 입력 신뢰성 판정

작업지시자는 이 절편을 종료하고 다음으로 진행하도록 지시했으며, 그 의미를 다음처럼
명확히 했다: **재저장본의 rhwp 피델리티가 맞으므로 원본 샘플의 저장 조판 정보 문제로
판정하고, 이번 차이를 우리 조판 로직의 결함으로 보지 않는다.**

이를 이번 샘플에 대한 메인테이너 판정으로 반영한다. 원본27–29쪽과 현재 한컴 PDF의
차이는 앞의 실측 기록 그대로 보존하지만, 이 차이는 더 이상 이번 작업의 수정 대상이나
다음 진행 차단 항목이 아니다. 같은 원본/재저장 비교와 원인 재조사는 중단한다.
재저장 대조군을 통과 근거로 수용하며, 원본 파일 교체·삭제나 저장 정보 강제 수정은 하지 않는다.
이 판정을 다른 문서의 저장 정보 또는 모든 V2 경로의 정확성으로 일반화하지 않는다.

다음 절편은 승인 계획B의 기존 실물 TAC·어울림 사례 연결이다. 기존 바이너리로 입력을
바꾸지 않고 최초 진입을 실행한 결과, 다음 세 가지 지원 공백을 확인했다.

| 기존 회귀 | V2 최초 거부 | 다음 처리 |
| --- | --- | --- |
| #6601, #7150 | p0 `stored TAC mixed text requires qualified space advances` | 같은 공백/TAC 구성 경로의 source 축·서식 조건부터 함께 확인 |
| #7008 | `section decoration, grid or writing direction` | 표 배치 이전 구역 속성 제한으로 분리 |
| #7158 | p0 `TAC carrier paragraph constraints` | 목표 어울림 문단 이전의 문단 제약으로 분리 |

이 결과는 기존 Legacy 회귀 실패가 아니라 V2 미지원 진입점이다. 네 입력 모두 아직
전체 출력/시각 통과로 세지 않는다. 샘플을 재작성하거나 미지원 속성을 제거하지 않고
공통 원인을 처리하며, 이번에 종료한 원본27–29쪽 조사로 되돌아가지 않는다.

#### B 진행 — 첫 본문 TAC의 공백·구조 슬롯 소유 연결

2026-09-27 다음 작업 승인에 따라 기존 실물 사례의 V2 진입을 처리했다.
원본27–29쪽 종료 판정은 유지한다. 이번 변경은 Legacy 기본값이나 저장 LineSeg의
내용을 바꾸지 않고, 기존 공통 IR의 문자 위치를 V2 공백 경로에서도 수용하는 것이다.

**입력 → 규칙 → 소비 경로.** #6601 무수정 HWPX의 첫 문단은
`secd, cold, 표 A, 공백 3개, 표 B`이며 parser 결과는 `char_offsets=[24,25,26]`,
`hwpx_axis_shift=8`, `control_text_positions=[0,0,0,3]`이다. 구조 컨트롤의8-unit
슬롯은 원문 소유 위치이지 가로 점유 폭이 아니다. 기존 control-only 경로의
구조 접두부 검증을 `tac.rs::qualified_structural_axis`로 공유하고, `tac_spaces::compose`
및 `tac.rs::object_rows`가 같은 검증을 사용하도록 연결했다. 후자는 source 슬롯의
연속성·줄 소유를 검증하고 공백 advance와 표 rect를 `ParagraphItem::InlineTables`로
생산한다. 기존 공통 fit/예약/paint가 이 결과를 소비하며, 이후 좌표 보정이나 문서별
예외를 추가하지 않았다. 모호한 다중 줄 HWPX 축, 탭, 보이는 혼합 텍스트의 제한은 유지했다.

정식 회귀 `issue_7353_table_v2_document_flow.rs`의
`first_body_space_separated_tac_preserves_common_ir_order_in_both_formats`는 위 source 순서의
합성 입력을 HWP/HWPX로 직렬화·재파싱한다. 좌/중앙/우 정렬별 실제 두 표 좌표,
3개 공백의 위치·연속성·보존, 뒤 문단 y=74와 양 포맷의 표 bbox를 검사한다.
기대값의 근거는 구조 슬롯의 무점유, 표80px·바깥여백2px, 줄 너비300px의 정렬 불변식이다.
합성 계약을 정상 한컴 저장본 또는 #6601 전체 피델리티 증거로 세지 않는다.

- 수정 전 첫 계약 실행은 동일한 source축 자격 오류로116 PASS/1 FAIL이었다
  (`frame-after-child/structural-before-issue_7353_table_v2_document_flow.log`).
  이후 검사 코드의 공개 accessor/노드 순회와 종료 간격 옵션을 정정했으므로 이를
  최종 테스트 파일 그대로의 수정 전 실행이라고 표현하지 않는다. 실물 #6601의
  수정 전 거부는 앞 절편의 최초 진입 표에, 수정 후 거부는
  `core-flow-cases/6601-after.log`에 남겼다. `before.log`는 합성 계약의 요약이다.
- Native build와 lib Clippy(`--locked -p rhwp --lib`, 공용 `target/pr-review`) PASS.
  `run-policies.sh structural-after issue_7353_table_v2_document_flow` 117 PASS,
  관련 ir_text/nested_text/split_policy/headers 37 PASS, 기존 Legacy 네 사례12 PASS:
  이번 선택 검사 **166 PASS/0 FAIL**. 로그는 `output/7353/r19/core-flow-cases/`의
  `after.log`, `controls.log`, `legacy.log`; fmt 및 `git diff --check` PASS.
- `docker compose --env-file .env.docker -p rhwp run --rm wasm` 성공(7m08s).
  `node output/7353/r19/core-flow-cases/check-wasm.mjs`로 fresh WASM을 Chrome에서 실행했다.
  초기 Chrome 시작 실패는 기존 로컬 검증과 같은 `--no-sandbox` 실행 옵션으로 해소했다.
  합성 계약은 Native/fresh WASM JSON·SVG 동일이며 backend 비교 PNG에서 두 표·공백
  간격·뒤 문단을 직접 확인했다. `structural-spaces-backends.png`와 standalone
  `structural-spaces-overlay.png`는 **backend 계약 자료이며 한컴 정답지 비교가 아니다**.
- 검증 source는 HEAD `50823731af6050c60ec3cfcc898abf36daa6b35a` 위 기존 WIP를 포함한다.
  변경 source/test·입력·WASM SHA256은 `core-flow-cases/wasm-results.json`에 기록했다.
  전체 CI 상당 검사나 push 전 전체 lint 묶음은 실행하지 않았으며 커밋/push/PR/기본값
  전환도 하지 않았다.

**남은 실물 경계.** 같은 오류 문자열을 공통 원인으로 묶었던 앞 기록을 정정한다.

| 사례 | 이번 fresh WASM의 최초 거부와 조사 결과 | 다음 구현 경계 |
| --- | --- | --- |
| #6601 | 공백/구조 축 거부 해소 후 `unequal TAC occupied envelopes`; 두 표의 바깥여백 포함 높이13136/13116HU, 저장 줄13136HU | 서로 다른 표의 기준선·상하 점유 공통 계산. 20HU 임의 tolerance로 우회하지 않음 |
| #7150 | 첫 셀 내부 `s0/p0/t2/c0/p0`의 탭과 TAC 혼합이 공백 자격 오류의 원인 | 탭 정지점의 위치 의존 advance; 공백으로 치환하지 않음 |
| #7008 | `section decoration, grid or writing direction` | 구역 속성 지원 경계 |
| #7158 | `TAC carrier paragraph constraints` | 어울림 대상 이전 carrier 문단 속성 경계 |

네 사례의 Legacy 선택 회귀는 통과하지만 V2 전체 출력은 아직 미완료다. 이번 결과로
B 완료나 한컴 시각 판정 통과를 선언하지 않는다. 다음 작업 우선순위는 #6601의 서로
다른 TAC 점유 높이를 실제 기준선 규칙으로 연결하는 것이며 별도 소규모 재승인은 요구하지 않는다.

#### B 진행 — 서로 다른 TAC 높이의 기준선·점유 영역 공유

2026-09-28(KST). 승인된 다음 작업으로 표별 `높이+여백`이 같아야 한다는 제한을
제거했다. 기존 #7049/#7150의 한컴 관측과 이번 정상 저장 대조군이 확인하는 표 기준선
(높이의85% 위/15% 아래)을 사용한다. 임의 오차로 두 높이를 같다고 취급하지 않는다.
`tac_metrics::TableBand`는 기준선 위 `max(.85*h+top)`와 아래
`max(.15*h+bottom)`를 각각 구해 줄 점유 높이와 각 표의 원점을 함께 생산한다.
두 최댓값의 소유 표가 다를 수 있으므로 개별 전체 높이의 단순 최댓값으로 대체하지 않는다.
Baseline 이외 문단 세로정렬은 명시적 미지원으로 남겼으며 그림 경로는 변경하지 않았다.

**실제 소비 연결.** 저장 경로 `tac::object_rows`는 저장 줄 높이와 공통 band의 일치를
검사하고, 재조판 경로 `tac_fresh::compose`는 같은 band를 생성한다. 둘 다
`ParagraphItem::InlineTables {height, advance, rects}`를 반환한다.
`document_input.rs` 본문 경로와 `ir.rs` 셀 경로는 `tac::bind`에서 실제 자식 plan 크기를
검사한 뒤 `FlowBlock::InlineTables`로 연결한다. `content.rs`는 그 rect의 점유를 검증하고,
`flow.rs`는 같은 height로 예산을 검사한 후 동일한 자식 x/y를 배치한다. 예산 부족이면
행 전체를 이월한다. paint에서 높이를 다시 늘리거나 위치를 clamp하지 않는다.
행 내부 분할/rowspan 컷은 이 원자 TAC 행 변경에 비해당이며 기존 분할 회귀는 유지했다.

**정식 검사와 독립 근거.** `issue_7353_table_v2_document_flow.rs`에 다음을 연결했다.

- 저장/재조판 × 본문/부모 셀 × 현재 쪽 수용/이월: 다른 높이의 실제 표 y·높이,
  빈 host 줄 보존, 내용 각각1회, 앞쪽에 일부만 출력되지 않는 원자 행 이월.
- 서로 다른 표가 최대 ascent/descent를 소유하는 반례: 개별 최대5900HU와 달리
  실제 공통 점유6325HU. 뒤 문단 위치를 검사하고5900HU 저장 줄은 거부한다.
- Top/Center/Bottom 문단 세로정렬을 Baseline으로 추측하지 않는 비적용 계약.
- `tests/fixtures/issue7353_unequal_tac_review/`의 정상 한컴 저장본: 두 자식 표,
  부모 외곽, 셀 안팎 뒤 문단의 최종 좌표·보존. 저장본과 carrier LineSeg만 지운
  재조판 파생본을 별도 검사한다. 생성 방법·SHA·MCP job·PDF 측정값은 fixture README 참조.

이 fixture는 기존 정상 `noop-saved.hwp`의 글꼴·용지·부모 표를 유지하고 내부 표만
높이5000/8000HU, 너비10000HU, 여백200HU로 바꾼 독립 대조군이다. 한컴이 HWPX를
저장한 HWP를 무수정 입력으로 사용하고 그 HWP에서 PDF를 생성했다. #6601 원본의
미지원 속성을 지운 대체본이 아니다. 앞서 작은 합성 HWP의 한컴 변환은180초 timeout으로
끝나 시각 근거에서 제외했다. 모든 입력/실패 증거는 `output/7353/r19/unequal-tac/`에 보존했다.

**검증 결과.**

- 변경 전 WASM에 동일한 두 최종 합성 입력을 실행하면 저장/재조판 각각
  `unequal TAC occupied envelopes` / `fresh TAC unequal baseline envelopes`로 거부했다.
  `before-wasm.json`에 이전 WASM 및 동일 입력 SHA와 오류를 보존했다. 변경 후 두 입력은
  각1쪽을 출력하고 Native/fresh WASM JSON·SVG가 완전히 같다(`after-wasm.json`).
- Native build, lib Clippy(`cargo clippy --locked -p rhwp --lib`, 공용 target/pr-review),
  해당 source/test rustfmt 및 diff check PASS. 최종 document_flow121 PASS,
  ir_text9/nested_text11/split_policy3/headers14 및 기존 Legacy 네 사례12 PASS:
  선택 검증 합계 **170 PASS/0 FAIL**. `unequal-tac/after.log`(기존120개)와
  `frame-after-child/unequal-fixture-final-issue_7353_table_v2_document_flow.log`(121개)를 연결한다.
- 최초 PDF 좌표 계약은 인쇄 축척을 반영하지 않아 부모 폭320pt 대319.643pt로 실패했다.
  PDF trace의 독립 x/y transform(.119851/.12, .119935/.12)을 반영해 비교했고
  0.25pt 허용치는 늘리지 않았다. 이는 테스트의 PDF 좌표 변환 수정이며 엔진 보정이 아니다.
- Docker fresh WASM 성공(7m14s). `node output/7353/r19/unequal-tac/check-wasm.mjs`,
  `node output/7353/r19/unequal-tac/capture.mjs --wasm` PASS.
  Chrome의 일시 시작 실패 뒤 같은 명령 재실행이 성공했다. 정상 한컴 fixture도
  Native/fresh WASM JSON·SVG 동일, 각각1쪽이다.
- `review/wasm-review-1.png`와 `review/wasm-overlay-1.png`를 직접 판독했다.
  작은 표 시작점이 큰 표보다34px 낮고, 각 표 하단·부모 외곽·CELL AFTER·뒤 본문 위치가
  대응한다. 대체 글꼴 외형 및 PDF 인쇄 축척에 따른 미세한 외곽 차이는 남는다.
  PNG에는 좌표 축척 보정을 적용하지 않았다. 자동 픽셀 점수로 시각 통과를 대신하지 않는다.
  에이전트 직접 확인 완료이며 이번 대조군의 메인테이너 판정은 아직 받지 않았다.

검증 기준은 HEAD `50823731af6050c60ec3cfcc898abf36daa6b35a` + 기존 WIP.
source/test SHA는 `unequal-tac/source.sha256`, WASM SHA는
`2323510f3a8fd31ad367a90b6dfc5a5ea8c660d98970870598d64f8890440270`.
WASM 생성 뒤에는 검사/fixture/기록만 추가했으며 production source는 변경하지 않았다.
전체 CI 및 push 전 전체 workspace lint는 이번 절편에서 실행하지 않았다.
Legacy 기본값·원본 샘플·baseline/ignore는 변경하지 않았고 commit/push/PR도 하지 않았다.

**다음 경계.** 무수정 #6601은 이번 TAC 높이 제한을 통과한 뒤 첫 셀에서
`cell direction or line wrap` 미지원으로 멈춘다. SQUEEZE/KEEP 셀 줄바꿈 속성의
실제 의미와 저장/재조판 소비 경로가 다음 대상이다. #6601 전체 출력이나 B/R5 완료로
세지 않으며, 종료 승인된86712 원본/재저장 조사로 돌아가지 않는다.

#### B 진행 — 저장 SQUEEZE 셀의 정책 전달과 공통 자간 계산

2026-09-28 메인테이너가 직전 높이 차이 TAC 대조군을 시각 통과로 판정하고 다음 절편을
승인했다. 그 판정은 해당 대조군 범위에 반영한다. #6601 원본 첫 두 표의 셀은 모두
HWPX `subList lineWrap=SQUEEZE`이며 parser가 공통 IR `Cell.line_wrap=1`로 보존하고 있다.
V2 `ir::bind_table`은 이 값이0이 아니라는 이유로 내용 구성 전에 전체를 거부했다.

**규칙과 소비 연결.** 셀 줄바꿈 정책은 grid에서 버릴 속성이 아니라 문단 구성 입력이다.
`CellParagraphComposer::compose_with_cell_wrap`로 전달하고, 기본/custom composer는
지원하지 않는 정책을 거부한다. 실제 TextComposer/IrTextComposer는 공통
`stored_text::validate_cell_wrap`로 intact plain 저장 줄만 수용한다. dirty/fresh/개체 혼합 및
KEEP/알 수 없는 값은 기존 BREAK 재조판으로 대체하지 않는다. 기존 localize의 저장 폭·원문
소유 검증과 최종 paint 검증도 그대로 거친다.

속성 거부만 제거한 첫 실행에서 긴 한 줄 대조군은 `text preview run outside occupied line`으로
실패했다. 따라서 공통 `layout_composed_paragraph_in_frame`에 명시적 `squeeze_stored_line`
정책을 추가했다. V2는 source SQUEEZE일 때만 true를 전달하며 기존 Legacy 호출은 false다.
공통 glyph 폭 계산의 자간 수렴 함수를 사용하고, 여백/셀 폭 변경이나 출력 clip은 하지 않는다.
완성된 TextLine/TextRun·End를 구성기가 한 번 생산하고 `ir.rs`의 Lines→FlowBlock,
fit/예약과 `text_ir::record_items`의 paint가 동일 결과를 소비한다. paint 전 별도 좌표 보정은 없다.
분할 컷/rowspan/이어받기 알고리즘 변경은 비해당이며 관련 선택 회귀를 실행했다.

**독립 증거와 정식 검사.** `tests/fixtures/issue7353_squeeze_review/README.md`에 입력 생성,
한컴 저장/PDF job, SHA 및 관측값을 기록했다. 정상 한컴 글꼴·용지·부모 표를 가진 기존 대조군에서
자식 표를10000HU 너비 SQUEEZE로 바꾸고 긴 문구를 넣어 한컴이 LineSeg를 생성했다.
저장 HWP를 무수정으로 시험하고, 별도 HWPX roundtrip도 검사한다. 두 형식 모두 실제 한 줄,
좌우 안여백, 음수 자간, 자식/부모 외곽, 셀 안팎 뒤 문단의 위치·내용1회 보존을 검사한다.
한컴 PDF는 glyph 겹침을 포함한 강한 자간 압축을 보여준다. rhwp의 SVG 대체 글꼴/글자 형태는
그와 완전히 같지 않아 **한 줄·폭·표 기하 일치와 glyph 외형 완전 일치를 구분**한다.

`issue6601_first_paragraph_replays_original_squeeze_cells`는 원본 첫 문단의 속성·내용·저장
줄을 바꾸지 않고 이후 문단만 제외한 분리 계약이다. 두 표의 크기·기준선차17HU, 모든 셀,
비공백 내용의 순서/보존을 실제 V2 출력에서 검사한다. 전체 문서 피델리티로 세지 않는다.
원본 전체는 이제 paragraph index1의 `TAC row exceeds stored width`에서 거부되며,
첫 두 표의 SQUEEZE 미지원에서 벗어났음을 `squeeze/original.log`에 남겼다.

변경 전 WASM을 보존해 같은 정상 저장 대조군과 무수정 #6601을 실행했으며 둘 다
`cell direction or line wrap` 오류다(`squeeze/before.json`). 초기 negative 테스트는 dirty flag를
HWP로 직렬화해 재현하려다 실패했다. runtime flag가 파일에 보존되지 않는 테스트 오류이므로
Document IR에서 직접 검증하도록 고쳤으며 구현 조건/기준값을 완화하지 않았다.

**최종 검증과 시각 확인.** Native build, lib Clippy, 해당 source/test rustfmt 및 diff check PASS.
`document_flow`124, `ir_text`9, `nested_text`11, `split_policy`3, `headers`14,
기존 Legacy #6601/#7150/#7008/#7158 합계12: 선택 검증 **173 PASS/0 FAIL**.
`squeeze/tests.log`, `squeeze/original-test.log` 및
`frame-after-child/squeeze-original-issue_7353_table_v2_document_flow.log`에 남겼다.
이는 전체 CI 통과 주장이 아니며 전체 workspace lint와 전체 회귀는 이번 절편에서 실행하지 않았다.

Docker `docker compose --env-file .env.docker -p rhwp run --rm wasm` 성공(7m12s),
`node output/7353/r19/squeeze/capture.mjs --wasm` 성공.
`squeeze/review/backend-comparison.json`은 Native/fresh WASM 숫자 차이0, 기타 차이0,
SVG 동일을 기록한다. 각1쪽이다. 직접 `wasm-review-1.png`와 `wasm-overlay-1.png`를 열어
한 줄 유지, 양쪽 안여백, 부모/자식 외곽 및 CELL AFTER/뒤 본문 위치를 확인했다.
강한 SQUEEZE의 개별 glyph 모양·겹침 차이는 남는다. 글꼴만의 원인이라고 단정하지 않으며
완전한 glyph 피델리티 통과로 보고하지 않는다. 자동 픽셀 보조값34.94%도 판정을 대신하지 않는다.
이번 대조군의 메인테이너 시각 판정은 아직 대기다.

검증 source는 HEAD `50823731af6050c60ec3cfcc898abf36daa6b35a` + 기존 WIP이며,
`squeeze/source.sha256`의 현재 source/test 일치를 확인했다. fresh WASM SHA256은
`27005fe478f33f3bfb325ee28c3776725f0cb943d0771901ec33403b5bc90afa`.
`squeeze/review/run.json`에 입력/PDF/WASM/source SHA와 DPI를 연결했다.
빌드 후 production source를 변경하지 않았다. Legacy 기본값·원본·baseline/ignore는 그대로이며,
commit/push/PR은 하지 않았다. 다음 대상은 원본 paragraph index1의 TAC 저장 폭 거부 원인이다.

#### B 진행 — 가운데 정렬 과대폭 TAC와 셀의 물리 영역

2026-09-28 메인테이너가 직전 SQUEEZE 시각 대조군을 통과 판정하고 다음 절편을 승인했다.
그 판정은 이전 대조군 범위에 반영하며 #6601 전체 완료로 확대하지 않는다.

**원인과 독립 근거.** #6601 paragraph1 자체의 표는 폭50741+바깥여백276=51017HU로
저장 줄51024HU 안에 든다. 거부 지점은 첫 셀 paragraph0의 가운데 정렬 자식 표다.
저장 줄49720HU에 자식49855+바깥여백280=50135HU가 놓인다. `tac::object_rows`는
단일 과대폭 표를 Left/Justify/Right만 수용해 Center를 거부했다.
원본 한컴 PDF(`unequal-tac/36331407_side_by_side_tac_tables-2020.pdf`)의 부모
왼쪽43.896pt/자식 왼쪽50.373pt는 안여백510+바깥여백140HU를 반영한다.
별도 정상 한컴 저장 대조군도 같은 규칙을 보인다. 가운데 여유가 음수일 때 시작점을
음수로 이동하거나 표를 축소하는 규칙이 아니다.

**생산→소비 추적.** `tac::object_rows`의 정렬 여유→공통 Rect→`tac::compose`의
ParagraphItem::InlineTables→`ir::bind_table`/`tac::bind`의 불변 InlineTableInput→
`TableContentPlan::from_grid_rows`의 물리 검사→`FlowCursor::fit_cell_until`의
area.x+child.x 및 child.plan.width→재귀 paint다. 가운데 정렬도 단일 과대폭 규칙에
포함했다. 후속 물리 검사는 안여백을 제외한 줄 폭을 clipping 경계로 혼동하고 있었다.
이를 셀 오른쪽 물리 경계로 수정해 오른쪽 안여백에 걸치는 표를 수용한다.
측정된 표 폭·높이·여백·그리기 좌표를 바꾸거나 clamp하지 않는다.
부모 외곽을 넘는 자식은 별도 clipping 계약이 없으므로 여전히 명시적 거부다.
여러 표/선행 공백/후행 공백만 넘치는 행의 저장 줄 거부와 본문 텍스트 폭 검사는 유지한다.

**검증.** 정상 저장 대조군과 가운데 정렬 후행 공백 계약은 수정 전 각각
`TAC row exceeds stored width`로 FAIL, 나머지123 PASS였다
(`frame-after-child/center-before-issue_7353_table_v2_document_flow.log`).
가운데 정렬 수용만 바꾼 중간 실행은 셀의 `ContentBounds`로 실패해 실제 후속 소비자를
추가 확인했다. 최종 document_flow126 및 geometry9/nested18/ir_text9/nested_text11/
split_policy3/headers14/Legacy12 합계 **202 PASS/0 FAIL**이다.
`center-overwide/tests-final.log`와 해당 `frame-after-child/center-final-*` 로그에 연결한다.
실제 출력 HWP/HWPX, 부모·자식·뒤 문단 좌표, 오른쪽 물리 경계 정확히 fit 및1HU 초과를
검사한다. 정상 문서와 합성 경계를 구분한 출처는
`tests/fixtures/issue7353_center_overwide_review/README.md`다.
분할 컷·예약·이어받기·종료 코드는 비변경이고 관련 nested/split/header 대조를 실행했다.

최종 Native build17.05s, lib Clippy27.33s, 해당 Rust fmt/diff check PASS.
Native review를 직접 열어 자식 시작점·너비·외곽·부모 및 뒤 문단 위치를 확인했다.
Docker fresh WASM 성공(7m10s), `node output/7353/r19/center-overwide/capture.mjs --wasm`
성공. Native/fresh WASM 각각1쪽, JSON 숫자/기타 차이0 및 SVG 동일이다.
`center-overwide/review/wasm-review-1.png`와 `wasm-overlay-1.png`를 직접 열어
자식 표 시작·외곽·너비, 부모와 앞뒤 문단을 대조했다. 인쇄 축척에 따른 미세한 외곽 차이와
글꼴 외형 차이는 남으며 자동 잉크 보조값40.10%로 시각 통과를 대신하지 않는다.
에이전트 직접 확인 완료. 2026-09-28 메인테이너가 이 가운데 정렬 과대폭 TAC
대조군을 시각 통과 판정하고 다음 절편을 승인했다.

source는 HEAD `50823731af6050c60ec3cfcc898abf36daa6b35a` + 기존 WIP이며,
`center-overwide/source.sha256` 일치를 확인했다. WASM SHA256은
`a697b550eae681fd665eac925494914c7016d6d48930569dc48c0d81288f214d`.
입력/PDF/source/WASM SHA는 `center-overwide/review/run.json`에 연결했다.
원본 전체 재실행은 기존 폭 거부를 지나 paragraph index1의
`stored child anchor ownership`에 도달한다(`center-overwide/original-final.log`).
이 결과는 전체 원본 완료나 B/R5 완료가 아니며 다음 절편의 경계다.
전체 CI·workspace lint는 미실행이며 Legacy 기본값·원본·baseline/ignore 변경과
commit/push/PR은 하지 않았다.

#### B 진행 — 셀 시작의 단 정의와 TAC 그림 소유권

2026-09-28 직전 가운데 정렬 과대폭 TAC 판정 통과를 반영하고 다음 경계를 진행했다.
원본 #6601의 paragraph1/parent cell1/child cell0 첫 문단은 `[ColumnDef, Picture]`다.
정상 단 정의는1단/동일폭/간격0/구분선 없음이고 그림은7087×7087HU TAC다.
`ir::initial_cell_column`은 이미 이를 검증하며 `seen[0]`을 구조 슬롯으로 처리한다.
거부 원인은 `text_ir::compose_items`의 그림 진입 조건이 Picture만 허용한 점이다.

**생산→소비.** 이 조건에 검증된 ColumnDef를 포함하되 슬롯을 삭제하지 않았다.
`tac::stored_object_rows`가 원래 UTF-16 위치로 그림의 저장 줄과 Rect를 만든다.
`pictures::compose`는 이 결과로 ObjectRow와 실제 ImageNode를 함께 생성한다.
`ir::bind_table`은 구조 슬롯과 그림 슬롯을 별도 소유 검사하고 동일 bounds를 Lines로
전달한다. `FlowCursor`가 그 줄 상자를 수용/이월하며 `text_ir::bind_paint`와 TextPaint는
같은 payload를 해당 줄 위치에 배치한다. 뒤에서 구조 슬롯 너비·별도 그림 원점을
추측하거나 덮어쓰지 않는다. 다단·비초기 단 선언/그림 appearance/리소스 검사는 그대로다.
분할 컷·예약·이어받기·종료 알고리즘은 비변경이며 새 문단은 기존 picture row 경로를 쓴다.

**입력과 독립 기준.** 원본 그림·crop·단 슬롯을 검증 가능한 중첩 표 대조군에 옮기고,
LineSeg를 비운 후 한컴이 정상 저장/출력한 HWP와 PDF를 만들었다. 생성 절차, 변경 속성,
job ID, 해시 및 독립 기대 좌표는 `tests/fixtures/issue7353_column_picture_review/README.md`.
원본의 재저장/수정본이 아니라 독립 대조군이며 원본 전체 통과를 주장하지 않는다.

**수정 전후.** 정상1단을 설정한 정식 export 경계 검사는 수정 전
`stored child anchor ownership` FAIL/기존27 PASS였다
(`column-picture/tests-before-valid.log`, `frame-after-child/column-picture-before-valid-*`).
최초 합성 입력은 ColumnDef 기본값의0단 설정으로 잘못 거부되어1단으로 고친 뒤 다시 실행했다.
정상 한컴 저장본도 같은 수정 전 코드의 실제 DocumentV2 constructor에서 동일하게 실패했다
(`column-picture/fixture-before.log`; 출력 생성 전 거부되어 before PNG는 없음).
최종 export29는 정상 HWP/HWPX96/144DPI의 그림·부모/자식·뒤 문단 좌표와 종료,
합성 같은 줄/다른 줄 두 그림의 소유권·1회 출력·다단 거부를 검사한다.
새 PDF 검사의 Rust 소수 리터럴 빌드 오류는 고친 후 export29를 다시 실행해 통과했다.
이 빌드 오류는 결함 재현으로 세지 않는다.

최종 focused export29/document_flow126/geometry9/nested18/ir_text9/nested_text11/
split_policy3/headers14/cell_column_insets3/local_column_anchor6/Legacy12 = **240 PASS/0 FAIL**.
`column-picture/tests-final.log`(초기 export 빌드 오류 포함), `tests-export-final.log`,
`tests-legacy.log` 및 `frame-after-child/column-picture-final-*`의 최종 개별 로그를 연결한다.
Native build16.84s, lib Clippy27.68s, 변경 Rust fmt/diff check PASS.
Native review를 직접 열어 그림 중앙 위치·크기·내용, 자식/부모 외곽, CELL BEFORE/AFTER와
뒤 본문을 확인했다. 미세한 인쇄 축척·glyph 차이를 자동 픽셀 보조값으로 통과 처리하지 않는다.

원본 전체는 이번 조건을 통과하고 paragraph index2의 `body control or multiple anchors`에
도달했다(`column-picture/original.log`). 원본 앞 두 문단만 보존한 별도 진단 출력은2쪽으로
기준 PDF 표지1쪽과 차이가 남아 있다(`column-picture/excerpt/native.json`). 이 원본의
페이지 배치 경계는 별도 후속 대상이며 대조군 통과로 해소했다고 보고하지 않는다.
Legacy 기본값, 원본, baseline/ignore를 변경하지 않았고 전체 CI·workspace lint는 미실행이다.
commit/push/PR은 하지 않았다. fresh WASM 대조는 아래 최종 검증으로 연결한다.

Docker `docker compose --env-file .env.docker -p rhwp run --rm wasm` 성공(7m17s),
`node output/7353/r19/column-picture/capture.mjs --wasm` 성공.
Native/fresh WASM 각각1쪽, JSON 숫자 차이0/기타 차이0, SVG 동일이다.
`column-picture/review/wasm-review-1.png`와 `wasm-overlay-1.png`를 직접 열어
그림 중앙 위치·크기·crop 내용, 부모/자식 외곽, 앞뒤 문단을 대조했다.
미세한 인쇄 축척·글꼴 외형/래스터 차이는 남는다. 2026-09-28 메인테이너가 이번
대조군의 시각 판정 통과와 다음 절편 진행을 승인했다.
source는 HEAD `50823731af6050c60ec3cfcc898abf36daa6b35a` + 기존 WIP,
`column-picture/source.sha256` 일치 확인. fresh WASM SHA256은
`aad67a9d4ffeef7c6ecf4783dce037e9741af067766c4b60265f973a202daca8`.
입력/PDF/source/WASM SHA는 `column-picture/review/run.json`에 연결한다.
최종 production 변경 뒤에 Native·WASM·시각 대조를 수행했고 이후 production은 바꾸지 않았다.

#### B 진행 — 본문 TAC 종료 줄간격과 빈 페이지

원본 #6601의 표지 앞 두 문단을 그대로 보존한 HWPX에서 내용이 없는2쪽이 생성됐다.
원본 PDF1쪽과 대응하며 추출은 뒤 문단만 제거한다. 입력 생성 방식·해시·독립 기대 좌표는
`tests/fixtures/issue7353_body_tac_tail_review/README.md`에 기록했다.
본문 높이71428HU 중 실제 두 줄과 사이 간격은71222HU다. 마지막 TAC의 다음 줄 간격
780HU를 물리 내용처럼 이월하여574HU만 있는 빈2쪽을 만들던 문제다.

**규칙과 실제 경로.** `tac::compose`가 만든 ParagraphEnd의 마지막 줄간격을
`document_input.rs`의 TAC lowering에서만 문서 끝/다음 명시적 Page break 조건으로
소비하지 않게 한다. 기존 `into_flow_items_at_end`를 사용하며 문단 뒤 간격·표 여백·
작성된 빈 문단·일반 후속 줄 간격은 유지한다. 결과 FlowBlock을 BodyCursor가 수용/이월하고
FlowCursor의 기존 InlineTables 줄 상자와 TextPaint가 같은 실제 점유를 배치한다.
뒤에서 높이를 축소하거나 원점을 덮어쓰지 않는다. 페이지 나눔의 블록 인덱스는 lowering 뒤
확정된다. 중첩 표의 컷·rowspan·예약·clipping 알고리즘과 일반 텍스트/앵커 호스트 종료는
비변경이다. 마지막 줄간격을 실제 빈 문단이나 임의 Space와 동일시하지 않는다.

**수정 전후와 반례.** 정식 `tests/cases/issue_7353_table_v2_document_flow.rs`의
추가3건을 수정 전 production에서 실행해127 PASS/2 FAIL을 확인했다
(`output/7353/r19/body-tac-tail/before-final.log`). 원본 표지는2쪽→1쪽,
합성 문서 끝/명시적 쪽 나눔은[2,3]쪽→[1,2]쪽이다. 일반 다음 문단 간격과 작성된 빈 문단은
전후 모두 보존된다. 최초 합성 입력의 Justify 거부는 Left로 바로잡고 재실행했으며
그 입력 오류를 결함 재현으로 세지 않는다. `body-tac-tail/before-after.json`은
수정 전2쪽의 Body가 비었고 **전후1쪽 JSON/SVG가 완전히 동일**함을 확인한다.
표6개·그림1개·제목/부서명·부모 위치/높이 및 뒤 본문의 실제 좌표와 종료를 검사했다.

최종 document_flow129/export29/geometry9/nested18/ir_text9/nested_text11/
split_policy3/headers14/page_number_timeline5/Legacy12 = **239 PASS/0 FAIL**.
로그는 `body-tac-tail/tests-final.log`, `tests-document-final.log`와
`frame-after-child/body-tail-final-*`의 개별 결과를 연결한다. 종합 로그에 포함된 초기
HWP 라운드트립 거부는 최종 document_flow 재실행129 PASS로 구분한다.
이 별도 rhwp HWP 직렬화본의 picture appearance/resource 거부는 수정 전후 동일하다
(`roundtrip-before.log`, `roundtrip-after.log`). 해당 경로는 개선/통과 범위에서 제외했다.
Native build27.35s, lib Clippy28.85s, 변경 Rust fmt/diff check PASS.

Native review에서 결재란·부모/자식 외곽·제목·로고·부서명의 보존을 직접 확인했다.
원본 PDF와 남는 대체 글꼴 외형 및 미세한 인쇄 축척 차이를 해소했다고 주장하지 않는다.
원본 전체의 다음 경계는 paragraph index2의 ClickHere+TAC 혼재
`body control or multiple anchors`이며 이번 표지 발췌 통과는 원본 전체/B/R5 완료가 아니다.
Legacy 기본값·원본·baseline/ignore는 변경하지 않았다. 전체 CI/workspace lint는 미실행,
commit/push/PR은 하지 않았다. 2026-09-28 메인테이너가 이 절편의 시각 판정 통과와
다음 절편 진행을 승인했다.

Docker `docker compose --env-file .env.docker -p rhwp run --rm wasm` 성공(7m21s).
`node output/7353/r19/body-tac-tail/capture.mjs --wasm`으로 실제 Chrome에서 검증했다.
Native/fresh WASM 모두1쪽, JSON 숫자 차이0/기타 차이0, SVG 동일이다.
`body-tac-tail/review/wasm-review-1.png`와 `wasm-overlay-1.png`를 직접 열어 결재란,
제목 상자, 로고, 부서명의 위치와 내용 보존을 확인했다. 기존 글꼴 차이는 남는다.
source는 HEAD `50823731af6050c60ec3cfcc898abf36daa6b35a` + 기존 WIP이며
`body-tac-tail/source.sha256` 일치 확인. fresh WASM SHA256은
`e4e7859e8e78ac993de8f3641ada49051f362bf742216344a03df3129ed1ba83`.
입력·기준 PDF·source·WASM 해시는 `body-tac-tail/review/run.json`, backend 대조는
`backend-comparison.json`에 기록했다. 최종 production 변경 후 검증이며 이후 production은
바꾸지 않았다. 표지1쪽 보존과 불필요한 빈2쪽 제거가 이번 시각 판정의 범위다.

#### B/C 진행 — 저장 누름틀 시작과 TAC 표의 원본 슬롯 소유

원본 #6601 paragraph2는 `[ClickHere(본문, dirty=1), TAC Table]`, char_count17,
표시 문자열 없음, LineSeg 시작0이다. 로컬 field_ranges/orphan ends는 없다.
공공기관 문서의 삽입 콘텐츠 앞에 남는 누름틀 시작 슬롯을 표시 객체와 혼동하지 않는다.
필드·저장 줄을 삭제하지 않고 원래 table control index1과 UTF-16 offset8을 보존한다.

**규칙·소비 경로.** `fields::stored_tac_prefix`가 저장된 채워진 ClickHere 시작 접두부와
TAC 표의 조합을 검증한다. body `document_input`과 cell `ir::bind_table`/`text_ir`가
같은 조건으로 진입하며 필드만 비가시 소유 슬롯으로 계상한다. `tac::object_rows`는
완전한 슬롯 스트림과 기존 저장 축을 검증하고 필드를 가로 점유에 더하지 않는다.
원본 표 인덱스로 StoredTacRow의 Rect를 만들어 `tac::compose` → InlineTables →
body/recursive FlowCursor의 요구 높이·컷 → 기존 paint로 전달한다. 줄 소속·최종 원점은
뒤에서 다시 추측하지 않는다. child 컷/rowspan/반복 제목/예약·이어받기 알고리즘은 비변경이다.
날짜/계산 필드, dirty=0, 표 뒤 시작, 닫힌 범위, fresh 필드+TAC는 계속 거부한다.
누름틀 명령을 평가하거나 문단 밖 끝 범위를 만들어내지 않는다.

**독립 입력과 제한.** 원본 앞 네 문단을 보존한 `prefix.hwpx`와 한컴 재저장
`prefix-saved.hwp`는 필드 진입을 통과한 다음 표 안의 `text preview run outside occupied line`에서
거부된다. 이 검사를 완화하지 않았다. 두 진단본과 실패 로그를 보존하며 원본 전체 통과로
보고하지 않는다. 이번 규칙만 검증할 독립 대조군은 원본 표지 둘째 TAC 앞에 원본 필드
시작을 추가하고 LineSeg를 비워 **한컴이 정상 저장**한 `control-saved.hwp`다.
동일 HWP에서 만든 `control-2020.pdf`와1쪽을 대조한다. 생성 과정·변경 속성·job ID·
해시·독립 기대 좌표는 `tests/fixtures/issue7353_body_field_tac_review/README.md`.
PDF 없이 합성 슬롯 추가만으로 시각 통과를 주장하지 않는다.

**수정 전후와 반례.** 최초 경계 실행은130 PASS/1 FAIL로 body 필드 거부를 확인했다
(`body-field/tests-before.log`, `frame-after-child/body-field-before-*`).
최종 정상 저장 fixture도 이전 production의 정적 probe에서 동일 거부가 재현된다
(`body-field/control-before.log`). 동일 bytes의 새 production 출력은1쪽이다.
정식 document_flow는 해당 fixture의 field/table 인덱스·표6개·그림1개·부모 y/높이·
제목/부서명·안내문 없음·종료를 검사한다. 합성 본문/중첩 셀 각각 시작1/2개와
같은 줄/다른 줄 표의 최종 Table/Cell/TextLine/TextRun 상자가 필드 없는 대조군과 같고,
AFTER가1회만 출력되는지도 검사한다. 잘못된 슬롯 수·fresh·다른 필드·guide 상태·
빈 명령·interleaved 시작은 public source query와 실제 DocumentV2에서 거부한다.

초기 테스트 작성의 타입/API 빌드 오류는 수정했으며 결함 재현으로 세지 않는다.
첫 문단 자동 구조 슬롯의 다중 줄 축 모호성과 합성 HWP 직렬화 높이 차이는 테스트 입력을
선행 BEFORE 문단이 있는 HWPX로 정리했다. production의 저장 축/높이 검사를 느슨하게
바꾸지 않았다. 정상 저장본 검사에서 private accessor 참조 오류도 공개 query로 고친 뒤
최종132건을 재실행했다. 이 중간 빌드 실패는 `tests-document-final.log`의 최종 결과와 구분한다.

최종 document_flow132/export29/geometry9/nested18/ir_text9/nested_text11/
split_policy3/headers14/open_field_markers3/filled_field5/nested_field3/Legacy12
= **248 PASS/0 FAIL**. `body-field/tests-final.log`와 `tests-document-final.log`,
`frame-after-child/body-field-final-*`를 연결한다. Native build21.89s,
lib Clippy29.42s, 변경 Rust fmt/diff check PASS.

Native review/standalone overlay를 직접 확인했다. 누름틀 안내문 없이 원래 표·제목·로고·
부서명을 유지한다. 자동 내용 픽셀 보조값35.33%는 남는 글꼴 외형/미세한 인쇄 차이를 포함하며
시각 승인 근거를 대신하지 않는다. 이번 대조군은 메인테이너가 시각 판정 통과와
다음 절편 진행을 승인했다.
원본 전체의 다음 거부는 paragraph2의 표 내부 text run/line 경계다. 정상 저장 대조군만의
통과를 원본의 일치나 B/C/R5 전체 완료로 승격하지 않는다. Legacy 기본값·baseline/ignore·
원본 샘플은 비변경, 전체 CI/workspace lint는 미실행이며 commit/push/PR은 하지 않았다.

Docker `docker compose --env-file .env.docker -p rhwp run --rm wasm` 성공(7m18s),
`node output/7353/r19/body-field/capture.mjs --wasm`으로 실제 Chrome 출력 확인.
Native/fresh WASM 모두1쪽, JSON 숫자 차이0/기타 차이0, SVG 동일이다.
`body-field/review/wasm-review-1.png`와 `wasm-overlay-1.png`를 직접 열어 표지 내용과
위치를 대조했다. compare는 `wasm-compare-1.png`, 픽셀 보조값은35.33%이며
대체 글꼴 외형 차이를 포함한다. 원래 source 슬롯이 남은 상태에서 표를 출력하고 안내문을
추가하지 않는지 보는 판정 자료다. 수정 전은 출력 생성 전 명시적 거부여서 before PNG가 없다.
source HEAD `50823731af6050c60ec3cfcc898abf36daa6b35a` + 기존 WIP,
`body-field/source.sha256` 일치. WASM SHA256은
`c49facc7d86ce6ae94468ec9c9d26b045d26bce1e8eba3252b60a1b3eb3ad306`.
입력/PDF/source/WASM 해시와 backend 대조는 `body-field/review/run.json`,
`backend-comparison.json`에 연결했다. 최종 production 변경 뒤의 검증 결과이며
이후 production 변경은 없다.

#### B/C 진행 — 배분 정렬 마지막 글리프와 caret 전진 폭의 분리

앞선 누름틀+TAC 대조군의 시각 판정 통과를 반영하고 원본 paragraph2의 내부 연락처
행을 추적했다. `사회복지과장…오치호☎2573`의 마지막 run 끝650.900529px가 저장 줄
끝648.32px를 넘었지만, 차이2.580529px는 양수 배분 간격1회분과 정확히 같았다.
규칙상 배분 정렬의 가시 N글자에는 N-1개 간격이 있다. caret 전진의 말미 간격을
실제 glyph 점유로 간주한 V2 qualification 결함이며 셀 폭이나 폰트 크기 문제는 아니다.

**소비 경로.** 공통 `paragraph_layout::compute_line_extra_spacing`의 Distribute 계산 →
TextRun의 `extra_char_spacing`/replay positions → `table_v2/text::painted_inline_ends`가
기존 `TextStyle::glyph_fit_advance`로 cluster 점유 끝을 확인 → 기존 Lines의 높이·advance와
payload를 그대로 FlowCursor/paint에 전달한다. 실제 SVG의 `svg_cluster_text_length_attrs`와
Canvas의 `canvas_cluster_fit_scale`가 소비하는 공통 projection을 재사용한다.
run bbox·줄 폭·원점·표 크기·뒤 표 위치를 줄이거나 덮어쓰지 않는다. 양수 배분 간격이 있는
장식 없는 끝 run에만 적용하며, 밑줄/취소선/배경 등 advance 자체를 그리는 효과는 전체 경계를
유지한다. 기존 plain 말미 공백 처리와 음수 간격은 비변경이다. pagination·rowspan·컷·
누적 예약·continuation 알고리즘은 이번 절편 비해당이다.

**독립 근거와 수정 전후.** 원본 앞 네 문단만 보존한 `prefix.hwpx`(줄 정보 수정 없음)에서
기존 명시적 거부를 확인했다. 합성 배분 정렬 계약도 수정 전37 PASS/1 FAIL로 같은 원인을
재현했다(`run-extent/before.log`, `frame-after-child/run-extent-before-issue_7353_table_v2_text.log`).
최종 text38건은 단일/분리 run의 실제 SVG 마지막 glyph 끝이 독립 줄 오른쪽 경계에 닿는지,
논리 advance 보존, 다음 줄18HU 전진, 장식 오버플로우 거부를 검사한다. document_flow의
정상 fixture 계약은 연락처 전체 문자열·48324HU 저장 폭·1200HU 높이·두 표8500/6234HU와
후속 표 위치·종료를 검사한다. 수정 후 발췌 HWPX와 독립 한컴 재저장 HWP 모두2쪽이다.

동일 `prefix.hwpx`로 한컴 PDF를 생성했다: job `688605b1-8cfb-4624-8571-3f38e3289358`,
한컴11.0.0.9136/engine2020/direct DLL/32bit/preprocess none/2쪽.
`tests/fixtures/issue7353_body_field_tac_review/prefix-2020.pdf`이며 상세 생성 근거는 fixture
README와 `run-extent/pdf-{start,status,download}.json`이다. PDF SHA256
`c3589223be35eacd409f4ec3737bf4c36258cd03779cbafc4eb2d4ee6d3e3056`.
Native 2쪽 review/standalone overlay를 직접 열어 제목·연락처2573·셀 외곽·뒤 요약 표를
확인했다. 대체 글꼴 외형 차이는 남는다. 1쪽은 이전 승인된 body-tac-tail 출력과 JSON/SVG 동일.

최종 text38/document_flow133/export29/geometry9/nested18/ir_text9/nested_text11/
split_policy3/headers14/open_field_markers3/filled_field5/nested_field3/Legacy12
= **287 PASS/0 FAIL**. `run-extent/tests-final.log`의 document_flow 초기 빌드 오류는
테스트 helper 타입을 수정한 `tests-related.log`133 PASS로 대체하며 결함 재현으로 세지 않는다.
각 최종 로그는 `frame-after-child/run-extent-final-*`다. Native build11.52s,
lib Clippy28.27s, 변경 Rust fmt/diff check PASS.

중간 Native 빌드의 디스크 부족 후 stale library로 실행한 `tests-after.log`는 검증에서 제외했다.
소스/입력/증적은 보존하고 비사용 증분 캐시
`/home/edward/mygithub/rhwp/target/pr-review/debug/incremental/rhwp-0wymmoghcdain`
한 폴더(약2GB, 빌드로 재생성 가능)를 정리한 뒤 최종 build와 테스트를 다시 실행했다.

원본 전체의 다음 미지원은 paragraph index8의 `stored text requires intact single-segment rows`
(`run-extent/original.log`)다. 이번 2쪽 발췌 통과를 원본 전체 또는 B/C/R5 완료로 보고하지 않는다.
Legacy 기본값·원본·baseline/ignore 비변경. 전체 CI/workspace lint 미실행,
commit/push/PR 없음. fresh WASM 시각 증적은 아래에서 최종 기록한다.

Docker `docker compose --env-file .env.docker -p rhwp run --rm wasm` 성공(7m14s,
Rust release4m16s). `node output/7353/r19/run-extent/capture.mjs --wasm`으로 실제 Chrome에서
fresh WASM DocumentV2 출력을 얻었다. Native/WASM 모두2쪽, JSON 숫자 차이0/기타 차이0,
양쪽 페이지 SVG 동일(`run-extent/review/backend-comparison.json`). WASM SHA256
`31b2ae2c4d0eb14da7fcb27004de3e09da683d23003aa7403b886a719c879368`.
source HEAD `50823731af6050c60ec3cfcc898abf36daa6b35a` + 기존 WIP,
`run-extent/source.sha256` 검증 일치. 입력/PDF/source/WASM은 `run-extent/review/run.json`에 고정했다.
`wasm-review-2.png` 및 standalone `wasm-overlay-2.png`를 직접 열어 마지막2573,
제목 셀·뒤 요약 표·줄바꿈을 대조했다. compare는 `wasm-compare-2.png`다.
1쪽 표지는 Native review 확인과 backend 동일성으로 보존을 확인했다. 2쪽 픽셀 보조값68.14%는
글꼴 외형 차이를 포함하며 판정 기준이 아니다. Canvas 직접 paint 실행은 이번 증거가 아니며
공통 projection의 호출 경로만 대조했다. 메인테이너가 이 절편의 시각 판정 통과와 다음 진행을 승인했다.

#### B/C 진행 — 저장 Shift+Enter의 줄 소유권과 빈 줄 보존

앞선 배분 정렬 연락처 행의 승인 뒤 원본 paragraph index8을 조사했다. 원본 XML의
`hp:lineBreak`가 IR LF로 파싱되며, 저장 LineSeg는 시작0/31, vpos32046/34598HU,
높이1500HU로 두 줄을 지정한다. 문단13에도 정상 강제 개행이 있으며 두 줄의 높이가
1400/1200HU로 다르다. 기존 V2 `stored_text::localize`가 LF를 일반 제어문자와 함께
일괄 거부한 것이 원인이다. 줄 정보 손상이나 폰트 fallback 문제가 아니다.

**규칙·독립 입력.** LF는 가시 glyph가 아니라 줄 경계다. 각 LF 바로 다음 UTF-16 위치에
저장 줄 시작이 있는 경우만 재사용하고, 어긋난/누락된 저장 경계는 계속 거부한다.
연속/선두 LF의 빈 줄도 줄 상자를 가진다. 마지막 LF 뒤의 명시적 빈 저장 줄은
paragraph terminator 앞의 경계일 때만 수용한다. 원본 앞15문단을 보존한
`tests/fixtures/issue7353_stored_break_review/prefix.hwpx`는 뒤 문단만 제거하며
텍스트·스타일·LineSeg를 변경하지 않는다. 같은 입력의 한컴 PDF는 job
`44d60f06-c26d-4653-bb9f-270afb4207bd`, engine2020/11.0.0.9136/direct DLL/32bit/
preprocess none으로 생성한2쪽이다. 입력/PDF 해시와 생성 스크립트는 fixture README,
서버 영수증은 `stored-break/pdf-{start,status,download}.json`에 연결했다.

**생산·소비 경로.** `stored_text::localize`327행의 경계 수용 → `text.rs`512행에서
원래 줄 구성을 공통 composer에 전달 → 공통 paragraph layout의 TextLine/TextRun과
`is_line_break_end` → `stored_text::validate_paint`456행이 LF를 구조적 경계로 재구성해
원문 전체 소유권·행 메트릭·최종 전진량을 검증한다. `text.rs`587–673행은 이 최종
노드에서 높이·advance·payload를 함께 생성한다. 셀은 content plan, 본문은
`document_input.rs`513행의 FlowBlock::Lines로 전달한다. `flow.rs`356행은 전체 줄 높이로
fit 판단 후 같은 소유 유닛과 advance를 수용하며, `text.rs`234행/`document.rs`146행은
그 소유 키의 기존 payload를 실제 줄 원점으로 평행 이동한다. paint의 재줄바꿈·clamp·
별도 높이 증가는 없다. 이번 변경은 원점/높이 생산식을 바꾸지 않고 저장 경계의 수용과
내용 검증만 수정한다. TAC/앵커·rowspan·caption·header 분기는 비변경이다.

**경계 계약.** `saved_forced_breaks_preserve_blank_rows_and_fragment_ownership`는
선두/연속/말미 LF의 줄 높이12HU, 간격6HU, 뒤 문단과 총 소유권을 검사한다.
첫 예산21HU(상단3+첫 줄12+간격6)만 수용한 후 다음 페이지에 남은 빈 줄·텍스트를
중복 없이 배치하고 종료하는지 검사하며, 저장 경계 누락/오정렬은 실패해야 한다.
초기 수정 전38 PASS/1 FAIL은 준비 단계의 동일한 LF 거부로 재현했다.
수정 직후 시험에서 사용한 CellBreak는 셀 전체 원자성을 요구하므로 작은 예산이
의도대로 거부되었다. 행 내부 줄 분할을 시험하도록 기존 RowBreak 입력으로 정정한
후 최종39 PASS다. 이 시험 설정 오류를 production 회귀로 분류하지 않는다.
선두/연속/말미 조합은 합성 계약이며 한컴 실물 출력의 검증으로 승격하지 않는다.
기존 fresh 명시적 개행/빈 줄 계약도 함께 통과한다.

실물 fixture의 `saved_body_forced_breaks_keep_indented_rows_blank_and_successor`는
최종 DocumentV2의 두 개행 문단, 저장 들여쓰기3000/3918HU, 줄 높이1500/1400/1200HU,
중간 빈 문단500HU, 뒤 `점검 개요`/`자활근로사업단` 위치와 종료를 검사한다.
수정 전 발췌는 paragraph8에서 명시적 거부하여 before PNG가 없다
(`stored-break/before.log`); 수정 후2쪽이다. 원본 전체는 이제 paragraph23의
`text preview run outside occupied line`까지 진행한다(`stored-break/original.log`).
원본 전체 통과나 R5 완료가 아니다.

최종 text39/document_flow134/export29/geometry9/nested18/ir_text9/nested_text11/
split_policy3/headers14/open_field_markers3/filled_field5/nested_field3/Legacy12
= **289 PASS/0 FAIL**. `stored-break/tests-final.log` 및
`frame-after-child/stored-break-final-*`에 결과를 보존했다. Native build25.78s,
lib Clippy28.34s, 변경 Rust fmt/diff check PASS. 전체 CI/workspace lint 미실행,
Legacy 기본값·baseline/ignore·원본 샘플 비변경, commit/push/PR 없음.

Native review와 standalone overlay2쪽을 직접 열어 공문번호 줄·※ 주석·중간 빈 문단·
뒤 사업단 줄의 위치를 대조했다. 저장 줄바꿈과 위치는 보존되고 대체 글꼴 외형 차이는
남는다. 픽셀 보조값49.85%는 시각 판정 대신 사용하지 않는다. Docker fresh WASM
빌드와 실제 Chrome 결과는 아래에 최종 기록한다. 메인테이너가 이 절편의 시각 판정 통과와 다음 진행을 승인했다.

Docker `docker compose --env-file .env.docker -p rhwp run --rm wasm` 성공(7m20s,
Rust release4m21s). `node output/7353/r19/stored-break/capture.mjs --wasm`으로 실제 Chrome의
fresh WASM DocumentV2를 실행했다. Native/WASM 모두2쪽, JSON 숫자/기타 차이0,
두 페이지 SVG 동일(`stored-break/review/backend-comparison.json`). WASM SHA256
`98a33d194b5b66c7ccc9a7f907d7aa8febadf9e4d84164ac137bcd0815dd169d`.
source HEAD `50823731af6050c60ec3cfcc898abf36daa6b35a` + 기존 WIP,
`stored-break/source.sha256` 일치 확인. 입력/PDF/source/WASM은 `stored-break/review/run.json`에
고정했다. `wasm-review-2.png`와 standalone `wasm-overlay-2.png`를 직접 열어 대상 줄과
후속 배치의 보존을 확인했고 compare는 `wasm-compare-2.png`다. Native1쪽도 직접 확인했고
이전 승인 run-extent1쪽과 JSON 전체가 동일하다(`stored-break/source-observation.json`).
Canvas 직접 paint는 이번 실행 범위가 아니며 실제 WASM에서 생성한 SVG의 Chrome 출력이다.
남은 대체 글꼴 차이와 원본 paragraph23의 미지원은 유지한다. 최종 production 변경 뒤 검증이며
검증 이후 production 변경은 없다.

#### B/C 진행 — 배분 정렬 제목 셀의 말미 공백과 실제 glyph 끝점

앞선 저장 Shift+Enter 절편은 메인테이너 시각 판정 통과다. 다음 원본 paragraph index23의
6×10 표는 `안전\n교육 ` 및 `하절기 재난` 제목 셀에서 V2 수용이 거부되었다.
원본은 배분 정렬, 13pt, 저장 줄 높이1300HU, 가용 너비3572/4140HU다.
말미 공백 처리 분기가 배분 간격 처리보다 먼저 실행되어 마지막 글자 뒤 caret 간격까지
가시 glyph 끝점으로 판단한 것이 원인이다. 셀 너비·글자 크기·줄 높이의 문제가 아니다.

**공통 결과와 적용 경로.** 공통 `paragraph_layout::compute_line_extra_spacing`1784행이
만든 TextRun 간격 → `TextRun::replay_positions_for`의 원래 문자 위치 →
`text.rs::painted_inline_ends`325행이 같은 `glyph_fit_advance`로 가시 끝점을 계산 →
`text.rs`615행의 수용 검사로 연결된다. 논리 말미 공백은 그대로 두고 원래 run의 위치를
사용해 공백 이전 glyph의 실제 끝점을 검사한다. 문자열을 잘라 다시 배분하거나 너비를
줄이지 않는다. 별도 style run으로 분리된 말미 공백도 같은 규칙을 적용한다.
밑줄·취소선·배경 등 가시 장식은 기존 별도 경계를 유지한다.

이 값은 V2의 overflow 수용 판정만 바꾼다. 실제 TextLine/TextRun bbox·위치·높이·
advance·payload는 수정하지 않는다. 이후 flow가 동일 줄 높이로 fit/소유 유닛을 확정하고
기존 payload를 평행 이동한다. SVG/Canvas의 기존 replay projection을 재사용하며,
실제 backend 직접 실행 증거는 Native와 fresh WASM의 SVG/Chrome 경로다.
rowspan·컷·예약 높이·pagination·앵커 분기는 이번 수정 대상이 아니다.

**검출 계약.** `distributed_saved_suffix_spaces_keep_glyph_edges_and_following_rows`는
저장 줄의 배분 정렬, soft wrap/LF, 단일/분리 run과 마지막 숫자 glyph 끝점, 다음 줄·
뒤 문단 및21HU 예산의 continuation 소유권을 검사한다. 밑줄까지 셀을 넘으면 계속
거부해야 한다. 수정 전39 PASS/1 FAIL은 의도한 `text preview run outside occupied line`;
수정 후40 PASS다. 합성 경계는 실물 한컴 검증과 구분한다.

실물 `distributed_header_spaces_preserve_table_rows_and_following_heading`는
최종 DocumentV2의 대상 셀 두 줄·공백 보존·원래 glyph 크기·표 너비50465HU와
높이1582+2882+4×3274HU·뒤 `□ 점검 사진`을 검사한다. 초기 검사에서 CJK SVG의
없는 textLength를 요구하고 같은 `기` 글자를 다른 셀까지 선택한 시험 코드 오류를
정정했다. production 변경 없이 최종135 PASS이며 이 초기 실패는 조판 회귀가 아니다.

**독립 출력과 범위.** `tests/fixtures/issue7353_distributed_header_review/table.hwpx`는
원본 문단0,1,2,3,22,23,24만 남긴2쪽 발췌다. 문단 내부·표·저장 줄은 수정하지 않는다.
앞25문단 전체 진단본 `prefix.hwpx`도3쪽으로 보존한다. 각각 같은 입력의 한컴 PDF를
생성했다(engine2020/11.0.0.9136/direct DLL/32bit/preprocess none).
job·해시·재현 명령은 fixture README와 `header-runs/*pdf-*.json`에 기록했다.
생성 스크립트 namespace 변수와 옵션 변수의 이름 충돌을 정리한 뒤 재생성한 두 입력은
기존 PDF 생성 입력과 SHA256이 동일하다.

Native 직접 판독에서 대상 표의 제목 두 줄·외곽·뒤 제목 위치가 대응한다. 그러나 진단본
3쪽 위쪽의 원본 paragraph21 별표 마스킹 문단에는 줄바꿈·위치 차이가 남는다.
이 문단은 저장 LineSeg가 없어 재조판된다. 이번 셀 수용 수정과 구분하고 진단본의
전체 페이지 일치로 보고하지 않는다. 표 중심 발췌의 통과를 원본 전체로 확장하지 않는다.
원본 전체 다음 미지원은 index34의 `text preview stored rows or controls`다
(`header-runs/original.log`). 수정 전 표 발췌도 명시적 거부했으므로 before PNG는 없다.

최종 text40/document_flow135/export29/geometry9/nested18/ir_text9/nested_text11/
split_policy3/headers14/open_field_markers3/filled_field5/nested_field3/Legacy12
= **291 PASS/0 FAIL**. `header-runs/tests-final.log`의 document_flow 초기 검사 실패는
`header-runs/tests-document-verified.log` 및
`frame-after-child/header-runs-final-verified-issue_7353_table_v2_document_flow.log`의
최종135 PASS로 대체한다. 나머지 source/test는 해당 실행 후 비변경이다.
Native build24.54s, lib Clippy28.64s PASS, 변경 Rust fmt/diff check PASS.
Legacy 기본값·원본·baseline/ignore 비변경. 전체 CI/workspace lint 미실행, commit/push/PR 없음.

Docker `docker compose --env-file .env.docker -p rhwp run --rm wasm` 성공(7m22s,
Rust release4m19s). `node output/7353/r19/header-runs/capture-table.mjs --wasm`과
`capture.mjs --wasm`으로 실제 Chrome fresh WASM DocumentV2를 실행했다.
표 발췌2쪽 및 진단본3쪽 모두 Native/WASM JSON 숫자/기타 차이0, 각 페이지 SVG 동일하다
(`header-runs/{table-review,review}/backend-comparison.json`). WASM SHA256은
`88b851561cbea32007a7be16bb7d29d8dc70a0a3900719165b583b96266ec663`이다.
source HEAD `50823731af6050c60ec3cfcc898abf36daa6b35a` + 기존 WIP,
`header-runs/source.sha256` 일치. 각 `run.json`에 입력/PDF/source/WASM을 고정했다.

`header-runs/table-review/wasm-review-2.png`와 standalone `wasm-overlay-2.png`를
직접 열어 두 줄 제목·셀 및 표 외곽·후속 제목의 대응을 확인했다. compare는
`wasm-compare-2.png`다. 진단본 `header-runs/review/wasm-review-3.png`도 직접 확인했으며
표 위 별표 문단의 차이가 유지됨을 기록한다. 대체 글꼴 외형 차이는 남고, 픽셀 보조값을
판정 대신 사용하지 않는다. 메인테이너가 이번 제목 셀 절편의 시각 판정을 통과시키고
다음 진행을 승인했다. 기호 글리프 차이는 별도 처리 범위로 남기되, 지시에 따라 이슈는
등록하지 않는다. 셀 여백 및 기준선 대조는 `header-runs/vertical-diagnosis.json`에 보존한다.

#### B/C 진행 — 본문 누름틀의 문단 간 종료와 빈 줄 점유

앞 절편의 시각 판정 통과를 반영했다. 기호 글리프 차이에 대한 이슈는 등록하지 않았다.
다음 원본 #6601의 마지막 paragraph34는 paragraph2의 ClickHere 시작을 닫는 빈 문단이다.
V2가 `orphan_field_ends`를 문단 단독으로 거부하여 원본 전체를 수용하지 못했다.
입력의 beginIDRef1561678090과 ctrlID627272811을 같은 본문 story에서 대응시켜야 하며,
마커가 비가시여도 저장 줄1600HU와 뒤 간격1280HU는 실제 점유로 남아야 한다.

**공통 결과와 실제 경로.** `fields.rs::body_ends`15행은 본문 순서로 시작 ID를 추적하고
지역적으로 닫힌 range 및 셀 story를 제외한다. ID·컨트롤 종류·역순 종료·저장 문단의
무결성이 확인된 마커 전용 문단에만 자격을 부여한다. `document_input.rs`119행이
문단별 결과를 만들고470행에서 `text.rs::compose_text_with_body_end`446행으로 전달한다.
기존 저장 줄 composer가 같은 TextLine/높이/간격 payload를 생산하고 기존 body flow가
이를 소비한다. 높이·원점·컷·예약·pagination·paint는 변경하지 않았다. 셀/기본 compose는
자격 없이 호출하므로 셀의 orphan 종료를 허용하지 않는다. 문자열·IR 마커를 지우거나
누름틀 결과를 재평가하지 않는다. 가시 텍스트와 종료가 섞인 문단은 여전히 미지원이다.

**검출/반례.** `tests/cases/issue_7353_body_field_end.rs`는 종료 빈 줄의 실제1600HU 높이,
후속 줄 원점2880HU 전진 및 완전 소진을 검사한다. 잘못된 ID·중복 종료·가시 텍스트
혼합·무효 줄 높이는 거부한다. 두 중첩 시작의 역순 종료는 빈 줄 하나만 만들고 교차
종료는 거부한다. 최초2개 계약 수정 전1 PASS/1 FAIL(의도한 paragraph34 미지원),
수정 후 추가 중첩 경계 포함3 PASS다. `header-runs/body-end-before.log` 및
`body-end/contract-final.log`에 기록했다. 합성 AFTER FIELD의 저장 좌표를 실물 독립
출력으로 취급하지 않는다. 이전 초기 진단 harness의 컴파일 오류는 결함 재현이 아니다.

**독립 시각 자료.** 원본 전체는 이제4쪽을 끝까지 생성한다. `issue7353_body_end_review`
fixture의 `after.hwpx`는 원본 문단을 보존하고 저장 LineSeg 없는 가시 후속 문단을
덧붙인 대조군이다. `bounded.hwpx`는 문단0,1,2,3,32,33,34와 이 후속 문단만 남긴2쪽
발췌다. 원본 내부 저장 줄·표는 비변경이다. 두 입력 각각의 한컴 PDF를 MCP로 생성했고
생성 절차·작업 ID·SHA256은 fixture README에 기록했다. 발췌를 원본 전체로 부르지 않는다.

bounded2쪽의 붙임2→후속 문단 기준선 간격은 rhwp74.40px, 한컴74.31875px다.
빈 줄 높이21.3333px와 뒤 간격17.0667px가 보존된다. 기준선 절대 위치 차이는 약0.47px
이하이며 review/standalone overlay를 직접 열어 표 외곽·붙임 줄·후속 문단을 확인했다.
전체 after4쪽에는 기존 붙임과 후속 줄이 약8.6px 아래에 배치되는 차이가 남는다.
수정 전 바이너리의 앞34문단 prefix와 수정 후 원본의4쪽 가시 SVG는 모두 동일하다
(`body-end/before-comparison.json`). 이 차이는 이번 종료 처리로 발생한 이동이 아니다.
3쪽의 별표 문단 재조판과 글꼴 외형 차이도 남으며 원본 전체 일치를 선언하지 않는다.

**검증.** 기존 집중291 + 신규3 = **294 PASS / 0 FAIL**.
`body-end/regressions-final.log`가 개별 suite 로그를 연결한다. Native build36.04s,
lib Clippy29.57s, 변경 Rust fmt/diff check PASS. 전체 CI/workspace lint 미실행.
Legacy 기본값·baseline/ignore·원본 샘플 비변경, commit/push/PR 없음.

Docker `docker compose --env-file .env.docker -p rhwp run --rm wasm`의 wasm-pack은
7m31s(Rust release4m30s)에 산출을 완료했다. 외부 실행 세션은 종료코드143을 반환했으므로
셸 성공으로 기록하지 않는다. 완료 로그·새 산출 시각과 실제 브라우저 실행으로 산출물을
검증했다. WASM SHA256 `8680ee90bec1ea4b9e80fbd234fc80d0d53644fe88bfde45502d9de9248f0d03`.
`node output/7353/r19/body-end/capture-bounded.mjs --wasm` 및 `capture.mjs --wasm` 성공.
Native/fresh WASM JSON 숫자/기타 차이0, 발췌2쪽/전체 대조군4쪽 SVG 모두 동일하다.
`body-end/{bounded-review,review}/backend-comparison.json` 및 `run.json`에 고정했다.
source HEAD `50823731af6050c60ec3cfcc898abf36daa6b35a` + WIP,
`body-end/source.sha256`36파일 일치 확인. 검증 후 production 변경 없음.

대표 `body-end/bounded-review/wasm-review-2.png`, `wasm-overlay-2.png`와
전체 `body-end/review/wasm-review-4.png`를 직접 판독했다. 앞의 두 이미지는 종료 빈 줄과
뒤 문단 점유의 판정 자료이며, 전체4쪽 이미지는 남은 위치 차이를 보존하는 자료다.
Canvas 직접 paint가 아닌 실제 WASM 생성 SVG의 Chrome 출력이다.
다음 확인 대상은 남은 본문 재조판/페이지 시작 위치 차이이며 이번 절편의 완료 범위와 구분한다.

메인테이너가 본문 누름틀 종료 빈 줄 절편의 시각 판정을 통과시키고 다음 진행을 승인했다.

#### B/C 진행 — 본문 문단 말미 줄간격의 페이지 경계 소유

원본 #6601의4쪽 첫 줄은 수정 전 y103.04px였다. 본문 상단은7088HU/75=94.506667px이고,
저장 baseline1275HU를 더하면111.506667px다. 독립 원본 PDF의 첫 baseline111.398253px와
대응해야 하는데, 이전 문단31의 줄간격 잔여가8.533333px 이월되어 있었다.
문단31 자체는3쪽에20px 높이의 빈 줄로 정상 배치된다. 실제 빈 줄 누락이 아니라
`ParagraphEnd`의 following-line gap을 물리 `Space`로 낮추면서 발생한 다음 쪽 원점 오류다.

**경로와 범위.** 저장/재조판 composer → `ParagraphEnd::from_composed` → 본문
`into_body_tail` → `FlowBlock::FollowingLineGap` → `FlowCursor::fit_cell_until` →
`DocumentV2Session::next_page_json`의 `fit.lines`/`translate`로 이어진다.
composer가 구분한 문단 뒤 간격은 `Space`로 유지하고 다음 글줄까지의 간격만 현재 본문
페이지에서 한 번 소비한다. 다음 페이지에 잔여 간격을 재예약하지 않는다. fit이 확정한
줄 원점이 paint 원점이며 paint에서 별도 clamp하거나 문단을 지우지 않는다.
본문 일반 텍스트와 TAC의 문단 종료가 공통 경로를 쓴다. TAC의 명시적 쪽 나눔/문서 끝에서
줄간격을 생략하던 기존 분기는 유지한다. 음수 줄간격은 기존 row advance 소유를 유지한다.
셀 IR·중첩 표·padding·명시적 물리 Space·실제 빈 줄·저장 cell frame tail 경로는 비변경이다.
문단 중간 줄 간격의 일반 페이지 경계 정책은 이번 말미 간격 수정 범위가 아니다.

**검출과 대조군.** `issue_7353_body_field_end`의
`body_page_start_does_not_inherit_previous_paragraph_line_gap`는 원본의3쪽 빈 문단31 보존,
4쪽 문단32의 본문 상단 일치, 다음 문단33까지36px, 종료 후 추가 페이지 없음을 검사한다.
수정 전3 PASS/1 FAIL은 실제 원점 assertion 실패이며 수정 후 통과한다.
`fresh_body_gap_preserves_blank_lines_and_separate_paragraph_after`는 새 조판의12px 빈 줄과
18px pitch를 본문14/30/36px에서 대조하고, 별도4px paragraph-after는 본문18px 경계에서
다음 원점에 보존함을 검사한다. 첫 대조군은14px 본문에4px 물리 공간과12px 줄까지 함께
fit한다고 잘못 기대했으므로18px로 정정했다. 테스트 입력 오류이며 production 수정은 없었다.
최종 계약5 PASS(`body-end/contract-page-gap-verified.log`). 다음 절편에서 로그를 재확인한
결과, `body-page-gap/tests.log`는 실제 **289 PASS/2 FAIL**이었다. 앞서 기록·보고한
총296 PASS/0 FAIL은 잘못된 집계이며 **294 PASS/2 FAIL**로 정정한다. 실패 두 건은
아래 탭 절편에서 추적했다. 기존 baseline/ignore는 비변경이다.

**현재 관측.** 원본4쪽 첫 y94.506667px로 정상화, 문단33 y130.506667px, 종료 빈 줄
y166.506667px다.3쪽 첫 줄에 이월되던0.08px도 같은 규칙으로 제거되었고 내용 전체가
그만큼 이동했다.1–2쪽 SVG 동일,3–4쪽 내용·줄 소유·순서와4쪽 종료는 보존된다.
`body-page-gap/before-after.json`에 전후 bbox를 보존한다. 원본/기준 PDF SHA는
기존과 동일하며 신규 발췌·재저장·PDF 재변환 없이 같은 원본으로 검증한다.

Native build29.91s/lib Clippy30.00s PASS. Native4쪽 review를 직접 열어 첫 붙임 줄과
다음 줄 위치 개선을 확인했다.3쪽 별표 문단의 기존 재조판/대체 글꼴 차이는 남기며,
페이지 수 일치만으로 원본 전체 피델리티 통과를 선언하지 않는다.
기존 WIP 및 Legacy 기본값 유지, commit/push/PR/이슈 등록 없음. 전체 CI는 미실행이다.
최종 Docker fresh WASM과 Native/WASM·직접 시각 판독 결과는 아래에 연결한다.

Docker 표준 빌드는7m27s에 종료코드0으로 완료했다(`body-page-gap/wasm-build.log`).
WASM lib Clippy도 PASS(`body-page-gap/clippy-wasm.log`). 새 WASM SHA256은
`81cd4e66145b8adef7e78d4cc400e069aba21f5a7010f19c08890cd3439038e8`이다.
`node output/7353/r19/body-page-gap/capture.mjs --wasm`을 실행해 실제 브라우저 WASM의
DocumentV2 결과를 생성했다. Native/WASM 숫자·기타 차이0, 전체4쪽 SVG 동일이다
(`body-page-gap/review/backend-comparison.json`, `run.json`). Canvas 직접 paint가 아닌
WASM 생성 SVG의 Chrome 출력임을 구분한다.

`body-page-gap/review/wasm-review-4.png`, `wasm-overlay-4.png`, `wasm-review-3.png`를
직접 열어 확인했다.4쪽 붙임 첫 줄의 세로 시작과 두 줄 간격은 기준 PDF에 대응하며,
3쪽 표 외곽과 뒤 문단은 보존된다. 기존3쪽 별표 문단의 정렬·줄바꿈 및 대체 글꼴 외형
차이는 남으므로 전체 원본 일치로 판정하지 않는다. 전후 전체 노드의 내용·순서·소유·크기
보존과1–2쪽 이동0,3쪽 이동−0.08px,4쪽 이동−8.533333px는
`body-page-gap/placement-preservation.json`에 별도로 기록했다(정식 테스트 건수에 미포함).
source HEAD `50823731af6050c60ec3cfcc898abf36daa6b35a` + WIP이며
`body-page-gap/source.sha256`의 production36파일 일치 및 diff check를 확인했다.
검증 뒤 production 변경 없음. 메인테이너가 페이지 시작 간격 수정의 시각 판정을 통과시키고
다음 절편을 승인했다. 이 승인은 아래에서 발견한 다른 앵커 경계의 무회귀까지 뜻하지 않는다.

#### B/C 진행 — TAC 표 사이 저장 LEFT 탭의 원문 슬롯과 가로 간격

**원인/공통 결과.** `tac_spaces::compose`가 탭을 허용하지 않아 정상 한컴 저장본의
두 결재표 문단을 거부했다. 저장 LEFT 탭의 확정 advance799HU를 기존
`compute_char_positions`로 읽고 동일 `layout_positions`를 측정·paint가 소비하도록 했다.
원문 슬롯은 일반 공백1과 탭8을 구분하며 `tac::object_rows`가 coverage를 검사한다.
또한 `Paragraph::control_utf16_positions`의 마지막 문자 끝/이전 문자 끝 계산은 탭을
UTF-16 1칸으로 세고 있었다. 기존 공통 `char_stream_len`을 사용하여 탭 뒤 표의 source
시작을25가 아닌32로 복원한다. parser 속성이나 원문을 임의로 바꾸지 않는다.
저장 마지막 줄의 Justify는 분배하지 않는 기존 문단 정렬 의미로 두 표를 배치한다.

수용 범위는96dpi, 한 저장 줄의 유효한 LEFT 탭1개이며 RIGHT/leader/무효 advance는
거부한다. fresh tab-stop solver나 가시 문자열 혼합 TAC로 범위를 확장하지 않았다.
`issue_7353_table_v2_document_flow`에 신규3계약을 추가했다. HWP/HWPX의 body/cell,
네 정렬, 뒤 내용의 단일 보존, 거부 대조군 및 정상 한컴 저장본의 두 표 좌표를 검사한다.
모델 원문축 변경의 Legacy 소비 경계도 별도7계약으로 확인했다.

**입력과 독립 기준.** `issue7353_tac_tab_review/README.md`에 source·발췌 절차·정상
저장 HWP와 동일 입력의 한컴 PDF·MCP job·해시를 고정했다. 원본 첫 부모 표 첫 셀의
문단을 분리 후 한컴으로 정상 저장한1쪽 대조군이다. 원본 전체 피델리티의 증거가 아니다.
표 왼쪽 원점은 source의 body5385HU, 첫 표폭18921HU, 좌우 margin140HU 및
tab799HU로 각각73.666667/340.333333px다. 같은 HWP를 이전 Native와 이전 WASM
(`81cd4e66…`)에 넣으면 qualified space advances 거부, 수정 Native는 두 표를 출력한다
(`tac-tabs/saved-before.log`, `before-wasm.json`, `after/native.json`).
초기 raw 발췌본은 cell→body 이동 후 저장 폭이 맞지 않아 거부되며 탭 결함 검출로 세지 않는다.

원본 전체의 paragraph22 field-end 거부는 조판 전 입력 검사다. 이를 근거로 앞 부모 표가
통과했다고 할 수 없다. parent-prefix도 stored frame 미지원으로 남는다. 해당 입력·실패를
`tac-tabs/parent-prefix.hwpx`, `parent-after.log`, `original.log`에 보존했다.

**기존 실패 정정/분리.** 이번16 suite는292 PASS/2 FAIL이며 신규3계약은 모두 PASS다
(`tac-tabs/tests-final.log`). 직전 같은 suite는289 PASS/2 FAIL이었다. 누락했던 실패는:

- `nonterminal_tac_gap_and_authored_blank_still_advance_the_body`: 예전 기대값은 잔여
  following-line gap8px를 다음 쪽에 이월한다. 승인된 새 정책에서는 실제 빈 줄이 다음
  본문 상단30px, 뒤 문단48px에 있다. 계약 기대값과 정책의 충돌로 분류하며 이 절편에서
  assertion을 바꾸거나 ignore하지 않았다.
- `stored_anchor_gap_crossing_page_uses_deferred_origin_without_duplicate_host`: 실제
  좌표 결함도 확인했다. host가y30px이고 offset9+margin6px이면 표 원점은45px여야
  하지만33px다. `BodyAnchor::resolve`가 비절단 host.next_origin84에서 만든
  tail_overlap69를 `body_flow`가 페이지 끝으로 줄어든pen72에서 빼기 때문이다.
  생산 결과→예약 소비에서12px가 어긋난다. `tac-tabs/boundary/anchor-gap-boundary.native.json`
  에 실행 노드를 보존했다. 탭 변경 전에 이미 실패하던 별도 경계이며 다음 수정 우선 대상이다.

따라서 이번 상태를 전체 회귀 통과로 보고하지 않는다. 기준값/ignore/Legacy 기본 엔진은
변경하지 않았다. native build, native/WASM lib Clippy 통과; 전체 CI/workspace lint는
미실행이다. fresh Docker WASM 및 시각 비교 결과는 아래에 연결한다.

**최종 증적.** Docker 표준 빌드는7m19s, 종료코드0으로 완료했다
(`tac-tabs/wasm-build.log`). WASM SHA256은
`a47da5aadf68477a3439b95a2d7fccf624548732b97898b355ca3570e6e5fe60`이다.
`node output/7353/r19/tac-tabs/capture.mjs --wasm`은 첫 Chrome 시작 실패 후 같은 명령
재실행으로 성공했다. Native/fresh WASM 숫자·기타 차이0,1쪽 SVG 동일이다
(`review/backend-comparison.json`, `run.json`). 실제 WASM 생성 SVG의 Chrome 출력이며
Canvas 직접 paint 검증은 아니다. `review/wasm-review-1.png`와
`review/wasm-overlay-1.png`를 직접 열어 두 표 외곽·간격·셀 보존을 확인했다.
대체 글꼴 굵기·글립 세부 위치 차이는 남는다. 전체 원본 조판의 승인 자료로 사용하지 않는다.

추가 Legacy7계약 PASS(`tests-additional.log`), 기존 body-field5계약 PASS
(`body-field-contract-results.log`)로 중복을 제외한 최종 **304 PASS/2 FAIL**이다.
body-field 첫 직접 harness는 zip extern 누락으로 빌드 실패했으며 기존 전용
`body-end/run-contract.sh`로 재실행했다. 환경 실패를 결함 재현으로 세지 않는다.
추적용 capture를 추가한 document_flow도136 PASS/2 FAIL로 재확인했다.
변경 Rust fmt, diff check, production37파일 source manifest 일치 확인.
검증 head는`50823731af6050c60ec3cfcc898abf36daa6b35a`+WIP다. 빌드 뒤 production 변경 없음.
다음 우선 절편은 위 앵커의 host 원점/소비한 following gap 좌표 일치와 관련 계약 정리다.
commit/push/PR/이슈 등록은 수행하지 않았다.

#### B/C 진행 — 페이지에서 소비한 줄간격과 표 앵커 원점 분리

메인테이너가 앞 TAC 탭 절편의 시각 판정을 통과시키고 다음 절편을 승인했다.
이번에는 그때 남긴 두 실패를 처리한다. 신규 서식 기능이나 Legacy 표 경로로 범위를 넓히지 않는다.

**규칙과 실제 호출 경로.** 문단 상대 표의 위치는 선언 문단의 점유 줄과 원문 offset/바깥여백에
귀속된다. 페이지 끝에서 following-line gap이 덜 소비되었다고 그 표를 위로 이동시키지 않는다.
기존 `BodyAnchor::resolve`는 `host.next_origin()`에서 구한 tail_overlap을 반환하고,
`BodyCursor::fit`은 페이지 끝으로 줄어든 story pen에서 그 값을 빼고 있었다.
합성 경계의 host12+gap72=84, object top15는 overlap69가 되지만 실제 pen72에서 빼면
3이 되어 source의15보다12px 작았다. 앞 절편에서 기록한 실제 y33/기대45의 원인이다.

이제 `BodyAnchor::resolve`는 `top - host.occupied_end()`를 initial band로 생산한다.
`document_input`이 이를 `AnchoredFlow.initial`의 Space+Table로 전달하고,
`BodyCursor::fit`이 실제 수용된 host `LinePlacement.bounds`의 끝에서 예약한다.
같은 line placement가 `DocumentV2Session`의 paint에도 쓰인다. 선언 줄간격이나 가시 문자열로
원점을 다시 추측하지 않는다. `anchor.fit`→`FlowCursor`가 컷/실제 요구 높이와 아래여백을
예약한 결과를 `BodyFit::accept`가 수용하고, story pen은 이미 소비한 점유보다 후퇴하지 않는다.

원 host가 없는 새 페이지(물리 paragraph-after가 이월된 경계)는 deferred frame의 위여백만
적용한다. 첫 조각이 안 들어가면 initial band를 commit하지 않고 기존 pending queue로
넘긴다. 이미 분할 수용한 표는 기존 cursor/cut와 restart_top으로 이어받는다. continuation의
컷·제목 반복·rowspan·cell clip·표 외곽 paint는 변경하지 않았다. 일반 TAC/셀 앵커/어울림의
원점 생산 경로에도 이번 변경은 적용되지 않는다.

**정식 계약.** 기존 잘못된 간격 이월 기대값38/56은 승인된 body-gap 정책과 실제 빈 줄의
12px 높이·18px pitch에 따라30/48로 정정했다. 물리 paragraph-after는 별도 계약으로 계속
보존한다. 앵커 테스트는 페이지 수만 바꾸지 않고 독립 source offset으로 y45를 검사한다.
수정 전 새 기대값과 gap/빈 문단 경계 둘 다33≠45로 실패했다(`anchor-gap-origin/tests-before.log`와
연결된 suite 로그). 수정 후에는 다음 경계를 추가해 문서 흐름142 PASS다.

- 빈/가시 host, 앞 문단 유무, gap6/60/72/144px, HWP/HWPX에서 같은 상대 원점.
- gap이 잘려도 RowBreak 표의 A/B/C→D 컷, 본문 하단·위여백·남은 내부 간격·뒤 문단 보존.
- paragraph-after80px가 이월되면 잔여26px를 보존하고 source offset을 반복하지 않음.
- 정상 한컴 저장본의 제목·두 표 행·2쪽 후속 문단의 실제 최종 좌표와 종료.

`tests-final-split.log`142 + `tests-other.log`156 +
`body-end/contract-anchor-origin.log`5 = **303 PASS/0 FAIL**.
이번 선택 집합의 실행 수이며 이전 절편의 별도 source-axis7건을 재실행으로 합산하지 않았다.
초기 after 진단은 Native build 완료 전에 기존 rlib에 연결되어 before와 같은 실패를 반환했다.
`tests-built.log` 이후는 완료된 새 rlib 기준이다. 새 실물 계약의 첫 텍스트 assertion은 한컴의
run 분할을 문단 분할로 잘못 가정해 실패했으므로 run 결합+실제 TextLine3개 검사로 정정했다.
이 두 진단 실패를 production 결함이나 최종 통과 수에 섞지 않았다.

**독립 시각 증거.** `tests/fixtures/issue7353_anchor_gap_review/README.md`에 생성 절차와
입력/한컴 저장/PDF 해시·job ID를 남겼다. 제목에 긴 줄간격과 문단 상대 표를 둔 독립
대조군이며 원본 발췌가 아니다. 같은 정상 저장 HWP에서 이전 Native 및 WASM은 첫 페이지
`InconsistentAtomicPlan`으로 실패한다(`before.log`, `before-wasm.json`).
수정 Native는2쪽을 생성한다. PDF 표 위/아래117.130667/179.217333px,
rhwp117.16/179.186667px로 약0.03px 차이다. 제목 기준선 차이는 약0.05px다.
1–2쪽 review를 직접 열어 제목→표 거리·두 행·뒤 문단의 페이지 시작과 무중복을 확인했다.
대체 글꼴 잉크 위치·폭·굵기는 여전히 다르다. 초기 용지 방향이 모순인 생성본은 정상 입력으로
간주하지 않고 별도 보존했으며 수정 과정은 fixture README에 공개했다.

**대조군/검증 범위.** 이전/현재 Native로 #6601 원본4쪽, 기존 defer2쪽, 탭1쪽을 비교해
모든 노드와 SVG 동일을 확인했다. 기존 anchor split2쪽은 최대2.84e-14px의 부동소수 차이,
내용 차이0이며2쪽 SVG는 동일하다(`controls-comparison.json`). 첫 쪽 SVG 문자열 차이는
이 수치 변화이며 새로운 행/경계 이동이 아니다. source HEAD50823731…+WIP,
`anchor-gap-origin/source.sha256`37파일로 고정했다. Native build28.06s,
native lib Clippy26.24s/WASM lib Clippy31.74s, 변경 Rust fmt/diff check PASS.
전체 CI·workspace all-target lint는 이번 내부 절편에서 실행하지 않았다. baseline/ignore,
Legacy 기본값·원본 샘플 비변경. fresh Docker WASM 최종 증적은 아래에 기록한다.

**fresh WASM 완료.** `docker compose --env-file .env.docker -p rhwp run --rm wasm`
종료0, wasm-pack7분16초(`anchor-gap-origin/wasm-build.log`). WASM SHA256은
`5be1cab6d0cc4d4ca7cd8a0f59fc5c866304e222a94f91319166aaadea36beb9`다.
`capture.mjs --wasm`, `capture-control.mjs --wasm`으로 새 바이너리를 브라우저에서 실행했다.
새 경계 대조군과 기존 anchor split 각각2쪽에서 Native/WASM 노드 차이0, SVG 모두 동일이다
(`review/backend-comparison.json`, `control-review/backend-comparison.json`). 각 `run.json`에
source/input/PDF/WASM 식별 정보를 고정했다. 이는 브라우저 WASM DocumentV2→SVG 경로이며
별도 Canvas paint 전체 검증을 의미하지 않는다.

직접 확인한 대표 이미지는 `anchor-gap-origin/review/wasm-review-1.png`,
`wasm-overlay-1.png`, `wasm-review-2.png`와
`anchor-gap-origin/control-review/wasm-overlay-2.png`다. 새 대조군의 제목→표 원점과
두 행 외곽, 다음 쪽 후속 문단을 확인했고, 기존 분할 대조군의 이어받기 외곽·후속 문단도
유지된다. 남은 글꼴 잉크 차이는 위 범위 설명과 같다. 메인테이너가 이번 새 대조군의
시각 판정 통과와 다음 절편 진행을 승인했다. 커밋·push·PR·이슈 등록은 하지 않았다.

#### B/C 진행 — 본문 텍스트 말미의 문단 간 누름틀 종료

다음 원본은 앞 TAC 탭 작업의 `samples/issue2470/36382471_masked.hwpx`다. 본문22의
“붙임 … 끝.”27문자 뒤 종료8슬롯과 문단 끝1슬롯(char_count36), 시작 문단2의
ClickHere ID1553175006/ctrl627272811을 확인했다. 기존 `body_ends`는 빈 종료 문단만
수용하여 전체 조판 이전에 거부했다. 문자 뒤의 구조 종료를 가시 객체/빈 문단으로 바꾸지 않는다.

변경은 `fields::body_ends`의 소유 증명이며 저장 줄·실제 점유/분할 알고리즘은 바꾸지 않는다.
문자 offset의 연속 UTF-16 축(탭8슬롯), 말미 종료의 scalar 경계, char_count와 열린 시작의
역순 ID를 확인한다. 이 증명을 `document_input`→`compose_text_with_body_end`가 소비하고
기존 stored localization/공통 composer의 동일 TextLine이 FlowCursor의 점유와 실제 paint에
쓰인다. 종료 슬롯을 없애거나 줄 메트릭을 재계산하지 않는다. 텍스트 중간/시작의 종료,
짝 불일치/교차/중복, 편집 후 무효 캐시는 이 확장의 수용 대상이 아니다.

수정 전 신규 정식 경계는 `unqualified body field end`로 실패하고 기존5건은 통과했다
(`body-end/contract-trailing-before.log`). 이 합성 경계는 문자·공백·UTF-16 surrogate와
독립 저장1600HU 줄 높이/앞 문단의2700HU pitch를 검사하며 정상 한컴 시각 증거와 구분한다.

**실행 결과.** `body-end/contract-trailing-complete.log`8 PASS,
`body-field-tail/tests-selected.log`12 suite286 PASS로 **294 PASS/0 FAIL**이다.
문자/공백/surrogate 말미 종료, 중첩 종료의 역순 및 교차 거부, LF 두 줄과 뒤 문단의
독립 pitch, 정상 한컴 저장 HWP의2쪽 두 줄과 종료를 실제 최종 노드에서 검사했다.
앞 절편의 Legacy12건은 이번 변경에서 재실행한 수에 합산하지 않았다.

합성 다중 줄 테스트를 처음 HWP/HWPX round-trip으로 만들었을 때 원본 그림의 직렬화
경계와 축약된 Field-only host/section 진입이 현재 수용 범위와 충돌했다. 이를 production
완화로 통과시키지 않고 원본 HWPX의 종료 문단만 수정하는 명시적 합성 계약으로 분리했다.
중간 실패 로그는 `body-end/contract-trailing-multiline*.log`에 보존하며 한컴 HWP round-trip
검증으로 세지 않는다. HWP 증거는 별도로 정상 저장한 실물 대조군 계약에서 확인했다.

**독립 대조군.** `tests/fixtures/issue7353_body_field_tail_review/README.md`에 생성 절차,
동일 HWP→PDF 출처·job ID·해시·저장 메트릭을 남겼다. 기존 승인된 누름틀+표지 대조군에
종료 문장과 뒤 문단을 추가한2쪽 문서로, 원본 #2470의 전체 일치 증거가 아니다.
동일 정상 저장 HWP가 이전 정적 Native에서는 index2 `unqualified body field end`로
거부되지만 현재는2쪽 출력된다(`body-field-tail/before.log`, `after.log`). Native1–2쪽
review를 직접 열어 표지 보존, 종료 문장·후속 문단의 위치와 무중복을 확인했다.
대체 글꼴의 잉크 외형 차이는 남는다. PDF/저장 메트릭과 V2 좌표는 fixture README에 연결한다.

**대조군과 남은 범위.** #6601 원본4쪽, 기존 anchor split2쪽/defer2쪽, TAC 탭1쪽을
전후 비교해 모든 노드와 SVG가 동일했다(`body-field-tail/controls-comparison.json`).
#2470 원본은 종료 소유 검사를 통과한 뒤 paragraph0의
`stored TAC carrier requires unambiguous intact rows`에서 거부된다(`original.log`).
앞서 보존한 parent-prefix의 동일 경계이며 다음 종단 연결 대상이다. 원본 전체 수용/일치를
완료했다고 하지 않는다. Legacy 기본값, baseline/ignore, 원본 파일은 변경하지 않았다.

Native build23.31s, native/WASM lib Clippy27.93/32.29s, 변경 Rust fmt/diff check PASS.
source HEAD50823731…+WIP와37파일 manifest(`body-field-tail/source.sha256`)를 고정했다.
전체 CI/workspace all-target lint는 이번 내부 절편에서 미실행이다. 최종 fresh Docker WASM
증적은 빌드 완료 후 아래에 연결한다.

**fresh WASM/직접 판독.** 표준 Docker 빌드는7분16초, 종료0으로 완료했다
(`body-field-tail/wasm-build.log`). WASM SHA256은
`b0c4529887afc7dc414380df2f5cd677856b2eeec7a5d25f92c40eb6168d9603`다.
`node output/7353/r19/body-field-tail/capture.mjs --wasm`과 `detail.mjs`를 실행했다.
동일 저장 HWP의2쪽 전체에서 Native/fresh WASM 노드 차이0/SVG 모두 동일이다
(`review/backend-comparison.json`). source37파일, 입력/PDF/WASM 해시는 `review/run.json`에
연결했다. 이는 실제 Chrome의 WASM DocumentV2→SVG 출력이며 Studio Canvas 직접 paint
및 편집 경로 검증은 아니다.

`review/wasm-overlay-1.png`와 `wasm-detail-2.png`를 직접 열어 표지 외곽 보존,
종료 문장·뒤 문단의 위치와 무중복을 확인했다. 상세 이미지는 같은2쪽 y65~215px만 표시하며
원점 정합 이동은 하지 않았다. 전체 compare/review/standalone overlay1–2쪽도 보존했다.
남은 글꼴 외형 차이는 fixture README와 같다. 메인테이너가 이번 절편의 시각 판정 통과와
다음 절편 진행을 승인했다.
Rust 변경 뒤 다시 생성한 출력이며, 빌드 이후 production 변경 없음과 manifest 일치를 확인했다.
커밋·push·PR·이슈 등록은 수행하지 않았다.

#### B/C 진행 — 셀의 서로 다른 높이 TAC 그림과 공백 전진

앞 누름틀 종료 절편의 메인테이너 시각 통과를 반영했다. #2470 원본의 다음 거부를
추적한 결과, 문단0의 부모 TAC나 첫 셀의 두 표가 아니라 **부모 두 번째 셀 첫 문단의
그림2개+공백58개**가 원인이었다. 부모69128HU 줄은 선언68562HU+위아래283HU와
일치하고 첫 셀 TAC 탭도 통과한다. 공통 오류 문구가 이 경로를 구별하지 못했다.
임시 호출 진단은 제거했으며 `body-field-tail/parent.log`, `parent-trace.log`에 경계를
보존했다. 부모 표의 높이·여백·분할 조건은 변경하지 않았다.

**규칙과 공통 결과.** 가시 잉크가 없는 공백/탭도 원래 source 슬롯·가로 전진을 소유한다.
서로 다른 높이의 TAC 그림은 동일 줄의 기준선에 배치하며, 모든 그림이 저장 줄 높이와
개별적으로 같을 필요는 없다. 기존 TableBand와 저장 줄 높이를 대조하여 전체 점유를
검증한다. 근거는 아래 정상 한컴 저장 대조군의 실제 두 그림 위치다.

`tac_spaces::compose`의 source 위치/폭 → `tac::object_rows`의 그림 rect와 공백 rect →
`pictures::compose`의 단일 TextLine/`ObjectRow.bounds`가 공통 결과다.
`ir.rs`의 ObjectRow 분기는 같은 bounds.height로 FlowBlock::Lines를 만들고,
`flow.rs`의 Lines fit은 전체 줄이 예산에 맞지 않으면 유닛을 소비하지 않는다.
`text_ir::record_items/bind_paint`는 동일 line payload를 최종 줄 원점으로 이동해 그린다.
그림마다 별도 높이를 합하거나 paint에서 높이/앵커를 덮어쓰지 않는다. 공백만 있는 선행
줄도 Lines로 소유하고 이미지 줄과 분리해 보존한다. 그림 변환/리소스 guard는 유지한다.
그림의 문단 프레임도 기존 TAC physical_frame 검사를 함께 소비한다.

**수정 전후/반례.** `tests/cases/issue_7353_table_v2_export.rs`의 신규 혼합 줄 검사는
변경 전 명시적 미지원으로 FAIL, 기존29건은 PASS였다
(`frame-after-child/picture-space-before-issue_7353_table_v2_export.log`). 최초 테스트
작성 시 배열 길이/helper 누락으로 발생한 컴파일 오류는 재현 성공으로 세지 않는다.
수정 뒤 좌/중앙/우 정렬, 저장1500HU 탭, 높이1800/1200HU의85% 기준선,
24px가 fit/23px만 남아 다음 쪽으로 전부 이월되는 경계, 앞의900HU 공백 줄,
그림2개/탭1개/뒤 문단1회의 최종 좌표를 검사했다. 저장 줄 자체보다 큰 그림과
잘못된 source/리소스/변환/visible text의 기존 거부 검사도 유지한다.
기존 “작은 그림 하나만1HU 낮추면 거부”는 잘못된 동등 높이 가정이므로,
“저장 줄을 가장 큰 그림보다1HU 작게 만들면 거부”로 의미를 수정했다.

**독립 시각 대조군.** `tests/fixtures/issue7353_picture_space_review/README.md`에
생성 코드, HWP→PDF job/해시, 저장 줄과 PDF 그림 위치를 기록했다. 기존 정상 그림
대조군의 자식 셀에 작은 그림과 탭을 추가하고 한컴이 새로 저장한1쪽 문서다.
이 입력은 이전 정적 Native에서 동일 미지원으로 거부되고 현재는1쪽 출력된다
(`picture-space/before.log`, `after.log`). 원본을 수정하거나 이 대조군을 원본 일치로
보고하지 않는다. Native review/overlay를 직접 열어 두 그림·탭 간격·상대 높이,
부모/자식 외곽, CELL BEFORE/AFTER와 뒤 본문을 확인했다. PDF의 인쇄 양자화와
대체 글꼴 잉크 차이는 남으며 큰 위치·내용 누락은 관찰하지 않았다.

**검증.** 출력 계약32건과 나머지6 suite217건, 앞 누름틀 종료8건으로 **257 PASS/0 FAIL**.
로그는 `picture-space/tests-boundary-final.log`, `tests-selected.log`,
`body-end/contract-picture-space.log`다. 중복 실행한 export31건은 다시 합산하지 않는다.
정상 저장 HWP와 그 IR의 HWPX에서 그림/부모/자식/앞뒤 문단의 좌표와 종료도 검사했다.
기존 그림1쪽, TAC 탭1쪽, 누름틀 종료2쪽, #6601 원본4쪽은 변경 전후 모든 노드/SVG가
동일하다(`picture-space/controls-comparison.json`). 최초 진단 probe의 불필요한 JSON
재파싱에서 생긴 약1e-13 반올림 차이는 engine 원문을 저장하는 probe로 제거했다.

Native build13.41s, native/WASM lib Clippy30.15/30.41s, 변경 Rust fmt/diff check PASS.
전체 CI/workspace all-target 검증, Studio Canvas 편집 경로는 이번 내부 절편에서 미실행이다.
원본 #2470은 이제 `V2 stored picture appearance or resource`에서 거부된다
(`picture-space/original.log`). 해당 그림의 shape offset/변환을 별도로 추적해야 하며,
원본 전체 수용이나 R5 완료는 아니다. Legacy 기본값·baseline·ignore·원본은 변경하지 않았다.
**fresh WASM/직접 판독.** 표준 Docker 빌드는7분19초, 종료0이다
(`picture-space/wasm-build.log`). WASM SHA256은
`f50168e49c3aa1c5f33a41cada7f84fc88ebcb275bb8a953d93036e4487b4ba8`이다.
`node output/7353/r19/picture-space/capture.mjs --wasm`으로 동일 정상 저장 HWP를
브라우저에서 실행했다. Native/fresh WASM1쪽의 숫자·기타 노드 차이0, SVG 동일이다
(`review/backend-comparison.json`). source HEAD50823731…+WIP의37파일 manifest와
입력/PDF/WASM 해시는 `review/run.json`에 연결했다. 초기 Chrome 실행은 실패했고
검증으로 세지 않는다. `--disable-dev-shm-usage --disable-gpu`를 추가한 실제 브라우저
실행으로 compare/standalone overlay/review를 모두 새로 생성했다.

`review/wasm-review-1.png`와 `wasm-overlay-1.png`를 직접 열어 두 그림의 상대 높이와
간격, 표 외곽 및 후속 문단을 확인했다. 남은 PDF 인쇄 양자화/글꼴 잉크 차이는 위 범위와
같다. 메인테이너가 이번 절편 시각 판정 통과와 다음 절편 진행을 승인했다. 검증 뒤 production 변경 없음과
source manifest 일치를 확인했다. 별도 Canvas paint/편집은 미검증이다.
커밋·push·PR·이슈 등록은 하지 않았다.

#### B/C 진행 — 빈 그림 참조의 프레임 보존

앞 그림/공백 절편의 메인테이너 시각 통과를 반영했다. #2470의 두 그림은
`binaryItemIDRef=""`, BinData 파일 없음, IR ID0/외부 경로 없음이다. 파싱 중 정상
리소스를 잃은 경우가 아니다. 기존 `picture_footnote.rs`의 #2225 MissingPicture 규칙과
`serializer/hwpx/picture.rs`의 #1567 빈 참조 보존 규칙을 따른다. 원본에 동봉된 PDF는
로고가 보여 현재 마스킹 입력과 내용이 다르므로 이번 절편의 기준 출력으로 쓰지 않았다.

**규칙/공통 결과.** 빈 참조도 글줄에 참여하는 그림 프레임이다. 인쇄에서 잉크가
없다는 이유로 높이·너비·source 소유를 제거하지 않는다. `tac::object_rows`의 rect →
`pictures::compose`의 동일 TextLine/ObjectRow.bounds → `ir.rs`의 Lines 높이 →
`flow.rs`의 원자적 fit/이월 → `text_ir`의 같은 payload 배치 경로를 유지하고,
그림 payload만 기존 `Placeholder(MissingPicture)`로 만든다. `svg.rs`는 인쇄 profile에서
이 노드의 잉크를 내보내지 않는다. 프레임 원점/높이의 후속 재계산은 추가하지 않았다.
분할·rowspan·컷 알고리즘 변경은 비해당이며 줄 전체가 fit하지 않을 때 소비하지 않는
기존 경계를 실행했다. V2 읽기 전용 snapshot에 가짜 편집용 DocumentCore 주소를 넣지 않는다.

비영 group-local offset은 ungrouped 빈 그림의 픽셀 변환이 아니므로 그 프레임 위치에
더하지 않는다. 실제 이미지에는 기존 offset guard를 유지한다. nonzero 리소스 누락,
외부 경로, 회전, 그룹, 행렬 이동은 빈 참조라는 이유로 허용하지 않는다.

**독립 기준/전후.** `tests/fixtures/issue7353_missing_picture_review/README.md`에 생성
코드와 한컴 정상 저장 HWP/PDF job·해시·관측값을 연결했다. 왼쪽 빈 프레임과 오른쪽
정상 그림이 있는 중첩 표이며, 한컴 재저장에도 ID0와 `(3745,-1509)` group-local offset이
그대로 남는다. 정상 그림은 이전 두 그림 대조군과 같은 위치에 인쇄된다.
변경 전 신규 합성 계약은 `V2 stored picture appearance or resource`로 FAIL, 기존32건은
PASS(`frame-after-child/missing-picture-before-issue_7353_table_v2_export.log`).
이전 정적 Native도 정상 저장 대조군을 같은 이유로 거부했다(`missing-picture/before.log`).
생성기의 최초 잘못된 control index로 인한 panic은 수정했으며 결함 재현으로 세지 않는다.

변경 후 합성24px 줄의 fit/1px 부족 이월, 빈 프레임·정상 그림의 최종 좌표, 뒤 문단
유일 출력, 인쇄 SVG의 정상 그림만 출력, 정상 HWP/HWPX의 부모·자식 외곽과 종료를
검사했다. 집중7suite266건과 누름틀 종료8건으로 **274 PASS/0 FAIL**이다.
`missing-picture/tests-selected.log`, `tests-boundary-final.log`,
`body-end/contract-missing-picture.log`가 결과이며 export34건의 재실행은 중복 합산하지 않는다.
정상 대조군5개(그림1개/그림2개/탭/누름틀/#6601)는 전후 모든 노드·SVG가 같다
(`missing-picture/controls-comparison.json`).

Native review/overlay를 직접 열어 빈 자리와 오른쪽 그림, 부모·자식 외곽·앞뒤 문단의
위치를 확인했다. 새 대조군의 PDF 인쇄 양자화와 글꼴 잉크 차이는 남는다.
원본은 이제 문단0의 그림 경계를 통과하고 문단2의
`SQUEEZE requires intact stored text rows`에서 거부된다(`missing-picture/original.log`).
이는 다음 종단 연결 대상이며 원본 전체 수용/R5 완료를 뜻하지 않는다.
Legacy 기본값·baseline·ignore·원본은 변경하지 않았다.
Native build14.42s, native/WASM lib Clippy30.10/30.44s, 변경 Rust fmt/diff check PASS.
전체 CI/workspace all-target 검증과 Studio Canvas 편집 경로는 미실행이다.
**fresh WASM/직접 판독.** 표준 Docker 빌드7분28초, 종료0
(`missing-picture/wasm-build.log`). WASM SHA256은
`11fda6e1064d37cf3705193ec314bb362a89c44e541e24680b9c937b97b8a1a1`이다.
`node output/7353/r19/missing-picture/capture.mjs --wasm`으로 동일 정상 저장 HWP를
실제 Chrome DocumentV2에서 실행했다. Native/fresh WASM1쪽의 노드 차이0, SVG 동일
(`review/backend-comparison.json`). source HEAD50823731…+WIP37파일 manifest와
입력/PDF/WASM 해시는 `review/run.json`에 보존했다. 검증 뒤 production 변경 없음과
manifest 일치를 확인했다.

`missing-picture/review/wasm-review-1.png`와 `wasm-overlay-1.png`를 직접 열어 왼쪽 빈
프레임 공간, 오른쪽 정상 그림, 부모/자식 외곽, CELL BEFORE/AFTER 및 뒤 본문 위치를
확인했다. PDF 인쇄 양자화와 글꼴 잉크 차이는 남고 메인테이너가 시각 통과와 다음 진행을 승인했다.
별도 Canvas paint/편집은 미검증이며 커밋·push·PR·이슈 등록은 하지 않았다.

#### B/C 진행 — SQUEEZE 셀 첫 문단의 정상 1단 정의

앞 빈 그림 절편의 메인테이너 시각 통과를 기록했다. 다음 원인은 원본 #2470의
`s0:p2/t1/c0/p0` 제목 문단이다. SQUEEZE 셀이고 저장 줄1개, dirty=false,
높이2700HU/폭42076HU가 유효하지만 `controls=[ColumnDef(normal,1)]` 때문에
`stored_text::validate_cell_wrap`에서 거부됐다. 이 컨트롤은 인라인 점유 개체가 아니라
셀 story의 단 선언이다. 저장 줄이 잘못됐거나 글자가 넘쳤다는 근거로 해석하지 않는다.

**호출 경로/범위.** `ir::initial_cell_column`이 첫 문단·첫 source 슬롯·정상1단을
검증하고 `bind_table`이 column break 의미와 소유 상태를 처리한다. 그 뒤
`IrTextComposer::compose_with_cell_wrap`의 SQUEEZE 경로에서 이 선언만 제거한
읽기 전용 text view를 만든다. 실제 Document IR/char_offsets/LineSeg/char_count는
수정하지 않는다. 기존 BREAK 경로의 동일 구조 해석을 사용하되 SQUEEZE 플래그는 보존한다.
`TextComposer::compose_with_cell_wrap → compose_text`의 단일 줄 결과를
`record_items`와 `bind_paint`가 함께 소비한다. 너비/기준선/높이를 별도로 추측하거나
paint에서 덮어쓰지 않는다. 분할·이어받기 알고리즘은 비해당이며 기존 소유 유닛은 그대로다.
다단/후속 문단 단 정의는 상위 IR 검증에서 거부하고, 다른 컨트롤·dirty·없는 저장 줄은
기존 SQUEEZE 검증을 우회하지 않는다. 글리프 축소 알고리즘은 변경하지 않았다.

**독립 기준/전후.** `tests/fixtures/issue7353_column_squeeze_review/README.md`에
정상 저장 생성 코드, HWP/PDF job·해시·기대값을 기록했다. 기존 SQUEEZE 대조군의
자식 첫 문단에 정상1단 선언만 추가하고 한컴에서 줄 메트릭을 다시 저장한 파일이다.
새 PDF와 선언이 없던 기존 PDF의96DPI 래스터는 동일하다. 이 비교와 별도로 Native
review/overlay의 한 줄 유지·셀 시작·외곽·뒤 문단도 확인한다.

정식 신규 source 슬롯 계약은 변경 전 명시적 미지원 FAIL/기존142 PASS
(`frame-after-child/column-squeeze-before-issue_7353_table_v2_document_flow.log`).
이전 정적 Native도 새 정상 저장 HWP를 같은 이유로 거부했다(`column-squeeze/before.log`).
변경 후 source 슬롯 전후 및 정상 HWP/HWPX의 전체 노드·SVG가 독립 plain-story 저장본과
같고, 다단·없는 저장 줄은 올바른 원인으로 거부한다. 집중7suite268건과 누름틀 종료8건,
**276 PASS/0 FAIL**. `column-squeeze/tests-selected.log`, `tests-boundary-final.log`,
`body-end/contract-column-squeeze.log`에 연결하며 document_flow144 재실행은 중복 합산하지 않는다.
기존 SQUEEZE/빈 그림/정상 그림/탭/누름틀/#6601 등6개 대조군의 모든 노드·SVG도 전후
동일하다(`column-squeeze/controls-comparison.json`).

Native build19.66s, native/WASM lib Clippy30.32/29.37s, 변경 Rust fmt/diff check PASS.
원본은 이제 이 제목 문단을 통과하고 같은 표의 하단 `ThinThickDouble` 테두리에서
`V2 cell border style`로 거부된다(`column-squeeze/original.log`, `next-borders.log`).
원본 전체 수용/R5 완료는 아니다. 기존 SQUEEZE glyph 외형 차이는 이번에 변경하지 않았고
한컴 glyph와 완전 일치로 보고하지 않는다. baseline·ignore·원본·Legacy 기본값은 유지했다.
전체 CI/workspace all-target와 별도 Canvas 편집은 미실행이다.

**fresh WASM/시각 자료.** Docker 빌드7분24초, 종료0
(`column-squeeze/wasm-build.log`), WASM SHA256은
`10170a53f4d31a3eae0e8bd1396cee7d5c3d8ffb8f68355b0c5928f6fa2bb08f`다.
`node output/7353/r19/column-squeeze/capture.mjs --wasm`으로 실제 Chrome DocumentV2의
1쪽 Native/fresh WASM compare·standalone overlay·review를 생성했다. 노드 차이0/SVG
동일이며 `review/run.json`에 HEAD50823731…+WIP37파일 manifest, 입력/PDF/WASM 해시를
연결했다. production 고정 후 생성했고 검증 뒤 source manifest도 일치한다.

`column-squeeze/review/wasm-review-1.png`와 `wasm-overlay-1.png`를 직접 열어
ONE TWO THREE FOUR FIVE의 한 줄 유지·셀 시작과 안여백·부모/자식 외곽·뒤 문단을
확인했다. 기존 글리프 외형 차이는 이미지 상단에도 명시했다. 이번 구조 선언 처리의
메인테이너가 시각 통과와 다음 진행을 승인했다. 커밋·push·PR·이슈 등록은 하지 않았다.

#### B/C 진행 — 독립된 ThinThickDouble 셀 테두리

앞 SQUEEZE 절편의 시각 통과 후 원본 `samples/issue2470/36382471_masked.hwpx`의
root-p2 제목 표 하단선에서 막히던 `V2 cell border style`을 처리한다. 표 borderFill17은
Solid이지만 셀 borderFill6의 명시적 None/ThinThickDouble이 소유권을 가진다.
하단 폭index9는0.7mm다. 문서ID나 제목 문구에 따른 분기는 추가하지 않았다.

**규칙·독립 근거.** 표의 가는·굵은 이중선은 일반 shape 비율이나 같은 두 펜을 쓰는
Double과 다르다. 한컴 정상 저장 대조군에서 전체16굵기의 펜·중심 offset을 계측했다.
가장 가는 index0은 중심 단선이고, 나머지는 위/왼쪽 가는 선과 아래/오른쪽 굵은 선이다.
오른쪽/아래 변에서도 순서를 반전하지 않는다. index9는600DPI 단위로 선폭4/9,
중심offset-6/+4다. 다른 굵기도 임의 비율로 추측하지 않고 표준 폭 catalog에 대응하는
독립 관측표를 사용한다. `tests/fixtures/issue7353_thin_thick_review/README.md`에 생성
과정·HWP/PDF job과 해시·16종 기대값·판정 범위를 기록했다. 모든 저장 줄은 한컴이 만들었다.

**실제 소비 경로.** `CellBorders::prepare/resolve_edges`가 명시적 셀 edge를 보존하고
`CellBorders::append`가 수용된 `TablePlacement.cells.bounds`에서 동일한 물리 경계와
공유 edge union을 만든다. `borders/thin_thick.rs::append`는 이 경계로 최종 LineNode 두
펜과 ink_bbox만 생성한다. 측정/분할/이어받기 유닛·필요 높이·예약 높이·셀 원점은 바꾸지
않으며 실제 paint 뒤 좌표 덮어쓰기나 clamp도 없다. SVG와 WASM DocumentV2는 동일
RenderTree를 사용한다. 모서리·T/십자 교차 및 다른 collinear 스타일 접합, zone 이중선은
아직 검증되지 않아 명시적 Unsupported로 남긴다. 원본 제목의 독립된 하단선은 해당하지 않는다.

**전후 계약.** 새 네 방향 최종 좌표 검사는 변경 전 기존28 PASS/신규1 FAIL로
`V2 cell border style`을 검출했다(`frame-after-child/thin-thick-before-issue_7353_table_v2_borders.log`).
이전 정적 Native도 새 정상 저장 HWP를 같은 이유로 거부했다(`thin-thick/before.log`).
변경 후 정식 `tests/cases/issue_7353_table_v2_borders.rs`는16굵기·96/192DPI의
선폭/중심/ink bounds, 네 방향, 반복 제목/중첩 분할의 셀 외곽·내용·뒤 문단 보존,
교차점의 retryable 거부, 정상 HWP/HWPX 전달 경로와 종료를 검사한다.
신규 정상 저장 테스트는 cell의 자식이 아니라 소유 Table에 붙는 실제 LineNode를 검사한다.

border33 + split_borders11 + headers14 + export34 + document_flow144 + body-end8,
**244 PASS / 0 FAIL**. `thin-thick/tests-selected.log`의 초기 border32건 중 zone fixture
주소 오류는 셀 주소(병합 셀의 시작col0)로 바로잡았으며, 최종 border33건 결과는
`thin-thick/tests-boundary-final.log`로 대체한다. 구현 코드 보정으로 테스트를 맞춘 것이 아니다.
누름틀은 `body-end/contract-thin-thick.log`; SQUEEZE/빈 그림/정상 그림/기존 Double
4개 대조군의 전체 JSON/SVG 전후 동일은 `thin-thick/controls-comparison.json`이다.

Native build23.16s, Native/WASM lib Clippy28.86/30.85s, 변경 Rust fmt/diff check PASS.
원본은 p2 테두리를 통과해 root-p10의 `TAC content changed stored occupied box`까지
진행한다(`thin-thick/original.log`). 이 후속 제한은 이번 선 paint 절편에서 완화하지 않는다.
원본 전체 수용/R5 완료가 아니다. baseline·ignore·Legacy 기본 경로는 변경하지 않았다.
전체 CI/workspace all-target, Studio Canvas 편집은 이번 절편에서 미실행이다.

Native의 동일 HWP/PDF1쪽 review·standalone overlay와384DPI 가로/세로 확대 자료를
`thin-thick/review/`에 생성했다. 전체 표·뒤 문단·선의 방향/굵기를 직접 확인했으며 미세
좌표 반올림/래스터 및 대체 글꼴 차이는 남는다. catalog PDF2쪽은16종 펜 계측 자료이지
그 문서 전체 조판 일치 판정 자료가 아니다.

**fresh WASM 최종 증적.** Docker 빌드7분20초, 종료0(`thin-thick/wasm-build.log`).
WASM SHA256 `996e959e417f15e1d43bb618b4d50254c7917c5085f42aeb95f70e173cf23238`.
`node output/7353/r19/thin-thick/capture.mjs --wasm` 및 `highdpi.mjs --wasm`으로
Chrome DocumentV2의 동일 정상 저장 HWP1쪽을 다시 출력했다. Native/WASM 노드 차이0,
SVG 동일(`review/backend-comparison.json`). HEAD50823731…+WIP38파일 manifest와
입력/PDF/WASM 해시는 `review/run.json`에 연결했다. 빌드 이후 source manifest도 전부 일치한다.

`review/wasm-review-1.png`와 `wasm-overlay-1.png`를 직접 열어 선 방향·굵기·부모 외곽과
뒤 문단을 확인했다. 확대본은 `wasm-horizontal-review.png`, `wasm-vertical-review.png`다.
모서리/교차점은 계속 미지원이며 글꼴·미세 래스터 차이를 전체 일치로 보고하지 않는다.
이 절편은 메인테이너 시각 판정 통과이며 다음 진행을 승인받았다. 커밋·push·PR·이슈 등록은 하지 않았다.

#### B/C 진행 — 저장 줄 없는 본문 TAC의 실제 자식 높이 공유

ThinThickDouble 절편 시각 통과 후 원본 `samples/issue2470/36382471_masked.hwpx`의
root-p10에서 `TAC content changed stored occupied box`가 발생하는 원인을 추적했다.
이 문단에는 **저장 LineSeg가 없다**. common.height11565HU와 달리 세 셀에는 마스킹된
`*` 한 글자만 있으며 실제 계산 높이는1282HU다. `tac_fresh`가 선언 높이로 먼저 호스트
줄을 만들고, 이후 child plan의 실제 높이와 대조하면서 거부했던 문제다. 원본의 내용이
바뀐 사실과 저장 줄 유효성을 혼동하지 않는다. 마스킹 이전 PDF를 같은 내용의 정답지로
사용하지 않고 독립 대조군을 만들었다.

**규칙·공통 결과와 범위.** 저장 줄 없는 본문 문단은 자식 셀/내용의 계산 결과를 먼저
확정하고 동일 결과로 TAC 줄의 기준선·바깥여백·높이·소속을 결정한다.
`document_input::prepare`의 fresh TAC 분기 → `PreparedTextTable`의 plan.width/height
→ `tac_fresh::compose_with_dimensions` → 기존 `TableBand`/InlineTables의 height와 advance
→ `tac::bind` → FlowBlock/paint가 같은 준비 결과를 한 번 소비한다. body fit은 이 줄의
점유 높이를 사용하고 안 맞는 줄은 통째로 이월한다. 출력에서 높이를 다시 늘리거나 clamp하지 않는다.
저장 LineSeg 분기는 기존 compose/bind 불일치 검사를 유지한다. 셀 내부 fresh TAC의
선행 준비 방식 확장, child 내부 분할/rowspan/cut 변경은 이번 범위가 아니다.

**독립 증거.** `tests/fixtures/issue7353_fresh_tac_box_review/carrier.hwpx`는 저장 줄이 없는
본문 SHORT/TALL TAC를 만들며 선언 높이는 둘 다11565HU, 셀 최소 높이는2700/5400HU다.
그 입력 자체를 MCP2020 job `9a3a3282-a877-4cf6-bdf8-cecd8cc2b662`로 PDF 출력했다.
한컴도36/72px 높이와 기준선 정렬을 적용했다. 재저장한 파일을 원래 입력인 것처럼 바꾸지
않았으며 fixture README에 생성기·입력/PDF 해시·독립 테두리/뒤 문단 좌표를 연결했다.

**전후 실행과 회귀.** 이전 ThinThick Native 실행 파일
SHA256 `39d579ae6bfe310a19cc64d2f75d8ef0da63c0edf55e40409bea1c3b35ca1cdb`는 동일 HWPX의
문단1에서 위 불일치로 거부했고 새 실행 파일은1쪽으로 완료한다(`tac-box/before.log`,
`after-final.log`). 정식 document_flow 계약은 이 입력의 실제 테두리·내용 순서·뒤 문단
기준선·종료를 독립 PDF에 대조한다. 합성 HWP로 선언 높이 확대/축소, 명시적 개행,
너비에 따른 줄바꿈, 페이지 소유/누락·중복을 별도로 검사한다. 초기 손작성 HWPX 계약의
구조 슬롯 오류는 수정 전후 모두 거부되는 입력 오류였으므로 결함 재현으로 세지 않는다.
정상 converter HWPX와 구조 슬롯이 보존되는 HWP 계약으로 검증 대상을 구분했다.

document_flow147 + fresh_full1 + ir_text9 + nested18 + borders33 + export34 + body-end8,
**250 PASS / 0 FAIL**. 최종 로그는 `tac-box/tests-final.log`, `tests-document-final.log`,
`body-end/contract-tac-box.log`다. 기존6개 대조군(총12쪽)의 전체 JSON/SVG 전후 동일은
`tac-box/controls-comparison.json`으로 확인했다. Native build20.26s,
Native/WASM lib Clippy29.30/30.37s, 변경 Rust fmt/diff check PASS.

원본은 이 문단 거부를 넘겨 문서 준비를 완료하지만 실제 첫 페이지 실행에서
`DoesNotFit { page:0, required_height:921.7066666666667 }`가 남는다(`tac-box/original.log`).
이 후속 제한과 원본 전체 수용은 완료하지 않았다. baseline/ignore/Legacy 기본 경로는
바꾸지 않았다. 전체 CI/workspace all-target와 Studio Canvas 편집은 이번 절편에서 미실행이다.

Native same-input review/standalone overlay를 직접 열어 두 표의 다른 높이·기준선 정렬과
뒤 문단을 확인했다. 대체 글꼴 외형 및 미세한 인쇄 축척/래스터 차이는 남는다.
**fresh WASM 최종 증적.** source manifest를 고정한 뒤 Docker 빌드7분26초, 종료0.
WASM SHA256 `4c5b2eecc844a761da1ab6c9afecf88b90dcfd59fc4bd395adfe22c399487cf1`.
`node output/7353/r19/tac-box/capture.mjs --wasm`으로 같은 HWPX/PDF1쪽을 Chrome DocumentV2에서
대조했다. 최초 Chrome 프로세스 시작이 일시 실패해 동일 명령을 한 번 재실행했고 종료0
(`capture-wasm.log`, `capture-wasm-retry.log`). 코드·입력·WASM을 바꿔 오류를 우회하지 않았다.
Native/WASM 노드 차이0, SVG 동일(`review/backend-comparison.json`). HEAD50823731…와
WIP37파일 manifest·입력/PDF/WASM 해시는 `review/run.json`에 기록했고 빌드 후 source
manifest도 전부 일치했다. 테스트 source 해시는 `tac-box/tests.sha256`에 있다.

`review/wasm-review-1.png`와 `wasm-overlay-1.png`를 직접 열어 SHORT/TALL의 다른 높이,
기준선 배치, 앞뒤 문단과 외곽을 확인했다. 메인테이너가 시각 판정 통과와 다음 진행을 승인했다.
커밋·push·PR·이슈 등록은 하지 않았다.

#### B/C 후속 진단 — 첫 TAC 줄의 본문 초과와 한컴 출력 경계

앞 절편 시각 통과 후 같은 원본 `samples/issue2470/36382471_masked.hwpx`의 첫 페이지
`DoesNotFit`를 조사했다. 아직 구현 수정이나 시각 통과가 아니다. 산출은
`output/7353/r19/oversize-body/`에 있다.

입력 SHA256 `43572dad5e17395aa02d1b0000b736b8467278931086604776ef30393dd0f54b` 자체를
MCP2020 job `587082bc-0852-458c-8b14-529369b94214`로 PDF 출력했다(전처리none,
한컴11.0.0.9136, 물리2쪽). PDF SHA256
`70b137b9d69496fab4dc7170ccb81b64f7f596e07260b70af4f58b68170c0d46`.
원본 첫 부모 표의 행 분할 속성만 보고 분할 미구현으로 추정했으나, 한컴은 이 표를
첫 페이지에 온전히 배치한다. 따라서 이번 입력의 기대값을 임의의 분할로 정하지 않았다.

| 측정 대상 | 원본 HU | 96dpi px |
| --- | ---: | ---: |
| 본문 높이 | 68600 | 914.666667 |
| 준비된 부모 표 높이 / common.height | 68562 | 914.160000 |
| 위·아래 바깥여백 포함 저장 TAC 줄 높이 | 69128 | 921.706667 |
| 본문 하단 절대 위치 | 77103 | 1028.040000 |
| 위 바깥여백 포함 부모 표 하단 절대 위치 | 77348 | 1031.306667 |

소비 경로는 `tac::object_rows`의 저장 줄/바깥여백 → `TableBand` 공통 점유 →
`document_input::prepare`의 `InlineTables` → `FlowCursor::fit_until`의 원자 줄 예산 검사
→ `BodyCursor::fit` → `DocumentV2Session::next_page_json`의 진행 없음 오류다.
자식 표의 `TableCursor`는 이 첫 줄 예산 실패 이후 호출되지 않는다. 준비된 자식 표 자체는
행 분할이 가능하지만 이 사실이 호스트 TAC 줄의 분할 규칙을 입증하지 않는다.

원래 부모 테두리가 보이지 않아, `reveal.rs`로 **부모 셀 border_fill_id만 기존 가시 테두리로
바꾼 관측 대조군**을 만들었다. 치수·여백·내용·저장 줄은 유지했고 원본 파일은 수정하지 않았다.
MCP2020 job `a53b89ce-9db8-4b6d-85a8-a83e45c633a5`의 `reveal-2020.pdf`에서도2쪽이며,
원본 PDF와 대조군 PDF의92개 word bbox/문자열이 모두 동일하다(`verify.mjs`,
`observations.json`). 이 대조군을 원본 자체나 조판 수정 결과로 부르지 않는다.

`reveal-trace.xml`의 부모 아래 테두리는 페이지 위에서772.675pt다. 같은 한컴 인쇄 축척으로
환산한 본문 하단은770.224058pt여서 **한컴도 약3.27px 본문 초과**가 관측된다.
`current-hancom-1.png`, `reveal-1.png`를 직접 확인했다. `paint.rs`의 `unbounded-white.png`는
페이지 예산을 적용하지 않은 표 단독 진단 출력으로, 내부 주요 위치 대조용일 뿐 실제 문서
페이지네이션 성공/시각 승인 증거가 아니다.

현재 Native와 이미 빌드된 동일 소스 WASM 모두 첫 줄에서 동일한921.706667px 요구 높이로
거부한다(`native.log`, `observations.json`). WASM 재호출2회도 page0에서 같은 오류를 내며
페이지를 발행하지 않았다. 직전 `tac-box/source.sha256`37개 소스는 전부 일치하므로 변경 없는
WASM을 재사용했으며 재빌드·전체 회귀를 반복하지 않았다.

이번 진단에서 renderer/test 계약·baseline·기본 경로는 바꾸지 않았다. 한컴의 첫 oversized
TAC를 본문 밖까지 배치하는 동작을 V2의 본문 내 점유 계약에 허용할지, 본문 초과를 허용하지
않고 별도 분할 규칙으로 다룰지는 메인테이너 판정이 필요한 경계로 남긴다. 소폭 허용치,
표 축소, 바깥여백 생략으로 원본 수용을 강제하지 않는다. R3/R5 완료로 세지 않는다.

#### B/C 후속 구현 — 빈 본문 페이지의 oversized TAC 줄 유지

메인테이너는 같은 샘플의 마지막 셀 내용을 증가시켜도 표가 다음 쪽으로 분할되지 않는다고
확인하고 구현 진행을 승인했다. 위의 정책 선택 대기는 해소됐으며, 앞서 제안했던 **한 줄
높이 이내 overflow 허용 가정은 폐기**한다. 임계값·문서 ID·쪽 수로 분기하지 않는다.

**적용 규칙과 독립 근거.** 본문 TAC 줄은 이미 준비된 원자적 줄 구성 그대로 소비한다.
현재 쪽이 차 있으면 다음 빈 쪽으로 넘기고, 빈 쪽의 본문보다 줄이 높더라도 그 쪽에서
실제 높이를 유지한다. 이 허용은 셀 내부/자리차지/앵커 표에 전달하지 않는다.
`tests/fixtures/issue7353_oversize_tac_review/`의 HWPX3건을 한컴 PDF로 출력해,
first/grown은 표→후속 문단2쪽, preceded는 앞 문단→표→후속 문단3쪽임을 확인했다.
grown의 테두리 끝718.174pt는595pt 종이 밖이며 후속 쪽에 표 조각을 만들지 않는다.
입력 생성 방식·job·PDF trace 좌표·해시·HWP 계약과 HWPX 독립 출력의 구분은 fixture README에 있다.

**실제 소비 경로.** `document_input::prepare`가 `TableBand`의 동일 줄 메트릭을
`InlineTables {height, advance, ...}`로 보낸다. `BodyCursor::fit`의 본문 story 호출만
`FlowCursor::fit_body_until`을 사용한다. 잔여 높이가 전체 본문 높이와 같고 선행 점유가
없는 첫 InlineTables만 초과를 수용한다. 각 자식은 자기 준비 높이 전체로
`TableCursor::fit`하며 continuation 완료를 확인한 뒤 한꺼번에 수용한다. 컷·유닛을 재추측하지 않는다.
실제 occupied height/advance와 `body_inline_overflow`를 `BodyFit::accept`가 그대로 전달하고,
`DocumentV2Session::next_page_json`은 그 수용 결과만 body 높이 불변식의 명시적 예외로 사용한다.
`paint.build_node`는 수용한 placement를 사용하며 본문 clip은 추가하지 않는다. 본문 bbox는
원래 편집 영역을 유지하고 표 bbox는 실제 높이다. paint 단계에서 늘리거나 축소하지 않는다.
뒤 gap/0공간은 같은 쪽에서 소비하되 다음 실제 내용은 다음 쪽으로 보낸다.
일반 셀/앵커의 `fit_cell_until`/`fit_until`은 overflow 허용false여서 기존 예산·분할 계약을 유지한다.

**검출과 회귀.** 수정 전 원본 HWPX와 높이80/160px 합성 TAC 계약2건이 각각
`DoesNotFit`로 실패했다(`frame-after-child/oversize-before-issue_7353_table_v2_document_flow.log`:
147 PASS /2 FAIL). 변경 후 원본은 실제 DocumentV2에서2쪽 끝까지 완료한다.
표 원점8786/75px·전체 높이68562/75px, 표지 기관명 보존과 다음 본문을 검사한다.
합성 계약은 앞 문단 유무·늘어난 높이·실제 표 원점·뒤 문단 원점·유닛 중복/누락·종료를 검사한다.
셀 내부50px TAC가49px 예산을 초과하지 않는 반례도 추가했다.
최종 집중 회귀는 document_flow150 + fresh_full1 + ir_text9 + nested19 + borders33 + export34
+ body-end8 = **254 PASS /0 FAIL**다. 로그는 `oversize-body/tests-final.log`,
`fresh-final.log`, `controls-final.log`, `nested-final.log`, `body-end/contract-oversize-final.log`.
최초 fresh_full의 잘못된 파일명으로 인한 build 실패는 정확한 target으로 재실행했고,
결함 검출 건수로 세지 않았다.

Native build는 첫 링크가SIGBUS로 실패했으나 같은 소스로 재시도해23.35s에 성공했다.
Native/WASM lib Clippy는34.65/34.53s, 변경 Rust fmt/diff check PASS.
Native `oversize-body/probe`로 원본 전체2쪽을 생성했고 같은 원본 PDF와 직접 비교했다.
`review/native-review-{1,2}.png`에서 첫 쪽 표·제목·하단 기관명과 다음 쪽 본문을 확인했다.
대체 글꼴의 굵기/폭·일부 기호 위치·미세 인쇄 축척 차이는 남으며 이번 정책 수용과 구분한다.

**범위.** 이 구현은 본문 원점에 오는 indivisible TAC 줄에 한정한다. 양수 문단 앞 간격을
동반한 oversized TAC, 일반 TAC 줄의 분할 가능성 전체, Studio Canvas 편집, 전체CI는
미검증이다. Legacy 기본 경로·baseline·ignore는 변경하지 않았고 R3/R5 전체 완료를 주장하지 않는다.
HWPX 첫 문단의 구조 축 이동 거부를 완화하지 않았다. 별도 HWP 합성 계약의 한컴 PDF
변환 요청은 지연 중이며 독립 출력 완료로 세지 않는다. 같은 입력의 원본 HWPX/PDF 비교와
이 보조 대조군을 혼동하지 않는다. Docker fresh WASM의 최종 결과는 아래에 이어 기록한다.

**fresh WASM 완료.** Docker 빌드7분30초, 종료0. WASM SHA256
`285376977e42aecbf6c06ba7112516e8adb4b7998f1ffd1467445e42c819b292`.
`node output/7353/r19/oversize-body/capture.mjs --wasm`과 `check-controls.mjs` 종료0.
원본2쪽의 Native/WASM 노드 차이0·SVG 동일, 보조 HWP3건(2/3/2쪽)도 노드/SVG 차이0이다.
`review/run.json`에 HEAD50823731… + WIP37파일 source manifest, 동일 원본/PDF/WASM 해시를
고정했고 빌드 뒤 manifest37건 전부 일치했다. `review/wasm-review-{1,2}.png`와
`wasm-overlay-1.png`를 직접 열어 첫 쪽 표 배치·하단 기관명·다음 쪽 본문을 확인했다.
이것은 표 단독 unlimited 진단이 아니라 실제 DocumentV2의 처음부터 끝까지 출력이다.
앞서 명시한 글꼴/기호 차이는 남는다. 이후 메인테이너가 “시각 판정 통과입니다”로 승인했다.

보조 HWP3건도 보존한 수정 전 `tac-box/probe`로 실행해 각각page0/1/0의
682.533333/682.533333/882.533333px `DoesNotFit`를 확인했다(`*-before.log`,
`probes.sha256`). 현재 출력은 동일 입력에서2/3/2쪽으로 완료한다. 외부 HWP→PDF 작업은
`first/preceded/grown-hwp-start.json`의job으로 추적하며, 지연 결과를 기다려 이 절편을
반복 확장하지 않는다. 원본 및 정상 완료된 HWPX3건의 독립 PDF 증거는 이미 확보했다.

### 다음 묶음: #7158 셀 내부 쪽번호 선언의 소유 줄

이전 oversized 본문 TAC 절편의 시각 승인을 반영하고 #7158 원본을 진행한다.
#6601 원본은 현재 Native DocumentV2에서4쪽 종료하며, #7158 원본은 첫 표의 셀 안
`PageNumberPos`를 미지원 컨트롤로 거부한다. #7008의 section decoration/grid 제약과 구분한다.

입력은 `samples/issue4090/156492236_규제샌드박스_min.hwpx`이며 수동 수정하지 않았다.
`s0/p0/c2/cell1/p0`의 “보도참고자료” 뒤 쪽번호 선언은 저장 첫 줄에 속한다.
글자 offsets는0..5, 저장 첫 줄 높이2900HU·baseline2465HU다. 쪽번호는 셀 글자나
공간이 아니며, 해당 줄의 실제 수용 페이지부터 footer story를 활성화해야 한다.
기존 PDF의 출처는 `pr_4763_review.md`의 한컴2020 MCP 대응 기록을 확인했다.

추적: `page_number::collect_cell_stories`의 표/셀/문단/줄 소유 경로 →
`ir::bind_table`의 구조 선언 수용 → `text_ir`의 UTF-16/LineSeg를 보존한 글줄 구성 →
기존 recursive fit의 `CellPlacement.lines/tables` → `PageNumberHost::accepted` →
`DocumentV2Session`의 성공한 페이지에만 footer 활성화. 본문 높이나 표 컷은 바꾸지 않는다.
부모 표가 시작했다는 이유로 뒤쪽 셀 문단의 선언을 앞 쪽에 적용하지 않는다.
저장 줄이 없는 재조판 문장 중간 선언·여러 컨트롤 혼재는 아직 거부한다.

첫 경계 테스트는 수정 전11 PASS/1 FAIL이며 실패 원인은 `non-table cell control`이다.
로그: `output/7353/r19/frame-after-child/cell-number-before-issue_7353_table_v2_page_number.log`.
분할 문단의 뒤 줄·빈 문단·중첩 자식 셀·HWP/HWPX 경계를 추가해 검증 중이다.
전체 #7158 어울림 해결이나 최종 시각 통과로 미리 판정하지 않는다.

**집중 검증 결과.** 셀 쪽번호15 + 쪽번호 timeline5 + document_flow150 + ir_text9 +
nested19 + borders33 + export34 + fresh_full1 + 기존 Legacy #7158 계약3 = **269 PASS/0 FAIL**.
로그는 `frame-after-child/cell-number-final-*`, `cell-number-final-export-*`,
`cell-number-controls-*`다. `cell-number-final`의 최초 저장 줄 합성 계약1건은
`TAG_SINGLE_SEGMENT_LINE`을 누락하여 실패했다. 입력의 누락된 유효성 태그를 보완했으며
엔진 수용 조건을 완화하지 않았다. 최종15건은 `cell-number-final-export`에 기록했다.
최초 수정 전 결함 검출11/1과 이 합성 입력 오류를 구분한다.
Native build26.44s, Native/WASM lib Clippy33.07/33.50s 성공, 변경 Rust fmt·diff check PASS.
#6601 전체4쪽과 #7150 전체2쪽은 수정 전 Native JSON과 `cmp` 동일하다.
`next-flow/source.sha256`/`tests.sha256`에 최종 소스를 고정했다.

**남은 경계.** #7158 원본은 쪽번호 선언 수용 후 `V2 stored picture appearance or resource`에서
멈춘다(`next-flow/7158-after.log`). 첫 표의 그림들에 그룹 로컬 offset 등 별도 변환 속성이 있다.
이 속성을 삭제하거나 원본 그림을 지워 통과시키지 않았다. 목표9쪽 어울림은 아직 미검증이다.
셀 저장 LineSeg의 프레임 리셋과 page-number 혼재, 다른 컨트롤과의 혼재는 이번 확장의
완료 범위로 주장하지 않는다. 기본 Legacy 경로·baseline·ignore·원본 샘플 변경은 없다.

`next-flow/fixtures/`는 경계 테스트가 생성한 HWP/HWPX 합성 입력이다. 독립 한컴 출력이
필요한 late-cell 선언은 `split-cell-number.hwp`로 MCP2020 PDF를 요청했다
(job `666f9948-2392-4526-9d66-825a522a0941`, `next-flow/pdf-start.json`).
현재 서버 queue2 대기이며 합성 계약 통과를 한컴 시각 일치로 바꾸어 보고하지 않는다.
Docker fresh WASM와 backend 대조 결과는 아래에 추가한다. 원본 #7158 전체 시각 판정을
요청할 준비가 끝난 것은 아니다.

**fresh WASM 결과.** Docker 빌드7분26초 종료0, WASM SHA256
`9e3c538668c586f8d12e50d80c664d1a24d28ef37e5f9d94cbb1c5cb105a7570`.
`node output/7353/r19/next-flow/check-wasm.mjs` 종료0. HWP/HWPX7입력·총19쪽의
전체 Native/WASM 노드·SVG 차이0 (`backend-comparison.json`), source37파일 해시도
빌드 전후 동일하다. 분할 셀은2쪽, 중첩/빈 문단/저장 두 번째 줄은각3쪽이다.
`split-cell-diagnostic-wasm.png`를 직접 열어 첫 쪽 before/A/B/C에 쪽번호가 없고,
다음 쪽 D/E/host/after와 `- 2 -`가 표시됨을 확인했다. 이미지 제목에도 명시했듯
이는 합성 경계 진단이며 한컴 비교용 승인 이미지가 아니다.
최종 추가 assertion은 중첩 입력의 중간2쪽 활성화와3쪽 후속 본문에서 번호 유지까지
검사한다(`cell-number-final-assertions-issue_7353_table_v2_page_number.log`:15 PASS).
PDF 작업은 마지막 확인에서 `running/converting`으로 독립 출력이 아직 없다.
이번 구현의 시각 피델리티는 미검증으로 남기며 원본 전체 통과를 주장하지 않는다.

### 다음 묶음: #7158 비그룹 TAC 그림의 그룹 내부 offset 해석

원본 첫 표 `s0/p0/c2/cell2/p0/c0`는 `group_level=0`, `offset_x=320`이며
회전·이동·shear 행렬 성분은0이다. HWP5.0 revision1.3 표83은 offset_x/y를
**개체가 속한 그룹 내부 좌표**로 정의한다. `model/shape.rs`의 IR 의미와 기존
`layout/picture_footnote.rs::layout_picture_full`의 TAC 배치도 이를 페이지 이동으로
사용하지 않는다. 그룹 없는 TAC의 이 속성을 이유로 그림을 거부하던 V2 조건만 제거했다.
실제 그룹·render_tx/ty·회전·shear는 계속 거부한다. 원본 속성·그림 데이터는 수정하지 않았다.

소비 경로: `pictures::payload`의 수용 조건 → `pictures::compose`의
`tac::object_rows` 저장 줄별 객체 rect → `ParagraphItem::ObjectRow.bounds` →
기존 fit의 줄 수용/이월 → `fragment`의 `CellPlacement.lines` → `text::paint`의
동일 line node 이동과 Image bbox다. 새 원점 보정·덮어쓰기·clamp는 없다.
그룹 내부 좌표를 이 흐름의 페이지 원점에 더하지 않는 것이 이번 변경의 전부이며,
컷·예약 높이·종료 로직은 변경하지 않았다.

`issue_7353_table_v2_export`에 양/음 로컬 offset, 같은 줄/별도 줄,
그림 줄24px에 대해 남은23/24px 예산을 비교하는 계약을 추가했다. 그림의 실제 최종 좌표,
표 외곽, 다음 문단, 페이지 수와 SVG를 포함한 **전체 결과**가 offset0 대조군과 같아야 한다.
기대 위치는 기존 저장 LineSeg 계약의 baseline/정렬/점유에서 정하며 새 구현이 재산출한
숫자를 정답으로 쓰지 않는다. 그룹 그림과 실제 affine 이동은 비적용 반례로 남겼다.
수정 전34 PASS/1 FAIL은 해당 그림 거부 오류로 실패했고, 수정 후 통과했다
(`frame-after-child/picture-offset-before-issue_7353_table_v2_export.log`).

정상 한컴 저장 `issue7353_picture_space_review/picture-saved.hwp`의 두 그림을 그대로
사용한 추가 불변성 계약도 작성했다. 메타데이터 변경 입력은 사양 기반 합성이며 새 한컴
저장본이라고 주장하지 않는다. 처음 전체 JSON 비교는 HWP→HWPX 변환 시 예약 tab word
3개의32→0 정규화 때문에 실패했다. 두 대조군을 같은 HWPX 직렬화 경로로 만들어 해당
별도 변수를 통제했고, 좌표·SVG assertion을 완화하지 않고 전체 동일 검사를 통과했다.
원래 HWP 정상 대조군은 수정 전/후 Native JSON이 byte-identical이다.

집중 검증: export36 + page_number15 + timeline5 + document_flow150 + ir_text9 +
nested19 + borders33 + fresh_full1 + 기존 Legacy #7158 계약3 = **271 PASS/0 FAIL**.
로그: `frame-after-child/picture-offset-{after,final-normalized,legacy}-*`.
Native build20.68s, Native/WASM lib Clippy28.85/29.09s, 변경 파일fmt·diff check PASS.
push/PR 전 전체workspace lint 및 전체CI 검증을 실행했다는 뜻은 아니다.
source HEAD는 `50823731af6050c60ec3cfcc898abf36daa6b35a` + WIP이며
`picture-offset/source.sha256`, `tests.sha256`로 실제 내용을 고정했다.

같은 정상 HWP와 독립 `picture-2020.pdf`를 대상으로 Native compare/overlay/review를
새로 산출해 직접 확인했다(`picture-offset/review/native-{compare,overlay,review}-1.png`).
두 그림의 기준선·크기·간격, 부모/자식 표 외곽과 뒤 문단 위치는 보존된다.
글꼴 외형·인쇄 래스터 가장자리 차이는 남으며 이 절편에서 보정하지 않는다.
#7158 원본은 그림 리소스가 흰색 placeholder로 최소화되어 있어 이것만으로 그림 시각
피델리티를 판정하지 않았다. 전체 원본은 그림 차단을 넘어 **문단 index9의
`stored text requires intact single-segment rows`**로 진행했다
(`next-flow/picture-after.log`). 이 저장 글줄 경계가 다음 대상이며 전체 문서 완료는 아니다.
기본 Legacy 선택·ignore·baseline·원본 파일은 변경하지 않았다.

이전 셀 쪽번호 합성 입력의 MCP PDF job은 이번 확인에서600초 초과로 실패했다
(`next-flow/pdf-status-latest.json`, output0bytes). 이전 기록의 running은 당시 상태이며,
현재도 그 입력의 한컴 출력 일치는 미검증이다. 이번 정상 그림 대조군의 기존 독립 PDF와
해당 변환 실패를 혼동하지 않는다.

다음 대상의 입력 증거는 `picture-offset/next-paragraph.log`에 남겼다. 본문index9는
텍스트와 저장4줄만 있고 control0·dirty=false이며, 줄의vpos는67001→69401→0→2400HU다.
`stored_text::localize`는 문단 첫 줄 대비 음수가 되는 세 번째 줄을 거부한다.
저장된 페이지/프레임 좌표 리셋을 보존할 계약이 다음 구현 대상이다. 단순 글줄 상자 겹침을
페이지 경계라고 추정하는 예외를 추가한 것은 아니다.

**fresh WASM 최종 확인.** Docker 빌드7분18초·종료0. WASM SHA256
`a82a3ed9542089a87a67bf6ee863343aec6b9dcfc3a3741cdf9b38df24ea5c11`.
`node output/7353/r19/picture-offset/check-wasm.mjs`와
`node output/7353/r19/picture-offset/capture.mjs --wasm` 모두 종료0이다.
경계4입력3/2/4/3쪽 + 그림이 보이는 속성변경 문서1쪽 + 정상 대조군1쪽 =
**6입력14쪽**의 Native/fresh WASM 전체 노드·SVG 차이0. source manifest도 전후 동일하다.
`picture-offset/local-offset-invariance.png` 및
`picture-offset/review/wasm-review-1.png`를 직접 열어 그림2개의 위치·크기와
부모/자식 외곽·후속 문단을 확인했다. 전자는 사양 기반 속성 불변성 비교이며,
후자만 동일 정상 한컴 저장 입력/기존 독립 PDF 비교다. 정상 대조군의 대체 글꼴 외형과
인쇄 가장자리 차이는 남는다. 변경된 offset 입력의 별도 한컴 PDF, #7158 전체 출력,
전체CI는 이번 결과로 검증됐다고 주장하지 않는다.

### R19 — 본문 문단 내부 저장 페이지 좌표 리셋 (2026-09-28)

#7158 문단9는 편집되지 않은 본문4줄이며 저장vpos가67001→69401→0→2400HU다.
기존 로컬 변환은 세 번째 줄의 음수 원점을 거부했다. LineSeg 표준의 본문 쪽 기준 좌표와
정상 한컴 저장 대조군의 실제 리셋을 근거로, 단일 단 본문의 문단 내부 페이지 경계를
보존했다. 줄 상자의 바닥 겹침만으로 경계를 추정하지 않는다.

`body_text::frame_starts`는 원점 감소와0복귀를 검증한다. 편집된 저장 분할,
별도 원본좌표 배열, 필드, 비구조 컨트롤, 비0리셋은 여전히 미지원이다.
section/column/page-number 선언은 상위 문서 검증을 거친 비점유 선언만 허용한다.
문단 간 리셋이나 앵커가 있는 문단으로 지원 범위를 확장하지 않는다.

소비 경로: `document_input.rs` 원본 LineSeg → `body_text::frame_starts`의 다음 쪽
첫 LineOwner → `stored_text::continuous_paragraph` 연속 로컬 좌표 → TextComposer
공통 줄 노드/점유 상자 → 같은 owner의 FlowBlock 앞 page_break → BodyCursor
fit/이월 → 수용된 같은 줄의 실제 paint다. 셀용 연속화 helper는 이름만 일반화했다.
앞 쪽 마지막 interline gap은 FollowingLineGap으로 소비하여 새 쪽으로 이월하지 않는다.
실제 빈 줄은 LineOwner/상자로 보존한다. paint clamp나 문단 앞 간격 반복은 없다.

정식 `issue_7353_table_v2_document_flow.rs`에4계약을 추가했다. 작은/큰 본문 예산,
반복 리셋, 이어받는 첫 빈 줄, 문단 앞 간격, 뒤 문단, 원본 IR 불변성, 겹치는 줄,
비0리셋 거부, 정상 저장 HWP/HWPX의 최종 위치를 검사한다. 기대값은 저장 줄 메트릭과
쪽 원점에서 정했다. 초기 합성 입력의 첫 문단 section슬롯/UTF16 축 작성 오류는
PREFIX 문단으로 분리해 수정했으며 LineSeg 수용 검증을 완화하지 않았다.
수정 전 실행파일 `next-flow/picture-probe`는 유효한 합성 빈 줄 입력과 정상 한컴
저장본 모두 문단1의 `stored text requires intact single-segment rows`로 실패한다
(`body-reset/before.log`, `portrait-before.log`). 수정 후 각각3/2쪽이며 계약 통과다.

독립 대조군의 생성·MCP job·SHA256은
`tests/fixtures/issue7353_body_frame_review/README.md`에 기록했다.
한컴 저장 vpos는1800→3600→5400(빈 줄)→7200→0→1800HU다.
첫 쪽 BRAVO 뒤 빈 줄과 CHARLIE, 다음 쪽 DELTA/ECHO/AFTER 위치를 동일 입력 PDF와
대조했다. 초기 가로 용지의 인쇄 방향 차이는 부적절한 대조군으로 보존하고 채택하지 않았다.

집중 검증: document_flow154 + page_number15 + timeline5 + ir_text9 + nested19 +
borders33 + export36 + fresh_full1 + Legacy #7158 3 = **275 PASS/0 FAIL**.
로그 `frame-after-child/body-reset-{normal,focused}-*.log`.
최종 Native lib build15.04초, Native/WASM lib Clippy28.79/30.28초 PASS,
변경 파일rustfmt와diff check PASS. 전체workspace/CI 검증을 뜻하지 않는다.
source HEAD `50823731af6050c60ec3cfcc898abf36daa6b35a` + WIP이며
`body-reset/source.sha256`으로 실제 V2 소스를 고정했다.

원본 #7158의 입력 준비는 문단9를 지나 문단21의
`text preview paragraph decoration or keep`에서 멈춘다(`next-flow/body-reset-full.log`).
앞10문단만 보존한 진단 입력의 실제 렌더링에는 첫 표의
`conflicting shared V2 cell borders`도 남는다. 이 거부를 끄지 않았고,
대조군 통과를 원본 전체 통과로 보고하지 않는다. Legacy 기본 선택·baseline·ignore·
원본 샘플은 변경하지 않았다.

**fresh WASM 최종 확인:** Docker 빌드7분20초·종료0. SHA256
`7fbf686ac3b979f3ee4d18459fc2bab7e74481e66b8dd96b3705ba2b914b5611`.
`node output/7353/r19/body-reset/check-wasm.mjs` 및
`node output/7353/r19/body-reset/capture.mjs --wasm` 종료0.
빈 줄/반복 리셋 합성 입력3쪽과 정상 한컴 저장 대조군2쪽, 총5쪽의
Native/fresh WASM 최종 노드·SVG 차이0이다. source manifest도 변경 없이 일치한다.
`body-reset/review/wasm-review-{1,2}.png`와2쪽 standalone overlay를 직접 열어
첫 쪽의 의도적 빈 줄, CHARLIE 위치, 다음 쪽 DELTA/ECHO/AFTER 시작을 확인했다.
Native/fresh WASM compare·standalone overlay·review와 SHA는 같은 review 폴더에 있다.
대체 글꼴 폭/모양과 인쇄 가장자리 차이는 남으며 이 절편에서 보정하지 않았다.
2026-09-28 메인테이너가 이 절편의 시각 판정 통과와 다음 진행을 승인했다.
전체 원본 #7158과 전체CI는 미완료다.

### R19 — 일반 텍스트 문단 보호(keep_lines) 수용

문단21의 실제 거부는 문단 자체의 장식이 아니라 내부 표cell7/p0("632")의
ParaShape34 `attr1 bit18`이었다. resolved `keep_lines=true`, 그 외 keep/장식은
비활성이다(`next-flow/paragraph-properties.log`). HWP 사양 bit18의 문단 보호,
정상 한컴 저장/PDF의 전체 문단 이월을 독립 근거로 사용했다.

`paragraph_keep::group`은 TextComposer가 확정한 줄 상자와 간격을 하나의 Lines
단위로 묶는다. 시작 공백과 내부 빈 줄·줄간격을 포함하고 높이는 실제 줄 상자 끝의
최댓값, 전진은 기존 합성 결과의 pen이다. 줄 높이를 단순 합산하지 않는다.
ParagraphEnd는 그대로 별도 소유여서 셀 말미 정책/뒤 문단을 변경하지 않는다.

소비 경로: ParaShape bit18 → resolved keep_lines → TextComposer의 실제 paint 노드와
ParagraphItem 줄/Space → paragraph_keep 공통 그룹의 height/advance/owner별 y →
본문 document_input 또는 셀 ir의 FlowBlock::Lines → FlowCursor fit의 전체 높이 수용 →
같은 owner별 bounds를 옮기는 paint다. 부족하면 그룹 전체를 미소비 상태로 남겨 다음
영역에서 재시도한다. 새 clamp·행 높이 상수·별도 backend 배치는 없다.
문단 내부 저장 frame cut을 그룹으로 삼켜서는 안 되므로 본문에도 원자 그룹 내부 컷
거부를 추가했고, 셀의 기존 같은 거부를 유지했다. 한 페이지보다 큰 보호 문단,
keep_with_next/widow_orphan, 개체 혼합은 이번 지원/한컴 일치로 주장하지 않는다.

정식 회귀6건 추가: 본문 이월/비보호 대조, 정상 한컴 HWP, 모순된 저장 컷 거부,
실제 문서 셀 조각/표 외곽/후속 본문, 셀의47/48px 경계·빈 줄·종료,
음수 줄간격으로 겹치는 상자와 문단 앞뒤 간격(전체 출력 불변성)이다.
기대값은 입력의12px 줄 상자와18px pitch/간격, 정상 저장본16px/24px 메트릭이다.
추정된 그룹 높이만 검사하지 않고 최종 줄 위치와 표 조각 높이·후속 문단을 확인한다.

수정 전 새 본문/셀 계약은 각각 keep 미지원으로 실패했다
(`frame-after-child/keep-before-*.log`: 기존154/40 PASS, 새1/1 FAIL).
이전 실행파일의 정상 HWP도 동일 거부다(`keep-lines/normal-before.log`).
수정 후 document_flow158 + text42 + page_number15 + timeline5 + ir_text9 + nested19 +
borders33 + export36 + fresh_full1 + Legacy #7158 3 = **321 PASS/0 FAIL**.
`frame-after-child/keep-export-final-issue_7353_table_v2_document_flow.log` 및
`keep-final-*`에서 현재 실행 결과를 확인한다. 추가 테스트의 초기 참조 타입 오류는
테스트 작성 오류이며 엔진 결함 재현으로 세지 않았다.

첫 Native 빌드는 여유134MB에서 linker signal7로 실패했다. 빌드 실패 직후 이전
rlib를 소비한 `keep-after-*`는 수정 후 검증에서 제외했다. 과거 `frame-end/`의
export/document_flow/borders/text 실행파일4개만 gzip 압축해 공간을 확보했다.
소스·로그·이미지는 변경하지 않았고 실행파일은 gunzip으로 정확히 복원할 수 있다.
재빌드12.03초 PASS 뒤 계약을 재실행했다. Native/WASM lib Clippy29.39/29.64초 PASS.
전체workspace/PR CI는 실행하지 않았다.

독립2쪽 대조군은 `tests/fixtures/issue7353_keep_lines_review/README.md`의
입력·MCP job·SHA256을 따른다. Native review를 직접 열어1쪽의 남은 공간과
2쪽 ALPHA/빈 줄/CHARLIE/AFTER 위치를 확인했다. 한컴 글꼴 외형 차이는 남는다.
정상 HWP를 rhwp에서 HWPX로 재직렬화하면 보호 문단 전 p0의 저장 줄 검증에서
거부된다. 초기 정상 fixture 테스트의 이 경로 실패는 기록하고 HWPX round-trip
성공 주장에서는 제외했다. fresh HWPX의 본문·셀 경로는 별도 정식 계약으로 통과한다.

원본 #7158은 keep 거부를 지나 같은 표의 `mixed-em CENTER text`에서 멈춘다
(`keep-lines/full-after.log`). 앞 표의 공유 테두리 거부도 이번 변경 범위 밖이다.
Legacy 선택·ignore·baseline과 원본 속성을 변경하지 않았으며 전체 문서 완료가 아니다.

**fresh WASM 확인:** Docker 빌드7분23초·종료0. WASM SHA256
`7593e09954218b690ecd1a0aa840858c5b5d23a8cd8844e9b44a1ffcdde0d365`.
최종 Native lib build15.00초 뒤 정상 대조군을 재출력했다.
`node output/7353/r19/keep-lines/check-wasm.mjs`와
`node output/7353/r19/keep-lines/capture.mjs --wasm` 모두 종료0.
fresh HWPX 본문2쪽·분할 셀3쪽, 정상 한컴 HWP2쪽 = **3입력7쪽**의
Native/fresh WASM 전체 노드와SVG 차이0이다. 합성 입력의 backend 동등성과
정상 HWP의 독립 PDF 대조를 구별한다.
`keep-lines/review/wasm-review-{1,2}.png`와2쪽 standalone overlay를 직접 열어
문단 전체 이월·빈 줄·뒤 문단 위치를 확인했다. 같은 폴더에 Native/WASM
compare·overlay·review와run.json이 있다. 한컴과의 대체 글꼴 폭/외형 차이는 남는다.
source HEAD `50823731af6050c60ec3cfcc898abf36daa6b35a` + WIP,
`keep-lines/source.sha256`와`tests.sha256`은 빌드/검증 전후 동일하다.
이번 절편은 메인테이너가 시각 판정 통과 및 다음 절편 진행을 승인했다. 전체 #7158 출력, HWP→HWPX 재직렬화
대조군의 첫 문단, 전체CI가 완료됐다는 뜻은 아니다.

### R19 — 혼합 글자 크기의 문단 세로 CENTER

승인된 다음 절편이다. 원본 문단21/cell12의 `135 (21%)`는12pt/10pt,
문단 세로 CENTER, 저장 text_height1200HU/reference600HU다.
기존 새 경로는 균일 em만 처리해서 `mixed-em CENTER text`로 거부했다.
단순히 거부를 제거하면 작은 글자까지 큰 글자 기준선에 놓여 CENTER 의미가 사라진다.

독립 근거: `tests/fixtures/issue7353_mixed_center_review/README.md`의 정상
한컴 HWP/PDF2쌍. 24pt/12pt 대조군에서 CENTER 작은 글자 기준선은 큰 글자보다
약5.60px 위, BASELINE 대조 문단은 같은 기준선이다. 원본 표를 분리한 대조군은
41개 셀/42개 저장 줄과 혼합 크기 승인합계 열을 보존한다. 분리·정상 저장한 입력이며
원본 전체 문서와 동일하다고 주장하지 않는다. 수동 저장 메트릭으로 수용 조건을 풀지 않았다.

생산→소비 경로는 `stored_text::resolve_vertical_alignment`의 각 줄 max-em
검증/중심 → `text::compose_text_with_body_end`의 공통 LayoutEngine 호출 → `align_center_runs`의
최종 run 기준선 → 같은 최종 노드에서 ParagraphItem 높이/전진/owner 생성 →
본문·셀 FlowBlock/fit → 해당 payload의 실제 paint다. glyph 기준선은
`line origin + line center + own nominal baseline - own em/2`다.
Legacy·backend별 별도 규칙은 변경하지 않았다. 저장 줄의 높이/기준 검증,
줄 구성과 전진은 보존하고, 큰 글자가 뒤에 있어도 max-em 중심은 같다.
위/아래 첨자는 별도로 명시 거부한다. 분할 컷/rowspan/예약 정책 자체는 비변경이며
변경된 payload가 셀 분할을 지나도 위치와 후속 문단을 보존하는지는 실행했다.

수정 전 새 formal 셀 계약은 `Unsupported("mixed-em CENTER text")`로 실패했다
(`frame-after-child/mixed-center-before-issue_7353_table_v2_text.log`). 기존 script
거부의 오류명 변경 assertion 실패는 결함 재현과 구별한다. 변경 전 실행파일의
정상 HWP2개도 같은 거부였다(`mixed-center/{normal,table}-before.log`).
수정 후 document_flow160 + text43 + ir_text9 + nested19 + borders33 + export36 +
page_number15 + timeline5 + fresh_full1 + Legacy7158 3 = **324 PASS/0 FAIL**.
`frame-after-child/mixed-center-after-*`, `mixed-center-final-*`,
`mixed-center-regression-*` 로그를 따른다. 초기에 잘못 지정한 Legacy 파일명 및
새 테스트의 control index/cell_context 가정 오류는 테스트 작성 오류이며 엔진 회귀가 아니다.

Native build 성공, Native/WASM lib Clippy29.64/30.93초 PASS.
Native compare/review/overlay를 직접 열어 CENTER/BASELINE 차이, 원본 표 분리본의
행과 외곽·혼합 글자 위치를 확인했다. 대체 글꼴 외형 차이는 남는다.
실제 표 PDF의 큰/작은 글자 baseline 차이는 약0.799px, 공통 nominal 메트릭을
쓰는 rhwp는0.933px로 약0.134px 잔차가 있다. 대조 문단의5.60px 일치와 구별하며
글꼴별 세부 메트릭까지 완전히 일치한다고 주장하지 않는다.
원본은 문단21을 지나 문단30 `ContentBounds { row:1,column:0 }`로 진행한다
(`mixed-center/full-after.log`). 이 다음 차단 지점이나 원본 앞 표의 기존 테두리
문제를 이번 절편에서 함께 완료했다고 주장하지 않는다.

**최종 fresh WASM/Visual Sweep:** Docker 빌드7분21초·종료0.
WASM SHA256 `5105d64275321c0ab42e16e34f3642b02952d018045a0ee63c997bdb0d6cb3ac`.
`node output/7353/r19/mixed-center/capture.mjs --wasm`,
`capture-table.mjs --wasm`, `check-fresh.mjs` 모두 종료0이다.
정상 HWP2개와 저장 줄 정보 없는 fresh HWPX1개, 총3입력3쪽에서
Native/fresh WASM 전체 노드·SVG 차이0이다. `review/`, `table-review/`에
각 Native/WASM compare·standalone overlay·review·run.json을 남겼다.
WASM review2종과 표 standalone overlay를 직접 열어 글자 세로 위치·행·외곽을
확인했다. 글꼴 차이와 위의 미세 baseline 잔차는 남긴다. 메인테이너가 후속 응답에서
시각 판정 통과와 다음 절편 진행을 승인했다. source HEAD `50823731af6050c60ec3cfcc898abf36daa6b35a` + WIP이며
`mixed-center/source.sha256`와 `tests.sha256`을 검증했고 코드의 빌드 후 변경은 없다.
명시 파일 rustfmt check 및 git diff check PASS. 전체CI는 이번 절편에서 미실행이다.

### 2026-09-28 — 문단30 자식 표의 실제 높이 예약과 Dot 테두리

이전 혼합 글자 CENTER는 메인테이너 시각 통과로 닫고, 승인된 다음 절편으로
원본 문단30 `ContentBounds {row:1,column:0}`를 처리했다. 모든 증적은
`output/7353/r19/inline-bounds/` 및 `frame-after-child/inline-dot-*`에 있다.

원인은 bind 전 slot155.40000000000001px에 실제 자식 plan155.40000000000003px를
그대로 담은 것이다. 원본 실행의 gdb 값은 `values-before.log`에 보존했다.
두 값은 기존 bind의 동일 기하 검증을 통과하지만, content의 실제 자식 끝점
검사는 올바르게 큰 값을 거부했다. `ContentBounds` 허용치를 늘리지 않았다.

생산/소비 경로: `tac::bind`의 slot/plan 검증 → `tac::bound_height`의 실제
`child.y + child.plan.height` → `ir.rs`의 셀 및 `document_input.rs`의 본문
InlineTables 예약 높이 → `content.rs`의 경계 검사 → `flow.rs:423`의 예산 검사/
자식 전체 fit/실제 placement다. bind 후 결과를 공통으로 쓰며 글줄 advance,
줄 소속, TAC 분할 규칙, Legacy는 변경하지 않았다. 실제 크기가 다른0.29/0.31
slot은 기존 거부를 유지한다. rowspan 컷/종료와 캡션·각주 경로는 비변경이다.

경계 계약은 실제0.1+0.2 높이, 시작 offset0/0.1, 실제 끝점보다1ULP 작은 예산과
정확한 예산, 다음 조각의 AFTER 소유/완전 종료를 검사한다. 원래 작은 예약만
수용한 뒤 paint에서 늘리는 방식이 아니다. 최초 계약 실행1FAIL/1PASS 이후
후속 문단 이월 검증을 RowBreak에 명시해 최종2PASS다. 최초 CellBreak 실행과
최종 RowBreak 검사의 범위 차이를 숨기지 않는다. 동일 정상 HWP의 변경 전
실행은 `normal-before.log`에 ContentBounds로, 변경 후는1쪽으로 남겼다.

높이 처리 후 같은 표의 부모 Dot 테두리가 미지원으로 드러나, 원본 테두리를
삭제하지 않고 같은 표의 Dot paint를 함께 구현했다. 독립 정상 저장 카탈로그의
16개 표준 펜과 두 축의 PDF stroke를 근거로 기존 Dash 공통 physical-stroke
생성기에 Dot 펜 정의를 추가했다. backend별 점선 속성을 따로 사용하지 않는다.
`borders::resolve_edges` → 공유 경계 union → `dash::append` → Solid LineNode →
공통 출력 경로다. 표준 폭 인덱스 외의 문서별 수치 예외는 없다. 생성과 기대값,
입력/PDF SHA는 `tests/fixtures/issue7353_inline_bounds_review/README.md`에 있다.

새 정상 입력 검사2건은 Dot 구현 전 명시적 미지원으로 실패하고 후 통과했다.
기존 borders/export 음성 계약의 Dot은 이제 지원하므로 DashDot으로 변경했다.
초기 회귀 export1FAIL은 이 오래된 거부 기대값이며 실행 결함과 구별한다.
Dot16종 최종 획 길이/주기는96/192dpi에서 검사하고, 추가 분할 계약으로 일반/
중첩 표의 반복 제목·후속 내용·외곽/텍스트 좌표 보존과 중복 획 부재도 검사한다.

Native 정상 표의 compare/review/standalone overlay를 직접 열어 내부 표3개의
전체 행·합계·주석·외곽을 확인했다. 대체 글꼴 폭/굵기·잉크 위치 차이는 남는다.
원본 전체 실행은 문단30을 지나 문단44 `stored body anchor mode`로 진행하며,
이는 다음 절편 대상으로 남겼다. 카탈로그 전체 본문은 기존 단일 LineSeg 수용
조건에 걸리므로 선택 표의16종 펜 검증을 전체 카탈로그 조판 통과로 바꾸지 않는다.

**최종 검증:** document_flow160 + text43 + ir_text9 + nested19 + borders34 +
export36 + page_number15 + timeline5 + fresh_full1 + Legacy7158 3 +
rowspan_roundoff2 + cell_dash2 + inline_bound2 + cell_dot2 = **333 PASS/0 FAIL**.
`run-policies.sh inline-dot-regression ...` 및 변경된 borders/export의
`inline-dot-final` 로그가 최종 결과다. Native build19.66초, Native/WASM lib
Clippy31.19/30.35초 PASS. 명시 변경 파일 rustfmt check, git diff check PASS.
전체 CI와 workspace/all-target Clippy는 이번 절편에서 실행하지 않았다.

Docker fresh WASM 빌드7분21초·종료0,
SHA256 `da7737e58d1e104d128b02c20c41339d0b70ac5d315572aa1d22f5d1fbbbd46b`.
`node output/7353/r19/inline-bounds/capture.mjs --wasm` 종료0.
동일 정상 HWP1쪽의 Native/fresh WASM 전체 노드 차이0, SVG동일이다.
`review/wasm-review-1.png`, `wasm-overlay-1.png`를 직접 열어 내부 표3개의
행/합계, 부모 외곽, 아래 주석을 확인했다. 한컴 PDF와의 글꼴 외형 차이는 남는다.
원본 전체 통과는 아니다. 메인테이너가 후속 응답에서 시각 판정 통과와
다음 절편 진행을 승인했다.
source HEAD `50823731af6050c60ec3cfcc898abf36daa6b35a` + WIP,
`inline-bounds/source.sha256`, `tests.sha256`, `review/run.json`으로 입력/기준PDF/
코드/WASM을 고정했다. 빌드 후 production 변경 없이 SHA 검증을 완료했다.

### 2026-09-28 — 문단44 본문 어울림 표와 후속 저장 줄

이전 문단30 높이/Dot 절편의 시각 통과를 반영했다. 다음 차단 지점은 원본
문단44의 Square/BothSides 표였다. Legacy는 변경하지 않고 V2의 저장 줄
경로에 같은 쪽의 온전한 어울림 표 배치를 추가했다. 증적은
`output/7353/r19/body-column-anchor/`에 있다. 폴더 이름과 달리 실제 속성은
Column 기준이 아니라 Para/Left 및 Para/Top이다.

독립 입력은 원본 문단43~46을 분리해 한컴에서 정상 저장하고, 그 HWP를
인쇄한 PDF1쪽이다. 생성 변경점·job·SHA·저장 메트릭은
`tests/fixtures/issue7353_body_wrap_review/README.md`에 연결했다. 첫 초안의
앞쪽 나눔 잔존 오류는 정상 입력의 엔진 결함 증거와 구별한다. 최종 정상
HWP의 이전 실행파일은 `before3.log`에서 `stored body anchor mode`로 거부하며,
수정 후 동일 HWP는1쪽을 생성한다. 새 formal4개를 모두 이전 library에서
실행한 것은 아니므로 그 전체를 수정 전 FAIL 증거로 주장하지 않는다.

규칙/실제 경로: `body_anchor::resolve`가 저장 호스트 줄의 실제 끝점에서
Para offset·바깥여백을 반영한 객체 원점을 만든다. `document_input`은
Square 객체 query 폭만 용지 우측까지 허용하고, 본문 줄 폭은 유지한다.
`body_flow::AnchoredFlow::fit`과 `BodyCursor::fit`은 동일 객체 결과를 받아
물리 점유 끝점을 예약하되 후속 본문 pen을 표 아래로 밀지 않는다.
`validate_side_wraps`가 최종 배치된 표+여백과 본문 줄/다른 표의 교차를
검사하며 `document::next_page`에서 paint/commit 전에 소비한다. 같은 helper
호출만을 공통 결과로 간주하지 않고 마지막 좌표에서 저장 좁은 줄의 유효성을
확인한다. 호스트 빈 줄과 후속 빈 줄의 높이도 유지한다.

첫6줄은 표 왼쪽 저장 폭을, 마지막 줄은 전체 본문 폭을 사용한다. 표가
본문 우측을 조금 넘는 정상 출력은 용지 내부 객체 배치와 구별했다. 기존
TopAndBottom의 본문 경계는 완화하지 않았다. 쪽을 건너는 Square 객체,
부분 fit, 호스트와의 겹침 및 fresh 어울림 재조판은 이번 구현 범위가 아니며
명시 거부한다. 행 컷/rowspan/캡션·각주 정책은 비변경이다. 예산 실패는
표 일부나 저장 좁은 줄만 먼저 commit하지 않는 것으로 검사한다.

정식 회귀4개는 정상 HWP 및 HWPX round-trip의96/192dpi 최종 좌표,7개 줄의
내용·폭·소유, 빈 줄·쪽 번호·종료를 검사한다. 반례는 좁은 줄을 전체 폭으로
변경하거나 객체를 왼쪽으로 침범시킨 경우, 용지 밖/저장 호스트 없음 및
객체 전체가 못 들어가는 페이지 예산이다. 실패 후 emitted_pages=0과 반복
호출 거부를 확인한다. 기존 document_flow의 Square 미지원 기대값1FAIL은
지원 확장으로 바뀐 계약이다. 해당 음성 항목을 여전히 미지원인 Tight로
바꾸고 새 Square 양성·음성 계약을 별도 유지했다. baseline 완화가 아니다.

집중 검사 최종 document_flow160 + body_square4 + body_exclusion10 + text43 +
ir_text9 + nested19 + borders34 + export36 + page_number15 + timeline5 +
fresh_full1 + Legacy7158 3 + inline_bound2 + cell_dot2 = **343 PASS/0 FAIL**.
`frame-after-child/body-square-regression-*`와 document_flow 재실행
`body-square-final-*`을 따른다. Native build 및 Native/WASM lib
Clippy33.50/30.32초 PASS. 전체 CI·workspace/all-target Clippy는 미실행이다.

원본 전체 입력은 문단44를 지나 문단52 `stored body anchor intersects host
flow`에서 준비가 멈춘다(`full-after.log`). 전체 문서 통과나 다음 앵커의
지원 완료로 보고하지 않는다. Native review/standalone overlay에서 제목,
오른쪽 빈 표, 왼쪽6줄/전체 폭 마지막 줄, 쪽 번호를 직접 확인했으며
대체 글꼴 굵기·잉크 baseline 차이는 남는다. fresh WASM 검증은 아래에 기록한다.

**최종 fresh WASM/Visual Sweep:** Docker 빌드7분25초·종료0,
WASM SHA256 `5527daedbd801af98073d9eac9235ace30f2dc0f2faa97330559415d158b8666`.
`node output/7353/r19/body-column-anchor/capture.mjs --wasm` 종료0.
동일 정상 저장 HWP1쪽에서 Native/fresh WASM 전체 노드 차이0, SVG동일이다.
`review/wasm-review-1.png`, `wasm-overlay-1.png`를 직접 열어 제목 표, 오른쪽
빈 표와 왼쪽6줄, 표 아래 전체 폭 마지막 줄, 쪽 번호를 확인했다. 글꼴 외형
차이는 남는다. 메인테이너가 시각 판정 통과를 확인했다. 제목 셀의 시작 위치
차이에 대한 후속 확인은 따옴표 `“`, `”`의 글자 폭·글립 차이로 정리했고,
셀 여백/좌표 보정을 추가하지 않는다.
source HEAD `50823731af6050c60ec3cfcc898abf36daa6b35a` + WIP이며
`source.sha256`, `tests.sha256`, `review/run.json`으로 코드/입력/PDF/WASM을
고정했다. 빌드 후 production 변경은 없으며 manifest 검증, 명시 파일
rustfmt check와 git diff check는 PASS다. Legacy 기본 경로는 비변경이다.

### 2026-09-28 — 문단52 빈 호스트 줄 옆의 Square 앵커

앞 절편의 시각 통과와 다음 진행 승인에 따라 문단52의 거부 조건을 처리했다.
증적은 `output/7353/r19/host-side-wrap/`이다. 원본의 빈 호스트 줄800HU와
표 상단498+138=636HU가 세로로 겹치지만, 호스트의 저장 폭26319HU는 표의
왼쪽 제외 영역 시작에 접한다. 기존 `body_anchor::resolve`는 두 차원의
영역을 확인하기 전에 TopAndBottom과 같은 세로 분리 조건으로 거부했다.

입력/기준은 원본 문단51~55 분리본을 한컴에 저장하고 같은 HWP를 인쇄한
PDF1쪽이다. `tests/fixtures/issue7353_host_wrap_review/README.md`에 생성
변경점·job·SHA·독립 메트릭을 기록했다. 표의 y 구간이 호스트와 공유되어도
왼쪽 줄은 살아 있어야 한다. 줄을 지우거나 객체 원점을 호스트 아래로 clamp하지 않는다.

생산/소비 경로: `body_anchor.rs:resolve`의 `host_end_offset`은
`표 offset+위여백 - 호스트 occupied_end`라는 부호 있는 **객체 좌표**다.
`document_input.rs`가 Square의 선행 Space를0으로 하고 같은 값을
`AnchoredFlow`에 전달한다. `body_flow.rs:BodyCursor::fit`은 수용된 호스트
전체 줄의 실제 끝점에 이 값을 더해 query 원점/남은 예산을 결정한다.
`AnchoredFlow::fit`은 실제 표+아래여백을 예약하고 `BodyFit::accept`는 그
객체 끝점으로 본문 점유를 갱신하되 본문 pen은 전진시키지 않는다.
기존 `validate_side_wraps`가 최종 줄·표+여백의2차원 겹침을 검사하고
`document.rs:next_page_json`이 paint/commit 전에 소비한다.

TopAndBottom의 세로 분리 조건은 유지한다. Square 호스트가 이전 페이지에서
이어진 일부 줄이면 저장 문단 원점으로 환산할 수 없으므로 온전한 host의
소유 줄 수를 대조해 명시 거부한다. 일반/rowspan 표의 컷·반복 제목·paint
높이는 비변경이고, 이 절편의 Square는 온전한 같은 쪽 객체만 수용한다.

새 정식 계약6개는 정상 HWP/HWPX 96/192dpi 최종 좌표,5개 본문 줄·주석의
누락/중복과 빈 줄/쪽 번호/종료를 검사한다. 빈/가시 HOST 줄의 안전한 옆
공간과 전체 폭 침범, TopAndBottom 반례, 실제 점유 끝의 ±1HU 예산,
host5줄이 나뉘는 경계를 포함한다. 비수용 페이지의 emitted_pages 불변과
재호출 거부도 확인했다. 가시 HOST 변이에서 PageNumberPos를 함께 두었던
초기2FAIL은 별도 미지원 선언 조합의 테스트 작성 오류로, 선언을 제외해
이 계약이 겨냥한 가시성/점유 차이만 검사했다.

정상 저장 입력의 핵심 계약은 수정 전 `anchor intersects host flow`로 실패하고
수정 후 통과했다(`frame-after-child/host-square-before-*`, `host-square-final-*`).
6개 최종 계약 전부를 이전 library에서 실행한 것은 아니다. 변경 전 실행파일도
동일 정상 HWP를 같은 이유로 거부한다(`normal-before.log`). 수정 후1쪽 출력이다.
원본 전체의 다음 차단은 문단61 `stored TAC carrier requires unambiguous intact
rows`다(`full-after.log`). 원본 전체 통과로 보고하지 않는다.

Native review에서 제목 표, 실제 빈 줄 높이와 오른쪽 표 시작·하단, 왼쪽5줄,
작은 빈 줄·주석·쪽 번호를 직접 확인했다. 따옴표를 비롯한 글꼴 폭/글립·굵기
차이는 별개로 남긴다. 최종 회귀/fresh WASM 결과는 아래에 연결한다.

집중 회귀 최종 host_square6 + body_square4 + body_exclusion10 + host_anchor_gap4 +
document_flow160 + text43 + ir_text9 + nested19 + borders34 + export36 +
page_number15 + timeline5 + fresh_full1 + Legacy7158 3 = **349 PASS/0 FAIL**.
`frame-after-child/host-square-regression-*`와 가시 다중 줄 host의 온전한 배치
대조까지 추가한 `host-square-final-issue_7353_host_square_wrap.log`를 따른다.
Native build와 Native/WASM lib Clippy29.78/33.13초 PASS다. 전체 CI와
workspace/all-target Clippy는 이번 절편에서 미실행이다.

이전 문단44 대조군도 현재 코드로 다시 출력했다. 이전 승인 입력의
`body-column-anchor/normal-after/native.json`과 이번
`host-side-wrap/control-after/native.json`은 `cmp`로 완전히 동일하다.
이 검사는 현재 최종 코드의 새 원점 계산이 호스트 다음에 시작하는 기존
Square 배치를 바꾸지 않았다는 근거이며, 다음 문단61의 지원 증거가 아니다.

**최종 fresh WASM/Visual Sweep:** Docker 빌드7분23초·종료0,
WASM SHA256 `fc619b46a03e8459deecc25b51ffb338b36bd8a606baaa395647bcd2b6baba11`.
`node output/7353/r19/host-side-wrap/capture.mjs --wasm` 및
`control-capture.mjs --wasm` 종료0. 정상 저장 후보/이전 대조군 각1쪽에서
Native/fresh WASM 전체 노드 차이0, SVG동일이다. 후보 `review/wasm-review-1.png`,
`wasm-overlay-1.png`와 대조군 `control-review/wasm-review-1.png`를 직접 열어
빈 줄 옆 표의 시작·하단, 제목/본문/주석, 기존 옆6줄·전체 폭 복귀 보존을
확인했다. 글꼴 차이는 남기며 메인테이너가 시각 판정 통과를 확인했다.
source HEAD `50823731af6050c60ec3cfcc898abf36daa6b35a` + WIP를 기준으로
`source.sha256`, `tests.sha256`, 각 `run.json`에 코드·입력·PDF·WASM을
고정했다. 빌드 후 production 변경 없이 해시 검사, 명시 파일 rustfmt check,
git diff check를 통과했다. PR/push/기본 엔진 전환과 전체CI는 수행하지 않았다.

### 2026-09-28 — 승인 상태 baseline 커밋과 과거 worktree 정리

작업지시자는 현재 승인 상태를 `task_m100_7353`에 커밋하여 후속 작업의
기준점으로 보존하고, 과거 검토용 worktree 7개의 변경은 각각 로컬 보존
브랜치에 기록한 뒤 제거하도록 승인했다. devel 및 원격은 변경하지 않는다.
이 커밋은 위 검증된 소스와 현재 작업 자료의 checkpoint이며 R5 완료나
전체 CI 통과를 의미하지 않는다. 문단61 TAC 줄 구성은 다음 작업으로 남는다.

과거 검토용 WIP는 현재 승인된 조판 상태와 섞지 않는다. 해당 HEAD·보존
커밋·ignored 증적의 보관 위치와 삭제 결과는 기본 저장소의
`output/cleanup-20260928/`에 남긴다. #7353의 원본·시각 증적과 실행파일은
그대로 유지한다. 불필요한 검증 재실행이나 baseline 기대값 변경은 하지 않는다.

### 2026-09-28 — 셀 안 non-TAC 자리차지 그림의 점유/배치 연결

baseline `cc13f573a43ba96093198f9bcf93ad6ca1c8bd38`에서 후속 절편을 진행했다.
앞 기록의 “문단61 TAC 줄 구성”은 오류 메시지에 따른 분류였으며 정정한다.
실제 원본 문단61의 바깥 표는 Square/BothSides, 셀 안 그림은 non-TAC
TopAndBottom이다. 빈 호스트 줄을 TAC 그림 전용 composer에 전달하여
`stored TAC carrier requires unambiguous intact rows`로 거부하던 경로다.

`pictures.rs:compose_excluded_cell`에서 단일 열 셀의 Column/Left 또는
Para/Left, Para/Top 기준 그림을 별도로 구성한다. 지원 범위는 문단 안 여백0,
저장된 단일 빈 호스트 줄/단일 그림/자리차지이며, 다른 조합은 명시 거부한다.
그림 원점은 offset+바깥여백, 공통 점유는
`max(호스트 줄높이+줄간격, 그림 y+높이+아래여백)`이다. 호스트의 실제 줄높이와
기준선은 그대로 두고 그림과 함께 하나의 원자적 점유 그룹으로 보존한다.

생산/소비 경로: `text_ir.rs:compose_items` →
`pictures.rs:compose_excluded_cell`의 ObjectRow/Group 동일 bounds →
`ir.rs:503`의 소유 슬롯검사와 `FlowBlock::Lines` 높이/advance →
기존 content/fragment fit/셀 중앙정렬 → `text.rs:239`의 payload clone/translate.
paint에서 높이를 다시 늘리거나 clamp하지 않는다. 일반/rowspan의 컷 알고리즘,
TAC 경로와 기본 엔진은 비변경이다. 이 그림 그룹은 분할하지 않고 부족하면
빈 호스트와 함께 이월한다. 더 넓은 그림 side-wrap·편집 후 재조판은 미지원이다.

독립 기준은 source 문단60~63 분리본의 한컴 HWP 저장/PDF다. 원본 그림이
흰색이므로 그림 데이터만4색 패턴으로 바꾼 대조군도 한컴 저장/출력했다.
원본은 보존하며 속성·저장 줄은 임의 수정하지 않았다. 입력 생성 코드, 전체
변경점, converter job, 파일 해시는
`tests/fixtures/issue7353_cell_picture_review/README.md`에 있다.
색상 이미지의 PDF 사각형은360.766/118.79001/177.505/135.452pt이고,
저장HU로 계산한 그림 상단은 셀 상단+539HU다. Native review/overlay에서
그림4변·색상 경계·표 외곽·제목과 왼쪽7줄을 직접 확인했다. 폰트 외형 차이는 남는다.

정식 계약6개는 HWP/HWPX96/192dpi 최종 좌표·본문/그림/빈 줄 보존,
그림 여백·작은 그림에서도 유지되는 호스트 줄, 점유13825HU보다1HU 부족한
예산의 거부/정확한 예산의 수용, 후속 빈 문단 이월/중복 없는 종료를 검사한다.
Square/Page 기준/폭 초과/음수 offset/겹침 허용/가시 호스트/다중 줄은 반례다.
그림 margin과 높이/예산 변이는 합성 계약으로, 한컴 관측으로 확대하지 않는다.

정상 HWP는 변경 전 실행파일에서 해당 TAC 오류로 실패(`before.log`), 변경 후
1쪽 출력이다. 정식 fit 계약도 보관된 변경 전 library에서 동일 오류로 FAIL,
현재 코드에서 PASS다(`before-contract-{build.log,log}`). 이 library는
`frame-after-child/librhwp-before.rlib` 보관본이며 정확한 baseline 전체 빌드를
재실행한 것은 아니다. 변경 전6개 전수검증으로 보고하지 않는다.
초기 테스트 작성 중 Debug bound/존재하지 않는 필드 오류를 고쳤고, 문단의
명시 개행은 렌더 TextRun 텍스트에 포함되지 않으므로 줄 수와 내용 보존을
각각 검사하도록 수정했다. 이 작성 오류들은 조판 회귀 건수에 포함하지 않는다.

집중 회귀 **279 PASS/0 FAIL**: 새6 + export36 + document_flow160 + text43 +
host_square6 + ir_text9 + nested19. 후속 빈 문단 경계를 추가한 최종6개도 PASS.
`output/7353/r19/cell-floating-picture/tests{,-final}.log`와
`frame-after-child/cell-picture{,-final}-*.log`를 따른다.
Native build, Native/WASM lib Clippy53.25/58.41초 PASS, 명시 파일 rustfmt와
diff check PASS다. 원본 전체의 다음 차단은 **문단137 `TAC row exceeds stored width`**
(`full-after.log`)이며 전체 문서 통과 또는 R5 완료로 보고하지 않는다.

Docker 최초 실행은 네트워크 주소풀 고갈로 빌드 시작 전에 실패했다.
다른 네트워크를 삭제하지 않고 output의 compose override에서 기존 bridge를
사용하여 표준 wasm 서비스를 다시 실행했다. fresh WASM 결과는 아래에 연결한다.

최종 Docker 빌드 **7분52초/exit0**. WASM SHA256
`ae76ec5f42f2a0336ba81462530c844897ce3bf44bd54735b23bbb75dbf1ed34`.
`node output/7353/r19/cell-floating-picture/capture.mjs --wasm`는 첫 시도에서
Chrome 시작 실패 후 같은 명령 재시도로 종료0이다. Native/fresh WASM의
전체 JSON 노드 차이0, SVG동일(`review/backend-comparison.json`)이다.
`review/wasm-review-1.png`, `wasm-overlay-1.png`를 직접 열어 오른쪽 그림의
4변·셀 여백·외곽과 제목/왼쪽7줄 보존을 확인했다. 메인테이너 시각 판정은
대기 중이다. 이전 승인 host-side-wrap 샘플도 현재 Native로 재출력하여
이전 JSON과 `cmp` 동일을 확인했다(`control.log`).

코드 HEAD는 위 baseline+WIP이며 source/test/input/PDF/WASM 해시는
`source.sha256`, `tests.sha256`, `review/run.json`에 연결했다. 빌드 후
production 변경은 없다. 전체CI/workspace-all-target lint, push/PR/기본 엔진
전환은 수행하지 않았다. 이번 결과는 문단61 셀 그림 경로의 지원/검증이며,
원본 전체 다음 절편인 문단137과 R5 전체 완료를 대신하지 않는다.

메인테이너가 위 셀 자리차지 그림의 시각 판정을 통과로 확정하고 다음 절편을 승인했다.

### 2026-09-28 — 저장 줄보다 넓은 단일 TAC 그림

원본 문단137은 Square 표의 셀에 TAC 그림1개를 둔다. 원본과 한컴 정상
재저장본 모두 그림 폭19686HU, 줄 폭19604HU, 셀 폭19607HU를 유지한다.
기존 `tac.rs:object_rows`는 단일 TAC 표의 초과 폭만 허용하고 그림은 거부했다.
독립 PDF에서는 중앙 정렬 문단의 그림을 축소하거나 음수 offset으로 이동하지
않고 줄 시작점에 놓는다. 따라서 수치 허용치를 추가하지 않고 기존 단일
분할 불가 객체 규칙을 표/그림이 공유하도록 했다. 여러 객체·선행 내용이나
잘못된 저장 기준선 높이를 함께 허용하는 변경은 아니다.

입력/독립 근거: 원본136~139 분리본을 보존하고, 판독용으로 그림 BinData만
4색 패턴으로 바꾼 대조군을 한컴 HWP/PDF로 저장했다. 원본 자체와 크기·줄
정보는 변경하지 않았다. 생성 코드·변경 범위·job·SHA256·PDF 그림 사각형은
`tests/fixtures/issue7353_overwide_picture_review/README.md`를 따른다.
정상 재저장 후에도 같은 폭 차이가 유지되므로 임의 LineSeg 수용 완화가 아니다.

생산/소비 경로: `tac.rs:object_rows`는 저장 제어 슬롯으로 줄 소속을 정하고
물리 객체 폭+바깥여백에서 정렬 여유를 구한다. 초과 폭 단일 객체는 정렬 여유0,
객체 폭/높이는 유지한다. `pictures.rs:compose`는 저장 줄의 가용 lane 폭과
Image의 실제 사각형을 서로 다른 필드에 보존한다. `ObjectRow` →
`ir.rs:503`의 `FlowBlock::Lines`가 공유하는 세로 점유/advance로 fit하며,
`text.rs:239`는 같은 payload 전체를 translate한다. 저장 줄 폭을 그림 폭으로
바꾸거나 그림을 셀 폭으로 축소·clip하지 않는다. 셀 오른쪽79HU 돌출까지
최종 Image와 Table 사각형으로 검사한다. 분할/rowspan 컷 알고리즘은 비변경이며
그림을 쪼개지 않고 전체 높이가 들어갈 때 호스트 줄과 함께 수용한다.

새 정식 계약5개는 정상HWP/HWPX96/192dpi 그림·줄·표 최종 위치/크기와
본문2문단 보존, 폭 경계(19603/19604/19605/19686HU)의 Left/Center/Right,
여러 그림·선행 공백 비적용,1HU 부족한 세로 예산, 후속 빈 문단 이월/종료를
검사한다. 폭 변이/정렬 조합/예산은 합성 경계이며 독립 한컴 관측과 구분한다.
초기 반례2건은 테스트 옵션에 cell-end 정책을 생략하여 제목 표에서 먼저
실패했다. 정상 테스트와 같은 정책을 명시한 뒤 의도한 거부 원인까지 확인했다.
구현을 테스트 통과에 맞춰 바꾼 것은 아니다.

변경 전 실행파일은 정상 저장본을 `TAC row exceeds stored width`로 거부
(`tac-width-next/before.log`), 변경 후1쪽이다. 정식 fit 계약도 보관된 변경 전
library(`frame-after-child/librhwp-before.rlib`)에서 같은 원인으로 FAIL,
변경 후 PASS다. 정확한 baseline 전체 빌드나 이전5건 전수실행은 하지 않았다.
원본 분리 HWPX도 변경 후1쪽이며 흰색 그림 원본과 색상 대조군을 구별한다.

집중 회귀 **278 PASS/0 FAIL**: 새5 + 직전셀그림6 + export36 +
document_flow160 + text43 + ir_text9 + nested19.
`output/7353/r19/tac-width-next/tests.log`와
`frame-after-child/overwide-picture-*.log`에 연결한다.
Native build17.95초, Native/WASM lib Clippy27.86/30.00초 PASS다.
명시 파일 rustfmt/diff check PASS. 직전 승인 셀 그림 샘플도 현재 Native로
재출력해 기존 JSON과 `cmp` 동일이다(`control.log`).

원본 전체는 이번 변경으로 **DocumentV2 초기 구성 성공**까지 진행했다.
실제1쪽 출력은 `conflicting shared V2 cell borders`로 거부된다
(`full-progress.log`). 전체 페이지 출력 통과나 R5 완료로 보고하지 않으며,
테두리 충돌 처리는 다음 절편으로 분리한다.

Native review를 직접 열어 오른쪽 그림 시작/전체 폭·높이와 제목/본문 배치를
확인했다. 대체 글꼴 외형 차이는 남긴다. fresh WASM/최종 시각 증적은 아래에 연결한다.

최종 Docker WASM 빌드 **7분25초/exit0**, SHA256
`bfcf661c1f32660fc0fb867b40535049d2727bc6389bbf1cd6cca7a4ba3abf42`.
`node output/7353/r19/tac-width-next/capture.mjs --wasm` 종료0이며,
Native/fresh WASM 전체 JSON 노드 차이0·SVG 동일이다
(`review/backend-comparison.json`). `review/wasm-review-1.png`와
`review/wasm-overlay-1.png`를 직접 열어 그림의 시작점·전체 프레임과
왼쪽 본문 줄바꿈을 확인했다. 글꼴 외형·부제목 기준선 차이는 남아 있으며
그림 배치 검증과 구분한다. 이번 절편의 메인테이너 시각 판정은 대기 중이다.

본문 두 문단의 줄 수/위치/폭과 첫 조각의 예약 높이 검사를 보강한 최종
신규 계약5건도 PASS(`tests-final.log`)다. production은 빌드 이후 불변이며
`source.sha256`, `tests.sha256` 전수 재확인과 `git diff --check` PASS다.
source는 `cc13f573a43ba96093198f9bcf93ad6ca1c8bd38`+WIP이며,
입력/PDF/코드/WASM 식별자는 `review/run.json`에 연결했다.
전체 CI/workspace-all-target lint, push/PR, 기본 엔진 전환은 수행하지 않았다.

메인테이너가 저장 줄보다 넓은 단일 TAC 그림의 시각 판정을 통과로 확정하고
다음 절편을 승인했다. 다음 대상은 원본 첫 표의 공유 경계에서 검정0.12mm와
회색0.10mm 실선이 만날 때 발생하는 `conflicting shared V2 cell borders`다.
원본0~1문단을 속성 변경 없이 분리하여 한컴 정상 저장/출력과 대조한다.

### 2026-09-28 — 색이 다른 공유 셀 실선의 paint 소유

한컴 정상 저장본/PDF에서 공유 경계 x368.562pt에 검정0.36pt 다음 회색0.24pt가
같은 중심선으로 출력됨을 확인했다. 색·굵기를 뒤집고 가로 경계에도 다른 색을
놓은 대조군 PDF는 왼쪽/위 셀 다음 오른쪽/아래 셀 순서를 보였다. 기존 V2는
같은 색 실선의 굵기 합집합만 지원하여 서로 다른 색은 거부했다.
입력·수정 범위·job·관측값·SHA는
`tests/fixtures/issue7353_color_border_review/README.md`에 연결한다.
정상 저장본은 원본 그대로가 아니라 처음 두 문단을 분리한 한컴 재저장본이다.
기존 repo PDF 두 개는 Cairo producer여서 이 새 Windows Hancom 출력과
혼용하지 않았다. 기존 PDF 자체가 틀렸다는 판정은 하지 않는다.

수정: `borders.rs:CellBorders::append`의 확정 셀 조각 → 행/열 소유 순서 정렬 →
공유 경계별 `union(spans, cell_layers)` → `LineNode` 두 펜 →
`text.rs:269`의 최종 Table 자식 연결이다. 같은 색은 기존 최대 굵기 결합,
다른 색의 불투명 실선은 개별 paint layer를 보존한다. `zone` 간 충돌은
셀 위치 소유와 다른 계약이므로 계속 거부한다. 복합 선종 혼합도 미지원이다.
측정/fit/컷/예약/종료는 비변경이고, 이미 확정한 조각 사각형만 소비한다.
출력 색을 맞추려고 셀/행/표 기하를 바꾸지 않았다.

정식 새 검사4건은 실제 최종 두 펜의 색·굵기·순서·좌표(96/192dpi), 가로/세로
대조군, 셀 벡터 역순, 색 제거 대조군과 Table/TableCell/TextLine/TextRun 기하
일치를 확인한다. split 검사1건은 두 조각(36px/18px)의 선 끝점·두 펜과
L,A,B → C 내용 소유 및 정상 종료를 검사한다. 기존 오류/재시도 검사2건은
이제 지원한 서로 다른 실선 색 대신 미지원 Dash/Solid 혼합으로 대상을 변경했다.
원래 실패를 숨긴 것이 아니라 새 성공 계약과 남은 거부 계약을 분리했다.

수정 전 실행파일은 이 정상 저장본에서 공유 테두리 충돌로 FAIL(`before.log`).
보관된 이전 library로 새 실물 계약을 실행했을 때는 더 이른 TAC carrier 제약에서
실패하여 이 결함 검출 증거로 세지 않는다(`before-contract.log`). 같은 library에서
새 split 계약은 정확히 공유 테두리 충돌로 FAIL(`before-split.log`), 수정 후 PASS다.
정확한 baseline 전체 재빌드는 하지 않았으며 위 두 경로를 구별한다.

집중 **240 PASS/0 FAIL**: 새color4 + borders34 + split12 + document_flow160 +
직전overwide5 + cell_picture6 + nested19. 명령/로그는
`output/7353/r19/shared-color/{tests,tests-final}.log`와
`frame-after-child/color-borders*.log`에 연결한다. Native build28.12초,
Native/WASM lib Clippy27.81/30.35초 PASS. 명시 파일 rustfmt/diff check PASS.
직전 승인 TAC 그림을 재출력해 이전 전체 JSON과 cmp 동일을 확인했다.

원본 전체 `DocumentV2Session`은 **17쪽을 실제 출력하고 정상 종료**했다
(`full-progress.log`). 새로운 미지원 오류는 없지만 이 실행은 문서 종단 수용
증거이며, 전체17쪽의 한컴 시각 일치 또는 R5 전체 완료를 뜻하지 않는다.
분리본/색 반전 대조군 Native review를 직접 열어 공유선·표 외곽·뒤 제목을
확인했다. 글꼴 외형·일부 글자 위치와 PDF printer 양자화 차이는 남긴다.
fresh WASM 및 최종 시각 증적은 아래에 연결한다.

병합 반례를 추가했다. 오른쪽 셀이 rowspan2인 한컴 직접 출력에서는 시작 행이
같은 첫 구간은 검정→회색, 오른쪽 셀이 더 이른 행에서 시작한 다음 구간은
회색→검정이다. 따라서 위의 왼쪽/위→오른쪽/아래 설명은 시작 행/열이 같은
경우이며, 실제 구현의 원본 셀 시작 `(row,col)` 소유 순서를 정식 검사로 확인했다.
추가1건 포함 신규color5/전체집중 **241 PASS/0 FAIL**이다(`tests-span.log`).
이 검증 중 production 변경은 없었다.

Docker fresh WASM **7분21초/exit0**, SHA256
`06b5c0e501e6dda09b05e1e7deaccc78a44c65cb3eaae9bf9de0b656a99abb42`.
`capture.mjs --wasm`, `--variant --wasm`, `--span --wasm` 모두 종료0이며,
각 Native/WASM 노드 차이0·SVG동일이다. 각 review/overlay와 공유선384dpi
확대 비교 PNG를 직접 열었다. source/input/PDF/WASM은 각 `review/run.json`,
`source.sha256`, `tests.sha256`에 고정했다. source manifest와 diff check 재확인 PASS.

원본 전체도 `full-wasm.mjs`에서 **Native17/WASM17, 정상 종료, 전체 노드 차이0,
17개 SVG동일**을 확인했다(`full-wasm.json`). 전체 문서의 한컴 시각 검증은
이 backend 동등성으로 대신하지 않는다. 메인테이너의 이번 공유선 시각 판정은
대기 중이다. 전체 CI/workspace-all-target lint, push/PR, 기본 엔진 전환은
수행하지 않았다. 다음 범위는 더 이상 동일 원본의 미지원 예외 추적이 아니라
원본 전체 시각 검증과 잔여 R5 완료 계약의 점검이다.

메인테이너가 공유선 시각 판정을 통과로 확정하고 다음 진행을 승인했다.
다음 묶음은 같은 원본17쪽의 전체 시각 대조다. production/source manifest가
동일하므로 직전 Native/fresh WASM 빌드와241건 집중 검증은 재사용한다.
원본 HWPX를 재저장하거나 속성을 바꾸지 않고 Windows Hancom 직접 PDF를
획득하여 전체 문서의 표·문단·분할·종료를 확인한다. 전체 CI 및 기본값
전환은 이번 실행과 구분한다.

### 2026-09-28 — 원본17쪽 전체 검증과 잔여 완료 조건

Windows Hancom 직접 PDF job `2e11ab6c-00a7-41dd-9a4a-c3b8ec05c17c`는19초에
17쪽으로 완료했다. `input_preprocess=none`, Hancom11.0.0.9136이다.
원본의 정상 저장 분리본이 아니라 **같은 원본 HWPX를 그대로 출력**했다.
입력/PDF SHA와 독립 관측값은 `tests/fixtures/issue7353_sandbox_review/README.md`,
job의 전체 응답은 `output/7353/r19/document-review/pdf-{start,status,download}.json`이다.

`node output/7353/r19/document-review/capture.mjs --wasm` 종료0.
동일 source의 Native17쪽을 재사용하고 현재 fresh WASM 패키지로 원본을 다시 실행했다.
전체 노드 차이0·17개 SVG 동일이다. `run.json`은 baselineSHA+WIP source manifest,
동일 원본/PDF/WASM hash를 고정한다. 코드/패키지 비변경이므로 이전241건 집중검사와
빌드/lint를 새 실행인 것처럼 합산하지 않는다. 기본 Legacy CLI용 sweep/fidelity
원장을 V2 결과라고 사용하지 않았으며, 실험 DocumentV2 직접 capture 경로다.

WASM `wasm-review-1.png`부터 `wasm-review-17.png`까지 전부 직접 열었다.
Native2/9쪽 및 standalone `wasm-overlay-12.png`도 직접 대조했다.
전체 compare/overlay/review 링크는 `document-review/index.html`에 모았다.

| 직접 확인한 영역 | 관측 |
| --- | --- |
| 1~4쪽 | 첫 공유선 표·제목·본문·통계 표의 순서/외곽 보존. 2→3쪽 문장 이월 대응 |
| 5~8쪽 | 제목 표와 오른쪽 빈 그림 프레임, 왼쪽 문단 대응. 5→6,7→8 한 줄 이월 보존 |
| 9~14쪽 | 사례5~16의 제목/그림 옆 좁은 줄과 프레임 아래 넓은 줄, 후속 문단 대응 |
| 15~17쪽 | 15→16쪽 문장 이월,17쪽 사례19/20과 마지막 문장·쪽번호 보존 |

검토 범위에서 큰 배치 차이·누락·중복·겹침을 발견하지 못했다. 대체 글꼴의 굵기,
기호/따옴표 폭, 제목 glyph·일부 기준선과 한컴 printer scaling 차이는 남는다.
글꼴까지 픽셀 일치한다거나 Studio Canvas 편집까지 검증했다고 보고하지 않는다.
원본의 빈 그림 placeholder는 한컴에도 비어 있으므로 V2 그림 누락으로 분류하지 않는다.
작업지시자의 전체 원본 시각 판정은 대기 중이다(직전 공유선 판정과 구분).

정식 `tests/cases/issue_7353_sandbox_document.rs` **2 PASS/0 FAIL**:
17쪽 정상 종료·4개 경계 문장 소유/유일성·마지막 내용/쪽번호,96/192dpi에서
5쪽 최종 두 제목 표의 실제 좌표를 독립 PDF path와 대조했다.
초기2실패는 마지막 문장 전사의 오류와 PDF printer scaling을 생략한 테스트 오류였다.
PDF text/path/transform으로 수정했고 production은 바꾸지 않았다.
원래0.5pt 허용폭을 늘리지 않고 독립 printer scale을 적용한0.24pt(600dpi 두 dot)
경계로 검사했다. 초기 로그는 `contract-initial.log`, 최종은 `contract.log`다.
이 계약은 새 결함의 수정 전 FAIL/후 PASS가 아니라 현재 지원 경로의 보호 검사다.
rustc `-D warnings`, 명시 rustfmt check, diff check PASS. 전체 integration suite 등록/CI는
별도 통합 게이트에서 수행하며, 이번 standalone 실행을 CI 통과로 보고하지 않는다.

잔여 범위를 코드와 대조했다. `document.rs:DocumentV2Session` 및
`wasm_api/table_v2_preview.rs:DocumentV2`는 독립 snapshot/nextPage API다.
`rhwp-studio/src`에는 이 API의 실문서 편집 연결이 없다. 기존 session 검사에서
재준비/스냅샷 격리는 보호하지만 undo/redo/커서/저장 후 재열기 통합의 증거는 아니다.
`document_input.rs:prepare`의 복수section/grid/세로방향과
`ir.rs:validate_table`의 caption/cell-spacing 거부도 남아 있다.
따라서 원본17쪽의 종단 수용·시각 검토 완료를 R4/R5 전체 완료로 승격하지 않는다.

다음 우선순위는 계획의 남은 대표 문서(특히 #7008 section/grid)의 기능 경계와
편집/복합 콘텐츠 연결을 한 묶음으로 정리·구현하는 것이다. 이미 승인한 글꼴 차이와
#6923 재저장 판정은 다시 열지 않는다. 이후 최신devel 통합·Legacy/V2 동일입력 비교·
전체CI·기본 경로 전환/복귀 판단이 남는다. 원격 작업·기본 엔진 변경은 하지 않았다.

### 2026-09-28 — 원본17쪽 승인 및 #7008 종단 연결 범위 재점검

메인테이너가 원본17쪽 **시각 검증 통과**와 다음 진행을 승인했다. 위의 전체 원본
판정 대기는 해소됐다. source 변경이 없어 직전 증적·WASM 빌드는 재사용한다.

다음 대표 문서 `samples/21_언어_기출_편집가능본.hwp`를 그대로 파싱하고
DocumentV2 및 선택 표 API를 각각 실행했다. **격자 문제라는 이전 분류는 정정한다.**
`line_grid=0`, `char_grid=0`, `text_direction=0`이며 공통 거부 메시지에
grid라는 단어가 포함돼 있었을 뿐이다.

| 원본에서 확인한 속성 / 실행 결과 | 실제 연결 책임 |
| --- | --- |
| section1개, 본문325문단, flags `0xc0080004`, `hide_empty_line=true` | 문서의 구역/빈 줄 정책. flags 전체를0으로 바꿔 수용하지 않음 |
| `hide_master_page=true`, master_pages2개 | 바탕쪽 표시 조건을 평가해야 함. 두 개 모두 그려야 한다는 뜻이 아님 |
| 첫 문단 ColumnDef: 좌→우2단, 간격2836HU | 문서 단 흐름 및 각 단의 가용 영역 |
| 머리 표 s0:p0:c2,4×5,66492×13740HU, 자리차지, 용지 위 기준9872HU, 용지 가로 중앙 | 단을 가로지르는 표 앵커와 후속 본문 점유 |
| 같은 표 cell2/cell4의 사각형 Shape, 각각 비TAC/TAC | V2 셀 내부 도형/글상자 콘텐츠 및 줄 점유 |
| 전체 문서: `Unsupported("section decoration, grid or writing direction")` | `document_input.rs::prepare`의 첫 수용 검사에서 종료 |
| 원본 선택 표: `Unsupported("non-table cell control")` | `ir.rs::bind_table`이 Shape를 거부. 표 자체도 아직 종단 수용되지 않음 |

재현은 `output/7353/r19/section-scope/probe.rs`, 결과는 `probe.log`다.
원본SHA256 `905454045ca2e236839a7cab59750678116d08af3db31dbf846819af355b8d15`,
기존 대응PDF `pdf/21_언어_기출_편집가능본-2022.pdf` SHA256
`f2d858d7974393661d91a658e6b384b951114ef52783379f426a963effd97b72`.
선택 표 진단의 호출자 기하는 페이지 원본 속성에서 계산했으며 원본 표/셀/컨트롤은
수정하지 않았다. 이 진단은 시각 증적이나 #7008 수용 성공이 아니다.

현재 `BodyPlan`은 단일 `body: Rect`와 한 개 BodyCursor를 소유한다.
`document_input.rs`는 `ColumnDef::default()`로 페이지를 만들고2단 선언을 거부한다.
기존 `DocumentCore`에는 V2 선택 연결이 없고, `typeset.rs::format_table`의
MeasuredTable 및 `pagination.rs::PageItem::PartialTable`은 Legacy 컷 모델이다.
따라서 수용 조건만 완화하거나 V2 측정 높이를 기존 PartialTable에 넣는 것은
잘못된 연결이다. 후자는 Legacy 분할/paint와 섞여 승인된 경로 격리를 위반한다.

**다음 구현 순서 조정 권고(아직 구현하지 않음):** 독립 V2 세션에 다단·바탕쪽 등
문서 엔진을 계속 복제하기보다 기존 문서의 구역/단 흐름을 재사용하는 명시적 V2
어댑터 설계를 먼저 확정한다. 표는 `PreparedTextTable → TextTableCursor::fit →
TextFragment::{geometry,continuation,append_to}`의 동일 결과를 끝까지 소비하도록
전용 조각 전달 경계를 마련하고 Legacy 행 컷으로 변환하지 않는다. 선택은 세션 시작에
고정하고 기본값은Legacy, 미지원은명시적 오류로 유지한다. 그 경계와 함께 셀 내
도형/글상자 지원을 연결한 뒤 원본 문서 전체를 검증한다.

이는 계획1절의 “전체 문서 엔진 재작성 제외”와 C/D 연결을 위한 순서 조정안이다.
기존 독립 세션을 삭제하거나 기본값을 바꾸는 제안이 아니다. 연결 경계의 실제 구현
난이도·공개 API 영향은 아직 미검증이다. 이번에는 product source/테스트/입력/PDF를
변경하지 않았고 불필요한 빌드·CI·새 시각 판정을 반복하지 않았다. 구현 순서 조정에
대한 작업지시자 확인 후 다음 구현을 진행한다.

### 2026-09-28 — 승인된 호스트/V2 조각 전달 경계 구현

작업지시자가 위 권고안을 승인했다. 구현계획5.1에 순서 조정을 반영했다.
이번 변경은 **연결에 필요한 조각 전달 API**이며, 기존 typeset/layout 호출부나
DocumentCore의 엔진 선택이 이미 연결됐다는 의미가 아니다.

`table_v2/host.rs`에 `HostedTableSession`을 구현했다. 호스트는 기존
`PageLayoutInfo`에서 확정한 단 또는 본문 전폭 영역, 구역/쪽/단 주소와 예약 세대를
전달한다. V2가 default ColumnDef로 덮어쓰거나 다음 단/쪽을 자체 선택하지 않는다.
현재 Preview와 호스트의 source 검사는 `session.rs::prepare_selected_table`로 공유한다.
미지원 Shape를 제거하거나 Legacy로 자동 재실행하지 않는다.

소비 경로는 다음과 같다.

`원본 선택 표 + 서식/resources → PreparedTextTable → TextTableCursor::fit_with_page_height
→ HostedTableProposal의 점유 상자 → commit된 HostedTableFragment
→ TextFragment::append_to → 같은 조각의 실제 노드`

- query는 content cursor를 소비하지 않는다. fit 실패/제안 폐기 후에도 같은 내용부터 재시도한다.
- commit은 세션 identity·content revision·호스트 전체 frame/revision이 같은 제안만 수용한다.
  남은 공간/선행 내용이 바뀌면 낡은 제안은 거부한다.
- 확정 패킷은 원본 표 주소·점유 상자·재귀 자식 컷·paint payload를 유지한다. 호스트가 이 상자를
  예약하고 다음 내용을 배치해야 한다. layout은 패킷을 그대로 그리며 Legacy 행 컷으로 변환하지 않는다.
- pagination commit과 paint를 분리했다. 새 페이지 트리에 같은 패킷을 다시 그려도 cursor는 전진하지 않는다.
  다른 쪽/구역/크기의 tree로 전달하면 출력 전에 거부한다.
- 원본 편집은 기존 스냅샷을 바꾸지 않는다. 재준비한 세션만 새 텍스트를 소비한다.
- 포함 영역 검사는 HWPUNIT→px 부동소수점 연산의 수 ulp만 허용한다. 좌표/높이를 clamp하거나
  조판 overflow 허용량을 추가하지 않는다. 외곽선·각주·TAC 문단의 새 조판 규칙을 만든 변경이 아니다.

신규 `tests/cases/issue_7353_table_host_bridge.rs`는 합성 입력의9pt/12px 줄 상자,
18px 고정 전진, 위3/아래4px 패딩 및 기존 페이지 계산의212px×2단/16px 간격을 근거로 검사한다.
쪽별22px의 작은 가용 단을 사용하여 첫 조각21px, 다음18px, 마지막22px와 각 줄의 실제 좌표를
검사한다. 수용 불가 반례는 위패딩3+줄 점유12보다 작은14px다. 초기 검사 작성에서 줄 전진18을
최소 점유12와 혼동한 기대값, 전체120px 단의 next-frame fit 조건을22px 단과 혼동한 설정을
바로잡았다. 이 변경은 신규 전달 API 계약이며 기존 제품 결함의 수정 전 FAIL/후 PASS 주장이 아니다.

실물 `issue7353_color_border_review/saved.hwp`는 원본 그대로 준비하여 기존 선택 표 API와
새 호스트 API의 **전체 render tree 일치**를 검사한다. #7008 원본의 Shape 미지원도 명시적
오류로 유지되는지 검사한다. 이 두 검사는 #7008 전체 문서의 피델리티 판정이 아니다.

검증 명령/로그는 `output/7353/r19/section-scope/host-{build,contract,clippy-native,clippy-wasm}.log`,
관련 회귀는 `output/7353/r19/frame-after-child/host-bridge-*.log`에 둔다.
새 API9건 + session9/export36/sandbox2/color5 = 집중 **61건**이 검증 대상이다.
최종 source/test hash 및 결과는 아래 완료 기록에서 확정한다.

다음 구현은 기존 typeset의 가용 영역/점유 상태에 이 패킷을 예약하는 전용 항목과
layout의 패킷 소비를 연결하는 것이다. `PageItem::PartialTable`에 V2 높이만 대입하지 않는다.
그 뒤 source/편집 무효화와 명시적 세션 선택, 셀 내 도형을 연결한다. 기존 기본값/Legacy,
샘플/기준PDF, baseline/ignore는 변경하지 않았다. 전체 CI·push/PR도 수행하지 않았다.

최종 확인: baseline HEAD `cc13f573a43ba96093198f9bcf93ad6ca1c8bd38` + WIP.
이번 변경 파일은 `section-scope/host-source.sha256`, 신규 계약은 `host-tests.sha256`로
고정했다. `cargo build --locked -p rhwp --lib` PASS, Native 및 wasm32 lib Clippy
`-D warnings` 각각 PASS, 명시 rustfmt check/diff check PASS다. target은 기존
`/home/edward/mygithub/rhwp/target/pr-review`를 재사용했다.
최종 source에서 위61건과 기존 Legacy #7008 검사3건을 실행하여 **64 PASS/0 FAIL**을
확인했다(`host-focused-summary.log`). 새 검사는 별도 `tests/cases/`에 두었고
source unit test·파생 suite·Cargo target registry는 변경하지 않았다.

실제 DocumentCore/Studio 호출부는 아직 이 API를 사용하지 않는다. 따라서 기존 문서
출력 변경이나 #7008 미지원 해소를 주장하지 않으며, 이번에는 Docker fresh WASM 패키지와
새 브라우저/시각 증적을 생성하지 않았다. wasm32 Clippy 통과를 WASM 실행·시각 검증으로
대신하지 않는다. 실제 호출부 연결 후 새 패키지와 영향 페이지를 검증한다. 전체 workspace
lint/CI 및 기본값 전환 판단 역시 남아 있다. 추가 내부 승인 요청 없이 승인된 다음
연결 범위로 이어갈 수 있다.

### 2026-09-28 — 기존 typeset/layout 호출 경로에 V2 조각 연결

작업지시자의 계속 진행 지시에 따라 전달 API의 실제 소비자를 구현했다.
`HostedSectionSession`은 명시적으로 선택하는 불변 실험 세션이다. 기본 DocumentCore/Studio나
기존 독립 `DocumentV2Session`의 엔진을 바꾸지 않는다. 첫 수용 범위는 저장 LineSeg가 없는
일반 본문, 균등 폭 일반 단(LTR/RTL), 오프셋·바깥여백 없는 문단 상단/단 왼쪽 기준
TopAndBottom 표다. 표 안쪽은 기존 V2 자격 검사를 그대로 통과해야 한다.
저장 앵커, TAC/Square 외부 호스트, 중간 구역·단 변경, heading/keep-next/break-before,
도형·각주·바탕쪽 등 미연결 속성은 오류로 남긴다. 이 범위는 원본 #7008 전체의 수용 완료가 아니다.

실제 호출 연결은 다음과 같다.

`HostedSectionSession::from_document → TypesetEngine::typeset_hosted_section
→ TypesetState의 available_height / advance_column_or_new_page / flush_column
→ PageItem::HostedTable → LayoutEngine::build_render_tree / build_single_column`

- `typeset/hosted.rs`가 기존 구역 주소·단 영역과 가용 높이로 호스트 frame을 만든다.
  동일 제안의 commit 결과를 전용 PageItem으로 보존하고, 점유 하단에서 현재 흐름을 전진시킨다.
  fit 실패는 내용을 소비하지 않고 기존 단/쪽 전환을 사용한다. 비어 있는 동일 폭 단에서도
  수용하지 못하면 `NoProgress`로 종료하며 빈 페이지를 계속 만들지 않는다.
- `host.rs`는 commit 전에 같은 조각의 paint 노드를 구성·검증한다. 출력 실패 뒤 Legacy로
  재실행하거나 높이를 작게 예약한 뒤 paint에서 확장하지 않는다. 불변 노드 캐시와 원래 조각을
  함께 보관하고 출력마다 해당 페이지 allocator로 ID를 부여한다.
- `layout.rs::build_single_column`은 V2 항목을 Legacy vpos/PartialTable/뒤 간격 보정보다 먼저
  처리한다. 동일 절대 좌표의 노드를 단에 넣고 동일 점유 하단을 뒤 본문 시작값으로 쓴다.
  외부 표의 원본 section/paragraph/control 주소도 여기서 연결한다. 내부 셀 편집 hit-test까지
  연결했다는 의미는 아니다.
- PageItem의 소유 문단 조회, 페이지 번호 첫 출현, 진단 출력에 새 항목을 추가했다.
  Legacy 측정/행 컷 분기로 전달하지 않는다. 편집 후에는 새 세션을 만들며, Legacy의
  `with_offset`으로 확정 패킷을 비영(非零) 재색인하는 것은 명시적으로 거부한다.

**구현 중 발견하고 수정한 빈 소유 문단 누락:** 초기 연결에서는 표가 든 문단을 표 패킷으로만
대체했다. 기존 V2 본문 규칙(`document_input.rs`: fresh TopAndBottom 뒤 host line)과 대조하여,
표를 배치한 뒤에도 소유 문단의 실제 줄이 남아야 함을 확인했다. 원본은 보존하고, 일반 본문
소비용 투영에서 이미 V2가 소유한 Table control만 제외한다. 문단 슬롯·글자 모양·빈 줄은
유지하여 기존 문단 format/place/render에 함께 전달한다. 원본 LineSeg를 제거해 수용하는
처리가 아니며, 저장 줄이 있는 외부 호스트는 아직 처음부터 거부한다.

`tests/cases/issue_7353_hosted_section.rs`는 helper 반환값 대신 **실제 기존 LayoutEngine이
만든 최종 tree**를 검사한다. 기대값은 합성 입력의 9pt=12px 줄 점유, 고정 2700HU=18px
전진, 위/아래 225/300HU=3/4px 패딩, 40px 본문/212px×2단/16px 간격에서 정했다.
prefix 뒤 첫 표 조각 `(20,48,212,21)`, 다음 단 조각 `(248,30,212,40)` 및 이어지는 쪽의
소유 빈 줄 y30/뒤 문단 y48을 검사한다. 넓은 본문에서는 표 뒤 소유 빈 줄 y109,
추가 빈 문단 y127, suffix y145를 각각 보존한다. RTL의 단 순서, 재출력의 동일 tree,
노드 ID 중복 없음, 새 원본 스냅샷, 미지원 거부와 진행 불능 종료도 검사한다.

소유 빈 줄 검사를 보강한 뒤 초기 연결은 **4 PASS/2 FAIL**이었다. 출력 suffix y127 대
기대 y145, 마지막 단 예약18 대 기대36으로 실제 누락을 검출했다
(`frame-after-child/host-owner-before-issue_7353_hosted_section.log`). 위 투영 수정 뒤
**6 PASS/0 FAIL**이다. 이는 이번에 작성한 연결 코드의 개발 중 결함이며, 이전 배포판이나
원본 #7008 결함의 수정 전 FAIL을 입증한 것으로 확대하지 않는다.

집중 회귀는 신규6 + 기존 host9/session9/export36/sandbox2/color5/Legacy7008 3 =
**70 PASS/0 FAIL**. 명령은 기존 `frame-after-child/run-policies.sh host-callsite-final ...`,
요약은 `output/7353/r19/section-scope/callsite-focused-summary.log`다.
Native lib build 및 Native/WASM lib Clippy, workspace compile과 최종 source 해시 결과는
아래 최종 검증 기록으로 확정한다. 별도 파생 integration manifest·golden·ignore를 바꾸지 않았다.

이 합성 계약을 한컴 시각 일치나 R5 완료로 판정하지 않는다. 새 세션은 아직 저장 문서
admission/브라우저 API가 없으므로 이번 연결 자체의 독립 PDF·fresh WASM runtime·Visual Sweep은
미검증이다. 예전 브라우저 캡처를 재사용하지 않았고 새 시각 승인을 요청하지 않는다.
다음 남은 범위는 저장 호스트/문서 컨트롤 연결, 원본 #7008의 셀 내 도형 수용, 해당 실물의
Native/fresh WASM 종단 검증과 편집 재조판 연결이다. 기본값 전환·PR·push는 수행하지 않았다.

최종 검증: HEAD `cc13f573a43ba96093198f9bcf93ad6ca1c8bd38` + WIP,
변경 source/test31개 해시는 `section-scope/callsite-source.sha256`로 고정했다.
`cargo build --locked -p rhwp --lib` PASS, Native lib 및 wasm32 lib Clippy `-D warnings`
각각 PASS, `cargo check --locked --workspace` PASS다. 공통 target은
`/home/edward/mygithub/rhwp/target/pr-review`이며 로그는
`section-scope/callsite-{build,clippy-native,clippy-wasm,workspace-check}.log`다.
같은 최종 source에서 집중70건 PASS, `cargo fmt --all -- --check` 및 신규 case의 명시
rustfmt check, `git diff --check` PASS. workspace check는 all-targets Clippy/전체 CI나
fresh WASM 패키지 실행의 대체가 아니며 그 범위는 여전히 남아 있다.

### 2026-09-28 — #7008 셀 내 사각 도형과 글상자 연결

계속 진행 승인에 따라 원본의 실제 거부 원인인 셀 내 Shape를 연결했다. 입력은
`samples/21_언어_기출_편집가능본.hwp`, 대응 기준은
`pdf/21_언어_기출_편집가능본-2022.pdf`다. 둘 다 수정하지 않았다.
원본 `s0:p0:c2`의 cell2는 글 앞으로 배치한 “제1교시” 글상자, cell4는 TAC “홀수형”
글상자다. 전자의 보이는 도형과 빈 호스트 줄의 흐름 점유를, 후자의 TAC 점유를 구별한다.
도형 삭제/그림 변환 또는 Legacy 표 fallback으로 수용하지 않았다.

생산/소비 경로는 `text_ir::compose_items → pictures::compose / shapes::compose_floating
→ ParagraphItem::ObjectRow → ir::bind_table → FlowBlock::Lines → TextFragment::build_node`다.
TAC의 원래 제어 위치와 저장 줄 소속은 기존 `tac::object_rows`를 재사용한다. 글 앞으로
배치된 객체는 호스트의 실제 빈 TextLine과 도형을 하나의 paint payload로 보관하지만,
예약 bounds는 호스트 줄 그대로다. 도형의 높이를 뒤 문단 전진량으로 사용하지 않는다.
그룹의 최종 원점 이동은 기존 `text::translate`가 호스트/도형/글상자에 함께 적용한다.

`shapes.rs`는 회전/그룹/전단 없는 사각형과 한 문단 글상자를 명시적으로 수용한다.
외곽은 기존 `LayoutEngine::layout_shape_object`의 painter를 재사용하되 내부 글상자는
Legacy에서 다시 줄 나누지 않는다. V2 `TextComposer`의 동일 글줄 노드로 높이와 배치를
계산한다. Top은 저장 글상자 첫 vpos, Center/Bottom은 내부 여백을 뺀 영역의 정렬 불변식을
적용한다. 한컴 저장 TAC의 3610HU 도형 + 283HU 아래 여백 = 3893HU 점유를 보존한다.
기존 96dpi pen의 가시성 clamp를 가져오지 않고 명시된 실선 폭을 HU→현재 DPI로 변환한다.
복합 도형/회전/그림자/점선/자동 크기/글상자 내부 넘침/복수 문단 등 미연결 범위는 오류다.
도형 내부 문단의 원본 borderFill 검증도 수행한다. 기본 Legacy는 바꾸지 않았다.

`tests/cases/issue_7353_cell_shapes.rs`의 원본 셀 계약은 제어·글자·줄 메트릭을 유지하고
셀 주소/외부 그리드만 단일 셀로 분리한다. 이를 원본 전체 페이지 출력 증거로 세지 않는다.
기대값은 저장 폭·높이·여백·줄간격과 중앙 정렬식에서 정했으며 다음 최종 tree를 검사한다.

- 96/192dpi에서 도형9743×3610HU, 글줄2500HU, 가운데 여유 `(3610-2500)/2`, 선72HU.
- 글 앞으로 배치된 도형의 높이를 늘려도 빈 호스트2700HU와 표 흐름 높이가 그대로 유지됨.
  이 검사에서는 선언 최소 높이를0으로 두어 최소 높이가 잘못된 증가를 가리지 않게 했다.
- 3892HU 예산에서는 TAC 줄을 수용하지 않고, 3893HU에서 여백 포함 예약/출력/종료가 일치함.
- 같은 저장 줄의 두 도형은 `9743+71`HU 간격, 다른 줄의 도형은 `3893+716`HU 전진을 보존함.
- 미지원 효과와 글상자 넘침은 명시적 실패이며 내용 숨김으로 성공하지 않음.

수정 전 라이브러리에서 원본 두 셀은 `Unsupported("non-table cell control")`로 실패했다
(`section-scope`가 아닌 `frame-after-child/shapes-before-issue_7353_cell_shapes.log`,
1 PASS/2 FAIL). 초기 테스트 소스의 필드명 컴파일 오류는 이 검출 증거와 구별한다.
새 수용 뒤 최종 5건 PASS. 경계 테스트의 `/75` 대 `*(96/7200)` 부동소수점 표현 차이로
정확 fit가 한 ULP 부족했던 중간 실패는 동일 HU 변환식을 사용해 바로잡았으며 엔진의 fit
허용치를 완화하지 않았다. 이는 새 기능의 거부→수용 증거이지 기존 배포판 회귀 수정 주장이 아니다.

집중 회귀 **78 PASS/0 FAIL**: shapes5/hosted6/host bridge9/session9/export36/sandbox2/
cell picture6/overwide picture5. `section-scope/shapes-focused-summary.log`와
`frame-after-child/shapes-final-*.log`에 결과를 보존했다. host bridge의 #7008 원본 거부 계약은
도형 다음 실제 거부인 `TAC paragraph vertical alignment`로 갱신했다. 단순 성공/ignore로
바꾸지 않았으며 원본 전체 미지원을 계속 검사한다.

Native 진단 `section-scope/shapes-paint.rs`가 원본의 두 셀을 별도의 진단 위치에 배치한
`shapes-native.svg/json/png`를 생성했다. PNG를 직접 열어 외곽과 내부 글자 보존을 확인했다.
이것은 분리 셀 진단이며 원본 페이지 위치 비교·한컴 시각 판정 이미지가 아니다. 전체 표의
TAC 문단 수직 정렬과 저장 문서 호스트가 아직 미수용이므로 fresh WASM/원본 Visual Sweep은
이번에 실행하지 않았다. 과거 캡처 재사용이나 새 시각 승인 요청도 하지 않았다.

최종 소스/테스트 해시는 `output/7353/r19/section-scope/shapes-source.sha256`에 고정한다.
Native lib build, Native/wasm32 lib Clippy `-D warnings`, fmt 및 diff 검사 결과는 같은 폴더의
`shapes-{build,clippy-native,clippy-wasm}.log`와 함께 보존한다. 이는 전체 CI 상당 게이트가
아니며 push/PR/기본값 전환은 수행하지 않는다. 다음 작업은 원본 두 TAC 자식의 문단 수직
정렬을 독립 출력과 대조하고, 저장 호스트/다단 예약 연결 및 원본 종단 검증을 이어가는 것이다.

### 2026-09-28 — 저장 TAC CENTER와 원본 첫 표 배치

다음 단계 승인으로 원본 #7008 cell6/p0의 `ParaShape62 / CENTER / Right`를 연결했다.
저장 줄3015HU/중심1507HU, 두 자식의 높이2449/2448HU, 첫 자식 위·아래 여백283HU를
보존한다. 기존 Baseline 전용 guard를 무조건 제거하지 않고 CENTER의 독립 근거를 먼저
확인했다. `tests/fixtures/issue7353_center_tac_review/README.md`에 생성 코드·MCP job·
저장본/PDF 해시·관측 좌표를 모았다. 기존 정상 unequal TAC에서 문단 세로정렬만 변경한
대칭 대조군과 위/아래 여백을 다르게 한 비대칭 대조군을 각각 한컴2020으로 정상 저장,
그 저장본을 PDF로 출력했다. 원본 샘플/PDF는 수정하지 않았다.

비대칭 대조군에서 한컴은 **바깥여백 포함 상자**를 가운데 정렬한다. 작은 표의 carrier
내 y는 `(8700-(5000+400+100))/2+400=2000HU`다. 본체 중심만 맞추는1600HU가 아니다.
`tac_metrics::TableBand::measure_centered/centered_top → tac::object_rows`가 한 번 만든
Rect를 `tac::compose → ParagraphItem::InlineTables → ir::bind_table → FlowBlock::InlineTables
→ FlowCursor::fit → TextPaint::build_node`가 소비한다. 실제 child bottom도 같은 Rect에서
계산해 fit 높이에 반영한다. 후단 paint의 별도 정렬/좌표 clamp는 없다.
정상 저장 CENTER reference는 정수HU `floor(line_height/2)`로 확인하며, Baseline 캐시의
스타일만 바꾼 반례는 거부한다. 기존 Baseline, 사진/도형의 Baseline 전용 수용 조건,
fresh CENTER·TOP/BOTTOM 미지원은 유지한다. 문서 본문도 같은 compose를 호출하지만
이번 독립 PDF의 검증 범위는 셀 안 TAC이며 본문 CENTER 독립 시각 검증으로 세지 않는다.

원본 첫 표를 실제 fit하면서 추가로 발견한 오류는 TAC 정렬이 아니라 **아래 정렬 셀의
예산 재계산 상쇄 오차**였다. GDB의 `fit_row_groups`에서 원본 cell4의 물리 높이
122.13333333333333px, 측정 내용51.906666666666673px, offset70.22666666666666px를
확인했다. `height-offset`이 측정 내용보다 한 ULP 작아져 `InconsistentAtomicPlan`이었다.
`content::VerticalAlignment::content_window`가 측정된 내용 높이와 정렬 원점을 함께
생산하도록 바꿨다. Bottom의 예산은 원래 content, Center는 content+절반 slack이다.

소비 지점은 `content.rs`의 원점 생성, `fragment/row_groups.rs`의 혼합 span 재정렬/원자
그룹 fit, `fragment.rs`의 일반 intact fit/분할 뒤 intact 동반 셀 재정렬이다. 일반 intact
경로는 이미 수용한 행 높이의 예산에 남은 페이지 공간만 더한다. 내부 cut을 가진 셀은
그대로 기존 비정렬 예산을 사용한다. 새 epsilon, 높이 증대, clip 또는 내용 숨김은 없다.
원자 그룹의 성공은 모든 블록 소비 뒤 종료로 확인하고, 부족한 예산은 컷/소비 없이 거부한다.
기존 rowspan/분할 정렬 계약으로 다른 경로의 컷·이월·최종 배치를 함께 재검사했다.

검출/검증:

- CENTER 대조군 두 건은 수정 전 라이브러리에서 수직 정렬 미지원으로 실패했다.
  `frame-after-child/center-before-issue_7353_center_tac.log`에는 추가 거부 사유 검사까지
  3 FAIL이며, 그 세 번째를 새로운 배치 결함 검출로 세지 않는다. 최종3 PASS.
- 원본 host 계약은 이전의 예상 미지원 검사를 실제 fit/paint/종료 검사로 바꿨다.
  `center-budget-old-arithmetic-issue_7353_table_host_bridge.log`는 CENTER 연결 후 예전
  `height-offset` 계산식을 잠시 재현한8 PASS/1 FAIL(InconsistentAtomicPlan)이며,
  최종9 PASS. 실제 변경 전 실행은 `section-scope/center-original-gdb3.log`에도 남겼다.
  기존 코드 전체 baseline을 재현했다고 주장하지 않는다.
- 원본 표는 `OmitFinalParagraphGap`을 명시해 높이13740HU,1HU 부족 거부,
  원본 “제 1 교시”/“홀수형” 보존, 동일 예약/paint와 완료를 검사한다.
  기본 종료 정책의 수용으로 확대해 주장하지 않는다. 단일 셀 Top/Center/Bottom
  exact-fit 대조군은 기존 계산에서도 통과했으므로 결함 검출이 아닌 무회귀 계약이다.
- 최종 집중 **255 PASS/0 FAIL**. `section-scope/center-focused-summary.log`에
  CENTER3/shapes6/host9/vertical9/rowspan13/roundoff2/document160/hosted6/export36/
  session9/sandbox2를 연결했다. Native build, Native/wasm32 lib Clippy `-D warnings`,
  fmt·신규 test/fixture rustfmt·diff check PASS. 전체 CI/전체 corpus 검증은 아니다.

Native 시각 자료는 같은 한컴 저장본의 `center-review/native-review-1.png` 및
`center-asymmetric-review/native-review-1.png`다. 두 표의 시작·외곽·CELL AFTER·뒤 본문을
직접 확인했다. 원본은 `original-header-review/native-review.png`에서 Paper 기준 원래
앵커에 **첫 표 전체만** 그려 동일 PDF1쪽의 x100/y120/w930/h202 영역을 비교했다.
정렬 대조군의 기하와 원본 성명·수험번호/하단선은 대조했지만, 왼쪽 글상자의 시작 높이와
round_rate50의 곡률에 차이가 남는다. 전체 표의 수용 성공을 원본 시각 통과로 승격하지
않는다. 글꼴 외형 차이와 이 기하 차이도 구분한다. 다음 원인 추적은 Para/Top 앵커와
저장 첫 줄 vpos568HU의 관계, 외곽 painter의 round_rate 해석이다.

증적 루트는 `output/7353/r19/section-scope/`이며 final source는 `center-source.sha256`다.
Docker fresh WASM 빌드/동일 입력의 실제 backend 비교 결과는 아래에 이어 기록한다.
기본 Legacy, Studio/DocumentCore 선택, 원격 게시·push·PR은 변경하지 않았다.

최종 추가 검증: fragment minimum band12/grid terminal4/alignment8/headers14/split policy3,
split-line property8도 통과해 이번 실행 합계는 **304 PASS/0 FAIL**이다. 마지막8건은
직접 rustc runner에 `--extern zip`가 없어 처음 빌드 실패한 뒤 실제 의존성을 연결해
재실행했다. 이를 제품 회귀로 세지 않는다. 결과는 `center-boundaries-summary.log`와
`center-split-property.log`이며, 재실행 전 BUILD FAILED 기록도 보존했다.

Docker 명령은 `docker compose --env-file .env.docker -f docker-compose.yml
-f output/7353/r19/cell-floating-picture/docker-network.yml run --rm wasm`이다.
`center-wasm-build.log`: **7분23초, 성공**. WASM SHA256은
`f795329641289ee7eec026f6f71770ba3854b74b0eedd0fb6a44c9e71078d9fa`다.
source HEAD는 `cc13f573a43ba96093198f9bcf93ad6ca1c8bd38`에 승인된 WIP를 적용한 상태로,
정확한 파일 집합은 `center-source.sha256`이며 빌드 후 `sha256sum -c`를 통과했다.
Native 산출도 최종 라이브러리로 다시 생성한 후 캡처했다.

실행은 `node output/7353/r19/section-scope/capture-center.mjs --wasm`,
같은 명령의 `--asymmetric --wasm`, `capture-original-header.mjs --wasm`이다.
각 실제 WASM 세션에서 SVG/RenderTree를 얻었고 Native tree로 대체하지 않았다.
대칭/비대칭 대조군은 각1쪽, 원본 선택 표는1조각이며 세 경우 SVG 동일, 구조 차이0,
좌표 차이 최대5.684341886080802e-14px다. `*/backend-comparison.json`과
`run.json`/`manifest.json`에 입력/PDF/WASM/source hash를 연결했다.

직접 판독한 대표 자료:

- `center-review/wasm-review-1.png` / `wasm-overlay-1.png`: 대칭 여백 CENTER.
- `center-asymmetric-review/wasm-review-1.png` / `wasm-overlay-1.png`: 비대칭 여백 CENTER,
  작은 표 시작과 두 표 외곽, 부모 외곽·뒤 문단 보존. 글꼴 외형 차이는 남는다.
- `original-header-review/wasm-review.png` / `wasm-overlay.png`: 원본 선택 표의 실제
  Paper 앵커 비교. 첫 표의 수용은 확인했지만 앞서 적은 왼쪽 도형 차이 때문에 원본
  시각 통과는 **보류**한다. 원본 전체 다단/저장 호스트 연결도 여전히 미완료다.

메인테이너의 이번 새 자료 판정은 아직 받지 않았다. 이 결과로 R5 전체 완료 또는
DocumentCore/Studio 기본 조판 변경을 선언하지 않는다.

### 2026-09-28 — CENTER 승인 및 원본 글상자 문단 앵커/곡률

메인테이너가 위 CENTER 대칭/비대칭 시각 자료를 통과 판정하고 다음 절차를 승인했다.
원본 첫 표에 남았던 글상자 기하를 이번 절편에서 처리한다. 원본 전체 본문 조판의
통과 판정으로 확대하지 않는다.

독립 근거는 원본 `samples/21_언어_기출_편집가능본.hwp`와 기존 한컴 PDF
`pdf/21_언어_기출_편집가능본-2022.pdf`다. 수동 재저장·좌표 보정 없이 같은 원본을
사용했다. PDF1의 글상자 clip 상단은 bottom-up1190pt 좌표의1043.664pt,
96dpi에서는195.114667px다. 원본 Para56의 문단 앞 간격은568HU, 첫 줄 vpos도568HU,
실제 빈 줄 높이는2700HU다. 앞선 보고의 vpos 의심을 구체화하면 **문단 앞 간격이 들어간
첫 글줄 원점을 Para/Top 도형 앵커로 사용한 것**이 원인이다. 저장 vpos 자체를 삭제하거나
빈 문단 높이를 축소하는 수정이 아니다.

변경 경로는 `shapes::compose_floating`의 문단 로컬 도형 원점(세로 offset) →
`ParagraphItem::ObjectRow`의 원래 빈 줄 점유 → `ir::bind_table`/FlowCursor의 줄 fit →
`TextPaint::build_node`의 동일 payload 평행 이동이다. group/host bbox는 유지하고
도형만 문단 원점에 둔다. 최종 y는202.693333→195.120000px이며 독립 PDF와0.006px 미만
차이다. 뒤에서 앵커를 다시 선택하거나 clamp하지 않는다. 이 경로의 전경 도형은 빈 줄의
흐름을 늘리지 않으며 TAC 도형은 기존 글줄 기준 배치를 그대로 사용한다.

곡률은 `한글문서파일형식_5.0_revision1.3.md` 표94의 `50%=반원`을 적용했다.
공통 Legacy painter의 `짧은 변×비율/2`를 그대로 가져오지 않고 V2 adapter에서
`짧은 변×비율`을 RectangleNode에 준다. SVG/WebCanvas의 최종 반지름 한계는 각 renderer의
기존 기하 처리를 그대로 사용한다. 원본의50은10.39→20.78px, TAC 도형의12는
2.888→5.776px다. Legacy 구현·기본 엔진을 변경하지 않았다.

회귀는 `tests/cases/issue_7353_cell_shapes.rs`에 추가했다. 독립 PDF 원점 검사와
빈 줄/문단 원점 분리, 사양 기반0/12/20/50 곡률을96/192dpi에서 검사한다. 세로720HU
offset과 셀 분리는 합성 계약이며 별도 한컴 생성본의 피델리티 증거라고 주장하지 않는다.
수정 전6 PASS/3 FAIL 로그는 `frame-after-child/shape-anchor-before-issue_7353_cell_shapes.log`다.
최초 수정 후 검사의 마지막 문단 뒤 간격 기대값에 오류가 있어8 PASS/1 FAIL이었다.
명시한 OmitFinalParagraphGap 정책에 따라284HU 뒤 간격만 제외하도록 테스트를 바로잡았고,
568HU 앞 간격과2700HU 빈 줄은 유지한다. 이 기대값 오류를 제품 결함으로 세지 않는다.

증적은 `output/7353/r19/section-scope/shape-anchor-*`에 보존한다. 기존 CENTER 및
원본 before 이미지/JSON을 덮어쓰지 않았다. `check-shape-anchor-delta.mjs`는 before/after
RenderTree 전체에서 전경 도형 하위 y5개와 두 곡률만 변경됨을 확인한다. 빈 host 줄,
셀/표 외곽, TAC 원점, 하단선 및 나머지 내용은 그대로다. Native review/standalone overlay를
직접 확인했고 글꼴 외형 차이는 조판 기하와 구분해 남긴다. fresh WASM 결과는 아래에 기록한다.

최종 Native 집중 검증은 **240 PASS/0 FAIL**이다. 도형9/CENTER3/host9/셀 그림6/
hosted6/sandbox2/document160/export36/session9이며 `shape-anchor-final-summary.log`와
`shape-anchor-paths-summary.log`에 연결했다. Native lib build, Native/wasm32 lib Clippy
`-D warnings`, cargo fmt check, 테스트 rustfmt, diff check도 통과했다. 전체 CI 또는
workspace all-targets 검증으로 세지 않는다. 소스 기준은 HEAD
`cc13f573a43ba96093198f9bcf93ad6ca1c8bd38` + 승인된 WIP이며 정확한 Rust source/Cargo/
이번 회귀 파일의 SHA256은 `shape-anchor-source.sha256`에 고정했다.

Docker fresh WASM은 기존 compose/bridge 명령으로 **7분26초, 성공**했다.
`shape-anchor-wasm-build.log`, WASM SHA256
`381e2dc22556cf63769b83883313631d21789f077a2eea921417c1421a8ed44b`.
완료 exit code0을 확인한 뒤 `node output/7353/r19/section-scope/capture-original-header.mjs
--shape-anchor --wasm`으로 실제 Chrome의 `TableV2Preview`를 실행했다.
소스 hash 재검사 PASS, SVG 동일, RenderTree 구조 차이0/수치 최대차이
5.684341886080802e-14px다. source/input/PDF/WASM hash 및 동일 crop은
`shape-anchor-review/manifest.json`, backend 대조는 `backend-comparison.json`에 있다.

대표 판정 자료는 `shape-anchor-review/wasm-review.png`이며 위부터 한컴/수정 전/수정 후/
수정 후 overlay다. `wasm-overlay.png`도 별도로 직접 확인했다. Native 자료도 같은 폴더의
`native-review.png`/`native-overlay.png`다. 원본 좌표 x100/y120/w930/h202에서 별도
확대·이동 fitting 없이 비교했다. ‘제1교시’ 위치와 외곽 곡률이 맞고, 하단선·성명/수험번호
위치 보존을 확인했다. 기존 제목·홀수형 등 폰트 외형 차이는 남으며 이번 해결로 세지 않는다.
CLI 기본 Legacy가 아닌 명시적 V2 선택 표 세션의 실제 출력을 검사한 것으로,
Studio Canvas/문서 전체 종단 검증으로 확대하지 않는다. 이번 새 자료의 메인테이너 최종
시각 판정은 요청 상태다. 이후 진행 대상은 저장 문서 호스트/원본 전체 종단 연결이며,
새 엔진 기본 전환·PR·push는 수행하지 않았다.

### 글상자 승인 및 저장 본문의 기존 호스트 연결

작업지시자가 앞선 “제1교시” 글상자 수정의 시각 판정 통과와 다음 진행을 승인했다.
그 결과를 확정하고, 기존 `TypesetState`의 페이지/단 흐름에 저장 본문을 연결했다.
독립 DocumentV2에 새 문서 엔진을 복제하지 않는다. 이번 완료 범위는 명시적
`HostedSectionSession`/`HostedSectionV2`이며 Studio 기본값이나 편집 경로는 아니다.

원본 `21_언어_기출_편집가능본.hwp`를 먼저 조사했다. 구역1개/문단325개/표14개 중
선택 표 준비11개가 성공한다. p167은 저장 TAC 혼합 텍스트의 space advance,
p206/p232는 미지원 셀 컨트롤 때문에 준비하지 못한다. 원본 구역의 바탕쪽·빈 줄 정책,
본문 도형·다단 및 외곽 표 앵커/TAC 연결도 남아 있다. 첫 표 성공을 원본 전체 수용으로
세지 않으며, 이 속성을 제거한 입력으로 원본 통과를 만들지 않는다. 조사 명령의 소스와
결과는 `output/7353/r19/section-scope/host-inventory.rs`/`host-inventory.log`다.

이번 공유 결과의 경로는 다음과 같다.

- `host_text::HostedParagraphPlan::prepare`: 기존 TextComposer로 저장 줄/새 줄과 paint
  payload를 한 번 구성한다. 실제 빈 줄과 물리 Space, 다음 줄까지의 간격을 구분한다.
- `body_text::frame_starts`/`stored_text::continuous_paragraph`: 정상 저장본에서 인정한
  단일 단의 vpos reset만 경계로 사용한다. 다단 reset의 소유가 미확정이면 명시적 오류다.
- `HostedParagraphPlan::fit` → FlowCursor의 실제 수용 컷·높이·advance →
  `TypesetEngine::typeset_hosted_section`: 기존 호스트가 다음 단/쪽을 선택한다.
  fit 실패는 cursor를 소비하지 않고, 빈 동일 크기 단에서도 진행 불가면 종료 오류다.
- `PageItem::HostedParagraph`의 불변 조각 → `LayoutEngine` 초기 소비 분기:
  예약한 줄의 최종 좌표/payload를 그대로 그린다. Legacy의 재조판·vpos 덮어쓰기·clamp를
  거치지 않는다. 기존 HostedTable query/commit과 표 점유 예약은 유지한다.

초기 SectionDef/ColumnDef는 검증 후 본문 투영에서만 제외한다. 표시되지 않는 구역
border-fill 참조는 기존 V2의 `source_border_is_unpainted` 판정을 재사용하며, 실제
테두리/배경·바탕쪽·격자 등은 계속 거부한다. 소스 Document는 변경하지 않는다.
공통 PageItem의 문단 소유/쪽번호/진단 소비자도 새 조각을 분류하도록 연결했다.

독립 기준은 기존 정상 한컴 저장 fixture
`tests/fixtures/issue7353_body_frame_review/portrait-saved.hwp`와 대응
`portrait-2020.pdf`다. 생성 절차는 fixture README에 있으며 저장 LineSeg를 손으로
추가하지 않았다. 1200HU 줄 높이/600HU 줄간격, 첫 쪽 BRAVO 뒤 빈 줄,
둘째 쪽 DELTA 시작과 AFTER를 96/192dpi에서 최종 RenderTree로 검사한다.
이전 호스트에서는 이 입력을 지원하지 않아 수정 전 해당 정상 저장본 검사가 FAIL이었다.
잘못된 nonzero reset 거부 검사의 이전 실패는 오류 종류 차이이며 별도 제품 결함으로
세지 않는다. 기존 fresh 합성 fixture의 빈 carrier에 없던 char-shape를 명시했고,
좌표 기대값을 변경하지 않았다. 조각 종류 검사만 새 HostedParagraph로 갱신했다.

정식 검사는 `tests/cases/issue_7353_hosted_section.rs`다. Native 최종 집중 검증은
**231 PASS/0 FAIL**(hosted8/host bridge9/shapes9/document160/export36/session9).
이후 문단 소유·JSON export 검사를 추가한 hosted8건도 재검증 통과했다
(`host-text-final-export-summary.log`). JSON 비교의 첫 실행에서는 원시 f64와 JSON
역직렬화 값을 직접 동등 비교하여 실패했고, 양쪽 모두 같은 직렬화 경계를 거치게
검사 방법을 수정했다. 독립 좌표 기대값과 허용 오차는 변경하지 않았다. 표 분할 후 빈 owner 줄,
별도 빈 문단과 suffix 위치, RTL 단 순서, snapshot 불변성, 진행 불가 종료를 함께 보호한다.
새 stored 검사는 본문 호스트 계약이며 #7008 표/다단 전체 피델리티 증거가 아니다.

증적 prefix는 `output/7353/r19/section-scope/host-text-*`다. 수정 전 검사 로그는
`frame-after-child/host-text-before-issue_7353_hosted_section.log`, 완료 build는
`host-text-final-build.log`, 최종 집중 결과는 `host-text-final-summary.log`다.
Native/wasm32 lib Clippy `-D warnings`와 fmt check가 통과했다. 전체 CI/workspace
all-targets 또는 기본 엔진 전체 회귀를 실행했다는 의미는 아니다. HEAD
`cc13f573a43ba96093198f9bcf93ad6ca1c8bd38` + 승인 WIP의 정확한 Rust/Cargo/test 내용은
`host-text-source.sha256`에 고정한다.

Native2쪽 review를 직접 판독했다. 첫 쪽 빈 줄, 다음 쪽 시작, 후속 문단이 보존되고
글꼴 외형 차이는 남는다. 페이지수/텍스트 존재만으로 판정하지 않았다.
`capture-host-text.mjs`는 PDF를192dpi로 raster한 같은 영역과 SVG를 비교하며 위치
fitting을 하지 않는다. Docker fresh WASM 결과와 최종 증적은 아래에 이어 기록한다.

Docker fresh WASM 빌드는 **7분29초 성공**, 실행 종료0을 확인했다.
명령은 `docker compose --env-file .env.docker -f docker-compose.yml -f
output/7353/r19/cell-floating-picture/docker-network.yml run --rm wasm`이며 로그는
`host-text-wasm-build.log`, WASM SHA256은
`5019b23c206941ed7149495cae1a68040f45e89a2834ef128ac8d36a73e0b1ad`다.
`node output/7353/r19/section-scope/capture-host-text.mjs --wasm`으로 Chrome에서
새 `HostedSectionV2`를 실제 실행했다. Native와 SVG가 같고, RenderTree 구조 차이0,
수치9개 차이/최대1.4210854715202004e-14px다. 미완성 JSON/알 수 없는 옵션/없는 구역/
없는 페이지4가지도 오류로 거부됨을 확인했다. source hash 재검사와 diff check도 통과했다.

대표 자료는 `host-text-review/wasm-review-1.png`, `wasm-review-2.png`이고,
standalone overlay는 `wasm-overlay-1.png`, `wasm-overlay-2.png`다. 두 쪽 review와
둘째 쪽 standalone overlay를 직접 판독했다. 빈 줄·글줄 시작·뒤 문단 위치는 기준과
맞고 폰트 글립 외형 차이는 남는다. `native-manifest.json`/`wasm-manifest.json`에
source/input/PDF/WASM hash와 실행 범위를 연결했다. 이 새 시각 자료의 메인테이너 판정은
요청 상태다. 본문 연결 구현·집중 검증은 완료했으나, 원본 #7008 전체 연결과 Studio
편집 경로·전체 CI는 미완료/미검증으로 남긴다. 다음 작업은 이 조사에서 확인한 구역·다단·
표 앵커의 기존 호스트 연결이며, 미지원 원본 속성을 지우는 우회는 하지 않는다.

### 원본 p167: 본문과 TAC 글상자가 섞인 셀의 수용

이전 BRAVO/DELTA 재검증은 신규 진척으로 다시 세지 않는다. 이번에는 실제 #7008
원본의 거부 경로를 해소했다. `samples/21_언어_기출_편집가능본.hwp`의 s0p167c0,
6번째 셀 첫 문단은 본문과 TAC 사각 글상자 “푸코”를 함께 가진다. 기존 공백 전용
어댑터는 `stored TAC mixed text requires qualified space advances`로 거부했다.
대응 기준은 기존 `pdf/21_언어_기출_편집가능본-2022.pdf`의 **PDF8쪽**이다.

- 공통 결과: `text_ir.rs::compose_items` → `text.rs::compose_stored_inline_shapes`
  → 기존 `compose_shared`의 저장 줄 구성/텍스트 paint → `shapes.rs::attach_inline`.
  가로 원점은 공통 문단 painter가 기록한 shape 위치, 세로 원점은 저장 기준선과
  TAC ascent의 정수 HU 값이다. 공통 painter의 기존 shape y 보정은 재사용하지 않는다.
  텍스트와 객체 모두 같은 줄 상자 안에 있는지 검사한 뒤, 해당 줄을 `ObjectRow`로
  전달하여 IR 바인딩의 객체 소비와 실제 paint를 연결한다. 높이/줄간격을 다시 추정하거나
  모자란 공간을 clamp하지 않는다. 그림·floating shape·기존 순수 TAC 경로는 유지한다.
- 범위: 유효 저장 줄의 zero-offset TAC rectangle/textbox와 본문 혼합이다. 줄 묶음,
  음수 pitch의 겹치는 객체 줄, 편집 후 재조판은 이번에 지원한다고 주장하지 않는다.
  원본 전체 표의192dpi 경로는 기존 text-run 점유 검사에서 거부되어 미지원으로 남긴다.
- 원본 회귀: 수정 전9통과/원본1실패(위 Unsupported), 수정 후 shapes11통과.
  1417HU 줄/객체 높이,2267HU 폭,142HU 오른쪽 여백, 저장10줄의 vpos와 본문 전체
  문자 보존을 실제 RenderTree에서 검사한다. 독립 PDF vector의 상대 원점도0.2px 안에서
  대조했다. 객체 높이 또는 바깥여백만 늘린 반례는 저장 줄을 넘으면 명시적으로 거부한다.
- focused231통과: shapes11 + floating_picture6 + host_bridge9 + sandbox_document2
  + table_v2_text43 + document_flow160. Native/wasm32 lib Clippy `-D warnings` 통과.
  전체 CI·workspace all-targets·Studio 기본 엔진 회귀 완료라는 의미가 아니다.

증적은 `output/7353/r19/section-scope/mixed-shape-*`, 개별 검사 로그는
`frame-after-child/mixed-shape-{before,final}-*.log`다. 빌드는 공유 target/pr-review의
`cargo build --locked -p rhwp --lib`, 검사는 기존 `run-policies.sh`로 실행했다.
HEAD `cc13f573a43ba96093198f9bcf93ad6ca1c8bd38` + 승인 WIP이며 정확한 소스와
입력/test hash는 `mixed-shape-source.sha256`, `mixed-shape-inputs.sha256`에 고정한다.

`mixed-shape-review/native-review.png`를 직접 판독했다. 선택 표 원점은 PDF 테두리와
원본 첫 행 높이에서 지정했다(문서 전체 앵커 검증 아님). 첫 줄의 “푸코” 글상자와 뒤 본문은
연결되지만, 아래 본문의 저장 줄바꿈은 PDF와 다르다. 이 차이를 숨기거나 전체 피델리티 통과로
올려 보고하지 않는다. 원본 표 준비 결과는11/14→12/14이며, p206/p232의 AutoNumber(Page)
두 건과 원본 전체 호스트의 section decoration/grid 수용은 남아 있다. 새 API나 이전 승인
예제의 반복이 아니라 원본 혼합 셀1건을 추가로 수용한 결과다.

Docker fresh WASM 빌드는7분29초/종료0이다(`mixed-shape-wasm-build.log`). 동일 compose
명령으로 생성한 `pkg/rhwp_bg.wasm` SHA256은
`4cf8f51e3c1b7c8efd3d1d76a33c3c62baebd49f5dace6526854e21f9f52dc49`다.
`node output/7353/r19/section-scope/capture-mixed-shape.mjs --wasm`으로 실제 Chrome의
`TableV2Preview`를 실행했고, Native와 SVG/RenderTree 구조·수치 차이가 없었다
(수치 비교 허용오차1e-9px). `mixed-shape-review/wasm-review.png`와 standalone
`wasm-overlay.png`를 직접 판독했다. 첫 줄 글상자/이어지는 본문 위치는 근접하며,
아래 저장 줄바꿈 차이는 Native와 동일하게 남는다. `manifest.json`에 source/input/PDF/
options/WASM hash와 선택 표 원점 지정 근거를 연결했다. 이 결과는 혼합 셀 지원 검증이며
문서 전체 피델리티·192dpi·Studio 기본 경로의 완료 또는 메인테이너 시각 승인이 아니다.

2026-09-29 메인테이너가 위 혼합 셀의 시각 판정 통과와 다음 진행을 승인했다.
다음 대상은 원본 p206/p232의 AutoNumber(Page)다. 저장 assigned_number는1/2이나
동일 원본 PDF10/11쪽에는10/11로 표시된다. 저장 숫자를 정답으로 사용하지 않는다.
저장 단일 줄의 공간 예약은 유지하고 최종 페이지 컨텍스트를 공통 문단 구성/paint에 전달한다.
측정한 줄 상자를 바꾸는 결과나 번호가 영역을 넘는 경우는 commit 전에 명시적으로 거부한다.

### 원본 자동 쪽번호 셀 2건 — 최종 페이지 컨텍스트 전달 (2026-09-29)

위 혼합 셀 승인을 반영했다. 이번 변경은 원본 s0p206c0/p232c0의
AutoNumber(Page)이며, 앞서 승인된 BRAVO/DELTA·“푸코”의 시각 검증을 반복한 것이 아니다.
원본 HWP의 저장 assigned_number=1/2와 독립 한컴 PDF10/11쪽의 표시10/11을 구별한다.
셀의 다른 문단에 있는15는 자동 총쪽수가 아니라 원문 고정 텍스트로 보존한다.

- 생산: `text_ir.rs:75`가 원본의 저장 줄을 `TextComposer::compose_page_field`로
  구성하고, 실제 사용한 폭·스타일·문단 snapshot을 `PageField`에 보존한다. 줄 폭을
  paint 좌표에서 역산하지 않는다. 단일 저장 줄/단일 decimal Page placeholder만 수용하며
  mixed text·서식 번호·편집 후 재조판은 이번 절편에서 미지원이다.
- 소비: 기존 `ParagraphItem::Lines`가 줄 높이/전진을 예약한다. 최종
  `TextPaint::build_node`가 동일 composer와 공통 `substitute_page_auto_numbers_in_composed`에
  호스트 번호를 전달한다. 원문 placeholder는 유지하고 display_text만 바꾼다.
  기존 줄 상자와 달라지거나 표시 폭이 넘으면 cursor/RenderTree를 commit하지 않는다.
- 호스트: `TableHostFrame::with_page_number`도 query/commit identity에 포함된다.
  `TypesetEngine::typeset_hosted_section`의 PageContent 번호는 finalize 전0이므로
  기존 PageNumberAssigner를 물리 페이지마다 한 번 실행한다(단마다 증가하지 않는다).
  이 제한된 호스트의 구역 시작 번호 변경은 명시 거부하고, NewNumber는 기존 admission에서
  거부한다. 독립 DocumentV2 경로는 SectionDef.page_num+실제 출력 페이지 순번을 전달한다.
  선택 표 preview는 `first_page_number`를 명시해야 하며 저장1/2를 대신 사용하지 않는다.

수정 전 실제 원본 prepare는 `Unsupported("non-table cell control")`로 FAIL이었다
(`frame-after-child/cell-page-before-issue_7353_cell_page_field.log`). 수정 후 원본2건,
10/11 표시·예약 기하 보존·컨텍스트 누락 후 재시도·좁은 줄 overflow 미커밋·호스트 번호
변경 시 stale proposal 거부·미지원 번호 서식 거부7건이 통과했다. 별도 호스트 계약은
여러 단/쪽에12개 필드를 배치하여 최종 PageContent 번호와 전부 대조하고, serializer로
생성한 독립 문서에서 시작 번호7을 검증한다. 이는 합성 계약이지 한컴 기준 출력은 아니다.

영향 검사 총282건 통과: cell_page_field7, host_bridge9, hosted_section10,
page_number_timeline5, table_v2_page_number15, text43, ir_text9, nested_text11,
document_flow160, sandbox2, shapes11. `cell-page-final-tests.log`의 hosted_section8건은
추가 계약 전 결과이며 최종10건은 `frame-after-child/cell-page-host-final-issue_7353_hosted_section.log`다.
Native/wasm32 lib Clippy `-D warnings`, fmt check 통과. 전체 workspace/CI 완료 주장은 아니다.

원본 표 준비는12/14→14/14가 되었으나 전체 원본 호스트는 여전히
`section decoration/grid requires host admission`으로 거부한다(`cell-page-inventory.log`).
기본 Legacy/Studio 엔진 전환은 하지 않았다. 다음 우선순위는 이 원본의 구역 속성·호스트
종단 연결이며, 표 준비14/14를 문서 전체 피델리티 또는 R5 완료로 계산하지 않는다.

Native `cell-page-review/{10,11}-native-review.png`와 독립 PDF10/11쪽을 직접 판독했다.
현재 번호10/11, 고정15, 대각선/외곽 배치는 근접하며 글꼴 굵기·글립 차이는 남는다.
선택 표 원점은 PDF의 외곽 x395.208/y1090.084pt에서 지정했다. 따라서 이 그림은
셀 내부 번호/배치 검증이며 원래 floating 표 앵커·문서 전체 페이지 배치의 증거는 아니다.
기준 PDF는192dpi로 rasterize하고 비교 그림은96dpi 좌표를3배 확대한다.

Docker fresh WASM도 종료0/7분27초로 완료했다(`cell-page-wasm-build.log`).
`pkg/rhwp_bg.wasm` SHA256은
`823e3df8a98a5cc9030e6a2c82f9e7f35dab649ff0765f1f468aa4b50c202e80`다.
`node output/7353/r19/section-scope/capture-cell-page.mjs --wasm`으로 실제 Chrome의
새 `TableV2Preview`를 실행했다. 두 표 모두 Native 대비 SVG/RenderTree 차이0
(수치 허용오차1e-9px)이며, WASM review/standalone overlay를 직접 판독했다.
10/11·고정15·대각선·외곽은 유지되고 Native와 같은 글꼴 차이가 남는다.
이는 구현자 확인이며 메인테이너의 이번 절편 시각 승인으로 대신하지 않는다.
정확한 HEAD는 `cc13f573a43ba96093198f9bcf93ad6ca1c8bd38`+승인 WIP이며,
`cell-page-source.sha256`, `cell-page-inputs.sha256`, `cell-page-review/manifest.json`에
소스/입력/PDF/옵션/실행 명령/WASM hash를 연결했다. 최종 소스 hash 재검사와 diff check도 통과했다.

### 자동 쪽번호 승인 및 원본 호스트 연결 경계 점검 (2026-09-29)

메인테이너가 위 자동 쪽번호 셀 2건의 시각 판정을 통과시키고 다음 절차를 승인했다.
해당 시각 증적은 재생성하지 않는다. 다음 대상은 원본 문서 종단 연결이다.

`output/7353/r19/section-scope/audit-original-host.rs`를 현재 Native library에 연결하여
원본을 변경하지 않고 실행했다. 상세 결과는 같은 경로의 `.log`다. 첫 admission 오류만
보고 구역 플래그를 완화하지 않도록 본문과 바탕쪽을 함께 확인했다.

| 원본 입력에서 확인한 대상 | 실제 관측 | 현재 연결 상태 |
| --- | --- | --- |
| 표 | 14개 모두 V2 prepare 성공 | 선택 표 준비이며 전체 문서 수용은 아님 |
| TAC 표 | p37/72/104/136/167/200/229/258/288의 9개 | 제한 호스트는 TAC를 거부 |
| 절대 위치 표 | p0 용지 위, p206/232 용지 아래, p324 쪽 아래 | 제한 호스트의 단 왼쪽/문단 위/offset0 계약 밖 |
| 어울림 표 | p299, 본문260자와 혼재, Para offset54/475HU | 줄별 가용 폭과 앵커/점유 연결 필요 |
| 본문 도형 | 19개 | 제한 호스트의 control admission 밖 |
| 구역 | 2단, 바탕쪽2개, 첫 바탕쪽 숨김, hide_empty_line=true | 기존 구역 orchestration/paint 연결 필요 |
| 문단 내부 저장 vpos 되감김 | p50/86/117/149/181/211/238/270/301 | 다단 소유 경계 미지원 |
| 격자/제목·keep-next·break-before | 격자0/0, 해당 문단 스타일0건 | 이번 원본의 차단 원인으로 보고하지 않음 |

호출 경로 점검 결과 `typeset/hosted.rs::typeset_hosted_section`은 TypesetState를
사용하지만 `typeset/section.rs::run_section`과 **별도의 문단 루프**다.
`host_section.rs::render_page`도 바탕쪽/쪽 테두리 인자에 None을 전달한다.
따라서 기존 기록의 “기존 호스트 연결”은 상태/출력 자료형 연결까지이며, 일반 문서
orchestration 연결 완료를 뜻하지 않는다. raw flags 가드를 지우거나 원본 속성을
삭제하여 통과시키면 조판 의미가 유실된다.

승인된 계획5.1의 기존 문서 흐름 연결을 다음 구현 단위로 유지한다. `run_section`의
문단 경계/단·쪽 상태를 소유자로 두고 표 배치 경계에서 V2 query/commit을 호출한다.
기존 표의 측정/컷을 만든 후 paint만 V2로 교체하지 않는다. V2가 확정한 점유 끝점을
호스트 예약과 PageItem::HostedTable/최종 paint가 함께 소비해야 한다. 바탕쪽 선택은
`document_core/queries/rendering.rs::assign_master_pages_for_section`, 최종 렌더링은
같은 파일의 build_render_tree 호출 경로를 사용하며 별도의 문서 엔진을 복제하지 않는다.

이번 절차는 승인 반영과 원본/실제 호출 경계 조사까지다. production 코드·WASM·기준값은
변경하지 않았고, 전체 문서 연결/회귀 통과 또는 공정률 증가로 계산하지 않는다.

### 일반 section 드라이버 연결 — 구현 및 남은 소유 줄 경계 (2026-09-29)

위 조사 이후 메인테이너의 다음 작업 승인으로 일반 문단 루프 연결을 구현했다.
이전 조사와 달리 이번 기록은 production 변경을 포함한다. 기준은
`cc13f573a43ba96093198f9bcf93ad6ca1c8bd38`+승인된 WIP이며 기존 변경은 보존했다.
아래 증적의 기본 경로는 `output/7353/r19/section-scope/`다.

적용 규칙: 명시 쪽/단 나누기와 문단 스타일의 쪽 나누기는 section 드라이버가
원래 문단/스타일로 한 번 처리한다. V2 조각의 내용 컷과 점유 끝점은 query/commit
결과가 소유하고, Legacy 표 컷·저장 vpos 보정으로 다시 계산하지 않는다.

| 실제 소비 경로 | 구현/검사 |
| --- | --- |
| 입력 → 문단 경계 | `host_section.rs:223`이 원본 문단을 전달하고 `section.rs:281`의 기존 `prepare_paragraph_boundary`가 실행된다. 텍스트 projection의 스타일 쪽 나누기만 지역 복제본에서 제외한다. 원본 IR/렌더링 스타일은 변경하지 않는다. |
| 공통 source-order loop | `section.rs:314` → `HostedSectionFlow::place_paragraph`. `hosted.rs`의 별도 문단 순회·페이지 초기화/종료를 없앴다. GDB 실제 stack은 `shared-driver-call-path.log`다. |
| 컷/요구 높이 → 예약 | `hosted.rs:80` 이후 실제 단·남은 높이를 query하고, commit 성공 조각의 occupied.bottom으로 `align_flow_to`한다. 미수용 query는 컷을 소비하지 않고 기존 단/쪽 전진을 호출한다. |
| 실제 배치/후속 내용 | `PageItem::HostedTable`의 같은 immutable packet을 기존 LayoutEngine이 소비한다. 뒤 문단도 HostedParagraph packet의 next_y를 예약한다. 원래 높이로 수용한 뒤 paint에서 확장하지 않는다. |
| 분기와 종료 | V2는 Legacy 저장 vpos/표 컷/문단 tail 보정을 거치지 않는다. 기존 경계가 문단을 흡수하면 V2는 명시 오류로 종료하며 남은 text/table도 finish에서 검사한다. Legacy 호출은 정책 None으로 기존 흐름을 유지한다. |

`tests/cases/issue_7353_hosted_section.rs:462`부터 여섯 계약을 추가했다.
명시 쪽/단 경계 뒤의 표 원점, 표 점유61px와 뒤 빈 소유 줄/문단 원점,
분할 뒤 스타일 쪽 나누기, 실제 출력 쪽번호12개, 마지막 표/빈 소유 줄의 단일 출력,
미수용 문단의 조용한 누락 방지를 검사한다. 61px는 입력 여백3+4px와
9pt 글줄의 고정 advance18px×3으로 산정한 합성 계약이며 한컴 일치 증거는 아니다.
쪽/단/스타일 경계3건은 수정 전 UnsupportedHost로 실패하고 수정 후 통과했다
(`../frame-after-child/shared-driver-before-issue_7353_hosted_section.log`).
나머지3건은 변경 후 추가된 보호 검사이며 수정 전 실패 증거로 세지 않는다.

집중 검사203건 통과: hosted_section16, host_bridge9, cell_page_field7,
page_number_timeline5, document_flow160, sandbox2, Legacy #6132 1, p122 3.
명령은 `bash output/7353/r19/frame-after-child/run-policies.sh shared-driver-regression ...`이며
기록은 `shared-driver-focused-final.log`, `shared-driver-regression.log`,
`shared-driver-cli-tests.log`와 연결된 개별 로그다. 중간의 잘못된 테스트 파일명 및
p122의 CLI 환경변수 누락은 harness 실패로 구분했다. 정확한 파일명과
`CARGO_BIN_EXE_rhwp=/home/edward/mygithub/rhwp/target/pr-review/debug/rhwp`로 재실행했다.
Native lib/CLI build, Native/wasm32 lib Clippy `-D warnings`, fmt/diff check도 통과했다.
전체 workspace CI를 실행하거나 완료로 보고한 것은 아니다.

Docker fresh WASM은 종료0/7분19초로 완료했다(`shared-driver-wasm-build.log`).
WASM SHA256: `39f746604d12ebd6292c2d5fe884ee71482efa33d767fd275382d17d2ace6f68`.
기존 승인된 `issue7353_body_frame_review/portrait-saved.hwp`/`portrait-2020.pdf`를
그대로 사용하여 `capture-shared-driver.mjs --body` 및 `--body --wasm`을 실행했다.
Native 출력 JSON은 연결 전 `host-text-native.json`과 동일하다. 실제 Chrome의
WASM과 Native는 비수치 차이0, 수치9개 최대차이1.42e-14로 허용오차1e-9px 안이다.
`shared-driver-body-review`의 Native/WASM compare와 standalone overlay를 직접 판독해
BRAVO 뒤 빈 줄, CHARLIE, 다음 쪽 DELTA/ECHO/AFTER 원점 유지와 누락 없음을 확인했다.
이는 기존 승인 대조군의 무회귀이지 새로운 표 종단 시각 승인 요청이 아니다.
source SHA 목록은 `shared-driver-source.sha256`, 입력/PDF/WASM hash와 명령은
각 backend manifest에 있으며 최종 source hash 재검사도 통과했다.

**새 정상 분할 표의 시각 검증은 미완료다.** 새 대조군 생성 중 첫 입력은
가로가 긴 용지인데 세로 방향으로 지정했고 셀 내부 나눔 대신 CellBreak(셀 전체 이동)를
사용했다. 해당 입력/PDF/비교 그림은 `shared-driver-input.hwpx` 계열로 보존하되
통과 자료로 쓰지 않는다. 용지를24000×30000HU 세로로, 나눔을 RowBreak(파일값2,
HWPX CELL, 셀 안 글줄 나눔)로 바로잡은 별도 `shared-driver-split-input.hwpx`를
한컴2020 MCP로 저장한 뒤 그 **동일 저장본**을 PDF로 변환했다. renderer 가드는
완화하지 않았고 저장된 LineSeg도 고치지 않았다.

- 저장 job: `dcefe94d-be07-403c-992d-ca1ac7c8556b`, PDF job: `5cf08ea8-49bc-4331-97b1-dfb8d3fa4e06`.
- 입력: `shared-driver-split-saved.hwp`, SHA256 `2738c34a6123168e1865ed82a9cb6a797c3d65bea90adf9df4a9f9c1fcfb921c`.
- 기준: `shared-driver-split-2020.pdf`, SHA256 `6138f3a977d668a35b07f99eeeb81ea5de16959582d5ff3b638ed6d57e6b3407`.
- PDF3쪽을 직접 판독하면 ROW20~25의 이어진 표 뒤에 AFTER가 있다. PDF 텍스트 추출 순서는 AFTER가 먼저이므로 추출 순서로 배치 순서를 판단하지 않는다.
- 저장 소유 문단p1은 빈 텍스트, chars9, 명시 Page 경계, 줄높이900HU/줄간격450HU이나 segment_width=0이다. `shared-driver-split-inspect.log`에 원본 값을 기록했다.
- 현재 표 제거 projection이 이 줄을 독립 본문 텍스트로 넘기므로 `host_text::prepare → stored_text::localize:442`에서 `stored text requires intact single-segment rows`로 거부된다. 실제 GDB stack은 `shared-driver-split-rejection.log`다.

다음 연결은 이 **표가 차지한 빈 소유 줄의 저장 의미와 실제 흐름 점유**다.
폭0을 폭 가득한 독립 빈 줄로 바꾸거나 줄을 삭제하는 우회는 하지 않는다.
줄/개체의 공동 점유와 뒤 문단 원점을 독립 PDF에 대조한 뒤 정상 분할 표의
종단 검증을 마쳐야 한다. 이 미지원 입력을 통과·시각 완료로 세지 않으며,
#7008 전체 원본, TAC/절대 위치/어울림·도형/바탕쪽/다단 연결과 기본 엔진 전환은 남아 있다.

### 폭0 표 소유 줄의 공동 점유 — 구현 및 검증 (2026-09-29)

위 미지원 경계를 다음 작업 승인으로 구현했다. 기준은 동일한
`cc13f573a43ba96093198f9bcf93ad6ca1c8bd38`+승인 WIP다. 이 절의 증적 경로는
`output/7353/r19/section-scope/`이며 기존 승인 대조군은 변경하지 않았다.

입력은 앞 절에서 한컴2020으로 정상 저장하고 동일 저장본에서 PDF를 만든 자료다.
이를 `tests/fixtures/issue7353_host_owner_review/`에 원본 HWPX·저장 HWP·PDF와
생성 job/hash를 보존했다. LineSeg를 수동 작성하거나 폭을 덮어쓰지 않았다.
독립 PDF는 BEFORE1쪽, ROW01~19 2쪽, ROW20~25와 AFTER3쪽이다.
3쪽 표 하단96.759pt/AFTER baseline104.442pt와 저장 AFTER vpos8175HU를 확인했다.
1500HU 위 여백을 더한 문단 원점은96dpi에서129px다. 셀 끝 간격은 기존
`OmitFinalParagraphGap` 검증 정책을 명시적으로 선택했고 기본값은 바꾸지 않았다.

| 생산 → 소비/경계 | 적용 규칙과 실제 경로 |
| --- | --- |
| 저장 줄 → 공통 구성 | `host_section.rs:214` → `host_text.rs:62`는 이미 수용한 단일 non-TAC/TopAndBottom/offset0 표의 폭0 소유 줄만 처리한다. 900HU 높이·765HU baseline·450HU gap을 실제 TextLine/FlowBlock에 보존한다. 독립 본문 텍스트의 저장 폭 검사에는 변화가 없다. |
| 구성 → 요구 높이/컷 | `hosted.rs:90`에서 소유 줄의 높이와 advance 중 큰 공동 요구량을 먼저 검사한다. 줄 또는 표만 fit하는 경우 어느 쪽도 소비하지 않고 기존 단/쪽으로 함께 이월한다. 표 query 실패도 소유 줄을 남기지 않는다. |
| 예약 → 최종 배치 | `hosted.rs:124`는 commit한 첫 표와 같은 frame에 소유 줄 packet을 배치한다. 표 점유 끝·줄 점유 끝·advance 끝의 최댓값을 예약하고 LayoutEngine은 동일 HostedParagraph/HostedTable packet을 소비한다. 소유 줄은 첫 조각에만 존재하고 연속 조각 뒤에 다시 붙이지 않는다. |
| 종료/비적용 | 별도로 입력한 빈 문단은 독립 글줄로 유지한다. TAC, 폭0 일반 본문, 잘못된 baseline/복합 소유 줄은 이번 수용 대상이 아니다. #7008 전체 연결/편집 후 재조판/기본 엔진 전환 완료를 뜻하지 않는다. |

`tests/cases/issue_7353_hosted_section.rs:586` 이후7건을 추가했다. 실제 저장본의
ROW01~25 단일 보존,19→20 컷, 첫 조각의 폭0/높이12px 줄, 뒤 문단129px 원점,
원본 IR 불변을 검사한다. 합성 경계는 표만 fit/줄만 fit/공동 예산과 정확히 일치,
별도 빈 문단, TAC 및 잘못된 줄 거부를 검사하며 한컴 출력 일치 증거로 세지 않는다.
정상 저장본 검사는 수정 전 의도한 `stored text requires intact single-segment rows`로
실패했다(`../frame-after-child/owner-before-issue_7353_hosted_section.log`:16 PASS/1 FAIL).
수정 후 hosted_section23건 통과다. 총 집중 검사210건:23+9+7+5+160+2+1+3.
`owner-regression.log`와 마지막 추가 계약/새 CLI로 실행한 `owner-final.log`를 연결한다.

Native lib/CLI build, Native 및 wasm32 lib Clippy `-D warnings`, fmt/diff check 통과.
각 명령/결과는 `owner-{build,cli-build,native-clippy,wasm-clippy,fmt}.log`에 있다.
전체 workspace CI/R5는 이번 검증 범위가 아니다. source 파일 hash는
`owner-source.sha256`와 `owner-source-check.log`로 고정했다.

Native의2·3쪽 review/standalone overlay를 직접 판독했다. ROW19→20 이어짐,
마지막 표 뒤 AFTER, 누락/중복 및 불필요한 빈 쪽 없음을 확인했다. 글꼴 폭/굵기와
2쪽 표 하단 약1.13px(96dpi) 차이가 남는다. 3쪽 표 끝/뒤 문단 경계는 약0.012px 차이다.
이 작은 외곽 차이를 숨기거나 픽셀 완전 일치로 보고하지 않는다.

Docker fresh WASM 빌드는 종료0/7분23초로 완료했다(`owner-wasm-build.log`).
WASM SHA256은 `af7620ee0479fc1614baddd32e7139a3cd1a077a710b88f2c4cf193dd8fb5575`다.
최적화 완료 전의 예비 캡처는 최종 증거로 쓰지 않고, 빌드 종료0 확인 후
`node output/7353/r19/section-scope/capture-shared-driver.mjs --owner --wasm`과
`--body --wasm`으로 최종 산출물을 다시 캡처했다. Native 캡처는 같은 명령에서
`--wasm`을 빼고 실행했다. backend manifest에 source/input/PDF/WASM hash와 정책을 기록했다.
신규3쪽은 비수치 차이0/수치27개 최대1.42e-14, 승인 본문2쪽은 비수치 차이0/
수치9개 최대1.42e-14로 모두1e-9px 이내다. 승인 본문의 Native JSON도 이전과 동일하다.

최종 `owner-review/wasm-review-{2,3}.png`와 `wasm-overlay-{2,3}.png`를 직접 판독해
Native에서 확인한 분할·뒤 문단 보존 및 남은 차이가 동일함을 확인했다.
기존 본문 대조군의 `owner-body-review/wasm-review-2.png`에서도 후속 문단 원점이 유지된다.
자동 검사와 직접 판독 후 메인테이너가 “시각 판정 통과. 다음 절차를 진행하세요”로 승인했다.
이후 범위는 원본의 TAC/절대 위치/어울림·도형/바탕쪽/다단 연결이며,
기본 Studio 경로나 전체 CI/R5 완료로 확대하지 않는다.

### 본문 TAC의 공통 section 호스트 연결 (2026-09-29)

앞 절의 시각 승인을 반영하고 다음 미지원 경계인 본문 TAC를 연결한다. 기존 V2의
저장/재조판 TAC 줄 구성·원자적 표 배치 규칙은 재구현하지 않고 `body_inline.rs`로
분리하여 standalone document와 기존 section 호스트가 함께 소비하게 한다.
입력의 실제 셀 크기/저장 줄 → InlineTables의 기준선/바깥여백/줄 소속 → 실제 단 예산의
FlowCursor → HostedParagraph의 immutable paint packet → LayoutEngine의 최종 노드 순이다.
일반 구역 루프가 쪽/단을 전진시키며 독립 DocumentV2 문서 루프는 호출하지 않는다.

독립 실물 대조군은 `tests/fixtures/issue7353_tac_overflow_review/sales-saved.hwp`와
**동일 HWP에서 출력한** `sales-2020.pdf`를 재사용한다. 생성 job/hash는 해당 README에 있다.
한컴 PDF의 실제 테두리 경계는72dpi에서(57.809,106.563)~(538.870,323.406)pt다.
뒤 자료출처, 셀의 수식 결과100.0과1쪽 종료까지 검사한다. 입력/기준을 다시 생성하지 않는다.
변경 전 정적 exporter는 동일 저장본을 `table host requires resolved anchor/line/margin policy`로
거부했다(`section-scope/inline-sales-before.log`). 새 호스트는 이를1쪽으로 수용한다.

합성 경계는 같은 줄 두 TAC, 명시 개행, 너비 부족, 사이 공백/바깥여백, 오래된 선언 높이,
빈 본문보다 큰 TAC와 뒤 문단을 검사한다. oversize의 `.hwp`는 합성 계약이며 그 디렉터리의
PDF는 `.hwpx` 출력이므로 같은 입력의 시각 근거로 섞지 않는다. 호스트 예산 검사에 필요한
normal 단을 명시한 합성 IR로만 사용한다. 남은 단 속성을 가드 완화로 수용하지 않는다.
새 TAC 지원에 따라 기존 'TAC면 무조건 거부' 검사는 어울림 표의 명시 거부로 바꾸며,
TAC/떠 있는 표 혼합과 미연결 PageNumberPos story는 별도로 거부하는 계약을 둔다.

| 구현 주장 | 실제 소비 경로와 검사 |
| --- | --- |
| 저장/재조판의 공동 TAC 구성 | `document_input.rs:219`와 `host_text.rs:73` → `body_inline::prepare`. 저장은 기존 tac::compose, 재조판은 실제 자식 PreparedTextTable 크기를 tac_fresh에 공급한다. 동일 줄 여러 표 높이를 합산하지 않으며 각 원래 control index를 보존한다. |
| 표 원자성과 단/쪽 예약 | `host_text.rs:310` → 기존 `FlowCursor::fit_body_until`. InlineTables는 같은 줄을 원자적으로 수용한다. 부족하면 host가 단/쪽을 전진하고, 빈 본문보다 큰 첫 TAC 줄만 기존 승인된 overflow 규칙을 적용한다. 별도 자식 페이지 컷을 생성하지 않는다. |
| paint와 뒤 문단 | fit.tables의 확정 placement → 같은 TextPaint::build_node → HostedParagraph packet → `layout.rs:7070`. 실패한 fit은 cursor를 소비하지 않는다. `hosted.rs:168`은 초과 높이 TAC 뒤 실제 후속 문단이 있을 때만 다음 단/쪽을 열어 종료 빈 쪽을 만들지 않는다. |
| 미지원 보존 | 원본 문단 장식은 source/resolved 두 검증을 거친다. 셀 AutoNumber는 물리 쪽번호로 paint하지만 별도 PageNumberPos story의 timeline은 아직 미연결이므로 명시 거부한다. TAC/float 혼합도 별도 명시 거부다. |

최종 집중 검사211건 통과: hosted_section28, document_flow160, host_bridge9,
cell_page_field7, page_number_timeline5, sandbox2. `inline-regression.log` 및
`../frame-after-child/inline-final-*.log`에 명령/결과를 보존했다. 실제 저장본의 수정 전
거부는 위 `inline-sales-before.log`, 새 수용과 PDF 테두리/뒤 문단의 최종 좌표 검사는
`hosted_normal_saved_sales_tac_keeps_cells_and_after_source`다. 합성 예산/비적용 검사들은
변경 후 보호 검사이며 정상 한컴 저장본의 증거로 세지 않는다.

Native build, Native/wasm32 lib Clippy `-D warnings`, fmt/diff check 통과했다.
로그는 `inline-{build-final,native-clippy,wasm-clippy,fmt}.log`다. 검증 source는
이전과 동일 HEAD+WIP, `inline-source.sha256`/`inline-source-check.log`로 고정한다.
처음 WASM 빌드는 문단 source 장식 guard 보완 때문에 중지했고 완료 증거로 세지 않는다.
완성 코드의 빌드는 `inline-wasm-build-final.log`로 별도 구분한다.

Native 최종 compare/standalone overlay를 직접 판독했다. 저도주 판매 표의 전체 외곽,
셀 배치,100.0 결과와 뒤 자료출처의 위치가 보존된다. 대체 폰트 폭/굵기와 일부 글립 및
작은 인쇄/래스터 차이는 남는다. 테두리 경계는 독립 PDF와96dpi에서0.6px 이내다.
이전 승인된 소유 줄3쪽/본문2쪽 대조군의 Native JSON은 이전 출력과 byte-identical이다.
Chrome 실행 오류2회는 환경 실패로 구분했고 재실행한 최종 캡처의 성공을 확인했다.
합성 동일 줄 두 TAC에서 발생한 기존 `LAYOUT_TABLE_OVERLAP` 로그는 세로 구간만
겹치는 후보이며, 정식 최종 x/폭 검사는 두 표의 가로 영역이 분리됨을 확인한다.

최종 Docker fresh WASM은 종료0/7분29초로 완료했다(`inline-wasm-build-final.log`).
WASM SHA256은 `b6120e5b7408b0f1788a0c6dd255d07744861a14832d01e4d23a918e82d59afe`다.
빌드 완료 뒤 `node output/7353/r19/section-scope/capture-inline.mjs --wasm`,
`--owner --wasm`, `--body --wasm`을 순서대로 실행해 모두 종료0을 확인했다.
각 `inline{,-owner,-body}-review/wasm-manifest.json`은 입력/PDF/source/WASM hash와
실행 명령을 기록한다. Native 대비 비수치 차이는 모두0이며, 수치 차이는 각각
223개/최대4.55e-13px,27개/최대1.42e-14px,9개/최대1.42e-14px로1e-9px 이내다.

`inline-review/wasm-review-1.png`와 독립 `wasm-overlay-1.png`를 직접 판독했다.
표 외곽·행/열 경계, 합계100.0과 뒤 자료출처가 유지되며 Native에서 관측한 폰트·미세 인쇄
차이도 동일하다. `inline-owner-review/wasm-review-3.png`의 ROW20~25와 뒤 AFTER 역시
기존 승인 위치를 유지한다. 최종 source hash와 diff check도 통과했다.
이번 신규 판정 대상은 판매 현황1쪽의 **본문 TAC 호스트 연결**이다. 기존 대조군의 재승인을
요구하지 않는다. 자동 검사/직접 판독은 완료했고 신규 연결의 메인테이너 시각 판정은 대기한다.
Studio 기본 경로 전환, #7008 전체 수용, 전체 workspace CI/R5 완료는 이번 결과에 포함하지 않는다.

#### 현재 WASM의 Studio 실행 준비 (2026-09-29)

사용자의 Studio 확인 요청으로 이 worktree의 Vite를 `http://127.0.0.1:7353/`에 실행했다.
기존 주 저장소의7700 서버는 변경하지 않았다. package-lock이 동일함을 확인한 뒤
기존 node_modules를 symlink로 재사용했으며 WASM 재빌드는 하지 않았다.
명령은 `cd rhwp-studio && npm run dev -- --host 127.0.0.1 --port 7353 --strictPort`다.
실제 Chrome에서 Studio 초기화 및 `open-document-bytes`를 통한 `sales-saved.hwp` 열기
성공과 창 제목을 확인했다. HTTP로 받은 WASM SHA256도 위 `b6120e5b...` 빌드와 동일하다.
캡처는 `section-scope/studio-legacy-current-wasm.png`다.

중요한 범위 구분: Studio WasmBridge는 `HwpDocument`를 사용하고 `HostedSectionV2`를
호출하지 않는다. 따라서 이 주소는 **현재 WASM을 사용하는 기존 Studio 경로**이며,
앞 절 V2 TAC 연결을 Studio에서 검증하는 화면은 아니다. V2 결과의 Studio 내 검토에는
별도의 명시적 선택형 미리보기 연결이 필요하며, 이번 실행 준비에서 기본 엔진을 전환하지 않았다.

### 자리차지 표의 위치·여백을 공통 section 호스트에 연결 (2026-09-29)

작업지시자는 Studio의 V2 연결을 전체 공정률90% 이후로 미루고 다음 구현을 승인했다.
Studio 경로·기본 엔진은 바꾸지 않는다. 독립 DocumentV2에서 이미 사용하던 저장 폭0
소유 줄의 `ExcludedTable` 구성을 `body_excluded.rs`로 추출하고 같은 결과를
`HostedParagraphPlan`에 연결했다. 새 페이지 루프나 Legacy 컷 변환은 만들지 않았다.

| 생산 → 소비 | 규칙/경계 |
| --- | --- |
| `cell_anchor::compose_body` → `body_excluded::prepare` | 원본 문단/표 속성·저장 줄과 실제 자식 plan을 사용한다. 소유 줄, 별도 세로 offset, 위/아래 여백, 가로 원점을 하나의 AnchoredTable로 묶는다. common/mirror 여백 불일치·음수·미지원 앵커는 오류다. |
| `host_text::prepare_excluded` → `FlowCursor::fit_body_until` | 기존 section의 실제 남은 높이를 소비한다. 첫 offset과 소유 줄은 첫 자식 조각과 함께 수용하며 실패 시 소비하지 않는다. 이어받기에서는 offset을 반복하지 않고 각 조각의 바깥여백을 예약한다. |
| fit.tables → `TextPaint::build_node` → HostedParagraph packet | 예약과 paint가 동일 placement를 사용한다. 원래 control index와 최종 물리 쪽번호를 보존하고 paint 뒤 원점을 덮어쓰지 않는다. 다음 문단은 확정 advance에서 시작한다. |
| 처음/이어받기 소유권 | 분할 표가 block0에 머무르더라도 첫 문단 packet으로 반복 식별하지 않는다. 실제 packet commit 여부로 `first`를 결정한다. |
| 비적용 | 현재 신규 저장 앵커는 단일 단의 Para/Column Left + Para Top/자리차지다. 다단 앵커, Page/Paper 절대 위치, 가시 텍스트 소유 문단·어울림, 별도 쪽번호 story는 계속 미지원이다. 기존 무오프셋 호스트 계약은 유지한다. |

독립 규칙 근거는 기존 정상 저장 입력/PDF의 `tests/fixtures/issue7353/anchor-offset/README.md`와
기존 `ExcludedTable` 계약이다. 그 원본90문단의 HostedSection 전체는 다른 control admission에서
거부된다(`positioned-before.log`). **그 문서 전체가 새 호스트에서 통과했다고 보고하지 않는다.**
이번 최종 section 좌표 검사는 수동으로 구성한 합성 IR임을 테스트에 명시했다.
prefix18px, 소유 줄12+6px, offset(5,7), 바깥여백(2,4,3,2), 자식 줄18px에서
실제 표 x27/y58, 뒤 문단y78을 검사한다. 첫 조각 예산 부족/3조각 이어받기의 원점
40→33→33, 소유 줄1회, alpha/beta/gamma/suffix 각각1회와 끝 쪽 보존도 검사한다.
잘못된 mirror·음수·용지 초과·다단·Square 입력은 명시적으로 거부한다.

집중 검사219건 통과: hosted_section31, excluded_anchor_offset5, document_flow160,
host_bridge9, cell_page_field7, page_number_timeline5, sandbox2.
`section-scope/positioned-tests.log`와 `../frame-after-child/positioned-final-*.log`가 실행 기록이다.
신규 합성 계약은 처음 예산을 잡을 때 실제 줄 높이와 줄 advance를 구별하여 정정했다.
기존 excluded_anchor_offset 계약1건은 첫 조각의 아래 여백2px를 누락한30px 예산으로
실패했다. 현행 FlowCursor는 첫 조각부터 아래 여백을 예약하므로 입력의7+3+20+2=32px로
계약을 수정하고31px 실패/32px 성공을 검사했다. 조판 코드를 바꾸거나 허용치를 늘린 것은 아니다.
`flow.rs` SHA256 `737fb93b3b4df23dfacd5d1bb93ad55bebee234a5de4e629f24b1ddb9e1c62e6`은
이전 inline-source manifest와 같다. 실패 원문은 `positioned-issue_7353_excluded_anchor_offset.log`에 보존했다.

Native build, Native/WASM lib Clippy, fmt/diff/source hash check 통과. 로그는
`positioned-{build-final,native-clippy,wasm-clippy}.log`, source는 `positioned-source.sha256`다.
판매 표1쪽·분할 소유 줄3쪽·일반 본문2쪽 Native JSON은 이전 inline 출력과 byte-identical이다.
이 대조군은 기존 시각 승인을 반복 요청하지 않는다. 비영 offset의 신규 호스트 경계는 합성
최종 좌표 검사이며, 해당 경계의 새로운 한컴 종단 시각 일치 증거는 아직 없다.

추가 backend 대조는 `section-scope/positioned-backend-probe.rs`로 기존 분할 소유 줄
fixture의 복사본에서 flow_with_text, offset(375,525)HU, 바깥 위/아래(225,150)HU만
바꿔 HWP로 직렬화·재파싱한 **합성 입력**이다. 원본 fixture와 PDF는 수정하지 않았다.
`positioned-synthetic.hwp`를 이전 절편의 정적 `inline-export`로 실행하면
`table host requires resolved anchor/line/margin policy`로 실패하고, 새 Native 호스트는
수용한다(`positioned-synthetic-before.log`, `positioned-synthetic-native.json`).
이 신규 지원의 전후 실행 증거와, 원본90문단의 다른 admission 실패를 구별한다.

최종 Docker fresh WASM 빌드는 종료0/7분24초로 완료했다(`positioned-wasm-build.log`).
WASM SHA256은 `1f998f702768f8b4a2b6a539fef10cc5c5fa9be60b01e0528f4c95bc3a0154a4`다.
`node output/7353/r19/section-scope/capture-positioned.mjs`에 각각
`--synthetic --wasm`, `--owner --wasm`, `--wasm`, `--body --wasm`을 전달한
실제 Chrome 검사는 모두 종료0이다. 각 `positioned{,-synthetic,-owner,-body}-review`의
manifest와 `backend-comparison.json`에 입력/source/backend 증거를 남겼다.
Native 대비 비수치 차이는 모두0이며 최대 좌표 차이는 합성/소유 줄/일반 본문
1.42e-14px, 판매 표4.55e-13px다. 합성 입력은 기준 PDF 없이 backend 계약만 검사했다.

fresh WASM의 `positioned-owner-review/wasm-review-3.png`와
`wasm-overlay-2.png`를 직접 판독했다. ROW20~25와 뒤 AFTER의 보존, 이어받기 외곽을
확인했다. 기존 대체 폰트 폭·글립·선 두께 차이 및 첫 조각 하단 약1.13px(96dpi)의
차이는 남으며 이번 연결에서 새 위치 변화는 없다. 기존 승인 대조군의 재판정은 요청하지 않는다.
신규 비영 offset의 한컴 종단 시각 일치는 여전히 미검증이고 전체 workspace CI/R5 완료로
보고하지 않는다. Studio V2 연결은 작업지시자가 정한 전체 공정률90% 이후로 보류한다.

### 본문 쪽번호 선언을 공통 section 호스트에 연결 (2026-09-29)

다음 작업 승인에 따라 정상 입력의 `PageNumberPos` admission 장애물을 처리했다.
쪽번호 규칙을 새로 만들지 않고 기존 `page_number::PageNumberStory`를 재사용한다.
`prefix90-saved.hwp`는 앞 절에서 본문p3의 선언 때문에 거부되었다. 연결 뒤에는 다른 표의
앵커 지원 조건에서 거부된다(`section-scope/story-prefix-admission.log`). 원본 전체 수용은 아니다.

- 입력→구성: 정상 저장 `tests/fixtures/issue7353/page-number-timeline/timeline-saved.hwp`와
  동일 HWP의 `timeline-2020.pdf`를 그대로 사용했다. 생성 출처와 독립 관측은 해당 README에 있다.
  기존 standalone의 문단 진입 UTF-16 슬롯 검사를 `validate_body_entry`로 공유했다.
  text 전용 projection에서 비점유 선언만 제외하고 원본 텍스트·char_offsets·LineSeg를 유지한다.
- 소비→실제 배치: `run_section`이 확정한 `HostedParagraphFragment`의 실제 TextLine 소유를
  확인한다. fit 시도·빈 문자열 여부·spacing-only packet을 활성 근거로 쓰지 않는다.
  쪽별 수용 선언 중 소스 순서상 마지막 것을 선택하고 이어받기에서 과거 선언으로 되돌리지 않는다.
- 최종 출력: Legacy finalizer의 전역 `page_number_pos`는 이 명시적 V2 세션에서만 비우고,
  동일 PageNumberStory가 측정/배치한 immutable node를 실제 페이지 트리에 한 번 추가한다.
  기존 구역 루프/본문 예산/물리 번호는 바꾸지 않는다. 반복 render는 상태를 소비하지 않는다.
- 비적용: 셀 내부 선언, 동일 문단의 표와 선언 혼합, 문단 중간 선언, 비십진 형식,
  NewNumber/다른 시작번호와 구역 전환은 계속 미지원이다. Studio 기본 경로는 변경하지 않았다.

`tests/cases/issue_7353_hosted_page_number.rs`5건을 추가했다. 정상6쪽 계약은 독립 PDF의
가운데/왼쪽/오른쪽 edge와 baseline,1→2→3→4→숨김→6 번호를 확인한다. 같은 쪽 두 선언은
뒤 선언이 적용되고 번호 위치 변경으로 물리 번호를 재시작하지 않는다. 전체 비활성 대조와
본문 트리 동일성, source 불변 및 반복 render 동일성도 검사한다.
합성 경계는 예산 부족 시 선언/소유 줄의 다음 쪽 공동 이월, 이전 쪽 소급 변경 금지,
빈 선언 줄1000HU와 뒤 문단 간격1600HU 보존이다. 합성 변형을 한컴 생성본으로 세지 않는다.

수정 전 정적 librhwp에서 신규 정상/이월/소급금지3건은 실제 미지원 사유로 실패했고,
거부 대조1건은 통과했다(`frame-after-child/story-before-issue_7353_hosted_page_number.log`).
빈 줄 계약은 그 뒤 추가한 보호 검사이며 수정 전 검출 증거로 세지 않는다.
수정 후 집중 검사224건 통과: hosted_page_number5, hosted_section31, excluded_anchor_offset5,
table_v2_document_flow160, table_host_bridge9, cell_page_field7, page_number_timeline5, sandbox_document2.
로그는 `frame-after-child/story-final-*.log`다. 최초 실행의 잘못된 파일명
`issue_7353_document_flow`는 실행 준비 오류이며, 실제 `issue_7353_table_v2_document_flow`160건을
별도 재실행했다. 정상 판매 표 대조군 JSON은 이전 `positioned-native.json`과 byte-identical이다.

Native build와 Native/WASM lib Clippy `-D warnings`, fmt/diff/source check가 통과했다.
로그/소스 고정은 `section-scope/story-{build,native-clippy,wasm-clippy}.log`와
`story-source.sha256`다. HEAD+승인 WIP이며 전체 workspace CI/R5 완료가 아니다.
Native6쪽을 직접 출력하고 `capture-positioned.mjs --story`, `story-sweep.py native`로
192dpi compare/standalone overlay/review를 만들었다. 최초 sweep은 출력 디렉터리 준비
누락으로 실패했고 wrapper에서 디렉터리를 준비한 뒤 성공했다.
4쪽/6쪽 직접 판독에서는 번호의 오른쪽 정렬·재활성 및 본문 두 줄을 확인했고 폰트 폭/글립
차이는 남는다. 시각 비교용 좌표 이동·스케일 보정은 하지 않았다.

Docker fresh WASM은 종료0/7분23초로 완료했다(`story-wasm-build.log`). SHA256은
`6e583b6b8628c79e24b51a1f83ab911e5c81cd0cdd1d1b49028095e1ed7a8060`이다.
`capture-positioned.mjs --story --wasm`, `story-footer-review.mjs wasm`,
`story-sweep.py wasm`이 모두 성공했다. 실제 WASM6쪽 트리와 Native의 비수치 차이는0,
수치 차이69개/최대2.2737367544323206e-13px로1e-9px 이내다.
`story-review/wasm-manifest.json`과 `backend-comparison.json`에 실행·해시·대조를 보존했다.

직접 판독한 증거:

- `section-scope/story-review/wasm-footer-review.png`:6쪽 하단의 동일 영역/동일 배율 비교.
  가운데→왼쪽→오른쪽→해제→재활성 및 번호 연속을 확인했다. 전체 페이지 증거의 대체가 아니다.
- `section-scope/story-review/wasm/compare/compare_004.png`
- `section-scope/story-review/wasm/overlay/overlay_004.png`
- `section-scope/story-review/wasm/review/review_004.png`

canonical sweep의4쪽 `visual_accuracy_proxy_percent`는5.4026이다. 글립/잉크 형태와
미세 위치 차이가 남으므로 이 수치나 합성 계약으로 완전한 한컴 피델리티 통과를 선언하지 않는다.
번호의 적용 쪽·정렬 기준·기준선 검증과 글꼴 차이를 구분한다. Native/fresh WASM 결과는
동일하고 기존 판매 표 출력도 유지했다. 최종 source/input/test hash와 diff 검사를 통과했다.
전체 CI/R5 및 Studio 연결은 이번 완료 범위가 아니며, Studio 전환의90% 조건을 유지한다.

### 문단 뒤 자리차지 표 예약을 공통 section 호스트에 연결 (2026-09-29)

다음 작업 승인으로 저장 문단의 양수 폭 소유 줄 뒤에 Para/Top/Left 기준으로 놓는
TopAndBottom 표를 연결했다. 기존 standalone의 규칙을 새 문서 루프로 복제하지 않는다.
정상 저장본은 `tests/fixtures/issue7353_stored_anchor_review/`의 anchor/defer와 동일 HWP에서
출력한 PDF다. 생성 이력·독립 좌표는 해당 README에 있다. 기본24행 표의2쪽 이어받기와
3행 atomic defer의 “제목/뒤 본문은1쪽, 표만2쪽”을 서로 다른 계약으로 유지한다.

생산→소비→최종 배치 경로:

| 책임 | 실제 호출/결과 |
| --- | --- |
| 줄 구성 | `host_text.rs::prepare_positioned`가 기존 TextComposer의 동일 결과에서 실제 줄과 ParagraphEnd를 보존한다. source IR/저장 LineSeg를 바꾸지 않는다. |
| 위치와 예산 | `body_anchor.rs::resolve/flow` → `body_flow.rs::AnchoredFlow::fit`. standalone도 같은 예약 생성 함수를 사용한다. 첫 위치·이어받기 위 여백·조각별 아래 여백·기존 saved-frame clearance를 공유한다. |
| 컷/실제 paint | `host_anchor.rs::fit`에서 같은 FlowFit의 cuts/placement를 TextPaint에 전달한다. 출력 후 별도 좌표 clamp/높이 확장은 없다. 실패한 offset-only query는 소비하지 않는다. |
| 호스트 예약 | `typeset/hosted.rs::place_paragraph/drain_pending`이 수용된 소유 TextLine 끝을 기준으로 query한다. 요구 위치와 뒤 줄 원점을 구분하고, table fit 성공 시에만 실제 advance를 반영한다. |
| 이월/종료 | pending queue가 다음 물리 쪽/단에서 정확한 child cursor를 재개한다. 본문 종료에도 queue를 소진한다. `section.rs`는 다음 문단의 명시적 break 적용 **전** 및 최종 페이지 확정 전에 대기 표를 완료한다. |

표가 첫 쪽에 하나도 수용되지 않으면 원래 세로 offset을 빈 본문 공간으로 소비하지 않는다.
뒤 본문은 현재 쪽에서 계속할 수 있고, 다음 쪽 표는 위 바깥여백만 사용한다. 이미 일부가
수용된 경우는 정확한 이어받기 컷을 우선 소진한다. continuation packet은 문단 진입을
재발화하지 않는다. Square/절대 위치/다단/표 내부 쪽번호 선언은 명시적 미지원으로 유지했다.
Legacy 기본 경로·Studio 선택·분할 알고리즘 `flow.rs`는 바꾸지 않았다.

`tests/cases/issue_7353_hosted_anchor.rs`8건을 추가했다. 정상 저장4변형의 첫 위치/이어받기
여백,24행 각1회 및 뒤 문단, 정상 atomic defer, 정상 긴 줄간격, 정상90문단17쪽의
마지막 빈 소유 줄과 표 좌표를 검사한다. 합성 경계는 본문 종료 후 남은 표, 자식만 fit하고
자식+여백은 fit하지 않는 예산의 NoProgress, 복수 대기 표 뒤 명시적 page break, 소유 줄과
겹치는 위치의 거부다. 합성 변형은 한컴 생성본/시각 근거로 승격하지 않는다.

수정 전 static export는 정상 anchor에서 `table host requires resolved anchor/line/margin policy`
로 실패했다(`section-scope/anchor-host-before.log`). 이전 librhwp에서 최초6계약 중 신규
수용4건 실패/거부대조2건 통과했다(`frame-after-child/anchor-host-before-*.log`). 이후 추가한
prefix/복수 대기 계약은 이 최초 검출 건수에 포함하지 않는다. 구현 중 명시적 page break를
대기 표 이동에 먼저 소비하는 오류를 추가 검토로 발견했다. 올바른 source 순서의3쪽 계약은
보완 전2쪽으로 실패했고 보완 후 통과했다(`anchor-boundary-before-*` → `anchor-host-final-*`).

최종 집중 검사232건이 통과했다: 신규8, hosted_page_number5, hosted_section31,
excluded_anchor_offset5, table_v2_document_flow160, table_host_bridge9, cell_page_field7,
page_number_timeline5, sandbox_document2. 기존160건은 standalone 공유 함수 이동과
기존 분할/여백/빈 줄 계약의 보호이며 새 호스트 모든 경로의 증거로 대신하지 않는다.
로그는 `output/7353/r19/frame-after-child/anchor-host-final-*.log`다.
정상 `prefix90-saved.hwp`는 이제 호스트에서도17쪽까지 수용되고 마지막 표/소유 줄의
독립 좌표 계약이 통과한다. 원본 전체 #7008 또는 모든17쪽 시각 통과라는 의미는 아니다.

증적은 `output/7353/r19/section-scope/anchor-{host,defer}-*`에 보존한다.
최종 source는 `anchor-host-source.sha256`, Cargo/test/대표입력은
`anchor-host-input-test.sha256`으로 고정한다. HEAD+승인 WIP 상태이고 remote 변경은 없다.
초기 Chrome146 실행이 실패해 설치된 Chrome152로 캡처 도구를 전환했다. Python 실행명도
설치된 `python3`를 사용했다. Native 이미지에서는 표 외곽·행20~24·뒤 문단 및 defer의
쪽 소유를 확인했고, 대체 폰트 폭/글립·인쇄 선 잔차는 남는다. 좌표 이동·배율 보정은 없다.
최종 fresh WASM/시각 확인 결과는 아래에 이어 기록한다.

Native build20.07초, Native lib Clippy27.52초, WASM lib Clippy29.16초 모두 종료0이다.
fmt/diff/hash 검사도 통과했다. 최초 Clippy의 불필요한 `as_deref_mut`를 제거하고 다시
실행했다. 명시적 break 순서 보완 때 진행 중 첫 WASM 빌드는 중단했으며 최종 source를
새 Docker 빌드로 검증한다. 이전 캡처를 최종 증거로 재사용하지 않는다.
최종 Native 재출력에서 판매표와 쪽번호 timeline JSON은 기존 승인 결과와 byte-identical이다.

최종 Docker fresh WASM 빌드가 종료0/7분23초로 완료되었다. WASM SHA256:
`f57fd33ae0c026446591b28ffb4984d57ec5e0e57c6f354708fcd4c41a95d101`.
`capture-positioned.mjs --anchor --wasm`, `--defer --wasm` 및
`python3 output/7353/r19/section-scope/anchor-host-sweep.py wasm` 모두 종료0이다.
Native 대비 비수치 차이0, 수치 차이272/53개, 두 입력 모두 최대2.2737367544323206e-13px다.
각 `anchor-{host,defer}-review/wasm-manifest.json`과 `backend-comparison.json`에
source·입력·독립 PDF·WASM hash 및 실제 명령을 보존했다. 마지막 hash/diff 검사도 통과했다.

fresh WASM `anchor-host-review/wasm-review-{1,2}.png`와
`anchor-defer-review/wasm-review-{1,2}.png` 네 장을 직접 판독했다. 동일 입력 PDF 대비
제목 다음 표의 첫 위치,01~19/20~24의 분할·누락/중복 없음, 최종 표 바깥 뒤 문단,
defer의1쪽 두 문단/2쪽01~03행을 확인했다. canonical
`anchor-host-review/wasm/overlay/overlay_002.png`도 직접 확인했다.
폰트 폭/굵기/글립 및 미세 인쇄 테두리 차이는 남는다. canonical ink-match proxy는
일반 표1/2쪽32.9838/35.08172%, defer1/2쪽7.43835/37.01575%다. 이것을 완전 일치
또는 승인 점수로 사용하지 않는다. 좌표/소유/내용 검증과 폰트 차이를 분리해 보고한다.

이번 호스트 연결의 구현·집중 검증·시각 판정 준비는 완료했다. 메인테이너의 새 경로 시각
판정은 아직 받지 않았다. 전체 workspace CI/R5 완료, 원본 전체 문서와17쪽 전수 시각
검증, Studio V2 전환은 이 결과에 포함하지 않는다. Studio90% 조건과 Legacy 기본값을 유지한다.

### 저장 어울림 표를 공통 section 호스트에 연결 (2026-09-29)

다음 진행 승인에 따라 `HostedSectionSession → run_section`의 남은 Square 연결을
구현했다. 앞 자리차지 절편의 메인테이너 시각 통과로 승격하지 않으며, 이번에는
기존 standalone에 이미 있는 규칙을 새로 구현하지 않고 공통 호스트에 연결한다.
원본 #7008 전체/편집 경로/Studio 전환은 여전히 남아 있다.

입력은 한컴 정상 저장 `issue7353_host_wrap_review/host-saved.hwp`와
`issue7353_body_wrap_review/wrap-saved.hwp`, 기준은 각 동일 HWP의 `*-2020.pdf`다.
기존 fixture README의 생성 job·hash·독립 HU/PDF 근거를 재사용하며 저장 LineSeg나
원본을 수정하지 않았다. 첫 입력은 빈 소유 줄과 표의 세로 구간이 겹치지만 왼쪽 폭이
충돌하지 않는 사례이고, 두 번째는 왼쪽6줄 뒤 전체 폭으로 복귀하는 사례다.

| 경로 | 공통 결과와 소비 |
| --- | --- |
| 원점/요구 높이 | 기존 `BodyAnchor::resolve/flow`의 `host_end_offset`, 여백과 `AnchoredFlow::fit`을 `host_anchor.rs::fit`이 그대로 소비한다. 같은 쪽의 모든 소유 줄이 수용된 경우에만 원점을 결정한다. |
| 컷/이월 | Square는 기존 qualification대로 온전한 같은 쪽 객체만 허용한다. 표+여백이 예산에 안 맞거나 소유 문단이 여러 쪽에 걸치면 오류이며, 좁은 저장 본문을 남기고 표만 이월하거나 부분 조각을 paint하지 않는다. 자리차지 pending/continuation은 비변경이다. |
| 본문과 물리 점유 | `typeset/hosted.rs`는 Square packet의 점유를 보존하되 본문 pen을 표 하단으로 전진시키지 않는다. `state/transition.rs::hosted_used_height`를 일반·종료 flush 양쪽에서 소비하여 마지막 본문보다 큰 객체의 물리 높이도 보존한다. Legacy 항목은 기존 높이를 유지한다. |
| 최종 배치/겹침 | `host_anchor.rs`의 실제 FlowFit → TextPaint와 같은 상자로 exclusion을 만든다. `host_section.rs`가 쪽별 모든 수용 packet의 앞뒤 줄/표와 대조한다. `body_flow::validate_exclusion`은 standalone과 공통이며 빈 줄 상자도 검사한다. paint 후 clamp/재확장은 없다. |
| 쪽 번호 | Square 빈 소유 문단의 선행 PageNumberPos 선언도 기존 entry 검증과 수용된 본문 줄로 활성화한다. 표 속 선언/미지원 혼합을 광범위하게 허용하지 않는다. |

`tests/cases/issue_7353_hosted_square.rs`6개 계약을 추가했다. 정상 HWP와 HWPX round-trip,
96/192dpi에서 표 외곽·소유 빈 줄·옆 본문·전체 폭 복귀·뒤 주석·쪽 번호·source 불변과
재출력 결정성을 검사한다. 합성 경계는 빈/가시 소유 줄 충돌, 뒤 줄/다른 표 충돌,
실제 객체+여백 끝의 ±1HU 예산, 본문 종료/명시적 쪽 전환의 점유 높이, 소유 문단 분할,
새 쪽에서 배제 영역 해제다. 합성 입력을 독립 한컴 관측으로 보고하지 않는다.

수정 전 정상 입력은 `body page-number declaration with table controls`로 거부되었다.
`square-host-before-*`의6실패는 수용 미지원 근거이며 6개의 기존 시각 결함 발견이라는 뜻이
아니다. 첫 빌드 종료 전 실행된 `square-host-after-*`도 이전 library 결과라 수정 후 근거에서
제외했다. 빌드 완료 후 `square-host-built-*`에서 정상 대조는 통과하고, 종료 시 물리 높이가
106.186667 대신277.053333px여야 하는 계약이 실패했다. 일반 flush에만 있던 packet 소비를
종료 flush까지 공통화한 뒤 통과했다. 가시 소유 줄 합성은 문자열만 바꿔 옛 문자 오프셋을
남긴 테스트 작성 오류가 있었으며 HWPX serialize/parse로 일관된 합성 입력을 만든 뒤
안전한 폭의 양성/넓힌 폭의 음성 대조를 검사했다. production 저장 수용 조건은 완화하지 않았다.

집중 검사236건이 통과했다: hosted_square6, hosted_anchor8, hosted_section31,
hosted_page_number5, host_square_wrap6, body_square_wrap4, document_flow160,
table_host_bridge9, excluded_anchor_offset5, sandbox_document2. 로그는
`output/7353/r19/frame-after-child/square-host-final-*`, 신규 최종6건은
`square-host-complete-issue_7353_hosted_square.log`다. 전체 CI를 실행한 결과는 아니다.

증적은 `output/7353/r19/section-scope/square-*`에 모았다. Native export/JSON과
192dpi `capture-positioned.mjs --square`, `--square-control`, canonical
`square-host-sweep.py native`가 완료되었다. 두 Native review를 직접 열어 제목/오른쪽 표,
왼쪽5줄·주석 및6줄→전체 폭 복귀를 확인했다. 기존 따옴표·폰트 폭/굵기/글립과 미세 인쇄
테두리 차이는 남으며 완전 픽셀 일치로 판정하지 않는다. source·test·HWP·PDF는
`square-host-source.sha256`과 `square-host-input-test.sha256`에 고정했다.
Docker fresh WASM/최종 검증 결과는 아래에 이어 기록한다.

추가 대조로 Legacy7158 3건과 기존 host_anchor_gap4건을 실행해 **총243 PASS/0 FAIL**이다
(`square-host-control-*`). 앞 절편의 정상 anchor/defer를 최종 Native로 재출력해 기존
`anchor-{host,defer}-native.json`과 `cmp` 동일도 확인했다. 소유 줄/어울림의 이번 연결이
자리차지 표의 첫 위치/이어받기를 바꾸지 않았다. `flow.rs` SHA256은 이전과 같은
`737fb93b3b4df23dfacd5d1bb93ad55bebee234a5de4e629f24b1ddb9e1c62e6`이다.
Native 최종 build18.65초, Native lib Clippy28.82초, WASM lib Clippy30.93초 모두 종료0이다.
fmt, 신규 test rustfmt, source/test/input hash, diff 검사를 통과했다. 최종 source 변경 없이
Docker 빌드 중이며 전체 workspace CI/PR 검증은 아직 실행하지 않았다.

최종 Docker fresh WASM은 종료0/7분29초로 완료되었다. SHA256:
`45a07849301b8231cc31e40a05f8d9475bab202ebe4b35c52adcc0ba6ba6950e`.
`capture-positioned.mjs --square --wasm`, `--square-control --wasm`과
`square-host-sweep.py wasm`이 완료되었다. Native 대비 비수치 차이0,
수치 차이327/216개·두 입력 모두 최대2.2737367544323206e-13px다. 각
`square-{host,control}-review/wasm-manifest.json`, `backend-comparison.json`에
실제 입력/PDF/WASM/source hash와 명령을 연결했다. 마지막 source/input hash 검사도 통과했다.

fresh WASM 두 `wasm-review-1.png`와 control의 canonical
`wasm/overlay/overlay_001.png`를 직접 판독했다. 빈 소유 줄 옆 오른쪽 표의 상단·하단,
왼쪽5줄과 주석, 다른 대조군의6줄 뒤 전체 폭 복귀, 제목과 쪽 번호를 확인했다.
기존 글꼴 외형/따옴표 폭·미세 인쇄 테두리 차이는 남는다. canonical ink-match는
57.2525/47.98725%이며 자동 시각 통과 기준으로 사용하지 않았다.

이번 **저장 Square 호스트 연결의 구현·집중 검사·fresh WASM 시각 판정**을 완료했다.
메인테이너가 위 두 비교 자료에 대해 시각 판정 통과를 확인했다. 전체 workspace CI, #7008 원본
종단, 절대 앵커/다단/바탕쪽/본문 도형, 편집 후 어울림 재조판과 Studio V2 전환은
이 완료 주장에 포함하지 않는다. Legacy 기본값과90% 이후 Studio 검토 조건을 유지했다.

### 저장 절대 위치 표와 다단을 공통 호스트에 연결 (2026-09-30)

다음 작업 승인에 따라 현재 승인 상태를 `4af6a8ce0`에 중간 커밋했다(99파일).
새로 들어온 `.agents/skills/rhwp-*`, `.codex/`는 이 타스크와 분리하여 stage하지 않았다.
이후 아래 변경은 그 commit 위 WIP이며 push/PR/전체 CI는 실행하지 않았다.

**입력 → 독립 기준 → 기대:** `issue7353_host_absolute_review/create.rs`가 LineSeg 없는
2단/용지 기준 표 입력을 생성하고, 한컴이 정상 HWP로 저장한 뒤 동일 HWP를 PDF로 출력했다.
최종 save job `9a391169-102f-47db-a82b-28de1e3d7aee`, PDF job
`4862edc8-8d1e-4296-bd55-86cfb0e1a4f3`, runtime11.0.0.9136/2쪽이다.
줄·단·표 외곽의 HU/PDF 기대값과 입력 변경 이력은 fixture README를 따른다.

| 경계 | 실제 공통 결과와 소비 |
| --- | --- |
| 소유 쪽/단 | `HostedAbsolute::prepare`가 원문 control UTF16 위치에서 소유 LineSeg를 결정한다. `host_section`이 `run_section`의 수용 packet에서 그 line index를 찾아 최종 page/column을 바인딩한다. 문단 첫/마지막 쪽이나 표 좌표로 소유 쪽을 추정하지 않는다. |
| 좌표/높이 | `host_absolute.rs::place`의 PageLayoutInfo Paper/Page/Column 기준과 `PreparedTextTable.plan`의 같은 폭·높이가 정렬→PageArea fit→동일 fragment paint로 이어진다. 아래 정렬에서 실제 용지 높이를 사용한다. 미완료 continuation/용지 밖은 오류이고 clamp·임의 축소는 없다. |
| 저장 다단 | `body_text::frame_starts`의 검증된0 리셋→`host_text`의 frame 컷→`typeset/hosted.rs::take_frame_break`→기존 `advance_column_or_new_page`. 동일 폭 Normal 단의 흐름 순서만 수용하며 겹치는 줄 하단을 경계로 오인하지 않는다. |
| 본문 점유/배치 | 절대 표는 쪽 소유 paint packet이고 용지 여백의 위치를 본문 pen에 더하지 않는다. TopAndBottom 외곽/바깥여백이 영향을 주는 단의 모든 실제 줄·표·Square 배제 영역과 대조한 뒤 session을 공개한다. 충돌이면 재조판 필요 오류이며 기존 저장 본문을 억지로 밀지 않는다. |
| 분할 비해당 | 이번 절대 표는 온전한 조각만 수용한다. 부분 fit를 완료로 보고하거나 뒤 paint에서 높이를 늘리지 않는다. 기존 Para anchor의 pending/분할/이어받기는 변경하지 않았다. |

신규 `issue_7353_hosted_absolute`5개 계약: 정상 HWP/HWPX round-trip·96/192dpi의
표 외곽/원문 불변/빈 줄/왼쪽→오른쪽→다음 쪽/뒤 문단, 절대 객체 없는 다단 대조,
충돌·용지 밖 명시 오류, Page 기준 좌표, 다음 쪽 소유 줄로 컨트롤을 옮긴 합성 경계다.
마지막 세 변형을 한컴 출력 일치로 보고하지 않는다. 소유 줄 합성의 초기 중간-control
메타데이터는 저장 partition 검증에서 거부되어 근거로 사용하지 않고, 정확한 끝-control
원문 축으로 계약을 고쳤다. 수용 조건 완화는 하지 않았다.

최종 입력은 수정 전 export에서 `table host requires resolved anchor/line/margin policy`로
거부된다(`absolute-before-final.log`). 수정 후 신규5 PASS는
`frame-after-child/absolute-owner-final-issue_7353_hosted_absolute.log`이다.
초기 `absolute-before-*`의3실패는 미지원 근거이며 기존 시각 회귀3건이라는 뜻이 아니다.
잘못된 border 리소스 초안의4실패와 그 export 실패 이후 만들어진 임시 비교는 폐기 판정했다.
원문 오류/실패 증거는 `absolute-border-draft/` 및 로그에 보존했다.

최종 생산 코드로 기존 대조240건 PASS: hosted_section31, hosted_square6, hosted_anchor8,
hosted_page_number5, host_square_wrap6, body_square_wrap4, document_flow160,
table_host_bridge9, excluded_anchor_offset5, sandbox_document2, host_anchor_gap4.
로그 `frame-after-child/absolute-final-*`. 신규5와 합계245 PASS이며 전체 CI가 아니다.
Native build23.24초, 최종 Native lib Clippy29.53초/WASM lib Clippy32.66초 모두 종료0이다.

최종 정상 입력의 Native2쪽 review를 직접 판독했다. 1쪽 두 표 외곽과 소유 단,
LINE02 다음 빈 줄, LINE07→08 단 전환, 2쪽 LINE16~18/AFTER와 표 중복 없음이 맞는다.
PDF custom-paper 높이399pt 대 원문400pt/인쇄 transform 차이로 하단 약0.75pt 차이가
남으며 엔진/이미지를 보정하지 않았다. 글꼴·선 외형 차이도 별도다. source/입력/명령은
`section-scope/absolute-{source,input-test}.sha256`, `absolute-review/native-manifest.json`.
Docker fresh WASM 최종 결과와 판정 준비는 아래에 이어 기록한다.

최종 Legacy7158 대조3건도 통과하여 이번 집중 검사는 **248 PASS / 0 FAIL**이다
(`frame-after-child/absolute-legacy-issue_7158_square_wrap_continuation.log`).
Docker fresh WASM은 종료0/7분35초로 완료되었다. WASM SHA256은
`8a5fe0ef9f7b7042ff6f8abffa8043b4a73747758c5d6247dc73e711856f3fee`다.
`capture-positioned.mjs --absolute --wasm`과 `absolute-sweep.py wasm`을 실행했고,
Native 대비 비수치 차이0, 수치 차이56개/최대1.1368683772161603e-13px다.
`section-scope/absolute-review/backend-comparison.json`, `wasm-manifest.json`에
결과와 실제 source/입력/PDF/WASM 해시·명령을 연결했다. 최종 source/input 해시와
`git diff --check`도 통과했다.

fresh WASM의 `absolute-review/wasm-review-1.png`, `wasm-review-2.png`와
`wasm/overlay/overlay_001.png`를 직접 판독했다. 용지 위 중앙 표/아래 오른쪽 단 표,
의도된 빈 줄, 단 전환과 다음 쪽 LINE16~18/AFTER를 확인했다. 표 중복이나 추가 빈 쪽은
없다. 앞서 기록한 PDF 하단 약0.75pt 및 글꼴·선 외형 차이는 남는다. canonical ink-match
9.2332%/10.06514%는 시각 통과 지표로 사용하지 않았다.

이번 저장 절대 표·동일 폭 Normal 다단 연결은 구현과 집중 검증을 마쳤으며
**메인테이너 시각 판정 통과**다(2026-09-30 다음 작업 승인). 원본 전체 문서의 바탕쪽/본문 도형/첫 쪽 속성 통합,
편집 후 절대 객체 재조판, 절대 표 분할, 전체 workspace CI는 완료 범위에 포함하지 않는다.
Legacy 기본값과90% 이후 Studio V2 검토 조건은 유지했다.

### 저장 바탕쪽 선택·첫 쪽 감추기·본문 TAC 도형의 공통 호스트 연결

직전 절편의 메인테이너 시각 통과/다음 작업 승인에 따라 승인 상태를
`91a8efd21`에 중간 커밋했다. 아래 작업은 그 위 WIP이며 push/PR은 하지 않았다.
기존 도구가 추가한 `.agents/skills/rhwp-*`, `.codex/`는 이 타스크와 분리하여 보존했다.

**근거/범위:** 구역의 첫 쪽 바탕쪽 감추기와 Both/Odd 선택은 본문 점유를 바꾸지 않는다.
TAC 도형은 저장 소유 줄의 상자·기준선을 통해 문단 흐름에 참여한다. 바탕쪽을 본문
높이에 더하거나, 본문 도형을 별도 쪽 pass에서 다시 그리지 않는다. 기존 문서 호스트의
바탕쪽 선택 함수를 `renderer/master_page.rs`로 그대로 분리해 두 호스트가 공유한다.
선택 함수의 기존 extension/carry 로직은 변경하지 않았고 V2는 아직 이를 수용하지 않는다.

**입력 → 독립 기준 → 기대:** `issue7353_host_master_review/create.rs`가 기존2단
대조군에 바탕쪽/본문 도형을 추가한 뒤 한컴이 LineSeg를 새로 생성해 저장했다.
HWP save job `1ee75387-4c43-49a6-b8f7-77afd8b80032`, 동일 HWP의 PDF job
`721cd1ff-9a4c-426c-86e2-7ce4592892fb`, runtime11.0.0.9136/3쪽/전처리 없음이다.
1쪽 바탕쪽 없음,2쪽 BASE MASTER,3쪽 ODD MASTER와2쪽 LEFT/BOX/RIGHT/TAIL의
최종 위치를 판독한다. 독립 HU/정렬 불변식·입력 해시는 fixture README를 따른다.

| 경계 | 공통 결과와 실제 소비 |
| --- | --- |
| 바탕쪽 소유 | `master_page::assign_master_pages_for_section` → 최종 `PaginationResult.pages[*].active_master_page` → `host_section::render_page`가 동일 원문 master를 선택한다. 첫 쪽 감추기는 선택 단계에서 처리하며 paint 단계에서 본문을 밀지 않는다. |
| 바탕쪽 글상자 | `host_master::outlines`는 원문을 수정하지 않은 clone에서 rectangle textbox만 분리한다. 기존 Paper/Page 원점 계산 결과의 rectangle bbox와 원문 pi/ci를 `host_master::complete → shapes::node`가 소비한다. V2가 구성한 동일 글줄의 물리 높이로 여백/세로 정렬/최종 glyph를 결정한다. bbox를 clamp하거나 뒤에서 원점을 보정하지 않는다. |
| 본문 도형 | `host_section::prepare_shapes → host_text::prepare_content`가 기존 mixed-inline/pictures 구성 결과를 사용한다. `ObjectRow`의 확정 상자는 같은 `FlowBlock::Lines`로 페이지 예산을 차지하고 같은 payload가 최종 위치로 이동한다. 도형 높이를 다시 합산하지 않는다. |
| 분할/이월 | 도형 소유 줄은 온전한 line unit이며 기존 `run_section` frame 전환을 사용한다. 별도 도형 continuation이나 표 컷 규칙은 추가하지 않았다. 본문 단 전환/후속 TAIL/END를 실제 최종 노드로 검사한다. |
| 미지원 | 확장 바탕쪽, 바탕쪽 중첩 표·필드 컨트롤/세로쓰기, 본문 floating 도형, 큰 TAC 줄의 Legacy 축소, `hide_empty_line`은 수용하지 않는다. Legacy 표 layout으로 fallback하지 않는다. |

초기 Native 직접 판독에서 바탕쪽 선택은 맞지만 기존 글상자 경로가 가운데 텍스트를
아래로 미는 것을 발견했다. 단순 공통 painter 연결만으로 완료하지 않고 위 V2 텍스트
구성 결과로 연결했다. 독립 기대값은 원문 패딩/3000HU 외곽/1200HU 글줄의 가운데
정렬 불변식이며, 새 계약은 외곽뿐 아니라 실제 텍스트 y도 검사한다. 기존 문서 호스트의
글상자 배치는 변경하지 않았고 selector 공유 회귀만 별도로 검사한다.

2400HU 큰 도형/1800HU 피치 초안은 `shared text paint changes stored metrics`로
거부된다. `tall-unsupported.hwp`와 `master-tall-draft/`에 보존하고 명시적 거부를
정식 계약으로 보호했다.1200HU 초안은 한컴의 current_height1202/common.height1200
차이로 textbox 상자에 맞지 않아 거부됐다(`master-textbox-draft/`). 최종1600HU는
작성 단계에서 다시 지정하여 정상 저장한 **다른 대조군**이다. 초안 문제의 해결/일치로
바꿔 보고하지 않으며, 저장 LineSeg 수용 조건도 완화하지 않았다.

`issue_7353_hosted_master`7건은 정상 HWP/HWPX 왕복/96·192dpi, source 불변,
실제 바탕쪽 외곽/텍스트 y, TAC 도형/앞뒤 텍스트/후속 문단, 첫 쪽 표시/바탕쪽 제거,
도형 단독 줄, 미지원 명시 거부, 기존 문서 호스트의 Both/Odd 선택을 검사한다.
합성 변형은 별도 한컴 피델리티 주장으로 승격하지 않는다.

직전 승인 binary의 동일 최종 HWP는 section decoration 미지원으로 거부됐다
(`section-scope/master-before.log`). 이는 미지원 경로의 전후 증거이며 기존 시각 회귀가
아니다. 최종 source 신규7건 PASS(`frame-after-child/master-final-seven-*`), 기존248건과
바탕쪽 selector #6323의2건 PASS(`section-scope/master-focused.log`)로 **257 PASS/0 FAIL**다.
최초6건 실행과 최종 build 완료가 겹쳐 신규7건은 최종 라이브러리로 다시 검증했다.
전체 workspace CI/통합 target Clippy를 실행한 결과는 아니다.
Native build23.83초, Native lib Clippy33.10초, WASM lib Clippy32.09초 모두 종료0이다.

최종 Native3쪽을 재캡처하고 canonical sweep을 실행했다. 비교 명령은
`capture-positioned.mjs --master`, `python3 master-sweep.py native`이며 prefix는
`output/7353/r19/section-scope/`다. source/입력/PDF/명령은 `master-source.sha256`,
`master-input-test.sha256`, `master-review/native-manifest.json`에 고정했다.
2/3쪽 직접 판독에서 바탕쪽 텍스트 가운데 정렬과 소유 위치, 본문 도형/후속 줄이 맞는다.
글꼴 외형·폭 및 custom-paper PDF 인쇄 transform의 하단 약0.75pt 차이는 남으며
이미지/출력 좌표를 보정하지 않았다. fresh WASM 결과는 아래에 이어 기록한다.

원본 #7008 전체는 여전히 완료가 아니다. 원본의 `hide_empty_line`, 바탕쪽 쪽번호
필드, 큰/특수 TAC 도형 및 다단 Square 결합 등의 수용/시각 근거가 남아 있다.
이번 결과를 R3/R5 전체 완료나 Studio V2 전환으로 세지 않는다.

Docker fresh WASM 빌드가 종료0/7분33초로 완료됐다. 산출 SHA256은
`860b988c50c25bcf5e109bf4d383e2022943360bbf17bf714f0c51ab002c8713`이다.
`capture-positioned.mjs --master --wasm`, `python3 master-sweep.py wasm`을 실행했다.
`master-review/backend-comparison.json`은 비수치 차이0, 수치 차이85개/최대
1.1368683772161603e-13px다. 최종 source/input 해시 검증과 fmt/diff check도 통과했다.

`master-review/wasm-review-1.png`, `wasm-review-2.png`, `wasm-review-3.png` 및
`wasm/overlay/overlay_002.png`를 직접 판독했다. 첫 쪽 바탕쪽 감추기·두 절대 표·빈 줄,
2쪽 BASE MASTER와 LEFT/BOX/RIGHT/TAIL,3쪽 ODD MASTER/END가 보존된다.
바탕쪽 글상자의 가운데 정렬 개선도 확인했다. 글꼴 외형/인쇄 transform 차이는 남는다.
canonical ink-match7.36014/13.8512/12.53682%를 시각 통과 지표로 사용하지 않았다.
근거는 `master-review/wasm-manifest.json`에 연결했다. 이 절편의 구현/집중 검증/판정 자료
준비를 완료했으며 **메인테이너 시각 판정 통과**다(2026-09-30 다음 작업 승인).
원본 전체 지원/전체 CI는 위 잔여 범위다.

### 바탕쪽 글상자의 자동 쪽번호 연결

직전 바탕쪽·본문 도형 절편은 메인테이너 시각 통과를 반영해 `67bc0eff5`에
커밋했다. 이번 변경은 그 위 WIP이며 Legacy 기본값·Studio는 변경하지 않았다.

**규칙과 근거:** 바탕쪽 자동 쪽번호는 저장된 AutoNumber.number/assigned_number가
아닌 실제 배치된 쪽의 번호를 표시해야 한다. 원본 #7008의 바탕쪽 글상자에는
Page 자동번호와 문단 끝 글자모양 참조(start8/char_count9)가 있다.
문단 끝 서식은 offset0의 번호 글립 서식과 구별한다. 일반 번호·장식·위첨자나
글상자 안 표까지 수용하는 변경은 아니다.

**입력과 독립 기준:** `tests/fixtures/issue7353_host_master_field_review/`의 정상
한컴 저장 HWP와 같은 입력의 PDF를 사용했다. 생성 job·해시·기대 좌표는 폴더 README다.
1쪽 숨김,2쪽2,3쪽3을 읽을 수 있는 3쪽 제어 문서다. 최초 초안/재저장/최종 저장 모두
바탕쪽 LineSeg가 비어 있었다. 재저장으로 해결됐다고 판단하지 않았으며, 기존 일반
글상자의 공통 스타일 기반 줄 구성 경로로 번호 필드도 구성했다. 셀은 저장 줄을
요구하는 기존 수용 조건을 유지하며 이를 별도 회귀 검사로 보호한다.

| 생산과 소비 | 코드와 계약 |
| --- | --- |
| 번호 소유 | `host_section::render_page`의 확정 `page.page_number` → `host_master::complete` → `shapes::node_with_page_number`. 원문 번호를 대신 쓰지 않는다. |
| 줄 구성 | `TextComposer::compose_page_field`는 저장 줄이 있으면 보존하고 없으면 공통 `layout_paragraph_in_physical_frame` 결과를 사용한다. 필드 치환도 기존 LayoutEngine API를 공유한다. |
| 예약과 실제 출력 | `PageField::render`가 치환 전후 줄 상자 동일성과 표시 숫자의 가로 수용을 확인한다. 같은 최종 줄의 물리 높이로 글상자 세로 정렬과 paint를 수행한다. 독립적인 줄 높이 재계산이나 clamp는 없다. |
| 비적용 경로 | 일반 도형에는 쪽번호를 전달하지 않으며 페이지 미확정 필드는 거부한다. 표 셀은 `qualify`의 저장1줄 조건을 유지한다. 분할 컷·예약 예산·continuation은 변경하지 않았다. 바탕쪽은 본문 흐름을 전진시키지 않는다. |

정식 신규6건은 HWP/HWPX 왕복·96/192dpi·역순/반복 출력, 두 자리 쪽번호, 번호 캐시91/92 무시,
원문/본문 불변,1200HU 글자 크기와 원문 패딩에 따른 가운데 정렬, 끝 글자모양과
중간 글자모양 구별, 번호 서식 거부, 셀 저장 줄 조건을 검사한다. 변형은 합성 계약이다.
직전 binary의 동일 최종 입력은 `master-page content requires host admission`으로
거부되고(`master-field-before-final-input.log`), 최종 구현은3쪽을 출력한다.
이는 미지원 경로 연결의 전후 증거이며 기존 Legacy 시각 회귀 수정으로 확대하지 않는다.

검증 prefix는 `output/7353/r19/section-scope/master-field-`다. 관련 호스트·표·어울림·
이어받기·쪽번호 테스트 총 **270 PASS/0 FAIL**이다. 실행은 기존
`frame-after-child/run-policies.sh`를 사용했고, `master-field-tests.log`,
`master-field-regression.log`, `frame-after-child/master-field-final-six-*`에 결과가 있다.
초기 테스트 컴파일 오류와 기대 좌표의 원문 패딩 누락은 수정 후 재실행했으며
이를 제품 결함의 수정 전 재현으로 세지 않는다. Native build, Native/WASM lib Clippy,
fmt/diff check를 수행했다. 전체 workspace CI 또는 integration-target Clippy 결과는 아니다.

Native 명령은 `capture-master-field.mjs`, `python3 master-field-sweep.py native`다.
3쪽 review와 overlay를 직접 판독해 첫 쪽 숨김·2/3쪽 번호·글상자 정렬·본문 보존을
확인했다. 글꼴 외형/미세 폭과 custom-paper 인쇄 transform 차이는 남는다.
`master-field-source.sha256`, `master-field-final-manifest.sha256`과
`master-field-review/native-manifest.json`에 source/input/PDF/명령을 연결했다.
Docker fresh WASM 빌드는 종료0/7분36초로 완료했다. 산출 SHA256은
`2c411064d685fd144b81d905ad50c73d0deae0f0b5f976eb35d63da284f58870`이다.
`capture-master-field.mjs --wasm`, `python3 master-field-sweep.py wasm`을 실행했으며
Native 대비 비수치 차이0, 수치 차이87개/최대1.1368683772161603e-13px다.
`master-field-review/wasm-manifest.json`에 같은 source/input/PDF/산출을 고정했다.
WASM review1~3쪽과 canonical2쪽 overlay를 직접 판독해 첫 쪽 숨김·실제2/3 번호·
가운데 정렬·본문 보존을 확인했다. 글꼴/미세 인쇄 차이는 Native와 동일하게 남으며
자동 픽셀 점수를 통과 근거로 사용하지 않았다. 최종 source·테스트·입력 해시,
fmt/diff check도 통과했다. **메인테이너 시각 판정 통과**다(2026-09-30 다음 작업 승인).
원본 #7008의 hide_empty_line·큰/특수 TAC 도형·다단 Square 결합과 전체 호스트 수용은
남아 있다. 이번 절편을 전체 지원이나 R5 완료로 세지 않는다.

### 단과 쪽 경계의 빈 줄 감추기

바탕쪽 쪽번호의 메인테이너 시각 통과를 `8b101340a`에 보존하고 다음 승인 범위로
진행했다. Legacy 기본값·Studio 경로는 그대로다. 이번 WIP는 구역의
`hide_empty_line`을 공통 `run_section`의 V2 호스트에 연결한다.

독립 근거는 [정상 한컴 ON/OFF 대조군](../../tests/fixtures/issue7353_hide_empty_review/README.md)이다.
페이지 끝 일곱 줄 뒤 빈 문단 세 개를 둔 입력과 두 단 입력을 각각 ON/OFF로 저장·출력했다.
저장 LineSeg와 PDF에서 **각 단의 끝을 넘는 두 빈 문단만 감추고 세 번째는 다음 단/쪽에서
점유**하는 동작을 확인했다. Legacy의 물리 페이지 단위 카운터를 그대로 복사하지 않았다.

| 생산과 소비 경로 | 실제 처리와 경계 |
| --- | --- |
| 입력 → 호스트 | SectionDef.hide_empty_line/bit19 → `host_section::from_document` → `typeset_hosted_section` → 기존 `run_section` 상태. |
| 측정 → 배치 | 기존 `HostedParagraphPlan::fit`의 공통 FlowCursor 결과로 수용한다. 실패한 동일 줄만 감춤 후보가 된다. 별도 높이 추정은 없다. |
| 예산 음수 | 직전 줄 뒤 간격 등으로 cursor가 가용 높이를 넘었으면 새 줄을 배치할 수 없으므로 같은 감춤 규칙을 적용한다. 마지막 줄 자체가 fit하면 간격만으로 숨기지 않는다. |
| 감춤 → 최종 출력 | `is_unconsumed_empty_line`은 미소비·단일 줄·진짜 빈 문자열·제어/필드 없음만 허용한다. `hide_overflowing_empty`는 `(page,column)`별 두 개 제한과 source 숨김 집합만 갱신한다. Legacy FullParagraph나 0높이 paint 노드를 삽입하지 않는다. |
| 비적용 | 본문 중간/명시적 쪽 나누기 뒤 빈 줄, 필드/표 소유 줄, 셀 내부 빈 줄은 이 규칙으로 삭제하지 않는다. 표의 컷·요구 높이·예약/분할·continuation은 변경하지 않았다. |

정식 `issue_7353_host_hide_empty`7건은 단일/두 단 ON/OFF,96/192dpi 최종 좌표,
source 불변/숨김 소유 집합, 저장 줄 없는 재조판, 정확한 fit/뒤 간격 초과,
명시적 쪽 나누기·중간 빈 줄, 보이지 않는 쪽번호 제어 소유 문단, 분할 표 소유 줄과
셀 내용 불변을 검사한다. PDF 생성은 정상 한컴 경로이며, 편집 변형은 합성 계약으로 구별했다.

증적 prefix는 `output/7353/r19/hide-empty/`다. 승인 상태의 Native 라이브러리에
신규 테스트를 적용하면 section decoration 미지원으로 실패한다(`before.log`와
`frame-after-child/hide-empty-before-issue_7353_host_hide_empty.log`). 이는 미지원 연결의
전후 증거이며 기존 Legacy 시각 회귀 수정 주장이 아니다. 최종 신규7건 PASS(`seven.log`),
기존270건 PASS(`regression.log`)로 **277 PASS/0 FAIL**다. 전체 CI 결과가 아니다.
Native build24.97초, Native lib Clippy34.14초, WASM lib Clippy31.21초와 fmt/diff check가
통과했다. source/input/test 해시는 `source.sha256`, `input-test.sha256`이다.

Native는 `shared-driver-export.rs`를 최종 라이브러리로 컴파일하여 실제 호스트를 실행했고,
`capture.mjs --variant=on|off|columns-on|columns-off`, `python3 sweep.py native`로 비교했다.
2쪽 AFTER의 ON/OFF 시작 차이와 두 단의 AFTER/NEXT 위치를 직접 판독했다.
글꼴 외형/폭·custom-paper 인쇄 transform의 미세 차이는 남으며 좌표 보정은 하지 않았다.
새 WASM 결과와 메인테이너 판정은 아래에 이어 기록한다.

승인 상태의 `master-field-export`로 OFF 대조군 두 개를 별도로 실행했다.
`off-before.json`, `columns-off-before.json`은 변경 후 Native의 SVG/render tree
전체 JSON과 byte 동일하다. binary 해시는 `exporters.sha256`에 고정했다.
이는 OFF 무회귀의 추가 증거이며 ON의 한컴 대조를 대신하지 않는다.

**남은 범위:** 한컴이 정상 저장한 HWPX counterpart 두 개와 rhwp HWPX 왕복은
별도의 `stored text requires intact single-segment rows` 수용 단계에서 거부됐다.
직접 작성한 저장 줄 없는 HWPX에서는 공통 paragraph_layout panic도 관측했다.
이들 입력과 실패 근거는 보존하며, HWP 통과로 HWPX 완료를 주장하거나 수용 조건을
완화하지 않았다. 원본 #7008은 다음 `stored TAC baseline envelope mismatch`에서
거부된다(`original-admission.log`). 큰/특수 TAC 도형·다단 Square·전체 호스트 수용과
전체 CI는 여전히 남아 있다. 신규 이슈를 임의로 등록하지 않았다.

Docker fresh WASM 빌드는 종료0/7분36초로 완료했다. WASM SHA256은
`f56194edc8f64a4921a8e5d0df91cdcdc5e9f2fd1a06cf670e059f41240facf8`이다.
`capture.mjs --variant=on|off|columns-on|columns-off --wasm`과
`python3 sweep.py wasm`으로 같은 네 입력의8쪽을 생성했다. 각 대조군의
`*-review/wasm-manifest.json`은 source/input/PDF/명령/WASM 해시를 고정한다.
`backend-comparison.json`의 Native 대비 비수치 차이는 전부0이며,
수치 차이는21/21/34/34개, 최대2.842170943040401e-14px다.

WASM의 단일 단 ON/OFF2쪽, 두 단 ON/OFF1·2쪽 review와 Native/WASM의
단일 단 ON2쪽 canonical standalone overlay를 직접 판독했다. ON은 세 번째 빈 줄
하나를 남긴 위치, OFF는 세 빈 줄을 모두 남긴 위치이며 다음 내용의 소유·순서도 맞는다.
글꼴 외형/폭·미세 인쇄 transform 차이는 남는다. 자동 ink-match를 판정 근거로
삼지 않았다. **이번 절편의 메인테이너 시각 판정 통과**다(2026-09-30 다음 작업 승인). 대표 자료는
`on-review/wasm-review-2.png`, `off-review/wasm-review-2.png`,
`columns-on-review/wasm-review-1.png`다. 문서 작성 스킬의 출처·검증 범위 구분을
적용해 정상 저장 HWP와 미수용 HWPX를 분리 기록했다.

### 작은 TAC 선 도형의 글줄 점유와 배치

빈 줄 감추기의 메인테이너 통과를 `b643b7bbf`에 커밋했다. 다음 원본 수용 오류인
`stored TAC baseline envelope mismatch`를 추적하니 #7008의 p4는 큰 표가 아니라
높이4 HU의 TAC 선이었다. 저장 줄은 글자 모양11pt에 따른 높이1100/기준선935 HU다.
도형만 있는 줄의 객체 높이와 글줄 높이가 항상 같다는 가정을 제거했다.

독립 근거는 [정상 한컴 저장 대조군](../../tests/fixtures/issue7353_small_inline_review/README.md)이다.
저장 줄을 작성하지 않은 HWPX를 한컴에서 HWP로 저장하고 그 HWP의 PDF를 얻었다.
11pt/20pt, 작은/큰 선, 위/아래 바깥여백900/200 HU의 여덟 사례를 한 쪽에서 비교한다.
큰 객체를 폰트 높이로 축소하지 않으며 작은 객체의 나머지 공간은 기준선 양쪽에 배분한다.

| 값의 생산과 소비 | 적용 경로와 최종 결과 |
| --- | --- |
| 입력 → 공통 줄 | `pictures::character_heights`가 저장 줄의 source 범위에 해당하는 글자 모양을 읽고 `tac::object_rows`가 기존 객체 band에 최소 글줄 높이를 합성한다. 저장 높이 불일치 검사는 유지한다. |
| 측정 → 예약 | 동일 `StoredTacRow`의 높이가 `ObjectRow`의 물리 점유 및 `ParagraphEnd`로 넘어가 본문/셀의 fit과 뒤 문단 전진에 사용된다. |
| 실제 배치 | 동일 행의 객체 Rect를 `shapes::line_node`가 사용한다. 원본 component 끝점을 현재 상자에 한 번만 투영한다. `text::translate`는 상자뿐 아니라 backend가 읽는 Line의 절대 끝점도 함께 옮긴다. |
| 비적용 | 표 TAC의 기존 경로에는 character strut를 추가하지 않았다. 컷·rowspan·continuation 알고리즘, Legacy 기본값, Studio 선택은 바꾸지 않았다. 회전/연결선 등 검증하지 않은 선은 명시적으로 거부한다. |

신규 정식 테스트의 정상 대조군은 변경 전 `baseline envelope mismatch`로 실패했다.
최종 신규5건은 실제 선/줄/뒤 문단 좌표, PDF 경로 좌표,96/192dpi, source 불변,
높이 cache 변조 거부, 회전 거부, 같은 줄 복수 객체, 셀의1100 HU 경계 fit와
비영점 원점의 끝점 이동 및 종료를 검사한다. 셀 예산은1099 HU에서 실패하고1100 HU에서
수용한다. 합성 입력을 정상 저장본 시각 증거로 확대하지 않았다.

증적 prefix는 `output/7353/r19/tac-baseline/`이다. 기존317건과 신규5건이 통과해
**322 PASS/0 FAIL**다(`regression.log`, `frame-after-child/small-inline-final-issue_7353_small_inline.log`).
변경 전 실패는 `frame-after-child/small-inline-before-issue_7353_small_inline.log`다.
Native build28.45초, Native lib Clippy30.25초, WASM lib Clippy32.43초와 fmt/diff check가
통과했다. 전체 workspace CI 결과는 아니다. `source.sha256`, `input-test.sha256`에
작업 소스와 입력/테스트를 고정한다.

`shared-driver-export.rs`를 새 라이브러리로 컴파일하고 `capture.mjs`, `sweep.py native`로
Native compare·standalone overlay·review를 만들었다. 여덟 선과 AFTER의 순서/위치,
글줄 높이 보존을 직접 판독했다. 글꼴 차이와 PDF의 y축1.0015 인쇄 배율/12 HU 격자에
따른 미세 차이는 남으며, PNG 좌표를 보정하거나 자동 점수로 통과를 선언하지 않았다.

원본 전체 실행은 이 TAC 높이 거부를 넘었지만 다음 p5의 일반 본문에서
`text preview stored rows or controls`로 거부된다(`original-admission.log`, `next-probe.log`).
p0의 절대 표/다단 흐름 등 다른 미지원도 남으므로 원본 전체 수용이나 R5 완료가 아니다.
새 Docker WASM 빌드 및 동일 입력의 시각 증적은 아래에 이어 기록한다.

Docker fresh WASM은 종료0/7분31초로 완료했다. WASM SHA256은
`6b90185deddbc0d811cf05add3bb3b1d87f945a4307f4f1f9bf904a562245f47`이다.
`node capture.mjs --wasm`과 `sweep.py wasm`으로 실제 브라우저의 V2 호스트 출력을
생성했다. Native 대비 비수치 차이는0, 수치 차이는77개/최대1.1368683772161603e-13px다.
`small-review/wasm-manifest.json`과 `backend-comparison.json`에 근거를 고정했다.

Native/fresh WASM의 review 및 canonical standalone overlay를 직접 열어 여덟 선과
뒤 문단의 위치·줄 점유를 확인했다. 동일한 글꼴·미세 인쇄 차이는 남는다.
대표 판정 자료는 `small-review/wasm-review-1.png`와
`small-review/wasm/overlay/overlay_001.png`, 샘플은
`tests/fixtures/issue7353_small_inline_review/expanded.hwp`다.
**이번 절편의 메인테이너 시각 판정 통과**다(2026-09-30 다음 작업 승인). 문서 작성 스킬의 근거/범위 구분에 따라
정상 저장 시각 대조와 합성 셀·복수 객체 계약을 분리 기록했다.

### 다음 본문 수용 경계 — 영역 태그 조사

작은 TAC 선 도형 절편을 `ed02ae095`에 커밋했다. 다음 원본 #7008의 s0/p5
`[1~3] 다음 글을 읽고 물음에 답하시오.`는 `text.rs::compose_shared`의
`!para.range_tags.is_empty()`에서 거부된다. NBSP·들여쓰기·글줄 높이가 이번 거부의
원인은 아니다. 원본 태그는 `[0,1)`, `[4,6)`의 `0x01000007`이다.

`output/7353/r19/range-tags/isolate.rs`로 원본 p5/p14/p26을 각각 셀의 문단으로
조판했다. 태그가 있으면 셋 모두 같은 수용 오류이며, 진단용 복제에서 태그만 제거하면
저장 LineSeg를 그대로 사용해 수용된다(`isolate.log`). 이 제거는 원인 분리용이며,
원본·production 코드·테스트 계약에는 반영하지 않았다.

공개 HWP5 사양4.3.5/표63은 상위8비트가 종류, 하위24비트가 종류별 데이터라는
구조만 설명한다. 이번 종류1을 임의로 맞춤법 정보라고 명명하거나 모든 영역 태그를
인쇄 비대상으로 간주할 근거는 아직 없다. 공통 paragraph painter의 종류2는 실제
형광펜 Rectangle을 생성하므로 영역 태그 전체를 통과시키는 변경도 하지 않는다.

독립 출력 비교를 위해 원본 p5의 텍스트/글자모양/영역 태그를 보존하고 저장 LineSeg를
비운 추출본 `range-tags/tagged.hwp`와 태그만 제거한 `plain.hwp`를 만들었다.
구역은 단일 단,42000×30000HU로 바꾸고 후속 문단 `AFTER / 다음 문단`을 추가했다.
두 입력은 원본 전체 증거가 아니라 진단용 파생본이다. `probe.rs`에 생성 절차를 보존한다.
원격 한컴2020 profile의 HWPX 변환과 양쪽 PDF를 요청했으나 현재 결과 대기 중이다.

| 변환 | job ID |
| --- | --- |
| tagged → HWPX | `1b626104-2ccc-4cc8-83d6-f2266422879f` |
| tagged → PDF | `9d32b3d4-97b6-4987-8974-1b4da7031820` |
| plain → PDF | `06f8016e-08a4-4b21-8210-998f0fd6239b` |

결과 확인 전에는 영역 태그의 수용 조건을 완화하지 않는다. 이번 새 경계의 구현·
회귀 통과·시각 통과는 아직 주장하지 않는다. 다음은 변환 terminal 상태와 실제 출력의
차이를 확인하고, 확인된 범위만 공통 문단 구성 경로에 연결하는 것이다.
마지막 확인에서 HWPX job은367초 경과/`running`, 두 PDF job은`queued`다.
세 status JSON을 같은 증적 폴더에 저장했다. 재개 시 기존 job을 조회하며 결과를 받기
위해 같은 입력을 중복 제출하지 않는다. 승인된 기존 WASM과 조판 소스는 유지한다.

**검증 입력 무효 — 2026-09-30 메인테이너 확인:** 후속 진단 추출본
`output/7353/r19/range-tags/column-tagged.hwp`는 한컴편집기에서 **손상 파일**로
판정됐다. 이 파일은 정상 한컴 출력 비교·회귀 기대값·영역 태그 수용 조건의 근거에서
제외한다. 앞선 `tagged.hwp`와 `plain.hwp`도 정상 열림이 입증되지 않았으므로 유효한
대조군으로 사용하지 않는다. 실패 파일과 생성기는 원인 조사 자료로만 보존한다.

후속본은 `with-column.rs`가 앞선 `tagged.hwp`를 다시 파싱한 뒤 첫 문단에
ColumnDef를 넣고 char_offsets/char_shapes/range_tags/char_count를 수동 이동시킨
파생본이다. 별도 첫 문단을 추가했다는 생성기 첫 주석과 달리 실제 코드는 기존 첫 문단을
수정했다. 이 생성 과정의 유효성을 한컴 열림으로 확인하기 전에 검증 후보로 제시한
것은 에이전트의 잘못이다. 단 정의 누락이 손상의 원인이라는 가설은 입증되지 않았다.

기존 HWPX job `1b626104-2ccc-4cc8-83d6-f2266422879f`와 tagged PDF job
`9d32b3d4-97b6-4987-8974-1b4da7031820`은 각각600초 timeout으로 끝났다.
후속본 PDF job `daf82d01-ea55-4756-a4b4-5e57067740db`는 timeout60초로
이미 제출되어 있으나, 결과가 생성되더라도 정상 입력 증거로 사용하지 않는다.
따라서 이전의 단순 변환 지연이라는 설명을 수정한다. 정확한 손상 필드·원인은 아직
미확정이며, timeout 자체를 영역 태그의 인쇄 의미나 원본 문서의 결함으로 해석하지 않는다.

원본의 V2 range_tags 거부 재현과 추출본의 손상은 서로 다른 사실이다. 원본과 기존
기준 PDF, 승인된 코드/WASM은 변경하지 않았으며 손상 추출본으로 구현 조건을
완화하지 않는다. 이 추출본 변형·재제출은 중단한다.

### 원본 한컴 변환으로 확인한 글자 스타일 영역 — 2026-09-30

손상 추출본 대신 **변경하지 않은 원본** `samples/21_언어_기출_편집가능본.hwp`를
한컴 MCP로 HWPX 변환했다. 기존 대기열 종료 후 실제 변환은11초/성공이며,
job은 `5287c9a0-3d66-4e3b-98b4-d51bfd35c131`, profile2020의 실제 엔진은
11.0.0.9136, input_preprocess는none이다. 원본 SHA256은
`905454045ca2e236839a7cab59750678116d08af3db31dbf846819af355b8d15`, 결과
`output/7353/r19/range-tags/original-7008.hwpx`의 SHA256은
`d0c4238c303ce0715244ddf82d78b5d91032da7166ddeab0e401122aa679bc59`다.

독립 출력의 `Contents/section0.xml`에서 HWP 종류1은 `hp:t/@charStyleIDRef`로
변환된다. p5의 대괄호는7, p14의 문제 번호는1, p26의 “않은”은20이다.
`Contents/header.xml`의 CHAR 스타일은 각각 `[~]대괄호만`/charPr12,
`문제번호`/charPr9, `문제 밑줄`/charPr21이다. 실제 글자모양은 각 run의
`charPrIDRef`에 별도로 존재한다. 따라서 종류1을 맞춤법 정보라고 추정하지 않고,
**이름 있는 글자 스타일 연결을 IR에 보존하되 유효 run의 글자모양을 중복 적용하지 않는다.**

`text.rs::compose_shared`의 영역 태그 일괄 거부를 유효 범위의 종류1 수용으로 바꿨다.
종류2 형광펜·알 수 없는 종류·역전/문단 밖 범위는 계속 명시적으로 거부한다.
원본 태그·글자모양·LineSeg를 삭제하거나 수정하지 않는다. 공통 구성 결과의
`compose_paragraph → layout_composed_paragraph_in_frame → ParagraphItem::Lines`
경로와 조각의 최종 노드 배치를 유지하며, 새 높이/원점 보정은 없다.

정식 `tests/cases/issue_7353_character_style_ranges.rs`는 원본 p5/p14/p26의
저장 줄을 **메모리 내 셀 어댑터**에서 조판한다. HWP 추출/재저장본이 아니며 원본 전체
호스트 검증도 아니다. 96/192dpi의 최종 TextLine y/높이, 원문 텍스트와 실제 run의
글자모양 ID를 검사한다. 높이1100/1300HU는 원본 저장값과 한컴 HWPX의 동일 LineSeg가
근거다. 별도 fresh 구성 대조와 형광펜/미지정 종류/잘못된 범위 거부를 포함한다.

- 수정 전: 새3건 중2FAIL/1PASS. 두 양성 사례가 기존 명시적 수용 거부에서 실패했다.
- 수정 후: 새3건 모두PASS, 관련 기존28suite/322건 모두PASS.
- `cargo build --locked -p rhwp --lib`, Native 및 wasm32 `--lib` Clippy/`-D warnings` 통과.
- 로그: `range-tags/build.log`, `clippy-native.log`, `clippy-wasm.log` 및
  `frame-after-child/style-ranges-{before,after,regression}-*.log`.

원본 전체 HostedSectionSession 실행은 기존 p5 태그 거부를 넘었지만
`text preview paragraph decoration or keep`에서 여전히 거부된다.
`range-tags/next-gate.log`의 원본 prefix 진단으로 다음 경계는s0/p6까지 포함할 때임을
확인했다. 문단 장식 조건의 실제 소유/소비 경로를 다음으로 조사하며, 이 오류를 없애려고
borderFill·빈 문단을 삭제하지 않는다. `original-admission-after.log`에 전체 실패를 보존했다.

이번 결과는 **글자 스타일 영역 수용/계약 검증 완료**, **원본 종단 및 시각 검증 미완료**다.
새 코드의 fresh WASM 캡처는 아직 없으며 기존 승인 WASM을 새 증거로 재사용하지 않는다.
구현계획5.1에 따라 의미 있는 원본 종단 후보가 준비되면 Docker fresh WASM 및 직접
Visual Sweep을 실행한다. 손상 입력이나 단일 문단 계약 성공으로 시각 승인을 요청하지 않는다.

### 본문 연결 테두리와 빈 시작 문단 — 2026-09-30

작업지시자의 다음 작업 승인으로 원본s0/p6을 조사했다. p6은 컨트롤 없는 빈 문단이며
borderFill7/`border_connect=true`, 저장 높이1100HU·간격-56HU·전진1044HU다.
원본 한컴 변환 HWPX도 동일 값을 보존한다. p7~p12의 지문과 테두리가 연결된다.
기존 대응 PDF1쪽은 왼쪽 단의 테두리 상단415.379pt, 오른쪽 단의 마지막 하단592.301pt를
그리며 중간 문단마다 수평선을 만들지 않는다. 빈 p6을 제거하는 것은 지문 시작 간격과
외곽선의 의미를 동시에 바꾸므로 허용하지 않는다.

이번 구현은 **본문 일반 텍스트의 zero-inset solid/None 문단 테두리**다. 배경 채움,
문단 테두리 간격, 복합 펜/특수 효과와 표 셀/객체 혼합의 문단 장식은 여전히 미지원이다.
`host_border.rs`에서 원본 효과와 resolved 표면을 각각 검증한다. 원본 테두리를 삭제하거나
style ID를0으로 바꾸지 않는다. 네 변의 실제 펜이 같고 앞 문단의 연결 속성이 켜진 경우에만
연결하며, 위쪽 펜 하나가 같다는 이유로 다른 좌우 테두리를 합치지 않는다.

소비 경로는 `TextComposer::compose_host → 기존 공통 TextLine/ParagraphEnd →
HostedParagraphPlan::fit → HostedParagraphFragment → LayoutEngine의 HostedParagraph
분기 → host_border::append`다. 줄 구성·빈 줄 높이·advance는 기존 확정 결과를 유지한다.
테두리 x/폭은 호스트 frame, y/하단은 수용 조각의 점유와 advance에서 얻는다.
Legacy 문단 테두리의2px 최소 확장·30px 간격 병합·용지 clamp는 호출하지 않는다.
연속 그룹이나 한 문단 자체의 계속 조각은 중간 상·하변을 생략하고 좌우변을 이어 그린다.

`tests/cases/issue_7353_host_paragraph_border.rs`에 최종 트리 계약6건을 추가했다.
원본 p6의 줄과 메모리 내 합성 프레임을 사용하며, HWP로 직렬화한 시각 샘플이 아니다.
첫 진단에서 뒤 문단의 스타일만 바꾸고 저장 가로 구간을 함께 구성하지 않은 테스트 입력
오류를 발견해 고쳤다. 이 입력 오류 때문에 production의 저장 폭 수용 조건을 완화하지 않았다.

- 초기 구현 전 검사: 양성3건이 기존 문단 장식 거부로FAIL, 음성1건PASS.
- 최종6건PASS: 빈 줄1100HU/전진1044HU(96/192dpi), 연결/비연결 외곽,
  서로 다른 측면 펜 분리, 페이지 경계 및 원본10줄 문단의 계속 조각에서 내용·줄 수 보존,
  미지원 효과/간격 거부. 추가2건의 구현 전 실행은 하지 않았다.
- 직전 글자 스타일 영역3건과 기존28suite/322건PASS. 전체 CI가 아닌 집중 검증이다.
- Native lib build 및 Native/wasm32 lib Clippy `-D warnings` 통과.
- 증적: `range-tags/host-border-{build,clippy-native,clippy-wasm,admission,next-gate}.log`,
  `frame-after-child/host-border-{before,after,final,regression}-*.log`.

원본 전체 실행의 다음 거부는 **s0/p17**로 이동했다. p6부터의 테두리 문단은 준비 단계를
통과했지만 문서 전체 paint 성공을 뜻하지 않는다. p17은 첫 저장 줄의 `vertsize=1128`과
`textheight=1156`이 서로 다르며 정상 한컴 HWPX도 이를 보존한다. 현재 `stored_text::localize`의
두 값 동일성 조건이 거부하므로 다음 조사 대상이다. 값을 같게 덮어써 우회하지 않는다.

이번 연결 테두리는 **구현·집중 계약 완료 / 원본 전체 Native 및 fresh WASM 시각 검증 미완료**다.
직전 승인 WASM은 교체하지 않았다. 기본 엔진 전환·remote push·PR 생성은 하지 않았다.

### 저장 줄의 텍스트 점유 높이 — 2026-09-30

작업지시자의 “다음 거부 지점까지” 승인 범위다. 기준 HEAD는
`ed02ae095f2b1eb0f290e422167c81bd45c6b835`이며 아래 검증은 그 위의 미커밋 변경을 포함한다.
원본 HWP p17과 정상 한컴 변환 HWPX 모두 첫 줄 높이1128HU, 텍스트 높이1156HU,
baseline935HU, 간격752HU를 가진다. 다음 줄의 저장 원점 차이1908HU는1156+752와 같다.
HWP 사양 표62도 줄 높이와 텍스트 부분 높이를 별도 필드로 정의한다. 입력값을 동일하게
고치지 않고 공통 구성기의 기존 `stored_line_box_height=max(line_height,text_height)`를
V2 저장 줄 수용·최종 paint 검증·프레임 이어받기에서도 소비하도록 연결했다.

소비 경로는 `composer::stored_line_box_height → compose_paragraph의 ComposedLine →
layout_composed_paragraph_in_frame의 실제 TextLine → stored_text::validate_paint →
ParagraphItem::Lines의 height/advance → TableCursor fit → TextFragment::append_to`다.
줄 사이 원점은 저장 vpos를 유지한다. 모든 줄의 전진을 max+spacing으로 재계산하지 않는다.
마지막 줄의 문단 끝과 프레임 reset을 연속 좌표로 옮기는 부분은 같은 점유 높이를 사용한다.
측정 전 최종 노드 검증이 남아 있어 후단이 저장 메트릭을 바꾸면 계속 명시적으로 거부한다.

높이 조건을 넘긴 원본은 **같은 p17의 글자 테두리 paint**에서 거부된다.
`paragraph_layout.rs`의 글자 테두리 생성은 TextLine 자식에 TextRun과 선 노드를 함께 넣지만
V2 일반 텍스트 계약은 TextRun만 받는다. 종전의 포괄적인 저장 줄 오류를
`stored text paint has non-text-run children`로 구분했다. 선을 삭제해 원본을 통과시키지 않는다.
원본 전체 실행 및 prefix 진단 모두 이 경계를 확인했으며, 이번 승인 범위의 정지점으로 삼는다.

정식 `tests/cases/issue_7353_stored_text_extent.rs`의 양성 사례는 **resolved 글자 테두리만
제외한 메모리 내 기하 계약 어댑터**다. 원본 Paragraph/LineSeg는 그대로이며 시각 기준 입력이
아니다. 원본 무수정 사례는 별도 테스트로 새 거부 지점까지 도달하는 것을 검사한다.
추가 합성 사례에서는 마지막 줄의 text_height를 확장해 최종 TextLine과 종료를 검사한다.

- 동일 높이 제한만 복원한 대조 실행:4건 중3FAIL/1PASS. 기하 양성2건 및 원본의 다음
  거부 도달 검사가 기존 저장 줄 수용 조건에서 실패했다. 마지막 줄 합성 사례는 추가 후
  검사했으므로 수정 전 검출 증거에 포함하지 않는다.
- 수정 후 신규5건PASS:96/192dpi의 줄 높이·baseline·저장 원점·텍스트 소속,
  1128/1155HU 예산 거부와1156HU 수용, 계속 조각의 원문 보존·종료,
  잘못된 높이/baseline 거부, 마지막 줄 확장, 원본의 글자 테두리 경계.
- 관련21건PASS:영역 태그3, 본문 테두리6, 저장 셀 프레임6, 프레임 끝4, #6656 줄 전진2.
- 기존28suite/322건도PASS, 이번 실행 총348건PASS. 전체 CI가 아닌 집중 회귀다.
  Native lib build, Native/wasm32 lib Clippy `-D warnings`, 포맷과 diff 공백 검사 통과.
- 로그:`frame-after-child/stored-extent-{before-isolated,final,regression}-*.log`,
  `range-tags/stored-extent-{build,clippy-native,clippy-wasm,admission,next-gate}.log`.

원본 전체 Native/fresh WASM 시각 검증은 아직 불가능하다. 원본 한컴 피델리티 통과나
R5 전체 완료를 주장하지 않는다. unequal-height와 frame reset이 동시에 있는 독립 한컴
사례는 미검증이다. 기존 승인 WASM 교체·Studio 기본 경로 전환·원격 작업은 하지 않았다.

### 글자 테두리 수용과 정상 한컴 3쪽 대조군 — 2026-09-30

작업지시자의 시각 검증 준비 요청에 따라 p17의 글자 테두리 자식을 지원했다. 원본 IR의
테두리를 지우지 않는다. `char_border::validate_source/qualify`는 지원하는 실선 외곽선과
지원하지 않는 채움·대각선·3D 효과를 구분한다. common paragraph paint가 생성한 Line을
TextRun과 같은 TextLine payload에 보존하여 측정·분할·이동·SVG가 함께 소비하게 했다.
글자가 아닌 자식이 있다고 마지막 TextRun의 hard-break 소유권을 잃지 않도록 보정했다.

Native 직접 비교에서 common producer가 charPr48/49/48 경계마다 사각형을 생성하는
차이가 확인됐다. 한컴 정상 저장본 PDF는 “프로세스에 내재된 업무 관련 규정” 전체에 하나의
테두리를 그린다. `text::compose_shared → char_border::connect_outlines →
stored_text::validate_paint → ParagraphItem::Lines → fit → append_to/translate`에서
측정 전에 연속 TextRun의 동일한 네 면 실선 속성을 연결한다. 밑줄·폰트 run 변경은 연결을
끊지 않지만, 무테두리 run·다른 pen·새 줄은 연결을 끊는다. 좌표가 겹친다는 이유로 서로 다른
소유자의 선을 합치지 않는다. 원래 자식 경계를 검사한 뒤 최종 경계도 검사하며, 잉크 bbox는
공통 `LineNode::ink_bbox`의 butt-cap 규칙으로 계산한다. 줄 높이·기준선·저장 vpos는 불변이다.
일부 면만 있는 선은 기존 개별 payload로 보존한다. 부분 테두리의 연결 피델리티와 CENTER
baseline 정렬의 테두리 수용은 이번 지원/독립 시각 검증 범위에 포함하지 않는다.

독립 기준은 원본 전체를 흉내 낸 수동 HWP가 아니다. 정상 HWP→HWPX 변환본에서 문단5~18의
텍스트와 서식을 보존하고, 단일 단·43888×60000HU 용지·6000HU 여백으로 분리한 입력을
**한컴에서 다시 저장**했다. 이전 LineSeg는 입력에서 제거하여 한컴이 다시 만들게 했다.
원본 페이지 분할과의 일치가 아니라, 이 수정 입력과 한컴 재생성 결과의 대조다.

- 생성 스크립트: `output/7353/r19/range-tags/make-review.py`.
- 한컴 정상 저장 HWP: `range-tags/text-border-portrait.hwp`.
- 실제 Native/WASM 검증 입력: `range-tags/text-border-portrait.hwpx`.
- 대응 기준: `range-tags/text-border-portrait-2020.pdf`,3쪽.
- 한컴11.0.0.9136/profile2020/preprocess none. HWP 저장 job
  `67654f5b-3ef3-4da6-8198-5f3f0d4626fc`, HWPX job
  `d1401ff7-2a5b-4f6c-a1fd-500ade2f3baa`, PDF job
  `2bf92138-1ee2-4276-b090-b8502b823df8`. 상태·다운로드 기록은 `portrait-*json`.
- 정상 저장 HWP에는 한컴이 kind0/tag0 영역 태그를 추가하므로 현재 V2는 거부한다.
  정상 한컴 HWPX 변환본에는 해당 태그가 없다. 이를 임의 제거하거나 kind0 수용을
  완화하지 않았다. 이번 시각 비교는 HWPX 입력이며 HWP 경로까지 통과했다고 하지 않는다.
- 처음 만든 landscape=NARROW 입력/`text-border-review.*`는 별도 진단 산출물이다.
  시각 자료에는 수정된 WIDELY 입력의 `text-border-portrait.*`만 사용했다.

정식 회귀는 `tests/cases/issue_7353_stored_text_extent.rs`에서 원본 p17의 실제 최종 선·
텍스트·행 높이·이동 후 좌표를 검사한다. 동일 pen의 다른 ID는 연결하고 다른 pen/무테두리
중간 run은 연결하지 않는 반례를 포함한다. 연결 계약은 수정 전12선/기대4선으로 FAIL,
수정 후4선으로 PASS했다. 수평 butt-cap bbox도 수정 전 FAIL / 수정 후 PASS로 검사했다.
기하 어댑터 테스트와 정상 한컴 출력의 역할은 계속 구분한다.

검증 source는 HEAD `ed02ae095f2b1eb0f290e422167c81bd45c6b835` 위 미커밋 변경이며
`range-tags/visual-source.sha256`과 각 backend의 `visual/*/manifest.json`으로 고정한다.
Native는 `shared-driver-export.rs`, fresh WASM은 `HostedSectionV2`를 실제 Chrome에서
실행한다. `capture.mjs`와 `sweep.py`가 Studio webfont rasterizer 및 canonical Visual Sweep의
compare/standalone overlay/review를 만든다. 원점을 맞추기 위한 별도 이미지 정렬은 하지 않는다.
대표는3쪽③ 글자 테두리이며1~3쪽의 연결 문단 외곽·분할 전후 줄·후속 선택지도 직접 비교한다.

원본 전체의 다음 거부는
최종 Native prefix 진단에서도 **s0/p145 `V2 paragraph decoration`**으로 확인했다
(`char-border-final-{original-gate,next-gate}.log`). 이 경계의 구현은 이번 시각 준비와 분리한다.
기본 Legacy/Studio 경로 전환·remote push·PR 생성은 하지 않는다.

Native 직접 판독 결과, 대상 문구의 내부 세로선은 없어지고 외곽선 하나로 연결된다.
문단 테두리의1→2→3쪽 이어짐과 선택지 줄바꿈·후속 문구도 비교했다. 남은 차이를
폰트로만 묶지 않는다. PDF의 해당 글자 테두리 stroke는0.24pt, Native192dpi의
stroke0.32px는0.12pt다. 이는 common border producer의 고정 px 폭을 이번 수용 경로가
그대로 보존한 차이이며, 선 굵기 정합 완료를 주장하지 않는다. PDF 좌측 경로168.661pt와
Native168.079pt의 미세한 x 차이 및 글꼴 외형 차이도 있다. 이 차이를 이미지 정렬·
픽셀 기준 완화로 숨기지 않고 시각 판정 자료에 그대로 남긴다.

최종 검증:

- 34suite/350건PASS (`frame-after-child/char-border-final-*.log`). 초회 실행의3개
  잘못된 suite 파일명은 실행기 입력 오류였으며, 실제 파일명으로 위 최종 묶음을 전부 실행했다.
- Native build, Native/wasm32 lib Clippy `-D warnings`, 변경 파일 포맷·diff 공백 검사PASS.
  전체 PR CI/워크스페이스 lint 묶음을 대신하는 결과는 아니다.
- Docker 최종 WASM 빌드 성공8m04s (`range-tags/char-border-final-wasm-build.log`).
  WASM SHA256 `6f16e2062dc60889f018e567388f80a3cd0bbeea910d970a3a7266a235ddc16b`.
  JS SHA256 `4b88c81091df6bbc816aea8ee7568240a1976749d8516f77abcbae41540ab004`.
- fresh WASM을 Chrome에서 실행한 결과3쪽. Native와 render tree의 비수치 차이0,
  수치 차이336항목의 최대2.274e-13px (`visual/wasm/backend-comparison.json`).
- Native와 WASM의1~3쪽 review 및 대표3쪽 standalone overlay를 직접 판독했다.
  동일한 줄바꿈·연결 테두리·후속 선택지를 확인했으며 위 글꼴·선 굵기 차이는 남는다.
  자동 잉크 일치율은1쪽3.79%,2쪽3.77%,3쪽5.38%로, 시각 통과 판정이 아니다.
- 판정 이미지: `range-tags/visual/wasm/review/review_001.png`~`review_003.png`.
  별도 겹침 이미지: `range-tags/visual/wasm/overlay/overlay_003.png`.
  각 backend의 `compare/`, `overlay/`, `review/`, `manifest.json`, 원 SVG/PNG를 보존했다.

상태는 **지원 경계 구현·집중 회귀·Native/fresh WASM 시각 자료 준비 완료 / 메인테이너 판정 대기**다.
정상 재생성3쪽 대조군의 결과를 원본 전체 일치나 HWP kind0 태그 지원 완료로 승격하지 않는다.
기존 pkg는 이번 fresh 빌드로 갱신했지만 rhwp-studio 기본 엔진은 변경하지 않았다.

메인테이너가 위 정상 저장 HWPX 3쪽 비교 자료의 **시각 판정 통과**와 다음 절편 진행을
승인했다. 다음 대상은 원본 s0/p145의 문단 테두리와 혼합 TAC 도형이다. 일반 본문과 달리
`prepare_shapes`의 source admission과 mixed-shape composer가 문단 테두리를 거부한다.
저장 줄의 텍스트/도형 공통 점유 결과를 그대로 fit과 배치에 쓰고, host fragment의 기존
테두리 painter에 연결하는 범위로 조사한다. 셀 내부 및 순수 개체 carrier의 수용은 확대하지 않는다.

### 후속 절편: 혼합 TAC 텍스트 상자의 본문 문단 테두리

대상은 원본 `samples/21_언어_기출_편집가능본.hwp` s0/p145다. 표가 아닌 TAC Rectangle의
텍스트 상자 **‘르포르’**가 본문 3번째 저장 줄에 들어 있다. 그 줄의 독립 저장 높이는
1417HU, 기준선1204HU이며 나머지 본문 줄은1100HU다. 기존 host는 보통 본문의 테두리는
그리지만 `shape_paragraphs`의 source admission과 mixed composer에서는 테두리를 거부했다.

`host_section.rs`의 mixed-inline 분기에만 `host_border::validate_source`를 적용하고,
`host_text::prepare_content → TextComposer::compose_host_inline_shapes → compose_shared`가
동일한 문단 테두리 qualification을 사용하도록 연결했다. `text_ir.rs`의 셀 경로는 기존
`compose_stored_inline_shapes`와 무테두리 계약을 유지한다. 도형 전용 carrier와 배경/인셋
미지원 조건도 그대로다. 소스 속성 삭제나 특정 문서·높이 예외는 없다.

실제 소비 연결:

- 생산: `text.rs::compose_shared` 저장 TextLine 및 `shapes.rs::attach_inline`의 도형을
  포함한 같은 줄 payload/점유 상자. 도형 높이를 별도로 다시 합산하지 않는다.
- 측정: `host_text.rs::prepare_content`가 ObjectRow의 같은 bounds를 FlowBlock::Lines로
  전달한다. `FlowCursor::fit_body_until`은 수용한 LineOwner와 높이/advance를 반환한다.
- 예산 실패와 컷: 미수용 줄은 다음 cursor에 남고, 이미 수용한 줄 payload만 fragment에
  담는다. 합성5000HU 프레임에서13개 본문 줄 및 도형1개가 누락·중복 없이 보존된다.
- 배치: `HostedParagraphPlan::fit`이 그 payload를 최종 줄 위치로 translate한다.
  `layout.rs`의 HostedParagraph 분기는 `host_border::append`에 같은 fragment를 전달한
  뒤 `render_nodes`를 붙인다. 외곽 하단은 공통 occupied end/next_y이며 후속 문단도
  같은 next_y를 소비한다. shape 전용 추가 앵커 선택이나 paint 재조판은 없다.

검증은 `issue_7353_host_paragraph_border.rs`의 실제 최종 노드에서 원본13줄의 저장 간격,
1417HU 도형 높이와 해당 줄 원점, 분할 후 텍스트 순서/도형 개수/외곽선 개수를 검사한다.
두 양성 검사는 변경 전 `V2 paragraph decoration`으로 FAIL, 변경 후96dpi PASS다.
첫 after 실행은 build 완료 전에 이전 rlib를 참조해 거부가 남았으며, 완료된 rlib로 다시
컴파일한 `mixed-border-built`/`mixed-border-final` 결과를 사용한다.
셀 경로가 host 테두리 지원을 잘못 상속하지 않는 반례는 `issue_7353_cell_shapes.rs`에 추가했다.

원본 p145의192dpi에서는 테두리 유무 모두 `text preview run outside occupied line`로
거부된다. 기존 공통 mixed-shape paint의 독립 제약이며 이번에 gate를 완화하지 않았다.
이를 명시적 음성 회귀로 고정했다. 이번 시각 비교는 **96dpi 조판, 2배 raster**이며192dpi
조판 성공을 의미하지 않는다. 원본 전체96dpi의 다음 거부는 **s0/p181
`unqualified stored body frame reset`** (`mixed-host-border/next-gate-96.log`)이다.

시각 입력 생성과 실패 자료:

- 정상 원본 HWPX 변환본에서 p144~146의 텍스트·문단/글자 서식·TAC 도형을 유지하고
  단일 단31888HU의 별도 문서로 분리했다. 원래 줄 캐시는 제거하여 한컴이 생성했다.
  생성 절차는 `output/7353/r19/mixed-host-border/make-review.py`다.
- 최초33000HU 높이 입력은 한컴에서3쪽으로 정상 저장됐지만 TAC 문단 내부에 저장 vpos=0
  reset이 있어 V2가 `unqualified stored body frame reset`으로 거부했다. 이 실패와
  `mixed-host-border/mixed-border.*`, `inspect.log`를 보존한다. 분할본 성공이라고 하지 않는다.
- 테두리+TAC 줄 배치의 독립 판정을 위해 **높이60000HU 한 쪽** 대조군을 별도 `single-page/`에
  만들었다. 텍스트/서식/도형은 같다. 이 대조군 성공을3쪽 분할본/원본 전체의 성공으로 승격하지 않는다.
- 한컴2020 profile /11.0.0.9136/preprocess none. 한 쪽 HWP 저장 job
  `877fa0a1-c0df-4c3a-a85c-52905a16544e`, HWPX `7abea019-814d-4413-baa1-eeb81891622e`,
  PDF `5d8adc73-ae64-45f3-b2db-102cba0515ac`; start/status/download JSON을 같은 폴더에 보존했다.
- 판정 입력 `single-page/mixed-border.hwpx`, 한컴에서 열 수 있는 HWP `mixed-border.hwp`,
  대응 PDF `mixed-border-2020.pdf`. Native에서는 HWP와 HWPX 모두1쪽이며 전체 export JSON도 동일하다.
- Native review에서 연결 문단 외곽, 3번째 줄의 ‘르포르’ 상자, 후속 문단의 줄바꿈을 확인했다.
  글립/글자 잉크 위치와 선 표현의 미세 차이는 남으며, 자동 잉크 보조값3.83%는 시각 판정이 아니다.

집중 검증 `frame-after-child/mixed-border-final-*`: host paragraph border10, cell shapes12,
stored extent7, char style ranges3, hosted section31, inline envelope2, small inline5 = **70건PASS**.
Native와 wasm32 lib Clippy `-D warnings` PASS. 전체 CI/PR 검증은 실행하지 않았다.
검증 source는 기존 HEAD 위 미커밋 변경이며 `single-page/source.sha256`으로 고정한다.
fresh Docker WASM 빌드 성공7m41s (`mixed-host-border/wasm-build.log`). WASM SHA256은
`b478354322ab7d929c2085b247d7b1c00692e2e4b7a9a1844de0f1c07ba50350`이며 JS SHA256은
`4b88c81091df6bbc816aea8ee7568240a1976749d8516f77abcbae41540ab004`다.
Chrome에서 새 `HostedSectionV2`를 실제 실행해 HWP/HWPX 출력이 완전히 같은 것을 확인했다.
Native↔WASM render tree의 비수치 차이0, 수치93항목 최대차이5.684e-14px다
(`single-page/visual/wasm/backend-comparison.json`). SVG 직렬화 문자열은 수치 비교에서 제외한다.
Native/fresh WASM 모두 compare/standalone overlay/review를 생성하고 대표 이미지를 직접 판독했다.
자동 잉크 보조값은 양쪽 모두3.83%이며 남은 글자 잉크·선 표현 차이는 그대로 표시한다.
`capture.mjs`, `sweep.py`, 양 backend의 `manifest.json`에 source/input/PDF/pkg 해시와 명령이 있다.
최종 source 해시 재검사와 변경 파일 rustfmt/diff 공백 검사도 PASS했다.

시각 판정 대상은 `single-page/visual/wasm/review/review_001.png`의 첫 문단3번째 줄
**‘르포르’ 상자**, 연결 문단 외곽, 뒤 문단의 줄바꿈이다. `compare/compare_001.png`와
`overlay/overlay_001.png`도 별도 보존한다. 상태는 **이번 범위 구현·집중 검증·시각 준비 완료 /
메인테이너 판정 대기**다. 기본 Studio/Legacy 경로 전환과 원격 작업은 하지 않았다.

메인테이너가 위 한 쪽 mixed TAC 문단 테두리의 시각 판정을 통과시키고 다음 절편을 승인했다.
다음은 원본 p181 및 보존한3쪽 정상 저장 대조군의 `unqualified stored body frame reset`이다.
원본 p181은13개 저장 줄 중8번 줄이 vpos=0으로 돌아가며, TAC ‘마흐디’ 상자는2번 줄 소유다.
`body_text::frame_starts`와 `stored_text::continuous_paragraph`가 컨트롤 전체를 금지하는 것이
원인이다. 호스트에서 이미 저장 줄 소유/기준선/외곽 점유를 검증하는 mixed TAC Shape 경로에
한정하여 프레임 원점을 연속화하고, 동일 원본 줄 인덱스로 컷을 되돌리는 계약을 연결한다.
비TAC/떠 있는 객체·편집으로 무효화된 저장 줄·임의의 nonzero reset은 수용하지 않는다.
측정은 `host_text::prepare_content`의 같은 줄 bounds, 컷과 예산은 `fit_body_until`, 배치는
`HostedParagraphPlan::fit`의 payload 이동 및 `host_border::append`의 조각 외곽으로 이어진다.
도형을 문단 시작이나 이어지는 모든 쪽에 다시 붙이지 않는다. 표/셀 continuation은 비해당이다.

### Mixed TAC Shape의 호스트 저장 프레임 전환

`body_text.rs::inline_shape_frame_starts`는 기존 `shapes::validate_inline`로 저장 줄과
TAC 소유가 검증된 경로만 허용한다. `stored_text.rs::continuous_inline_shapes`가 기존
원점 연속화 구현을 공유하고, `host_text.rs::prepare_content`에서 원래 줄 번호의 컷으로
되돌린다. 텍스트·컨트롤 슬롯·UTF-16 축과 입력 IR은 바꾸지 않는다.

실제 소비 경로는 `shapes.rs::attach_inline`의 저장 줄 소유/기준선과 margin 포함 외곽 →
`host_text.rs:392`의 공통 bounds/ObjectRow → 같은 파일의 frame_breaks와
`flow.rs:90::fit_body_until`의 수용 높이/이월 → `host_text.rs:524::fit`의 수용한 줄 payload
이동 → `host_border.rs:92::append`의 첫/마지막 조각 외곽이다. 예산이 작으면 원래 저장 컷
앞에서도 같은 줄 단위로 이월하며, 도형을 다음 조각마다 다시 생성하지 않는다.
저장 컷 직전 FollowingLineGap은 이전 프레임 소유이며 실제 빈 문단과 구분한다.
표의 rowspan/캡션/각주 경로와 편집 후 재조판의 범위 확대는 하지 않았다.

`tests/cases/issue_7353_host_shape_frames.rs`에서 원본 p181의13줄/상자 소유2번/저장 컷8번과
독립 저장 줄 높이·간격을 검사한다. 합성 반례는 컷을2번으로 옮겨 상자를 이어지는 첫 줄로
만든 경우, 원래 컷보다 작은 물리 예산, 후속 빈 문단과 다음 텍스트 보존이다. 최종 render tree에서
모든 줄0..12가 정확히1번, 상자1개, 실제 상자 원점/높이, 불필요한 빈 쪽 없음과 외곽 시작·끝을
검사한다. 비TAC/무효 캐시/nonzero reset/줄 외곽 초과는 거부한다. 합성 반례는 한컴 fidelity
증거가 아닌 소유/배치 계약이다. 수정 전3건은 실제 `unqualified stored body frame reset`으로
FAIL하고 음성1건은PASS했다. 수정 후 후속 문단과2단 검사까지6건PASS했다.
2단 검사는 같은 소유 줄들이 같은 너비의 다음 단으로 옮겨지며 쪽 수는1, 상자는1개인 것을
최종 좌표로 확인한다. 처음 테스트의 x=0 가정은 원본 들여쓰기를 빠뜨린 것이므로, 같은 저장
문단을 같은 폭의 단에 옮길 때 원점 차이만 더해진다는 평행이동 불변식으로 수정했다.
이 테스트 보정 중 production source는 바꾸지 않았다.

집중 검사: 새6, hosted section31, cell shapes12, host border10, stored hyperlink4,
stored cell frames6, body field end8 = **77건PASS**. `frame-after-child/host-shape-frames-*`에
로그가 있다. field end의 첫 시도는 임시 rustc runner가 zip 의존성을 전달하지 않아 빌드 실패했고,
동일 source에 `--extern zip`을 전달한 재실행8건PASS다(결함 재현으로 세지 않음).
Native/wasm32 lib Clippy `-D warnings`, 변경 파일 rustfmt check PASS. 전체 CI는 미실행이다.
원본 전체96dpi의 다음 거부는 **s0/p212 `text preview run outside occupied line`**이며
`mixed-host-border/frame-next-gate.log`에 보존했다. 그 gate는 완화하지 않았다.

시각 입력/기준 보정:

- 최초3쪽 기준 PDF를 직접 열었을 때 오른쪽 내용 잘림을 발견했다. 입력43888×33000HU와
  PDF328×439pt가 서로 다른 가로/세로 방향이므로 그 이미지는 판정에서 제외했다. 원본 실패
  자료는 `mixed-host-border/visual/native/`에 보존하며 이 출력에 맞춰 조판 코드를 바꾸지 않았다.
- `make-review.py --square-split`은 같은 p144~146의 텍스트/서식/TAC 도형을 유지하고
  용지만43888×43888HU, 위아래 여백11444HU로 바꾼다. 본문 폭31888HU/높이21000HU는
  이전 분할 입력과 동일하다. 저장 LineSeg는 한컴이 다시 생성한다. 원본 전체 페이지 fidelity와
  분리된 정상 저장 대조군이다. `square-split/`에 입력·HWP·HWPX·PDF·호출 JSON을 보존했다.
- 한컴2020 profile/11.0.0.9136/preprocess none: HWP job
  `43540352-3222-479d-b93f-c1a3bddf9c19`, HWPX `383a3d83-6cd2-48d6-8bff-1ce3468d755f`,
  PDF `3027d727-885f-43e0-8eb2-40dbf52446ff`. PDF는439×439pt/3쪽이며 잘림 없이 읽힌다.
- HWP SHA256 `2e555a72a23aa48956f440771446c6686dad9cf84db22f84bd0f601d4ffba7ef`,
  HWPX `ecff0af04c2b209bb2d77d1a4475a4f6e731271f0d853eace16f38ef74b79e28`,
  PDF `96bc602cdd91a04bd27eb2c8630f70b9a45d52d35c6044dd4d7b7e6c6d376a77`.
- 동일 정상 저장 HWPX는 수정 전 export 실행에서 저장 프레임 거부, 수정 후3쪽이다.
  Native HWP/HWPX의 export JSON은 완전히 같다. `before.log`, `native*.json/log`에 근거가 있다.
- Native96dpi/2배 raster의3쪽 review를 직접 판독했다. 1쪽의 ‘르포르’ 상자는1회만 표시되고,
  문단이2쪽에서 이어진 뒤 후속 문단과3쪽 끝 테두리까지 보존된다. 줄바꿈은 같으며 글립과 잉크
  위치·선 표현의 차이가 남는다. 자동 잉크 보조값은1쪽3.53%,2쪽3.62%,3쪽2.93%로 사람 판정이 아니다.

fresh Docker WASM 빌드 성공7m41s (`mixed-host-border/frame-wasm-build.log`). WASM SHA256
`b8be778da1e1bbe6edd26cdffada8fb5faa943f572606ecd12e7947dbcbbb9cf`, JS SHA256
`4b88c81091df6bbc816aea8ee7568240a1976749d8516f77abcbae41540ab004`다. 새 바이너리의
Chrome HostedSectionV2로 HWP/HWPX 모두3쪽, 두 입력의 출력 JSON 완전 일치를 확인했다.
Native↔WASM은 비수치 차이0, 수치104항목 최대5.684e-14px다. SVG 직렬화 문자열은 이 수치
비교에서 제외하며 각 backend의 실제 SVG를 별도로 raster/직접 판독했다.
`square-split/visual/wasm/backend-comparison.json`, 양 backend `manifest.json`,
`source.sha256`, `test.sha256`에 source와 입력/출력 식별자를 고정했다.
실행 명령은 `REVIEW_BASE=output/7353/r19/mixed-host-border/square-split`을 지정한
`node output/7353/r19/mixed-host-border/capture.mjs --wasm` 및 같은 디렉터리의
`sweep.py wasm`이다. Native는 `--wasm` 없이 같은 capture를 사용했다.

fresh WASM3쪽 review/compare/standalone overlay를 직접 확인했다. Native와 동일한 줄 분할과
상자/외곽 이어짐이며 자동 보조값도3.53/3.62/2.93%다. 남은 글립/잉크 위치 차이는 통과로
숨기지 않는다. 판정 자료는 다음과 같다(경로 접두어 `output/7353/r19/mixed-host-border/square-split/visual/wasm/`).

| 쪽 | Review | Compare | Overlay | 확인 대상 |
| --- | --- | --- | --- | --- |
| 1 | `review/review_001.png` | `compare/compare_001.png` | `overlay/overlay_001.png` | 르포르 상자1회, 문단 끝 컷 |
| 2 | `review/review_002.png` | `compare/compare_002.png` | `overlay/overlay_002.png` | 이어지는2줄과 후속 문단, 옆 테두리 |
| 3 | `review/review_003.png` | `compare/compare_003.png` | `overlay/overlay_003.png` | 마지막2줄과 닫힘 테두리 |

상태: **이번 범위 구현·집중 검증·Native/fresh WASM 시각 준비 완료 / 메인테이너 판정 대기**.
원본 전체의 다음 거부 p212와192dpi의 기존 mixed-shape paint 거부는 남는다.
Studio 기본 Legacy 경로를 V2로 전환하거나 원격 작업·커밋을 하지 않았다.

메인테이너가 위3쪽 시각 판정을 통과시키고 다음 절편을 승인했다.
다음 p212의 첫 줄 마지막 run은 우단413.813333px에 대해414.109524px까지 기록된다.
원인은 공통 `paragraph_layout.rs`의 TAC 뒤 마지막 조각만 `estimate_text_width`로 정수 반올림하는
것이다. 측정의 `estimate_line_run_widths`와 TAC 앞 조각은 이미 exact 폭을 쓴다(#7254 계약).
실제 backend는 소수 글자 전진을 소비하므로 같은 exact 폭으로 마지막 조각의 bbox와
후속 run 원점을 갱신한다. V2 수용 오차·LineSeg·글꼴 메트릭·TAC 크기는 바꾸지 않는다.
원본 p212와 기존 p145의96/192dpi로 최종 줄 외곽 및 글자 전진/내용 보존을 검사한다.

TAC 뒤 마지막 run 폭 절편 결과:

- `paragraph_layout.rs`의 마지막 TAC 뒤 조각을 exact 폭으로 바꿨다. 측정의
  `estimate_line_run_widths` → 마지막 조각 `seg_w` → `emitted_run_layout_positions` →
  TextRun bbox/후속 x가 같은 소수 전진을 소비한다. trailing-space 분배와 font replay는
  기존대로이며 저장 줄의 폭이나 수용 오차를 늘리지 않았다.
- 새 `tests/cases/issue_7353_inline_tail_advance.rs` 4건은 원본 p212/p145의96/192dpi에서
  최종 bbox와 공개 `EmbeddedTextMeasurer.compute_char_positions`의 전진을 비교하고,
  저장 줄 수·전체 본문·도형1개를 검사한다. 수정 전4건 FAIL / 수정 후4건 PASS.
  기대값은 수정한 helper를 재호출하지 않고 backend 문자 전진 계약과 원본 저장 줄에서 정한다.
  trailing space의 caret 전진은 잉크 끝점과 구분한다.
- 기존 p145192dpi 거부 계약은 같은 반올림 원인의 결과였으므로 저장 줄 소유/위치 보존
  양성 계약으로 변경했다. guard 완화나 golden 갱신이 아니다.
- 집중34건 + tab/inline-table/shape/picture 정상 대조6건 + stored-inline suffix2건 =
  **42 PASS**. 로그: `output/7353/r19/frame-after-child/inline-tail-final-*`,
  `inline-tail-controls-*`, `mixed-host-border/tail-width/suffix-test.log`.
  suffix 검사는 기존 `unused_mut` 때문에 `-D warnings` 빌드가 실패했으며, source를 바꾸지
  않고 경고를 허용한 별도 진단 실행에서2건 PASS였다. 이를 lint 통과로 세지 않는다.
- Native build와 Native/WASM-target lib Clippy 통과(30.39s/31.82s).
  workspace/all-target Clippy, 전체 회귀, fresh WASM은 아직 실행하지 않았다.
  원본 전체96dpi의 다음 거부는 p299 `positioned table requires single-column anchor context`.

시각 선행 검증에서 별도 결함을 검출했으므로 **이번 절편 전체 완료/시각 승인 요청은 보류**한다.
정상 저장 대조군은 `output/7353/r19/mixed-host-border/tail-width/`다. 원본 HWPX의 p212만
분리하고 본문 폭31888HU·단일 단·portrait 용지를 지정한 뒤 저장 LineSeg를 지워 한컴에서
재생성했다. 원본 전체 문서의 페이지 fidelity를 주장하지 않는다. HWP job
`394f6477-dda5-4815-9199-5957e9723e5d`, HWPX job `fd6183a8-dd67-4a23-9b4b-9d661c32e0f8`,
PDF job `dbd8a0cc-a636-4de6-aa6d-f3d5633bb92f` 모두2020 profile/succeeded.
PDF는439×600pt/1쪽이다. HWP는 V2 출력되나 별도 HWPX 변환본은
`V2 cell rectangle/textbox geometry`로 거부된다(정확한 하위 조건 미조사).

HWP Native review를 직접 판독한 결과 첫 줄의 **‘낙론계 유학자들’ TAC 글상자**가
‘이이의 계승자인’ 다음이 아니라 문단 맨 앞에 나온다. 앞서 언급한 김원행은 본문 인명이며
글상자 내용이 아니다. `host_section.rs`의 local.controls.retain은 SectionDef/ColumnDef를
제거하지만 char_offsets는 보존한다. 정상 저장본의 컨트롤 위치 `[0,0,9]`에서 구조 컨트롤을
제거한 뒤 `control_text_positions()`가 남은 shape를 첫 슬롯0으로 읽는 연결을 확인했다.
수정 전 `frame-export`와 수정 후 `tail-width/export`에서 도형 bbox는 모두
x104.5866667,y80,width103.96,height18.8933333으로 동일하므로 이번 폭 변경의 신규 회귀는 아니다.
단, 기존 결함이라는 이유로 시각 검증을 통과시키지 않는다.

증적: `tail-width/parsed.txt`, `before-native.json`, `native.json`, `source.sha256`,
`visual/native/manifest.json`, `visual/native/review/review_001.png`,
`visual/native/compare/compare_001.png`, `visual/native/overlay/overlay_001.png`.
자동 잉크 보조값3.46%는 사람 판정이 아니다. 다음 우선순위는 구조 컨트롤 제거 시 원본
TAC 슬롯/문자 위치 보존이며, 이 경계를 수정 전 FAIL / 수정 후 PASS로 고정하고 같은 정상
입력에서 재캡처한 뒤 fresh WASM을 한 번 빌드한다. p299 지원 확대는 그 이후다.
이번에는 원격 작업·커밋·Studio 기본 경로 변경을 하지 않았다.

### 2026-09-30 후속 — 구조 컨트롤 뒤 TAC 글상자의 문자 위치 보존

작업지시자가 결함 수정과 시각 검증 준비를 승인했다. 정상 저장 대조군은 바꾸지 않고
`tail-width/mixed-border.hwp`와 같은 입력의 한컴 PDF를 재사용한다. 원본 전체 문서가 아닌
p212 단일 문단 대조군이라는 범위는 유지한다.

- 위반 규칙: SectionDef/ColumnDef는 보이는 글자가 아니지만 원문 문자 스트림의 슬롯이다.
  슬롯을 삭제한 controls와 삭제 전 char_offsets를 함께 소비하면 뒤 TAC의 위치가 바뀐다.
- 수정: `host_section.rs`의 mixed-shape 분기는 이미 호스트가 수용한 원본 controls를
  그대로 보존하고 문단 쪽/단 break만 호스트 소비로 처리한다. `shapes.rs`는 해당 구조
  컨트롤을 mixed TAC 문단에서 허용하되 도형 크기·저장 축·줄 외곽 검사는 그대로 유지한다.
- 소비 경로: 원본 `[secd,cold,shape]`/char_offsets → `prepare_shapes` →
  `compose_host_inline_shapes` → 공통 paragraph painter의 `control_text_positions`/inline
  object x → `shapes::attach_inline`의 같은 control index/저장 baseline → 최종 Rectangle.
  앞 텍스트, 글상자, 뒤 텍스트가 같은 순서/위치 결과를 소비하며 임의 x 보정은 없다.
  문단의 모든 컨트롤을 삭제하는 일반 본문·absolute table 경로와 text-free 도형 경로는 바꾸지 않았다.
  분할 컷·예약 높이·종료 규칙 변경은 비해당이며 기존 host frame 분할6건으로 비영향을 검사한다.
- `tests/cases/issue_7353_host_shape_slots.rs`에 정상 HWP96/192dpi 두 검사를 추가했다.
  같은 줄에서 prefix `이이의 계승자인 ` → 글상자 → suffix의 실제 순서와 인접 x,
  원문142HU 오른쪽 바깥여백, 저장 줄 y/전체 텍스트/글상자1회를 검사한다.
  독립 근거는 한컴 PDF와 정상 저장 문자 위치 `[0,0,9]`, 저장 LineSeg/여백이다.
  수정 전 두 건 모두 prefix가 빈 문자열이어서 FAIL, 수정 후 PASS다.
  정상 HWP/PDF는 `tests/fixtures/issue7353_host_shape_slots/`에 보존했고 생성 job/hash는
  testcase 머리말에 기록했다. 새 파일은 아직 커밋하지 않았다.
- 실제 글상자 x: 수정 전104.5866667px → 수정 후205.2561905px.
  y80/width103.96/height18.8933333px는 동일하다. 처음부터 글상자 앞에 존재하던9문자의
  전진을 되찾은 것이다. 테스트의 기대값을 이 관측 x로 바꾸지 않는다.
- 별도 한컴 HWPX 변환본은 구조 축 검사를 통과한 뒤
  `text preview run outside occupied line`로 거부된다(`shape-slots/hwpx-after.log`).
  해당 입력의 종단 출력과 HWP/HWPX 동등성은 미검증이며 수용 조건을 완화하지 않았다.

증적 루트는 `output/7353/r19/mixed-host-border/shape-slots/`.
Native 선행 review에서 첫 줄 순서가 한컴과 같고 후속 줄바꿈·외곽선이 보존됨을 직접 확인했다.
글립/미세 잉크 위치 차이는 남으며 내용 픽셀 보조값3.56162%는 사람 판정이 아니다.
source manifest를 고정해 Docker fresh WASM 검증을 진행한다. 포맷 정리 전 시작한 첫 빌드는
초기에 중단했으며 `wasm-before-format-aborted.log`는 성공 증거가 아니다. 아래 최종 검증은
포맷 완료 후 source를 기준으로 기록한다.

최종 Native 검증: build PASS, 수정 파일 rustfmt/check와 `git diff --check` PASS,
Native lib Clippy29.14s/WASM-target lib Clippy31.04s PASS. 집중41건은
`bash output/7353/r19/frame-after-child/run-policies.sh shape-slots-final`로 다음 source를
실행했다: `issue_7353_host_shape_slots`2, `issue_7353_inline_tail_advance`4,
`issue_7353_host_shape_frames`6, `issue_7353_cell_shapes`12,
`issue_7353_host_paragraph_border`10, `issue_7353_hosted_master`7.
전후 로그는 같은 디렉터리의 `shape-slots-before-*`/`shape-slots-final-*`에 있다.
이전 절편 테스트 수와 중복 합산하지 않는다. source-side cfg(test)를 변경하지 않았으며,
전체 CI 상당 회귀·workspace/all-target Clippy·새 fixture baseline 등록은 이번 내부 집중 검증에서
실행하지 않았다. PR 준비 완료를 의미하지 않는다.

검증 head는 `ed02ae095f2b1eb0f290e422167c81bd45c6b835` + 미커밋 작업 변경이다.
`shape-slots/source.sha256`은 Rust1100개 파일의 실제 내용이며 manifest 파일 자체의 SHA256은
`6cf1cc15bd8e982e0f3c300c39753666063cc0a9aa1050149d8f1e46535deb25`다.
최종 Native capture는 아래와 같다(같은 env에서 fresh WASM에는 `--wasm`을 추가).

```bash
REVIEW_BASE=output/7353/r19/mixed-host-border/shape-slots \
REVIEW_INPUT_FORMAT=hwp \
REVIEW_SCOPE='Hancom regenerated p212 HWP; source control slots preserved; not original whole document' \
VISUAL_SWEEP_CHROME=/home/edward/.cache/puppeteer/chrome/linux-152.0.7977.54/chrome-linux64/chrome \
node output/7353/r19/mixed-host-border/capture.mjs
REVIEW_BASE=output/7353/r19/mixed-host-border/shape-slots \
/home/edward/mygithub/rhwp/venv/bin/python output/7353/r19/mixed-host-border/sweep.py native
```

Docker fresh WASM 빌드 **PASS/7m44s**. 명령은
`docker compose --env-file .env.docker -f docker-compose.yml -f output/7353/r19/cell-floating-picture/docker-network.yml run --rm wasm`이며
`shape-slots/wasm-build.log`에 보존했다. WASM SHA256은
`d32d8087a093b0db199134279bf93e85bc9a5f76f825069a0e9d9fca3d69526c`, JS SHA256은
`4b88c81091df6bbc816aea8ee7568240a1976749d8516f77abcbae41540ab004`다.
Chrome의 새 `HostedSectionV2`로 HWP1쪽을 출력했다. Native↔WASM의 비수치 차이0,
수치68항목 최대차5.684341886080802e-14px다. SVG 직렬화 문자열은 수치 비교에서 제외하고
각 backend의 실제 SVG를 별도로 raster했다. 빌드·캡처 후 source manifest1100파일도 모두 일치한다.

최종 fresh WASM review를 직접 열어 ‘이이의 계승자인’ → ‘낙론계 유학자들’ 글상자 →
‘은 귀신을…’ 순서, 후속 줄바꿈/문단 외곽 보존을 확인했다. 자동 보조값3.56162%,
글립·미세 잉크 위치 차이는 남는다. 최종 시각 판정은 메인테이너에게 요청한다.

- 검토 HWP: `tests/fixtures/issue7353_host_shape_slots/saved.hwp`
- 한컴 기준: `tests/fixtures/issue7353_host_shape_slots/reference-2020.pdf`
- review: `output/7353/r19/mixed-host-border/shape-slots/visual/wasm/review/review_001.png`
- compare: `output/7353/r19/mixed-host-border/shape-slots/visual/wasm/compare/compare_001.png`
- standalone overlay: `output/7353/r19/mixed-host-border/shape-slots/visual/wasm/overlay/overlay_001.png`
- Native 증적은 같은 루트의 `visual/native/`, backend 차이는 `visual/wasm/backend-comparison.json`.

상태: **HWP 구조 슬롯 결함 수정·집중 검증·Native/fresh WASM 시각 판정 준비 완료**.
별도 HWPX 변환본의 텍스트 줄 경계 거부, 전체 회귀/PR 필수 검증은 남아 있다.
Studio 기본 경로 전환·원격 변경·커밋은 하지 않았다.

### 2026-09-30 — 구조 슬롯 시각 승인 및 기반 인계 마감으로 전환

작업지시자가 위 HWP 구조 슬롯 수정의 시각 판정을 **통과**로 승인했다.
이 승인은 해당 정상 HWP 입력과 Native/fresh WASM 증적 범위이며, 원본 전체 또는 별도
HWPX 변환본의 미지원 경로까지 통과한 것으로 확대하지 않는다.

이어 작업지시자는 모든 디테일을 이번 타스크에서 해결하는 대신, 전체 구조와 검증된 기반을
인계하여 리팩토링 전용 원격 브랜치에서 후속 개발한 뒤 devel에 병합하고 **0.9.0으로 출시**하는
전략을 결정했다. 현재 브랜치를 바로 devel에 병합하지 않는다.

종료 범위·고정 인계 항목 H1~H4·조사 중단 조건·진행 보고 방식의 정본은
[구현계획 5.2](../plans/task_m100_7353_impl.md#7353-closeout-090)다.
기존 절편을 되풀이하거나 새 거부 지점을 자동 편입하지 않는다. 다음 작업은 코드/기존 증거를
기준으로 인계 잔여량과 예상 소요 범위를 확정하는 것이며, p299 추가 구현이 아니다.
미해결 사항은 후속 분류하되 해결·PASS로 변경하거나 기준값을 완화하지 않는다.

이번 변경은 수행·구현계획과 결정 기록만 수정했다. 제품 코드 추가 수정, 테스트 재실행,
커밋·원격 브랜치 생성/push·devel 병합·버전 변경은 수행하지 않았다.

### 2026-09-30 — H1/H3 인계 확정과 로컬 기준점 보존

이번 실행의 범위는 H1/H3 문서 확정과 H4 로컬 커밋이며, 예상 작업 묶음은 15~25분으로
설정했다. 다음 거부 지점 구현이나 새로운 지원 조합 조사는 수행하지 않았다.

- **H1 완료**: 현재 V2의 내용 생산→공통 컷/예약→호스트 query/commit→실제 layout 경로를
  구현계획 5.2의 범위표에 고정했다. `typeset/hosted.rs`는 `HostedTableSession::query`의
  proposal을 `commit`해 `PageItem::HostedTable`로 전달하며, layout이 그 packet을 소비한다.
  선택 표/독립 본문/기존 호스트 연결의 세 진입점을 구분했다. Studio 기본 엔진 전환은 없다.
- **H3 완료**: 원본 p299 다단 위치 표는 후속 개선, p212 유래 HWPX 거부는 추가 조사로
  인계했다. 전체 문서 수용·Studio 편집/기본값 전환·devel 통합/0.9.0 출시는 후속 범위다.
  메인테이너를 분류 책임자로 명시하고 구현 담당 미배정 사항을 숨기지 않았다.
- `mydocs/tech/typesetting_architecture.md`에 V2의 기여자 수정 위치와 Legacy 경계를 추가했다.
  `tests/cases/issue_7353*.rs` 70개/`#[test]` 716개는 정적 인벤토리이며 실행 통과 수가 아니다.

**기존 증거 재사용 확인**:

```bash
sha256sum --check --quiet output/7353/r19/mixed-host-border/shape-slots/source.sha256
sha256sum output/7353/r19/mixed-host-border/shape-slots/source.sha256 pkg/rhwp_bg.wasm pkg/rhwp.js
sha256sum tests/fixtures/issue7353_host_shape_slots/saved.hwp tests/fixtures/issue7353_host_shape_slots/reference-2020.pdf
```

Rust 1,100파일 모두 일치했다. manifest `6cf1cc15...`, WASM `d32d8087...`, JS `4b88c810...`,
HWP `78c1b9af...`, PDF `85210494...`는 위 최종 시각 증거와 동일하다. 이전 집중 41건의
로그도 확인했다. 이것은 같은 구현의 증거 재사용이지 새로운 전체 회귀 PASS가 아니다.
제품 소스 변경 없이 문서와 기준점만 정리하므로 Docker 재빌드·재캡처를 반복하지 않았다.

**이번 실행**:

- `cargo fmt --all -- --check`: exit 0. 확인 로그 `/tmp/rhwp-7353-closeout-fmt-confirm.log`.
- `node --test scripts/tests/rust-test-suite-manifest.test.mjs`: 23 PASS/0 FAIL.
  로그 `/tmp/rhwp-7353-closeout-manifest-tests.log`. suite 배정 알고리즘 검사이며 Rust 회귀가 아니다.
- `git diff --check`: PASS.
- 전체 nextest·Native Skia 3종·native/WASM/workspace Clippy 및 base 고정 정책 검사는
  별도 실행 승인을 요청했다. 미실행을 통과로 보고하지 않는다. 신규 fixture는 `tests/fixtures/`
  아래이며 `samples/` 자동 스캔 대상에 추가한 것은 아니다. 정식 회귀 준비 시 적용 범위를 확인한다.

**기준점 커밋 범위**: 이전 시각 승인까지의 미커밋 renderer 변경, 정식 테스트 6개 신규 source와
기존 cell_shapes 수정, HWP/PDF fixture, 수행·구현계획/본 기록/책임 지도만 포함한다.
사용자 소유 `.agents/skills/`, `.codex/`, 비밀 설정, `pkg/`, `output/`, 파생 suite/manifest는
stage하지 않는다. 이 기준점은 검증 대기 상태를 보존하는 로컬 인계점이지 PR 제출 완료가 아니다.
원격 생성/push·devel 병합·버전 변경·이슈 종료는 수행하지 않는다.

고정 잔여 작업은 **H2 전체 마감 검증 → H4 원격 인계와 SHA 확인**이다. 승인 뒤 검증은
잠정 30~90분(재컴파일·디스크/실패 불확실성 포함), 통과 후 원격 게시 확인은 5~10분 예상이다.
새 피델리티 결함을 이 일정에 자동 편입하지 않는다.

### 2026-09-30 — 승인된 H2 전체 마감 검증

작업지시자가 전체 검증 실행을 승인했다. 검토용 detached worktree에 파생 suite를 준비하고,
기존 `target/pr-review`를 공유하여 순차 검증했다. 기준은 로컬 인계점
`780de8512e7a1e4bc34ace072a07f1ff957d8886`, 비교 base는 fetch한
`upstream/devel`의 `0e8fd49fb868da0d47ac1294dcbbda81f0211233`이다.
devel을 작업 브랜치에 병합하거나 새 조판 기능을 구현하지 않았다.

**검증 중 정리**: 파생 suite 준비 후 fmt가 기존 테스트 3개
(`issue_7353_hosted_page_number`, `issue_7353_hosted_section`, `issue_7353_table_v2_oversize_row`)의
포맷 오류를 검출했다. rustfmt로 정리했다. 이어 workspace/all-target Clippy가
`issue_7353_page_number_timeline.rs`의 double-ended iterator `.last()`를 검출하여
같은 마지막 일치 항목을 선택하는 `.next_back()`으로 바꿨다. 기대값·검사 의미·제품 코드는
변경하지 않았다. 이전 단계의 파생 suite 미준비 fmt PASS가 이번 확장 범위의 PASS를 뜻하지 않는다.
최초 lint 실패 로그도 보존했다.

실행 대상은 위 head + 테스트 전용 patch다. patch는
`output/7353/closeout/test-hygiene.patch`, SHA256
`d4eefe8ff3750f31d662a6b1bdf88b5a58d07d9cd0593bd58eda607cf3558636`이다.
task/review worktree의 4개 테스트 내용은 동일하다. 이후 로컬 기록 커밋은 이 검증 상태를 보존하며,
문서 커밋 때문에 동일 제품 코드의 WASM/시각 캡처를 반복하지 않는다.

**실행 및 증거** (`output/7353/closeout/`):

| 검사 | 결과 | 로그 |
| --- | --- | --- |
| integration suite 준비 | 1,470 source → 28 generated suite + 20 exception target | `prepare.log` |
| fmt | 포맷 수정 후 PASS | `fmt.log` |
| Native Clippy | PASS, 61초 | `clippy-native.log` |
| WASM target lib Clippy | PASS, 60초 | `clippy-wasm.log` |
| workspace build | PASS, 119초 | `workspace-build.log` |
| workspace/all-target Clippy | 반복자 수정 후 PASS, 34초 | `clippy-workspace.log`, 최초 실패 `clippy-workspace-attempt1.log` |
| base 고정 suite 정책 | PASS | `policy-final.log` |
| base 고정 source-side unit 정책 | PASS: 4,205 tests / 298 modules | `unit-test-policy.log` |
| 전체 release-test nextest | **10,851 PASS / 8 FAIL / 50 SKIP**, 78 binaries | `regression.log` |
| #7353 전용 계약 | 위 전체 실행에 포함된 **716 PASS / 0 FAIL** | `regression.log`의 `issue_7353` 항목 |
| Native Skia lib | **4,112 PASS / 0 FAIL / 13 ignored**, 424초(빌드 포함). rhwp 3,930건 + 나머지 lib 182건 | `skia-lib.log` |
| Native Skia placeholder | **2 PASS / 0 FAIL**, 298초(빌드 포함) | `skia-placeholder.log` |
| Native Skia PDF 직접 내보내기 | **4 PASS / 0 FAIL**, 14초(빌드 포함) | `skia-pdf.log` |

명령 원문은 각 `*.command`, 시각/종료값은 `status.tsv`, 재현 순서는 `run-validation.sh`에 있다.
전체 명령은 `cargo nextest run --locked --cargo-profile release-test --target-dir
/home/edward/mygithub/rhwp/target/pr-review --tests --test-threads 4 --no-fail-fast`였다.
컴파일 15분26초, 테스트 617.041초, 해당 gate 총 1,546초다. 50건은 기존 제외이며 새 ignore를
추가하지 않았다. nextest 0.9.137은 required 0.9.91을 만족하지만 recommended 0.9.140보다 낮고,
미사용 `ci-duration-observation.junit.report-skipped` 설정 경고가 있었다. 실행 실패는 아래
assertion 8건이며 이 도구 경고로 대체 설명하지 않는다.

**실행으로 확인한 실패 — 원인·기존/신규 여부는 미검증**:

| 구분 | 실패 검사 | 실제 관측 |
| --- | --- | --- |
| 폰트 추적 1건 | `issue_4961_font_decision_trace::stage4_public_hwp_hwpx_profiles_are_end_to_end_and_feature_detected` | `exact-face` layout hash: actual `a4491b55…`, expected `52a2230c…` |
| SVG 4건 | `svg_snapshot::{form_002_page_0, issue_157_page_1, issue_677_bokhakwonseo_page1, issue_617_exam_kor_page5}` | golden/실제 SVG 불일치. 양쪽 원본을 `svg-mismatches.tar.gz`에 보존 |
| 저장 보존 1건 | `issue_3528_nested_caption_boundary::caption_holding_a_table_keeps_its_paragraphs` | 재로드한 캡션 내부 표의 문단 수 목록 `[]`, expected `[3]` |
| 원본 공백 1건 | `issue_6699_terminal_tracking::logo_and_first_glyph_match_existing_hancom_pdf` | `원본 공백 유지` assertion 실패 |
| 인라인 위치 1건 | `issue_6699_textbox_table_inline_pictures::first_page_inline_pictures_follow_their_text_line` | 실제 text x `334.33333333333337`에서 assertion 실패 |

위 결과는 테스트 계약 미충족이며 원인 또는 한컴 피델리티의 최종 판정을 대신하지 않는다.
이번 실행에서 base/release 대조 빌드는 하지 않았다. #4961 fixture의 기대 해시도 현재 devel과
다르므로 단순히 devel 기존 실패라고 주장할 수 없다. baseline/golden 갱신, ignore 추가,
새 조판 수정은 하지 않았다. 실패가 있는 H2를 완료로 바꾸지 않으며, 상세 해결은 자동 편입하지 않는다.

제품 source manifest 1,100파일을 다시 검사해 최종 승인된 Native/fresh WASM 빌드와의 일치를
확인했다. 기존 구조 슬롯 시각 PASS의 제한 범위만 재사용한다. #7353 계약 전부 PASS는 전체
기본 경로 무회귀나 모든 문서 지원 완료를 뜻하지 않는다.

Native Skia 3종은 전체 회귀 실패와 독립적으로 실행해 모두 통과했다. focused 명령의 234/255
skipped는 선택한 모듈 밖 테스트를 필터한 수이며 새 ignore가 아니다. 실행 기록은 `run-skia.sh` 및
`status.tsv`에 이어 남겼다. 이것이 전체 회귀 실패를 면제하는 것은 아니다.

**마감 상태**: 승인된 검증 실행은 19:23~20:09 KST, 약 46분에 마쳤다. H1/H3는 완료,
H2는 8건의 계약 실패로 **미충족**, H4는 로컬 보존/원격 게시 보류다. 다음 결정 범위는 실패 8건의
제한된 base 대조 및 인계 조건이며 전체 회귀 재실행·새 조판 구현을 자동 착수하지 않는다.
결과 기록과 테스트 정리만 로컬 커밋하고, 이번 검증용 `/tmp/rhwp-7353-closeout-2KbqaT`는
패치/실패 SVG/로그 보존 확인 후 제거한다. 공유 build target과 사용자의 기존 worktree·WIP는
보존한다. 원격 push·PR·devel 병합·0.9.0 버전 변경·이슈 종료는 하지 않았다.

### 2026-09-30 — 실패 원본의 V2 WASM 실행 경로 재검사

작업지시자는 WASM 조판 경로를 V2로 전환한 뒤 실패 여부를 다시 확인하도록 지시했다.
검증 runner의 문서 진입점을 `HostedSectionV2`로 고정하고 실제 Chrome의 WASM에서 실행했다.
이 runner에는 `HwpDocument` 생성이나 Legacy fallback이 없다. 제품/Studio의 기본 문서 API를
교체한 것은 아니며, 그 전환에는 편집·페이지 API 연결이 추가로 필요하다. 전환 범위를 질문했으나
응답 전에는 독립된 V2 검증 경로만 실행했다.

검증 head `1f28344b673b66911d14e0c1f3d8e35444b8a308`의 제품 Rust 1,100파일이 기존 fresh WASM
source manifest와 일치함을 확인했다. 제품 코드를 변경하지 않아 Docker 빌드를 반복하지 않았다.
WASM SHA256 `d32d8087a093b0db199134279bf93e85bc9a5f76f825069a0e9d9fca3d69526c`,
JS SHA256 `4b88c81091df6bbc816aea8ee7568240a1976749d8516f77abcbae41540ab004`다.

명령: `node output/7353/closeout/wasm-v2/recheck.mjs`.
기록: 같은 디렉터리 `results.json`(입력 SHA/Chrome 버전/실행 engine), `run.log`.
최종 실행 exit 1은 아래 입력 거부를 반영하며 환경·컴파일 오류가 아니다.
구역0, 96dpi, `omit_final_paragraph_gap` 정책으로 실행했다. 한 구역 결과를 전체 문서의 쪽
번호와 동일하다고 가정하지 않으며, 거부된 문서는 배치·기하 assertion에 도달하지 못했다.

| 입력 / 연결된 기존 실패 | V2 WASM 실제 결과 |
| --- | --- |
| `tests/fixtures/issue7353_host_shape_slots/saved.hwp` — 승인된 정상 대조군 | `table_v2_hosted_section` engine으로 1쪽 렌더 성공. packet/SVG 보존 |
| `samples/hwpx/form-002.hwpx` — SVG 1건 | `body page-number declaration with table controls` 거부 |
| `samples/hwpx/issue_157.hwpx` — SVG 1건 | `table host requires resolved anchor/line/margin policy` 거부 |
| `samples/복학원서.hwp` — SVG 1건 | `host control or multiple tables on one paragraph` 거부 |
| `samples/exam_kor.hwp` — SVG 1건 | `master-page content requires host admission` 거부 |
| `samples/table-in-tbox.hwp` — #6699 기하 2건 | `stored TAC carrier requires unambiguous intact rows` 거부 |
| `samples/143E433F503322BD33.hwp` — #4961 참고 입력 | `section decoration/grid requires host admission` 거부. V2에는 동등한 font trace API도 없음 |

**판정**: 기존 8건 중 조판 출력 6건은 V2 입력 거부로 원래 assertion **미검증**, 폰트 trace 1건은
동등한 V2 API 부재로 **미검증**, CLI 저장/재로드 1건은 조판 엔진 전환 검사에 **비해당**이다.
즉 실패 관련 6개 입력이 거부됐고 기존 8건 중 해결 확인은 0건이다. V2가 기존과 같은 좌표/해시
오류를 냈다는 의미도 아니다. 정상 대조군 성공은 WASM V2 호출이 실제 실행된 증거이며,
전체 V2 무회귀·Studio 전환 완료·새 시각 PASS로 확대하지 않는다.

현재 확인으로는 단순 기본값 전환만으로 해당 원본의 V2 조판을 검증할 수 없다. 확인된 미지원
조건을 임의로 해제하거나 원본을 수정하지 않았고, 이번 마감 범위에 새 구현을 자동 편입하지 않는다.
제품 소스/테스트 계약/기준값/ignore/원격 상태는 변경하지 않았다.

### 2026-09-30 — rhwp-studio에 실제 V2 WASM 읽기 경로 연결

작업지시자의 추가 요청에 따라 진단 runner가 아닌 Studio 시작점에서 V2 경로를 선택하도록
연결했다. `index.html → bootstrap.ts → table-v2-studio.ts → TableV2Session →
HostedSectionV2 → renderPage → SVG DOM` 순서다. `table_v2_hosted_section` engine과
페이지 인덱스를 확인한 출력만 표시한다. V2 모드에서는 Legacy `main.ts`를 import하지 않아
기존 DocumentCore/편집/자동 저장 초기화가 실행되지 않는다. 기존 폰트 로더를 재사용하며 SVG의
fallback family 목록을 개별 글꼴 이름으로 전달한다. 조판 Rust 및 WASM 바이너리는 변경하지 않았다.

**사용 방법**:

```bash
cd /home/edward/mygithub/rhwp-task-7353/rhwp-studio
npm run dev:v2 -- --host 127.0.0.1 --port 7735 --strictPort
```

`http://127.0.0.1:7735/?typeset=v2`에서 문서 열기로 HWP/HWPX를 선택한다. 이 dev 모드에서는
루트 주소도 V2가 기본이다. 일반 `npm run dev`/production의 기본 편집 경로는 Legacy를 유지하며,
`?typeset=v2`로 명시적 선택이 가능하다. 같은 서버의 `?typeset=legacy`는 기존 편집기를 연다.

**정확한 범위**: V2가 조판한 SVG의 읽기 전용 표시, 구역 선택, 구역 내 쪽 이동, 배율 조절이다.
전체 문서가 아니라 선택한 구역만 표시하며 UI에도 이를 명시했다. V2 편집·저장·CanvasView 연결은
아직 없다. 미지원 입력은 원래 오류를 표시하고 이전 페이지를 제거한다. 자동 Legacy 우회나
거부 조건 해제는 하지 않았다. 이 연결은 앞 절의 실패 8건을 해결했다는 의미가 아니다.

검증 source는 `1f28344b673b66911d14e0c1f3d8e35444b8a308` 위의 이번 Studio 변경이며,
WASM/JS SHA는 앞 절과 동일하다. 증적은 `output/7353/closeout/wasm-v2/`에 보존한다.

- `npm run build`: TypeScript 검사 및 Vite/PWA production 빌드.
- `npm test`: 기존 프런트엔드/편집기 테스트 및 신규 V2 모드·세션·오류·폰트 목록 계약.
- `VITE_URL=http://127.0.0.1:7735 node e2e/table-v2-reader.test.mjs --mode=headless`:
  실제 파일 선택 UI에서 WASM 실행, 미지원 오류/빈 화면, 정상 재열기/동일 파일 재선택,
  잘못된 구역 선택 후 복구, 2쪽 앞뒤 이동/내용 소속, V2에서 Legacy 미초기화,
  명시적 Legacy 편집기 초기화를 검사한다.
- 정상 입력 `tests/fixtures/issue7353_host_shape_slots/saved.hwp`, 2쪽 입력
  `tests/fixtures/issue7353_body_frame_review/portrait-saved.hwp`, 거부 입력
  `samples/hwpx/form-002.hwpx`. 2쪽 내용 소속의 기대값은 정상 한컴 저장본 README에서 가져왔다.
- 로그 `studio-{build,tests,e2e}.log`, 실행 결과 `studio/result.json`, 캡처
  `studio/{approved-control,page-two,unsupported-input}.png`. 캡처는 Studio 연결·상태 표시 확인용이며
  새 한컴 피델리티 판정이나 Native/fresh WASM 조판 수정 sweep을 주장하지 않는다.

최종 결과: production 빌드 PASS, 프런트엔드 테스트 **1,767 PASS / 0 FAIL / 2 SKIP**
(신규 집중 계약 6건 포함), 실제 Chrome E2E의 위 7개 흐름 PASS. 최종 캡처 3장을 직접 열어
정상 셀/문단 표시, 두 번째 쪽 내용, 오류 시 이전 출력 제거를 확인했다. 최초 E2E에서 글자별
SVG 노드 사이의 XML 개행을 본문으로 읽은 assertion을 수정해 `svg text` 내용만 결합했다.
제품 조판 결과나 fixture 기대값을 변경한 것은 아니다. 서비스 HTTP 200도 확인했다.

이 변경은 로컬에만 보존하며 원격 push·devel 병합·버전 변경은 하지 않는다.

### 2026-10-01 — Studio 확인 후 로컬 인계 기준점 보존

작업지시자가 Studio 연결을 확인하고 다음 절차를 승인했다. 이번 묶음은 H4의 로컬 보존이며,
새 조판 기능 구현이나 전체 회귀 반복이 아니다. 이전 절의 Studio 코드·검증 기록과 V2 경로
재검사 기록을 task 브랜치에 함께 커밋하고, 동일 commit에 로컬 `refactor/0.9.0` 브랜치를
생성한다. 현재 작업 worktree는 `task_m100_7353`에 유지한다.

- `1f28344b6` 이후 Rust/Cargo/정식 Rust 회귀 source 변화 없음. 이전 빌드·검증의 한계를 그대로
  유지하며 같은 검사를 반복하지 않았다. Studio의 1,767 PASS/2 SKIP 및 실제 브라우저 7개 흐름
  결과도 코드 변경 없이 재사용했다. 사용자 확인을 모든 문서 지원/전체 회귀 PASS로 확대하지 않는다.
- 커밋 전 `python3 scripts/check_e2e_manifest.py`에서 기존 HEAD의
  `canvaskit-cropped-contain.test.mjs`, `probe-flow-input-latency-issue3794.mjs` 미등록 2건을
  확인했다. 실제 파일·npm/CI 배선을 대조해 목록 행만 추가했다. 실행 코드 변경이나 해당 2개
  테스트의 재실행은 없으며, 최종 목록 검사는 **138개 파일/138개 행 PASS**다.
- 신규 V2 소스/테스트와 관련 문서만 명시적으로 stage한다. `.agents/skills/`, `.codex/`,
  환경 설정·비밀값·생성 빌드/로그/PNG 및 다른 worktree의 변경은 커밋하지 않는다.
- H1/H3 완료, H2 전체 회귀 8건 미충족, H4 로컬 보존 완료/원격 인계 보류를 구분한다.
  V2 입력 거부를 기존 실패의 해결로 세거나 baseline/golden/ignore를 변경하지 않았다.

다음 승인 대상은 **남은 회귀 실패를 명시한 WIP 통합 브랜치의 원격 게시 여부와 인계 조건**이다.
현재 계획의 H2 인계 차단을 임의로 해제하지 않는다. 원격 게시 승인 시 `refactor/0.9.0`의 목적은
0.9.0 후속 개발의 공유 기준점이며, devel 병합·PR 생성·0.9.0 릴리즈·#7353 종료를 뜻하지 않는다.

### 2026-10-01 — 승인된 전용 원격 브랜치 게시, CI 미실행

작업지시자가 게시를 승인하고 CI가 동작하지 않도록 요청했다. 게시할 tree의 workflow 22개를
확인했다. `CI`와 release는 `v*` 태그 push, Pages는 main, 이슈 종료/실행시간 갱신은 devel,
Oracle advisory는 `task_m100_*` 및 해당 workflow 경로 변경에만 push로 동작한다.
`refactor/0.9.0`은 어느 push 필터에도 해당하지 않는다. PR·태그·수동 workflow 실행은 하지
않으므로 workflow 수정, 저장소 Actions 비활성화, `[skip ci]` 커밋 없이 게시했다.

명령: `git push --set-upstream upstream refs/heads/refactor/0.9.0:refs/heads/refactor/0.9.0`.
`edwardkim/rhwp`에 새 브랜치 생성 성공. 첫 게시 구현 SHA는
`6873720638b891f7db65b0c15e32b9c76121e287`이며 `git ls-remote`와 일치했다.
게시 후 branch Actions 조회는 `total_count: 0`, 해당 SHA Checks도 `total_count: 0`이었다.
이 결과는 CI PASS가 아니라 **CI 실행 없음**이다. 본 기록은 제품 코드가 바뀌지 않은 문서 전용
후행 커밋으로 같은 브랜치에 보존하고, 최종 head와 Actions를 다시 확인한다.

최신 devel `02530b9ed567a44663edb26c65fb565c4a79f00d`은 fetch만 했으며 승인 기준점에
병합하지 않았다. 기존 로컬 검증의 base `0e8fd49fb868da0d47ac1294dcbbda81f0211233`와 구별한다.
H4의 개발 중 원격 공유는 완료했지만 H2 회귀 8건 미충족, V2 미지원 입력, 편집/저장 미연결은
해결되지 않았다. 후속 PR 생성이나 향후 workflow 필터 변경 시 CI 실행 여부는 별도 확인해야 한다.
devel/main, 저장소 CI 설정, required checks, baseline/ignore, 버전 및 이슈 상태는 바꾸지 않았다.

<a id="090-resume-triage"></a>

### 2026-10-01 — 0.9.0 재개: 기존 실패 8건의 제한 조사

작업지시자의 계속 진행 요청에 따라 `refactor/0.9.0`에서 시작했다. 조사 시작 head는
`3b8eab0f8b4d5b21ddac6858d9d026d929b26f36`, tracked 변경은 없었다. 미추적 도구 설정은
작업 대상에서 제외했다. 확보된 여유 공간은 277GiB다. 새 worktree/전체 빌드는 만들지 않았다.
원격 push·devel merge·테스트 기대값 변경도 하지 않았다.

#### 실제 재현: #3528은 캡션 유실이 아니라 저장 거부

기존 검증 바이너리 `/home/edward/mygithub/rhwp/target/pr-review/release-test/rhwp`
(SHA256 `2cef88814d8629807645dd9871193ba5c3692ccb31d90c7362cd8f53d36aeee1`)로 실행:

```sh
rhwp convert samples/issue1891_external_bindata_link.hwpx \
  output/7353/closeout/triage-090-ZVmeuA/caption.hwp --verify
```

exit **1**, 출력 HWP 생성 없음. stdout/stderr는 위 디렉터리의 `caption.stdout`,
`caption.stderr`에 보존했다. 오류는
`HWP5 export of breakCellSeparateLine is not implemented`다. 재사용 바이너리의 진단이며
새 head 전체 테스트 실행으로 보고하지 않는다.

입력 ZIP의 `Contents/header.xml`에는 `borderFill id=15`의 해당 속성이 참이고,
`Contents/section0.xml`의 실제 표 5개가 이를 참조한다. 미사용 스타일만의 문제가 아니다.
입력 속성 → `parser/hwpx/header.rs` → `BorderFill.break_cell_separate_line` →
`serializer/cfb_writer.rs::serialize_hwp_inner`의 조기 오류로 연결된다.
`git log -S 'HWP5 export of breakCellSeparateLine'`로 유입 커밋 **bf9cc2308**을 확인했다.

기존 `tests/issue_3528_nested_caption_boundary.rs`는 subprocess 상태를 검사하기 전에,
파일이 없으면 `after=[]`로 만들어 캡션 목록 `[3]`과 비교한다. 따라서 기존 실패 메시지는
실제 원인인 저장 거부를 가린다. 새 속성 보존 계약
`issue_7353_table_v2_split_line_property::unsupported_hwp_export_does_not_silently_drop_enabled_property`
는 바로 이 거부를 요구한다. **기존 변환 지원의 제한이 #7353에서 새로 생긴 것**이며,
캡션 경계 파서가 다시 깨졌다는 증거는 아니다. 매핑 근거 없이 속성을 버리거나 테스트를
ignore 처리하지 않았다. 이전 바이너리의 성공 비교 실행은 하지 않았으므로 그 실행 결과는 주장하지 않는다.

#### 나머지 7건: 가로 배치/해시 원인 후보 묶음

보존된 `output/7353/closeout/svg-mismatches.tar.gz`의 각 golden/actual SVG를 XML로
파싱해 같은 순서의 노드·텍스트·속성을 비교했다. 새 렌더링이나 시각 통과 판정은 아니다.

| 사례 | golden/actual 노드 수 | 달라진 속성 수 |
| --- | --- | --- |
| form-002/page-0 | 956 / 956 | x 173, transform 21 |
| issue-157/page-1 | 523 / 523 | x 41 |
| issue-677/bokhakwonseo-page1 | 1,010 / 1,010 | x 78 |
| issue-617/exam-kor-page5 | 1,905 / 1,905 | transform 879, x1 1, x2 1 |

노드 종류·텍스트·세로 좌표의 차이는 검출되지 않았다. 이 검사로 글립/겹침의 시각적 정확성을
입증하지 않는다. #6699 두 실패 역시 기존 로그에서 이미지와 텍스트의 가로 간격/시작 좌표
assertion이므로 같은 조사 묶음으로 분류했다. #4961은 `layoutHash` 불일치이며, 동일 원인인지
또는 환경 차이인지는 아직 미검증이다.

분기 기준 `7a95e46e0` 대비 `layout/paragraph_layout.rs`에는 후행 공백·마지막 텍스트 run의
측정을 `estimate_text_width_exact`로 바꾼 공통 코드가 있다. 유입은 **cc13f573a** 및
**780de8512**다. 이 함수들은 V2 전용 디렉터리 밖의 공통 배치 소비 지점이다. 실제 출력에 대한
변경별 A/B 실행은 하지 않았으므로 일곱 실패의 확정 원인으로 승격하지 않는다.

현재 devel 비교용 `0e8fd49fb`는 원래 분기 기준 `7a95e46e0`과 다르다. #4961 기대 hash도
그 devel과 task 브랜치에서 다르므로, 비교 base/폰트 환경을 고정하지 않고 devel의 동일 실패나
새 회귀라고 단정할 수 없다. 기존 8건은 여전히 실패 상태로 보존한다.

이번 묶음 종료: **1건 직접 원인 확인, 6건 가로 좌표 변화 분류, 1건 hash 추가 확인 필요**.
수정 완료 0건, 새 전체 회귀 실행 0회. V2 미지원 입력을 PASS로 세지 않았다. 다음 작업 순서는
[구현계획 5.3](../plans/task_m100_7353_impl.md#53-2026-10-01--refactor090-작업-재개)에 연결했다.

#### 후속 승인 실행 — 공통 소수 폭 변경의 원인 분리 완료

작업지시자가 위 다음 작업을 승인했다. 제품 코드와 기대값은 그대로 두고, 같은 head의 임시
detached worktree `/tmp/rhwp-090-width-odtNKR`에서 집중 실행했다. 고정 target은 기존
`/home/edward/mygithub/rhwp/target/pr-review`를 재사용했으며 Cargo 동시 실행은 하지 않았다.
`CARGO_BUILD_JOBS=2`, nextest test-threads=2였다. 정식 generated suite 준비 후 현재 코드의
관련 계약 **27건: 20 PASS / 7 FAIL**, 빌드 6분24초/테스트 0.920초로 기존 실패를 재현했다.
이는 전체 회귀 재실행이 아니다. 명령·결과는 아래 증거 디렉터리에 보존했다.

대조 코드는 임시 worktree에만 넣었다. 기존 테스트 source와 assertion을 수정하지 않고,
동일 파일들을 별도 진단 harness로 묶어 세 계산만 독립적으로 전환했다. 이 방식은 생성 suite
전체를 조건마다 다시 컴파일하지 않기 위한 진단이며, 정식 suite 제출을 대체하지 않는다.
진단 스위치가 꺼진 결과는 원래 27건의 결과와 같았고, 추가한 양쪽 정렬 계약도 통과했다.

- `tail`: 마지막 TAC 뒤 텍스트 조각의 `seg_w`를 정수 반올림으로 복원.
- `align`: 가운데/오른쪽 정렬에서 제외할 후행 공백 폭을 정수 반올림으로 복원.
- `justify`: 양쪽 정렬의 후행 공백을 각 run 스타일 대신 마지막 run 스타일·정수 폭으로 복원.

| 이전 계산으로 바꾼 부분 | 기존 실패 7건 중 통과로 바뀐 검사 | 정상 계약에 생긴 실패 |
| --- | --- | --- |
| 없음 (현재 코드) | 없음 | 없음 |
| tail만 | #6699 2건 | TAC tail 96/192dpi 4건 |
| align만 | 복학원서 SVG 1건 | 가운데/오른쪽 정렬 1건 |
| justify만 | #157 SVG, 국어 시험 SVG, #4961 hash 3건 | TAC tail 4건 + 양쪽 정렬 1건 |
| align + justify | SVG 4건과 #4961 hash 1건 | 위 정렬 2건 + TAC tail 4건 |
| 세 계산 모두 | **기존 실패 7건 전부** | **정상 계약 6건** |

나머지 두 조합 `tail+align`, `tail+justify`도 실행해 form-002가 align/justify 양쪽 변경을
소비함을 확인했다. 3개 스위치의 8조합을 검사한 것이며 새 샘플 거부 지점으로 범위를 넓히지
않았다. 기본 대조 28건과 별도 가운데/오른쪽 정렬 1건을 합쳐 조건별 **29개 고유 검사**다.
현재 계산은 22 PASS/7 FAIL, 모두 복원하면 23 PASS/6 FAIL이다. 통과 수가 1개 늘어났다는
이유로 후자를 채택하지 않는다. 기본 대조 빌드 4분58초 뒤 각 조건 실행은 약 1초였다.

**독립 계약과 실제 소비 지점.** `text_measurement::estimate_text_width_exact`는 기존
메트릭·탭 처리를 유지하고 최종 반올림만 제거한다. `estimate_line_run_widths`의 측정 →
`trailing_space_width_after_last_inline_object`/`justified_trailing_space_width`의 후행 폭 →
`x_start`/`compute_line_extra_spacing` → `emitted_run_layout_positions` → 최종 TextRun bbox와
다음 `x += seg_w`까지 연결된다. exact kerning이 실제 적용되는 경로는 positions 끝값으로
덮어쓸 수 있으며, 이를 모든 경로가 자동 공유한다고 가정하지 않았다.

기대값을 새 구현의 bbox로 정하지 않고 기존 정식 계약의 독립 소비자/정렬 불변식과 대조했다:

- tail 복원 시 paragraph212/192dpi에서 bbox **365**, backend replay **365.40761904761905**;
  paragraph145/96dpi에서 bbox **53**, replay **53.444379999999995**로 불일치한다.
  다른 두 사례는 `text preview run outside occupied line`으로 실제 V2 수용이 실패한다.
- align 복원 시 Right/11.3/단일 스타일에서 가시 끝점 **224.875**, 줄 오른쪽 끝 **225**다.
  기대값은 반올림 이전 snapshot 숫자가 아니라 줄의 오른쪽 정렬 불변식이다.
- justify 복원 시 마지막 가시 글자의 replay 끝점이 저장 줄 오른쪽 끝에 닿는 계약이 실패한다.
  #4961은 같은 바이너리·입력·환경에서 이 스위치로 실패/통과가 바뀌므로, 이번 hash 차이를
  폰트 설치 환경 탓으로 분류할 근거는 없다.

**판정:** 기존 실패 7건의 원인은 공통 폭 처리의 세 변경으로 확인됐다. 단순 되돌림은 측정/배치
일치와 정렬 계약을 깨뜨리므로 제품에 반영하지 않는다. 다만 이 실행은 합성/기존 계약 검사이며
4개 SVG의 한컴 피델리티 승인이나 golden 갱신 승인으로 승격하지 않는다. #6699 기존 한컴 PDF의
첫 글자 x=250.68pt도 확인했으나, px 환산/글꼴 차이를 고려하지 않은 단일 수치로 기준을 바꾸지
않았다. 독립 출력과의 영향 영역 비교 및 기대값 변경 근거 검토가 남는다.

증거: `output/7353/closeout/triage-090-ZVmeuA/`의 `width-A.log`,
`width-{exact,tail,align,justify,all,tail-align,tail-justify,align-justify}.log`,
`alignment-{exact,tail,align,justify,all}.log`, `run-width-matrix.sh`,
`width-diagnostic.patch`, `width_matrix.rs`. 진단용 환경변수 `RHWP_WIDTH_DIAGNOSTIC`는
제품 브랜치에 추가하지 않았다. 기존 WASM/Studio 배포물도 바꾸지 않았다.

이번 원인 분리 묶음은 완료한다. 제품의 현재 실패는 여전히 8건이며, 기존 #3528 저장 거부와
가로 배치 7건을 각각 저장 지원 계약/기대값 근거 검토로 넘긴다. 전체 회귀·Clippy·fresh WASM·
새 Visual Sweep은 실행하지 않았고 제출 완료를 주장하지 않는다. 작업 브랜치의 Rust/Cargo/
정식 테스트/golden/ignore 변경은 없다.

실험 종료 후 새로 만든 임시 worktree만 제거해 약 4.4GiB를 회수했다(여유 공간 276GiB).
기존 두 worktree와 WIP는 보존했다. 실험은 기록한 head·진단 패치·harness로 재구성 가능하다.
고정 target의 진단 빌드는 출시 산출물이 아니며 다음 제품 검증은 정상 source에서 빌드한다.

#### 2026-10-01 — 실패 계약의 메인테이너 시각 재판정 준비

작업지시자는 기존 테스트 충족을 위한 규칙 없는 상수 조정을 지적하고, 실패 사례를 직접
시각 판정한 뒤 성공 여부를 정정하도록 지시했다. 이번 작업은 판정 자료 준비이며 제품 코드,
assertion, golden, hash, ignore는 수정하지 않았다. 기존 8건의 실패 상태도 아직 유지한다.

판정 진입점: [로컬 비교 화면](../../output/7353/closeout/maintainer-visual/index.html).
원본·PDF 링크, Native/WASM review·compare·standalone overlay, 기존 golden/actual 비교,
대상 영역과 판정 메모를 한 화면에 모았다. 판정은 전부 `미판정`으로 시작하며 브라우저의
선택은 자동으로 테스트에 반영되지 않는다. JSON 내려받기로 판정 범위와 이유를 보존할 수 있다.

| 실패 묶음 | 원본 | 화면 쪽 | 확인 범위 |
| --- | --- | --- | --- |
| #6699 2건 | `samples/table-in-tbox.hwp` | 1 | 하단 로고 뒤 공백·첫 글자·가운데 정렬. 동일 페이지 좌표 확대도 제공 |
| form-002 SVG | `samples/hwpx/form-002.hwpx` | 1 | 셀 본문 정렬·줄바꿈·외곽과 앞뒤 내용 |
| #157 SVG | `samples/hwpx/issue_157.hwpx` | 2 | 자리차지 표·위임장·본문 간격과 겹침 |
| #677 SVG | `samples/복학원서.hwp` | 1 | 접수증·점선·가운데 정렬·인라인 표 |
| #617 SVG | `samples/exam_kor.hwp` | 6 | 16번 보기 셀 여백과 줄 구성. 테스트의 page5는 0-based |
| #4961 hash | `samples/143E433F503322BD33.hwp` | 1 | 본문 양쪽 정렬. 아래 기준 출력 제한에 따라 원본 직접 판정 필요 |

검증한 source head는 `06cf3c7be`다. 정상 checkout에서
`CARGO_BUILD_JOBS=2 cargo build --locked --bin rhwp --target-dir /home/edward/mygithub/rhwp/target/pr-review`
를 실행해 성공했다. `scripts/visual_sweep.py`로 위 6개 영향 페이지를 Native/WASM 각각
120dpi로 새 캡처했으며 12회 모두 exit 0이다. 전체 회귀나 전체 페이지 피델리티 검증은 아니다.
WASM은 build head `1f28344b673b66911d14e0c1f3d8e35444b8a308`의 기존 pkg를 재사용했다.
현재 head까지 `src/`, `crates/`, `Cargo.toml`, `Cargo.lock` 차이가 없고 JS/WASM 해시가 기존
검증 기록과 같음을 확인했다. 따라서 새 캡처이지 fresh WASM 재빌드라고 보고하지 않는다.

**경로 구분:** 이번 출력은 실패 테스트와 같은 기존 전체 문서 `HwpDocument` 경로다.
`HostedSectionV2`가 이 6개 입력을 거부한 기존 결과는 유효하며, V2 fallback이나 수용 조건
변경은 하지 않았다. 여기서의 통과는 V2 전체 문서 지원 완료·Canvas·편집/저장 통과가 아니다.

Native/WASM 페이지 PNG는 6건 모두 byte-identical했다. 대표 review들을 직접 열어 확인했다.
이는 경로 간 출력 일치일 뿐 한컴 피델리티 통과가 아니다. #6699 굵기, form-002 본문 위치,
#677 상단 로고·영문 줄 등 보이는 차이는 화면에 남겼다. 이를 자동으로 폰트 문제나 기존 차이로
면책하지 않고 작업지시자의 범위별 판정을 기다린다.

**#4961 제한:** 보유 `pdf/hwpx/143E433F503322BD33.pdf`는 HWPX 대응 출력이다. 같은 상공신문
내용은 확인했지만 차트 형상과 단 흐름에 큰 차이가 있어 해당 HWP의 확정 정답지로 사용할 수
없다. 화면에는 참고 비교라고 명시했으며 원본 HWP를 한컴에서 직접 판정하거나 같은 HWP에서
생성한 정상 PDF가 필요하다. 시각 통과만으로 폰트 trace의 선택·provenance 계약을 대체하지 않는다.

**#3528 분리:** `samples/issue1891_external_bindata_link.hwpx`의 HWP5 저장은
`breakCellSeparateLine` 미지원으로 거부되어 결과 파일이 없다. 기존 오류 로그를 연결했으며,
원본의 시각 통과로 저장 후 캡션 보존을 통과시킬 수 없다. 지원 범위/직렬화 계약 판단으로 남긴다.

증적은 `output/7353/closeout/maintainer-visual/`의 `manifest.json`(head·입력/PDF·binary·pkg
해시와 실행 명령), 각 실행 로그와 `snapshots/manifest.json`에 있다. snapshot 4쌍은 보존한
`svg-mismatches.tar.gz`에서 추출해 같은 Chrome webfont 정책으로 raster화했으며 정답지가
아니라 이전 rhwp 출력과의 변경점 비교로 표시했다. 브라우저에서 이미지 23개 로딩과 링크 존재,
6개 판정의 `미판정` 초기 상태를 확인했다(`browser-check.json`).

메인테이너 판정 뒤 승인된 영역·허용 차이·규칙을 근거로 계약을 수정한다. 현재 관측 좌표로
상수만 치환하거나, 시각 판정을 받기 전에 실패를 PASS/ignore로 바꾸지 않는다.

#### 2026-10-01 — 메인테이너 시각 판정 접수 및 #3528 확인 파일

작업지시자가 위 6개 문서 모두 시각 통과로 판정했다. 이는 가로 배치 관련 자동 실패 7건의
시각 수용 근거이며 자동 테스트 실행 결과가 PASS로 바뀌었다는 뜻은 아니다. 테스트 계약의
정정 시 아래 허용 범위를 연결하며, 렌더러를 기존 상수에 맞추는 보정은 하지 않는다.

| 대상 | 판정 | 메인테이너 허용 사유 |
| --- | --- | --- |
| #6699 1쪽 (2개 검사) | 통과 | 폰트의 차이 |
| form-002 1쪽 | 통과 | 사각형 입력 박스와 폰트 차이. 이 정도 조판 차이는 허용 |
| #157 2쪽 | 통과 | 이 정도 조판 차이는 허용 |
| #677 1쪽 | 통과 | 이 정도 조판 차이는 허용 |
| #617 6쪽 | 통과 | 이 정도 조판 차이는 허용 |
| #4961 1쪽 | 통과 | OLE가 다음 단으로 이동해 생긴 빈 공간을 한컴은 후속 텍스트로 일부 채우지만, rhwp는 비워 둠. 이전 legacy에서도 수용한 차이 |

판정 원문은 `output/7353/closeout/maintainer-visual/maintainer-decisions.json`과 비교 화면에
반영했다. #4961 참고 PDF의 출처 한계는 유지하되, 이를 이유로 메인테이너가 수용한 조판을
반복 보류하지 않는다. V2 admission, Canvas, 편집/저장이나 trace의 비시각 의미 계약을 이번
시각 승인에 포함하지 않는다. 아직 Rust/test/golden/hash/ignore 변경은 없다.

#3528은 생성 파일을 한컴편집기로 직접 열면서 해결하는 방식으로 진행하도록 지시받았다.
현재 HWP5 변환은 여전히 `breakCellSeparateLine` 미지원으로 출력 전 거부된다. 이 속성을
몰래 버리거나 거부 분기를 제거하지 않았다. 대신 속성을 표현할 수 있는 HWPX 재직렬화로
한컴 확인용 산출물을 준비했다. **HWP5 실패 테스트의 성공 산출물이 아니다.**

- 원본: `samples/issue1891_external_bindata_link.hwpx` (변경 없음).
- 생성본: [issue3528-rhwp-resaved.hwpx](../../output/7353/closeout/maintainer-visual/issue3528-rhwp-resaved.hwpx).
- 명령: `/home/edward/mygithub/rhwp/target/pr-review/debug/rhwp export-hwpx samples/issue1891_external_bindata_link.hwpx output/7353/closeout/maintainer-visual/issue3528-rhwp-resaved.hwpx --verify`.
- 실제 파일 175,969 bytes 생성. ZIP 무결성 검사 통과, `breakCellSeparateLine="1"` 보존 확인.
- `--verify` **exit 3**: section0 문단176/444에서 `pageNumPos` 2개가 1개로 줄어든 IR 차이 2건.
  자체 재파싱은 실행됐지만 저장 동등성이나 한컴 정상 열기 통과로 보고하지 않는다.
- 로그: 같은 디렉터리 `issue3528-export.stdout`, `issue3528-export.stderr`.

#### 2026-10-01 — 승인된 6개 문서의 계약 변경

메인테이너의 위 시각 승인 및 계약 변경 지시에 따라 렌더러 수정 없이 반영했다.
form-002/157/677/617 SVG 4개만 현재 승인 출력으로 갱신했다. #6699는 PDF 원점
허용오차를 유지하면서, 과거 x/공백 폭 상수를 원문 공백의 메트릭 replay와
그림→공백→텍스트의 연속 배치 관계로 교체했다. 내부 tracking 7개 간격 검사도 유지했다.
#4961은 전체 advance를 포함하는 고정 해시를 제거하고 각 fixture 반복 실행 동일성,
글꼴 선택/누락/대체 계보, HWP/HWPX 동등성 검사를 유지했다. missing-face 프로필도
동일 계약을 적용하며 이 문서의 새로운 시각 승인을 주장하지 않는다.

검증은 공유 target/pr-review에서 파생 suite 006/020/025/011의 해당 모듈만 실행:
Native 7+2+6+8=23 PASS. SVG 갱신 후 UPDATE_GOLDEN 없이 다시 통과했다.
`node --experimental-strip-types --test scripts/tests/font_decision_trace_e2e.test.mjs`도 통과.
로그는 `output/7353/closeout/maintainer-visual/approved-*.log`에 보존했다.
기존 WASM은 렌더러 변경이 없는 이 계약 검증에 재사용했으며 fresh build로 표기하지 않는다.
전체 회귀 완료 주장이 아니고, 다음 범위는 #3528 HWP5 속성 저장 구현이다.

후속은 생성 HWPX의 한컴 열기·캡션/내부 표·쪽번호 확인 및 HWP5 미지원 속성의 저장 계약
해결이다. 시각 승인 6건과 저장 문제의 미해결 상태를 분리해 유지한다.
