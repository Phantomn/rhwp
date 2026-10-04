---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-05
---

# PR #7540 기여자 변경 검토

## 최종 판정

**승인** — KoPub 내장 독립 PDF 자산과 출처의 개별 범위와 통합 후보의 최종 로컬 검증을 완료했습니다. 원 PR 직접 병합이 아니라 이 문서에 기록한 체리픽 통합 후보를 대상으로 합니다. #7445 이관 문서의 전체 피델리티 수용을 주장하지 않습니다. 원격 통합 PR의 최신 head CI·보호 요건 통과 전에는 병합하지 않습니다.

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

| 항목 | 현재 근거와 범위 | 판정 |
| --- | --- | --- |
| 구현 근거와 일반성 | KoPub 내장 독립 PDF 자산·원장; renderer 변경 없음 | 개별 범위 확인 |
| 측정·배치 일관성 | 생산 배치 변경 없는 자산 기여 | 적용 안 됨 |
| 분할·이어받기 계약 | 자산/Studio crop 변경으로 일반 표 분할 경로 변경 없음 | 적용 안 됨 |
| 줄 소속과 점유 높이 | 자산/Studio crop 변경으로 줄 배치 경로 변경 없음 | 적용 안 됨 |
| 사례와 증거의 독립성 | 아래 통합 직접 실행·독립 한컴 PDF 또는 사양 및 입력 hash 참조 | 확인 |
| 기준값 변경 | 아래 기존 회귀 보정·유지 이유 참조; 출력 좌표를 새 정답으로 고정하지 않음 | 확인 |
| 주장과 검증 범위 | 개별 기여 범위 확인; 최종 전체 nextest·lint/build·통합 CI 완료 전 병합 불가 | 통합 보류 |

## 검증 입력 커밋 확인

아래 개별 단계의 실제 입력·독립 PDF·실행 head와 증적 JSON/manifest에 내용 hash를 기록했습니다. 기존 #7445 이관 자료는 해당 단계에 적은 범위만 유지하며 전체 피델리티 승인을 주장하지 않습니다. 최종 전체 검증 결과는 별도 완료 후 기록합니다.

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

## 최종 통합 로컬 검증 — 2026-10-05

전체 회귀 head `aa10d4093f60c4e0c12aa186f2464861acbf89ea`, 최종 lint/build·fresh WASM·Skia head `42ef60c96`(테스트 모듈 순서만 정리), base `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`입니다. 전체 nextest는 **10,318 PASS/0 FAIL/50 skip**,563.885초, threads=8로 완료했습니다. Format·Native/WASM32 Clippy·workspace build·workspace all-target Clippy·manifest/unit tier 정책을 통과했습니다. fresh WASM은 Mac 로컬 `--no-opt` 대체 빌드이며 Docker 최적화 검증으로 보고하지 않습니다. Skia lib·그림 placeholder·직접 PDF 출력 범위도 실제 실행했습니다. Studio 전체1,815 PASS/0 FAIL/2 skip와 TypeScript/Vite build도 완료했습니다.

[정확한 head·명령·결과·원 source CI·개별 시각 범위](../assets/planet6897_20261004/final_validation.json)를 참고합니다. 시각 자료는 각 단계의 생산 code head와 입력/PDF hash로 고정되어 있으며, 이후 변경은 테스트·문서 범위입니다. 최종 fresh WASM hash와 기존 단계의 hash, 생산 본문 동일성 증적을 함께 기록합니다. 대용량 미달 문서는 #7445의 기존·추가 이관 기록으로 추적하고 그 문서 전체의 정확성을 승인하지 않습니다. 위의 “통합 검증 미완료/대기”는 해당 단계 작성 시점의 기록이며 최종 로컬 판정은 이 절을 따릅니다. 통합 CI는 아직 미실행입니다.

## Merge 후 contributor PR comment 계획

기여자의 KoPub 내장 독립 PDF 자산과 출처 결과와 이 문서에 적은 추가 보정의 원인·범위를 한국어 존댓말로 구분해 설명합니다. 실제 통합 merge SHA·CI URL·Visual Sweep 정본과 이관 잔여 범위를 안내하고, 통합 대체 병합으로 원 PR을 close합니다. UTF-8 `--body-file`로 게시 후 API로 본문·close 상태를 재확인하며 중복 댓글은 게시하지 않습니다. 배치 변경 없는 자산/파일 속성 기여로 시각 렌더 승인 주장은 하지 않습니다.
