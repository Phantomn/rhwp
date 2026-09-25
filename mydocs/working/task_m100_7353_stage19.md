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
