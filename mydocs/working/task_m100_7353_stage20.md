# Task #7353 — WASM 제품 V2 전환

- 승인: 2026-10-01 구현계획 §5.4, W1 → W2 → W3. 내부 helper별 재승인은 요청하지 않는다.
- 기반 source: `3b2390949e69cff4f6c35ffd7353a8ecf02de2a7`; 승인 계획 커밋 `8df0aa1d1`.
- 브랜치: `refactor/0.9.0`. 이번 실행은 로컬 구현·검증이며 추가 push/PR/devel 병합은 하지 않는다.
- 정본: [구현계획 §5.4](../plans/task_m100_7353_impl.md#54-2026-10-01--wasm-제품-조판을-v2로-전환).

## W1 — 문서 단위 읽기와 공통 출력

`DocumentCore::from_bytes_with_engine` 및 WASM `HwpDocument.openWithTypesetter(bytes, "v2")`
에서 파싱한 IR로 V2를 준비한다. Legacy compose/pagination 후 SVG만 덮어쓰는 방식이 아니다.
기존 `new HwpDocument(bytes)`의 기본값은 W3 전까지 Legacy다. `getTypesetter()`로 구분한다.

### 책임·실제 경로

- `document_core/typesetting.rs`: 엔진 선택과 완성/거부/미준비 결과 경계. 실패 시 페이지 수 0과
  명시적 오류를 공개하고 이전 page/layer cache를 재사용하지 않는다. Legacy 자동 재시도 없음.
- `table_v2/host_document.rs` → `HostedSectionLayout`: 문서 전체의 구역 순서/전역 쪽 주소와
  쪽 번호 carry를 전달한다. V2 준비 결과는 IR을 소유하지 않고 DocumentCore 원본을 참조해
  paint한다. 기존 독립 preview만 불변 IR snapshot을 소유한다.
- 쪽 번호의 생산·소비: document host `first_number` → `typeset_hosted_section`의
  `PageNumberAssigner` → `TableHostFrame.with_page_number`/paragraph fit → 실제 cell field paint.
  이후 공개 `PageContent`와 absolute/master/story도 같은 쪽 번호를 소비한다. 출력 문자열을
  사후 치환하지 않는다. 전역 index와 표시 쪽 번호의 명시적 재시작은 별개다.
- `build_page_tree` → `with_page_tree_cached` → `PageLayerTree` → 제품 SVG/Canvas.
  `HostedParagraph`/`HostedTable`의 확정 조각을 재사용하며 Legacy PartialTable로 변환하지 않는다.
- W1의 구역 간 master/page-story/note 상속은 미지원으로 명시 거부한다. 정상적으로 전달할 수 없는
  문맥을 초기화하여 성공시키지 않는다. 기존 구역/표 admission은 유지한다.

### 집중 검증

정식 계약: `tests/cases/issue_7353_product_v2.rs`.

- 정상 한컴 저장본 `issue7353_body_frame_review/portrait-saved.hwp`와 같은 입력의 PDF:
  2쪽, 실제 빈 줄, 줄 y=20+24×n 및 높이16px 보존. 제품 트리와 기존 V2 호스트 트리 동일.
- 서로 다른 폭의 두 구역: 전역 index 0–3, section 소유 0/1, 표시 쪽 번호 1–4,
  각 구역의 실제 TextLine 소유까지 검사. 이 파생 입력은 합성 소유/기하 계약이며 한컴 오라클이 아니다.
- 정상 한컴 저장본 `issue7353_host_owner_review/split-saved.hwp`와 같은 입력의 PDF:
  p1 BEFORE, p2 ROW01–19, p3 ROW20–25/AFTER. 25개 유닛이 정확히 한 번 배치되고
  AFTER y=129px(저장 vpos8175HU+위여백1500HU)이 유지된다.
- V2 거부 후 페이지/Canvas/SVG에 이전 결과가 노출되지 않음. 명시적 Legacy 재열기는 별도 검사.
- 셀 쪽 번호 필드: 두 번째 구역의 실제 문자열 `2`, 명시적 번호 재시작의 `1`을 검사한다.
  전역 page index는 재시작하지 않는다. 기존 정상 저장본의 필드 paragraph를 사용한 합성 구역
  계약이며, 별도 한컴 출력과 일치했다는 주장은 하지 않는다.
- 테스트 중 두 번째 구역의 페이지 root만 section 0으로 남는 연결 누락을 검출하고 수정했다.
  기존 preview 오류를 문단 주소로 감싸면서 typed rejection 3건이 깨진 것도 확인하여 기존 preview의
  오류 variant는 보존하고 새 제품 경계에서만 문단/구역 위치를 유지하도록 정정했다.

증적: `output/7353/wasm-product/w1/`. fresh WASM 검사는
`scripts/verify-product-v2-wasm.mjs`가 **HwpDocument 제품 API**를 호출한다.
Native/WASM tree·layer·SVG를 대조하고 실제 `renderPageToCanvas`를 별도로 캡처한다.
미리보기 SVG를 Canvas 출력 증거로 대체하지 않는다.

### W1 결과 — 2026-10-01

W1의 명시적 V2 읽기·공통 출력 연결과 집중 검증은 완료했다. 전체 WASM 제품 전환 완료는 아니다.
W2/W3, 전체 회귀 및 최종 제출 검증은 아래에 남긴다. W1 코드는 아직 미커밋이며 원격 게시하지 않았다.

- 검증 source: `8df0aa1d1` + W1 변경. 12개 Rust 파일의 정확한 바이트는
  `output/7353/wasm-product/w1/source-sha256.txt`로 고정하고 빌드 후 `sha256sum -c` 통과.
- `cargo test --locked --jobs 2 --target-dir /home/edward/mygithub/rhwp/target/pr-review
  --test regression_suite_022 issue_7353_product_v2 -- --test-threads=4`: **6 통과**.
- 같은 target의 `--test regression_suite_017 issue_7353_hosted_section`: **31 통과**.
  로그는 `native-tests.log`, `host-regression.log`.
- `CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/home/edward/mygithub/rhwp/target/pr-review
  scripts/wasm-pack-locked.sh --target web --out-dir output/7353/wasm-product/w1/pkg --no-opt`:
  fresh WASM 성공. `--no-opt` 진단 빌드이며 Studio 배포 자산을 교체하지 않았다.
- Chrome `152.0.7977.54`에서 제품 `openWithTypesetter("v2")`로 두 정상 저장본 **5쪽** 검사:
  Native/WASM 공개 트리·레이어·SVG 일치, 실제 `renderPageToCanvas` 캡처 성공.
  `browser-manifest.json`과 `table/browser-manifest.json`에 입력/JS/WASM SHA 보존.
  WASM SHA256: `ee09a2d29a107f74f90d8cab7bc624c6257d7031c8a24636e1582fb74d5fd195`.
  브라우저 JSON 전송의 `-0`→`0` 정규화만 Native에도 동일하게 적용했다. 비영 좌표 반올림·허용 오차 없음.
- 시각 비교: 위 각 저장본과 대응 `portrait-2020.pdf` / `split-2020.pdf`, 전체 2쪽/3쪽을 96dpi로
  비교했다. `output/7353/wasm-product/w1/sweep.py`는 canonical Visual Sweep의 compare/overlay/review를
  사용하되 Legacy CLI 대신 제품 API에서 추출한 Native/fresh WASM SVG와 실제 Canvas PNG를 입력한다.
  실행 환경은 `VISUAL_SWEEP_CHROME=/home/edward/.cache/puppeteer/chrome/linux-152.0.7977.54/chrome-linux64/chrome`.
  `scripts/verify-product-v2-wasm.mjs --pkg .../pkg --input <위 저장본> --out <w1 또는 w1/table>`로
  브라우저 검증 후 main repo venv Python의 `sweep.py native`, `sweep.py wasm canvas`를 실행했다.
- 직접 판독: 본문 1쪽 실제 빈 줄, 2쪽 DELTA/ECHO/AFTER, 표 1쪽 BEFORE와
  2쪽 ROW01–19 → 3쪽 ROW20–25/AFTER의 배치·분할을 확인했다. 대표 Canvas review와 standalone overlay:
  `visual/canvas/review/review_002.png`, `table/visual/canvas/review/review_003.png`,
  `table/visual/canvas/overlay/overlay_003.png` (모두 위 증적 root 기준).
  Native/WASM SVG review도 같은 디렉터리 구조의 `visual/native`, `visual/wasm`에 보존했다.
- 남은 차이: 기준 PDF와 글립 형태/폭 및 테두리 명도·래스터화가 다르다. 내용 픽셀 자동 일치율은
  Native/WASM SVG 2.05–4.32%, Canvas 8.67–16.94%로 낮다. 이를 피델리티 합격 점수로 사용하지 않는다.
  이번 연결의 핵심인 V2 출력 동일성·유닛 보존·줄/표/뒤 문단 원점 검증과 폰트/paint 차이를 구분한다.
  실제 전체 Studio 편집 시각 판정은 W3에서 수행한다.

`cargo fmt --all -- --check`, Native 및 `wasm32-unknown-unknown` 각각의
`cargo clippy --locked --jobs 2 -p rhwp --lib --target-dir /home/edward/mygithub/rhwp/target/pr-review
-- -D warnings`도 통과했다 (`clippy-native.log`, `clippy-wasm.log`). 브라우저 검증 스크립트의
`node --check`와 `git diff --check` 통과. 전체 workspace/all-target lint와 전체 회귀·정책은
W3 안정 통합 시점의 필수 게이트로 남아 있으므로 전체 CI 통과를 주장하지 않는다.

## 다음 묶음과 남은 검증

- W2: 편집/reflow/undo/redo/문서 교체/폰트/DPI 세대, 커서·선택·hit-test, 실패의 원자성,
  저장·재열기. W1은 편집 제품 지원 완료라는 뜻이 아니다.
- W3: 일반 Studio/worker와 새 문서·암호·복구 진입을 연결하고 WASM 기본값을 V2로 전환.
  실제 편집 여정, 필수 전체 회귀·3종 Clippy·정책·최종 fresh WASM 시각 판정.
- Native/CLI 기본값, Legacy 삭제, devel 병합·0.9.0 릴리즈는 이번 로컬 전환 완료와 구분한다.
