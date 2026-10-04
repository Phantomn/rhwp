---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-05
---

# PR #7557 기여자 변경 검토

## 최종 판정

**메인터너 보정 후 수용 가능** — Square 표 차선과 저장 후속 줄의 개별 범위와 통합 후보의 최종 로컬 검증을 완료했습니다. 원 PR 직접 병합이 아니라 이 문서에 기록한 체리픽 통합 후보를 대상으로 합니다. #7445 이관 문서의 전체 피델리티 수용을 주장하지 않습니다. 원격 통합 PR의 최신 head CI·보호 요건 통과 전에는 병합하지 않습니다.

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

| 항목 | 현재 근거와 범위 | 판정 |
| --- | --- | --- |
| 구현 근거와 일반성 | Square 표 옆 차선의 저장 줄 배치·후속 흐름 | 개별 범위 확인 |
| 측정·배치 일관성 | 아래 저장 사양·내용 소유 및 직접 결과의 범위로 확인 | 개별 범위 확인 |
| 분할·이어받기 계약 | 아래 개별 구조 검사·시각 결과에 한정; 전체 corpus 수용 주장 없음 | 개별 범위 확인 |
| 줄 소속과 점유 높이 | 아래 저장 줄·개체 소유 및 정상 대조 검증 범위에 한정 | 개별 범위 확인 |
| 사례와 증거의 독립성 | 아래 통합 직접 실행·독립 한컴 PDF 또는 사양 및 입력 hash 참조 | 확인 |
| 기준값 변경 | 아래 기존 회귀 보정·유지 이유 참조; 출력 좌표를 새 정답으로 고정하지 않음 | 확인 |
| 주장과 검증 범위 | 개별 범위와 최종 로컬 검증 통과; 최신 head 통합 CI 완료 전 병합 불가 | 통합 CI 대기 |

## 검증 입력 커밋 확인

아래 개별 단계의 실제 입력·독립 PDF·실행 head와 증적 JSON/manifest에 내용 hash를 기록했습니다. 기존 #7445 이관 자료는 해당 단계에 적은 범위만 유지하며 전체 피델리티 승인을 주장하지 않습니다. 최종 전체 검증 결과는 별도 완료 후 기록합니다.

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

## 최종 통합 로컬 검증 — 2026-10-05

전체 회귀 head `aa10d4093f60c4e0c12aa186f2464861acbf89ea`, 최종 lint/build·fresh WASM·Skia head `42ef60c96`(테스트 모듈 순서만 정리), base `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`입니다. 전체 nextest는 **10,318 PASS/0 FAIL/50 skip**,563.885초, threads=8로 완료했습니다. Format·Native/WASM32 Clippy·workspace build·workspace all-target Clippy·manifest/unit tier 정책을 통과했습니다. fresh WASM은 Mac 로컬 `--no-opt` 대체 빌드이며 Docker 최적화 검증으로 보고하지 않습니다. Skia lib·그림 placeholder·직접 PDF 출력 범위도 실제 실행했습니다. Studio 전체1,815 PASS/0 FAIL/2 skip와 TypeScript/Vite build도 완료했습니다.

[정확한 head·명령·결과·원 source CI·개별 시각 범위](../assets/planet6897_20261004/final_validation.json)를 참고합니다. 시각 자료는 각 단계의 생산 code head와 입력/PDF hash로 고정되어 있으며, 이후 변경은 테스트·문서 범위입니다. 최종 fresh WASM hash와 기존 단계의 hash, 생산 본문 동일성 증적을 함께 기록합니다. 대용량 미달 문서는 #7445의 기존·추가 이관 기록으로 추적하고 그 문서 전체의 정확성을 승인하지 않습니다. 위의 “통합 검증 미완료/대기”는 해당 단계 작성 시점의 기록이며 최종 로컬 판정은 이 절을 따릅니다. 통합 CI는 아직 미실행입니다.

## Merge 후 contributor PR comment 계획

기여자의 Square 표 차선과 저장 후속 줄 결과와 이 문서에 적은 추가 보정의 원인·범위를 한국어 존댓말로 구분해 설명합니다. 실제 통합 merge SHA·CI URL·Visual Sweep 정본과 이관 잔여 범위를 안내하고, 통합 대체 병합으로 원 PR을 close합니다. UTF-8 `--body-file`로 게시 후 API로 본문·close 상태를 재확인하며 중복 댓글은 게시하지 않습니다. [대표 PNG](../assets/planet6897_20261004/7557-stage13/native_review_p014.png)는 실제 merge SHA에 고정한 raw URL로 표시합니다.

### 최종 출력 provenance 재확인

최종 Native CLI(`native-skia`)와 실제 Chrome/fresh WASM으로 수용 범위20개 target·104쪽씩을 다시 내보냈습니다. **두 경로 최저90.80846%,90% 미만0쪽**입니다. SVG가 byte-identical인 쪽의 raster만 재사용하고, 달라진 SVG·글꼴 정책 블록은 새 raster로 비교했습니다. [전쪽 수용 범위 TSV·입력/PDF hash·현재 SVG hash와 재사용/재출력 구분](../assets/planet6897_20261004/final-visual/results.json). #7445 이관 문서의 전체 수용 주장은 아닙니다. 자산/속성 기여는 원래의 제한된 검토 범위를 유지하며 Studio crop은 동일한 최종 WASM의 실제 Canvas2D/CanvasKit 증적을 유지합니다.
