---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-04
---

# PR #7557 기여자 변경 검토

## 최종 판정

**개별 차선 변경 범위 검증 통과·통합 머지 보류** — 기존 검사 선택 조건을 의미 기반으로 보정하고 영향 14쪽·대조 201쪽의 Native/fresh WASM과 PNG를 확인했습니다. 나머지 원 PR 및 통합 최종 검증은 진행 중입니다.

## 접수·범위

- 원 PR: https://github.com/edwardkim/rhwp/pull/7557
- 기여자: planet6897. 제목: fix(layout): 어울림 표 옆 차선에서 시작하는 다음 문단을 표 하단으로 밀지 않는다 (#7548 1단계)
- 원 head: `144271afb19c4f3c386184c696c8040d2b433b9a`. base: devel.
- 검토 branch: `review/planet6897-20261004`. 통합 base: `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`. 적용 단계 head: `fe0e6b9a772bd783d8c5b09bf05721d973e9e7d4`.
- 원 PR reviewer: jangster77 지정
- 경로: collaborator_external_pr 9.1.1; intake_and_review, local_validation, visual_fixture_evidence, multi_pr_update_branch, post_merge.
- 작성 시점 CI/mergeability는 참고값이며 수용·push 전에 최신 source head를 다시 확인합니다.

## 원 기여와 체리픽 출처

| 원 commit | 통합 commit/처리 | 비고 |
| --- | --- | --- |
| `f40f56b915addf305ee3e167ea7401b5f2fe66c2` | `88799ba556509a071d4673f919cb2c2601d7075a` | #6761 TAC 기준선 상수와 #7548 전폭 저장 줄 helper를 모두 보존 |
| `61b36c9a1dd0044334f46cef1e8648ab39251ad4` | `297788228ee0a85a5ce6ce06ef5f788fa843d9e3` | 저장 줄 없는 Square host 공통 결과를 유지하고 저장 줄 옆 차선 후속 분기 추가 |
| `558072f3782e855060e11295bed5966eb572b2c0` | `4aeed337e670542f6c832fa209b78982bbe9ac00` | applied |
| `144271afb19c4f3c386184c696c8040d2b433b9a` | `3252bd02dac29025bcbe94f867fb5d2369b56b61` | applied |

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

## 2026-10-04 메인터너 보정과 직접 검토

- 생산 후보 `4cfe96e52`. `samples/21_언어_기출_편집가능본.hwp`와 독립 `pdf/21_언어_기출_편집가능본-2022.pdf` **14쪽** Native/fresh WASM **98.67045%**. 대조 `samples/task1725/text_footnote_tail_overpagination.hwp` / `pdf/task1725/text_footnote_tail_overpagination-hwp-2024.pdf` **201쪽** **97.65430%**. 선택한 2쪽 모두 미달·누락 0이며 전체 문서 점수·쪽수 검증으로 확대하지 않습니다. 이 PR의 기존 회귀는 전체 쪽수 대신 14쪽 문단 소속을 확인합니다.
- 두 review PNG를 직접 열었습니다. 14쪽의 pi=300 첫 줄이 [A] 꺾쇠 표 옆 차선에서 시작하고 둘째 줄은 전폭으로 돌아오며, host 마지막 줄에서 이어집니다. 201쪽 표·본문과 다음 표 위치도 보존됩니다. 201쪽 화살촉의 잉크 크기 차이는 남아 있으며 차선 수정이 해결했다고 주장하지 않습니다. 이 대조 쪽의 전체 화소 완전 일치를 승인하는 기록은 아닙니다.
- 기존 `issue_7548_square_lane_successor.rs`에서 표 선택 `width < 40px`, `host_top + 40px`를 제거했습니다. host 문단 299가 소유하는 최외곽 TableNode를 선택하므로 폭·위치가 조정되어도 대상이 유지됩니다. 고정 1816HU 간격 상수도 입력의 host 마지막 LineSeg와 다음 문단 첫 LineSeg vpos 차이에서 읽도록 보정했습니다. 실제 판정은 저장 간격·차선/전폭·표 띠 소속 관계를 유지합니다. 새 검사 추가나 기대값 완화는 없습니다.
- 수정한 `regression_suite_008` 기존 **2/2 PASS**. 실행 로그는 ignored `output/pr-review/planet6897-20261004/stage13-7557-nextest.log`에 있습니다.
- 코드 대조에서 layout/typeset이 같은 `square_successor_starts_beside_table`을 소비합니다. 정상 저장 host의 좁은 첫 줄 차선과 successor 첫 줄이 겹치는지 판단하며 빈 sw·구현 합성 줄을 제외합니다. 저장 전폭 한 줄을 기준으로 cs 차이를 반영해 왼 여백을 두 번 더하지 않습니다. 저장 줄 없는 host의 선행 공통 flow plan과 #7564 쪽 기준 분기 우선순위는 보존했습니다.
- [결과](../assets/planet6897_20261004/7557-stage13/results.json), [14쪽 PNG](../assets/planet6897_20261004/7557-stage13/native_review_p014.png), [201쪽 대조 PNG](../assets/planet6897_20261004/7557-stage13/native_review_p201.png). 실행 바이너리·입력·PDF·글꼴·스크립트 해시와 TSV를 함께 보존했습니다. fresh WASM은 stage8 로컬 대체 빌드이며 실제 Studio Canvas 또는 Docker 최적화 검증으로 확대하지 않습니다.

### 병합 후 기여자 설명 계획

기여자께 공통 차선 판정의 측정·배치 일관성과 표 선택의 좌표 의존성을 제거한 메인터너 보정 이유를 설명드립니다. 통합 merge SHA·최신 CI·정본 링크·merge SHA 고정 PNG를 안내합니다. #7548의 남은 일반화 작업과 관련 원본 자료는 보존합니다.
