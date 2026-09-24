# Task #7353 — V2 셀 내부 분할의 실선 경계

- 선행 커밋: `079a39f1c` ([stage13](task_m100_7353_stage13.md)).
- 상태: 승인 범위의 구현·집중 회귀·Native/fresh WASM 비교 완료. R3 전체 완료는 아님.
- Legacy·분할 위치·요구/예약 높이·문서 전체 엔진은 변경하지 않는다.

## 구현 전 근거와 범위

독립 출력은 `samples/task3236/issue3236_split_table.hwpx`와
`pdf/task3236/issue3236_split_table-hwpx-2020.pdf`다. 기존 대응 기록은
[task3236 stage3](archives/task_m100_3236_stage3.md)이다. PDF Creator는 Hwp 2022이고
2쪽이다. 두 쪽을 PNG로 내보내 직접 확인했다: 첫 쪽 끝과 다음 쪽 시작 모두 수평선이 닫힌다.
원본 XML의 바깥 1×1 표는 `pageBreak=CELL`, 셀 borderFill12는 좌우0.12mm·상하0.3mm
실선, `breakCellSeparateLine=0`이다. 오래된 테스트 설명보다 실제 원본 속성을 우선한다.
페이지 경계에서 셀 조각의 네 변을 그린다는 근거이며 모든 한컴 서식에 대한 일반화는 아니다.
상이한 공유선 우선순위와 table-level 선은 계속 명시적 미지원이다.
별도 분할선 속성은 미지원/미검증이다. 특히 기존 HWPX `parse_border_fill`은
`breakCellSeparateLine`을 IR에 보존하지 않는다. `validate_source`는 IR의 attr/effect만
거부할 수 있으므로 이 XML 속성까지 검출한다고 주장하지 않는다. 이번 입력은 해당 속성0의
독립 기준과 기본 합성 IR로 한정하며, 실물 입력 일반 수용 전 파서/IR 계약 보강이 필요하다.

신규 계약 입력은 저장 LineSeg 없는 합성 IR을 HWPX로 serialize/parse하여 생성한다.
기대 좌표는 18px 줄, 명시된 padding·최소 높이·36px 예산에서 독립적으로 정한다.
이 합성 입력의 Native/WASM 일치를 원본 #3236 전체의 한컴 피델리티로 보고하지 않는다.

## 공통 결과와 소비 경로

`text_ir::bind_paint → CellBorders::prepare`에서 CellBreak의 실선 서식을 허용한다.
`fragment::fit_rows(WithinCells) → FlowCursor::fit → CellPlacement.bounds →
TextPaint::build_node → CellBorders::append → LineNode`는 기존 경로 그대로 사용한다.
paint가 소비한 내용 컷이나 높이를 추측하지 않고 실제 수용한 물리 사각형을 닫는다.
내용이 끝난 이웃 셀/빈 밴드도 양의 물리 높이를 가지면 경계를 유지한다.
`fit_with_header`의 반복 제목은 물리 높이를 다시 예약하지만 본문 유닛을 되감지 않는다.
중첩 자식은 자신의 확정 조각과 paint를 재귀 소비하며 부모 컷을 재계산하지 않는다.
한쪽 변이 None이면 컷에서도 해당 변을 추가하지 않는다. 동일 공유선 합치기와 충돌 거부를 유지한다.

원래 높이만 fit하는 예산·유닛 소비·padding·최소 높이·이월·종료 알고리즘은 무변경이다.
이번 검사는 같은 분할 결과에 선을 추가해도 줄 소유, 물리 밴드, 후속 내용이 보존됨을 확인한다.
rowspan·행 간격·저장 LineSeg·TAC/어울림은 여전히 지원 범위 밖이다.

## 검증 계획

`tests/cases/`에 실제 최종 선/셀/글줄/종료 계약을 추가한다. 단계13 코드에서 CellBreak
명시적 거부로 실패하는지 먼저 확인하고 수정 뒤 재검증한다. 제목·colspan, 중첩, 빈 줄,
내용 이후의 물리 높이, 한쪽 선 없음, 공유선 충돌과 rollback을 검사한다.
기존 미지원 테스트 중 CellBreak 자체를 거부하던 기대는 새 지원 계약으로 대체하며,
복합선·표 자체 선 등 나머지 미지원 검사를 유지한다. baseline/golden은 완화하지 않는다.
선택 회귀·Native/WASM lint·fresh Docker WASM·Chrome compare/overlay/review를 수행한다.
전체 CI 게이트는 계획의 후속 단계이며 이번 집중 검증과 구분한다.

## 경계 실행 결과

기준 코드 `079a39f1c236cf5540718abe2585a46a19eb6d63`에서 신규7건 모두 빌드 성공 후
`Unsupported("V2 borders across cell cuts")`로 실패했다(`output/7353/r14/before.log`).
변경 후7건 모두 통과했다(`after-focused.log`). 결함 수정 전 FAIL로 확대하지 않고,
명시적 미지원이 새 지원 경계로 전환된 실행 증거로 해석한다.

| 정식 계약 / fixture | 독립 기대값과 실제 검사 |
| --- | --- |
| `cut-lines` | 18px 줄4개(빈 줄 포함)를2쪽에2개씩. 좌우10px padding은 글줄에만 적용. 경계 x20..220, y30..66 |
| `cut-tail` | 최소90px=36+36+18. 글자 A는 첫 조각 한 번만, 나머지는 빈 물리 밴드의 닫힌 선.3쪽 뒤 종료 |
| `cut-siblings` | L이 먼저 끝나도 오른쪽 A..D와 같은36px 물리 셀. 다음 조각에도 왼쪽 셀 유지, 공유 세로선1개 |
| `cut-header` | 18px 병합 제목 반복 + 본문18px. L/A→B→C를3쪽에 배치. 제목 안에는 가짜 내부 세로선 없음 |
| `cut-nested` | 부모/자식 모두 CellBreak. A/B→C/host→after.2쪽의 자식18px·부모36px,3쪽 부모18px로 각각 닫힘 |
| `cut-padding` | 위18+본문36+아래18=72px. A는1쪽 y48, B는2쪽 y30. top=None인 선은 컷에서도 생성하지 않음 |
| conflict | 좌우 셀의 공유선 굵기 충돌은 paint 오류. 반복 호출에도 emitted=0, cursor를 소비하지 않음 |

실제 컷/예약 경로는 `fragment.rs:158`의 `fit_rows`에서 `flow.rs:26`의 각 셀 fit을 소비한다.
요구 높이를 수용한 조각만 `used`에 계상하고, 내용이 끝나면 `minimum_left`를 별도 물리 밴드로
예약한다(`fragment.rs:234`). 셀·표 bounds와 이어받기는 같은 used에서 나온다(:248~276).
제목은 `fit_with_header`(:96), 중첩은 `FlowCursor::fit`의 child cursor로 동일 결과에 연결된다.
최종 출력은 `text.rs:142`의 `build_node → borders.append`(:211)이며, 여기에서 높이/컷을
바꾸지 않는다. `session.rs:199`와 `export.rs:78`의 성공 후 commit도 그대로다.
rowspan 등 미지원 경로를 실행한 것으로 세지 않는다.

기존 `unsupported_cut_table_and_complex_edges...`의 CellBreak 거부는 신규7건의 긍정/반례
계약으로 대체했다. table-level/복합선 거부는 유지했다. export의 `fill-border` 부정 fixture는
Solid→Dash로 바꾸고 parse 후 Dash 보존까지 검사한다. 기대 페이지 수·golden·래칫은 바꾸지 않았다.

## 선택 검증

증적은 모두 `output/7353/r14/`에 있다. review worktree에서 파생 suite를 준비했다.
policy base는 `7a95e46e025470a4d7a7b59ad68ec02958bda738`으로 고정했다.

- 신규7 + 기존 V2 99 + Legacy #5301 3 = **109 PASS** (`selected-tests.log`).
- Native library / WASM library / 변경 integration targets Clippy **PASS**
  (`clippy-native.log`, `clippy-wasm.log`, `clippy-tests.log`).
- fmt / 고정 base manifest **PASS** (`fmt-final.log`, `policy.log`).
- source-side cfg(test)·정책 baseline·파생 manifest를 source PR 대상으로 변경하지 않았다.

```sh
node scripts/rust-test-suite-manifest.mjs --prepare
ISSUE7353_EXPORT_DIR=/home/edward/mygithub/rhwp-task-7353/output/7353/r14/fixtures \
  node scripts/run-rust-test.mjs issue_7353_table_v2_split_borders -- \
  --cargo-profile release-test --target-dir /home/edward/mygithub/rhwp/target/pr-review --no-fail-fast
cargo clippy --locked -p rhwp --lib --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo clippy --locked -p rhwp --lib --target wasm32-unknown-unknown \
  --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo fmt --all -- --check
node scripts/rust-test-suite-manifest.mjs --check --base-ref 7a95e46e025470a4d7a7b59ad68ec02958bda738
```

109건 단일 nextest 호출의 실제 case·target·filter는 `selected-tests.log` 첫 부분에 있고,
integration Clippy도 `resolveCase`로 실제 세 target을 해석한 명령을 기록했다.
전체 workspace/CI·Native Skia 회귀를 수행했다고 보고하지 않는다.

## Fresh WASM과 직접 시각 판독

```sh
docker compose --env-file .env.docker -p rhwp run --rm wasm
VISUAL_SWEEP_CHROME=/home/edward/.cache/puppeteer/chrome/linux-146.0.7680.31/chrome-linux64/chrome \
  node scripts/verify-table-v2-preview-wasm.mjs --pkg pkg \
  --fixtures output/7353/r14/fixtures --out output/7353/r14/browser \
  --dependencies-root /home/edward/mygithub/rhwp --solid-backgrounds --solid-borders --split-borders
node --test scripts/frontend-wasm-bindings.test.mjs
```

Docker release + wasm-opt는 **8분06초 성공**(`docker-wasm.log`). 실제 Chrome146.0.7680.31에서
**41쪽 PASS**: 기존26쪽+신규15쪽의 tree/SVG exact parity, 경계 픽셀, 독립 세션, 오류 rollback,
종료를 확인했다(`browser.log`, `browser/manifest.json`). frontend 선언1건과 JS 구문도 통과했다.
기존26쪽의 새 review PNG는 stage13 직접 판독본과 byte-identical이다
(`previous-visual-compare.log`). 옛 캡처를 이번 결과로 복사하지 않았다.

`browser/cut-{lines,siblings,padding}-{0,1}.review.png`,
`browser/cut-{header,nested,tail}-{0,1,2}.review.png` **15장을 모두 직접 열었다**.
분할 양쪽의 닫힌 외곽선, None인 위 선의 부재, 제목 아래의 공유 경계, 먼저 끝난 왼쪽 셀,
자식 끝 뒤의 host/after,36/36/18 높이의 빈 밴드를 확인했다. Native/WASM 간 위치 차이는 없다.
각 페이지의 `*.native.png`, `*.wasm.png`, standalone `*.overlay.png`, SVG/JSON을 함께 보존했다.
0padding 합성 사례에서는 글자가 왼쪽 선에 닿는다. 여백 피델리티나 실물 문서의 시각 승인으로
해석하지 않으며, 이번 판독의 대상은 컷에서의 선·내용 소유·물리 높이와 backend 일치다.

독립 PDF의 원본/해시는 `reference/source.sha256`, 메타데이터는 `reference/pdfinfo.txt`,
직접 판독 PNG는 `reference/hancom3236-{1,2}.png`다. 한컴 PDF는 컷 경계 규칙의 근거이고
합성 Native/WASM overlay의 기준 이미지가 아니다. 원본 #3236의 V2 전체 조판은 미검증이다.

실행 소스는 위 HEAD + 이번 작업 변경이며 빌드 시작 시 `compiled-source.sha256`으로 고정했다.
이후 두 Rust 파일의 설명 주석만 갱신했다(온전한 셀 제한 설명 제거, IR에 표현된 효과의
검증 범위 명확화). `comment-only-check.log`는 주석 행을 제외한 소스가 동일함을 확인한다.
동작 변경이 없어 build/lint를 반복하지 않았고, 최종 fmt는 재확인했다. 최종 `source.sha256`은
review와 일치한다(`review-source-match.log`). 이 주석 변경을 실행 로직 재검증으로 세지 않는다.
실행 패키지는 `pkg.sha256`에 고정했다. WASM SHA-256:
`eb7eac577ff1a44eeb02ea2ba3d14d88463064acdc69c314dcaec8aab9c1f166`.

## 후속 범위

실험적 V2의 분할 셀 실선 계약은 충족한다. 기본 Legacy/Studio 엔진은 그대로이고 사용자
시각 승인·문서 전체 V2 전환·전체 CI/Skia·push/PR/이슈 종료는 수행하지 않았다.
다음에는 실물 입력 수용 전에 이번에 확인한 **별도 분할선 속성의 parser→IR 보존/검출 경계**를
먼저 보강한다. 그 뒤 상이한 공유선 우선순위·표 자체 선·rowspan 등 잔여 범위를 진행한다.
한컴 전체 피델리티 및 parser에서 소실되는 속성의 자동 거부는 미검증/미구현으로 남긴다.
