---
kind: report
status: historical
---

# #7353 — Legacy 표 구현 의존 제거

> **2026-10-01: 이번 접근 백지화로 기존 실행계획을 폐기했다.** 아래 구현·검증은
> 당시 범위의 기록이며 다음 제거 작업의 지시가 아니다.
> [실패 경위와 폐기 결정](../troubleshootings/issue7353_v2_refactoring_approach_withdrawal.md)을 따른다.
> 코드와 증적은 보존했고, Legacy 제거 완료나 결함 해결을 선언하지 않는다.

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

## 후속 묶음 — 공통 텍스트 paint와 Legacy 표 어댑터 분리

- 글꼴 소유권 검증 묶음을 `3bef3228b`로 보존했다. 승인 범위 안에서 로컬 작업하며 원격 push는 하지 않는다.
- `table_v2/text::compose_shared`의 임시 `LayoutEngine` 생성 두 곳을 제거한다. 쪽번호 표시 치환은 모델 문자 위치를 보존하는 순수 함수로, 줄·run·탭·필드·인라인 그림/수식 paint는 `renderer/paragraph_paint`로 추출한다. Legacy도 같은 구현을 호출하며 이중 구현을 만들지 않는다.
- Legacy 인라인 표의 분류·기준선/여백 배치·`layout_table` 호출은 `layout/paragraph_layout.rs`의 `ParagraphPaintHost` 어댑터에 남긴다. V2 physical-frame 진입점은 표 컨트롤을 받지 않고, V2의 별도 표 조각 paint를 그대로 사용한다. 잘못 분리된 표가 들어오면 노드를 내보내기 전에 오류로 반환하며 누락을 성공으로 처리하지 않는다.
- 값의 연결: V2 저장/재조판의 `physical_rows`와 composed runs → 공통 frame/run paint → 같은 최종 노드를 담은 `ParagraphItem`의 점유와 V2 fragment 배치. 저장 줄 admission·폭·높이·여백 조건과 기준값은 변경하지 않는다. Legacy는 기존 상태를 빌려 공통 paint에 전달하므로 표 내부 캐시나 페이지 소유권을 공통 모듈에 옮기지 않는다.
- 구조 계약에 V2 text 및 추출 모듈을 추가한다. 정상 한컴 저장 master/stories/wrap/body/table의 직전 검증 출력(`legacy-font-owner/after`)과 전후를 대조하고, shared text/field 및 Legacy 인라인 표 대조군도 실행한다. 이번 변경은 구조 분리이며 조판 결함 수정 또는 전체 Legacy 삭제 완료를 주장하지 않는다.
- 검증 결과는 최종 소스에 대해 아래에 기록한다.
- 검증 소스: `3bef3228b` 이후 현재 변경. `output/7353/paragraph-paint/source-sha256.txt`에 production 파일 12개의 해시를 고정했으며 목록 자체 SHA256은 `54261b9d08498f56c8ef5d84373a921262a1cf5945bfa731e031a7ee036668a3`이다. 검증 종료 시 모두 재확인했다. 공통 모듈은 기존 구현의 이동이며 V2에 Legacy 표 알고리즘을 복사하지 않았다.
- 집중 회귀: `CARGO_BUILD_JOBS=2 cargo nextest run --locked --cargo-profile release-test --target-dir /home/edward/mygithub/rhwp/target/pr-review --test regression_suite_004 --test regression_suite_006 --test regression_suite_012 --test regression_suite_013 --test regression_suite_015 --test regression_suite_016 --test regression_suite_017 --test regression_suite_020 --test regression_suite_022 --test regression_suite_024 --test regression_suite_025 --test regression_suite_026 --test regression_suite_028 -E '(test(issue_7353) or test(issue_4968) or test(issue_4969) or test(issue_6986) or test(issue_6754) or test(issue_7150) or test(issue_6699) or test(issue_3216)) and not test(unsupported_first_page_flags_remain_explicit) and not test(unsupported_format_or_mid_paragraph_declaration_remains_explicit)' --test-threads 4 --no-fail-fast` → **420 PASS / 0 FAIL / 2689 skipped**. 로그 `output/7353/paragraph-paint/focused.log`. 앞 묶음의 U1 거부 계약 2건 제외는 그대로이며 테스트 삭제·새 ignore·baseline 변경은 없다.
- Legacy 문단 대조군: `CARGO_BUILD_JOBS=2 cargo nextest run --locked -p rhwp --lib --cargo-profile release-test --target-dir /home/edward/mygithub/rhwp/target/pr-review -E 'test(renderer::layout::paragraph_layout)' --test-threads 4 --no-fail-fast` → **40 PASS / 0 FAIL / 3846 skipped**. TAC 폭/줄 소속·문자 위치·PUA 매핑 등 기존 계약이 유지된다. 로그 `output/7353/paragraph-paint/legacy-unit.log`.
- Native/WASM lib Clippy(`--locked ... -- -D warnings`) **PASS**. 로그 `output/7353/paragraph-paint-clippy{,-wasm}.log`. source-side unit-tier 정책도 고정 base `02530b9ed567a44663edb26c65fb565c4a79f00d` 대비 **PASS**, 4205 tests/298 modules. `cargo fmt --all -- --check`, `git diff --check`, 소스 해시 재확인 **PASS**. 전체 workspace lint/전체 회귀 완료로 확대 해석하지 않는다.
- 정상 한컴 저장 master/stories/wrap/body/table **5종 15페이지**: 이전 `legacy-font-owner/after`와 현재 Native tree/layer/SVG가 완전 일치하고 fresh WASM도 Native와 일치한다. `node output/7353/paragraph-paint/verify.mjs`가 `verify-product-v2-wasm.mjs --default-v2 --hf-addresses`와 body `--edit --font-source tests/fixtures/fonts/RHWPExactKerningSmoke.ttf`, table `--edit-cell`을 실행했다. 편집·셀 성장·저장·글꼴 등록 대조도 통과. 입력/글꼴/JS/WASM 해시는 `after/*/browser-manifest.json`, 실행 로그는 `verification.log`.
- Docker 표준 fresh WASM 빌드 **PASS (8분 17초)**: `docker compose --env-file .env.docker -f docker-compose.yml -f output/7353/wasm-product/final/compose-network.yml run --rm --no-deps -e CARGO_BUILD_JOBS=2 wasm`. WASM SHA256 `d61021a16e380589ee23ad47134209f7f6945899ace70d7d76f6e5e461f1c10b`; Studio `:7700` 제공 파일도 동일하다. 로그 `output/7353/paragraph-paint/wasm-build.log`.
- Native/실제 Canvas의 compare·standalone overlay·review를 새로 생성했다. master 2쪽 Native/Canvas review, stories 5쪽 Canvas review, wrap 1쪽 Canvas overlay, Studio 셀 편집 캡처를 직접 판독했다. 본문 줄·단·바탕쪽 외곽·어울림 공간·표 이어받기·뒤 문단 배치에서 이번 추출로 인한 변화는 확인되지 않았다. 기존 글꼴/획 차이는 남으며 한컴 완전 일치로 판정하지 않는다. 대표 증적: `output/7353/paragraph-paint/after/master/visual/canvas/review/review_002.png`, `after/stories/visual/canvas/review/review_005.png`, `after/wrap/visual/canvas/overlay/overlay_001.png`.
- Windows Chrome CDP 실제 Studio 여정 **PASS**, 미처리 브라우저 오류 **0**: `CHROME_CDP=http://localhost:19222 VITE_URL=http://localhost:7700 RHWP_V2_E2E_OUT=../output/7353/paragraph-paint/studio node e2e/product-v2.test.mjs --mode=host` (`rhwp-studio`에서 실행). 본문/셀 키보드 입력, undo/redo, 선택, 저장/재열기, 새 문서 편집, 명시적 Legacy 대조군을 확인했다. `studio/result.json`, `studio/cell-edited.png`, `studio.log`에 결과를 보존했다.
- 이번 묶음은 로컬 미커밋 상태이며 원격 push하지 않았다. V2 text의 Legacy 엔진 호출은 제거했지만 **`table_layout.rs` 및 명시적 Legacy 경로는 아직 존재한다**. 다음 제거 경계는 남은 Legacy API·소비자와 구현 전용 테스트의 정리다. 해당 파일 삭제 후 빌드/Studio 검증이라는 최종 종료 조건은 아직 미완료다.
- 변경 integration target lint: `CARGO_BUILD_JOBS=2 cargo clippy --locked -p rhwp --test regression_suite_025 --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings` **PASS**. 로그 `output/7353/paragraph-paint/clippy-tests.log`.

## 후속 묶음 — 편집 캐시와 개체 조회 소비자 분리

- 직전 공통 텍스트 paint 묶음은 로컬 `efa8a3257`로 보존했다. 원격 push는 하지 않는다.
- 실제 잔여 호출: 셀 replace/delete는 V2에서도 `table_layout.rs`의 nested-text predicate와 cell-units 지문을 미리 계산한 뒤, 캐시 무효화에서만 Legacy owner 존재를 검사했다. 이제 기존 Legacy owner가 있을 때만 opaque 편집 토큰을 만들고 소비한다. 표 알고리즘·캐시 규칙을 공통 계층으로 복사하지 않는다.
- 값의 소비: authoritative IR 편집 → 기존 셀 reflow/vpos 갱신 → V2 generation 무효화/재조판은 유지한다. Legacy 토큰은 별도의 memoized cell-units 무효화만 담당한다. V2의 줄 높이/컷/점유/배치 조건을 변경하지 않는다. immediate/deferred 동일 편집의 최종 SVG와 control-layout JSON 동등성, 이전 V2 generation 거부, 앞뒤 내용 보존으로 검사한다.
- 개체 조회는 최종 RenderNode의 wrap plane/z-order/source DocPath → 공통 `page_paint::paper_node_sort_key` → paint 정렬 및 Studio hit-test JSON을 소비한다. Legacy 엔진의 정적 메서드 호출을 제거하며 node id를 정렬 근거로 쓰지 않는 기존 규칙은 그대로 유지한다. 기존 `#4334` 단위 계약은 공통 함수를 직접 검사한다.
- 아직 사용 중인 Legacy cell-units 테스트는 삭제하지 않는다. 해당 구현이 실제 제거되는 단계에서 함께 정리한다. 이번 묶음 역시 `table_layout.rs` 물리 삭제 완료가 아니다.

### 편집 소비자 분리 검증 결과

- 검증 소스: `efa8a3257` 이후 미커밋 변경. Rust 변경 파일 7개의 해시 목록은 `output/7353/edit-owner/source-sha256.txt`, 목록 SHA256은 `34e931c7c658bc31ee6c503b4358963f00d939c35545a9b9231a6ee0fe6f6912`이다. 검증 harness `scripts/verify-product-v2-wasm.mjs` SHA256은 `45eabd2f1c79dbc13513ca05d617fb9a64b666728df9405ebc7a67d69160d1cd`이다. 검증 후 Rust 소스는 변경하지 않았다.
- 집중 회귀: `CARGO_BUILD_JOBS=2 cargo nextest run --locked -p rhwp --lib --test regression_suite_025 --test regression_suite_012 --cargo-profile release-test --target-dir /home/edward/mygithub/rhwp/target/pr-review -E 'test(issue_7353) or test(issue2214_deferred) or test(issue_4334_paper_node_sort_key)' --test-threads 4 --no-fail-fast` → **52 PASS / 0 FAIL / 4312 skipped**. 로그 `output/7353/edit-owner/focused.log`. 새 계약은 V2/Legacy 각각의 immediate/deferred 편집 결과 동등성을 확인하는 구조 변경 보호이며, 수정 전 실패하는 조판 결함 검출이라고 주장하지 않는다.
- 변경 integration target Native Clippy와 WASM lib Clippy(`--locked ... -- -D warnings`) **PASS**. 로그 `clippy-native-tests.log`, `clippy-wasm.log`. `rust-unit-test-tiers.mjs --check --base-ref 02530b9ed567a44663edb26c65fb565c4a79f00d` **PASS (4205 tests / 298 modules)**. 포맷, diff 공백, JS 문법 검사도 통과했다. 전체 workspace lint·전체 회귀·원격 CI를 수행한 결과는 아니다.
- Docker fresh WASM: `docker compose --env-file .env.docker -f docker-compose.yml -f output/7353/wasm-product/final/compose-network.yml run --rm --no-deps -e CARGO_BUILD_JOBS=2 wasm` **PASS (8분 14초)**. 로그 `output/7353/edit-owner/wasm-build.log`. WASM SHA256 `82d716b50f34031b086e73ee5945d344e2ff7cf2065fb66e3671dacf7d901ee6`; Studio의 실제 Vite 파일 URL `/@fs/home/edward/mygithub/rhwp-task-7353/pkg/rhwp_bg.wasm` 제공 내용과 일치한다.
- `node output/7353/edit-owner/verify.mjs` **PASS**: 정상 한컴 저장 master/stories/wrap/body/table **5종 15페이지**의 Native tree/layer/SVG가 직전 `paragraph-paint/after`와 일치하고 fresh WASM 출력도 Native와 일치한다. 입력과 기준 PDF는 앞 묶음의 동일 fixture를 재사용했다. 세부 입력/JS/WASM 해시는 `after/*/browser-manifest.json`, 실행 로그는 `verification.log`에 있다.
- WASM 셀 편집 harness에 지연 삭제·원자적 문자열 교체를 추가했다. 이전 generation 거부, flush 후 immediate 삭제와 모든 페이지 SVG/control JSON 일치, 교체 후 텍스트 및 커서 위치 보존, 원래 편집 결과 복원을 확인했다(`after/table/editing.json`). flush의 `status: fallback`은 재개형 스케줄러에서 동기 `paginate()`로 전환했다는 뜻이며 Legacy 엔진 전환을 의미하지 않는다. V2 라우팅은 유지된다.
- Windows Chrome CDP 실제 Studio: `CHROME_CDP=http://localhost:19222 VITE_URL=http://localhost:7700 RHWP_V2_E2E_OUT=../output/7353/edit-owner/studio node e2e/product-v2.test.mjs --mode=host` **PASS**, 미처리 브라우저 오류 **0**. 본문/셀 실키보드 편집, undo/redo, 선택, 저장/재열기, 새 문서와 명시적 Legacy 대조군을 검사했다. 증적 `studio/result.json`, `studio/cell-edited.png`, `studio.log`.
- Native/실제 Canvas compare·standalone overlay·review를 새로 생성해 master 2쪽 Native/Canvas review, stories 5쪽 Canvas review, wrap 1쪽 Canvas overlay 및 Studio 셀 편집 캡처를 직접 확인했다. 본문 줄·외곽·어울림 영역·표 이어받기·뒤 문단의 배치 변화는 확인되지 않았다. 한컴 대비 기존 글꼴/획 차이는 남는다. 대표 증적: `output/7353/edit-owner/after/master/visual/canvas/review/review_002.png`, `after/stories/visual/canvas/review/review_005.png`, `after/wrap/visual/canvas/overlay/overlay_001.png`.
- 현재 묶음은 미커밋이며 원격 push하지 않았다. 다음 제거 대상은 남은 명시적 Legacy 진입점과 소비자 및 그 구현 전용 테스트다. **`table_layout.rs` 삭제 후 빌드/Studio 정상 동작이라는 최종 조건은 아직 미완료**다. 이번 결과를 전체 대체 완료로 간주하지 않는다.

### 커밋·게시 승인 후 사전 게이트 (2026-10-01)

- 사용자 커밋·push 승인에 따라 `upstream/devel`, `upstream/refactor/0.9.0`을 fetch했다. 정책 base는 `02530b9ed567a44663edb26c65fb565c4a79f00d`로 동일하며, refactor 원격에만 있는 커밋은 0개였다. devel merge/rebase는 하지 않았다.
- 현재 메인터너 작업 트리에서 파생 suite를 `node scripts/rust-test-suite-manifest.mjs --prepare`로 준비했다. `cargo fmt --all` 및 포맷 검사, Native root Clippy, WASM lib Clippy, workspace build를 순차 실행해 통과했다. Rust 소스 해시도 앞 검증과 동일함을 확인했다. 로그: `output/7353/edit-owner/push-clippy-native.log`, `push-clippy-wasm.log`, `push-workspace-build.log`.
- **push 차단**: `cargo clippy --locked --workspace --all-targets --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings`가 `regression_suite_018`의 `clippy::duplicate_mod`로 실패했다. `tests/cases/cli_catalog_contract.rs:11`과 `tests/cli_json_contract.rs:12`의 `#[path]`가 같은 `src/cli/catalog.rs`를 같은 suite 안에서 각각 로드한다. 로그 `output/7353/edit-owner/push-clippy-all-targets.log`.
- 원인 경계: 두 source는 정책의 `moduleIntegrationOverrides`에서 `path_attr/root_mod` 통합을 허용하고 있다. 배정기의 `sourceMetrics → lightestSuite → assignSources`는 크기/테스트 수로 suite를 선택하지만 공통 path 모듈 충돌은 검사하지 않는다. 이번 prepare 결과 두 계약이 같은 suite에 배정되었다. 렌더링 실행 실패로 분류하지 않으며, 임의 suite 번호 지정·generated 파일 직접 수정·Clippy 경고 억제로 우회하지 않았다.
- 선행 실패 때문에 뒤의 manifest/unit-tier 정책 재검사는 실행되지 않았다. 앞서 통과한 정책 검사와 이번 전체 lint 실패를 구분한다. 검증 자료를 포함한 현재 변경은 로컬 커밋으로 보존하되 **원격 push 및 그 이후의 Legacy 제거 작업은 진행하지 않는다**. 테스트 배정기 수정은 렌더러 의존 분리와 다른 변경 범위이므로 작업지시자의 방향을 확인한다. 개인 `.agents/skills/`, `.codex/`와 파생 테스트 파일은 커밋 대상에서 제외한다.

### 승인된 게시 차단 수정

- 앞 구현은 `dff21c670`으로 로컬 보존했다. 사용자가 suite 배정기 수정을 승인했다.
- `sourceMetrics`가 문자열 리터럴 `#[path = "..."]`의 경로를 source 위치 기준으로 정규화하고, 신규 배정과 전체 재배정 모두 같은 모듈 경로를 이미 가진 suite를 후보에서 제외한다. 기존 크기·테스트 수 가중치는 호환 후보 사이에서 유지한다. suite를 임의로 늘리거나 특정 테스트 이름/번호를 고정하지 않는다. 분리할 후보가 없으면 명시적으로 실패하며 테스트를 누락하지 않는다. 일반 Rust AST/매크로/전이적 모듈 해석기 구현은 이번 범위가 아니다.
- 회귀 입력: 가벼운 suite에 동일 모듈을 사용하는 기존 source, 무거운 정상 대조 suite, `../src/catalog.rs`와 `../src/./catalog.rs`라는 동치 경로를 가진 신규 source. 배정·재배정·후보 고갈 3건은 수정 전 FAIL, 수정 후 PASS. 같은 파일명이지만 다른 경로인 정상 대조군은 전후 PASS. 배정기 전체 **27 PASS / 0 FAIL**. 로그 `output/7353/edit-owner/suite-before.log`, `suite-after.log`.
- 실제 `--prepare` 결과 `cli_catalog_contract`는 suite 017, `cli_json_contract`는 suite 018로 분리됐다. **1472 sources / 6765 static test attrs / 28 suites + 20 exceptions**는 유지된다. 파생 파일·정책 허용치·Clippy 경고 설정을 직접 변경하지 않았다.
- 후속 all-target 검사에서 발견한 `paragraph_layout.rs`의 고아 문서 주석은 이전에 이동한 `paragraph_paint/helpers.rs`의 실제 함수 3곳으로 옮겼다. 함수 본문·테스트 assertion·조판 조건 변경은 없다. 따라서 기존 fresh WASM/시각 검증을 재실행한 것으로 보고하지 않으며, 이번 Rust 변경은 주석 이동으로 한정한다.
- 최종 검증: 배정/격리/실행 잠금 Node 계약 **31 PASS / 0 FAIL** (`suite-related-final.log`). fmt 검사, Native root Clippy, WASM lib Clippy, workspace build, workspace all-target Clippy **모두 PASS** (`push-clippy-{native,wasm,all-targets}-final.log`, `push-workspace-build-final.log`). 고정 base `02530b9ed567a44663edb26c65fb565c4a79f00d` 대비 manifest 검사와 source unit-tier 검사도 **PASS (4205 tests / 298 modules)** (`push-manifest-final.log`, `push-unit-tier-final.log`). 로그는 모두 `output/7353/edit-owner/` 아래에 있다. 전체 회귀 재실행이나 원격 CI 성공을 의미하지 않는다.
- 게시 차단은 해소됐다. 이 보정과 앞서 보존한 로컬 커밋을 승인된 `upstream/refactor/0.9.0`으로 일반 fast-forward push한다. devel 병합·PR 생성·테스트 제외·생성 파일 커밋은 하지 않는다.

## 후속 묶음 — V2 공통 primitive의 Legacy 컴파일 의존 분리

- 시작점 `f7e66313b8e91d4c22d5c769ba6a1bb31251c0ff`는 `upstream/refactor/0.9.0`에 게시된 상태다. 승인된 §5.7의 공통 primitive 분리를 진행한다. Legacy 표 알고리즘을 다른 이름으로 옮기는 작업이 아니다.
- `CellContext/CellPathEntry`의 정의·주소 조회를 `renderer/cell_context.rs`로 옮겼다. V2 ownership과 RenderTree, 공통 paragraph paint가 직접 소비한다. 기존 `layout::CellContext`는 동일 타입 재수출로 유지하며 주소/JSON 형식이나 기하를 변환하지 않는다.
- 순수 글자 측정·클러스터·스타일 함수와 그 기존 테스트는 `renderer/text_measurement.rs`, BinData·번호·도형 스타일/좌표 변환은 `renderer/paint_resources.rs`로 이동했다. `layout` 내부 호출자는 같은 구현을 재수출로 사용한다. 현행 진단 도구의 source 경로는 새 위치로 연결하고 역사적 W1 snapshot의 경로/해시는 보존한다. 삭제된 이전 경로는 drift 검사에서 `actualSha256: null`로 보고된다.
- 테두리의 폭·다중선 분해·LineNode 생성만 `renderer/border_paint.rs`로 추출했다. Legacy 표 그리드 수집·셀 대각선 선택·병합/분할 처리는 기존 모듈에 남는다. 기존 상수·경계·계산식은 변경하지 않는다.
- 소비 연결: 공통 IR 소유 주소 → V2 확정 노드의 CellContext → RenderTree/편집 조회; resolved text style → 공통 측정/문단 paint → 최종 글자 위치; BorderLine → 공통 선 폭/노드 → 페이지·문단·표 paint. 이번 변경은 소유 모듈/import 변경이며 컷·요구 높이·페이지 예산·앵커/원점 변경은 비해당이다.
- `issue_7353_product_v2`의 구조 보호에 공통 primitive와 잔여 V2 소비자를 포함했다. 별도 계약으로 중첩 주소·이미지 주소·빈 경로·JSON 및 이전 공개 타입의 동일성을 검사한다. 기존 제품 출력/편집 계약도 실행하며 구조 assertion만으로 동작을 입증하지 않는다. 조판 결함 수정이나 한컴 차이 개선을 주장하는 변경은 아니다.
- 검증 결과는 `output/7353/shared-primitives/`에 보존한다. unit-tier 도구는 rename을 작업 트리가 아니라 `base..HEAD`에서 읽으므로 미커밋 이동 중에는 신규 모듈로 오인한다. 규칙 완화 없이 로컬 보존 커밋 후 동일 base 대비 검사한다.
- 최종 남은 범위: V2 미지원 host/control 조합의 기능 대체, 명시적 Legacy API/TypesetEngine 소비자 및 구현 전용 테스트 제거, `table_layout.rs` 실제 삭제 상태의 제품 검증. 이 공통 의존 분리를 최종 제거 완료로 보고하지 않는다.

### 공통 primitive 분리 검증

- 검증 소스: `f7e66313b8` 이후 변경. `source-sha256.txt` 목록 SHA256은 `107e1a1389f902d07719f462b1fbfc41d4674119d824283189056c0b656fd692`. 측정/리소스 함수 및 테스트, 선 primitive, 셀 주소의 본문은 import·공백·rustfmt 후행 쉼표 외에 동일하다(`extraction-check.log`). 조판 상수·기준값·ignore 변경은 없다.
- `CARGO_BUILD_JOBS=2 cargo nextest run --locked -p rhwp --lib --test regression_suite_027 --test regression_suite_020 --cargo-profile release-test --target-dir /home/edward/mygithub/rhwp/target/pr-review -E 'test(issue_7353_product) or test(renderer::text_measurement::tests) or test(renderer::paint_resources::tests) or test(renderer::layout::border_rendering::tests)' --test-threads 4 --no-fail-fast` → **92 PASS / 0 FAIL / 4267 skipped**. 제품 편집/저장/주소와 이전 측정·리소스·테두리 대조군을 포함한다(`focused.log`). 전체 회귀 결과는 아니다.
- fmt, diff 공백 검사 및 Native lib Clippy, WASM lib Clippy 통과. manifest는 고정 base `02530b9ed567a44663edb26c65fb565c4a79f00d` 대비 **1472 sources / 6766 static test attrs / 28 suites + 20 exceptions**로 통과했다. 신규 integration 계약 1건 증가이며 source-side unit 테스트를 추가하지 않았다. 전체 workspace all-target lint/원격 CI는 이번 묶음에서 재실행하지 않았다.
- fresh WASM 표준 Docker 빌드 **PASS, 8분 26초**. 앞 묶음과 같은 compose 명령이다. WASM SHA256 `4f2a880a8d95abe04b622f97fbcf4d982eb6ff8b689046189659b02c89f7c534`; 실제 Studio `:7700/@fs/home/edward/mygithub/rhwp-task-7353/pkg/rhwp_bg.wasm` 응답과 일치한다(`wasm-build.log`).
- `node output/7353/shared-primitives/verify.mjs`: 동일 정상 한컴 저장 master/stories/wrap/body/table **5종 15페이지** Native tree/layer/SVG가 `edit-owner/after`와 완전히 일치한다. fresh WASM도 Native와 일치하며 본문/셀 편집·성장·deferred/atomic 편집·글꼴 등록 대조군이 통과했다(`verification.log`, `after/*/browser-manifest.json`). 입력/기준 PDF를 다시 만들거나 바꾸지 않았다.
- Native와 실제 Canvas compare·standalone overlay·review를 새로 생성했다. master 2쪽 Native/Canvas review, stories 5쪽 Canvas review, wrap 1쪽 Canvas overlay, Studio 셀 편집 캡처를 직접 판독했다. 본문/바탕쪽 외곽, 빈 줄, 어울림 영역, 이어받는 셀/뒤 문단은 유지되며 기존 한컴 대비 글꼴·획 차이는 남는다. 대표 증적: `after/master/visual/canvas/review/review_002.png`, `after/stories/visual/canvas/review/review_005.png`, `after/wrap/visual/canvas/overlay/overlay_001.png`.
- `rhwp-studio`에서 `CHROME_CDP=http://localhost:19222 VITE_URL=http://localhost:7700 RHWP_V2_E2E_OUT=../output/7353/shared-primitives/studio node e2e/product-v2.test.mjs --mode=host` **PASS**, 미처리 브라우저 오류 **0**. 본문/셀 키보드 편집, undo/redo, 커서/선택, 저장/재열기, 새 문서, 명시적 Legacy 대조군을 확인했다(`studio/result.json`, `studio/cell-edited.png`).
- 진단 도구 검증: font trace **12 PASS**. ledger evidence/kerning 도구 **10 PASS / 1 FAIL**. 실패 1건의 3014 오류는 모두 과거 문서 `task_m100_4939.md`와 `font_metrics_fallback_causal_lineage_20260816.md`의 참조 부재이며 두 문서 모두 시작 HEAD에도 없다(`ledger-baseline-check.log`). 이번 이동으로 생긴 새 source/test 경로 오류는 없다. 과거 증적을 재작성하거나 오류를 억제하지 않았다. 도구 전체 통과나 전체 작업 완료로 보고하지 않는다.
- 변경 integration target Clippy(`--test regression_suite_027 --test regression_suite_020 -- -D warnings`)도 **PASS** (`clippy-tests.log`). 현재 묶음은 로컬 커밋으로 보존하며 원격 push하지 않는다. 다음 구현 대상은 V2 host/control 조합의 실제 로드 거부이며, 같은 primitive 분리를 재실행하지 않는다.
- 로컬 보존 커밋 후 동일 base의 unit-tier 검사 **PASS: 4205 tests / 298 modules** (`unit-tier-final.log`). 커밋된 rename을 정상 인식했으며 정책/래칫 완화는 없다. 이후 수정은 이 결과 기록뿐이며 검증한 Rust 소스는 그대로다.
