---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-05
---

# PR #7546 기여자 변경 검토

## 최종 판정

**개별 변경 범위 검증 통과·통합 머지 보류** — 기존 회귀 3개와 원본·정상 대조본 전체 1쪽씩의 Native/fresh WASM, 직접 PNG 검토를 완료했습니다. 나머지 원 PR 검토와 통합 최종 검증은 진행 중입니다.

## 접수·범위

- 원 PR: https://github.com/edwardkim/rhwp/pull/7546
- 기여자: planet6897. 제목: 수정(조판): 제 선언 폭과 모순되는 저장 줄 사다리는 다시 조판한다 (#7416, #7424 위)
- 원 head: `3a356923090d229beacdfa9d4dc83640429b877d`. base: devel.
- 검토 branch: `review/planet6897-20261004`. 통합 base: `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`. 적용 단계 head: `fe0e6b9a772bd783d8c5b09bf05721d973e9e7d4`.
- 원 PR reviewer: jangster77 지정
- 경로: collaborator_external_pr 9.1.1; intake_and_review, local_validation, visual_fixture_evidence, multi_pr_update_branch, post_merge.
- 작성 시점 CI/mergeability는 참고값이며 수용·push 전에 최신 source head를 다시 확인합니다.

## 원 기여와 체리픽 출처

| 원 commit | 통합 commit/처리 | 비고 |
| --- | --- | --- |
| `4cc0ffeedbfefa7a72a1bb03bce69ee26020394f` | `already-applied-source` | already-applied-source |
| `3d18a31ec5000d2d38cc596031e10849d65e74b5` | `already-applied-source` | already-applied-source |
| `235aaa6b092f251267ab993cdbc6dc9c15930d50` | `already-applied-source` | already-applied-source |
| `bd0660d312b4247c28f4da7194965c327112009d` | `already-applied-source` | already-applied-source |
| `a8388028a260c5d99e0107f30ba3a8ab2beb3801` | `already-applied-source` | already-applied-source |
| `7c2c1c05226731342e9b11b038d883adf3c0bab5` | `merge-excluded` | merge-excluded |
| `131eed5c2d117761cea9e583a3a27613dec02add` | `merge-excluded` | merge-excluded |
| `65b0414ef7e562fc265201f52d8408b589401dc5` | `a5cbf6e151450664204a0680c10fe55983e29f39` | applied |
| `d93c0e3f34b8b5a365b6a861f7ae3d025184fb9b` | `4bcb907a5a868a81c56f7554a12a362e8e99867c` | applied |
| `3a356923090d229beacdfa9d4dc83640429b877d` | `49bf7d134889bd5419eb6b188fe0ae66371cc5df` | applied |

## 조판 원칙과 검증 현황

| 항목 | 현재 근거와 범위 | 판정 |
| --- | --- | --- |
| 구현 근거와 일반성 | 자기모순 저장 줄의 제한된 재조판과 정상 사다리 보존 | 개별 범위 확인 |
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

## 2026-10-04 통합 후보 직접 검토 결과

- 생산 코드 후보 `4cfe96e52`, 실행 head `d78fdc0e0745f10b57e928b50716d298861c7559`. 이후 문서·#7532 검사 선택 조건 변경은 생산 코드를 바꾸지 않았습니다. 실행 바이너리·입력·글꼴·스크립트 해시는 각 manifest에 보존했습니다.
- `samples/issue6639/rhwp-table-cell-minimal-repro.hwp`와 한컴 재저장 정상 대조본 `samples/issue6639/issue6639-hancom-160.hwpx`를 `pdf/issue6639/issue6639-original-160-2020.pdf`와 비교했습니다. 같은 PDF를 쓰는 독립 근거는 `samples/issue6639/README.md`의 한컴 원본·재저장·저장 사다리 제거본 PDF 래스터 동일성 기록입니다.
- 원본과 대조본은 각각 1쪽이며 Native/fresh WASM 모두 전체 1쪽 **99.78048%**, 90% 미달·누락 0입니다. fresh WASM은 stage8 로컬 대체 빌드입니다. Docker 최적화 또는 실제 Studio Canvas 검증으로 확대하지 않습니다.
- 두 review PNG를 직접 열어 문단 1~10의 줄 소속, 문단 3 이후 빈 줄 띠 제거, 표 괘선·하단과 내용 누락 여부를 확인했습니다. 빨간 본문 색은 원본 재현 자료의 색상이며 결함 표시가 아닙니다. 실루엣은 보조값이고 엄격 화소 동일성을 주장하지 않습니다.
- `regression_suite_019`의 `issue_7416_self_contradicting_ladder_is_reflowed` 기존 검사 **3/3 PASS**. 한컴 저장 `textpos`에서 얻은 줄 텍스트, 재조판 뒤 문단 간격과 실제 줄 피치 관계, 포화된 정상 저장 사다리 보존을 검사합니다. 좌표 자체를 잠그지 않으므로 검사를 추가·삭제하거나 기대값을 바꾸지 않았습니다.
- 구현은 어절 내부에서 끊긴 왼쪽 정렬 저장 줄이 선언 폭에 두 글자 이상 여유를 남기는 모순에 한정합니다. 공백·인라인 제어·각주·분산/양쪽 정렬을 제외하고, 재조판된 앞 문단의 줄 수가 달라지면 다음 문단에 버린 저장 vpos를 재사용하지 않습니다. 정상 포화 사다리 대조본은 저장 줄을 유지합니다. 기여자의 코퍼스 수치 전부를 이번 직접 검증으로 주장하지 않습니다.
- [결과와 증적](../assets/planet6897_20261004/7546-stage10/results.json), [원본 PNG](../assets/planet6897_20261004/7546-stage10/issue6639-original_review_p001.png), [정상 대조 PNG](../assets/planet6897_20261004/7546-stage10/issue6639-hancom_review_p001.png). 로그는 ignored `output/pr-review/planet6897-20261004/stage10-7546-nextest.log`에만 보존합니다.

### 병합 후 기여자 설명 계획

기여자께 모순 저장 사다리의 제한된 판별 조건과 정상 한컴 사다리 보존을 설명하고, 실제 통합 merge SHA·최신 CI·정본 링크·merge SHA 고정 PNG를 안내합니다. 현 단계에서는 원 PR을 닫거나 병합하지 않았습니다.
