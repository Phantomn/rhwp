# Task #7353 — 표·셀 외곽선의 일치 계약

- 선행 커밋: `bf9cc2308` ([stage15](task_m100_7353_stage15.md)).
- 상태: 일치 외곽선 지원·선택 회귀·fresh WASM 비교 완료. R3 전체 완료는 아니다.
- Legacy와 기본 Studio 경로는 변경하지 않았다.

## 범위와 독립 근거

표의 borderFill 실선을 셀의 None 위에 무조건 보충하지 않는다.
[#6311 조사](https://github.com/edwardkim/rhwp/issues/6311)는 이 방식이
`samples/KTX.hwp` / `pdf/KTX-2022.pdf`의 의도적으로 비운 상단에 선을 추가하는
반례를 기록한다. #6420의 부분 프레임 조건을 새 일반 규칙으로 복제하지 않는다.
이번 지원 범위는 표와 셀이 **동일한 실제 외곽선**을 선언하여 우선순위 판단이
필요 없는 경우다. 표 실선과 셀 None/서로 다른 실선의 관계는 명시적 미지원으로 남긴다.
표 None은 셀이 명시한 선을 제거하지 않는다. 미사용 스타일은 영향을 주지 않는다.

독립 기대값은 같은 좌표·스타일 선의 합집합, 선언된 18px 줄·36px 예산·최소 높이,
기존 stage14의 기본 셀 분할선 계약이다. 신규 입력은 저장 LineSeg 없는 합성 HWPX이며
이 실행을 #6311/KTX 원본의 한컴 피델리티나 일반 우선순위 확정으로 해석하지 않는다.

## 실제 소비 경로와 반례

`text_ir.rs:136 bind_paint → borders.rs:35 prepare`에서 표와 셀 서식을 함께 수집하고,
`fragment::fit_rows → FlowCursor::fit → TablePlacement/CellPlacement.bounds →
TextPaint::build_node → borders::append`에서 수용한 물리 조각의 외곽을 검증한다.
표/셀 원점과 높이를 다시 계산하지 않는다. 반복 제목은 원본 행 번호가 아닌 실제
fragment 행 순서를 사용하며 중첩 표도 동일 경로다. 마지막 빈 물리 밴드도 검사한다.
충돌·누락은 전체 재귀 tree 완성 및 session commit 전에 실패해야 한다.
내용 컷·요구/누적 예약 높이·예산 실패 이월·후속 내용·종료 알고리즘은 무변경이다.

검증은 전체/행 분할/셀 내부 분할, 가로 병합, 제목 반복, 중첩 뒤 host·후속 문단,
내용 종료 뒤 물리 밴드, None과 상이한 선의 거부 및 실패 시 cursor 보존을 포함한다.
기존 tests/cases에 정식 최종 좌표 검사를 추가하고 수정 전 미지원 FAIL/수정 후 PASS를
확인한다. 이는 기존 지원 결함이 아니라 신규 지원 경계의 증거다. Native/fresh Docker
WASM compare·overlay·review를 생성하고 직접 판독한다. 전체 CI/기본 엔진 전환은 별도다.

실제 높이 소비는 `fragment.rs:158 fit_rows`의 일반/WithinCells 분기와
`flow.rs:26 FlowCursor::fit`이다. 수용한 fit.height의 최댓값, 마지막 유닛 이후의
minimum_left 밴드(:234), CellPlacement.bounds(:248), 남은 minimum_left(:269)를
그대로 사용한다. 제목 반복은 fit_with_header(:96), 중첩은 child cursor의 재귀 결과다.
`text.rs:142 build_node`가 셀·자식 출력 뒤 `borders.append`(:211)를 호출하고,
`session.rs:199 next_page`는 append_to 성공 뒤에만 continuation과 emitted_pages를
갱신한다. JSON export(:78)는 복제 candidate의 직렬화까지 성공해야 commit한다.
모두 기존 경로이며 이번에는 border 집합의 일치 검사만 추가했다. rowspan/TAC/저장 줄
경로는 계속 미지원이며 이를 검증한 것으로 세지 않는다.

## 수정 전후 집중 검사

stage15 `bf9cc23085a212378f09bc1905c2b1e4f4d90306`의 코드에 새 계약5건을 적용하면
모두 빌드 성공 후 기존 table-border 미지원으로 실패했다(`output/7353/r16/before.log`).
이 중4건은 출력 지원,1건은 명시적 paint 거부/rollback 계약이다. 기존 지원 버그가
검출됐다는 뜻이 아니라 신규 지원 수용 경계가 실행된 증거다. 이어 추가한3건의 수정 전
실행은 하지 않았으므로 해당3건의 red→green 검출 증거는 주장하지 않는다.

첫 수정 후24건에서23 PASS/1 FAIL이었다(`after-focused.log`). 실패 입력은
`BorderFill::default()`가 None이라는 잘못된 합성 가정이었다. 실제 BorderLineType의
default는 Solid다. None을 명시하고 serialize/parse 후 네 변 None 보존 assertion을
추가했다. 제품 코드나 기대 좌표를 완화하지 않았고 최초 실패 로그도 보존한다.

## 선택 회귀 결과

`output/7353/r16/selected-tests.log`: V2 **122 PASS** + Legacy #5301 **3 PASS** +
#6311 **1 PASS** = **126 PASS**(실행0.220초, 빌드 시간 별도).
실행 case·파생 target·필터·명령은 해당 로그 선두에 모두 기록했다.
review worktree에서 `--prepare`한 파생 suite만 사용하며 source branch에 포함하지 않는다.

| 신규 정식 계약 | 검사한 최종 결과 |
| --- | --- |
| matching_table_outline | colspan 제목/2열 본문, 제목 반복. 각 조각6선, 공유선1개, x20/120/220·y30/48/66 및 실제 글자 |
| nested_matching_outline | 자식 두 쪽 뒤 host/after. 자식 외곽선이 부모 뒤 문단까지 늘어나지 않음 |
| table_outline_does_not_override | 한 변 None/굵기 충돌 각각 명시적 오류, 두 번 호출해도 emitted=0 |
| table_only_outline | 셀 선이 없는 표를 성공처럼 출력하지 않음. 우선순위 미확정 상태를 오류로 유지 |
| outline_agreement_at_each_cut | 원본 전체 외곽은 일치하지만2번째 조각 위 선 None.1쪽 정상 뒤 두 번 실패에도 emitted=1 |
| none_table_outline | 명시적 표 None은 오른쪽 셀의4실선을 제거하지 않음 |
| matching_outline_cuts_and_tail | 빈 줄 포함 A/빈 줄→B/C,90px 최소 높이36+36+18. 빈 물리 조각에도 같은 경계, 마지막 뒤 종료 |
| matching_parent_and_child | A/B→C/host→after.2쪽 자식18px/부모36px,3쪽 부모18px, 글자 중복·누락 없이 각자 경계 |

좌표는 선언된 글줄·셀 폭·예산에서 정했다. 출력 helper의 높이를 기대값으로 재인용하지
않는다. 기존 complex-edge 거부 중 table Solid 자체의 거부만 새 긍정 계약으로 대체했다.
Dash/잘못된 폭/색의 거부는 셀과 표 각각 유지·검사했다. golden/래칫 수정은 없다.

## 최종 lint·정책·소스 증빙

검증 소스는 `bf9cc23085a212378f09bc1905c2b1e4f4d90306` + 이번 변경이다.
`output/7353/r16/source.sha256`의 V2 source/test/harness27개가 review overlay와 일치한다
(`review-source-match.log`). Docker 시작 시 production 두 파일의 해시를 고정했고,
완료 후에도 동일하다(`compiled-source.sha256`, `compiled-source-final-check.log`).
정책 base는 `7a95e46e025470a4d7a7b59ad68ec02958bda738`이다.

- Native lib Clippy **PASS**(28.88초), WASM lib **PASS**(31.16초), 변경 integration
  target `regression_suite_005` **PASS**(53.10초). 아래 순서로 실행했다.
- fmt·고정 base manifest **PASS** (`fmt-final.log`, `policy-final.log`).
- JS 구문, frontend bindings **1 PASS**, 문서2개 링크·diff 공백 검사 **PASS**.
- source-side cfg(test), baseline, generated suite/manifest의 source 변경은 없다.

```sh
# review worktree에서 준비; 실제 선택 nextest 명령/필터는 selected-tests.log 선두
node scripts/rust-test-suite-manifest.mjs --prepare
cargo clippy --locked -p rhwp --lib --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo clippy --locked -p rhwp --lib --target wasm32-unknown-unknown \
  --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo clippy --locked --test regression_suite_005 \
  --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo fmt --all -- --check
node scripts/rust-test-suite-manifest.mjs --check --base-ref 7a95e46e025470a4d7a7b59ad68ec02958bda738
```

## Fresh WASM·직접 Visual Sweep

```sh
docker compose --env-file .env.docker -p rhwp run --rm wasm
VISUAL_SWEEP_CHROME=/home/edward/.cache/puppeteer/chrome/linux-146.0.7680.31/chrome-linux64/chrome \
  node scripts/verify-table-v2-preview-wasm.mjs --pkg pkg \
  --fixtures output/7353/r16/fixtures --out output/7353/r16/browser \
  --dependencies-root /home/edward/mygithub/rhwp --solid-backgrounds --solid-borders \
  --split-borders --split-line-property --matching-table-borders
```

Docker release + wasm-opt **7분58초 성공** (`docker-wasm.log`). WASM SHA-256:
`f670c60276675a5e61c5e7790f0294ca57aa76f520a993d466306c89c66a7849` (`pkg.sha256`).
Chrome146.0.7680.31에서 기존41쪽 + 신규15쪽 = **56쪽 PASS**.
Native/WASM tree·SVG exact parity, 독립 세션, 안정적 종료, 부정 입력 거부,
신규 outline3입력의0쪽/1쪽 이후 오류 rollback을 확인했다(`browser.log`,
`browser/manifest.json`). 입력 bytes hash와 생성 SVG/review hash는 manifest에 보존했다.

다음 **새로 생성된** compare PNG의 Native/fresh WASM/standalone overlay를 직접 판독했다.
이는 합성 입력의 독립 기하 계약 확인이며 한컴 원본 문서와의 시각 일치 판정은 아니다.

- `browser/outer-border-header-1.review.png`: 반복 제목에 내부 세로선 없음,
  L2/R2 경계·바깥 선 각각 한 겹, 밑 선과 셀 높이 일치.
- `browser/outer-cut-nested-1.review.png`: 자식 C 아래 경계와 후속 host 아래 부모 경계를
  구별하며 자식 높이를 부모 끝까지 늘리지 않음.
- `browser/outer-cut-tail-2.review.png`: 마지막18px 빈 물리 밴드에 닫힌 경계 유지.
- `browser/outer-border-one-sided-0.review.png`: L1 주위에 없는 외곽을 만들지 않고
  R1의 명시된 선만 유지.

신규15쪽의 Native/WASM PNG는 각각 셀 전용 대조군과 동일하다. 동일한 선을 별도
paint layer로 중복 출력하지 않는 계약에 부합한다. 기존41쪽 새 review PNG도 stage15와
byte-identical이다(`visual-compare.log`). 이전 PNG를 새 증적으로 복사하지 않았다.

## 판정과 남은 범위

일치 외곽선 수용·동일 fragment 소비·출력/커서 보존 계약은 **충족**이다.
표 Solid/셀 None 보충과 상이한 선의 우선순위는 독립 규칙이 미확정이므로 **미검증**이며
현재 명시적으로 거부한다. #6311/KTX의 전체 한컴 피델리티, 활성 별도 분할선 paint,
rowspan·TAC/어울림·저장 줄·문서 전체 V2 전환은 이번 완료 범위가 아니다.
전체 workspace/CI/Skia, push/PR/이슈 종료는 수행하지 않았다.
