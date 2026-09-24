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

## 현재 작업과 남은 경계

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
