---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-04
---

# PR #7540 기여자 변경 검토

## 최종 판정

**머지 보류** — 체리픽 적용 후 통합 head의 focused·전체 회귀와 필수 시각/구조 검증 진행 중. 원 source의 CI·PNG를 통합 검증 통과로 취급하지 않습니다.

## 접수·범위

- 원 PR: https://github.com/edwardkim/rhwp/pull/7540
- 기여자: planet6897. 제목: test(oracle): 편람 한컴 2024 KoPub 설치 환경 PDF 등재 — 정답지는 383 (#7009)
- 원 head: `ae656ec6ac9952c5613ba3d5419f667a5de7cd4b`. base: devel.
- 검토 branch: `review/planet6897-20261004`. 통합 base: `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`. 적용 단계 head: `fe0e6b9a772bd783d8c5b09bf05721d973e9e7d4`.
- 원 PR reviewer: jangster77 지정
- 경로: collaborator_external_pr 9.1.1; intake_and_review, local_validation, visual_fixture_evidence, multi_pr_update_branch, post_merge.
- 작성 시점 CI/mergeability는 참고값이며 수용·push 전에 최신 source head를 다시 확인합니다.

## 원 기여와 체리픽 출처

| 원 commit | 통합 commit/처리 | 비고 |
| --- | --- | --- |
| `ae656ec6ac9952c5613ba3d5419f667a5de7cd4b` | `f2e6b4ac7a4f769153327a8eff74dc5cf90a5a40` | applied |

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

## 독립 기준 PDF 자산 검토 — 개별 범위 충족

- 이 PR은 생산 코드·회귀 기대값 변경이 아니라 KoPub 설치 환경의 한컴 2024 기준 PDF 두 벌과 출처 기록을 추가합니다. 기존 #7009/#7445의 전체 피델리티가 해결된 것으로 확대 판단하지 않습니다.
- HWP/HWPX 원문 SHA-256과 두 PDF SHA-256이 기여자 변환 원장의 값과 일치함을 파일에서 재계산했습니다. 각 PDF 4,692,840바이트, 383쪽, 555×754pt이며 creator/producer `Hancom PDF 1.3.0.550`입니다. 두 PDF에는 실제 KoPubBatang/KoPubDotum Light·Medium·Bold 6종이 모두 내장되어 있습니다. [실제 pdfinfo·pdffonts·hash 검증](../assets/planet6897_20261004/7540-provenance/results.json)을 보존했습니다.
- 생성 원장은 한컴 2024 엔진 13.0.0.3901의 두 job·입력/결과 hash를 연결합니다. cairo 출력과는 별도이며 원문 font 환경이 다른 기존 384쪽 PDF와 섞지 않습니다. 입력/결과의 원장 연결은 확인했으나 과거 원격 변환을 이번 head에서 다시 수행한 것으로 쓰지 않습니다.
- 자동 PDF 선택 규칙·기존 회귀 쪽수 원장 변경은 없습니다. Native/HWPX의 전체 383쪽 렌더 검증을 이번 자산 검토로 대체하지 않습니다. **PDF 자산/출처 기여는 개별 범위를 충족**, 통합 최종 검증은 대기입니다.
- merge 후 contributor 설명에는 독립 PDF·KoPub 내장 환경 검증과 남은 #7009/#7445 범위를 구분하고 정확한 merge SHA를 기록합니다. 아직 issue 종료·원 PR comment/close를 수행하지 않았습니다.
