---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-05
---

# PR #7564 기여자 변경 검토

## 최종 판정

**개별 변경 범위 검증 통과·통합 머지 보류** — 공개 한컴 저장 합성 입력의 기존 관계 검사 3개와 Native/fresh WASM 1쪽을 직접 검토했습니다. 나머지 원 PR 및 통합 최종 검증은 진행 중입니다.

## 접수·범위

- 원 PR: https://github.com/edwardkim/rhwp/pull/7564
- 기여자: planet6897. 제목: fix(layout): 쪽 기준 어울림 표를 본문 영역 기준 절대 위치에 두고 흐름에 표 높이를 예약하지 않는다 (#7548 2단계, #7557 위)
- 원 head: `bb1dacd33f02d828ef6581e7a935ffa2767187a8`. base: devel.
- 검토 branch: `review/planet6897-20261004`. 통합 base: `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`. 적용 단계 head: `fe0e6b9a772bd783d8c5b09bf05721d973e9e7d4`.
- 원 PR reviewer: jangster77 지정
- 경로: collaborator_external_pr 9.1.1; intake_and_review, local_validation, visual_fixture_evidence, multi_pr_update_branch, post_merge.
- 작성 시점 CI/mergeability는 참고값이며 수용·push 전에 최신 source head를 다시 확인합니다.

## 원 기여와 체리픽 출처

| 원 commit | 통합 commit/처리 | 비고 |
| --- | --- | --- |
| `da761bf8e6ee6c374b831cd79723654abbd826d9` | `b99dc65f9d6d845405c22acf7d0df815438cd0e1` | 저장 줄 없는 Square host 공통 결과 우선, 저장 쪽·종이 앵커 후속 분기에 본문만 전진 |
| `8563efc451c3ba13ae1b0d182dfea6842aae69c1` | `02964fa558b76efecd9a4ae427ee23ede2943b16` | applied |
| `bb1dacd33f02d828ef6581e7a935ffa2767187a8` | `fe0e6b9a772bd783d8c5b09bf05721d973e9e7d4` | applied |

## 조판 원칙과 검증 현황

| 항목 | 현재 근거와 범위 | 판정 |
| --- | --- | --- |
| 구현 근거와 일반성 | 본문 host의 PAGE/PAPER Square 위치와 흐름 비전진 | 개별 범위 확인 |
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

- 생산 후보 `4cfe96e52`, 실행 head `c393aa1f4`. 각 manifest에 실행 바이너리·입력·PDF·글꼴·스크립트 해시를 보존했습니다. stage8 로컬 WASM 대체 빌드이며 Docker 최적화·실제 Studio Canvas 검증은 아닙니다.
- `samples/page_anchored_square/page_anchored_square.hwp`와 독립 한컴 PDF `pdf/page_anchored_square/page_anchored_square-2020.pdf` 전체 1쪽: Native/fresh WASM 모두 **99.87139%**, 미달·누락 0입니다. 합성 입력을 한컴으로 저장해 얻은 LineSeg·PDF 근거는 [기존 자료](../assets/issue_7548_page_anchored_square/README.md)와 대조했습니다.
- review PNG를 직접 열어 host 뒤 문단 4~7이 표 위에 있고, 표 띠와 겹치는 문단 8 이후는 아래에 있는지 확인했습니다. 표 괘선의 작은 차이는 남지만 본문 순서·개수와 표 내용의 누락·중복은 보이지 않습니다. 엄격 화소 동일성을 주장하지 않습니다.
- `regression_suite_006`의 `issue_7548_page_anchored_square` 기존 **3/3 PASS**. 표 상단은 본문 상단 + 입력의 PAGE vertOffset(13000HU), 다음 문단은 host와의 저장 vpos 간격, 표 띠 위/아래 소속을 검사합니다. 절대 렌더 좌표를 고정하지 않아 검사나 기대값 변경 없이 유지했습니다.
- 코드 대조에서 저장 줄 없는 Square host의 기존 공통 flow plan이 먼저 실행되는 조건을 보존했습니다. 저장 Page/Paper 기준 표는 layout에서 절대 위치를 사용하고 typeset에서는 host 본문 높이만 전진하며 표 높이를 재예약하지 않습니다. 문단 기준 저장 차선 분기는 뒤에 유지됩니다. 빈 host 전용이던 경로의 visible host 확장이며 소스 사례 식별자로 갈라지지 않습니다.
- 선행 #7557의 source 4개는 한 번만 적용했으며 #7564 고유 source 3개만 추가했습니다. 비공개 `36295751` 1·2쪽과 `156617659` 12쪽의 기여자 점수를 이번 공개 입력 직접 검증으로 확대하지 않습니다. 선행 #7557 실제 차선 문서 검토는 별도 진행합니다.
- [결과](../assets/planet6897_20261004/7564-stage12/results.json), [검토 PNG](../assets/planet6897_20261004/7564-stage12/native_review_p001.png). 로그는 ignored `output/pr-review/planet6897-20261004/stage12-7564-nextest.log`입니다.

### 병합 후 기여자 설명 계획

기여자께 쪽 기준 표의 절대 배치와 본문 흐름 비예약 관계, 선행 PR 중복 적용을 피한 출처를 설명드립니다. 통합 merge SHA·최신 CI·정본 링크·merge SHA 고정 합성 PNG를 안내합니다. 현 단계는 실제 원 PR 병합·close가 아닙니다.
