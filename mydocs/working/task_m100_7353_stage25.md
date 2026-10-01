---
kind: report
status: active
---

# #7353 — Legacy 표 구현 의존 제거

## 범위와 기준

- 승인된 구현계획 §5.7. 최종 종료 조건은 `table_layout.rs`를 실제 삭제한 상태의 Native/WASM 빌드와 Studio 편집·저장 검증이다.
- U1 승인 작업은 로컬 `d977557b7`로 보존했다. 원격 push는 하지 않았다. 이전 전체 회귀의 오래된 거부 계약 2건은 이 단계의 선행 작업으로 삼지 않는다.
- 2026-10-01 추가 지시: 제거한 Legacy 내부 구현 전용 테스트도 삭제한다. 사용자가 보는 기능 계약은 V2 정식 회귀로 보호하며 실패 은폐 목적으로 삭제하지 않는다.

## 현재 구현 묶음

1. V2 확정 조각 → `host_page::build` → 해당 조각의 `render_nodes/render_node` → 최종 본문 노드. Legacy `build_render_tree/build_columns/build_single_column`의 앵커 재선택·vpos 보정 경로를 사용하지 않는다.
2. 바탕쪽은 공통 `ObjectPlacementFrame` → V2 도형/글상자 paint로 연결한다. 사각형 외곽은 확정 bounds와 공통 스타일을 소비하며 Legacy 도형/표 dispatcher를 호출하지 않는다.
3. 본문 clip·셀 내부 paint 순서·단 구분선은 확정 노드만 읽는 `page_paint`로 분리하여 두 경로가 공유한다. 표의 계측/컷/이어받기 알고리즘은 옮기지 않는다.
4. 선/연결선 paint는 `shape_paint`로 분리한다. V2 바탕쪽의 폭/높이 중 한 축이 0인 선과 점선을 셀 내부 제한 조건으로 거부하지 않는다. 입력 선 끝점과 최종 노드의 끝점·source ownership을 정식 회귀에서 검사한다.

## 검증 계획 및 남은 범위

- 변경 전 증거: `output/7353/legacy-removal/before/{master,stories,wrap}`. 정상 한컴 저장 fixture와 기존 독립 PDF를 그대로 사용한다.
- 실제 줄·셀·외곽·뒤 문단 위치, 바탕쪽 선택/번호와 편집 계약을 정식 V2 테스트로 확인한다. 추출한 공통 paint는 Legacy 대조군도 검증한다.
- 다음 제거 차단점: V2 텍스트 paint의 LayoutEngine 결합, TypesetEngine과 Legacy 표 scan/continuation 타입 결합, DocumentCore의 명시적 Legacy API/캐시.
- 아직 `table_layout.rs` 삭제 및 전체 기능 대체 완료가 아니다. 새 코드의 fresh WASM·Studio·시각 검증은 실행 결과를 아래에 기록한다.

## 현재 묶음 검증

- 소스: `d977557b7` 이후 미커밋 변경. production 파일 10개의 SHA256 목록을 순서대로 해시한 값은 `621b263f5082d8ca099c46986fcf3c12f0ddad034bae8693d6cc1d5b5bf31529`. 대상은 `page_paint.rs`, `shape_paint.rs`, `table_v2/{host_page,host_master,host_section,shapes}.rs`, `layout.rs`, `layout/shape_layout.rs`, `renderer/mod.rs`, `table_v2/mod.rs` 순서다.
- Native Clippy, WASM lib Clippy, 변경 integration target(`regression_suite_007`, `regression_suite_023`) Clippy: 모두 `--locked ... -- -D warnings` 통과. 로그: `output/7353/legacy-removal/clippy-{wasm,tests}.log`. 전체 workspace lint/전체 회귀 완료를 의미하지 않는다.
- `cargo nextest run --locked --cargo-profile release-test --target-dir /home/edward/mygithub/rhwp/target/pr-review --test regression_suite_004 --test regression_suite_007 --test regression_suite_012 --test regression_suite_017 --test regression_suite_018 --test regression_suite_023 --test regression_suite_024 -E 'test(issue_7353) and not test(unsupported_first_page_flags_remain_explicit) and not test(unsupported_format_or_mid_paragraph_declaration_remains_explicit)' --test-threads 4 --no-fail-fast`: **203 PASS, 0 FAIL**, 필터 제외 1439. 기존 U1 거부 계약 2건은 명시적으로 제외했고 삭제/ignore/기준값 변경은 하지 않았다. 로그: `output/7353/legacy-removal/focused.log`.
- 정상 저장 master/stories/wrap 3종, 총 10페이지의 변경 전후 compact render tree는 완전 일치. Canvas layer는 `sourceNodeId` 재할당 외 모든 값이 일치한다. 이는 이번 경로 분리의 무변화 증거이며 한컴과의 완전 일치 주장과 구분한다.
- Native 대표 review(master 2쪽, stories 5쪽, wrap 1쪽)를 직접 확인했다. 본문 줄·단·바탕쪽 사각형·어울림 영역·쪽번호 배치가 유지된다. 한컴 PDF와 글꼴/획 차이는 남는다. 산출: `output/7353/legacy-removal/after/{master,stories,wrap}/visual/native/{compare,overlay,review}`.
- 최종 production 소스의 Docker 표준 WASM 빌드: **PASS (7분 57초)**. `docker compose --env-file .env.docker -f docker-compose.yml -f output/7353/wasm-product/final/compose-network.yml run --rm --no-deps wasm`. 로그 `output/7353/legacy-removal/wasm-final.log`. WASM SHA256 `e319a5fce14e0e76e19fe9cd8e2d9b7406e8b1f9dcfacbb1937c5b7d59ca19d1`; Studio `:7700`이 제공하는 파일도 같은 해시다.
- `node scripts/verify-product-v2-wasm.mjs --pkg pkg --input <fixture> --out output/7353/legacy-removal/after/<case> --default-v2 --hf-addresses`: master/stories/wrap/body/table **5종 15페이지 PASS**. Native/WASM tree/layer/SVG 일치와 실제 Canvas 캡처 확인. body에는 `--edit`, table에는 `--edit-cell`을 추가해 편집 후 출력·저장 계약을 검사했다. 입력/JS/WASM 해시와 세부 결과는 각 `browser-manifest.json`, 실행 로그는 `wasm-verification.log`.
- fresh WASM/Canvas의 compare·standalone overlay·review도 생성했다. Canvas master 2쪽·stories 5쪽 review, wrap 1쪽 overlay와 Studio 셀 편집 캡처를 직접 판독했다. 이번 의존 분리로 인한 배치 변경은 확인되지 않았다. 글꼴/획 차이를 새롭게 해결했다고 주장하지 않는다.
- Windows Chrome CDP의 실제 Studio: `CHROME_CDP=http://localhost:19222 VITE_URL=http://localhost:7700 RHWP_V2_E2E_OUT=../output/7353/legacy-removal/studio node e2e/product-v2.test.mjs --mode=host` **PASS**. 일반 파일 입력의 V2 선택, 본문/셀 실제 키보드 입력, undo/redo, 선택 표시, 저장/재열기, 새 문서 편집과 명시적 Legacy 대조군을 확인했다. 브라우저 미처리 오류 0. 결과·저장 HWP·캡처는 `output/7353/legacy-removal/studio`, 로그는 `studio.log`.
- 위 결과는 페이지·바탕쪽·도형 의존 분리 묶음에 대한 증거다. 아직 Legacy 표 파일 제거, 미지원 조합 전부 구현, 전체 회귀/전체 workspace lint 완료가 아니다. 테스트 삭제도 해당 구현 제거 시점에 수행한다. 현재 묶음은 미커밋이며 원격 push하지 않았다.

## 테스트 제거 경계

- `table_layout.rs` 내부 unit test 39개: 해당 구현 삭제와 함께 제거. 현재 구현이 여전히 사용되므로 먼저 삭제하지 않는다.
- 별도 테스트 중 Legacy private helper/타입만 검증하는 항목: V2 이전 여부를 확인하고 구현과 함께 제거한다.
- `#7140` 등의 실제 문서 넘침·최종 배치 검사: Legacy 함수명이 주석에 있다는 이유로 삭제하지 않는다. 사용자 기능 계약을 V2 최종 배치 검사로 유지한다.
- 신규 architecture guard는 페이지/바탕쪽/도형 소비 경로의 Legacy 호출 재유입을 검사한다. 실제 배치·편집 회귀를 대체하지 않는다.

## 후속 묶음 — V2 페이지 상태 독립

- 앞 묶음을 `4b7b529d0`으로 로컬 커밋했다. 원격 push는 하지 않았다.
- `host_section → host_flow::paginate → PageFlow`로 연결하여 V2 제품 페이지네이션의 `TypesetEngine::run_section`/`TypesetState` 호출을 제거한다. 기존 **V2** 예약·fit·commit 코드를 `typeset/hosted.rs`에서 V2 소유 모듈로 옮겼으며 Legacy 표 측정/분할 알고리즘을 복사하지 않았다.
- 공통 결과의 소비: `HostedTableSession::query/commit` 또는 `HostedParagraphPlan::fit`이 확정한 조각 → `occupied/next_y`의 본문 예약 → 다음 쪽/단의 잔여 예산 → `host_page`의 동일 조각 paint. 실패한 fit은 컷을 소비하지 않는다. 앞 문단의 미완료 예약을 마친 뒤 다음 문단의 명시적 쪽/단 나눔을 적용한다.
- V2 페이지 상태는 빈 문단을 Legacy 종료 휴리스틱으로 흡수하지 않는다. 합성 단일 단 문서의 `prefix → 단 나눔을 가진 빈 문단` 계약은 상단 여백 2250HU=30px, 글자 9pt=12px, 고정 줄간격 2700HU=18px에서 기대값을 정했다. 정상 저장 문서의 한컴 일치 증거와는 구별한다.
- 동일 새 테스트를 변경 전 `4b7b529d0`의 `release-test/deps/librhwp.rlib`에 직접 연결해 실행: `section boundary absorbed a V2 paragraph`로 **FAIL**. 컴파일 실패가 아니라 실제 V2 로드 거부를 확인했다. 로그: `output/7353/legacy-pagination/before-contract.log`.
- 검증 소스: `4b7b529d0` 이후 이 묶음의 미커밋 변경. `output/7353/legacy-pagination/source-sha256.txt`는 실제 페이지네이션 변경 파일 5개의 SHA256 목록이며 목록 자체의 SHA256은 `107582b28a2d8cc2928fcd1f8edabeec994ecb05fde744244a10c24a55063257`이다. 모듈 등록과 테스트 변경은 같은 작업 트리에 포함되어 있다.
- Native check, Native/WASM lib Clippy, 변경 integration target(`regression_suite_017`, `regression_suite_023`) Clippy, `cargo fmt --all -- --check`: **PASS**. 로그: `output/7353/legacy-pagination/clippy-{native,wasm,tests}.log`. 전체 workspace lint를 실행했다고 주장하지 않는다.
- 집중 회귀는 앞 묶음과 동일한 7개 suite·필터로 **203 PASS, 0 FAIL**(1439 filtered). 이전 U1 거부 계약 2건은 이전과 동일하게 제외했고 ignore/기준값 변경을 하지 않았다. 로그: `output/7353/legacy-pagination/focused.log`.
- 새 빈 문단 계약의 수정 후 **PASS**: `output/7353/legacy-pagination/after-contract.log`. `rustc --edition=2021 --test tests/cases/issue_7353_hosted_section.rs --extern rhwp=<shared-target>/release-test/deps/librhwp.rlib --extern serde_json=<shared-target>/release-test/deps/libserde_json-cd3bf15989922071.rlib -L dependency=<shared-target>/release-test/deps -o <before-or-after-contract>`로 동일 테스트를 전후 라이브러리에 연결했다. `<shared-target>`은 `/home/edward/mygithub/rhwp/target/pr-review`이다. 해당 파일의 31개 계약도 모두 통과했다(`hosted-section.log`).
- 정상 한컴 저장 master/stories/wrap/body/table **5종 15페이지**: 변경 전 `legacy-removal/after`와 이번 `legacy-pagination/after`의 Native tree/layer/SVG 완전 일치. 입력·기준 PDF는 앞 묶음과 같으며 재저장/기준값 갱신 없이 사용했다.
- Docker 표준 fresh WASM 빌드 **PASS, 8분 35초**. 앞 묶음과 동일 compose 명령. WASM SHA256 `a1024dd14ce611af6cbcd4612fe40eef0bb7f697b5264bf9af6a3e8642d146aa`; Studio `:7700`의 실제 제공 파일과도 일치. 로그: `output/7353/legacy-pagination/wasm-build.log`.
- `verify-product-v2-wasm.mjs --pkg pkg --input <fixture> --out output/7353/legacy-pagination/after/<case> --default-v2 --hf-addresses` **5종 15페이지 PASS**. body `--edit`, table `--edit-cell`도 통과. Native/WASM tree/layer/SVG 일치, 실제 Canvas 캡처, 셀 성장 페이지네이션·마지막 줄 보존을 검사했다. 세부 해시/결과: 각 `browser-manifest.json`, 로그: `wasm-verification.log`.
- Native와 fresh Canvas의 compare/standalone overlay/review를 새로 생성했다. master 2쪽·stories 5쪽 review, wrap 1쪽 Canvas overlay, Studio 셀 편집 캡처를 직접 판독했다. 본문 줄/단, 바탕쪽 외곽, 어울림 공간, 이어지는 표 및 뒤 문단이 유지된다. 기존 글꼴·획 차이를 해결했다고 주장하지 않는다. 증적: `output/7353/legacy-pagination/after/{master,stories,wrap}/visual/{native,canvas}`.
- Windows Chrome CDP Studio 실키보드 여정 **PASS**: `CHROME_CDP=http://localhost:19222 VITE_URL=http://localhost:7700 RHWP_V2_E2E_OUT=../output/7353/legacy-pagination/studio node e2e/product-v2.test.mjs --mode=host`. 본문/셀 입력, undo/redo, 커서/선택, 저장/재열기, 새 문서, 명시적 Legacy 대조군 확인. 브라우저 미처리 오류 **0**. 결과: `studio/result.json`, 대표 캡처 `studio/cell-edited.png`, 실행 로그 `studio.log`.
- 남은 제거 차단점은 `table_v2/text.rs`의 `LayoutEngine` 텍스트 paint와 DocumentCore의 명시적 Legacy API/캐시다. `table_layout.rs`와 그 전용 테스트는 아직 삭제하지 않았다.

## 후속 묶음 — DocumentCore의 Legacy 소유권과 글꼴 상태 분리

- 직전 검증 묶음을 `e03f0e4e8`로 로컬 커밋했다. 원격 push는 하지 않았다.
- `FontLayoutState`로 exact source/instance request와 immutable 측정 snapshot 소유권을 분리한다. 글꼴 선택·폭·줄바꿈 규칙과 제한값은 변경하지 않는다. font payload는 기존처럼 Arc로 공유한다.
- `DocumentCore::from_document_with_engine`은 명시적 Legacy 선택에만 `LayoutEngine`을 생성한다. V2 문서의 지속 상태에는 Legacy 엔진/셀 캐시가 없다. Legacy 전용 페이지·caret·컷 소비자는 확인된 선택적 소유자를 사용하며 V2에서 자동 생성하거나 fallback하지 않는다.
- 값의 연결: embedded/public font 등록 → `FontLayoutState` → `refresh_exact_font_measurement_contexts`가 같은 registry generation의 kerning/shaping snapshot 생성 → 기존 compose/측정/배치의 style context 소비. Legacy를 선택한 문서에만 같은 font state snapshot을 전달한다. 문서 교체는 등록과 두 context를 함께 초기화한다.
- Legacy overflow 진단 API의 V2 빈 결과는 기존 비사용 Legacy 카운터와 같은 의미이며 V2의 무결함 판정으로 사용하지 않는다. V2 최종 좌표·출력·편집 검사를 별도로 실행한다.
- `table_layout.rs` 및 cursor private 테스트의 owner 접근만 조정했다. 아직 쓰이는 Legacy 테스트를 먼저 삭제하지 않는다. `table_v2/text.rs`의 임시 Legacy 텍스트 paint 호출은 **남아 있으므로** Legacy 파일 삭제 완료 또는 V2의 모든 Legacy 의존 제거를 주장하지 않는다.
- 신규 `tests/cases/issue_7353_product_v2.rs` 계약은 구조 소유권과 양 엔진의 font 등록/멱등성/초기화/편집 후 실제 출력을 구분한다. 저장 문서의 반복 등록 전후 SVG를 정확히 비교하며 페이지 수만 검사하지 않는다. 결함 수정이 아닌 소유권 refactoring 계약이다.
- 검증 소스는 `e03f0e4e8` 이후 현재 작업 트리다. 변경 source digest: `output/7353/legacy-font-owner/source-sha256.txt`. 최종 검증 결과는 아래에 이어 기록한다.
- Native check, Native/WASM lib Clippy, 변경 integration suite 025 및 font 대조군 016/026 Clippy, fmt check, JS syntax, diff check: **PASS**. `output/7353/legacy-font-owner/clippy-{native,wasm,tests}.log`. source-side 테스트 owner 변경의 unit-tier 검사도 고정 base `02530b9ed567a44663edb26c65fb565c4a79f00d` 대비 **PASS**(4205 tests/298 modules). 전체 workspace lint/최신 PR CI 검증으로 확대 해석하지 않는다.
- 집중 회귀: `CARGO_BUILD_JOBS=2 cargo nextest run --locked --cargo-profile release-test --target-dir /home/edward/mygithub/rhwp/target/pr-review --test regression_suite_012 --test regression_suite_017 --test regression_suite_025 --test regression_suite_016 --test regression_suite_026 -E '(test(issue_7353) or test(issue_4968_kerning_capability_provider) or test(issue_4969_shaping_atomic_activation)) and not test(unsupported_first_page_flags_remain_explicit) and not test(unsupported_format_or_mid_paragraph_declaration_remains_explicit)' --test-threads 4 --no-fail-fast` → **179 PASS / 0 FAIL / 1008 skipped**. 기존 U1 거부 계약 2건 제외는 이전과 동일하며 새 ignore/기준값 변경은 없다. 로그 `focused.log`.
- 기존 정상 저장 master/stories/wrap/body/table **5종 15페이지**: `legacy-pagination/after` 대비 Native tree/layer/SVG 완전 일치. 이번 fresh WASM의 tree/layer/SVG도 Native와 정확히 일치한다. body/cell 편집·성장·저장 계약도 통과. `scripts/verify-product-v2-wasm.mjs --font-source tests/fixtures/fonts/RHWPExactKerningSmoke.ttf`로 Native뿐 아니라 실제 브라우저 WASM의 V2/Legacy font 등록·멱등성·최종 SVG 보존을 추가 검사했다. 입력/font/WASM 해시는 `after/*/browser-manifest.json`에 기록했다.
- Docker 표준 WASM 빌드 **PASS (8분 16초)**. 명령은 앞 묶음과 동일하다. WASM SHA256 `8fd9f60dd0419c04cb41a3c467b1b5c23b5ce819e76fd834c83bbf0b9d3184e5`; Studio `:7700` 실제 제공 파일과 일치. 로그 `wasm-build.log`.
- Windows Chrome CDP의 Studio 실키보드 여정 **PASS**, 브라우저 오류 **0**. `CHROME_CDP=http://localhost:19222 VITE_URL=http://localhost:7700 RHWP_V2_E2E_OUT=../output/7353/legacy-font-owner/studio node e2e/product-v2.test.mjs --mode=host`. 본문/셀 편집, undo/redo, 선택, 저장/재열기, 새 문서, 명시적 Legacy 대조군 확인. 증적 `studio/result.json`, `studio/cell-edited.png`, `studio.log`.
- Native/실제 Canvas의 compare·standalone overlay·review를 새로 생성하고 master 2쪽 Native/Canvas review, stories 5쪽 Canvas review, wrap 1쪽 Canvas overlay와 Studio 셀 편집 캡처를 직접 판독했다. 이번 소유권 분리로 인한 위치·줄바꿈·외곽 변화는 확인되지 않았다. 기존 한컴 대비 글꼴/획 차이는 해결 범위가 아니다. 산출 `after/{master,stories,wrap}/visual/{native,canvas}`. 패널 생성 환경 경로 오류는 제품 실패가 아니며, 이미 통과한 제품 검사는 반복하지 않고 저장소 `venv`와 Chrome 경로를 지정해 시각 단계만 재실행했다(`visual-complete.log`).
- Legacy 소유자 접근을 바꾼 source-side unit 검증: `cargo nextest run --locked -p rhwp --lib --cargo-profile release-test --target-dir /home/edward/mygithub/rhwp/target/pr-review -E 'test(issue2214_deferred)' --test-threads 2 --no-fail-fast` → **3 PASS / 0 FAIL / 3883 skipped**. 대상 셀/소유 표 캐시 무효화 2건과 같은 필터의 WASM API 캡션 흐름 1건이 통과했다. 로그 `legacy-unit.log`.
- 이 묶음의 변경은 로컬 미커밋 상태이며 push하지 않았다. 다음 제거 경계는 V2 텍스트 paint의 `LayoutEngine` 두 호출이다. Legacy 표 파일·전용 테스트 삭제와 전체 기능 대체는 여전히 미완료다.
