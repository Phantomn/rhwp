---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-05
---

# PR #7538 기여자 변경 검토

## 최종 판정

**머지 보류** — 원 head의 CI Impact Policy PENDING은 Controller attempt2로 해소됐습니다. 최신 devel 기반 `review/planet6897-7538-20261005`에 원 변경을 적용하고 충돌·호출 계약 보정을 완료했습니다. 최종 시각·전체 회귀·통합 CI 확인 전이므로 수용은 보류합니다. #7575의 검증을 이 PR의 통과 근거로 대신하지 않습니다.

## 접수·범위

- 원 PR: https://github.com/edwardkim/rhwp/pull/7538
- 기여자: planet6897. 제목: 수정(조판): 단 맨 위 문단 기준 자리차지 표 테두리를 바깥 위 여백 아래에 그린다 (#4068)
- 원 head: `e4abc99f2de1da071964bb3a86c22e4b3d0a4d64`. base: devel.
- 검토 branch: `review/planet6897-20261004`. 통합 base: `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`. 적용 단계 head: `fe0e6b9a772bd783d8c5b09bf05721d973e9e7d4`.
- 원 PR reviewer: jangster77 지정
- 경로: collaborator_external_pr 9.1.1; intake_and_review, local_validation, visual_fixture_evidence, multi_pr_update_branch, post_merge.
- 작성 시점 CI/mergeability는 참고값이며 수용·push 전에 최신 source head를 다시 확인합니다.

## 원 기여와 체리픽 출처

| 원 commit | 통합 commit/처리 | 비고 |
| --- | --- | --- |
| — | 미적용 | source CI Impact Policy 대기; 이번 녹색 통합 대상에서 보류 |

## 조판 원칙과 검증 현황

| 항목 | 현재 근거·해제 조건 | 판정 |
| --- | --- | --- |
| 구현 근거와 일반성 | 원 PR·관련 issue의 독립 기준을 코드와 대조 중 | 미검증 |
| 측정·배치 일관성 | source 공통 helper 이후 실제 원점/흐름 소비 지점 검토 중 | 미검증 |
| 분할·이어받기 계약 | 적용되는 컷·내용 소유·예약/배치와 정상 반례 검증 필요 | 미검증 |
| 줄 소속과 점유 높이 | 저장 LineSeg·재조판 경로의 실제 호출 및 반례 실행 필요 | 미검증 |
| 사례와 증거의 독립성 | source 증적은 참고; 통합 head 직접 검증 진행 중 | 미검증 |
| 기준값 변경 | changed baseline·새 회귀의 독립 PDF/사양 근거 검토 중 | 미검증 |
| 주장과 검증 범위 | focused 검사 실행 중; 선택 범위 완료 후 갱신 | 미검증 |

## 검증 입력 커밋 확인

미검증. 실제 실행한 HWP/HWPX/PDF를 열거하고 최종 검증 commit과 내용 hash를 대조합니다. 기존 #7445 이관 자료와 제외 회귀를 자동 재등록하지 않습니다.

## 시각 검증 계획

[Visual Sweep 정본](../../manual/verification/visual_sweep_guide.md#github-merge-comment)에 따라 Native/fresh WASM의 전체 대상 페이지 TSV를 먼저 산출합니다. 영향 페이지·미달 페이지·대표 경계는 review/overlay PNG로 직접 확인하며 쪽수·각주·문단 소유·누락/중복을 별도 판단합니다. 새 회귀는 실제 검증 범위 최저 90% 이상만 수용합니다.

## 다음 단계

원 PR 단위로 실패 원인과 증적을 먼저 분석하고, 필요한 보정은 코드 수정·결과 보고·커밋을 완료한 뒤 다음 보정으로 진행합니다. 통합 code candidate의 최종 검증 뒤 수용 판정·contributor 후속 comment 계획을 확정합니다. 아직 원 PR 또는 통합 PR을 병합한 것으로 표시하지 않습니다.

## 최신 접수 재확인 — 2026-10-05

원 head `e4abc99f2de1da071964bb3a86c22e4b3d0a4d64`는 접수 시점과 동일합니다. CI Impact Policy가 PENDING이므로 미적용으로 유지했습니다. 통합 branch의10,318 PASS는 이 원 head를 승인하는 근거가 아닙니다. 정확한 원 head와 CI 재실행이 조건을 충족하면 다음 접수에서 별도로 검토합니다. [원18개 CI 조회 기록](../assets/planet6897_20261004/final_validation.json).

## 2026-10-05 보류 해소 착수

- 검토 branch `review/planet6897-7538-20261005`, base `1e488ac370ec715a5b97c765542e16fc9bf65714`에서 원 head `e4abc99f2de1da071964bb3a86c22e4b3d0a4d64`의 두 commit을 `-x` 체리픽했습니다. 통합 commit은 `2743e2788`·`8d9fc8bd0`입니다.
- 충돌은 `issue_7062_tac_object_host_line_height`에서 발생했습니다. #7575의 기존 관계 기반 검사(제목·책임자의 셀 소속·내용 순서)를 유지해 해결했으며 원 PR의 절대141.37px·269.58px 고정값을 복원하지 않았습니다.
- 최초 focused 컴파일에서 새 devel 호출부 `paragraph_layout.rs`의 bool 인자가 원 PR의 f64 paint inset 계약과 충돌했습니다(E0308). 삽입 없는 호출의 인자를0.0으로 바꿨습니다. 저장 앵커 판정 helper의 오래된 주석도 실제 규칙(이월 시 이전 단 vOff 제거·새 단 outMargin 적용)에 맞췄습니다. 이는 문서별 위치 보정이나 공유 흐름 조건 확대가 아닙니다.
- 원 CI는 모두 green이나 Controller37012662838이 `missing-workflow:CI|CodeQL|Render Diff`로 pending을 발행했습니다. 현재 동일 branch/repository/head의 실행5개가 API에 조회되고 trusted selector가 CI37010657364를 선택합니다. Controller만 attempt2로 재실행했습니다. 회귀·시각 검증과 정책 최종 결과는 아직 확인 중입니다.

### 정책 보류 해소 결과

Controller [37012662838](https://github.com/edwardkim/rhwp/actions/runs/37012662838)의 attempt2가 success이며 정확한 원 head의 CI Impact Policy가 SUCCESS로 갱신됐습니다. 현재 API·trusted selector에서 같은 repository/branch/SHA의 CI를 정상 선택했습니다. 당시 수집 누락의 내부 원인은 재현되지 않았으므로 정책 구현 결함으로 단정하지 않습니다. 정책 코드를 바꾸거나 성공 status를 수동 발행하지 않았습니다. [현재 판정 증적](../assets/planet6897_7538_20261005/policy_resolution.json).

### 보정1 결과 — 새 devel 호출 계약 정합

`8d9fc8bd0` 이후 호출 인자·주석 보정에서 기존 #7538 focused3개가 **3 PASS/0 FAIL**입니다. 새 devel의 기존 관계 기반 #7062 검사는 충돌에서 보존했습니다. 전체·fresh WASM·시각 gate는 다음 단계이며 현재 최종 수용 판정은 보류입니다.

### 보정2 결과 — 원본 여백·흐름 관계 기반 회귀

- 신규 기여 검사3개의283/141HU와560HU를 원본 표 outMargin·vertical offset에서 직접 읽도록 바꿨습니다. 동일 쪽 둘째 표의 고정11.84px 기대값은 제거했습니다. 첫 표 paint 여백을0으로 바꾼 대조군에서도 뒤 표의 저장 흐름이 움직이지 않으며 두 표가 겹치지 않는 계약을 검사합니다. 허용 오차는 수치 환산 관용이며 절대 페이지 좌표를 정답으로 고정하지 않습니다.
- 수정한 기존 함수3개 **3 PASS/0 FAIL**, 기존 #7062 관계 기반 함수3개 **3 PASS/0 FAIL**입니다. 검사 함수를 추가하지 않았습니다.
- fresh Native/fresh WASM에서1쪽·2쪽·10쪽 문서의 전체13쪽은 두 경로 각각 최저99.95151%·98.62979%·95.17961%, 미달0입니다. Native 대표 PNG에서 표 위 여백·셀 안 그림·뒤 그림·둘째 표·머리 표의 제목/책임자 소속을 직접 확인했습니다.
- 정책연구215쪽의 Native24쪽은99.95810%이며 그림23·24와 설명·뒤 문단 소속이 정상입니다. 전체 비교의7쪽 미달은 devel과의 A/B로 새 회귀 여부를 확인 중입니다. 이 단계에서 문서 전체 수용을 주장하지 않습니다.

### 시각 비교와 최종 검증 착수

Native/fresh WASM 전체228쪽씩 TSV 산출을 완료했습니다. 단독1쪽·신청서2쪽·보도자료10쪽은 모두90% 이상이며, 정책연구의 영향24쪽은99.95810%입니다. 두 출력의 대표4쪽 PNG를 직접 확인했으며 내용 소속·표 위 여백·뒤 개체의 흐름과 그림 설명이 정상입니다. 정책연구의44·158·168·173·184·208·211쪽은 두 출력에서 모두90% 미만입니다. 정확한 base 빌드 A/B로 변경 출처를 확인하며 이 문서 전체의 수용은 주장하지 않습니다. 기존 회귀를 삭제하거나 기준값을 완화하지 않았습니다.

검사 설명에 남아 있던 고정11.84px 기대값도 실제 관계 기반 검사 내용에 맞춰 고쳤고,24쪽 TSV 값을 원본99.95810%로 정정했습니다. 필수 fmt·Native/WASM/all-target Clippy·workspace build·manifest는 모두 통과했습니다. 정확한 base에서 신규 검사의 수정 전 실패를 확인한 뒤 현 branch의 전체 nextest·Native Skia 검증을 순차 진행합니다.
