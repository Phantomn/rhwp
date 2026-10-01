# PR #7487 리뷰 — Enter로 넘친 빈 문단의 쪽 소유 보존

## 최종 판정

**머지 보류.** 원격 원 head `b28130e1bcea509d9969088c7e75a0e23e4974b2`에는 200%의 33번째 Enter와 300%의 22번째 Enter에서 새 문단의 쪽 소유가 사라지는 결함이 남아 있다. 사용자 승인 범위의 로컬 보정 `45863eb2b238929c22ecc606af801b6273dc8482`에서는 두 경계의 엔진 계약과 필수 로컬 검증이 통과했다. 보정과 기록은 아직 원격에 반영하지 않았다.

보류 해제는 보정이 포함된 정확한 새 PR head의 Full CI·required check와 mergeability 확인, 작업지시자의 시각 증적 판독 및 제한한 해결 범위 확인 뒤 다시 판정한다. 새 Enter 합성 문서의 동일 원문 한컴 기준 출력은 미검증이다. 아래 저장 문서 대조군의 Visual Sweep을 이 미검증의 대체 증거로 사용하지 않는다. 리뷰 게시·push·merge는 각각 별도 승인 단계다.

## 접수 정보

| 항목 | 값 |
| --- | --- |
| PR | [#7487](https://github.com/edwardkim/rhwp/pull/7487), devel 대상 |
| 기여자 | @semanticist21, 원 commit author semanticist |
| 원격 source | `semanticist21/rhwp`, `fix/issue-7486-enter-overflow-page` |
| 원 head | `b28130e1bcea509d9969088c7e75a0e23e4974b2` |
| 원 parent | `0e8fd49fb868da0d47ac1294dcbbda81f0211233` |
| 보정 후보 | `45863eb2b238929c22ecc606af801b6273dc8482`, 원 head 위 single-parent 추가 commit |
| 현재 정책·호환 base | `02530b9ed567a44663edb26c65fb565c4a79f00d` |
| 관련 이슈 | [#7486](https://github.com/edwardkim/rhwp/issues/7486), 부분 해결이므로 참조만 하고 종료하지 않음 |
| reviewer | @postmelee; bug/layout, v1.0.0, assignee @semanticist21 |
| 최신 확인 | 2026-10-01 16:46 UTC / 2026-10-02 01:46 KST; OPEN, non-draft, maintainerCanModify=true, MERGEABLE/CLEAN |

base route: `collaborator_external_pr.md` 9.3.1의 contributor source 직접 보정.
modifiers: `intake_and_review.md`, `local_validation.md`, `visual_fixture_evidence.md`.
loaded documents: `pr_review_workflow.md`, `pr_review/README.md`, 위 기본·보조 가이드, `review_template.md`, `visual_verification_governance.md`, `visual_sweep_guide.md`, `object_visual_regression.md`, `docs_and_git_workflow.md`, `dev_environment_guide.md`.

최신 devel과 원 source에서 위 정책 파일의 차이가 없는 것을 확인했다. 초기 최신-base 체리픽 검토 후보 `d10a64a0c9d4f85e1cad726feb7158585aa277a9`는 로컬 `refs/codex/review/pr7487/integration-d10`으로 보존했다. 보정은 같은 가시성 branch `codex/pr7487-review-20261001`를 원 source로 정렬한 뒤 계속했다. 체리픽 통합 PR을 만들거나 contributor 이력을 rewrite하지 않는다.

## 변경과 검토 범위

기여자의 변경은 끝 쪽의 빈 문단 제거가 Enter로 실제 넘친 줄까지 버리지 않도록 저장 LineSeg의 `vertical_pos + line_height`와 본문 높이를 대조한다. 기본 160%의 60회 Enter 회귀가 이 변경에 포함돼 있다.

추가 보정은 그보다 앞선 마지막 빈 문단 흡수 경로에도 같은 판별 결과를 적용한다. 이전에는 Task #676의 `Hidden` 또는 `Unadvanced` 처리에서 문단이 먼저 사라져, 끝 쪽 보존 코드까지 도달하지 못했다.

| 변경 파일 | 역할 |
| --- | --- |
| [paragraph.rs](../../../src/renderer/typeset/paragraph.rs) | `stored_line_overflows_body` 공통 판별을 만들고, 본문 밖 유효 줄을 마지막 빈 문단 흡수에서 제외 |
| [finalize.rs](../../../src/renderer/typeset/state/finalize.rs) | 기여자의 끝 쪽 보존 조건을 같은 판별로 공유 |
| [정식 회귀 원본](../../../tests/cases/issue_7486_enter_overflow_opens_page.rs) | 200%/300% 경계, 저장·재열기, 여섯 줄간격의 90회 연속 Enter 검사 추가 |

문서 번호·특정 줄간격 값으로 production 분기를 만들지 않았다. 명시적인 `hide_empty_line` 숨김 경로는 기존 순서대로 먼저 실행한다. Studio TypeScript와 표 분할 코드는 변경하지 않았다.

### 조판 원칙 준수 검토

| 항목 | 판정·근거 |
| --- | --- |
| 글자 가시성과 줄 점유 구분 | **충족(검증 범위)**. 글자가 없는 줄도 본문 밖이면 새 쪽과 cursor owner를 가진다. 빈 문자열을 줄 높이 0의 대용으로 사용하지 않는다. |
| 생산→소비→최종 원점 | **충족(일반 단일 본문 경로)**. 편집 재조판 LineSeg → `paragraph/flow.rs:62` 흡수 판별 → `flow.rs:99` whole-fit 및 실패 시 일반 배치 → PageItem/레이아웃 → `getCursorRect`. `section.rs:449`의 최종 제거도 같은 overflow 판별을 사용한다. 실제 경계의 cursor pageIndex·x·y·height를 검사했다. |
| 내용 컷·예약 높이·예산·배치 | 이번 보정은 일반 문단의 기존 fit/이월을 다시 사용하며 새로운 컷이나 높이 계산을 추가하지 않는다. rowspan/PartialTable continuation 알고리즘 변경은 **비해당**. 다단 경로는 이 helper 전에 조기 반환하므로 추가 보정 효과를 주장하지 않는다. |
| 저장 정보와 편집 재조판 | **충족(합성 계약)**. 같은 API로 생성한 문서를 HWPX 저장·재열기해 전 문단 owner를 검사한다. 저장 메타데이터를 손으로 고치거나 수용 조건을 완화하지 않았다. |
| 독립 기대값 | A4·기본 여백·10pt 줄 상자와 Percent 전진으로 설정. 본문 높이 약 65,760 HU, 200%×33 및 300%×22는 66,000 HU이므로 다음 쪽. 새 쪽 본문 시작은 x=30mm, y=20mm+15mm. 구현이 반환한 높이를 기대값으로 재인용하지 않았다. |
| 수정 전 실패 / 후 통과 | 원 head에서 두 신규 경계 테스트가 cursor owner 오류로 FAIL, 보정 뒤 PASS. 환경·컴파일 실패를 재현으로 세지 않았다. |
| 저장 문서 대조군 | p122의 저장 vpos 되감기·그림·끝 빈 쪽, #6087의 13쪽 보존 통과. p122 그림 geometry 변경 0건. |
| 동일 합성 원문의 한컴 대조 | **미검증**. 합성 계약·Chrome 관측과 한컴 출력 일치 주장을 구분한다. |
| 전체 Enter 사용자 여정 | **미충족(기존 Studio 결함 재현)**. 쪽은 만들어져도 캐럿·스크롤 갱신은 늦다. 아래 별도 후속 범위로 남긴다. |

## 검증 입력과 실행 결과

합성 빈 문서는 정식 테스트의 `create_blank_document_native` / `apply_para_format_native` / `split_paragraph_native`로 생성했다. 200%/300% 저장본은 테스트 내 동일 byte buffer를 재열기했다. 외부 수동 LineSeg 입력이나 한컴 생성본으로 분류하지 않는다.

| 실제 파일 | 출처·commit 포함 확인 |
| --- | --- |
| [samples/p122.hwp](../../../samples/p122.hwp) | 기존 저장 문서, 보정 commit blob과 실제 실행 byte 일치 |
| [pdf/p122-2022.pdf](../../../pdf/p122-2022.pdf) | 저장소의 대응 한컴 기준 PDF 재사용, 동일 byte 확인; 형식/연도만으로 제외하지 않음 |
| [#6087 대조 입력](../../../samples/issue6060/30307_local_service_reform.hwp) | 기존 정상 저장 입력, commit byte 일치 |
| [blank2010.hwp](../../../saved/blank2010.hwp) | 이전 기본 빈 문서 진단의 기존 입력, commit byte 일치 |

네 입력의 SHA-256·크기와 산출물 해시는 [provenance.json](../assets/pr_7487/provenance.json)에 고정했다.

검증 source SHA는 모두 보정 `45863eb2b238929c22ecc606af801b6273dc8482`다. 아래 결과는 실제 실행 기록이며, 원 head의 과거 CI를 새 보정의 검증으로 재사용하지 않았다.

| 검증 | 실행 결과 |
| --- | --- |
| 변경 경계 focused | 4 PASS. 기본 Enter, 200%/300% owner·본문 시작 좌표, 저장 재열기, 6개 줄간격×90 Enter 줄 상자 검사 |
| 저장 vpos p122 | 3 PASS |
| #6087 top collision | 1 PASS, 13쪽 유지 |
| fmt·native Clippy·WASM32 Clippy·workspace build·workspace all-target Clippy | 순차 실행 모두 PASS, `--locked --target-dir target/pr-review`, Clippy `-D warnings` |
| integration manifest | `--prepare` 후 `--check --base-ref 02530b9ed567a44663edb26c65fb565c4a79f00d` PASS; 파생 suite/manifest 커밋 제외 |
| 전체 default-feature 회귀 | `cargo nextest run --locked --cargo-profile release-test --target-dir target/pr-review --tests --no-fail-fast`: 10,273 PASS / 50 skipped |
| Native Skia lib | `cargo test --locked --profile release-test --target-dir target/pr-review --features native-skia --lib`: rhwp 3,930 PASS / 13 ignored, workspace 부속 lib 182 PASS |
| Native Skia placeholder / direct PDF | 각각 2 PASS / 4 PASS |
| fresh WASM | 루트에서 `CARGO_TARGET_DIR=target/pr-review scripts/wasm-pack-locked.sh --target web --out-dir pkg --no-opt` 성공; root pkg·Studio public·실제 HTTP 제공 JS/WASM SHA-256 일치 |
| 실제 Chrome Studio | 새 문서 로딩 완료 뒤 160% Enter41, 200% Enter33, 300% Enter22에서 두 쪽 생성. 100% 한 쪽 / 66% 두 쪽 보기에서 남은 캐럿 지연 재현 |

[명령·exit·시간 원장](../assets/pr_7487/validation.json), [수정 전후 및 전체 검사 발췌](../assets/pr_7487/test-results.txt), [엔진 before](../assets/pr_7487/engine-before.json), [엔진 after](../assets/pr_7487/engine-after.json), [Chrome DOM 관측](../assets/pr_7487/chrome-dom-observations.json), [실행 확인한 WASM 재현 스크립트](../assets/pr_7487/reproduce-wasm.mjs).

Native Skia 첫 시도는 sandbox DNS 제한으로 의존성 다운로드 전에 실패했다. 허용된 네트워크로 재실행해 위 결과를 얻었다. Docker daemon 미실행으로 host wrapper의 `--no-opt` WASM을 사용했다. 최적화된 배포 패키지의 직접 브라우저 확인은 미검증이며 이번 host 검증과 구분한다.

Chrome 첫 예비 시도에서는 새 문서 로딩 중 입력해 횟수를 신뢰할 수 없었고, 이전 문단 번호에 서식을 적용하는 경고가 발생했다. 그 시도는 폐기하고 로딩 완료 후 새 문서에서 다시 센 위 기록만 수용했다. 결함의 전후 증거에 예비 시도를 섞지 않았다.

## 시각 증적과 남은 차이

### 엔진 경계와 별도 Studio 결함

엔진 직접 WASM API에서 보정 전에는 Enter33/22의 split이 성공해도 1쪽이고 cursor 조회가 실패했다. 보정 후에는 즉시 2쪽, pageIndex=1, x≈113.4px·y≈132.3px·height≈13.3px다. Chrome에서도 Enter 한 번으로 페이지 canvas가 둘이 됐다.

100%에서 새 쪽이 생성된 직후 상태줄은 `1 / 2 쪽`이고 DOM caret은 `top:132.3px`, `left:530.4px`에 남았다. 실제 두 번째 canvas의 top은 `1142.5px`라 새 쪽 원점을 반영하지 못한 상태다. 다음 `a` 입력 뒤 `2 / 2 쪽`, caret top≈1275.4px와 화면 스크롤이 갱신됐다. 66% 두 쪽 보기에서도 생성 직후 caret top≈87.3px가 추가 입력 뒤≈94.3px로 보정됐다.

![300% Enter22 직후: 두 쪽 생성과 지연된 캐럿](../assets/pr_7487/chrome-300-enter22.png)

![다음 입력 뒤 두 번째 쪽으로 갱신](../assets/pr_7487/chrome-300-after-input.png)

[200% 직후](../assets/pr_7487/chrome-200-enter33.png) · [200% 추가 입력 뒤](../assets/pr_7487/chrome-200-after-input.png) · [66% 직후](../assets/pr_7487/chrome-300-zoom66-enter22.png) · [66% 추가 입력 뒤](../assets/pr_7487/chrome-300-zoom66-after-input.png).

사용자 첨부 [이전 경계 화면](../assets/pr_7487/user-before-300-enter.png)과 [이전 새 쪽 캐럿 화면](../assets/pr_7487/user-before-new-page-caret.png)은 관측 참고다. 그 화면의 정확한 빌드 SHA·입력 원문이 없으므로 동일 source의 독립 oracle로 승격하지 않는다.

Studio 원인 경로는 `InputHandler.executeOperation(command)` → `afterEdit`의 document-changed와 즉시 updateCaret → 비동기 `CanvasView.refreshPagesForMutation`의 VirtualScroll 확정이다. 완료 이벤트의 listener는 `CaretLayoutReveal.consume()`이 true일 때만 재투영한다. 현재 허용 operation은 pageBreak/columnBreak이며 command Enter에는 예약이 없다. 관련 TypeScript 파일은 원 PR과 최신 base에서 동일하여 새로 도입된 회귀로 분류하지 않는다. 화면 위치 clamp나 숨김으로 우회하지 않고 별도 Studio 작업에서 갱신 완료와 caret reveal을 연결해야 한다.

### 기존 저장 문서 대조군: Native / fresh WASM

`p122.hwp`의 2쪽을 동일 `p122-2022.pdf`의 2쪽과 96 DPI로 비교했다. Native는 아래 Visual Sweep 명령, fresh WASM은 직접 Node API의 SVG/render tree를 librsvg로 래스터한 뒤 같은 sweep helper로 compare/standalone overlay/review를 만들었다. 후자는 브라우저 렌더라고 보고하지 않는다.

```sh
venv/bin/python scripts/visual_sweep.py --hwp samples/p122.hwp \
  --pdf pdf/p122-2022.pdf --key pr7487-p122-control --pages 2 --dpi 96 \
  --rhwp-bin target/pr-review/release-test/rhwp --svg-rasterizer rsvg \
  --out output/pr-review/pr7487-review-20261001/p122-native-sweep
```

![Native 저장 문서 대조군 p2 review](../assets/pr_7487/native-p122-review-p002.png)

코멘트: 내용 픽셀 중심 자동 일치율 보조값 = 약 99.65%.<br>
높을수록 좋음: 기준 PDF와 rhwp PNG가 더 비슷함<br>
낮을수록 나쁨/검토 필요: 잉크 위치나 형태 차이가 큼<br>
단, 사람 판정 정확도가 아니라 내용 픽셀 중심 자동 일치율 보조값입니다

![fresh WASM 직접 API 저장 문서 대조군 p2 review](../assets/pr_7487/wasm-p122-review-p002.png)

코멘트: 내용 픽셀 중심 자동 일치율 보조값 = 약 99.65%.<br>
높을수록 좋음: 기준 PDF와 rhwp PNG가 더 비슷함<br>
낮을수록 나쁨/검토 필요: 잉크 위치나 형태 차이가 큼<br>
단, 사람 판정 정확도가 아니라 내용 픽셀 중심 자동 일치율 보조값입니다

[Native standalone overlay](../assets/pr_7487/native-p122-overlay-p002.png) · [WASM standalone overlay](../assets/pr_7487/wasm-p122-overlay-p002.png) · [지표·gate](../assets/pr_7487/wasm-p122-summary.json) · [OVR geometry](../assets/pr_7487/p122-ovr-summary.json).

양 backend 모두 픽셀 일치율 99.73420%, 엄격 내용 일치율 99.64882%, 2px 관용 내용 실루엣 일치율 100.0%, gate=passed다. review와 standalone overlay를 직접 열어 그림 외곽·원점·크롭이 겹치는 것을 판독했다. 미세한 이미지 가장자리/리샘플링 차이가 남는다. 글꼴 예외는 사용하지 않았다.

OVR는 보존한 보정 전 WASM `d10a64a...`와 보정 후 Native render tree를 `rhwp_objects`/`compare_objects`로 대조하고 x/y도 별도 검사했다. 3→3쪽, 그림 1→1개, page/x/y/w/h delta=0이다. 대상은 p122의 그림 한 개이며 ovr5 전수 검사로 확대 해석하지 않는다. 새로운 빈 Enter 페이지의 한컴 기준 출력 일치를 뜻하지 않는다.

## 원격·후속 처리 조건

- 보정 code/test commit과 archive review/report·asset·오늘할일 commit을 분리한다.
- 사용자 승인 뒤에만 정확한 source ref로 정상 push한다. force-push는 사용하지 않는다.
- code/test가 포함되므로 review-only fast-pass를 적용하지 않고 새 head Full CI를 확인한다.
- PR 본문 갱신을 승인받으면 대표 review/overlay PNG를 source repository와 새 head SHA 고정 raw URL로 실제 임베드한다. 아직 존재하지 않는 URL을 검증 완료로 보고하지 않는다.
- 판정 갱신과 게시할 review 본문을 제시해 별도 확인받는다. 지금 Approve·Request changes·comment는 게시하지 않았다.
- merge는 별도 승인 뒤 수행한다. 그 뒤 contributor comment에 merge SHA 고정 동일 asset·CI URL·남은 차이를 한국어 존댓말로 남긴다.
- #7486은 표 뒤 Enter와 Studio 캐럿·스크롤 문제가 남으므로 종료하지 않는다. 배포 전 전체 Enter 사용자 여정 해결을 주장하지 않는다.
