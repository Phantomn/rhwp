---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-05
---

# PR #7551 기여자 변경 검토

## 최종 판정

**개별 변경 범위 검증 통과·통합 머지 보류** — 19쪽 직접 Native/fresh WASM 및 기존 관련 검사 6개를 확인했습니다. 픽셀 고정 높이·대상 선택·정렬 기대값을 저장 입력의 의미와 관계로 보정했습니다. 통합 최종 검증은 진행 중입니다.

## 접수·범위

- 원 PR: https://github.com/edwardkim/rhwp/pull/7551
- 기여자: planet6897. 제목: fix: 쪼개지는 1행 중첩 표 조각이 옆 칸 Square 그림의 행 높이를 담는다 (Refs #7387)
- 원 head: `bff62d2ba3f9942c889cba3f0aae17f7ee50f0af`. base: devel.
- 검토 branch: `review/planet6897-20261004`. 통합 base: `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`. 적용 단계 head: `fe0e6b9a772bd783d8c5b09bf05721d973e9e7d4`.
- 원 PR reviewer: jangster77 지정
- 경로: collaborator_external_pr 9.1.1; intake_and_review, local_validation, visual_fixture_evidence, multi_pr_update_branch, post_merge.
- 작성 시점 CI/mergeability는 참고값이며 수용·push 전에 최신 source head를 다시 확인합니다.

## 원 기여와 체리픽 출처

| 원 commit | 통합 commit/처리 | 비고 |
| --- | --- | --- |
| `4eed06d7f853ff734f0e76cff13b27a28db21e48` | `c11fa8cc5cec62177e56cd41908b776bb7b403fe` | applied |
| `bff62d2ba3f9942c889cba3f0aae17f7ee50f0af` | `3393af4cafb18c0abf989752f07ce29912513b07` | applied |

## 조판 원칙과 검증 현황

| 항목 | 현재 근거와 범위 | 판정 |
| --- | --- | --- |
| 구현 근거와 일반성 | 중첩 한 행 표의 그림 바닥과 칸 안 여백 관계 | 개별 범위 확인 |
| 측정·배치 일관성 | 아래 저장 사양·내용 소유 및 직접 결과의 범위로 확인 | 개별 범위 확인 |
| 분할·이어받기 계약 | 아래 개별 구조 검사·시각 결과에 한정; 전체 corpus 수용 주장 없음 | 개별 범위 확인 |
| 줄 소속과 점유 높이 | 아래 저장 줄·개체 소유 및 정상 대조 검증 범위에 한정 | 개별 범위 확인 |
| 사례와 증거의 독립성 | 아래 통합 직접 실행·독립 한컴 PDF 또는 사양 및 입력 hash 참조 | 확인 |
| 기준값 변경 | 아래 기존 회귀 보정·유지 이유 참조; 출력 좌표를 새 정답으로 고정하지 않음 | 확인 |
| 주장과 검증 범위 | 개별 기여 범위 확인; 최종 전체 nextest·lint/build·통합 CI 완료 전 병합 불가 | 통합 보류 |

## 검증 입력 커밋 확인

아래 개별 단계의 실제 입력·독립 PDF·실행 head와 증적 JSON/manifest에 내용 hash를 기록했습니다. 기존 #7445 이관 자료는 해당 단계에 적은 범위만 유지하며 전체 피델리티 승인을 주장하지 않습니다. 최종 전체 검증 결과는 별도 완료 후 기록합니다.

## 시각 검증 계획

[Visual Sweep 정본](../../manual/verification/visual_sweep_guide.md#github-merge-comment)에 따라 Native/fresh WASM의 전체 대상 페이지 TSV를 먼저 산출합니다. 영향 페이지·미달 페이지·대표 경계는 review/overlay PNG로 직접 확인하며 쪽수·각주·문단 소유·누락/중복을 별도 판단합니다. 새 회귀는 실제 검증 범위 최저 90% 이상만 수용합니다.

## 다음 단계

원 PR 단위로 실패 원인과 증적을 먼저 분석하고, 필요한 보정은 코드 수정·결과 보고·커밋을 완료한 뒤 다음 보정으로 진행합니다. 통합 code candidate의 최종 검증 뒤 수용 판정·contributor 후속 comment 계획을 확정합니다. 아직 원 PR 또는 통합 PR을 병합한 것으로 표시하지 않습니다.

## 2026-10-04 메인터너 보정과 직접 검토

- 생산 후보 `4cfe96e52`, 실행 head `095c77199`의 생산 코드를 유지했습니다. `samples/hwpx_sample2.hwp` / 독립 한컴 `pdf/hwpx_sample2-2020.pdf` **19쪽** Native/fresh WASM 모두 **99.68937%**, 해당 쪽 미달·누락 0입니다. stage8 로컬 WASM 대체 빌드이며 실제 Studio Canvas·Docker 최적화 검증으로 확대하지 않습니다. 이번 선택 TSV는 문서 29쪽 전체 시각 품질·쪽수 검증이 아닙니다.
- review PNG를 직접 열어 하단 그림 칸과 「소명방법」 칸이 표 내부에 있고, 글자 칸 정렬·표 괘선·상단 본문 순서가 PDF와 맞는지 확인했습니다. 실루엣 보조값과 엄격 화소 동일성은 구분합니다. 기여자 점수 대신 통합 후보의 실행 근거를 보존했습니다.
- `issue_7387_nested_row_square_picture_floor.rs`의 고정 `107.6px` 기대값을 제거했습니다. 원 입력 문단 182의 중첩 1×2 표에서 그림 높이·세로 오프셋·유효 칸 상하 안 여백을 읽어 독립 행 높이 하한으로 비교합니다. 그림 바닥이 표 안에 포함되는 기존 관계도 유지합니다. 구현에서 낡은 표 선언 높이를 쓴 값을 테스트 정답으로 삼지 않습니다.
- 함께 바뀐 `issue_4068_unclipped_cell_honors_valign.rs`의 `955~975px` 대상 선택을 제거해 그림을 소유한 중첩 1×2 표의 칸을 선택합니다. `3~5px` 정렬 몫·`5.83px` 첫 줄·`2.5px` 그림칸 상한도 제거했습니다. 독립 저장 글줄의 내용 높이와 유효 안 여백을 읽고, Center 몫이 `(칸 높이−상하 안 여백−저장 내용 높이)/2`인지, 여유 없는 그림 칸은 저장 안 여백만 적용하는지 검사합니다. 허용 오차는 래스터/분할 반올림을 위한 비교 오차이며 기대 좌표가 아닙니다.
- 기존 `regression_suite_009`의 1개와 `regression_suite_026` 관련 5개, 합계 **6/6 PASS**. 새 검사 추가·삭제 없이 이번 PR가 바꾼 기존 검사만 보정했습니다. 별도 쪽-잘림 보호 입력의 기존 검사와 부모 viewport 반례는 통과했고 대상 범위 밖 계약을 고치지 않았습니다.
- 구현은 1행 중첩 표의 조각 유닛 합이 그림 등 개체 바닥+안 여백에 모자랄 때 마지막 조각에 부족분을 반영합니다. 선언 `common.height`를 무조건 하한으로 사용하지 않으며 다행·HWPX atom 경로의 높이 계약은 그대로입니다. 기여자가 수행한 전체 samples 바이트 A/B를 이번 직접 실행으로 재주장하지 않습니다.
- [결과·해시](../assets/planet6897_20261004/7551-stage14/results.json), [검토 PNG](../assets/planet6897_20261004/7551-stage14/native_review_p019.png). 로그는 ignored `output/pr-review/planet6897-20261004/stage14-7551-nextest-final.log`입니다.

### 병합 후 기여자 설명 계획

기여자께 그림 높이 하한 보존과 기존 검사 정답의 좌표 의존성을 제거한 보정 이유를 설명드립니다. 실제 통합 merge SHA·최신 CI·정본 링크·merge SHA 고정 19쪽 PNG를 안내합니다. 관련 #7387의 다른 문서 결함은 해결 완료로 닫지 않습니다.
