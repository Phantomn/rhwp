---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-04
---

# PR #7543 기여자 변경 검토

## 최종 판정

**머지 보류** — 체리픽 적용 후 통합 head의 focused·전체 회귀와 필수 시각/구조 검증 진행 중. 원 source의 CI·PNG를 통합 검증 통과로 취급하지 않습니다.

## 접수·범위

- 원 PR: https://github.com/edwardkim/rhwp/pull/7543
- 기여자: planet6897. 제목: 수정(layout): 어울림 표 꼬리 레인 오판과 음수 줄간격 TAC 줄 전진 — 156714641 1쪽 71%→99% (#4599)
- 원 head: `62332ec416efdbec603d85e5b98d377eec9c2083`. base: devel.
- 검토 branch: `review/planet6897-20261004`. 통합 base: `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`. 적용 단계 head: `fe0e6b9a772bd783d8c5b09bf05721d973e9e7d4`.
- 원 PR reviewer: jangster77 지정
- 경로: collaborator_external_pr 9.1.1; intake_and_review, local_validation, visual_fixture_evidence, multi_pr_update_branch, post_merge.
- 작성 시점 CI/mergeability는 참고값이며 수용·push 전에 최신 source head를 다시 확인합니다.

## 원 기여와 체리픽 출처

| 원 commit | 통합 commit/처리 | 비고 |
| --- | --- | --- |
| `232585fcbee4cdba16cd4897bf8ef7836c8075c4` | `ba569811dc0e4b73419a6ffa758c25b6c0ae50e4` | applied |
| `2b9a9fffaab161a89665df315c9d015a1ea51406` | `ef0ac05cf92e641e5d3937a11a0a2aef0fca92eb` | applied |
| `62332ec416efdbec603d85e5b98d377eec9c2083` | `f9f3d2dfbda154e6f3756fa740aa8ec43559e71b` | applied |

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

## 직접 통합 검증 — 개별 범위 충족 / 통합 전체 검증 대기

code candidate `c8c5f535cd3ebf331cadc2714c68af9fdc663a4d`와 생산 코드가 동일한 현재 head에서 기존 관계 회귀 2/2 PASS를 확인했습니다. 새 회귀를 추가하거나 절대 좌표로 기대값을 낮추지 않았습니다.

- 입력은 기여자가 제공한 1쪽 최소 재현 `samples/issue4599/156714641_wrap_tail_min.hwpx`입니다. 원본 5쪽 문서 전체의 시각 검증으로 확대 주장하지 않습니다. 최소 재현 그림은 source 자체가 작은 대체 그림이며 비교 도구가 원본 그림을 가린 것이 아닙니다.
- 기준은 이번에 한컴 2020 MCP로 독립 변환한 [최종 PDF](../../../pdf/issue4599/156714641_wrap_tail_min-2020.pdf)입니다. [변환 provenance](../assets/planet6897_20261004/7543-stage6/hancom_conversion.json)와 [입력 hash·검사 결과](../assets/planet6897_20261004/7543-stage6/results.json)를 보존했습니다. 한컴 11.0.0.9136 / PDF 드라이버 one-up / 1쪽, job `7d093cec-89d7-4483-8bd0-1114cfe468af`, PDF SHA-256 `d6fff0fcf4c1aa2d72b71f10f5996781e8fda077e3c608f4d4f52b3a647bd0b1`입니다.
- 전체 1쪽 Native/fresh WASM TSV는 둘 다 **99.41288%**, 미달·누락 0입니다. 원문·PDF·두 출력 쪽수가 모두 1쪽입니다. Mac 로컬 대체 WASM이며 Docker 최적화 검증은 아닙니다. [Native TSV](../assets/planet6897_20261004/7543-stage6/native_silhouette.tsv), [WASM TSV](../assets/planet6897_20261004/7543-stage6/wasm_silhouette.tsv).
- [review PNG](../assets/planet6897_20261004/7543-stage6/native_review_p001.png)와 fresh WASM raster를 직접 열었습니다. 머리 표 뒤 본문 위치, 어울림 표 오른쪽 접두 7줄과 전폭 꼬리 2줄, 다음 문단 순서가 유지되고 겹침·누락이 없습니다. 엄격 픽셀 값 37.13%와 관용 실루엣 99.41%를 구분합니다. 원본 임베딩 그림의 보간과 글꼴 잉크 차이는 남지만 줄 소유·흐름 결함은 재현되지 않습니다. 라벨·수치도 판독 가능합니다.
- 기여자의 옆 띠/전폭 꼬리 분리와 음수 줄간격 TAC host 수정은 독립 PDF 및 관계 회귀로 **개별 범위를 충족**합니다. 별도 메인터너 생산 코드 보정은 하지 않았으며 최종 통합 검사 전 머지 보류를 유지합니다.
- merge 후 설명에는 원 기여의 두 축과 정확한 CI/merge SHA, 위 1쪽 PNG·정본 링크를 기록합니다. 아직 원 PR에 게시·close하지 않았습니다.

## 공백 잉크 보정 뒤 재확인

후속 생산 보정 `4cfe96e52`의 Native/fresh WASM에서도 이 문서와 정상 반례의 전체 TSV가 90% 이상으로 유지됩니다. 현재 32쪽 통합 시각 실행은 [단계8 결과](../assets/planet6897_20261004/7534-stage8/results.json)에 입력/build 출처와 함께 기록했습니다. 기존 개별 범위 충족 판정은 유지하며 최종 전체 검사·다른 PR 검토는 대기입니다.
