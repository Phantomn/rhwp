---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-04
---

# PR #7545 기여자 변경 검토

## 최종 판정

**머지 보류** — 체리픽 적용 후 통합 head의 focused·전체 회귀와 필수 시각/구조 검증 진행 중. 원 source의 CI·PNG를 통합 검증 통과로 취급하지 않습니다.

## 접수·범위

- 원 PR: https://github.com/edwardkim/rhwp/pull/7545
- 기여자: planet6897. 제목: fix(layout): 셀 내용 폭을 한/글 저장 격자(4 HWPUNIT)로 내려 줄을 나눈다 (#7412)
- 원 head: `044027a9030f6893b8369be0e64efbb0ec410d57`. base: devel.
- 검토 branch: `review/planet6897-20261004`. 통합 base: `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`. 적용 단계 head: `fe0e6b9a772bd783d8c5b09bf05721d973e9e7d4`.
- 원 PR reviewer: jangster77 지정
- 경로: collaborator_external_pr 9.1.1; intake_and_review, local_validation, visual_fixture_evidence, multi_pr_update_branch, post_merge.
- 작성 시점 CI/mergeability는 참고값이며 수용·push 전에 최신 source head를 다시 확인합니다.

## 원 기여와 체리픽 출처

| 원 commit | 통합 commit/처리 | 비고 |
| --- | --- | --- |
| `044027a9030f6893b8369be0e64efbb0ec410d57` | `964829525d8c6ea61a31d0f64a3e90f6afa85cbe` | layout_frame::origin_is_authoritative 필드 보존; rows 주석은 셀 폭 격자 근거로 갱신 |

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

## Stage 16 메인터너 보정 — HWPX 구판 여백 단위와 첫 조각 원점

**계속 보류**입니다. 사용자 요구에 따라 HWP 121쪽은 원 PR 정본 93.56018% 이상, HWPX 121쪽은 90% 이상을 해제 조건으로 유지합니다. HWPX를 #7445로 이관하거나 회귀를 삭제하지 않습니다.

- 동일 HWPX를 한컴 engine 2020으로 PDF와 HWP로 각각 변환했습니다. 새 PDF도 156쪽이며, HWP 재저장의 문단 내어쓰기 값은 xmlVersion 1.2 case 값과 같습니다. 현재 파서가 여기에 최신 판본의 2배 정규화를 적용하던 단위 오류를 보정했습니다. 직렬화도 같은 패키지 판본의 역단위를 적용합니다.
- 저장 줄 없는 표 전용 호스트의 첫 조각은 예산에 예약된 앞 여백·오프셋을 확정 배치 원점으로 전달합니다. 픽셀 보정 상수를 추가하지 않았습니다.
- 기존 여백 왕복 검사 2개와 #7412 줄 소속 검사 2개: 4 PASS / 0 FAIL. 검사 수는 늘리지 않았습니다. HWPX의 규제 개요는 독립 PDF 120쪽에 대응하므로 기존의 잘못된 121쪽 전제를 바로잡았습니다.
- 새 WASM은 로컬 대체 빌드입니다. Native/fresh WASM HWP 121쪽은 91.73087%, HWPX 121쪽은 86.09215%입니다. 두 값 모두 이번 사용자 해제 조건을 충족하지 못했습니다.
- HWPX 출력과 PDF는 이제 모두 156쪽이지만, 전체 156쪽 TSV에는 4쪽 조문 대비표의 줄바꿈에서 시작하는 내용 대응 차이가 남아 있습니다. 쪽수 일치만으로 정상 페이지 소유를 주장하지 않습니다.
- 코드·바이너리·WASM 해시와 전체 TSV, 새 121쪽 PNG: [Stage 16 증적](../assets/planet6897_20261004/7545-stage16/results.json). 로그와 진단 변환본은 ignored output에만 보존합니다.

다음 보정은 4쪽 저장 줄의 너비·내어쓰기와 120~121쪽 표 이어받기의 내용 소유 및 글줄 폭을 독립 PDF와 대조합니다. HWP 121쪽의 원 PR보다 낮은 일치율을 허용하거나 Visual Sweep 수치를 인위적으로 변경하지 않습니다.
