---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-05
---

# PR #7549 기여자 변경 검토

## 최종 판정

**메인터너 보정 후 수용 가능** — 개체 앵커 문자의 원본 글자모양 높이의 개별 범위와 통합 후보의 최종 로컬 검증을 완료했습니다. 원 PR 직접 병합이 아니라 이 문서에 기록한 체리픽 통합 후보를 대상으로 합니다. #7445 이관 문서의 전체 피델리티 수용을 주장하지 않습니다. 원격 통합 PR의 최신 head CI·보호 요건 통과 전에는 병합하지 않습니다.

## 접수·범위

- 원 PR: https://github.com/edwardkim/rhwp/pull/7549
- 기여자: planet6897. 제목: fix(composer): 줄 재계산이 비-글자취급 개체 기준 문자의 글자모양을 줄 높이에 넣는다 (Refs #7330)
- 원 head: `8a13d7a9fdce57148eb3a7f3c8d4f424b9942273`. base: devel.
- 검토 branch: `review/planet6897-20261004`. 통합 base: `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`. 적용 단계 head: `fe0e6b9a772bd783d8c5b09bf05721d973e9e7d4`.
- 원 PR reviewer: jangster77 지정
- 경로: collaborator_external_pr 9.1.1; intake_and_review, local_validation, visual_fixture_evidence, multi_pr_update_branch, post_merge.
- 작성 시점 CI/mergeability는 참고값이며 수용·push 전에 최신 source head를 다시 확인합니다.

## 원 기여와 체리픽 출처

| 원 commit | 통합 commit/처리 | 비고 |
| --- | --- | --- |
| `53346656870abfb1f07cbfedeff924a3ea3927c1` | `b063579ae7282172f041d99fbd0a11bf59847cdf` | applied |
| `8a13d7a9fdce57148eb3a7f3c8d4f424b9942273` | `bb3fb58e7cb64745bc061f362565a3ac453b9362` | applied |

## 조판 원칙과 검증 현황

| 항목 | 현재 근거와 범위 | 판정 |
| --- | --- | --- |
| 구현 근거와 일반성 | 비-글자취급 개체 기준 문자의 원본 글자모양 높이 | 개별 범위 확인 |
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

## 2026-10-04 통합 후보 직접 검토

- 생산 후보 `4cfe96e52`, 실행 head `44cc13c2c`. Native/fresh WASM은 같은 생산 코드이며 각 manifest에 입력·실행 산출물 해시를 보존했습니다. stage8 로컬 WASM 대체 빌드를 사용했고 Docker 최적화 검증으로 보고하지 않습니다.
- `samples/anchor_char_line_height/anchor_char_height.hwpx`와 독립 한컴 PDF `pdf/anchor_char_line_height/anchor_char_height-2020.pdf`: 전체 **1쪽**, Native/WASM **99.79652%**, 미달·누락 0. review PNG에서 두 표, 작은 host 글줄과 뒤 문단의 위치·내용을 직접 확인했습니다. 본문 경계에 작은 래스터 차이가 남으며 엄격 화소 동일성을 주장하지 않습니다.
- 합성 입력·독립 PDF의 출처·결정적 생성기는 기존 [자료 설명](../assets/anchor_char_line_height/README.md)을 대조했습니다. 비공개 `36428535` 전체 문서 일치율이나 기여자의 코퍼스 11/11·71건을 이번 실행으로 확인했다고 기록하지 않습니다.
- `regression_suite_025`의 `floating_anchor_char_line_height` 기존 **2/2 PASS**. 큰 기준 문자를 가진 실물 4건(교육과정, 음수 간격 host, 각주 꼬리, 수형조절 서식)과 작은 기준 문자의 국어시험 대조 1건은 한컴이 저장한 첫 줄 `text_height`를 문단 끝 편집 후 값과 대조합니다. 이 값은 HWPUNIT의 문서 글자모양 의미값이며 렌더 좌표를 고정하지 않습니다. 실물 문서 전체 시각 품질을 이 구조 검사로 주장하지 않습니다.
- 코드에서 개체의 텍스트 위치와 UTF-16 글자모양 위치를 각각 추출하며, 줄 범위 안의 비-글자취급 표·그림·도형 기준 문자 글꼴 크기를 텍스트 크기와 함께 최댓값으로 사용합니다. 문단 끝 기준 문자는 마지막 줄에만 속합니다. TreatAsChar 개체 높이는 기존 전용 경로가 유지합니다. 줄 폭이나 개체 흐름 예약 높이를 바꾸는 수정은 아닙니다.
- 추가 검사나 기대값 변경 없이 기존 검사를 유지했습니다. [직접 결과](../assets/planet6897_20261004/7549-stage11/results.json), [검토 PNG](../assets/planet6897_20261004/7549-stage11/native_review_p001.png). 로그는 ignored `output/pr-review/planet6897-20261004/stage11-7549-nextest.log`입니다.

### 병합 후 기여자 설명 계획

기여자께 기준 문자가 줄 글자모양을 소유하는 근거와 작은 기준 문자 대조군의 보존 결과를 설명드립니다. 통합 merge SHA·최신 CI·정본 링크·merge SHA 고정 합성 PNG를 안내하며 비공개 전체 문서 검증으로 확대하지 않습니다. related #7330은 미완료 작업이 있으면 유지합니다.

## 최종 통합 로컬 검증 — 2026-10-05

전체 회귀 head `aa10d4093f60c4e0c12aa186f2464861acbf89ea`, 최종 lint/build·fresh WASM·Skia head `42ef60c96`(테스트 모듈 순서만 정리), base `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`입니다. 전체 nextest는 **10,318 PASS/0 FAIL/50 skip**,563.885초, threads=8로 완료했습니다. Format·Native/WASM32 Clippy·workspace build·workspace all-target Clippy·manifest/unit tier 정책을 통과했습니다. fresh WASM은 Mac 로컬 `--no-opt` 대체 빌드이며 Docker 최적화 검증으로 보고하지 않습니다. Skia lib·그림 placeholder·직접 PDF 출력 범위도 실제 실행했습니다. Studio 전체1,815 PASS/0 FAIL/2 skip와 TypeScript/Vite build도 완료했습니다.

[정확한 head·명령·결과·원 source CI·개별 시각 범위](../assets/planet6897_20261004/final_validation.json)를 참고합니다. 시각 자료는 각 단계의 생산 code head와 입력/PDF hash로 고정되어 있으며, 이후 변경은 테스트·문서 범위입니다. 최종 fresh WASM hash와 기존 단계의 hash, 생산 본문 동일성 증적을 함께 기록합니다. 대용량 미달 문서는 #7445의 기존·추가 이관 기록으로 추적하고 그 문서 전체의 정확성을 승인하지 않습니다. 위의 “통합 검증 미완료/대기”는 해당 단계 작성 시점의 기록이며 최종 로컬 판정은 이 절을 따릅니다. 통합 CI는 아직 미실행입니다.

## Merge 후 contributor PR comment 계획

기여자의 개체 앵커 문자의 원본 글자모양 높이 결과와 이 문서에 적은 추가 보정의 원인·범위를 한국어 존댓말로 구분해 설명합니다. 실제 통합 merge SHA·CI URL·Visual Sweep 정본과 이관 잔여 범위를 안내하고, 통합 대체 병합으로 원 PR을 close합니다. UTF-8 `--body-file`로 게시 후 API로 본문·close 상태를 재확인하며 중복 댓글은 게시하지 않습니다. [대표 PNG](../assets/planet6897_20261004/7549-stage11/native_review_p001.png)는 실제 merge SHA에 고정한 raw URL로 표시합니다.
