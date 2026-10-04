---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-05
---

# PR #7529 기여자 변경 검토

## 최종 판정

**메인터너 보정 후 수용 가능** — 그림 자르기 범위는 두 실제 Studio 백엔드와 독립 PDF로 확인했습니다. 최종 전체 회귀·lint/build·통합 CI는 아직 대기합니다.

## 접수·범위

- 원 PR: https://github.com/edwardkim/rhwp/pull/7529
- 기여자: planet6897. 제목: 수정(studio): 그림 자르기 적응 배율을 #7015 와 같이 축별로 판정한다 (#7525)
- 원 head: `1f71898331bb26978cf5bfe91899815e74de7e4d`. base: devel.
- 검토 branch: `review/planet6897-20261004`. 통합 base: `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`. 적용 단계 head: `fe0e6b9a772bd783d8c5b09bf05721d973e9e7d4`.
- 원 PR reviewer: jangster77 지정
- 경로: collaborator_external_pr 9.1.1; intake_and_review, local_validation, visual_fixture_evidence, multi_pr_update_branch, post_merge.
- 작성 시점 CI/mergeability는 참고값이며 수용·push 전에 최신 source head를 다시 확인합니다.

## 원 기여와 체리픽 출처

| 원 commit | 통합 commit/처리 | 비고 |
| --- | --- | --- |
| `7416b14bccd8520bea37a2c9946db6a768ba94d6` | `d4886808036b53a8cc26e2d43a8c3855c3d87200` | applied |
| `1f71898331bb26978cf5bfe91899815e74de7e4d` | `2b21d2474796b28fb36dcc1ab541dd35a0432fa7` | applied |

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

## Stage21 — Studio 그림 자르기 직접 검증 (2026-10-05)

- 원 기여의 축별 fallback을 유지했습니다. `cropReferenceSize` 두 축 우선, 시작이 0인 축의 전체 범위 확인, 둘 다 잘린 경우 기본 HWPUNIT 환산 순서를 코드와 실제 영상으로 대조했습니다. 일반 page flow/분할 계약은 이 TS helper 변경 범위에 해당하지 않습니다.
- 기존 회귀 1개의 고정 영상 좌표 기대값을 원본 자르기 HWPUNIT와 디코딩 크기의 관계로 보정했습니다. 같은 검사에서 잘못된 세로 축척을 잡으며 새 test 함수는 추가하지 않았습니다. 관련 `render-backend.test.ts` 65개 PASS/0 FAIL입니다.
- 검증 입력: `samples/issue7015/30442-acrc-recommendation-business-burden.hwp`, `samples/issue6866/156627451-quantum-science-press-note.hwpx`; 독립 기준은 `pdf/30442-acrc-recommendation-business-burden-2020.pdf`, `pdf/156627451-quantum-science-press-note-2020.pdf`입니다. 내용 SHA와 실제 renderer diagnostics는 아래 증적에 보존합니다.
- 통합 head `a9f4b0850`의 Studio + 생산 Rust/WASM `072048a8a`로 실제 Chrome headless/DPR 1/zoom 1/print profile에서 Canvas2D·CanvasKit software를 각각 요청했고 fallback 없이 해당 backend가 실행된 것을 기록했습니다.

| Studio backend | 30442 3쪽 | 30442 14쪽 | 정상 대조군 1쪽 |
| --- | ---: | ---: | ---: |
| Canvas2D | 99.11316% | 99.22282% | 98.90923% |
| CanvasKit | 95.73803% | 98.61356% | 97.90132% |

전체 페이지 레이어를 합성한 review PNG를 직접 확인했습니다. 3쪽 기관 로고 전체, 14쪽 유리 사진 두 장, 정상 대조군 오른쪽 로고가 PDF와 같은 영상입니다. 초기 주 canvas만 `toDataURL`로 저장한 것은 Canvas2D 별도 레이어를 누락하므로 검증 자료에서 제외하고 재캡처했습니다. CanvasKit의 일부 목록 번호·글꼴 굵기 차이는 crop 변경 밖의 기존 잔여이며 문서 전체 렌더링 승인을 주장하지 않습니다.

[검증 결과·입력 SHA](../assets/planet6897_20261004/7529-stage21/results.json) · [실제 backend diagnostics](../assets/planet6897_20261004/7529-stage21/capture.json) · [Canvas2D 14쪽 review](../assets/planet6897_20261004/7529-stage21/canvas2d/acrc/review/review_014.png) · [CanvasKit 14쪽 review](../assets/planet6897_20261004/7529-stage21/canvaskit/acrc/review/review_014.png).

### Merge 후 contributor PR comment 계획

최종 통합 merge SHA/CI와 merge SHA에 고정한 위 PNG 링크로 원 기여의 자르기 수정 결과와 메인터너의 고정 좌표 회귀 보정 이유를 한국어 존댓말로 설명합니다. 이 단계에서는 contributor PR을 close하거나 merge했다고 기록하지 않습니다.
