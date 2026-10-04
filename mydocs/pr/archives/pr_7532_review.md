---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-05
---

# PR #7532 기여자 변경 검토

## 최종 판정

**메인터너 보정 후 수용 가능** — 기울임 공백 메트릭·쪽번호 정렬 관계의 개별 범위와 통합 후보의 최종 로컬 검증을 완료했습니다. 원 PR 직접 병합이 아니라 이 문서에 기록한 체리픽 통합 후보를 대상으로 합니다. #7445 이관 문서의 전체 피델리티 수용을 주장하지 않습니다. 원격 통합 PR의 최신 head CI·보호 요건 통과 전에는 병합하지 않습니다.

## 접수·범위

- 원 PR: https://github.com/edwardkim/rhwp/pull/7532
- 기여자: planet6897. 제목: test: 우측 정렬 바탕쪽 쪽번호가 칸 오른쪽에 맞는지 회귀 고정 (#7428)
- 원 head: `d3d39b16bc2e33a7aeef664bc9d12451d05cd86a`. base: devel.
- 검토 branch: `review/planet6897-20261004`. 통합 base: `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`. 적용 단계 head: `fe0e6b9a772bd783d8c5b09bf05721d973e9e7d4`.
- 원 PR reviewer: jangster77 지정
- 경로: collaborator_external_pr 9.1.1; intake_and_review, local_validation, visual_fixture_evidence, multi_pr_update_branch, post_merge.
- 작성 시점 CI/mergeability는 참고값이며 수용·push 전에 최신 source head를 다시 확인합니다.

## 원 기여와 체리픽 출처

| 원 commit | 통합 commit/처리 | 비고 |
| --- | --- | --- |
| `d3d39b16bc2e33a7aeef664bc9d12451d05cd86a` | `0ede7647f2007e67b92a0cd16d0ff27bdd5f2841` | applied |

## 조판 원칙과 검증 현황

| 항목 | 현재 근거와 범위 | 판정 |
| --- | --- | --- |
| 구현 근거와 일반성 | 기울임 문자열의 공백 측정과 display_text 단위 계약 | 개별 범위 확인 |
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

## 20쪽 직접 시각 검증 및 기존 관계 회귀 보완 — 개별 범위 충족

- 원 기여는 이미 devel에서 해결된 #7428 쪽번호 표시 문자열/말미 공백 계약을 독립 한컴 기준과 회귀로 보존합니다. 절대 위치 대신 바탕쪽 표 칸 좌측·우측 경계와 숫자 run 경계의 관계를 검사합니다.
- 메인터너는 숫자 run을 찾는 `font_size > 35px` 조건을 자동 쪽번호의 `display_text` 존재 여부로 바꿨습니다. 숫자 표시값, 바탕쪽·표 칸 소속, 좌우 경계 관계는 유지했습니다. 테스트 2개를 그대로 수정했으며 새 검사를 추가하지 않았습니다.
- 현재 코드에서 기존 두 관계 회귀 2/2 PASS입니다. [실행·test source hash](../assets/planet6897_20261004/7532-stage9/results.json)를 보존했습니다.
- 입력 `samples/exam_kor.hwp`와 독립 `pdf/exam_kor-2022.pdf` 전체 20쪽을 직접 비교했습니다. Native/fresh WASM TSV의 페이지별 수치가 같고 최저 **90.80846%(17쪽)**, 미달·누락 0, 원문/PDF/출력 20쪽 일치입니다. [Native TSV](../assets/planet6897_20261004/7532-stage9/native_silhouette.tsv), [WASM TSV](../assets/planet6897_20261004/7532-stage9/wasm_silhouette.tsv). source/build/font 출처는 같은 디렉터리 manifest에 연결합니다.
- [2쪽](../assets/planet6897_20261004/7532-stage9/native_review_p002.png)·[3쪽](../assets/planet6897_20261004/7532-stage9/native_review_p003.png)·[11쪽](../assets/planet6897_20261004/7532-stage9/native_review_p011.png)·[17쪽](../assets/planet6897_20261004/7532-stage9/native_review_p017.png) PNG를 직접 열었습니다. 짝수쪽의 좌측 정렬, 홀수쪽 및 두 자리 11쪽의 우측 정렬과 실제 숫자 표시가 유지됩니다. 본문·문항·그림 소속 누락은 없습니다.
- 최저 17쪽에는 제목과 홀수형 상자의 글꼴 크기·잉크 차이가 남습니다. 쪽번호 회귀의 칸 정렬 계약과 구분하며 문서 전체가 픽셀 완전 일치한다고 쓰지 않습니다. 17쪽 엄격 픽셀 16.05%와 관용 실루엣 90.81%를 구분합니다. 도구 라벨·수치는 판독 가능합니다.
- 원 기여와 메인터너의 의미 기반 대상 선택 보완으로 **개별 회귀 범위 충족**입니다. 최종 전체 검증 전 통합 머지 보류를 유지합니다. merge 후에는 원 기여·보완 이유와 정확한 CI/merge SHA, 위 대표 PNG·정본 링크를 설명하며 아직 comment/close를 수행하지 않았습니다.

## 최종 통합 로컬 검증 — 2026-10-05

전체 회귀 head `aa10d4093f60c4e0c12aa186f2464861acbf89ea`, 최종 lint/build·fresh WASM·Skia head `42ef60c96`(테스트 모듈 순서만 정리), base `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`입니다. 전체 nextest는 **10,318 PASS/0 FAIL/50 skip**,563.885초, threads=8로 완료했습니다. Format·Native/WASM32 Clippy·workspace build·workspace all-target Clippy·manifest/unit tier 정책을 통과했습니다. fresh WASM은 Mac 로컬 `--no-opt` 대체 빌드이며 Docker 최적화 검증으로 보고하지 않습니다. Skia lib·그림 placeholder·직접 PDF 출력 범위도 실제 실행했습니다. Studio 전체1,815 PASS/0 FAIL/2 skip와 TypeScript/Vite build도 완료했습니다.

[정확한 head·명령·결과·원 source CI·개별 시각 범위](../assets/planet6897_20261004/final_validation.json)를 참고합니다. 시각 자료는 각 단계의 생산 code head와 입력/PDF hash로 고정되어 있으며, 이후 변경은 테스트·문서 범위입니다. 최종 fresh WASM hash와 기존 단계의 hash, 생산 본문 동일성 증적을 함께 기록합니다. 대용량 미달 문서는 #7445의 기존·추가 이관 기록으로 추적하고 그 문서 전체의 정확성을 승인하지 않습니다. 위의 “통합 검증 미완료/대기”는 해당 단계 작성 시점의 기록이며 최종 로컬 판정은 이 절을 따릅니다. 통합 CI는 아직 미실행입니다.

## Merge 후 contributor PR comment 계획

기여자의 기울임 공백 메트릭·쪽번호 정렬 관계 결과와 이 문서에 적은 추가 보정의 원인·범위를 한국어 존댓말로 구분해 설명합니다. 실제 통합 merge SHA·CI URL·Visual Sweep 정본과 이관 잔여 범위를 안내하고, 통합 대체 병합으로 원 PR을 close합니다. UTF-8 `--body-file`로 게시 후 API로 본문·close 상태를 재확인하며 중복 댓글은 게시하지 않습니다. [대표 PNG](../assets/planet6897_20261004/7532-stage9/native_review_p002.png)는 실제 merge SHA에 고정한 raw URL로 표시합니다.
