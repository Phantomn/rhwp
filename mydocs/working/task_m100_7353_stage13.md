# Task #7353 — V2 온전한 셀의 실선 테두리

- 선행 절편: `6ca7e7211` ([stage12](task_m100_7353_stage12.md)).
- 상태: 이번 절편의 구현·선택 회귀·Native/fresh WASM 검증 완료. R3 전체 완료는 아님.
- 범위: 나누지 않음/행 단위 분할에서 양의 높이를 가진 온전한 셀의 실선. 제목 반복·colspan·중첩 경로 포함.
- 비범위: 셀 내부 컷의 테두리, 상이한 공유선의 우선순위, 표 자체 테두리, 복합선/zone.

## 근거와 공통 결과

한 셀의 네 변은 그 셀의 확정된 사각형 경계다. 겹치는 동일 공유선은 한 번만 출력하고,
한쪽에만 선이 있으면 그 선을 보존한다. 상이한 두 선의 충돌은 임의의 승자를 정하지 않는다.
온전한 셀만 수용하므로 페이지 컷에 가짜 위/아래 테두리를 만들지 않는다. CellBreak에 테두리가
있으면 실제 fit 여부로 지원 여부가 바뀌지 않도록 미지원으로 거부한다.

굵기의 독립 근거는 [#6913 단일 변수 실측](../../samples/issue6913/README.md)의
16단계600dpi 격자다. 일반 paint의 `border_width_to_px`를 DPI 비율로 소비한다.
Legacy 표의 측정·분할·공유선 우선순위 함수는 호출하지 않는다.

`해소된 셀 서식 → immutable CellBorders → 확정 TablePlacement → 토폴로지 경계 합집합 → LineNode`
경로다. 각 조각의 실제 행 순서(반복 제목 포함)와 논리 열 경계를 사용한다. 같은 경계의
좌표는 확정 셀 시작점으로 통일해 부동소수점 덧셈 순서로 선이 중복되지 않게 한다.
숨겨진 colspan 내부 경계를 새로 만들지 않는다. 배경/내용 뒤에 선을 그려 후속 셀 배경이
공유선을 덮지 않게 한다. 선은 중심 정렬이므로 잉크는 경계에서 반 획만큼 확장되지만
셀/표의 흐름 높이는 바꾸지 않는다.

분할·요구/예약 높이 계산은 무변경이다. 새 paint의 오류는 페이지 commit 전에 반환한다.
합성 fixture와16단계 독립 굵기값으로 위치/중복/두께/이어받기를 검사하고 Native/fresh WASM의
실제 출력·픽셀을 대조한다. 한컴 PDF와 해당 합성 전체 조판의 일치로 확대 해석하지 않는다.

## 실제 호출 경로와 비적용 범위

- `text_ir.rs::bind_paint → CellBorders::prepare`: 셀의 1-based borderFill을 불변 서식으로
  연결한다. 각 재귀 자식은 자기 표의 서식/기하를 소유한다. 원본 효과는 기존
  `TablePreviewSession::from_document → decoration::validate_source`에서 검증한다.
- `fragment.rs::fit_rows`의 Never/BetweenRows 요구 높이·누적 예약 → `fit_with_header`의
  제목/본문 원자적 수용 → 확정 `CellPlacement` → `TextPaint::build_node → CellBorders::append`.
  paint에는 다른 높이/좌표 재측정, clamp, 컷 변경이 없다. 제목 뒤의 원본 행 번호가 뛰어도
  조각 내 물리 순서로 공통 경계를 결정한다. 가로 병합의 일부만 겹치는 공유선도 합집합으로 처리한다.
- 공유선 충돌은 `union`에서 명시적 오류다. `TextFragment::append_to`가 성공한 뒤에만
  `session.rs::next_page`가 cursor/쪽 번호를 commit한다. `export.rs::next_page_json`도
  후보 세션에서 성공한 결과만 반환한다. 첫 조각 실패와 이미 한 조각을 반환한 뒤의 실패를 구별해 검사한다.
- WithinCells에 테두리가 있는 경우, rowspan, 행 간격, 제목 일부 셀만 지정, 복합선은 비적용이다.
  부모 WithinCells가 온전한 자식 RowBreak를 포함하는 경우에는 자식 선만 출력한다.
  직접 `from_flow_rows`로 만든 셀의 자체 테두리는 여전히 없고 준비된 자식의 paint는 보존된다.
- 선이 있는 표의0높이 행은 퇴화 토폴로지로 명시적으로 거부한다. 무테0높이 행도 위/아래 행의
  선을 같은 물리 좌표에 놓을 수 있으므로 선을 가진 셀만 검사하지 않는다. 무테 표의0높이 행은
  기존대로 허용하며 높이를 임의로 늘리지 않는다.
- 배경은 기존 bounds를 채우고 선은 중심 정렬한다. 따라서 선의 잉크 bbox가 셀 경계 바깥으로
  반 획 나가는 것은 의도된 paint 계약이며, 셀 내용의 편집영역 침범과 구별한다.

## 입력·독립 기대값·수정 전 실행

`tests/cases/issue_7353_table_v2_borders.rs` 9개 계약을 추가했다. 정상 생성 HWP나
한컴 생성 PDF를 가장하지 않는 수동 IR → 정식 HWPX serializer/parser 왕복 입력이다.
36px 본문과18px 고정 줄간격,200px 표 너비/두100px 열이 독립 기대값이다.
제목이 두 열을 병합한2행 표는 선12개를 덧그리지 않고 공유 경계를 포함한6개 구간으로 출력한다.
한쪽 셀에만 선이 있으면4개 구간이며 이웃 무테 셀의 외곽선은 만들지 않는다.

16단계 굵기는 #6913 문서의 `[2,3,4,5,6,7,9,12,14,17,24,35,47,71,95,118] /600 inch`를
96/192dpi의 최종 LineNode와 대조한다. helper의 계산 결과를 기대값으로 복사하지 않는다.
분수 DPI에서는 관측한 셀 시작점으로 공유 경계를 하나만 만든다.

증적 폴더는 `output/7353/r13/`, 제품 기준은 `6ca7e7211` + 최종 `source.sha256`이다.
review worktree는 `/home/edward/mygithub/rhwp-review-7353`, 고정 target은
`/home/edward/mygithub/rhwp/target/pr-review`이며 policy base는
`7a95e46e025470a4d7a7b59ad68ec02958bda738`이다.

- 최초 `before-tests.log`는 테스트의 u32/usize 비교 컴파일 오류였다. 결함 검출로 세지 않는다.
- 이를 고친 `before-tests-final.log`: **8개 실행,7 FAIL/1 PASS**. 이전 구현에서 새 지원 입력이
  `V2 decoration supports only solid backgrounds`로 거부됨을 확인했다. 기존 Legacy 결함 재현이
  아니라 V2 지원 범위 확장의 전후 계약이다. 미지원 입력 거부 대조군1건은 이전에도 통과한다.
- 첫 성공 조각 뒤 오류를 재시도하는 assertion은 이후 보강했으므로 그 assertion 단독의
  변경 전 검출은 주장하지 않는다.
- 1차 선택 검사 `selected-tests-final.log`는101건 통과했다. 이후 코드 대조에서 발견한
  0높이 무테 행의 퇴화 경계를 별도 계약으로 추가했다. `zero-row-before.log`는9건 중
  이1건만 실패(명시적 거부 부재), 기존8건은 통과했다. 입증 범위는 합성 직접 IR의 수용 경계다.
  HWPX serializer가0문단 셀을 보존한다는 주장이나 실물 PDF 결함 재현이 아니다.
- 보강 전 Docker 빌드는 wasm-opt 중 중지했다(`docker-wasm.log`). 최종 제품 소스가 바뀌었으므로
  이전 패키지를 증거로 재사용하지 않고 `docker-wasm-final.log`로 fresh build를 다시 실행한다.

## 최종 검증

| 검사 | 결과 / 증적 (`output/7353/r13/`) |
| --- | --- |
| 신규 테두리9 + 기존 V2 90 + Legacy #5301 3 | **102 PASS**, `selected-tests-qualified.log` |
| Native / WASM library Clippy | **PASS**, `clippy-native-final.log`, `clippy-wasm-final.log` |
| integration Clippy | **PASS**, `clippy-tests-final.log`, `clippy-export-final.log` |
| fmt / 고정 base manifest 정책 | **PASS**, `fmt-final.log`, `policy-final.log` |
| Docker release + wasm-opt | **PASS**,8분04초, `docker-wasm-final.log` |
| 실제 Chrome WASM | **26쪽 PASS**, `browser.log`, `browser/manifest.json` |
| frontend 선언 검사 / JS 구문 | **1 PASS / PASS**, `bindings.log` |
| 문서 링크 / diff whitespace | **PASS**, `links-final.log` |

이번에는 합계102건을 단일 nextest 호출에서 선택했다. 실제 target 목록과 filter/명령은
`selected-tests-qualified.log` 첫 부분에 보존했다. 신규 테스트는 최종 크기 기준
`regression_suite_016`, export 대조는`regression_suite_014`에 배정된다. integration Clippy는
016/022를 실행한 뒤 실제 export 배정014도 확인했다. 새 원본은 `tests/cases/`에만 두며,
파생 suite/manifest는 review worktree 전용이다. source-side cfg(test)는 변경하지 않았다.

```sh
node scripts/rust-test-suite-manifest.mjs --prepare
ISSUE7353_EXPORT_DIR=/home/edward/mygithub/rhwp-task-7353/output/7353/r13/fixtures \
  node scripts/run-rust-test.mjs issue_7353_table_v2_borders -- \
  --cargo-profile release-test --target-dir /home/edward/mygithub/rhwp/target/pr-review --no-fail-fast
cargo clippy --locked -p rhwp --lib --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo clippy --locked -p rhwp --lib --target wasm32-unknown-unknown \
  --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo clippy --locked --test regression_suite_016 --test regression_suite_014 \
  --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo fmt --all -- --check
node scripts/rust-test-suite-manifest.mjs --check --base-ref 7a95e46e025470a4d7a7b59ad68ec02958bda738
```

포맷 뒤 prepare한 review worktree에서 수행한다. 위 focused 명령은 신규9건 재현용이며,
최종102건 실행은 기록된 전체 선택 filter를 사용한다. 일반 contributor checkout이나 product에
파생 suite를 stage하지 않는다. 전체 CI/workspace 회귀·Native Skia 검증을 수행한 것으로 세지 않는다.

## Fresh WASM 직접 판독

product worktree에서 실행했다. Studio 기본 패키지나 엔진 선택은 바꾸지 않았다.

```sh
docker compose --env-file .env.docker -p rhwp run --rm wasm
VISUAL_SWEEP_CHROME=/home/edward/.cache/puppeteer/chrome/linux-146.0.7680.31/chrome-linux64/chrome \
  node scripts/verify-table-v2-preview-wasm.mjs --pkg pkg \
  --fixtures output/7353/r13/fixtures --out output/7353/r13/browser \
  --dependencies-root /home/edward/mygithub/rhwp --solid-backgrounds --solid-borders
```

Chrome146.0.7680.31에서 기존19쪽 + 테두리7쪽의 실제 WASM tree/SVG가 Native와 exact 일치했다.
각 backend의 PNG에서 외곽·공유 경계의 어두운 픽셀과 반 획 바깥의 흰색을 검사했다.
독립 기대 좌표는 병합 헤더200px, 본문 두100px 열,18px 높이에서 정했다. 한쪽 셀의4변만
보여야 하는 대조군에서는 이웃 셀에 선을 생성하지 않는 것도 확인한다.
실제 JS class의 명시적 미지원, paint 오류 뒤0 emitted 유지, 독립 세션, 종료도 통과했다.
첫 성공 조각 뒤 paint 오류의1 emitted 유지와0높이 직접 IR 경계는 Native 계약으로 검증했다.

`browser/border-grid-0.review.png`, `border-header-{0,1}.review.png`,
`border-nested-{0,1,2}.review.png`, `border-one-sided-0.review.png` **7장을 직접 열어 판독**했다.
병합 헤더 안에는 잘못된 세로선이 없고, 본문에는 공유 세로선과 수평선이 이어진다.
제목이 반복되는 두 번째 조각에 L2/R2가 이어지며, 중첩 표가 끝난 세 번째 조각에는 선 없이
host/after가 남는다. 셀 배경에 공유선이 가려지는 현상이나 Native/WASM 간 위치 차이는
관찰하지 않았다. fixture는0padding이므로 글자와 선이 가깝다. 실물 한컴 여백/피델리티 판정은 아니다.

기존19쪽은 최종 소스로 새로 캡처했고, stage12에서 직접 판독한 review PNG와 byte-identical이다
(`previous-visual-compare.log`). 옛 캡처를 이번 산출물로 복사하지 않았다. 각 이름의
standalone `*.overlay.png`, `*.native.png`, `*.wasm.png`, SVG/JSON도 보존했다.
RGB overlay는 Native 기준이며 한컴 PDF가 아니다. 합성 출력 통과를 실물 한컴 일치로 승격하지 않는다.

`source.sha256`의28개 구현/테스트/하네스와 review 소스를 대조했고, `pkg.sha256`으로 실행 패키지를
고정했다. WASM SHA-256:
`000c17120ebbc874b0e912dbfaf6b804569499abcdbf8e097ca1ac74b65747cf`.
검증 뒤 제품 코드는 변경하지 않았다.

## 남은 범위와 다음 절편

셀 내부 컷의 테두리 소유·이어받기 경계, 상이한 공유선의 우선순위와 표 자체 선은 아직 미지원이다.
다음 절편은 이 중 분할 셀의 경계 규칙을 독립 기준으로 정리한 뒤 확장한다. 행 높이/페이지 수를
기준 출력에 맞추는 예외를 넣지 않는다. zone/복합 채움·rowspan·저장 LineSeg·TAC/어울림·문서 전체
V2 선택·편집 연결·실물 한컴 PDF 검증도 남아 있다. 기본 Legacy 엔진은 유지하며, 원격 push/PR/
이슈 종료나 전체 CI 통과는 이번 절편의 결과에 포함하지 않는다.
