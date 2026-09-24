# Task #7353 — 온전한 셀 콘텐츠의 세로 정렬

- 선행 커밋: `203f8cb41` ([stage17](task_m100_7353_stage17.md)).
- 상태: 승인 범위 구현 및 개발 절편 검증 완료. 한컴 실물 피델리티·전체 CI 판정은 아님.
- Legacy/default Studio는 변경하지 않는다. R3 선택 표 V2 미리보기 확장이다.

## 규칙·지원 경계·독립 기대값

OWPML `ParaList XML schema.xml`의 subList.vertAlign은 TOP/CENTER/BOTTOM을 구분한다.
이번 계약은 온전한 행의 공통 높이에서 셀 안 여백과 콘텐츠 점유를 제외한 여유를
위0/가운데1/2/아래전체로 배분하는 정렬 불변식이다. **TablePageBreak::None/RowBreak**에
적용한다. CellBreak의 Center/Bottom은 각 페이지별로 정렬해야 하는지 원본 전체 셀에
적용해야 하는지 독립 근거가 부족하므로 여전히 명시적 미지원이다. Top CellBreak는 유지한다.

입력은 저장 LineSeg 없는 합성 HWPX를 serialize/parse하여 실행한다. 고정18px 줄,
위6/아래12px 여백,90px 행에서 두 줄(빈 문단 포함)의 점유는36px이다. 여유36px이므로
콘텐츠 시작은 행 원점+6+[0/18/36]이다. 한 셀의 선언 높이만90px이어도 같은 행의
다른 셀은 공통90px에서 정렬한다. 빈 문자열을 높이0으로 간주하지 않는다. 이 계약을
한컴 실물 피델리티나 셀 내부 분할 정렬의 확정 근거로 승격하지 않는다.

## 생산·측정·분할·배치 경로

`grid::resolve`는 셀 주소/폭/정렬을 결합한다. `content::from_grid_rows`는 모든 셀의
FlowBlock 높이(빈 줄·중첩 표·padding 포함)와 최소 높이에서 공통 row_heights를 정하고,
그 같은 결과로 CellTrack.content_offset_y를 한 번 계산한다. 전체 행이 수용될 때만
`fragment::fit_rows`가 이 여유를 FlowCursor의 y/가용 예산 및 수용 높이에 함께 반영한다.
CellPlacement.content_origin도 같은 값을 사용한다. `TextPaint::build_node`와 border paint는
확정 좌표를 소비하며 paint 단계의 재정렬·숨김·clamp는 없다.

실제 위치: `grid.rs:15` → `content.rs:213`의 공통 행 높이 및 `:221`의 정렬 여유 →
`fragment.rs:216`의 수용 예산/줄 원점 → `:249`의 최종 content_origin이다
(모두 `src/renderer/table_v2/` 아래). 반복 제목은 `fragment.rs:98`, 본문은 `:124`에서
같은 fit_rows를 호출한다.

None은 표 전체 높이, RowBreak는 해당 행 높이를 먼저 예산과 비교한다. 공간 부족이면
내용 컷·cursor를 확정하지 않고 이월/DoesNotFit 처리한다. 행 정렬은 선언 높이를 축소하지
않으며, 정렬 여유는 수용 높이에 포함된다. 반복 제목은 같은 전체 행 계획을 재사용하고
본문 cursor만 전진한다. Top CellBreak 부모의 자식 RowBreak 표도 자식의 온전한 행에서만
정렬하며, 부모의 이어받기는 기존 컷을 유지한다. 마지막 자식 뒤 host/after와 종료를 검사한다.
rowspan·각주·TAC/어울림·저장 줄은 이 절편 비해당/미지원이다.

## 검증

`tests/cases/issue_7353_table_v2_vertical_alignment.rs`와 `output/7353/r18/`에 기록한다.
수정 전 stage17 코드에서는 새 지원 계약7건이 빌드 성공 후 기존 정렬 미지원으로 실패했고,
CellBreak 거부1건은 통과했다(`before.log`). 이는 신규 지원 경계의 red 증거이지 기존
지원 기능의 회귀 발견이 아니다. 수정 후 같은8건은 모두 통과했다(`after-focused.log`).

선택 회귀 **142 PASS** = V2 **138** + Legacy #5301 **3** + #6311 **1**이다
(`selected-tests.log`, 실행0.199초, 빌드 시간 별도). case/실제 target/filter/명령은 로그
선두에 남겼다. 정식 source test만 추가했고 기존 테스트 기대값·ignore·golden·래칫은 바꾸지 않았다.
source-side cfg(test) 변경이 없으므로 unit-test-tier 정책 검사는 비해당이다.

| 정식 계약 | 독립 기대값과 실제 검사 |
| --- | --- |
| row_break_alignment |90px 공통 행, 위6/아래12·두 줄36. Top/Center/Bottom 줄 y36/54,54/72,72/90. 다음 행은 다음 쪽 y36 |
| unsplit_table | 동일 정렬, 다음 행 y120/본문 y126. 전체126px를 한 번 예약 |
| repeated_centered_header |54px 제목+36px 본문을 매 쪽90px 예약. title y45 반복, A/B y90 각각 한 번 |
| child_row_alignment | 부모 CellBreak, 자식 RowBreak. 첫 쪽 before→빈 줄/C, 다음 쪽 D→host→after; 자식 전체 행90/72px 유지 |
| centered_parent | 부모 중앙 정렬 여유9px가 자식 표와 host/after에 한 번만 적용. 자식 x120 y45·host y63·after y81 |
| content_expansion |10px 선언 높이보다 실제54px가 우선. 개별6/12px 여백 적용, 음수 정렬 여유 없이 Top/Center/Bottom 동일 |
| split_cell_vertical_alignment | root/child CellBreak Center/Bottom을 명시적 Unsupported로 거부. 조용한 fallback 없음 |
| alignment_does_not_reduce_row_budget |90px 행을89px 예산에 넣지 않음. 반복 실패에도 emitted_pages=0 |

고정 줄간격18px의 **흐름 점유**와9pt 글꼴의96DPI **12px TextLine 상자**를 구별한다.
빈 문단도 줄간격을 소비하며 글자 유무로 줄 높이를 없애지 않는다.

## 소스와 검증 명령

source 기준은 `203f8cb41` + 이번 변경이다. `output/7353/r18/source.sha256`에
V2 source/test/harness를 고정하고 review overlay의 일치를 확인했다
(`review-source-match.log`). Docker 빌드 전 변경 source4개는 `compiled-source.sha256`에
별도로 고정했다. 파생 suite는 review worktree에서만 준비했으며 커밋하지 않는다.
정책 비교 base는 `7a95e46e025470a4d7a7b59ad68ec02958bda738`이다.

```sh
# /home/edward/mygithub/rhwp-review-7353
node scripts/rust-test-suite-manifest.mjs --prepare
ISSUE7353_EXPORT_DIR=/home/edward/mygithub/rhwp-task-7353/output/7353/r18/fixtures \
  node scripts/run-rust-test.mjs issue_7353_table_v2_vertical_alignment -- \
  --cargo-profile release-test --target-dir /home/edward/mygithub/rhwp/target/pr-review --no-fail-fast
cargo clippy --locked -p rhwp --lib --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo clippy --locked -p rhwp --lib --target wasm32-unknown-unknown \
  --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo clippy --locked --test regression_suite_022 --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo fmt --all -- --check
node scripts/rust-test-suite-manifest.mjs --check --base-ref 7a95e46e025470a4d7a7b59ad68ec02958bda738
```

Native Clippy(49.45초), WASM Clippy(1분15초), 변경 integration target Clippy(43.18초),
fmt 및 고정 base manifest 정책 검사는 모두 PASS다. 각각 `clippy-native.log`,
`clippy-wasm.log`, `clippy-tests.log`, `fmt.log`, `policy.log`에 기록했다.
이는 개발 절편의 범위별 검사이며 제출 직전 전체 workspace lint/CI를 대신하지 않는다.

## Fresh WASM 및 직접 시각 확인

```sh
# /home/edward/mygithub/rhwp-task-7353
docker compose --env-file .env.docker -p rhwp run --rm wasm
VISUAL_SWEEP_CHROME=/home/edward/.cache/puppeteer/chrome/linux-146.0.7680.31/chrome-linux64/chrome \
  node scripts/verify-table-v2-preview-wasm.mjs --pkg pkg \
  --fixtures output/7353/r18/fixtures --out output/7353/r18/browser \
  --dependencies-root /home/edward/mygithub/rhwp \
  --solid-backgrounds --solid-borders --split-borders --split-line-property \
  --matching-table-borders --nested-alignment --cell-vertical-align
node --test scripts/frontend-wasm-bindings.test.mjs
```

Docker 빌드는 **8분11초**에 성공했다(`docker-wasm.log`). 새 WASM SHA256은
`9bad0d0663391fb5b1a988ecf7c3a583c66fa0132b5f3f139ec2b193c2a5f9d6`이며
JS/WASM 전체 해시는 `pkg.sha256`, 입력·브라우저·개별 출력 해시는 `browser/manifest.json`에 있다.
Chrome 146.0.7680.31에서 **82쪽 PASS**: Native/fresh WASM의 render tree·SVG 완전 일치,
독립 좌표·행 높이·테두리·이월·종료 계약 통과(`browser.log`). frontend bindings도1건 PASS다.
이전 절편의74쪽 review PNG는 모두 byte-identical이다(`visual-compare.log`).

새8쪽의 Native/fresh WASM/overlay review PNG를 직접 열어 확인했다.
아래 파일은 `output/7353/r18/browser/`에 있으며 같은 stem의 `.overlay.png`도 생성했다.

| 직접 확인한 review PNG | 관측 |
| --- | --- |
| `valign-rows-0.review.png`, `valign-rows-1.review.png` | T/C/B의 세로 위치가 다르면서 셀 외곽90px은 동일. 다음 행 after는 다음 쪽 셀 내부에 표시 |
| `valign-whole-0.review.png` | 같은 정렬 아래에 후속 행이 이어지고 외곽선·after 누락 없음 |
| `valign-header-0.review.png`, `valign-header-1.review.png` | 중앙 정렬 title과 동일 높이의 제목 행 반복, 본문 A/B 중복·겹침 없음 |
| `valign-nested-0.review.png`, `valign-nested-1.review.png` | 자식 행 이월 전후 C/D 유지, 뒤 host/after가 자식 외곽 아래 부모 셀 내부에 배치. standalone overlay도 직접 확인 |
| `valign-parent-0.review.png` | 중앙 정렬 부모에서 자식 A와 host/after가 함께 이동하며 겹침 없음 |

기준 입력은 합성 HWPX이며 독립 기대값은 위 정렬 불변식이다. 한컴 기준 PDF는 사용하지
않았고, Native/WASM 일치를 한컴 출력 일치로 판정하지 않는다. 최종 source 해시는
`source-final-check.log` 및 `compiled-source-final-check.log`에서 일치를 확인했다.

판정: 온전한 셀 정렬·관련 선택 회귀·새 WASM 비교 **충족**. CellBreak Center/Bottom은
명시적 미지원이며 그 정렬 규칙은 **미검증**이다. 전체 workspace/CI/Skia, 한컴 실물 피델리티,
기본 엔진 전환·push/PR은 이번 개발 절편의 실행·완료 범위가 아니다.

## 후속 절차 변경 승인

2026-09-24 작업지시자는 진행 효율을 위한 절차 변경을 승인했다. stage18의 검증 결과와
미지원 범위는 그대로 보존한다. 다음 작업은 작은 서식별 절편이 아니라 실물 문서의
문단·표·후속 흐름을 연결하는 종단 묶음이다. 구현·집중 검증은 묶음 안에서 연속 진행하며
시각 판정 후보에서 fresh WASM과 영향 페이지를 검증한다. 상세 실행 순서와 유지되는 승인
경계는 [구현계획 5.1](../plans/task_m100_7353_impl.md#51-stage18-이후-실행-절차--종단-작업-묶음)에 기록했다.
이번 변경은 절차 문서에 한정되며 제품 source와 기존 검증 대상은 바꾸지 않았다.
