# Task #7353 — 별도 분할선 속성의 보존과 지원 경계

- 선행 커밋: `b50e0182a` ([stage14](task_m100_7353_stage14.md)).
- 상태: 승인 범위 구현·집중 회귀·fresh WASM 검증 완료. R3 전체 완료는 아니다.
- Legacy 조판·V2 기하·분할 위치는 변경하지 않았다.

## 근거와 범위

로컬 `mydocs/manual/OWPML SCHEMA/Header XML schema.xml`의 `breakCellSeparateLine`은
boolean/default=false이며 자동으로 나뉜 표의 경계선 설정이다.
[한컴 공식 모델](https://github.com/hancom-io/hwpx-owpml-model/blob/main/OWPML/Class/Head/BorderFillType.cpp)의
생성자·ReadAttribute·WriteElement도 false 초기화와 boolean 읽기/쓰기를 확인한다.
이는 속성 보존의 근거이지 활성화된 선의 실제 모양/우선순위 근거가 아니다.
HWP5 공개 사양의 BorderFill 비트표에는 대응 비트가 확인되지 않아 추측해 추가하지 않는다.

기존 파서는 속성을 버리고 저장기는 항상 0을 쓴다. HWPX → BorderFill → HWPX와
ResolvedBorderStyle에서 값을 보존한다. V2의 문서 세션과 resolved-style 직접 진입 모두
선택한 표/셀/중첩 자식의 활성 속성을 명시적 미지원으로 거부한다. 미사용 스타일은 거부하지
않는다. 모든 선 None, 한 쪽에 들어가는 표도 속성 자체의 지원 여부는 동일하다.
HWP/HML 저장은 대응 구현이 없으므로 활성 속성의 무음 소실을 허용하지 않는다.
Legacy paint의 별도 분할선 구현과 HWP 대응 비트 조사는 별도 후속 범위다.

## 검증 계약

입력은 저장 LineSeg 없는 합성 IR을 HWPX로 생성한 뒤 header XML의 해당 속성만 변경한다.
수동 저장 줄 메트릭을 근거로 수용 조건을 완화하지 않는다. true/1, false/0, 속성 생략을
독립 boolean 계약으로 검사하고, 원본 XML→IR→XML→IR 및 resolved-style 진입을 검사한다.
새 검사는 tests/cases에 배치하며 수정 전 의도한 실패/수정 후 통과를 기록한다.
기존 V2·Legacy 대조군, fresh Docker WASM의 명시적 거부와 정상 41쪽 출력을 재확인한다.
본 절편은 지원 경계 보강으로, 활성 별도 분할선의 한컴 피델리티나 R3 전체 완료가 아니다.

## 실제 소비 경로

`parser/hwpx/header.rs::parse_border_fill` → `model/style.rs::BorderFill` →
`serializer/hwpx/header.rs::write_border_fill` 및
`renderer/style_resolver.rs::resolve_single_border_style`에서 동일 bool을 전달한다.
원래 attr 비트는 변경하지 않는다. 문서 세션은 `session.rs::from_document`에서
`decoration.rs::validate_source`로 선택 표·셀·자식을 검사한 뒤 prepare한다.
직접 `PreparedTextTable::prepare`는 `text_ir.rs::bind_paint`의 table background,
cell background, 재귀 child bind가 `Background::resolve_inner`를 통과한다.
해소된 스타일에서도 속성을 보존/거부하므로 DocInfo 검사를 우회할 수 없다.
거부는 첫 페이지 출력 전에 발생하며 별도 선을 기본 실선처럼 출력하거나 Legacy로 fallback하지 않는다.
사용하지 않는 style은 preview에서 검사하지 않는다.

`serializer/cfb_writer.rs::serialize_hwp_inner`에서 일반/보고서/비밀번호 문서 저장 공통으로
활성 속성을 거부한다. 원본 DocInfo 재사용 전에 검사한다. HML은 기존 preflight의
border-fill 생략 blocker에 이 필드를 추가했다. 문서 전체 저장은 미사용 서식도 보존 대상이므로
preview와 달리 활성 속성이 있는 서식 전체를 검사한다. HWPX 저장만 해당 속성을 지원한다.
HWP5 원본은 기존 raw attr를 유지하고 새 필드는 false로 초기화한다. 대응 binary bit의
정확한 판독/수정은 미검증이며 활성 별도 선의 Legacy 출력 역시 구현하지 않는다.

컷·요구 높이·예약·이월·실제 배치의 수식과 호출 경로는 [stage14](task_m100_7353_stage14.md)와
동일하다. 이번 변화는 지원 수용 조건과 서식 보존이며 분할 예산 알고리즘 변경은 비해당이다.
생성자와 기존 source-side 테스트의 구조체 literal은 새 bool을 false로 초기화하는 것만 변경했다.
새 검사는 전부 `tests/cases/issue_7353_table_v2_split_line_property.rs`에 있다.

## 수정 전후 집중 검사

`b50e0182af604571d4afefbe74a836d0524d4c4c`의 구현에 새 계약8건을 실행했다.
빌드 성공 후 **6 FAIL / 2 PASS** (`output/7353/r15/before-final-tests.log`).
실패는 활성 속성의 IR 소실, writer의 상수0, source/resolved V2 잘못된 수용,
HWP/HML의 무음 소실이고, 생략/false/0 및 미사용 스타일 대조군은 통과했다.
수정 후 **8 PASS** (`after-focused.log`). source 거부 검사 뒤 같은 입력의 resolved 직접
진입에도 표/중첩 경계를 추가 검증했다. 후자의 추가 assertion은 수정 후 실행 증거이며,
수정 전 실패는 source 거부 assertion과 기존 resolved 셀/정책 assertion에서 관측했다.
실행하지 않은 별도 선의 형상이나 한컴 일치까지 검증했다고 해석하지 않는다.

## 최종 검증 결과

증적: `output/7353/r15/`. 실행 소스는 위 선행 HEAD + 이번 변경이며 `source.sha256`의
42개 소스/테스트/하네스 해시가 review overlay와 모두 일치한다(`review-source-match.log`).
Docker 시작 시점의 `compiled-source.sha256`에 기록한 production source는 빌드 이후에도
동일하다. 정책 비교 base는 `7a95e46e025470a4d7a7b59ad68ec02958bda738`로 고정했다.

- V2 114 + Legacy #5301 3 + HML 저장 31 + IR field sweep 4 = **152 PASS** (`selected-tests.log`).
- IR 전체 왕복 래칫 PASS(122.104초). 기준 581경로 대비 현재260경로, 증가0·감소321
  (`ir-fields.tsv`, `ir-fields.diff`, `ir-fields-summary.json`). 감소를 이번 변경의 개선 효과로
  귀속하지 않으며 기준 TSV는 수정하지 않았다. 동일 값이라는 뜻이 아니라 래칫 충족이다.
- Native lib / WASM lib / 관련 integration3 targets Clippy 모두 PASS
  (`clippy-native.log`, `clippy-wasm.log`, `clippy-tests.log`).
- fmt, 고정 base manifest, source-side unit tiers PASS (`fmt.log`, `policy.log`, `unit-policy.log`).
- frontend bindings 1 PASS (`bindings.log`), JS 구문·문서 상대 링크·diff 공백 검사 PASS.

review worktree에서 `--prepare`를 수행한 뒤 실행했다. 실제 전체 case/target/filter는
`selected-tests.log` 선두에 기록했다. 새 테스트의 target은 source 크기 변화에 따라 바뀌므로
focused 재현은 `run-rust-test.mjs`로 해석한다.

```sh
node scripts/rust-test-suite-manifest.mjs --prepare
ISSUE7353_EXPORT_DIR=/home/edward/mygithub/rhwp-task-7353/output/7353/r15/fixtures \
  node scripts/run-rust-test.mjs issue_7353_table_v2_split_line_property -- \
  --cargo-profile release-test --target-dir /home/edward/mygithub/rhwp/target/pr-review --no-fail-fast
cargo clippy --locked -p rhwp --lib --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo clippy --locked -p rhwp --lib --target wasm32-unknown-unknown \
  --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo clippy --locked --test regression_suite_018 --test regression_suite_007 \
  --test regression_suite_002 --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo fmt --all -- --check
node scripts/rust-test-suite-manifest.mjs --check --base-ref 7a95e46e025470a4d7a7b59ad68ec02958bda738
node scripts/rust-unit-test-tiers.mjs --check --base-ref 7a95e46e025470a4d7a7b59ad68ec02958bda738
```

## Fresh WASM과 직접 시각 확인

```sh
docker compose --env-file .env.docker -p rhwp run --rm wasm
VISUAL_SWEEP_CHROME=/home/edward/.cache/puppeteer/chrome/linux-146.0.7680.31/chrome-linux64/chrome \
  node scripts/verify-table-v2-preview-wasm.mjs --pkg pkg \
  --fixtures output/7353/r15/fixtures --out output/7353/r15/browser \
  --dependencies-root /home/edward/mygithub/rhwp --solid-backgrounds --solid-borders \
  --split-borders --split-line-property
```

Docker release + wasm-opt **8분04초 성공** (`docker-wasm.log`). WASM SHA-256은
`1b8a00ab7c1651c24f797996fb4c72c298e548916a67ff98b1c65a588120b481` (`pkg.sha256`).
실제 Chrome146.0.7680.31에서 기존 **41쪽 PASS**, Native/WASM tree와 SVG exact parity,
픽셀·독립 세션·오류 rollback·종료를 확인했다. 신규 `separate-cell/table/child` 3입력은
첫 페이지 전에 정확한 별도 분할선 Unsupported 메시지로 거부됐다(`browser.log`,
`browser/manifest.json`). 비활성/미사용 속성의 세부 계약은 Native8건에 포함된다.

이번에 생성한 `browser/cut-lines-{0,1}.review.png`, `cut-nested-1.review.png`를 직접 열어
두 쪽의 닫힌 셀 경계, 빈 줄의 높이, 이어진 B/C와 자식 C 뒤 host 위치를 확인했다.
Native/WASM 차이는 없으며 standalone overlay·양쪽 PNG·SVG/JSON도 함께 보존했다.
새 review PNG41장은 stage14의 직접 판독 증적과 byte-identical이다
(`previous-visual-compare.log`). 기존 파일을 새 증적으로 복사하지 않았다.
이는 기본 분할선 경로의 무변화/출력 동등성 확인이다. 활성 별도 분할선은 출력하지 않으므로
해당 선의 시각 통과, 실물 한컴 문서의 전체 피델리티나 사용자 시각 승인을 주장하지 않는다.

## 잔여 범위

이번 속성 보존·명시적 거부 계약은 충족했다. 활성 별도 선의 실제 paint, HWP5 대응 비트,
상이한 공유선 우선순위·표 자체 선·rowspan 등 R3 잔여 구현은 남아 있다.
기본 Studio/Legacy 전환, 전체 workspace/CI·Skia 검증, push/PR/이슈 종료는 수행하지 않았다.
새 generated suite/manifest나 baseline을 source 변경으로 포함하지 않았다.
