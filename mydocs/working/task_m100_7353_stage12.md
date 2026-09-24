# Task #7353 — V2 단색 표/셀 배경

- 선행 절편: `d81328534` ([stage11](task_m100_7353_stage11.md)).
- 상태: 단색 배경 구현·선택 회귀·Native/fresh WASM 검증 완료. R3 전체 완료는 아님.
- 범위: 명시적 V2 선택 표의 단색 배경. 테두리선·zone·패턴·그림·그라데이션은 미지원.

## 규칙과 소비 경로

IR의 표/셀 borderFill 참조는 1-based이며 0은 참조 없음이다. 단색 COLORREF는
일반 paint의 색상 해석을 사용한다. 표 배경 위에 셀 배경, 그 위에 셀 내용이 그려진다.
배경은 문단 줄 점유를 만들지 않고 이미 예약한 표/셀 조각을 채운다. 빈 문단·padding·
물리 잔여 밴드에도 적용하며 반복 제목·colspan·자식 표는 각자의 확정 사각형을 사용한다.

`IR 참조/ResolvedStyleSet → 불변 paint snapshot → FragmentPlan의 bounds → Rectangle`
경로로 연결한다. 측정과 분할에는 새 조건을 넣지 않으며 paint에서 높이를 다시 계산하지 않는다.
채움 없음과 흰색 채움을 구분하고 불량 참조를 무채움으로 감추지 않는다. 일반 스타일 해소에서
손실되는 원본 효과(alpha 등)는 문서 진입에서 먼저 검사한다. 직접 resolved-style API는
호출자가 해소한 스타일을 입력으로 하는 별도 계약이다.

이는 실험 경로 기능 확장이다. 합성 HWPX의 명시 색/고정18px 줄·선언 여백과 불변식을
독립 기대값으로 삼고 실제 최종 노드·SVG·Native/fresh WASM PNG를 대조한다.
한컴 생성본/PDF가 없는 입력을 한컴 일치로 판정하지 않는다. Legacy 알고리즘/기준값은 무변경.

## 구현 범위와 적용 경계

- `decoration.rs`: 1-based 참조 해소, 참조 없음/투명 COLORREF/흰색 구분, 불변 배경 snapshot,
  확정 bounds의 일반 Rectangle 출력. 부모 배경 → 자식 셀 배경 → 내용 순서다.
- `text_ir::bind_paint → TextPaint::build_node`: 논리 row/column으로 셀 색을 연결한다.
  colspan의 내부 열 경계나 셀 높이를 재계산하지 않는다. ID도 Table/TableCell에 보존한다.
- `TablePreviewSession::from_document → validate_source → resolve_styles`: 해소 과정에서
  사라질 수 있는 원본 alpha·3D·잘못된 복합 채움을 먼저 거부한다. 재귀 자식에도 적용하고
  사용하지 않는 서식은 검사 대상에 넣지 않는다.
- `PreparedTextTable::prepare`는 이미 해소된 서식을 입력받는 저수준 계약이다.
  source-only 속성의 검증을 주장하지 않는다. `from_flow_rows`가 직접 만드는 셀은 계속
  무배경이며, 기존 immutable child paint/geometry 소유 방식은 유지한다.

alpha=0/255의 불투명 처리, 1~254 반투명의 구분은
[사양 정오표 §8](../tech/hwp_spec_errata.md#8-채우기-투명도alpha-바이트--스펙-미기재)를 따른다.
반투명·테두리선·대각선/중심선·패턴·그림·그라데이션·zone은 구현한 척 무시하지 않는다.
이 절편에는 컷·요구 높이·예약·이월 계산 변경이 없다. 기존 FragmentPlan의 표/셀 bounds를
그대로 채우므로 새 분할 예외나 clamp는 없다.

## 검사와 중간 관측

증적 폴더: `output/7353/r12/`. source 기준은 `d81328534` + `source.sha256`의 작업 트리다.
고정 policy base: `7a95e46e025470a4d7a7b59ad68ec02958bda738`.

`tests/cases/issue_7353_table_v2_export.rs`에 6건을 추가했다. 새 실행 계약의 확장이며
기존 기능 결함 수정으로 포장하지 않는다. 기존 V2에서는 지원하지 않았던 입력이다.

- `before.log`: 기존6 PASS / 신규5 FAIL. 장식 미지원 거부(정상 단색을 포함)를 확인했다.
  물리 잔여 밴드 계약1건은 이후 추가했으므로 수정 전 검출 증거로 세지 않는다.
- product의 `cargo fmt --all`은 파생 suite가 없어 실패했다. review worktree에서 formatter를
  수행했다. 이후 formatter로 바뀐 test 크기에 비해 manifest가 낡은 실행(`after.log`)은
  **0건 / 실패**로 끝났고 통과로 세지 않았다. 포맷 뒤 `--prepare`를 실행해 해결했다.
- `after-prepared.log`:11 PASS /1 FAIL. 빈 문단이 빈 TextRun도 가지는데 테스트가 `""`를
  기대 목록에서 빠뜨렸다. 입력·제품 코드는 바꾸지 않고 빈 run과 그18px 점유를 함께 검사했다.
- `export-final.log`:12 PASS. `export-with-rejection.log`에서도 최종12 PASS이며, 실제 HWPX
  round-trip 뒤 테두리/패턴 속성 보존과 명시적 거부, 불변 snapshot을 추가 검증했다.
- `selected-tests.log`: 기존 V2 78 + Legacy #5301 3 PASS. 선택 검사 합계 **93 PASS**.
  총 페이지 수만이 아니라 최종 셀/배경 bounds·줄 원점·빈 밴드·소유 순서·종료를 확인한다.

HWPX fixture는 수동 생성 IR을 정식 serializer/parser로 왕복한 합성 입력이다.
고정 줄간격18px, 본문36px, 셀 좌우10px 여백, 별도 최소높이90px 등 입력값으로 기대값을
정했다. 90px 셀은36/36/18px로 이어지고 마지막 텍스트 뒤에도 배경만 남는 조각을 확인한다.
기존 출력의 페이지 수를 새 기대값으로 복사하지 않았다. 별도 HWP 이진 fixture나 한컴 PDF
생성·대조는 이번 절편에서 수행하지 않는다.

## 재현 명령

review worktree에서 테스트 원본 포맷 뒤 파생 suite를 준비한다. target은 기존 고정 경로다.

```sh
node scripts/rust-test-suite-manifest.mjs --prepare
ISSUE7353_EXPORT_DIR=/home/edward/mygithub/rhwp-task-7353/output/7353/r12/fixtures \
  node scripts/run-rust-test.mjs issue_7353_table_v2_export -- \
  --cargo-profile release-test --target-dir /home/edward/mygithub/rhwp/target/pr-review --no-fail-fast
```

product worktree의 Docker fresh WASM을 별도 Chrome에서 실행한다. 기존 Studio pkg는 바꾸지 않는다.

```sh
docker compose --env-file .env.docker -p rhwp run --rm wasm
VISUAL_SWEEP_CHROME=/home/edward/.cache/puppeteer/chrome/linux-146.0.7680.31/chrome-linux64/chrome \
  node scripts/verify-table-v2-preview-wasm.mjs --pkg pkg \
  --fixtures output/7353/r12/fixtures --out output/7353/r12/browser-final \
  --dependencies-root /home/edward/mygithub/rhwp --solid-backgrounds
```

`--solid-backgrounds`는 요청한 모든 fixture를 필수로 읽는다. 없으면 실패하며 자동 생략하지 않는다.
Native와 실제 WASM의 tree/SVG를 비교하고, 각 backend의 PNG에서 내부 두 지점과
좌우·아래 바깥의 픽셀을 독립 색상 값과 비교한다. compare/standalone overlay/review도 저장한다.
overlay 기준은 Native이며 한컴 PDF가 아니다. 폰트/Studio/Canvas 피델리티 증거로 확대하지 않는다.

## 최종 검증 및 직접 판독

| 검사 | 결과 / 증적 |
| --- | --- |
| 신규6 + 기존 export6 | **12 PASS**, `export-final-visual.log` |
| 기존 V2 78 + Legacy 대조3 | **81 PASS**, `selected-tests.log` |
| 선택 회귀 합계 | **93 PASS**, 전체 CI 아님 |
| Native / WASM library Clippy | **PASS**, `clippy-native.log`, `clippy-wasm.log` |
| 최종 integration suite Clippy | **PASS**, `clippy-tests-final.log` |
| fmt / 고정 base manifest 정책 | **PASS**, `fmt-final.log`, `policy-final.log` |
| Docker release + wasm-opt | **PASS**, 7분28초, `docker-wasm.log` |
| 실제 Chrome WASM | **19쪽 PASS**, `browser-final-retry.log`, `browser-final/manifest.json` |
| frontend 선언 검사 / JS 구문 | **1 PASS / PASS**, `bindings.log` |
| 변경 문서2개 링크 / diff whitespace | **PASS**, `links.log` |

Clippy는 review worktree에서 고정 target에 순차로 실행했다.

```sh
cargo clippy --locked -p rhwp --lib --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo clippy --locked -p rhwp --lib --target wasm32-unknown-unknown --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo clippy --locked --test regression_suite_014 --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo fmt --all -- --check
node scripts/rust-test-suite-manifest.mjs --check --base-ref 7a95e46e025470a4d7a7b59ad68ec02958bda738
```

suite 번호는 최종 test 원본 크기로 배정된 값이다. 파생 suite/manifest는 review 전용이며
product에 추가하지 않았다. source-side cfg(test)와 정책/래칫/ignore는 변경하지 않았다.

배경10쪽 + 기존 대조7쪽의 최초 Chrome 비교(`browser.log`)가 통과했다. 비교 PNG17장을
직접 열어 제목 반복, A→B, host/after, 빈 줄 여백, 텍스트가 없는36/18px 이어받기를 확인했다.
이후 투명/흰색2쪽도 실제 WASM 검증에 추가했다. 제품 코드는 그대로이며 최종19쪽 전부의
tree/SVG가 Native와 exact 일치하고, 내부/바깥 픽셀 검사도 통과했다. 최초 판독17장의
review PNG는 최종 캡처와 byte-identical임을 확인했고 추가2장도 직접 열었다.
투명 셀에서는 부모 표 배경이 보이며, 흰색 셀에서는 가려지는 것을 확인했다.
직접 판독에서 배경의 영역 침범·텍스트 가림·중복·색/배치 차이를 관찰하지 않았다.

대표 증적은 `browser-final/fill-split-0.review.png`, `fill-nested-2.review.png`,
`fill-band-2.review.png`, `fill-transparent-1.review.png`이며 각 이름의 standalone
`*.overlay.png`, `*.native.png`, `*.wasm.png`와 SVG/JSON도 보존했다.
Native 기준 RGB overlay는 채움 색의 정보를 회색화하므로 원본 PNG와 독립 RGB 픽셀 검사를
함께 사용했다. 한컴과의 동일성이나 한글 글꼴 피델리티의 증거는 아니다.

최종19쪽 비교 첫 시도는 Chrome launch가 code null로 실패했다(`browser-final.log`).
원인은 확정하지 않았고 제품 결함으로 분류하지 않는다. 코드/환경 설정 변경 없이 재실행한
`browser-final-retry.log`가19쪽 통과이며 실패 로그도 보존했다.

`source.sha256`은 최종 구현/테스트/하네스17개 파일, `pkg.sha256`은 실제 실행 패키지를 고정한다.
WASM SHA-256: `fa07e15e041965fd73038ec29f8e556692a6f05e59f286b86f38a1a7dca84130`.
Chrome:146.0.7680.31. product `pkg/`만 빌드했으며 다른 Studio 작업 디렉터리는 변경하지 않았다.

## 남은 범위

테두리선의 분할 경계/공유선 소유, zone/복합 채움, 문서 전체 V2 선택, 저장 LineSeg,
TAC/어울림·rowspan·편집 연결·실물 한컴 PDF 검증은 남아 있다. 이번 절편은 셀 배경만의
기능 확장이며 기존 Legacy 조판 개선이나 전체 V2 엔진 완성을 주장하지 않는다.
전체 workspace/CI·Native Skia·원격 push/PR/이슈 종료는 수행하지 않았다.
