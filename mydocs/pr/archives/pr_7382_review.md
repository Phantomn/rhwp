---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-09-26
---

# PR #7382 리뷰 — 분할 표와 저장 각주 경계

## 현재 판정

**머지 보류.** 원 변경은 표의 시작 쪽을 개선하지만 기준 PDF의 행·각주 소유와 전체 페이지 수를 충족하지 않는다. 아래 개별 보정을 진행 중이며 통합 PR 생성·승인을 완료했다고 보고하지 않는다.

## 접수와 provenance

- 원 PR: [#7382](https://github.com/edwardkim/rhwp/pull/7382), planet6897. 원 head `81a402179dc556cce781d844d4b9252be36ba8af`.
- 최신 base `eb9142dd7c73297d555383d7d8434a470bdef26e`, 검토 branch `review/planet6897-7382-20260926`. 기존 reviewer jangster77 확인.
- 원 기능 `8e0f0249460a5243e38162340bc1c49d061ae72a` → `-x 5b2cc61cd`; 원 이미지 `fcba72b18483053c436388a8d88c64b81d67c2c0` → `-x a57d6430a`. source devel merge 81a는 체리픽하지 않았다.
- 관련 이슈 [#7379](https://github.com/edwardkim/rhwp/issues/7379). 부분 개선으로 전체 이슈를 종료하지 않는다.

## 독립 입력과 기대값

- [원본 HWPX](../../../samples/정책연구용역사업%20중간진도보고서%28살아있는%20간장%20기증자의%20의학적%20선별기준%20연구%29.hwpx), [한컴 2024 기준 PDF](../../../pdf/정책연구용역사업%20중간진도보고서%28살아있는%20간장%20기증자의%20의학적%20선별기준%20연구%29-hwpx-2024.pdf), 215쪽. 이미 저장소에 있는 동일 입력·기준을 사용한다.
- 기준 66쪽은 문단728 표의 머리행+본문4행(원 행0..4), 67쪽은 나머지2행(5..6). 원본 개체 높이11645HU는 첫5행의 저장 높이 합과 정확히 같다.
- 각주77은 앞 두 줄이66쪽, `Part 482(CONDITIONS...)`와 출처인 꼬리는67쪽이다. 원본 첫 각주 문단의 저장 vpos `0,1172,0`과 다음 문단의1172가 이 경계를 지시한다. 번호77은 꼬리에서 반복하지 않는다.
- 동일 보고서 HWP 대조군은 현재 제품 코드에서215쪽이며, 표728도66쪽0..5/67쪽5..7(exclusive)으로 나뉜다. 이 대조군을 HWPX 최종 시각 검증의 대용으로 쓰지 않는다.

## 원 변경의 재검토 결과

| 실행 | 결과 | 의미 |
| --- | --- | --- |
| base 제품+원 회귀2개 | 1PASS/1FAIL, exit100 | 66쪽에 표 없음으로 의도한 실패; 각주 없는 표는 정상 대조군 |
| 원 변경 적용 후 원 회귀2개 | 2PASS, exit0 | 표 존재·대략적인 높이만 검증 |
| 원 변경 후 실제 출력 | 217쪽 / PDF215쪽 | 부분 개선이며 전체 쪽수 계약 불충족 |
| 새 행·각주 소유 검사, 원 변경에서 실행 | 1FAIL, exit100 | 실제 첫 행 집합{0,1,2,3}, PDF 기대{0,1,2,3,4} |
| 원 변경 Native Visual Sweep66/67 | 82.17683% / 61.0948%, exit1 | 표 마지막 행 이월·각주77 누락, 재검토 gate. 글꼴 예외 없음 |

원 `register_body_footnote` 설명과 실제 표 셀 각주 경로가 다르다. 통째 배치 뒤에는 `controls/paragraph_flow.rs` → `notes::register_unqueued_table_cells` → `table::register_unqueued_table_footnote`가 모든 셀 각주를 등록한다. 진입에서 전체 각주 예약을 제거하면 통째 배치의 사전 fit도 바뀐다. 원본 높이 합 검사만으로 행 누락·중복이나 각주 분할을 입증할 수 없다.

## 메인터너 보정 1: 저장된 각주 경계를 분할 큐에 연결

사전 분석 → 코드/회귀 수정 → 실행 결과보고 → 개별 커밋 순서로 진행한다. 전체 수정이 준비되기 전에는 원 PR과 통합 후보 모두 보류다.

1. `entry.rs`는 base의 전체 각주 예약을 복원해 통째 fit 계약을 유지한다.
2. HWPX 파서는0에서 시작한 각주의 양수→0 재시작을 보존한다. 기존2344→0 연속줄 보정·all-zero HWP5 및 미주 경로는 대조군으로 확인한다.
3. `table.rs`의 유효 저장 줄/composer 일대일 각주 경계 판정을 미편집 HWPX에도 연결한다. split 실패 뒤 `prepare.rs`에서 비TAC·RowBreak·rowspan 없는 다행 표의 저장 각주 경계를 fragment queue가 소비한다.
4. `fragment/emit.rs`가 확정한 행·컷과 물리 높이를 먼저 예약하고 `table/footnotes.rs`가 marker를 가진 중간 조각에prefix, 다음 조각에tail을 등록한다. 숫자상 전체 각주가 fit해도 저장된 물리 경계를 지우지 않는다. fresh page 큐 소진·terminal·intra-row 및 marker가 없는 조각의 기존 계약은 유지한다.
5. [정식 회귀](../../../tests/cases/issue_7379_rowbreak_table_footnote_reservation.rs)는 최종 render tree에서 행 소유·각주77 앞뒤 내용·번호 중복·표/각주 비충돌을 직접 검사한다.

파서/큐 연결만 수행한 중간 후보는2PASS/1FAIL이었다. 행은4+2로 개선됐으나77번 꼬리도66쪽에 표시되어 새 검사로 실패했다. 이는 완료로 보고하지 않고 같은 범위에서 각주 등록 경로를 추가 보정했다. 첫 보정 후 집중3/3 PASS(0.176s), 정상 대조군9/9 PASS(0.565s), exit0을 확인했다. 대조군은2344→0 보정, all-zero 보존, #6495/#6545 미주 및 HWP→HWPX 왕복215쪽, #1937 표 각주를 포함한다. Native review/standalone overlay66·67쪽을 직접 판독했다. 행·각주 앞뒤 소유는 개선됐지만 표의 원점·본문/캡션 간격·각주 줄 위치가 다르고 실루엣88.1603%/73.99603%, exit1이다. 현재 출력219/PDF215쪽이며 승인하지 않는다. 전체 회귀·lint·fresh WASM은 아직 실행 완료로 보고하지 않는다. [단계1 검증](../assets/pr7382_20260926/stage1_validation.json)과 [Native manifest](../assets/pr7382_20260926/stage1_native_run_manifest.json)는 imported head+단계1 worktree diff의 진단 증거다. 최종 code head에서는 새로 캡처한다. 원 source의 issue_7379 PNG5개도 과거 증적이며 현재 수용 근거로 재사용하지 않는다.

![단계1 Native66 review](../assets/pr7382_20260926/stage1_native_review_066.png)
![단계1 Native67 review](../assets/pr7382_20260926/stage1_native_review_067.png)

첫 추가 쪽은 HWPX의77쪽 TAC 그림/캡션 표876이78쪽으로 밀리는 경계에서 발생한다. 이어885 대형 표의 셀 각주,1822 대형 단일 셀과 이후 경계가 남는다. 이를 글꼴이나 기존 차이로 면제하지 않고 한 경계씩 사전 분석·수정·전후 검사한다.

## 메인터너 보정 2: 표 원점과 캡션 종료 간격

단계1 `483cfb4f8`의 표 상단796.9/83.2px와 캡션166.1px, 뒤 본문206.1px는 PDF799.925/86.945/156.434/200.261px와 달랐다. 새 최종 좌표 회귀는 수정 전 첫 표 원점으로 FAIL(exit100)했고, 위 여백·캡션만 연결한 중간 후보는 뒤 본문196.533px로 FAIL했다. 측정에서 예약한 끝 바깥여백이 paint의 반환 높이에 빠진 경로도 같은 범위에서 보정했다.

- `host_spacing → fragment/budget → table_partial`의 위 바깥여백 조건을 공통 query로 연결했다. 열수는 바깥여백의 근거가 아니므로 기존 1열 한정을 제거했다. 중첩 frame과 이미 해결된 저장 원점의 중복 inset 방지는 유지한다.
- 저장된 단일 zero-width 줄이 단일 비TAC 표만 소유한 앵커일 때, 호스트 줄간격을 캡션 gap에 추가하지 않는다. `prepare`의 요구 높이와 `table_partial`의 실제 캡션 위치가 같은 결과를 소비한다. 보이지 않는 줄이라는 이유로 줄 상자를 제거하는 처리가 아니다.
- 아래 캡션이 끝난 뒤의 바깥여백을 예산·흐름 전진·paint 반환 높이에 함께 연결했다. 중간 컷의 행 예산에서 이 종료 간격을 미리 차감하지 않는다.
- 집중5/5 PASS(0.794s), 정상 대조군·PrEP·공통 앵커51/51 PASS(0.901s), exit0. 여백/캡션 간격 IR 변형3종도 행 소유·캡션 한 번·각주 lane 비충돌을 확인했으나 수정 전에도 통과하는 대조 검사다. 진단 trace상 세 변형은 terminal 수용 예산 안에 들어갔으므로 **공간 부족 후 재분할 분기의 실행 증거로 인정하지 않는다**. 그 분기는 미검증으로 남기고 다음 경계 검사에서 확인한다. 합성 IR은 한컴 출력의 대용이 아니다.
- 새 CLI의 전체 출력은219/PDF215쪽으로 불충족이다. Native66/67의 2px 실루엣은98.24785%/93.58939%, 선택 페이지 gate passed(exit0), 글꼴 예외 없음. 대표 review와 standalone overlay 두 쪽을 직접 판독해 행·각주 소유, 괘선·캡션·뒤 본문 위치를 확인했다. 각주 가로 공백·글자 폭과 얇은 괘선 차이는 남으며 pixel-perfect 일치라고 보고하지 않는다.

[단계2 검증](../assets/pr7382_20260926/stage2_validation.json)과 [Native manifest](../assets/pr7382_20260926/stage2_native_run_manifest.json)는 `483cfb4f8`+단계2 worktree diff의 진단 증거다. 최종 head의 fresh WASM·전체 회귀/lint는 아직 완료하지 않았다. 전체 쪽수가 다르므로 선택 페이지 통과에도 PR 생성·승인은 보류한다. 첫 추가 쪽 앞 표866의 행 내부 컷과 뒤 그림876의 여유 공간을 HWP 대조군/PDF와 연결해 다음 보정을 준비한다.

![단계2 Native66 review](../assets/pr7382_20260926/stage2_native_review_066.png)
![단계2 Native67 review](../assets/pr7382_20260926/stage2_native_review_067.png)

## 메인터너 보정 3: 저장 셀 내부 컷과 그림51의 물리 페이지

보정2 뒤 전체219쪽의 첫 추가 쪽을 역추적했다. 표866의 원본 row4 첫 셀은 `vpos=0,1620,3240,0` / `textpos=0,17,35,53`으로 첫 세 줄과 `투석을 시작하게 된 경우` 꼬리를 다른 물리 쪽에 저장한다. PDF76/77 및 HWP 대조군의 실제 컷 `[3,1,1]`이 같은 소유를 뒷받침한다. HWP 대조군215쪽을 HWPX 좌표·시각 일치의 대용으로 쓰지 않는다.

`existing footnote43.4px → entry available → prepare table_available → fragment/budget → row_cut → PartialTable → table_partial/FootnoteArea`를 대조했다. 기존 HWP5 예산324.4px와 달리 HWPX는40px 안전 여백을 더 빼284.4px만 주고, 행4의 첫 한 줄 후보가 고아 줄 검사에서 기각됐다. 파서에서 저장 reset을 삭제한 문제가 아니었다. 새 최종 tree 회귀는 수정 전 행4가 없는 것으로1FAIL(exit100,0.234s)했다.

- 기존 각주만 있고 자체 각주가 없는 비TAC T&B RowBreak 표에서, HWPX의 **한 셀 문단 안** 유효한0→양수→0 저장 경계에도 실제 각주 경계를 사용한다. 편집/reflow와 rowspan은 새 적용에서 제외한다. 문단마다 시작하는 local0만으로는 이 계약을 열지 않으며 일반40px 여백·고아 줄 임계값을 전역으로 완화하지 않았다.
- 첫 후보는 prefix/tail과 그림51의77쪽 소유를 복원했으나 캡션908.973/PDF913.379px로5PASS/1FAIL이었다. 표tail 위86.933/아래257.546px는 PDF86.945/257.799px와 가까웠다. Top 외부 캡션 표의 끝 바깥여백283HU=3.773px가 뒤 빈 문단 흐름에 빠진 것이므로, 세로 캡션 개체 종료의 예산·flow·paint 반환을 같은 query로 연결했다. 중간 컷·가로 캡션·외부 캡션 없는 표의 기존 계약은 유지한다.
- 최종 집중6/6 PASS(0.889s), 추가 좌표 검사와 정상 대조군을 함께 실행한75/75 PASS(1.742s), exit0. 대조군은 기존51개에 셀 단위 원자 분할/왕복, profile 독립 source frame, #6761의 확인된 저장 경계와 무효/다른 컷 반례를 더했다. 표866 앞 세 줄/꼬리의 내용·행 소유, 표tail 상·하단, 뒤 본문288.289px와 그림51 캡션913.379px, 표/각주 비충돌을 최종 tree에서 검사한다.
- 새 release-test CLI의 전체 출력은**218/PDF215쪽**이다. 표866은76쪽0..5+cut[3,1,1] /77쪽4..7+startCut[3,1,1], 그림876은77쪽으로 바뀌었다. Native76/77은96.49182%/98.35446%, 선택 gate passed(exit0), 글꼴 예외 없음. review와 standalone overlay 두 쪽을 직접 판독했다. 첫 표 조각의 얇은 괘선·셀 글자 위치, 각주 링크 색/폭 차이는 남으며 완전한 픽셀 일치를 주장하지 않는다.

[단계3 검증](../assets/pr7382_20260926/stage3_validation.json)과 [Native manifest](../assets/pr7382_20260926/stage3_native_run_manifest.json)는 `22d23872c`+단계3 worktree diff의 진단 증거다. 첫 추가 쪽을 해소했지만 표885의 대량 셀 각주, 표1822의 단일 셀 분할 및 이후 경계가 남는다. 전체 쪽수·최종 head full Rust/lint/Skia/fresh WASM·terminal 캡션 예산 부족 경계는 미해결/미검증이며 PR 생성·승인 보류다.

![단계3 Native76 review](../assets/pr7382_20260926/stage3_native_review_076.png)
![단계3 Native77 review](../assets/pr7382_20260926/stage3_native_review_077.png)

## 남은 필수 게이트

- 전체 페이지 수215, 추가/누락 쪽의 첫 경계와 앞뒤 내용을 재검토한다. 선택66/67쪽의 통과만으로 이 조건을 면제하지 않는다.
- 최종 code head의 Native/fresh WASM Visual Sweep·대표 review/standalone overlay 직접 확인 및90% gate, 적용/비적용 경계 검사.
- cargo nextest threads8 전체 회귀, 모든Rust lint/WASM32/workspace all-target Clippy, workspace build, 고정base 정책 검사와 필요한 Skia/fresh WASM 검증.
- source-number review·오늘할일·대표 시각 증적을 같은 통합 PR에 포함하고 정확한 최신head CI/MERGEABLE/CLEAN 확인. 통합 owner reviewer 자동 지정 없음.
- 정상 merge 후 duration provenance·source supersede/범위에 맞는 issue 처리·devel 동기화·소유 output 정리. 로그는 ignored `output/pr-review/planet6897-7382-20260926/logs/`에만 저장하며 커밋하지 않는다.
