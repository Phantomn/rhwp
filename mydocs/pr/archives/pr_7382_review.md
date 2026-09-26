---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-09-26
---

# PR #7382 리뷰 — 분할 표와 저장 각주 경계

## 현재 판정

**머지 보류.** 원 변경은 기준 PDF의 행·각주 소유와 전체 페이지 수를 충족하지 않는다. 보정13 통합 후보는215쪽을 유지하고 선택66/67·30/31·178/179쪽을 개선했지만, 전체 시각 비교의 본문 소유 차이와 최종 필수 게이트가 남았다. 통합 PR 생성·승인을 완료했다고 보고하지 않는다.

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

## 메인터너 보정 4: 다행 표의 셀 각주 큐와 확정된 표시 소유

보정3 뒤 표885의 셀 각주18개를 표 시작 전에 모두 예약해 표가79쪽으로 밀렸다. 독립 PDF78은 행0..3의 앞 조각과 기존각주105/106, 79는 행3 꼬리와 끝2행 및각주107~111, 80은 남은각주112~124와 뒤 본문이다. 새 최종 tree 검사는 수정 전78쪽 표 부재로 FAIL(exit100,0.176s)했다.

- 미편집/reflow 없는 direct HWPX 다행·비TAC T&B RowBreak·rowspan 없는 원표에도 셀 각주 큐를 연결했다. 각주 자체의 저장 reset은 prefix/tail 분할의 근거이며 보통 각주의 큐 참여를 제한하는 근거가 아니다. 통째 fit의 전체 각주 예약과 기존 Native/단일행 경로는 유지한다. 빈1×1 래퍼의 내부 행 컷을 바깥 각주 소유로 혼동하지 않는다.
- 추가 반례에서 아직 다음 조각 row6에 있는82번이 앞쪽66에 등록돼 FAIL(exit100,0.253s)했다. 행만 제한한 후보도 같은 행의 미소비 줄에 있는116번을 앞쪽에 등록해 FAIL(exit100,0.257s)했다. 실제 확정 `end_row/end_cut → 같은 폭·방향의 composed 줄 → cached CellUnit ordinal → col순 셀 컷`을 등록 상한으로 사용한다. 앞 조각에서 표시를 이미 소비한 밀린 각주는 허용한다. 두 합성 IR은 원래 저장 줄/표를 유지하고 각주 몸통 또는 주석 종류만 바꿔 용량과 소유를 분리했으며, 한컴 출력의 대용이 아니다. 최종 marker/footer 누락·중복과 물리 소유를 검사한다.
- 최종 집중·정상 대조군 **78/78 PASS(exit0,2.008s)**. 실제 표885 행/각주 소유, 앞서 보정한 표728/866과 그림51, 저장 reset 반례, 원자 컷·왕복·공통 앵커·PrEP를 함께 확인했다. 새 CLI 전체 출력은 **217/PDF215쪽**이다.
- Native78/79/80은 **87.97988% / 96.21953% / 95.66118%**(2px 실루엣; 원 JSON 참조), sweep exit1/re_review_required, 글꼴 예외 없음. 세 review와 standalone overlay를 직접 판독했다.78쪽 표 앞 조각의 아래선은960.947/PDF970.937px이고 셀 줄 위치도 다르다.79쪽 표 끝은 근접하지만 각주 가로 폭/색·일부 줄 위치 차이가 남는다.80쪽 본문과 각주 소유는 맞지만 링크 폭·색/일부 줄 위치 차이가 있다. 이 결과를 시각 개선 완료로 승격하지 않는다.

각주 개수>=8에 따른 기존 첫 조각 지연 및 terminal guard 예외도 새 HWPX 큐에 따라 들어왔다. 이를 일반 정책의 근거로 인정하지 않는다. 개수 예외를 배제한 후보는8PASS/1FAIL이었다. 표 끝960.947px에 각주 영역945.007px가 겹쳤다. fit에서 빈 footer band를 회수하지만 실제 각주 영역은 본문 하단에 고정하며, 기존 Body각주105의 빠른 추정이 내부 줄간격을 빠뜨리는 차이가 연결된다. `caption_extra` 누락도 코드 우려로 발견했지만 표885는 해당 값0이므로 이 표의 원인으로 보고하지 않는다. 실패 후보는 회수했고 **용량 계산·물리 끝점·기존 개수 예외의 일반화는 다음 개별 보정의 미해결 사항**으로 남긴다. 현재 커밋은 소유 보정의 독립 중간 결과이며 PR 수용 후보가 아니다.

[단계4 검증](../assets/pr7382_20260926/stage4_validation.json), [Native manifest](../assets/pr7382_20260926/stage4_native_run_manifest.json), [Native summary](../assets/pr7382_20260926/stage4_native_summary.json)는 `cd2203a07`+단계4 Rust diff를 고정한 진단 증거다. 최종 head full Rust/lint/Skia/fresh WASM은 아직 완료하지 않았다.

![단계4 Native78 review](../assets/pr7382_20260926/stage4_native_review_078.png)
![단계4 Native79 review](../assets/pr7382_20260926/stage4_native_review_079.png)
![단계4 Native80 review](../assets/pr7382_20260926/stage4_native_review_080.png)

## 남은 필수 게이트

- 전체 페이지 수215, 추가/누락 쪽의 첫 경계와 앞뒤 내용을 재검토한다. 선택66/67쪽의 통과만으로 이 조건을 면제하지 않는다.
- 최종 code head의 Native/fresh WASM Visual Sweep·대표 review/standalone overlay 직접 확인 및90% gate, 적용/비적용 경계 검사.
- cargo nextest threads8 전체 회귀, 모든Rust lint/WASM32/workspace all-target Clippy, workspace build, 고정base 정책 검사와 필요한 Skia/fresh WASM 검증.
- source-number review·오늘할일·대표 시각 증적을 같은 통합 PR에 포함하고 정확한 최신head CI/MERGEABLE/CLEAN 확인. 통합 owner reviewer 자동 지정 없음.
- 정상 merge 후 duration provenance·source supersede/범위에 맞는 issue 처리·devel 동기화·소유 output 정리. 로그는 ignored `output/pr-review/planet6897-7382-20260926/logs/`에만 저장하며 커밋하지 않는다.


## 메인터너 보정 5: 첫 저장 프레임의 빈 밴드와 셀 정렬

보정4 `45f40cf44`의78쪽 표25 끝960.947px는 PDF970.937px와 달랐다. 원본 HWPX table.sz.height33323HU=444.307px는 PDF 첫 프레임443.833px와 대응하나 내용 컷만 그린433.32px 상자는 빈 하단 밴드를 잃었다. HWP 대조군의 셀/줄 원본 메트릭은 같지만 그 현재 출력도 geometry가 달라 독립 좌표 기준으로 승격하지 않았다. 새 끝점 검사는 수정 전 FAIL(exit100); 같은 실행의 raw-source 진단 PASS는 결함 검출 검사로 세지 않는다.

- `saved_multirow_opening_frame_height → emit partial_height/end_row_height_override → table_partial row_heights/partial_table_height → 실제 셀/표 bbox`가 같은 원본 프레임을 소비한다. 미편집·reflow 없는 stored 비TAC T&B RowBreak, rowspan 없는 다행 표의 첫 빈 start_cut과 유효한 plain-text 문단 재시작 end_cut을 요구한다. emit은 객체전용 저장 앵커·래퍼 아님·기존 override 없음·실제 남은 예산 fit도 확인한다. 프레임 빈 밴드는 소비 유닛이 아니므로 다음 내용 tail에서 빼지 않는다. 본문보다 큰 수동1000px 프레임을 강제 수용하지 않는 정식 거부 대조군도 PASS했다.
- 첫 프레임 후보는80PASS였지만 Native78 직접 판독에서 셀 row3/col2가 Top으로 강제되는 차이를 발견했다. 원본 Center, 처음 세 문단8개 저장 줄의125.013px, 원본 padding 및 최종 cell bbox로 정한 기대839.427px에 실제833.813px가 FAIL(exit100,0.169s)했다. 같은 페이지 PDF 첫 prefix839.52px도 독립 확인했다.
- paint는 동일 query의 높이와 실제 partial_table_height가 일치하는 root 첫 프레임에만 원래 세로 정렬을 적용한다. 소비한 line_ranges의 높이로 계산하며 뒤 내용 전체를 중앙에 배치하지 않는다. 일반 예산 컷·continuation·rowspan·slice를 넘는 중첩 다중열 내용은 기존 계약을 유지한다. 첫 정렬 후보는 Top-only lazy composition 가정과 충돌해7PASS/5FAIL했다. 유한 프레임에서 Center/Bottom에 필요한 내용 높이는 전체 paint와 cursor probe 모두 계산하도록 수정하고 재검증했다.
- 최종 집중/정상 대조군81/81 PASS(exit0,2.951s). fresh CLI build exit0(1m54s), Native78/79/80의2px 실루엣97.8181%/96.21953%/95.66118%, 선택 gate passed(exit0), 글꼴 예외 없음. 세 review와 standalone overlay를 직접 확인했다. 표 하단·첫 셀 글줄과 후속 페이지 소유는 개선됐으나 얇은 괘선·본문 글자 및 각주 URL 폭 차이가 남으며 pixel-perfect라고 보고하지 않는다.

[단계5 검증](../assets/pr7382_20260926/stage5_validation.json)과 [Native manifest](../assets/pr7382_20260926/stage5_native_run_manifest.json)는45f40cf44+단계5 Rust/test diff를 고정한다. 전체217/PDF215쪽 차이로 재검토 상태이며 PR을 만들지 않는다. 각주 수 기반 예외/실제 footer 용량,174·175쪽 단일 셀1822의3쪽 분할, 뒤 추가 페이지, terminal caption 예산 실패 및 최종 전체 Rust/Skia/lint/fresh WASM은 남았다. 단계5를 먼저 독립 커밋하고 다음 경계 보정을 수행한다.

![단계5 Native78 review](../assets/pr7382_20260926/stage5_native_review_078.png)
![단계5 Native79 review](../assets/pr7382_20260926/stage5_native_review_079.png)
![단계5 Native80 review](../assets/pr7382_20260926/stage5_native_review_080.png)


## 메인터너 보정 6: 각주 큐와 실제 paint 영역의 용량 공유

`913a04326`에서 각주107·108만 남기고 다른 marker 슬롯을 빈 Endnote로 보존한 합성 IR은 수정 전 FAIL(exit100,0.296s)했다. 78쪽 표끝972.173px / 실제 각주위945.007px로 충돌했다. 이 반례는 각주 개수8개 임계값의 가정을 깨는 계약 검사이며 한컴 출력 일치 근거로 승격하지 않는다.

- `확정 body fragment/end_cut → 후보 FootnoteRef 목록 → estimate_footnote_area_height_with_metrics → body-bottom 물리 예산 → 수용 목록/동일 높이 예약 → layout_footnote_area`를 연결한다. 기존 Body 각주도 후보 목록에 포함해 실제 composed 줄간격·선·각주간 간격을 함께 측정한다. shape 기반 최종 paint 측정은 같은 helper를 호출하며 일반 출력의 기존 산식은 유지한다.
- 단일단 direct HWPX 큐는 빈 footer 밴드를 실제 paint보다 아래로 회수하지 않는다. 수용 뒤 정확한 영역 높이와 body-bottom 예약 상태를 현재 page에 기록하고 페이지 전환 시 초기화한다. 다음 본문의 available_height와 추가 각주 fit도 이 상태를 소비한다. 그림에서 이월된 note body의 별도 상태를 재사용하지 않아 저장 vpos 경로를 바꾸지 않는다.
- 새 HWPX 경로의 첫 조각 지연·terminal 수용은 각주 개수/선언 비율과 분리했다. 기존 native의 개수/guard 정책은 별도 근거 없이 바꾸지 않았다. 확대된 HWPX 큐는 단일단만 적용한다. 다단의 body-wide footnote 계약은 이번 신규 경로에서 비해당이며 그 검증을 완료했다고 보고하지 않는다. synchronous source와 resumed source는 같은 section 문단 목록을 전달한다. 현재 resumable 진입은 편집 상태를 요구하고 새 stored HWPX 큐는 편집/reflow를 제외하므로 해당 신규 큐의 resumed 실행을 주장하지 않는다.
- 집중13/13 PASS(exit0,1.502s), terminal 뒤 실제 본문 비충돌까지 추가한 최종 집중/정상 대조군85/85 PASS(exit0,3.023s). 기존 #1937/#4882/#6495/#6545 및 row-cut/앵커/PrEP 대조에 각주 줄높이 #5708·영역 폭 #6034를 포함했다.
- fresh CLI build exit0(1m52s), Native66/67/78/79/80 선택 gate passed(exit0),2px 실루엣98.24785%/93.58939%/97.8181%/96.21953%/95.66118%, 글꼴 예외 없음. 다섯 review 및 standalone overlay를 직접 판독했다. 표/각주 소유와 위치는 유지됐으며 얇은 괘선·각주 glyph/URL 폭 차이는 남는다.

[단계6 검증](../assets/pr7382_20260926/stage6_validation.json)과 [Native manifest](../assets/pr7382_20260926/stage6_native_run_manifest.json)는913a04326+단계6 Rust/test diff의 증거다. 전체217/PDF215쪽으로 PR 생성·승인은 계속 보류한다. 이 보정을 독립 커밋한 뒤 단일 셀1822의174~176쪽 분할과 이후 추가 페이지, terminal caption 예산 실패를 각각 해결한다. 최종 전체 회귀·lint·Skia·fresh WASM은 아직 완료하지 않았다.

![단계6 Native66 review](../assets/pr7382_20260926/stage6_native_review_066.png)
![단계6 Native67 review](../assets/pr7382_20260926/stage6_native_review_067.png)
![단계6 Native78 review](../assets/pr7382_20260926/stage6_native_review_078.png)
![단계6 Native79 review](../assets/pr7382_20260926/stage6_native_review_079.png)
![단계6 Native80 review](../assets/pr7382_20260926/stage6_native_review_080.png)


## 메인터너 보정 7: 단일 셀 첫 프레임·표시 소유·호스트 종료 간격

보정6 뒤 단일 셀 표1822는174/175/176의 세 조각으로 나뉘었다. 원본 마지막 줄43311+1000+padding282=44593HU와 다음 셀 문단의 vpos0은 첫 프레임의 정확한 컷13을 증언한다. 독립 PDF174에는그림66과224번 표시를 가진 끝 문장이,175에는나머지표·각주223..231·뒤제목이 있다. 소유 회귀는 수정 전174쪽그림66 부재로 FAIL(exit100)했다.

- `saved_single_cell_opening_frame_cut/height → queue preparation → row scan exact end_cut/physical demand → partial-table paint`가 같은 컷/프레임을 사용한다. 정상 padding과 저장 프레임의 유효성은 다른 계약이므로 malformed-padding 조건을 프레임 근거로 대신하지 않는다. 경계의 inline Footnote/Endnote marker는 원래 글줄에 속하며 block control은 허용하지 않는다. 직접 저장 HWPX·미편집/reflow 없음·비TAC RowBreak·1×1·정확한 저장 끝점과 reset·실제 예산 fit 조건을 유지한다. 임의 각주 개수/선언 비율로 이 경로를 선택하지 않는다.
- HorzColumn T&B RowBreak의 바깥 상단 여백은 단일/다행에 동일하게 적용한다. resolved 원점·중첩/다른 위치의 continuation은 기존 호출자 조건으로 중복 여백을 막는다. 첫 geometry 검사는429.827/PDF433.127px로 FAIL했다. 수정 후 네 표 끝점은통과했지만 뒤제목624.027/PDF641.061px로14PASS/1FAIL했다.
- 추적에서 terminal 뒤 호스트 trailing spacing1000HU와 바깥 아래283HU가 빠졌다. 같은 helper의17.107px를 pagination 종료와 실제 partial paint 종료에서 한 번 소비하고 bbox에는 더하지 않는다. 첫 후보는86PASS/1FAIL: 폭0인 PrEP 객체전용 앵커에도 밴드를 더해40쪽본문238.6/PDF220.864px가 됐다. 기존 `object_only_saved_table_anchor` 계약을 공유해 그 앵커에는 글줄 전진을 더하지 않도록 수정했다. 이는 문서ID/각주수 예외가 아니다.
- 최종 집중/정상 대조군87/87 PASS(exit0,3.257s), fresh CLI build exit0. 전체216/PDF215쪽으로 한 추가 쪽을 해소했다. Native174/175/176은99.85942%/92.46177%/79.50902%, selected sweep exit1/re_review_required, 글꼴 예외 없음. 세 review와 standalone overlay를 직접 확인했다.174의그림·첫표하단 및175의꼬리·뒤제목이 맞지만176의표위치·각주234 저장 경계가 다르다. 각주 glyph/폭과 얇은 괘선의 차이도 남는다.

[단계7 검증](../assets/pr7382_20260926/stage7_validation.json), [Native manifest](../assets/pr7382_20260926/stage7_native_run_manifest.json)는8786be62b+단계7 Rust/test diff를 고정한다. 이 단계는 독립 중간 보정이며 PR 생성/승인을 계속 보류한다.176/177 각주234,182쪽뒤문단1913의 추가 쪽,terminal caption 예산 실패,최종 full Rust/lint/Skia/fresh WASM을 다음 개별 단계에서 해결한다. 테스트 작성의 소수점 표기·Rust 이동 소유 컴파일 오류는 수정했으며 결함의 수정 전 FAIL 증거로 세지 않는다.

![단계7 Native174 review](../assets/pr7382_20260926/stage7_native_review_174.png)
![단계7 Native175 review](../assets/pr7382_20260926/stage7_native_review_175.png)
![단계7 Native176 review](../assets/pr7382_20260926/stage7_native_review_176.png)


## 메인터너 보정 8: 큐 없는 셀 각주의 저장 페이지 경계

원본 각주234의 두 저장 줄은 vpos0/0, flags393216/1441792다. 독립 PDF176은 번호와 첫 줄,177은 번호 없는 꼬리를 소유한다. 보정7의 최종 tree에서 꼬리가176쪽에 남아 새 회귀가 FAIL(exit100,0.191s)했다. 합성 TAG를 붙인 대조군은 같은 실행에서 PASS이며 결함 검출 증거가 아니다.

- `원본 stored/composed 줄 일대일 → 기존 reset query → register_unqueued_table_footnote_with_content_height → prefix/suffix FootnoteRef → 최종 FootnoteArea/TextLine`을 연결했다. table.rs의 셀 각주 수집은 이미 미편집 direct HWPX를 허용하지만 whole 표의 큐 없는 등록은 Native만 허용해 fragment 정보를 잃었다. 같은 저장 경계 판정을 큐 없는 등록에도 적용한다. 미편집 HWPX만 확대하며 합성/일대일이 깨진 줄은 기존 query가 거부하고 다단은 기존 조건으로 제외한다. Body30/240은 별도 등록 경로이므로 이번 수정의 해결 범위로 보고하지 않는다.
- 실제 prefix/꼬리의 페이지 소유·번호 누락/중복·뒤 각주235와 각주 첫 줄 좌표(PDF176949.169px/177996.209px)를 검사했다. 수동 합성0/0은 페이지 이월을 강제하지 않는 반례도 PASS했다. 집중17/17 PASS(exit0,1.773s), 관련 각주/왕복/PrEP 정상 대조를 포함한46/46 PASS(exit0,3.070s), fresh CLI build exit0(1m53s).
- Native176/177은85.24789%/90.47055%, sweep exit1/re_review_required, 글꼴 예외 없음. 두 review와 standalone overlay를 직접 확인했다.234 꼬리의 이월과 footer 위치가 개선됐지만176의 표1832 상단483.2/PDF486.508px 및177 표1843의 같은 여백 차이가 남는다. glyph/각주 폭 차이도 있으며 시각 통과로 승격하지 않는다.

[단계8 검증](../assets/pr7382_20260926/stage8_validation.json)과 [Native manifest](../assets/pr7382_20260926/stage8_native_run_manifest.json)는0c35e4458+단계8 Rust/test diff의 중간 증거다. 전체216/PDF215로 PR은 계속 보류한다. 그림67 뒤의 guide 중복 점유와 그림/캡션 위치는 HWP 대조군에서도 독립 PDF와 차이가 있어 HWP 쪽수215를 geometry 정답지로 삼지 않는다. 단일 셀 whole 표의 바깥여백·마지막 추가 페이지·terminal caption 예산 실패와 최종 필수 검증을 이어 해결한다.

![단계8 Native176 review](../assets/pr7382_20260926/stage8_native_review_176.png)
![단계8 Native177 review](../assets/pr7382_20260926/stage8_native_review_177.png)


## 메인터너 보정 9: 단일 셀 통째 표의 바깥 상단 여백

PDF176/177의 실제 괘선은 각각486.508~903.012px/539.729~689.965px다. 원본 표1832/1843은 같은 비TAC T&B HorzColumn·Left RowBreak, 양의 사방 균등 여백과 오프셋0을 가진다. `original_hwpx_column_rowbreak_equal_outer_margin_hu → fragment_outer_top_px → raw_top/lane_top → 최종 table bbox`에서 행 개수 조건이 단일 셀을 잘못 제외했다. 수정 전 좌표 회귀는483.16/PDF486.508px로 FAIL(exit100,0.170s)했다.

- 빈/무효 행은 제외하되 단일 셀도 기존 저장 바깥 상자 규칙을 소비한다. 다른 원점·오프셋·비균등 여백의 적용 조건을 이번 근거 없이 확대하지 않는다. partial 조각은 단계7부터 같은 행 개수에 관계없는 상단 여백을 소비하며, 이 단계는 whole 경로를 보정한다. 가로 여백의 일반성만으로 모든 세로 앵커를 바꾸지 않는다.
- 두 원본 표의 실제 상·하단과 기존 #6378 다행 표, #7063 가로 여백, PrEP 및 공통 앵커 대조군64/64 PASS(exit0,2.699s). fresh CLI build exit0. Native176/177은96.81055%/98.67443%, 선택 gate passed(exit0), 글꼴 예외 없음. 두 review 및 standalone overlay를 직접 확인했다. 표 외곽과 본문 위치가 개선됐으며 얇은 괘선·각주 glyph/폭 차이는 남는다.

[단계9 검증](../assets/pr7382_20260926/stage9_validation.json)과 [Native manifest](../assets/pr7382_20260926/stage9_native_run_manifest.json)는7f3b0ee8b+단계9 Rust/test diff의 증거다. 전체216/PDF215로 PR 생성·승인은 계속 보류한다. 다음은 그림67의 원본 저장 종료 사다리를 사용한 원점/flow/guide 공통 결과와 마지막 추가 페이지이며, terminal caption 예산 실패 및 최종 full Rust/lint/Skia/fresh WASM도 남는다.

![단계9 Native176 review](../assets/pr7382_20260926/stage9_native_review_176.png)
![단계9 Native177 review](../assets/pr7382_20260926/stage9_native_review_177.png)


## 메인터너 보정 10: 이월된 그림 표의 닫힌 저장 프레임

그림67 표1904의 높이50256HU와 바깥 위·아래 여백283HU를 합하면50822HU이며, 같은 PS의 빈 guide 사다리 뒤 일반 문단1910의 저장 vpos가 정확히50822로 다시 시작한다. 원본 저장본·한컴 PDF182의 표 상단86.945px, 캡션741.701px, 뒤 매독/기생충 본문787.461/894.021px, PDF183의 그림68이 독립 근거다. HWP 대조군도215쪽이지만 그림67의 원점은 잘못되어 그 출력으로 HWPX 좌표 기대값을 정하지 않았다.

- 수정 전 대상 좌표는123.4533px로 FAIL(exit100, 대상0.186s); 종료 사다리50822→51822HU를 바꾼 반례는 PASS다. 좌표 겹침만으로 guide 소유를 수용하지 않는다.
- `stored_table_frame_with_guides → query_closed_source_frame_placement → whole entry / prepare의 clean-deferral 후 실제 frame → fragment budget → emit commit → layout raw_top/최종 lane flow → section guide`를 연결했다. 미편집·미reflow 저장본, 단일 단의 새 fragment, 문단 상대 양수 offset, 유효한 단일 host/guide 사다리와 닫힌 외곽 상자, 실제 측정 높이와 선언 높이 일치, 실제 예산 fit일 때 같은 원점/점유 끝을 소비한다. guide는 실제 수용된 배치 결과가 존재할 때만 그 상자의 일부로 처리한다.
- 첫 후보는 normal whole entry에만 연결하여19/20 PASS였다. 실제 경로는 whole-fit 실패 후 split prepare에서 새 쪽으로 이월하고 scanner가 전체 행을 수용해 `PageItem::Table`을 방출했다. 이 경로의 prepared placement와 frame도 함께 갱신하여 후속 예산·paint가 옛 anchor를 다시 적용하지 않도록 했다. 실패 후보를 완료 증거로 사용하지 않는다.
- 최종 관련 앵커·바깥여백·빈 host 줄 간격 및 기존 대상 회귀71/71 PASS(exit0,2.650s), fresh CLI build exit0. 원본 전체215/PDF215쪽, 그림67 뒤 본문의 중복·이월 없음과183쪽 그림68을 검사했다. Native182/183의 선택 gate passed(exit0),99.87143%/99.91466%, 글꼴 예외 없음. 네 review/standalone overlay를 직접 확인했으며 그림 외곽·캡션·본문 배치는 맞고 얇은 선/glyph 차이는 남는다.

[단계10 검증](../assets/pr7382_20260926/stage10_validation.json)과 [Native manifest](../assets/pr7382_20260926/stage10_native_run_manifest.json)는9b0e7cece+단계10 Rust/test diff의 증거다. 전체 페이지 수 보류 사유는 해소했으나 캡션 예산 실패 경계, Body 각주30/240, 최종 full Rust/lint/Skia/fresh WASM과 전체 Native를 완료하기 전에는 PR 생성·승인하지 않는다.

![단계10 Native182 review](../assets/pr7382_20260926/stage10_native_review_182.png)
![단계10 Native183 review](../assets/pr7382_20260926/stage10_native_review_183.png)


## 메인터너 보정 11: 캡션 종료 예산과 실제 행 소유의 재스캔

이전 여백/간격3종은 끝 캡션이 새 페이지 예산 안에 들어가 실패 분기를 실행하지 않았다. 양쪽i16 속성의 유효 상한32700HU와 본문 높이20px 감소로 만든 반례에서 수정 전 예약100.7733px / 실제 중간 조각81.48px가 FAIL(exit100,summary1.871s)했다. 끝행만 뒤로 물리는 기존 emit은 `end_row`만 바꾸고 그 행 높이·컷을 그대로 예약했다. 마지막2행을 rowspan 소유 유닛으로 합치고 각주를 제거해 새 페이지에 들어가는 독립 IR 반례도 FAIL했고, 두 반례 실행은0PASS/2FAIL(exit100,1.990s)이었다. 과도한 캡션과 원본6개 각주가 함께 새 페이지에 들어가지 않는 초기 입력 및 작성 오류/컴파일 실패는 결함 검출 증거로 세지 않는다. 이 입력들은 알고리즘 계약이며 한컴 출력의 대용이 아니다.

- `RowBlockQuery의 마지막 소유 유닛 → 실제 caption/terminal margin 예산 → 동일 scan_block_table_split_rows의 prefix 재스캔 → consumed/end_row/양쪽 컷/override 일괄 반영 → emit/실제 bbox`를 연결했다. 마지막 행 숫자만 감소시키는 emit 분기를 제거했다. 앞 유닛이 없고 현재 페이지에 앞선 항목이 있으며 전체 유닛이 새 페이지에 들어가면, cursor·각주·내용을 소비하지 않고 다음 frame에서 재시도한다. 저장 양수 원점만 있는 빈 frame을 반복 이월하지 않는다. 새 페이지에도 들어가지 않는 객체의 기존 진행 fallback을 이번 검증의 해결 주장으로 바꾸지 않는다.
- 단일 마지막 행의 통째 진입은 선언19.5px만 믿고 캡션을 빠뜨리던 우회였다(수정 전 FAIL,summary0.481s; table top796.92/body83.16px). forward 문단 Top/Inside·객체전용 저장 앵커의 캡션 상자를 공통 배치 계획으로 만들고 whole fit, clean defer 이후 scanner, whole/partial paint가 같은 원점과 occupied bottom을 소비한다. 중간 조각은 중간 여백만, 마지막 조각은 종료 여백을 예약한다. 초기 공유 후보는42개 중4개 FAIL했으며, 중간 조각에 끝 여백을 미리 차감/등록한 오류를 같은 범위에서 수정했다.
- 위 캡션은 첫 예산에서 이미 차감한다. 첫 행 강제 수용이0px 예산을 우회한 새 반례가 FAIL(exit100,0.658s)했고, 동일 종료 검사에서 위 캡션을 이중 차감하지 않고 새 frame으로 이월하도록 고쳤다. 양수2250HU 오프셋 반례도86.9333/기대116.9333px로 FAIL(2PASS/1FAIL,exit100,0.690s)했다. 첫 유닛이 통째로 이월돼도 공통 계획을 다시 질의해 오프셋을 보존하며, 진짜 continuation이 소비한 앵커와 구분한다. 기존 `para_offset_consumed_by_page_break` 계약도 같이 소비한다. 절대 기준·중앙/하단 정렬·후향 앵커는 이 forward 문단 원점으로 해석할 수 없으므로 기존 위치 해석의 비대상 경로다. 해당 조합 전부의 한컴 출력 검증을 주장하지 않는다.
- 정식 반례는 모든 행을 한 번씩, 끝 rowspan을 함께, 캡션을 한 번, 실제 예약/paint 끝점을 확인한다. 단일 행의 위/아래 캡션 및30px 오프셋은 뒤 본문의 소유와 같은 페이지에서의 종료 후 비충돌도 검사한다. 별도로 기존 nested no-caption3개가 보정2의 비대상 인덱싱으로 panic한 것을 발견했다. HWPX top-level 캡션 여백 조건을 통과한 뒤에만 source paragraph를 읽도록 수정해 정상 nested 경로를 재검증했다.
- 최종 집중/정상 **44/44 PASS(exit0,3.597s,threads8)**. source7379 전체24개와 #6756/#6803 rowspan 컷, #7288 원자 행, #6024 continuation, #6837 nested row, #6599 nested caption, #5136/#6284 caption, #7390 PrEP와 #1937을 포함한다. fresh CLI build exit0(2m20s), 전체 **215/PDF215쪽**. Native66/67은 **98.24785% / 93.58939%**, 선택 gate passed(exit0), 글꼴 예외 없음. 두 review와 standalone overlay를 새 코드에서 직접 판독해 행·각주77 소유, 표 원점·캡션·뒤 본문 위치를 확인했다. 얇은 괘선·glyph/URL 폭·링크 색 차이는 남으며 완전 픽셀 일치로 보고하지 않는다.

동일 원본 HWPX의 저장 제품13.0.0.3901을 확인하고 engine2024/timeout1800으로 다시 변환했다. 새 PDF도215쪽이며 **215쪽 전체의 텍스트 줄/좌표가 기존 PDF와 정확히 같았다**.10쪽 BMP 배경224/235/255와 한컴 PDF JPEG234/242/255의 색 차이도 재현됐다. 변환 색상 원인은 미확정이며 renderer gamma나90% gate를 바꾸지 않았다. 기존 기준 PDF와 실패 증거를 유지한다. [같은 입력 PDF 좌표 대조](../assets/pr7382_20260926/stage11_same_input_pdf_geometry.json)는 재변환 hash/engine/job과 전쪽 좌표 결과를 남긴다.

[단계11 검증](../assets/pr7382_20260926/stage11_validation.json), [Native manifest](../assets/pr7382_20260926/stage11_native_manifest.json), [run manifest](../assets/pr7382_20260926/stage11_native_run_manifest.json)는 `015985403`+최종 단계11 Rust/test diff의 증거다. 다음 보정 전에 이 결과를 별도 커밋한다. 전체 Native 진단에서31/32·108·121 등 본문 tail과 각주 소유가 다른 경계를 확인했으며, Body각주30/240 및 그 외 낮은 페이지를 하나씩 처리한다. 최종 전체 회귀·Rust lint·정책·Skia·fresh WASM과 전체 시각 gate는 아직 완료하지 않았으며 PR을 만들지 않는다.

![단계11 Native66 review](../assets/pr7382_20260926/stage11_native_review_066.png)
![단계11 Native67 review](../assets/pr7382_20260926/stage11_native_review_067.png)


## 메인터너 보정 12: 본문 각주의 반복 페이지 시작 보존

### 사전 분석과 실제 소비 경로

원본 본문1865의 각주240은 `0/0/1172`를 저장한다. 독립 한컴2024 PDF178에는 번호와 첫 줄,179에는 `HTLV-1` 및 출처 꼬리 두 줄과 뒤 각주241/242가 있다. [원본 줄과 PDF 좌표](../assets/pr7382_20260926/stage12_independent_geometry.json)로 기대값을 고정했다. 파서가 두 번째0을1172로 덮고 Body 등록이 HWP5 경로에서만 저장 각주 reset을 소비하는 두 원인을 확인했다.

`section.rs::normalize_hwpx_note_line_vpos` → 동일 stored/composed 줄 대응을 확인하는 `native_hwp5_footnote_reset_fragments` → `body.rs::register_body_footnote`의 현재 prefix 예약/물리 page 전환/다음 suffix 예약 → 실제 `FootnoteArea` 배치로 연결한다. 편집하지 않은 HWPX도 표와 같은 저장 각주 경계를 사용한다. 합성 줄·저장 줄 부재는 physical split의 근거로 쓰지 않는다. 양수로 시작한2344/0의 연속줄 복원과 미주는 기존 계약을 유지한다. 본문 자체의 저장 reset과 충돌 경계는 이번 단계에서 변경하지 않았다.

### 수정 전후 실행과 판정

검증 producer는 `ec8a37a84` + Rust/test diff SHA256 `f3f0d1dc8da845d278f436bec045c5202fbdca8862abe638799ad5227a2544d9`다. [실행 증거](../assets/pr7382_20260926/stage12_validation.json)에 연결한다.

| 검사 | 결과 | 판정 |
| --- | --- | --- |
| 원본 parser/실제 각주 소유 수정 전 | 2FAIL, exit100,0.247s | `[0,1172,1172]`와178쪽 꼬리 조기 소비를 실제 검출 |
| 관련 원본·대조군 수정 후 | 35PASS, exit0,4.820s | 기존 표/각주·HWP5 왕복·미주·정규화 및 합성/저장줄 부재 반례 통과 |
| 새 CLI | build exit0,2m11s; 215/PDF215 | 중간 후보의 쪽수 계약 충족 |
| Native178/179 직접 비교 | 94.31041% /97.53225% | 번호/앞줄178, 꼬리179, 뒤 본문·각주 보존. glyph폭·URL색·일부 양쪽정렬 차이가 남아 픽셀 완전 일치를 주장하지 않음 |
| Native31/32 직접 비교 | 45.37615% /34.68853%, sweep 전체 exit1 | 각주30의 prefix/tail은 개선했으나 앞 본문407/421 소유·표/그림 원점 및32쪽 각주 구분선 차이가 남아 보류 |
| 전체 중간 Native audit | 215개 완료,50개90%미만, exit1 | [보정10 중간 보류 목록](../assets/pr7382_20260926/stage10_full_native_hold_inventory.json). 최종 head acceptance가 아님 |

추가 반례의 첫 작성에서 저장 줄 하나를 제거하면 composer 줄 수도 함께 줄어 실제 count mismatch가 아니었다(34PASS/1FAIL). 이를 회귀 검출로 세지 않고 저장 줄 부재 반례로 정정하여 재실행했다. 일반 저장/구성 줄 수 불일치 guard 자체를 이번 반례로 검증했다고 확대하지 않는다.

[Native manifest](../assets/pr7382_20260926/stage12_native_manifest.json)·[summary](../assets/pr7382_20260926/stage12_native_summary.json)·[metrics](../assets/pr7382_20260926/stage12_native_overlay_metrics.json). review178/179 및 standalone overlay178/179, review31/32를 직접 판독했다. 전체/fresh WASM/lint/최종 필수 검증은 아직 남았으며 PR 생성·승인은 보류다.

![Native178 review](../assets/pr7382_20260926/stage12_native_review_178.png)
![Native178 overlay](../assets/pr7382_20260926/stage12_native_overlay_178.png)
![Native179 review](../assets/pr7382_20260926/stage12_native_review_179.png)
![Native179 overlay](../assets/pr7382_20260926/stage12_native_overlay_179.png)
![남은 본문 소유31](../assets/pr7382_20260926/stage12_native_review_031.png)
![남은 본문 소유32](../assets/pr7382_20260926/stage12_native_review_032.png)


## 메인터너 보정 13: 실제 각주 예약과 본문 저장 경계 공유

### 사전 분석·소비 경로

보정12 뒤에도 본문407/421은 통째 항목으로 남아 저장 reset 뒤 줄을 앞쪽에 소비했다. [독립 PDF 줄 좌표](../assets/pr7382_20260926/stage13_independent_geometry.json)는407 꼬리 두 줄이31쪽83.141/109.861px,421 꼬리가32쪽83.141px에 있음을 입증한다. 단순 writer 좌표 되감김을 모두 물리 경계로 취급하지 않고, 미편집 단일 단 HWPX에 기존 nonsynthetic 경계/marker/실제 각주 예약 query를 연결한다.

`boundary.rs::native_hwp5_first_footnote_overlap_break_line`의 source 줄/marker·실제 FootnoteArea 투영 → `section/flow.rs`의 marker 등록 route 및 `paragraph.rs`의 강제 경계 선택 → 실제 `PartialParagraph` 컷 → `native_hwp5_body_footnote_tail_reset` 및 `body.rs`의 완료 prefix/현재 tail 등록 → 실제 본문과 뒤 표/그림 배치. 저장 각주 자체도 같은 경계를 입증한 경우, 기존 각주가 없는 marker 쪽이어도 첫 각주 collision route가 입증한 완료 prefix owner를 사용할 수 있게 했다. 기존 multi-note/native 및 일반 reset 경로를 일괄 확장하지 않았다.

### 실행 결과

Producer `4001f5b0c` + Rust/test diff SHA256 `ccca0de8ec46961ba9473f98e3e756a7cc25cd4dfbc9273730d285a5832205de`; [실행·실제 컷 증거](../assets/pr7382_20260926/stage13_validation.json).

| 검사 | 결과 | 의미 |
| --- | --- | --- |
| 수정 전 원본 본문2개 | 2FAIL, exit100,0.162s | 두 꼬리가 앞쪽에서 소비되는 의도한 원인으로 실패 |
| 수정 후 원본·대조군 | 38PASS, exit0,3.437s | 두 꼬리 좌표·앞 각주 번호·뒤 소유 보존 및 합성 되감김 비적용 |
| 실제 dump cuts | 407:30쪽0..3/31쪽3..5;421:31쪽0..4/32쪽4..5 | 저장 경계 뒤 내용의 누락/중복 없는 실제 항목 보존 |
| 새 CLI/쪽수 | build exit0,2m14s;215/PDF215 | 중간 쪽수 계약 충족 |
| Native30/31 직접 review/overlay | 96.40652% /97.33830% | 앞 본문 꼬리·제목·표 원점 복원. 글꼴/그림색/얇은 괘선 차이는 잔여 |
| Native32 직접 review/overlay | 81.60723%, sweep exit1 | 꼬리와 그림/표 원점은 개선; 뒤 문단 높이·이월 각주 구분선 차이가 남아 보류 |
| Native178/179 재캡처 | 94.31041% /97.53225% | 앞 단계 각주240 무회귀; 이 단계에서 직접 재판독을 반복했다고 확대하지 않음 |

첫 pre-run의421 문구는 제가 옮긴 `35%`가 원문 `<그림35>`와 달랐다. 작성 오류를 결함 증거로 세지 않고 동일 수정 전 코드에서 실제 뒤 문구로 정정하여 실패를 다시 확인했다.

[manifest](../assets/pr7382_20260926/stage13_native_manifest.json)·[summary](../assets/pr7382_20260926/stage13_native_summary.json)·[metrics](../assets/pr7382_20260926/stage13_native_overlay_metrics.json). 30/31/32의 review와 standalone overlay6개를 직접 판독했다.32쪽 구분선은 같은 보고서 HWP·HWPX 기준 PDF 모두 y1018.725px에서 확인되며 별도 후속 보정 대상으로 남긴다. 전체 최종/fresh WASM/lint는 미완료여서 PR 생성·승인 보류다.

![Native30 review](../assets/pr7382_20260926/stage13_native_review_030.png)
![Native30 overlay](../assets/pr7382_20260926/stage13_native_overlay_030.png)
![Native31 review](../assets/pr7382_20260926/stage13_native_review_031.png)
![Native31 overlay](../assets/pr7382_20260926/stage13_native_overlay_031.png)
![Native32 남은 차이](../assets/pr7382_20260926/stage13_native_review_032.png)
![Native32 overlay](../assets/pr7382_20260926/stage13_native_overlay_032.png)
