---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-09-26
---

# PR #7382 리뷰 — 분할 표와 저장 각주 경계

## 현재 판정

**머지 보류.** 원 변경은 기준 PDF의 행·각주 소유와 전체 페이지 수를 충족하지 않는다. 보정14 통합 후보는215쪽을 유지하고 선택66/67·30/31·178/179쪽을 개선했지만, 전체 시각 비교의 본문 소유 차이와 최종 필수 게이트가 남았다. 통합 PR 생성·승인을 완료했다고 보고하지 않는다.

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


## 사용자 요청: 추가한 영어 주석 전체 한글화

사전 범위는 이번 통합 branch의 base 대비 추가 설명 주석 전체다. 코드·회귀 검사20개 파일의 영어108줄을 한글로 바꿨다. 변경 전후 주석을 제외한 파일 내용이 바이트 단위로 같음을 검사했고, base 대비 추가 영어 설명 주석 잔여0개, `cargo fmt --all -- --check`·`git diff --check` exit0을 확인했다. [파일별 검증](../assets/pr7382_20260926/comment_translation_validation.json). 이는 설명 주석 변경이며 새 렌더링 검증 통과를 주장하지 않는다. 최종 head의 필수 검증은 후속 기능 보정 뒤 실행한다.


## 메인터너 보정 14: 이월 각주의 물리 쪽 구분선 보존

같은 원본 보고서의 HWP/HWPX 한컴2024 PDF32는 번호 없는30 꼬리 위에 `x94.509..283.528/y1018.725px` 구분선을 표시한다. [두 기준 출력의 선 좌표와 해시](../assets/pr7382_20260926/stage14_independent_geometry.json)로 독립 기대값을 정했다. 기존 조각 조회와 문서가 구분선·번호를 모두 첫 조각에만 표시한다고 가정한 원인을 수정했다.

`boundary.rs`의 두 줄/저장 reset 조각 플래그 생산 → `state/notes.rs`의 실제 예약·투영·영역 동기화 → `picture_footnote.rs`의 동일 `any(draw_separator)` 높이와 선 배치로 이어진다. 각 물리 쪽에서 구분선은 한 번만 예약·배치하며 번호는 앞 조각만 표시한다. 명시적으로 구분선을 생략하는 기존 플래그의 소비 계약은 유지하고, 실제 물리 각주 경계 query가 올바른 표시 값을 생산하게 했다. 본문 위치와 그림색 차이를 이 변경으로 해결했다고 확대하지 않는다.

Producer `477c9aeb8` + Rust/test diff SHA256 `3d707ab6f3dd52af178be2ab424c21a52c7e65d8ac71df1148058479b57662f7`; [전후 검사](../assets/pr7382_20260926/stage14_validation.json).

| 검사 | 결과 | 의미 |
| --- | --- | --- |
| 같은 HWP/HWPX 실제 꼬리, 수정 전 | 2FAIL, exit100,0.247s |32쪽 구분선 수0/기대1로 실제 누락 검출 |
| 수정 후 관련·정상 대조군 | 40PASS, exit0,3.443s | 구분선 좌표·꼬리 위치·번호 무반복, 꼬리+정상 각주에서 선1개, 기존 표/각주/미주/왕복 보존 |
| 새 CLI/쪽수 | build exit0,1m46s;215/PDF215 | 중간 쪽수 유지 |
| Native32 review/overlay 직접 판독 |81.85013%, sweep 전체 exit1 | 구분선 복원; 뒤 문단 위치·그림색 등 잔여로 보류 |
| Native179 review/overlay 직접 판독 |97.53225% | 꼬리와 뒤 정상 각주의 구분선 중복 없음, 앞 단계 무회귀 |
| Native31/178 재캡처 |97.33830% /94.31041% | 선택 지표 무회귀; 이 단계에서 직접 판독을 반복했다고 확대하지 않음 |

[manifest](../assets/pr7382_20260926/stage14_native_manifest.json)·[summary](../assets/pr7382_20260926/stage14_native_summary.json)·[metrics](../assets/pr7382_20260926/stage14_native_overlay_metrics.json). 전체/fresh WASM/최종 필수 검증은 미완료이며 PR 생성·승인 보류다. 추가·수정한 설명 주석은 한글로 작성했다.

![Native32 구분선 복원](../assets/pr7382_20260926/stage14_native_review_032.png)
![Native32 overlay](../assets/pr7382_20260926/stage14_native_overlay_032.png)
![Native179 정상 대조](../assets/pr7382_20260926/stage14_native_review_179.png)
![Native179 overlay](../assets/pr7382_20260926/stage14_native_overlay_179.png)


## 메인터너 보정 15: 실제 본문 앞 빈 줄의 점유 보존

동일 입력 PDF32의 뒤 본문 세 줄은821.061/847.621/874.341px다. 원본 빈 문단426의 저장53340→다음 본문55340HU는1000HU 줄높이+1000HU 간격,26.666px 점유를 입증한다. [독립 좌표](../assets/pr7382_20260926/stage15_independent_geometry.json). 측정과 줄 메트릭 생산은 정상이었으며, 구역 꼬리 흡수 경로가 다음 본문을 제목으로 추정해 그 뒤 문단의 쪽 경계를 앞당겨 적용한 것이 원인이었다.

`absorb_section_tail`의 경계 판단 → `hidden_empty_paras` 및 높이0 항목 → `layout.rs`의 숨김 반환 → 뒤 본문 최종 원점을 추적했다. 글이 있는 문단을 건너뛰는 잘못된 가정을 제거하고 빈 문단들 바로 뒤 첫 본문에서 관측한 reset만 안내 줄 흡수 근거로 유지했다. 문서 ID/숫자 조건을 추가하거나 줄 높이를 임의 보정하지 않았다.

Producer `00e210b06` + Rust/test diff SHA256 `bc7038e0bb950733809b6a8f63109dce27fd792f847fbec299a4c1bfc80ff9d8`; [명령·결과·소스 해시](../assets/pr7382_20260926/stage15_validation.json).

| 검사 | 결과 | 의미 |
| --- | --- | --- |
| 수정 전 실제 본문 좌표 |1FAIL, exit100,0.173s |807.693px vs PDF821.061px, 의도한13.333px 앞당김 검출 |
| 최종 소스 관련·정상 대조군 |43PASS, exit0,3.912s | 세 줄의 실제 위치, 앞선 표/각주/원본215쪽 왕복 및 국제고속선 HWP/HWPX242쪽·원본 reset 소유 보존 |
| 페이지 끝 빈 줄·표 조각 unit |4PASS, exit0,0.012s | 실제 끝 안내 줄 흡수와 prepared-state 생명주기 유지 |
| unit 정책·fmt |exit0 | 정책은 고정 base `eb9142dd7` 비교 |
| CLI·쪽수 |build exit0,1m56s;215/PDF215 | 최신 소스 출력,33쪽 첫 본문428 보존 |
| Native31/32/178/179 |97.33830/94.52341/94.31041/97.53225%, exit0 | 네 쪽 gate 통과, 글꼴 예외 없음 |

빈 줄 unit 첫 실행은 앞 보정에서 바꾼 필드의 옛 이름이 기존 테스트 초기화에 남아 테스트 전 빌드 실패(exit101)했다. 내 누락으로 기록하고 같은0.0 값의 올바른 필드로 고쳤다. 이를 빈 줄 결함 검출 증거로 세지 않으며 수정 후 unit4개와 최종 관련43개를 다시 통과시켰다. 추가·수정 설명 주석은 한글이며 통합 branch의 추가 영어 설명 주석 잔여0개다.

네 쪽의 review·standalone overlay8개를 직접 판독했다. 32쪽은 뒤 본문 위치가 복원되고 구분선·꼬리 각주가 남으며31/178/179의 기존 배치도 유지된다. 그림색·얇은 선·일부 글리프 잔차를 완전 일치로 보고하지 않는다. [manifest](../assets/pr7382_20260926/stage15_native_manifest.json)·[summary](../assets/pr7382_20260926/stage15_native_summary.json)·[metrics](../assets/pr7382_20260926/stage15_native_overlay_metrics.json). **선택 네 쪽 통과이며 전체/fresh WASM/최종 필수 검증 미완료, 다른 보류 페이지 해결 전 PR 생성 보류**다.

![Native32 빈 줄 뒤 본문 복원](../assets/pr7382_20260926/stage15_native_review_032.png)
![Native32 overlay](../assets/pr7382_20260926/stage15_native_overlay_032.png)


## 메인터너 보정 16: 저장 되감김 표의 온전한 행과 바깥 상자 공유

독립 한컴 PDF106의 표29는0..2행,107은3..7행이다. 앞 표 괘선670.947..986.121px, 뒤 표85.027..517.833px, 끝 캡션529.714px 및 뒤 본문592.901px를 [같은 원본 HWP/HWPX 두 출력](../assets/pr7382_20260926/stage16_independent_geometry.json)에서 확인했다. 내용 컷 높이가 실제 온전한 행보다 행마다11.733px 작게 예약돼 뒤 행까지 수용한 원인이다.

기존 HWP 계약과 같은 원본 되감김·일반 행·행 병합/셀 각주 없음 형상의 미편집 단단 HWPX에도 실제 온전한 행 높이를 연결했다. `whole_fit` 생산 → `prepare` 행 높이 → `RowBlockQuery` fit/소비 → 이어받기 plan → `budget/emit` → `table_partial` 실제 원점·점유 끝이 같은 결과를 사용한다. 새 물리 프레임에서 시작하는 온전한 이어받기 행은 바깥 위·아래 여백도 같은 plan으로 다시 연다. 행 내부 컷과 쪽 중간 원점은 이미 소유한 좌표를 유지한다. 문서 ID/새 수치 특례나 golden 완화를 추가하지 않았다. 기존 footer-local4px 값은 변경하지 않았다.

Producer `415b4b046` + Rust/test diff SHA256 `2ca3b8cc2b49496dd93d46d04e808aa35176f28dc6c9b422b6b783f4dd21dead`; [명령·결과·소스 해시](../assets/pr7382_20260926/stage16_validation.json).

| 검사 | 결과 | 의미 |
| --- | --- | --- |
| 수정 전 실제 두 회귀 |2FAIL, exit100,0.171s | 앞 행0..4/기대0..2,38.8px 표 초과 및 뒤 본문 조기 소비 |
| 첫 후보 |61PASS/1FAIL, exit100,3.206s | 행 소유와 꼬리 개선, 캡션1.88px·뒤 본문3.76px 차이 검출; 허용치 유지 후 공통 상자 보정 |
| 최종 관련·정상 대조군 |66PASS, exit0,4.009s | 행·괘선·캡션·뒤 본문·108꼬리 실제 검사, 패딩/원본 프레임/행 병합/중첩/원자 행과 앞 보정 무회귀 |
| 기존 Native·경계 unit |6PASS, exit0,0.227s | 원본 HWP 표 행 경계/그림 캡션, 빈 줄·조각 생명주기 보존 |
| unit 정책·fmt |exit0 | 고정 base `eb9142dd7` 비교 |
| CLI·쪽수 |build exit0,2m04s;215/PDF215 | 앞 표[0,3), 뒤[3,8), 본문1144[0,3)/[3,6) |
| Native106/107/108 |80.27049/85.40785/99.94123%, exit1 |108 개선,106/107 셀 글줄 잔여로 보류 |
| Native94/95 재캡처 수치 |98.91562/94.55592% | 같은 행 계약의 추가 수치; 이 단계에서 직접 판독했다고 확대하지 않음 |

첫 after 빌드는 계약명 변경 중 전달 구조체 한 곳을 빠뜨려 테스트 전 exit101로 실패했다. 내 오류를 바로잡고 전체 참조를 확인했으며 결함 검출 증거로 세지 않았다. 최종 source의6unit/66integration을 실행했다. 편집/텍스트 재조판 제외의 실제 편집 counter는 이번 단계에서 직접 실행하지 않았으며, 코드 보호 조건과 정상 대조군 통과를 실제 편집 검증으로 확대하지 않는다. 추가 설명 주석은 한글이고 추가 영어 설명 주석 잔여0개다.

106/107/108의 review·standalone overlay6개를 직접 판독했다. 표 외곽·캡션·뒤 본문 소유는 개선됐으나106/107 셀 안 글줄이 기준과 다른 위치에 있어 **gate는 보류**다. 글꼴 예외로 분류하지 않고 후속 셀 배치 보정에서 확인한다. [manifest](../assets/pr7382_20260926/stage16_native_manifest.json)·[summary](../assets/pr7382_20260926/stage16_native_summary.json)·[metrics](../assets/pr7382_20260926/stage16_native_overlay_metrics.json). 전체/fresh WASM/최종 필수 검증 미완료이며 PR 생성 보류다.

![Native106 남은 셀 글줄 차이](../assets/pr7382_20260926/stage16_native_review_106.png)
![Native107 남은 셀 글줄 차이](../assets/pr7382_20260926/stage16_native_review_107.png)
![Native108 본문 꼬리와 그림 복원](../assets/pr7382_20260926/stage16_native_review_108.png)
![Native108 overlay](../assets/pr7382_20260926/stage16_native_overlay_108.png)


## 메인터너 보정 17: 온전한 행의 실제 높이와 안 여백 공유

원본 표29의 셀 최소 높이는282HU지만 실제 첫 행은90.933px이며, 저장 안 여백은 위·아래 각각510HU(6.8px)다. 기존 배치가 최소 셀 높이로 다시 판단해 위 여백을0.933px로 축소했고, 첫 조각에서 예산에 예약한141HU 위 여백도 배치 원점에 전달되지 않았다. [동일 입력/PDF의 독립 글줄 좌표](../assets/pr7382_20260926/stage17_independent_geometry.json)는106쪽678.514px,107쪽91.794px다.

온전한 단일 행의 측정 높이 생산 → `table_partial` 실제 셀 상자 높이 → 공통 `resolve_cell_padding_at_height`의 축 선택·이상값 방어 → 실제 글줄 원점으로 연결했다. 행 내부 컷·높이 덮어쓰기·병합 셀 및 측정 높이를 사용하지 않는 중첩 경로는 기존 컷 계약을 유지한다. 첫 되감김 조각은 기존 `host_before_overhead + vert_offset_overhead`를 공통 배치 plan에 전달하고, 예산·확정·배치가 같은 원점과 아래 여백을 소비한다. 새 수치 특례나 허용치 완화는 없다. 이전 #7406의 관련 영어 설명 주석9줄도 한글로 변경했다.

Producer `bacbabc9e` + Rust/test diff SHA256 `d1abcad092e275a6d3bce3c8f8da9f05056f9e379f474feae37dd7c55bbc6627`; [실행·소스·잔여 범위](../assets/pr7382_20260926/stage17_validation.json).

| 검사 | 결과 | 의미 |
| --- | --- | --- |
| 수정 전 실제 좌표 회귀 |1FAIL, exit100,0.166s |106쪽 글줄670.767/기대678.514로 조기 배치 검출 |
| 수정 후 관련·정상 대조군 |76PASS, exit0,3.319s | 실제 셀 안 여백·독립 글줄 좌표, 조밀 표/음수·쓰레기 여백/중첩/병합/행 내부 컷 보존 |
| 라이브러리 대조군 |6PASS, exit0,0.225s | Native 저장 되감김 표·그림 캡션·빈 문단·조각 생명주기 보존 |
| CLI·쪽수 |build exit0,1m57s;215/PDF215 | 필수 페이지 수 일치 |
| Native106/107 |99.94111/99.02564% | 기존80.27049/85.40785%에서 개선, 셀 글줄과 표 경계를 함께 직접 판독 |
| Native94/95/108 |98.91562/94.55592/99.94123% | 선택 gate exit0, 글꼴 예외 없음; 모든 review/overlay 직접 판독 |
| fmt·추가 설명 주석 |exit0 / 영어0개 | 이 단계에서 추가·수정한 설명 주석은 한글 |

[manifest](../assets/pr7382_20260926/stage17_native_manifest.json)·[summary](../assets/pr7382_20260926/stage17_native_summary.json)·[metrics](../assets/pr7382_20260926/stage17_native_overlay_metrics.json). 선택5쪽의 review/standalone overlay10개를 직접 확인했다. 표 행·캡션·뒤 본문과108쪽 꼬리/그림의 누락·겹침은 보이지 않는다. 일부 글자 실루엣·획·간격과 그림 색은 남아 있으므로 완전 픽셀 일치를 주장하지 않는다. 전체 최종/fresh WASM/lint는 미완료여서 PR 생성·승인 보류다.

![Native106 셀 안 여백 복원](../assets/pr7382_20260926/stage17_native_review_106.png)
![Native106 overlay](../assets/pr7382_20260926/stage17_native_overlay_106.png)
![Native107 이어받기 셀](../assets/pr7382_20260926/stage17_native_review_107.png)
![Native107 overlay](../assets/pr7382_20260926/stage17_native_overlay_107.png)
![Native108 후속 내용](../assets/pr7382_20260926/stage17_native_review_108.png)
![Native108 overlay](../assets/pr7382_20260926/stage17_native_overlay_108.png)


## 사용자 요청 추가 반영: 이전 통합의 영어 설명 주석 한글화

이번 통합의108줄뿐 아니라 앞 #7406/#7366 통합이 시작된 `c80a8370a` 이후 추가 설명 주석도 다시 확인했다. 보정17에서 번역한 모델 설명9줄 외에13개 파일의21문단86줄을 한글로 바꿨다. [파일별 검증](../assets/pr7382_20260926/prior_comment_translation_validation.json)에서 주석을 제외한 파일 바이트가 전후 동일하며, 이 시작 base부터 현재 branch까지 추가 영어 설명 주석은0개다. 식별자와 코드 울타리 언어 표시는 유지했다. 주석만 바뀐 범위는 새 기능 검증 통과로 확대하지 않으며, 최종 기능 head의 필수 검증은 계속 진행한다.


## 메인터너 보정 18: 저장 앵커로 입증한 낮은 시작 본문 경계

원본 문단512의44000/46000/48000/0,516의62711..70711/0은 [한컴 PDF44/45의 첫 꼬리](../assets/pr7382_20260926/stage18_independent_geometry.json)와 대응한다. 문단512는 본문 높이의 약61%에서 시작해 기존70% 후보 조건에 막혔지만, 실제 저장 앵커와 현재 흐름은 일치했다. 이 비율이 물리 쪽 소유의 증거를 대신한 것이 원인이다.

`prepare_forced_page_boundary`는 HWPX의 소스·구성 줄 수 일치와 기존 세션 편집 플래그를 확인하고, 공통 `hwpx_saved_reset_fragment_matches_current_flow`에서 단단/일반 본문/비합성 저장 줄/단조 앞 조각/정확한0 reset/현재 앵커 일치를 입증한 경계를 생산한다. 이 결과를 기존 forced boundary → scan/whole-fit/split → 확정·실제 글줄 배치가 소비한다. Native HWP/HWP3의 기존 비율 조건을 광역 완화하지 않았다. 무효 앵커와 합성 줄은 새 경계로 승격하지 않는다.

세션 편집 플래그는 Native HWP5용이므로 HWPX 전역 편집 제외의 증거로 확대하지 않는다. 실제 HWPX 본문 편집은 원래 줄 태그를 보존하지만 내부0 reset을 연속 위치로 재조판한다. 처음에는 모든 줄이 구현 태그라고 잘못 가정해 내 대조 assertion이 실패했고, 실제 생성 계약과 꼬리 무중복으로 바로잡아 통과했다. 새 경계 결함 검출로 세지 않으며 편집 후 한컴 출력 일치로도 확대하지 않는다.

Producer `1216114cb` + 최종 Rust/test diff SHA256 `93502ea7a03e1dd4a3dbf9f6052074cfa2f17da90cb602818a914fd9d54f967c`; [실행·명령·주석 외 코드 동일성·잔여](../assets/pr7382_20260926/stage18_validation.json).87개 검사 뒤 함수의 설명 두 줄만 수정했으며, 당시 실제 diff로 소스를 복원해 현재 함수의 주석 외 바이트가 같음을 확인했다. 별도 추가한 실제 편집 대조군은 최종 코드에서 실행했다.

| 검사 | 결과 | 의미 |
| --- | --- | --- |
| 수정 전 실제 꼬리 회귀 |1FAIL, exit100,0.167s |44쪽 첫 꼬리 누락 검출 |
| 관련·정상 대조군 |87PASS, exit0,12.902s |44/45 실제 꼬리·뒤 표 원점, 합성/불일치 앵커 거절, #6761의315쪽/부분 되감김, Native/HWP3/각주/표 보존 |
| 실제 HWPX 편집 대조 |1PASS, exit0,0.215s | 연속 재조판 위치, 원본 컷 무재사용과 꼬리 무중복 |
| 라이브러리 대조군 |6PASS, exit0,0.196s | 저장 표·그림 캡션·빈 문단·조각 생명주기 |
| CLI·쪽수 |build exit0,1m49s;215/PDF215 | 페이지 수 일치 |
| Native44/45 |71.92862/98.61863% |55.45553/50.97961%에서 개선.44 표 위치는 보류 |
| Native43/46/106/107 |91.32182/97.24002/99.94111/99.02564% | 직접 판독, 앞 보정의 큰 배치 무회귀 |
| Native121/122 |98.11430/98.01917% | 본문 개선. 직접 판독에서 각주160 소유가 여전히 틀림을 확인하여 의미 검증 보류 |
| fmt·영어 주석 |exit0 / 추가 설명0개 | 한글 설명 준수 |

선택8쪽의 review/standalone overlay16개를 직접 확인했다. 자동 선택 gate는44쪽으로 `re_review_required`, sweep exit1이며 글꼴 예외는 없다.121의 각주는 PDF159/160,122는161인데 현재160이122로 이월된다. 자동98%도 이 소유 결함을 해소하지 않는다.43 각주의 일부 줄바꿈/간격 차이도 남는다.44 표 위치와 각주 소유는 다음 개별 보정 대상으로 분리하고, 전체 최종/fresh WASM/lint 미완료로 PR 생성·승인 보류를 유지한다.

[manifest](../assets/pr7382_20260926/stage18_native_manifest.json)·[summary](../assets/pr7382_20260926/stage18_native_summary.json)·[metrics](../assets/pr7382_20260926/stage18_native_overlay_metrics.json).

![Native44 수정 전](../assets/pr7382_20260926/stage18_before_native_review_044.png)
![Native44 본문 복원 및 표 잔여](../assets/pr7382_20260926/stage18_native_review_044.png)
![Native44 overlay](../assets/pr7382_20260926/stage18_native_overlay_044.png)
![Native45 수정 전](../assets/pr7382_20260926/stage18_before_native_review_045.png)
![Native45 꼬리 및 뒤 표 복원](../assets/pr7382_20260926/stage18_native_review_045.png)
![Native45 overlay](../assets/pr7382_20260926/stage18_native_overlay_045.png)
![Native121 각주160 잔여](../assets/pr7382_20260926/stage18_native_review_121.png)
![Native122 잘못 이월된 각주160](../assets/pr7382_20260926/stage18_native_review_122.png)


## 메인터너 보정 19: 빈 호스트 형제 표의 바깥 상자 공유

문단515의 두 ParaTop/TopAndBottom 표에는 각각283HU 바깥 여백이 있다. [동일 원본 PDF44의 괘선·캡션 좌표](../assets/pr7382_20260926/stage19_independent_geometry.json)로 기대값을 고정했다. `host_spacing::resolve`는 이 여백을 계산하지만 `empty_float::prepare`가 위여백을 쓰지 않고, 아래여백도 마지막 fit 면제와 함께 예약에서 빠뜨렸다. 첫 표와 둘째 표의 위치 차이가 누적된 원인이다. 음수 저장 offset을 화면에 맞춘 수치로 덮어쓰지 않았다.

HWPX 빈 호스트의 복수 자리차지 표 중 문단 상단/안쪽 정렬 경로에서 수평 lane 충돌 결과와 host spacing으로 `ParagraphFloatPlacement.table_top/occupied_bottom`을 생산한다. fit은 마지막 아래여백 면제를 유지하되, lane과 다음 표는 실제 점유 하단을 사용한다. `commit_empty_float_table`가 계획을 column metadata에 남기고 layout의 표 원점·실제 paint·후속 흐름은 그 동일 결과를 소비한다. 기존 단일 저장 앵커, Square 형제, 가시 호스트, 가운데·아래 정렬과 Native 경로의 계약은 유지한다. 다른 문서에 맞춘 수치·허용치 변경은 없다.

Producer `369b17e2b` + Rust/test diff SHA256 `5720b4e171e66ca36855cf155f5198a4623e7da629f389eff106db8e3affb04b`; [명령·정확한 전후 결과·잔여](../assets/pr7382_20260926/stage19_validation.json).

| 검사 | 결과 | 의미 |
| --- | --- | --- |
| 수정 전 실제 괘선·캡션 회귀 |1FAIL, exit100,0.179s | 첫 표 상단324.427 vs 독립PDF327.961 검출 |
| 대상 회귀 |40PASS, exit0,3.323s | 두 표 상·하단, 캡션·뒤 본문,45쪽 꼬리·뒤 표 보존 |
| 형제 표·여백 정상 대조군 |31PASS, exit0,0.493s | #6946/#6795 block·partial 형제, #2439/#2279/#1880/#2097 보존 |
| CLI·쪽수·fmt |build exit0,1m51s;215/PDF215;fmt exit0 | 동일 원본과 검증 코드 사용 |
| Native44 |99.68706% |71.92862%에서 개선, 큰 표·캡션 위치 차이 해소 |
| Native43/45/46 |91.32182/98.61863/97.24002% | 앞뒤 본문·표·그림 경계 직접 판독 |

준비 과정에서 suite 자동 배정이018→005로 바뀌어 첫 실행은0검사/exit4였다. 올바른005로 다시 실행한 위 수정 전 실패만 결함 검출로 인정한다. 설명 주석은 한글로 작성했다. 선택4쪽 review/standalone overlay8개를 직접 확인했으며 gate `passed`, sweep exit0, 글꼴 예외 없음이다. 표 괘선 농도, 일부 글자·각주 간격 차이는 남아 완전 일치로 보고하지 않는다.121/122의 각주160 소유와 전체 최종/fresh WASM/lint는 여전히 보류이며 아직 통합 PR을 만들지 않는다.

[manifest](../assets/pr7382_20260926/stage19_native_manifest.json)·[summary](../assets/pr7382_20260926/stage19_native_summary.json)·[metrics](../assets/pr7382_20260926/stage19_native_overlay_metrics.json).

![Native44 형제 표 바깥 여백 보정](../assets/pr7382_20260926/stage19_native_review_044.png)
![Native44 overlay](../assets/pr7382_20260926/stage19_native_overlay_044.png)
![Native45 뒤 본문·표 보존](../assets/pr7382_20260926/stage19_native_review_045.png)


## 메인터너 보정 20: 표시 쪽 본문 점유와 각주 소급 예약 공유

원본 문단1297은 첫 문장 뒤에 각주160 표시가 있고, 저장 앞7줄과 뒤3줄이 121/122쪽에 나뉜다. [원본 XML·동일 입력 HWP/HWPX 기준 PDF 좌표](../assets/pr7382_20260926/stage20_independent_geometry.json)에서 121쪽은 각주159/160, 122쪽은161만이다. 본문 컷은 맞았지만 통째 각주 소급 등록이 Native 전용이어서 HWPX의160을 꼬리 쪽122에 잘못 붙였다. 자동 점수98%도 이 소유 결함을 잡지 못했다.

기존 `native_hwp5_body_footnote_tail_reset`가 단단·단일 각주·비합성 표시와 양수→0 저장 경계 및 실제 꼬리 컷을 입증한 HWPX에 완료 표시 쪽 등록을 연결했다. 소급 등록은 기존 각주가 있는 쪽에 한정하며, 첫 각주·두 줄 충돌·별도 표시 reset의 우선순위를 유지한다. Native 등록에는 새 HWPX fit 조건을 적용하지 않는다.

공통 `plan_fragment`는 같은 `FormattedParagraph` 메트릭으로 흐름 전진량과 수용한 모든 줄 상자의 최대 점유 끝을 함께 생산한다. `commit_split_paragraph_fragment`가 `(문단, 끝 줄)`의 단 상대 점유 끝을 보존하고, `completed_body_fragment_note_fits`는 완료 단의 마지막 실제 조각을 확인한 뒤 그 끝과 단 원점, 기존 각주 영역·추가 예약량으로 공존을 판단한다. 예약 조회와 확정은 같은 `completed_page_note_added_height`를 소비하며 실제 본문·각주는 확정된 조각/영역에서 배치한다. 값이 없는 다단·데코 호스트는 새 경로로 승격하지 않는다. 음수 줄간격이 전진량을 줄여도 앞의 큰 줄 상자의 점유 끝을 잃지 않으며, 뒤 문단 간격은 본문 점유 끝으로 잘못 가산하지 않는다.

Producer `5375b1f33` + 최종 Rust/test diff SHA256 `481af03a6f15c74cdc470c25dc88b379a22e0d94fe284f0fe36a55d93649c472`; [명령·소스별 해시·전후 결과·잔여](../assets/pr7382_20260926/stage20_validation.json).

| 검사 | 결과 | 의미 |
| --- | --- | --- |
| 수정 전 실제 각주 소유 회귀 |1FAIL/1PASS, exit100,0.413s |159 상단1027.347 vs 독립PDF1011.729, 잘못된160 이월 검출 |
| 최종 관련·정상 대조군 |86PASS/1FAIL, exit100,5.771s | 대상45PASS, 정상 대조41PASS; 잔여1개는 아래 별도 캡션 결함 |
| 큰 각주·합성 저장 줄·큰 줄 상자 |PASS | 완료 쪽 강제 예약 거절. 수정 전에도 통과한 대조군을 결함 검출로 세지 않음 |
| Native HWP 동일 원본 |PASS |121/122의 기존 번호·좌표·본문 소유 보존 |
| CLI·쪽수 |build exit0,1m50s;215/PDF215 | 동일 입력, 최종 검증 코드 |
| fmt·source 단위 테스트 정책 |exit0;4205검사/298모듈 | 고정 base `eb9142dd7c` 대비 검사 |
| Native120/123 |99.67776/95.08258% | 앞뒤 표·본문·각주 직접 판독 |
| Native121/122 |98.96031/99.37751% |121에159/160,122에161만 표시됨을 직접 판독 |

점유 생산식의 임시 단위 계약은 줄 상자0..30/5..15px와 흐름 끝15px로 수정 전1FAIL(0.015s)→수정 후1PASS(0.014s)를 확인했다. 신규 `src` 단위 검사 정책이 거절해 진단 소스·로그는 `output`에 보존하고 제출 소스에서는 제거했다. 추가 실물 배치 반례는 `tests/cases/`에 두었다. 수동 큰 줄 반례 두 구성은 보완 전에도 통과해 최대 점유 생산식의 결함 검출 증거로 확대하지 않으며 정상 거절 대조로만 남긴다. 첫 suite 실행은 준비 후005→003 재배정을 놓친 내 오류로0검사/exit4였고, 위 실제 결과와 구분했다.

선택4쪽의 review/standalone overlay8개를 직접 확인했다. gate `passed`, sweep exit0, 글꼴 예외 없음이다. 글자 외곽·간격과123쪽 각주 줄바꿈 차이는 남아 완전 일치를 주장하지 않는다. [수정 전 manifest](../assets/pr7382_20260926/stage20_before_native_manifest.json)·[수정 후 manifest](../assets/pr7382_20260926/stage20_native_manifest.json)·[summary](../assets/pr7382_20260926/stage20_native_summary.json)·[metrics](../assets/pr7382_20260926/stage20_native_overlay_metrics.json).

정상 대조에서 Native HWP90의 표27 캡션 겹침을 발견했고, [보정20 전 정확한 커밋의 대조](../assets/pr7382_20260926/stage20_native90_prior_control.json)에서도 같은 검사1FAIL/exit100/0.208s로 재현했다. devel 실패나 이번 각주 변경의 회귀로 분류하지 않는다. 앞선 통합·보정에서 놓친 결함으로 다음 개별 보정에서 해결한다. 전체 최종 회귀·lint·fresh WASM·전체 시각 gate도 미완료여서 통합 PR 생성·승인 보류를 유지한다.

![121쪽 수정 전 각주160 누락](../assets/pr7382_20260926/stage20_before_native_review_121.png)
![121쪽 각주159·160 소유 복원](../assets/pr7382_20260926/stage20_native_review_121.png)
![121쪽 overlay](../assets/pr7382_20260926/stage20_native_overlay_121.png)
![122쪽 수정 전 각주160 잘못 이월](../assets/pr7382_20260926/stage20_before_native_review_122.png)
![122쪽 각주161만 보존](../assets/pr7382_20260926/stage20_native_review_122.png)
![122쪽 overlay](../assets/pr7382_20260926/stage20_native_overlay_122.png)


## 메인터너 보정 21: 선방출 Native 캡션과 첫 표 조각의 문단 기준 공유

[동일 원본 HWP·한컴 PDF90](../assets/pr7382_20260926/stage21_independent_geometry.json)의 표27은 캡션696.421..709.701px 뒤에718.095px에서 시작한다. 저장 문단962의 vpos46000HU, 표의 문단 기준 양수 오프셋1390HU와 바깥 위여백283HU가 이를 뒷받침한다. 앞 보정에서 첫 조각까지 넓힌 일반 흐름 원점은 선방출 호스트 전진량을 뺀 offset을 사용해700.267px에서 표를 칠했고, 이미 정상 위치에 있는 캡션과 겹쳤다. 단순히 Native를 fallback 대상에서 빼거나 paint에서 clamp하지 않고, 실제 문단 기준 좌표의 생산 결과를 마련했다.

`query_pre_emitted_caption_rowbreak_placement`는 기존 Native 번호 캡션 선방출 조건과 현재 단의 실제 `PartialParagraph` 소유, 비합성·연속 저장 줄, 미편집 단단/문단 상단 정렬을 확인한다. 저장 캡션 앵커를 현재 단 영역 좌표로 변환하고 signed offset·바깥 위여백으로 `ParagraphFloatPlacement`를 생산한다. 첫 조각 budget은 같은 table_top으로 가용 높이·컷을 고르고, commit은 같은 원점과 확정 조각 높이로 점유 끝을 기록한다. layout의 호스트 줄은 같은 stored_host_origin, 표는 resolved_table_top을 소비하므로 별도 para_start_y가 뒤에서 덮어쓰지 않는다. 새 쪽 이어받기는 이 first-only 계획을 재사용하지 않고 기존 실제 바깥여백·소유 계약을 유지한다. 앞 캡션이 현재 단에 없는 이월, 합성/편집/무효 저장 프레임과 다른 정렬은 이 계획으로 승격하지 않는다.

Producer `c34c15bbd` + 최종 Rust/test diff SHA256 `8ac3def6592910224db5c6445c7871333f6d7d328a35dd95109759394710337f`; [정확한 명령·해시·전후 결과](../assets/pr7382_20260926/stage21_validation.json).

| 검사 | 결과 | 의미 |
| --- | --- | --- |
| 수정 전 원본/양수 오프셋 정식 회귀 |2FAIL, exit100,0.498s | 원본 표 상단700.267 vs 독립PDF718.095 검출 |
| 최종 대상·정상 대조군 |89PASS, exit0,6.583s | 대상47와 정상42. 앞 단계의 Native90 캡션 실패도 PASS |
| 실제 표·뒤 내용 |PASS | 원본 첫 조각 하단995.711, 캡션 불겹침,90 관계 행/91 끝 행·캡션 무중복 |
| signed 양수 offset 변형 |PASS |500HU 증가에 표만6.667px 이동, 캡션 원점·뒤 행 소유 보존. 수동 좌표 계약 |
| CLI·쪽수 |build exit0,1m55s;HWP/HWPX215/PDF215 | 검증 코드와 동일 입력 |
| fmt·source 단위 테스트 정책 |exit0;4205검사/298모듈 | 고정 base `eb9142dd7c` 비교 |
| Native HWP90/91 |83.32319/96.28253% | 캡션·표 겹침 해소.90 본문 줄바꿈 차이는 보류 |
| Native HWPX106/107 |99.94111/99.02564% | 앞 보정의 공통 첫 표 여백·이어받기 배치 보존 |

새 Native HWP90/91과 HWPX106/107의 review/standalone overlay8개를 직접 확인했다.90은68.56281→83.32319%로 개선됐지만 앞 본문의 줄바꿈 차이가 남아 HWP gate `re_review_required`, sweep exit1이다. HWPX 선택 gate는 `passed`, exit0이다. 글꼴 예외를 쓰지 않고 본문 잔여 원인을 다음 개별 보정으로 조사한다. 표·캡션 부분 개선을 문서 전체 승인으로 확대하지 않으며 전체 최종 회귀·lints·fresh WASM·시각 gate 미완료로 통합 PR을 만들지 않는다.

[HWP manifest](../assets/pr7382_20260926/stage21_native_hwp_manifest.json)·[summary](../assets/pr7382_20260926/stage21_native_hwp_summary.json)·[metrics](../assets/pr7382_20260926/stage21_native_hwp_overlay_metrics.json), [HWPX manifest](../assets/pr7382_20260926/stage21_native_hwpx_manifest.json)·[metrics](../assets/pr7382_20260926/stage21_native_hwpx_overlay_metrics.json).

![Native90 수정 전 캡션 겹침](../assets/pr7382_20260926/stage21_before_native_hwp_review_090.png)
![Native90 캡션·표 보정 및 본문 잔여](../assets/pr7382_20260926/stage21_native_hwp_review_090.png)
![Native90 overlay](../assets/pr7382_20260926/stage21_native_hwp_overlay_090.png)
![Native91 끝 행·뒤 본문 보존](../assets/pr7382_20260926/stage21_native_hwp_review_091.png)
![HWPX106 원점·위여백 보존](../assets/pr7382_20260926/stage21_native_hwpx_review_106.png)
![HWPX107 이어받기·뒤 본문 보존](../assets/pr7382_20260926/stage21_native_hwpx_review_107.png)
