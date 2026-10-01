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
