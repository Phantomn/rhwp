---
kind: review
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-03
---

# PR #7544 리뷰 — 표 뒤 Enter의 빈 줄 소속 보존

## 최종 판정

**머지 보류 — 저장본 종료 guide 분기의 필수 시각 증거 부족.** 재검토 head `3edcbdbe172767d135ce420e459a41fc9b44f12e`의 Full CI와 관련 checks는 성공했다. 반복 Enter 경로의 검증은 충족하지만, 실제 저장본 2개에 발동하는 별도 guide 분기의 위치·흐름 보존을 독립 한컴 출력으로 확인하지 않았다. 기존 승인 기록은 이 두 범위를 충분히 구분하지 못해 이번 리뷰에서 바로잡는다. 실행으로 새 회귀를 검출했다는 판정은 아니다. 본인 PR의 self-review이며 GitHub Approve event가 아니다.

직접 판독한 Native/fresh WASM 전체 7쪽 최저 실루엣 100%, 90% 미만·누락 쪽 0이다. 표 선 색상은 한컴 PDF보다 밝으며 전체 피델리티 100%로 해석하지 않는다. Docker daemon 연결 불가로 표준 Docker WASM은 미실행이고 host release `--no-opt` fallback을 사용했다. 이 제한을 실행 성공으로 바꾸어 기록하지 않는다.

## 2026-10-03 재검토 발견 사항

### [P1] 저장본 종료 guide 분기를 독립 출력으로 검증해야 한다 — 필수 증거 부족

`src/renderer/typeset/paragraph/empty.rs:25-70`의 새 판단은 fit하지 않는 마지막 빈 문단을 분할 표의 종료 guide로 수용한다. `paragraph.rs:623-635`에서 정상 이월을 생략하고, `state.rs:612-626` 및 `inline_flow.rs:61-82`에서 저장 vpos를 실제 배치 원점으로 쓰면서 흐름 전진은 0으로 유지한다. 이는 문단 소속 보존 외에 줄 위치·점유의 해석을 바꾸는 경로다.

제출된 4개 합성 HWPX와 대응 PDF, 대표 PNG·90% gate는 반복 Enter 경로에 대한 증거다. guide 판단은 직전 항목이 `PartialTable`이고 바로 다음 마지막 문단이어야 하므로, 표 뒤에 여러 빈 본문 문단을 만든 그 사례들의 통과를 guide 분기의 증거로 대신할 수 없다. 실제 발동 입력은 `samples/task2097/18095317_eogu_geumji.hwp`와 `samples/task2287/1342000_edu_curriculum_map.hwp`다. 현재 기록은 21/413쪽 유지, 저장 LineSeg 값, 기존 관련 회귀와 용지 밖 원장 통과다. `validation.json`의 `guidePlacementDiagnostic`도 어구 문서의 줄 좌표를 저장 vpos로 계산해 확인하며, 동일 원문의 독립 PDF와 직접 대조한 결과가 아니다. 어구 문서의 기존 마지막 표 off-canvas 1건은 남아 있다.

저장 줄이 본문 안에 있다는 사실과 쪽수 회귀 통과만으로, 이 빈 문단이 전진하지 않는 종료 guide인지 또는 표 뒤에서 실제 공간을 차지해야 하는 빈 줄인지 확정할 수 없다. 두 실제 입력의 변경 페이지에서 표 끝·빈 문단 원점·마지막 내용과 다음 구역 소속을 동일 원문 한컴 PDF의 Native/fresh WASM 출력과 대조하고 직접 판독해야 한다. 쪽수 주장은 전체 쪽 대응을 확인한다. 적용되는 실루엣·대표 이미지 gate와 입력 커밋 요건도 함께 충족한 뒤 재판정한다. 자료가 부족한 현재 상태를 새 결함 재현이나 시각 통과로 기록하지 않는다.

근거: [조판 원칙 검토 §2.7](../../manual/pr_review/intake_and_review.md#27-조판-원칙-준수-검토), [시각 검증 §3.5](../../manual/pr_review/visual_fixture_evidence.md#35-시각-검증-원칙). 저장 LineSeg 재사용과 편집 후 재조판의 증거를 구분하고, 실제 호출 경로의 미검증 범위를 승인으로 바꾸지 않는다는 공통 계약을 적용한다.

### 완료한 재검토

- 정확한 head `3edcbdbe…`의 [Full CI run 37045552995](https://github.com/edwardkim/rhwp/actions/runs/37045552995) 실제 로그에서 `CLASSIFICATION_STATUS: full`, lint·Native Skia·Archive A/B/C/D·Build & Test 성공을 확인했다. CodeQL, Render Diff, Adapter inter-diff, Proptest 및 CI Impact Policy도 성공이며 pending/failure가 없다. 정책상 skip과 성공은 구분한다.
- 같은 production/test source인 `97772d5d…`와 최종 head 사이 source/test 차이는 없다. 로컬 전체 Cargo·lint는 기존 실행과 정확한 head의 Full CI를 재사용했으며 중복 실행하지 않았다.
- 별도 review checkout에서 이번 리뷰의 경계·관련 회귀를 다시 실행했다. `cargo nextest run --locked --cargo-profile release-test --tests --no-fail-fast -E 'test(/issue_7486_enter_overflow_opens_page|issue_2097_band_fill|issue_6981_split_straddle_row_height|issue_7226_rowspan_only_row_cut|issue_6761_stored_vpos_rewind_page_break|off_canvas_does_not_grow_partition_12/)'`는 21 PASS / 0 FAIL / 10277 미선택, 11.176초다. shared `target/pr-review`를 재사용했으며 source를 변경하지 않았다. 로그는 ignored `output/pr-review/issue7486-table-fix-20261003/review7544-focused.log`에 있다.
- `section.rs`에서 명시적 경계와 저장 reset 처리는 일반 문단 flow 이전에 실행됨을 확인했다. `paragraph/flow.rs`의 조기 반환 순서만으로 저장 쪽 나눔 회귀라고 판단하지 않았다.
- guide 원점 생산 → `ColumnContent.inline_flow_plans` → layout `FullParagraph` → `layout_inline_flow_plan`의 실제 호출 경로를 대조했다. 같은 plan이 전달되는 것은 확인했지만, 독립 출력과의 일치까지 입증하는 근거로 확대하지 않았다.
- 원격 base `e1ecaa248…` / head `3edcbdbe…`의 merge simulation은 exit0, tree `b353863c3a4ae77cea2049af9db1fb8a4780d92e`다. 공백·상대 링크22건·기존 오늘할일228개 보존을 통과했다.

이 재검토는 로컬 review 문서에만 기록한다. 원격 push·review/comment 게시·merge·이슈 종료는 이번 요청에서 수행하지 않았다.

## 접수 정보

| 항목 | 값 |
| --- | --- |
| PR·작성자·base | [#7544](https://github.com/edwardkim/rhwp/pull/7544) / postmelee / devel |
| 기준 base | `e1ecaa248ecf7f667d8fccab4d9938e70a253392` |
| production/test 검증 source | `97772d5d40787e77c3238debc2b2576b11713476` |
| 제출 code candidate·재검토 head | `c0b075ca93b284af6d6c54975b92e7e7ae3531dd` → `3edcbdbe172767d135ce420e459a41fc9b44f12e`; 검증 source 이후 증적·문서만 변경 |
| 관련 이슈 | [#7486](https://github.com/edwardkim/rhwp/issues/7486), `Fixes`; #7487/#7539의 잔여 표 경로 |
| reviewer·시점 상태 | 본인 PR이므로 reviewer 미지정 / 재검토 시점 Open, non-draft, MERGEABLE / mergeStateStatus=CLEAN / GitHub CI 성공; 필수 증거 부족으로 리뷰 판정은 보류 |
| 라우팅 | collaborator self-merge §8.2.1; archive self-review + 오늘할일을 trailing 문서 commit으로 포함 |

## 변경과 검토 범위

`section.rs`의 사후 빈 쪽 삭제와 `trailing_disposition`의 앞 빈 줄 drift 기반 Hidden 처리를 제거한다. 이미 fit·배치한 새 빈 줄의 쪽·문단 소속을 유지한다. 저장본의 완료 PartialTable 직후 종료 guide는 유효한 실제 LineSeg와 본문/zone 기하를 검증한 뒤 같은 쪽에 소유시키며, 그 줄의 저장 원점과 표가 소비한 흐름 끝을 공유 plan으로 전달한다.

생산 결과 `FormattedParagraph` → `paragraph/flow.rs` fit/실패 이월 → `PageItem` → `section.rs` finalize → cursor lookup을 대조했다. 종료 guide는 `is_stored_table_closing_guide` → `place_stored_empty_guide` → `ColumnContent.inline_flow_plans` → layout `FullParagraph` plan → `layout_inline_flow_plan`으로 연결된다. `start`는 저장 vpos를 현재 zone 기준으로 변환한 실제 줄 원점이며 `end`는 기존 표 흐름 끝이다. 마지막 guide가 원점을 다시 추측하거나 흐름 끝을 clamp하지 않는다.

표 cut·rowspan·유닛 예약/실제 table paint는 변경하지 않는다. table coordinator가 모든 조각을 소비한 뒤의 문단 소유와 최종 페이지 할당을 고친다. 일반 본문 뒤 반복 Enter, 저장 줄이 본문 밖인 경우, 명시적 쪽/구역 나눔·상단 reset, 합성/무효 LineSeg, 다단은 새 guide 경로에 들어오지 않는다. 셀 내부 편집과 Studio source는 변경하지 않았다.

## 조판 원칙 판정

| 항목 | 판정·근거 |
| --- | --- |
| 구현 근거와 일반성 | 반복 Enter는 충족.종료 guide의 의미 해석은 미검증. API 생성·저장 원문과 독립 한컴 PDF의 1/2/2/2쪽은 확인했으나 실제 저장본 2개의 guide 해석을 독립 출력으로 검증하지 않았다. 문서 ID·표 행 수 예외, clamp는 없다. |
| 측정·배치 일관성 | 동일 plan 전달은 충족. 반복 Enter는 동일 fmt advance의 fit/배치를 보존한다. 종료 guide의 저장 줄 원점과 흐름 끝은 layout까지 공유하지만 그 원점·0 전진이 독립 출력과 맞는지는 미검증. |
| 분할·이어받기 계약 | table 컷/rowspan/요구·예약 높이 변경은 비해당. 기존 #7226/#6981의 쪽수·후속 구역·셀 포함과 #2097/#6761의 페이지 핀, off-canvas partition12는 통과했다. 최종 컷 뒤 guide의 독립 시각 계약은 미검증. |
| 줄 소속과 점유 높이 | 반복 Enter는 충족. 빈 글자의 표시·줄 상자·흐름 전진을 구분하며 정상 이월한다. 저장 종료 guide를 전진 없이 수용하는 의미 계약은 미검증. |
| 사례와 증거의 독립성 | 반복 Enter는 충족. 수동 XML 편집 없는 합성 계약 4개와 같은 원문의 한컴 PDF 전체 7쪽을 확인했다. 실제 저장본 guide 2개는 회귀·저장 메트릭 근거에 그쳐 필수 시각 증거는 미검증. |
| 기준값 변경 | 비해당. baseline·golden·허용치 변경 없음. 첫 전체 회귀 4 FAIL, 중간 guide의 off-canvas 1 FAIL은 원인 보정으로 해소했으며 래칫을 완화하지 않았다. |
| 주장과 검증 범위 | 부분 충족. 아래 source·명령·전후 실행 증거와 최신 captures는 반복 Enter 범위를 입증한다. 저장본 guide의 필수 시각 증거·Docker 표준 WASM·별도 정량 benchmark는 미검증. |

## 검증 입력과 실행 결과

입력 커밋 확인은 **충족**이다. 실제 사용한 HWPX 4개는 `tests/fixtures/issue7486_table_enter/`, 대응 한컴 PDF 4개는 `pdf/issue7486_*-2020.pdf`이며 `3f1e5c596`부터 검토 history에 있다. 검증 source commit에서 실행 파일과 Git blob의 SHA-256을 대조했다. 파일로 사용한 합성 입력과 비교 PDF는 외부 경로에만 두지 않았다. 실제 기존 문서 8개 및 #2097/#2287 저장본도 기존 repository blob과 대조했다. 경로별 SHA-256·MCP 작업 ID·실제 source/backend 출처는 [validation.json](../../working/assets/issue7486-table-enter/validation.json)에 있다.

[원인·전후 실행 보고서](../../report/task_m100_7486_table_report.md)와 [단계 기록](../../working/task_m100_7486_table_stage1.md)을 근거로 사용한다.

| 명령·범위 | 결과 |
| --- | --- |
| 별도 review checkout `--prepare`; `cargo fmt --all -- --check` | PASS |
| Native / WASM32 lib / workspace all-target Clippy `--locked`, `-D warnings`; workspace build | 모두 PASS |
| suite 정책 `--check --base-ref e1ecaa248…`; manifest 계약 검사 | PASS |
| 작은 경계 + off-canvas partition12 | Summary [  10.540s] 21 tests run: 21 passed, 10277 skipped |
| 관련 focused | Summary [  10.999s] 20 tests run: 20 passed, 10278 skipped |
| `cargo nextest run --locked --cargo-profile release-test --tests --no-fail-fast` | Summary [ 249.784s] 10248 tests run: 10248 passed (6 slow), 50 skipped |
| `cargo test --locked --profile release-test --features native-skia --lib` | test result: ok. 3927 passed; 0 failed; 13 ignored; 0 measured; 0 filtered out; finished in 44.50s; test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s; test result: ok. 165 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s; test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s |
| `run-rust-test.mjs issue_2225_missing_picture_placeholder -- … --features native-skia` | Summary [   1.186s] 2 tests run: 2 passed, 224 skipped |
| `run-rust-test.mjs render_p37_direct_pdf_export -- … --features native-skia` | Summary [   0.956s] 4 tests run: 4 passed, 219 skipped |
| root `CARGO_TARGET_DIR=target/pr-review scripts/wasm-pack-locked.sh --target web --out-dir pkg --no-opt` | fresh host release WASM PASS; root/public/서버 SHA-256 일치 |
| Chrome Enter/Undo/Redo / API 소속 진단 / 8개 저장 문서 대조 | 56 PASS / 720 owner 누락0 / 쪽수 변화0 |
| 전쪽 Native/fresh WASM sweep / 직접 review·overlay 판독 | 전체7쪽×2backend 최저100%, 대표4gate passed |

모든 Cargo는 같은 절대 `target/pr-review`를 순차 재사용했다. sweep 명령은 `pr-sweep-guide.sh`의 `--silhouette-only` 전체 쪽 및 `--pages 1,2` 대표 review 경로이며 각 실행의 provenance와 입력 해시는 validation.json에 보존했다.


새 회귀 최종 검사본을 동일 review checkout에서 base에 적용했을 때 기존 4개 PASS, 새 2개는 문단34/문단10 소속 없음으로 FAIL했고 보정 뒤 6 PASS다. 환경·빌드 실패를 수정 전 결함 FAIL로 계산하지 않았다. 정식 회귀 추가 전 `75b433ccb`의 Native/dev WASM 최저 100%·직접 판독 선행 증거를 보존했으며 최종 source에서 새 release host WASM과 Native로 다시 전쪽 비교·캡처했다.

API 12개 조합의 720 Enter 소속 누락 0, Chrome 실키 Enter/Undo/Redo 56 PASS, 기존 저장 문서 8개 쪽수 변화 0이다. Chrome은 사용자 세션과 별도의 headless 실행이며 사용자가 로컬 결과를 직접 재검증했다. 조작 없이 새 쪽 DOM 캐럿·viewport·100% 스크롤을 검사한다. 합성 계약 진단을 12개 조합 전부의 한컴 출력 검증으로 확대하지 않는다.

소스의 cfg(test), npm/editor, Studio source, capability, sample 원장은 변경하지 않아 해당 별도 게이트는 비해당이다. 실행 로그와 파생 suite는 ignored output에 있으며 제출 diff에 stage하지 않았다.

## 시각 증적과 남은 차이

| 경계 | Native review·overlay | fresh WASM review·overlay |
| --- | --- | --- |
| 10행·160%·Enter33 | [review](../../working/assets/issue7486-table-enter/native_table33_review_001.png) · [overlay](../../working/assets/issue7486-table-enter/native_table33_overlay_001.png) | [review](../../working/assets/issue7486-table-enter/wasm_table33_review_001.png) · [overlay](../../working/assets/issue7486-table-enter/wasm_table33_overlay_001.png) |
| 30행·300%·Enter9 | [review](../../working/assets/issue7486-table-enter/native_table30_review_001.png) · [overlay](../../working/assets/issue7486-table-enter/native_table30_overlay_001.png) | [review](../../working/assets/issue7486-table-enter/wasm_table30_review_001.png) · [overlay](../../working/assets/issue7486-table-enter/wasm_table30_overlay_001.png) |

Enter32/33/40의 1/2/2쪽, 30행 Enter9의 2쪽 전체를 96dpi print·2px 관용으로 비교했다. 양쪽 표 외곽·행/열 경계·시작/끝 위치와 빈 2쪽을 직접 판독했다. 모든 TSV 최저 100%, 원값 100%, boundary reconciliation 추가 픽셀0; 대표4출력 pr_review_gate=passed이다. 표 선 밝기 차이가 남으며 엄격 내용 픽셀은 약0.02%/7.46%다. 평균·실루엣으로 색상 차이를 면제하지 않는다. 글꼴 예외·영역 마스킹·관용치 변경 없음.

PR 본문에는 최신 head SHA의 실제 raw URL로 대표 review·overlay8개와 Studio4개를 표시한다. 게시 후 이미지 다운로드 해시와 브라우저의 실제 표시를 확인한다. source SHA와 asset이 추가된 제출 head SHA를 구분한다.

## Merge 후 contributor PR comment 계획

본인 PR이므로 원 기여자 PR 본문 수정·대신 Approve는 비해당이다. 별도 병합 승인을 받으면 이 PR과 #7486의 후속 처리에서 merge SHA·성공 CI URL 및 [Visual Sweep 정본](../../manual/verification/visual_sweep_guide.md#pr-body-visual-evidence)을 연결하고 실제 대표 PNG를 merge SHA로 고정해 재게시한다. raw URL은 `https://raw.githubusercontent.com/edwardkim/rhwp/<merge-commit-sha>/mydocs/working/assets/issue7486-table-enter/native_table33_review_001.png`와 동일 WASM/overlay 경로를 사용한다. 표 선 밝기와 Docker fallback 제한을 함께 적고 `--body-file` 게시 뒤 API로 본문·이미지를 재조회한다. 이 계획은 이번 턴의 게시·merge 승인으로 간주하지 않는다.

게시 문안: “표 뒤 Enter에서 새 빈 문단의 쪽 소속이 사라지는 경로와 저장본 종료 guide의 배치 원점을 보정했습니다. 선행 #7487/#7539의 수정 위에서 새 쪽 캐럿·스크롤, 저장·재열기와 관련 전체 회귀를 확인했습니다. 입력과 동일 한컴 PDF의 Native/fresh WASM 전체 비교 및 대표 증적을 연결드립니다. 표 선의 밝기 차이는 남아 있습니다.”


## 원격 제출·병합 사전 확인

code candidate `c0b075ca93b284af6d6c54975b92e7e7ae3531dd`를 upstream 작업 branch에 정상 push했고 Open PR #7544를 생성했다. 검증 source 이후 `src/`, `crates/`, `tests/`, Cargo·scripts·Studio source diff는 없다. force-push·GitHub Approve·merge·이슈 close는 수행하지 않았다.

candidate push 전에 base `e1ecaa248…`, head `c0b075ca9…`의 `git merge-tree --write-tree`는 exit0, tree `7d5a8d3de1d6b8a64a2317e4d48f9acb78c9deb8`이며 공백·상대 링크9건·기존 오늘할일228개 파일 보존을 통과했다. 이 문서와 오늘할일 trailing commit도 같은 절차를 push 전에 적용하고, 원격 head/base 재조회 및 최종 본문/이미지 API·브라우저 확인 결과를 PR 준비 로그에 보존한다. 작성 시점 CI URL: https://github.com/edwardkim/rhwp/actions/runs/37045229606 . 성공 결과로 기록하지 않는다.
