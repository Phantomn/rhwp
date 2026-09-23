# Task #7353 — V2 텍스트 구성 결과와 fragment 출력 연결

- 선행 절편: `18833e247` — [stage4](task_m100_7353_stage4.md)
- 범위: 신규 `table_v2/text.rs`의 표 내부 미리보기 API. Legacy 기본 경로는 변경하지 않는다.
- 상태: R3 진행 중. 문서 전체 V2 선택·실물 시각 검증 완료를 뜻하지 않는다.

## 이번 구현

`PreparedTextTable::prepare`가 원본 IR과 해소된 스타일을 받아 한 번 구성한다.
공통 `ParagraphBox`/`layout_paragraph_in_frame`으로 줄을 나누고,
`layout_composed_paragraph_in_frame`이 실제로 만든 TextLine/TextRun을 보관한다.
Legacy 표의 HeightMeasurer, table_layout, table_partial, CellUnit/RowCut은 호출하지 않는다.
공통 문단 코드의 기존 폰트·줄 구성 동작은 재사용하므로, 그 결과 자체가 독립적으로
한컴 정답이라는 의미도 아니다.

| 생산 결과 | 측정·fit 소비 | 실제 출력 소비 |
| --- | --- | --- |
| 최종 TextLine 상자와 문단 전진 끝점 | 동일 상자를 `ParagraphItem::Lines`에 넣고 사이/뒤 물리 공간은 `Space`로 보존 | 보관한 동일 TextLine/TextRun을 출력 |
| 셀 여백과 폭 | 기존 V2 IR adapter → flow rows → cursor | 확정된 CellPlacement 외곽 사용 |
| Fragment의 소유 줄 및 위치 | `TableCursor::fit`의 수용·비수용·이어받기 | `TextFragment::append_to`에서 평행 이동만 수행 |

출력 시 글자를 다시 나누거나 높이를 재계산하지 않는다. 클리핑으로 초과를 가리지 않는다.
내용 cursor와 paint snapshot을 함께 보관하므로 다른 폭/원본의 fragment와 payload를
임의로 결합할 수 없다. 원본 IR/스타일 수정은 이미 준비한 snapshot을 바꾸지 않는다.
문서의 편집 소유 관계가 아직 연결되지 않았으므로 가짜 문단 0의 provenance는 출력하지 않는다.

### 분할 경계

현재 지원하는 plain text는 한 실제 출력 줄이 분할 불가 유닛이다. 시작/끝 컷은 그 유닛의
소비 위치이며, 줄 사이 빈 공간·문단 앞뒤 간격·셀 여백은 별도 물리 공간으로 이어받는다.
`Never`는 전체 필요 높이 수용 여부를 먼저 확인하고, `WithinCells`는 기존 V2 cursor가
실제 수용한 유닛과 공간만 전진한다. 배치 후 높이를 늘리는 보정은 없다.
rowspan·반복 제목·TAC/어울림·자식 표 paint는 이 텍스트 미리보기 경로에 아직 적용되지 않는다.
기존 R3의 재귀 **기하** 지원과 이번 실제 텍스트 **paint** 지원 범위를 혼동하지 않는다.

## 지원 경계

- 지원: 저장 LineSeg 없는 새 일반 텍스트, 명시 개행, 폭 기반 줄바꿈, 빈 문단,
  문단 간격, 단순 사각 셀의 분할/이어받기, RenderTree 및 SVG 소비.
- 미지원은 명시적 오류: 저장 LineSeg, 개체가 섞인 문단, 테두리/배경, 목록/필드,
  keep 제약, 겹치는 줄 상자, 셀 밖으로 나오는 출력, 문서별 폰트 측정 context.
- 전체 문서의 앵커/후속 본문 배치, 편집 hit testing, 캐시 무효화, WASM continuation,
  기타 backend sidecar 연결은 아직 없다. 기존 문서를 V2로 성공 처리하는 fallback도 없다.

## 검사 근거

`tests/cases/issue_7353_table_v2_text.rs`는 수동 생성한 IR 계약이다. 저장 문서나 한컴 PDF의
대용이 아니다. 단위를 분명히 하기 위해 DPI 7200에서 1HU=1px로 구성하고, 스타일에
12px 글자와 **고정 18px 줄간격**을 직접 선언한다. HU 퍼센트 줄간격의 양자화와 혼동하지 않는다.

- 여백 좌5/우7/상3/하4, 폭212: 실제 첫 출력 x=25, y=33(페이지 원점20,30).
- 문단 두 개: 3 + 18×2 + 4 = 43의 예약과 실제 표 외곽을 함께 검사.
- 앞2/뒤3 문단 간격 및 빈 문단: 세 문단 각각 23의 전진, 총 76의 물리 높이.
- 세 페이지 예산21/18/22: alpha/beta/gamma가 정확히 한 번씩 출력되고,
  총 예약61, 마지막 유닛/아래 여백 뒤 Complete가 되는지 검사.
- 폭 변경/명시 개행, row-major와 다른 셀 저장 순서, 반복 fit의 불변성,
  비수용 뒤 재질의, 원본 변경 후 snapshot 보존, page-local ID 중복 방지를 검사.
- 실제 SVG 문자열에 글자가 나오는지는 backend 스모크이며 시각 판정으로 세지 않는다.
- 일반 96 DPI의 4pt 빈 문단도 별도로 검사한다. 400HU 글자와 156% 줄간격은
  400+224=624HU, 즉 8.32px의 전진이다. 빈 문단 앞뒤 실제 출력 y와 총 점유를 검사한다.

새 기능 검사이므로 이전 코드에는 해당 API가 없다. 이전 API 부재로 인한 컴파일 실패를
기존 결함의 수정 전 FAIL로 보고하지 않는다. 기존 회귀의 baseline/ignore는 수정하지 않는다.

## 검증 기록

- 소스: `18833e247` + 이번 작업 변경. [SHA-256](../../output/7353/r5/source.sha256)의
  V2 제품 파일과 검사 파일이 source/review worktree에서 바이트 단위로 같은지 확인했다.
- 실행 worktree: `/home/edward/mygithub/rhwp-review-7353`.
  고정 target: `/home/edward/mygithub/rhwp/target/pr-review`.
- 각 source를 `node scripts/run-rust-test.mjs <case> -- --cargo-profile release-test
  --target-dir /home/edward/mygithub/rhwp/target/pr-review --no-fail-fast`로 실행했다.
  [선택 검사](../../output/7353/r5/selected-tests.log)와
  [최종 텍스트 검사](../../output/7353/r5/text-tests-final.log) 기준으로
  **30건 PASS**: 텍스트7 + 기존 V2 기하8/중첩12 + Legacy #5301의3.
  마지막 추가/수정은 테스트뿐이며, 최종 텍스트7건에 선행6건을 중복 합산하지 않는다.
- Native lib Clippy `-D warnings` PASS:
  [native-clippy.log](../../output/7353/r5/native-clippy.log).
- WASM lib Clippy `-D warnings` PASS:
  [wasm-clippy.log](../../output/7353/r5/wasm-clippy.log).
- 새 검사 target `regression_suite_023` Clippy `-D warnings` PASS:
  [test-clippy.log](../../output/7353/r5/test-clippy.log).
- review worktree의 `cargo fmt --all -- --check` 및 고정 base
  `7a95e46e025470a4d7a7b59ad68ec02958bda738` 대비 manifest 검사 PASS:
  [policy-check.log](../../output/7353/r5/policy-check.log).
  `git diff --check`, 최종 파일 hash 재검사와 이 문서의 상대 링크 존재 검사도 PASS.

검증 준비 중 harness 생성 후 파일 크기가 바뀌어 자동 배정과 실행 target이 어긋난 0건 실행은
통과로 세지 않았다. 재생성 후 올바른 suite에서 실제 test 개수를 확인했다.
초기 합성 입력의 기본 span=0은 유효한 1×1 셀 선언으로 수정했으며, 제품의 span 거부 조건은
완화하지 않았다. SVG의 개별 tspan을 전체 문자열 검색으로 검사하던 스모크도 XML의 실제
text 자손을 읽도록 수정했다. 이 준비/검사 수정은 기존 제품 결함의 수정 전 FAIL 증거가 아니다.

전체 CI·Docker WASM·Native/fresh WASM Visual Sweep은 이 단계에서 통과로 주장하지 않는다.
WASM 대상 lint는 WASM 브라우저 실행이나 패키지 빌드가 아니다. 이 경로를 문서 엔진에 연결한 뒤
원본/독립 기준 출력과 동일 페이지를 직접 비교해야 R3 대표 사례 충족을 판정할 수 있다.
다음 연결 대상은 중첩 표의 실제 paint와 문서 조판 세션이다. 저장 LineSeg·TAC/어울림은
기존 계약표의 입력·반례를 유지하면서 각각 지원 범위를 넓힌다.
