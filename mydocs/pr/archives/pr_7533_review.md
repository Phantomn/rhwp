---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-05
---

# PR #7533 기여자 변경 검토

## 최종 판정

**머지 보류** — 체리픽 적용 후 통합 head의 focused·전체 회귀와 필수 시각/구조 검증 진행 중. 원 source의 CI·PNG를 통합 검증 통과로 취급하지 않습니다.

## 접수·범위

- 원 PR: https://github.com/edwardkim/rhwp/pull/7533
- 기여자: planet6897. 제목: fixture: issue5780 기준 PDF 를 한컴 2020 출력으로 재생성 (#7352)
- 원 head: `42e81807b575c5edcff3d23c6d81e204da770bbd`. base: devel.
- 검토 branch: `review/planet6897-20261004`. 통합 base: `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`. 적용 단계 head: `fe0e6b9a772bd783d8c5b09bf05721d973e9e7d4`.
- 원 PR reviewer: jangster77 지정
- 경로: collaborator_external_pr 9.1.1; intake_and_review, local_validation, visual_fixture_evidence, multi_pr_update_branch, post_merge.
- 작성 시점 CI/mergeability는 참고값이며 수용·push 전에 최신 source head를 다시 확인합니다.

## 원 기여와 체리픽 출처

| 원 commit | 통합 commit/처리 | 비고 |
| --- | --- | --- |
| `42e81807b575c5edcff3d23c6d81e204da770bbd` | `f04ac1f5e89a5fddb6927ff2fa58cddfb24f0cfb` | applied |

## 조판 원칙과 검증 현황

| 항목 | 현재 근거와 범위 | 판정 |
| --- | --- | --- |
| 구현 근거와 일반성 | 독립 한컴 PDF 자산 교체·출처; renderer 변경 없음 | 개별 범위 확인 |
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

## 기준 PDF 제자리 교체 검토 — 개별 범위 충족

- source commit `42e81807b`의 PDF와 통합 파일의 바이트가 일치합니다. SHA-256 `98447f3c11fc586cef42ba46f263d1f23406aa070d05d2a2d38b19b75665ae5d`, 12,497바이트, 1쪽, 595×841pt입니다. creator `Hwp 2020 0.0.0.0`, producer `Hancom PDF 1.3.0.550`, H2hdrM 글꼴 내장을 실제 파일에서 확인했습니다. [원문 hash·PDF metadata·source 일치](../assets/planet6897_20261004/7533-provenance/results.json)를 기록했습니다.
- [기준 1쪽 raster](../assets/planet6897_20261004/7533-provenance/reference_p001.png)를 직접 열어 짙은 쪽 배경, 제목/기간/날짜 3줄과 하단 흰 상자가 정상임을 확인했습니다. 유효 한컴 PDF의 독립 기준 교체이며 Native renderer 개선·시각 점수 검증으로 확대하지 않습니다.
- tests/src/scripts의 runtime 인용·회귀 기대값 핀은 없습니다. `tools/oracle_public`의 과거 감사 catalog에도 이 경로가 있으나 당시 크기를 담은 snapshot으로, 이번 실행 증적이나 새 PDF hash 핀으로 사용하지 않습니다.
- 생산 코드·회귀 변경이 없는 이 기여의 **기준 PDF 자산 교체 범위는 충족**합니다. 최종 통합 검증 대기이며 #7352의 다른 PDF 감사까지 완료하거나 관련 issue를 닫은 것으로 쓰지 않습니다.
- merge 후 contributor 설명에는 잘못 명명된 cairo 기준을 유효 한컴 출력으로 바꾼 기여와 정확한 merge SHA를 기록합니다. 아직 comment/close를 수행하지 않았습니다.
