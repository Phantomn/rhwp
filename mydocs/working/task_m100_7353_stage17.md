# Task #7353 — 중첩 자리차지 표의 가로 정렬

- 선행 커밋: `fdf11fee3` ([stage16](task_m100_7353_stage16.md)).
- 상태: 정렬 확장·선택 회귀·fresh WASM 직접 비교 완료. R3 전체 완료는 아니다.
- Legacy/default Studio는 유지한다. 선택 표 V2 미리보기의 지원 범위 확장이다.

## 범위와 독립 기대값

부모 셀 안 문단 기준, 세로 위·오프셋/바깥여백0인 non-TAC TopAndBottom 자식 표에
Left/Center/Right를 지원한다. 문단 여백·들여쓰기가 없는 기존 지원 경계에서 부모의
유효한 셀 안 여백을 뺀 콘텐츠 폭을 기준으로 좌측선/중심/우측선이 일치해야 한다.
Inside/Outside, 다른 상대 기준, 비영 오프셋, TAC/어울림은 계속 명시적 미지원이다.

입력은 저장 LineSeg 없는 합성 HWPX를 serializer로 만들고 parser로 다시 읽는다.
독립 기대값은 너비200px, 안 여백10/30px, 자식100px의 정렬 불변식이다.
콘텐츠160px에서 여유60px이므로 상대 x=0/30/60px, 페이지 x=30/60/90px이다.
18px 고정 줄과36px 페이지 예산에서 A/B → C/host → after 순서를 보존한다.
이 계약을 한컴 실물 출력 일치나 문서 전체 V2 완료로 해석하지 않는다.

## 공통 결과와 실제 소비 경로

`ir::bind_table`은 grid 폭과 유효 padding으로 구한 inner_width를 문단 구성에 사용하고,
같은 값에서 자식 plan.width를 빼 offset_x를 확정한다. 이 값은 FlowBlock에 저장한다.
`TableContentPlan::from_grid_rows`는 offset과 오른쪽 끝의 유한성·가용 폭을 검사한다.
`FlowCursor::fit`은 모든 자식 시작/계속 조각의 PageArea.x에 offset_x를 한 번 적용한다.
`TableCursor::fit → TablePlacement/CellPlacement → TextPaint::build_node → borders::append`
는 그 좌표를 그대로 소비한다. paint에서 정렬을 다시 추측하거나 좌표를 clamp하지 않는다.
명시적 TextFlowBlock 호출자 경로는 offset0으로 기존 계약을 보존한다.

수직 컷·소유 유닛·요구/누적 예약 높이·예산 실패 이월은 무변경이다.
`fragment::fit_rows → FlowCursor::fit → child.fit`의 수용 조각 높이를 누적하고
자식 종료 뒤 host/after로 진행한다. 반복 제목도 같은 child cursor와 offset을 사용한다.
검사는 두 단계 중첩 원점, 개별 셀 여백 우선, 제목 반복, 부정 offset/폭, 최종 텍스트·외곽선·
끝 조각·안정적 종료를 포함한다. rowspan/저장 줄/TAC/어울림 경로는 미지원으로 비해당이다.

## 검증 기록

실행 로그와 fresh Native/WASM 비교는 `output/7353/r17/`에 보존한다.

수정 전 stage16 코드에 초기3개 계약을 적용했다(`before.log`). 가운데·오른쪽2건은
빌드 성공 후 기존 앵커 미지원으로 실패했다. 이는 기존 지원 버그의 검출이 아니라 지원 경계
확장의 증거다. 왼쪽 대조군은 JSON의 정수/실수 표현 비교 때문에 실패했고, 기대값을 실수로
수정했다. 좌표 자체는 변경하지 않았다. 이어 재실행은 파일 길이에 따른 suite 재배정과
준비 파일의 불일치로0건 실행이었다(`before-corrected.log`, `after-focused.log`). 이들을
통과로 세지 않는다. 파생 suite 재준비 뒤 실제 실행한 새 계약8건은 모두 통과했다
(`after-prepared.log`). 추가5건의 수정 전 실행은 없으므로 red→green으로 주장하지 않는다.

선택 회귀는 V2 **130 PASS**, Legacy #5301 **3 PASS**, #6311 **1 PASS** =
**134 PASS**, 실행0.238초다(`selected-tests.log`; 빌드 별도). case/target/filter/명령은
로그 선두에 보존했다. 정식 원본은 `tests/cases/issue_7353_table_v2_alignment.rs`이며
파생 파일은 review worktree에서만 준비했다. 기존 테스트의 변경은 명시적 offset0 전달뿐이다.
기준값·golden·래칫 변경은 없다.

| 계약 | 실제 검사 |
| --- | --- |
| Left / Center / Right | 부모 폭200·자식100, 각 조각 x30/60/90, A/B→C/host→after, 외곽선 좌표·높이·종료 |
| 개별 셀 여백 | table padding 대신 apply_inner_margin 셀의10/30px를 사용, center x60 |
| 두 단계 중첩 | 부모 x20, 오른쪽 중간 표 x90, 가운데 손자 x120; 중간 host x100·바깥 host x30 |
| 반복 제목 | 두 쪽의 title x90, A/B 각각 한 번, 세 번째 쪽 host/after x30, 제목 높이18 |
| 미지원 앵커 | Inside/Outside·비영 x/y·Page 기준·TAC·Square 계속 거부 |
| 유효 폭 |161px 자식을160px에 넣지 않음. 수동 offset 음수/NaN/무한/너비 초과 거부 |

## 최종 소스 및 lint

검증 기준은 `fdf11fee3` + 이번 변경이며 `source.sha256`의 V2 source/test/harness가
review overlay와 모두 일치한다(`review-source-match.log`). WASM 빌드 전5개 변경 source의
해시도 `compiled-source.sha256`으로 고정했다. policy base는
`7a95e46e025470a4d7a7b59ad68ec02958bda738`이다.

- Native lib Clippy, WASM lib Clippy, 변경 test target `regression_suite_007/006` Clippy 순차 PASS.
- review worktree fmt·고정 base manifest PASS (`fmt.log`, `policy.log`).
- source-side cfg(test) 변경이 없으므로 unit-test-tier 검사는 비해당이다.

```sh
# /home/edward/mygithub/rhwp-review-7353
node scripts/rust-test-suite-manifest.mjs --prepare
cargo clippy --locked -p rhwp --lib --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo clippy --locked -p rhwp --lib --target wasm32-unknown-unknown \
  --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo clippy --locked --test regression_suite_007 --test regression_suite_006 \
  --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo fmt --all -- --check
node scripts/rust-test-suite-manifest.mjs --check --base-ref 7a95e46e025470a4d7a7b59ad68ec02958bda738
```

개발 절편의 선택 검증이다. 전체 workspace/CI/Skia, 실제 한컴 문서 피델리티, 기본 엔진
전환, push/PR은 실행·완료 범위가 아니다.

## Fresh WASM·직접 Visual Sweep

```sh
# /home/edward/mygithub/rhwp-task-7353
docker compose --env-file .env.docker -p rhwp run --rm wasm
VISUAL_SWEEP_CHROME=/home/edward/.cache/puppeteer/chrome/linux-146.0.7680.31/chrome-linux64/chrome \
  node scripts/verify-table-v2-preview-wasm.mjs --pkg pkg \
  --fixtures output/7353/r17/fixtures --out output/7353/r17/browser \
  --dependencies-root /home/edward/mygithub/rhwp --solid-backgrounds --solid-borders \
  --split-borders --split-line-property --matching-table-borders --nested-alignment
node --test scripts/frontend-wasm-bindings.test.mjs
```

Docker release + wasm-opt **8분9초 성공**. WASM SHA-256:
`46ab693e7822cf854eb8f758cb8bd602abb40a551739ddc338eadb472e5d1556` (`pkg.sha256`).
빌드 전후 변경 source5개 해시는 같다(`compiled-source-final-check.log`).
Chrome146.0.7680.31에서 기존56쪽 + 신규18쪽 = **74쪽 PASS**, Native/fresh WASM
RenderTree·SVG exact parity, 독립 세션·거부·rollback·안정적 종료를 확인했다.
신규 좌표·소유 글줄·외곽선 기대값도 browser runner에서 별도로 검사했다.
bindings **1 PASS**, JS 구문·문서2개 링크·diff 공백 검사도 통과했다.

`browser/manifest.json`은 input bytes·산출 해시를 보존한다. 동일 합성 입력의 Native를
overlay 비교 대상으로 삼았으며, PDF/한컴 출력은 사용하지 않았다. 독립 근거는 위에
명시한 정렬 불변식·선언된 줄 높이/폭/여백이다.

다음 **이번 빌드에서 생성한** compare PNG의 Native/fresh WASM/standalone overlay를
직접 판독했다. 텍스트 위치·표 외곽·후속 문단·겹침·누락 관점에서 기대 계약과 일치한다.

- `browser/align-left-0.review.png`, `align-center-0.review.png`: 비대칭 안 여백을
  적용한 좌측/중앙 위치. 부모 셀 전체가 아닌160px 콘텐츠 폭 기준이다.
- `browser/align-right-1.review.png`: 자식 C는 오른쪽의18px 조각, host는 부모 왼쪽에
  다음 줄로 이어진다. 자식의 높이가 부모36px 전체까지 잘못 늘어나지 않는다.
- `browser/align-deep-1.review.png`: 부모/자식/손자의 x20/90/120 구분과 middle x100.
- `browser/align-header-1.review.png`: 오른쪽 위치의 반복 title/B와 공유 가로선 한 겹.
- `browser/align-right-2.review.png`: 자식 종료 뒤 after만18px 부모 조각에 남는다.

기존56쪽의 새 review PNG는 stage16과 모두 byte-identical이다(`visual-compare.log`).
이전 이미지를 복사하지 않았다. 세션 기하·내용·경계 계약은 **충족**, 실물 한컴 피델리티는
**미검증**, Legacy 전환은 **비해당**이다. TAC/어울림·저장 줄·비영 오프셋·rowspan 등
남은 지원 범위를 이 결과로 완료 처리하지 않는다.
