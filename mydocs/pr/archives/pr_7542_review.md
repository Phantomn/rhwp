---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-04
---

# PR #7542 기여자 변경 검토

## 최종 판정

**통합 머지 보류 / #7542 자체 구조 계약은 통과** — 통합 head에서 독립 한컴 쌍둥이 입력과 h2x·x2h·x2x 왕복 4개가 모두 통과했습니다. 전체 검증과 다른 원 PR의 렌더링 보류가 남아 있어 아직 승인·병합하지 않습니다.

## 접수·범위

- 원 PR: https://github.com/edwardkim/rhwp/pull/7542
- 기여자: planet6897. 제목: fix(hwpx): 문단 "한 줄로 입력"(lineWrap) 을 HWP5 attr2 bits 0-1 로 왕복한다 (#6875)
- 원 head: `9db08a6698b9d124d74836122859874d1730ed4b`. base: devel.
- 검토 branch: `review/planet6897-20261004`. 통합 base: `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`. 적용 단계 head: `fe0e6b9a772bd783d8c5b09bf05721d973e9e7d4`.
- 원 PR reviewer: jangster77 지정
- 경로: collaborator_external_pr 9.1.1; intake_and_review, local_validation, visual_fixture_evidence, multi_pr_update_branch, post_merge.
- 작성 시점 CI/mergeability는 참고값이며 수용·push 전에 최신 source head를 다시 확인합니다.

## 원 기여와 체리픽 출처

| 원 commit | 통합 commit/처리 | 비고 |
| --- | --- | --- |
| `9db08a6698b9d124d74836122859874d1730ed4b` | `cfde24c5a9b7acc5cd5aec71f94ae9af5afed5a0` | applied |

## 구현 검토와 독립 근거

HWP5 명세의 ParaShape `attr2` bit 0~1(한 줄로 입력)과 한컴이 저장한 HWP/HWPX 쌍둥이를 대조했습니다. 파서는 해당 두 비트만 덮어쓰며 나머지 비트를 보존하고, 직렬화기는 동일 비트를 BREAK/SQUEEZE/KEEP으로 내보냅니다. 미정 값은 BREAK로 처리합니다. 새 회귀는 좌표가 아닌 문단 모양 id와 파일 저장 후 속성 보존을 검사합니다.

- [HWP5 명세](../../tech/한글문서파일형식_5.0_revision1.3.md)의 한 줄로 입력 필드.
- 쌍둥이의 문단 모양 358개 중 SQUEEZE id 117·120·121·156의 동일성 및 세 저장 경로: **4/4 PASS**.
- renderer는 문단 ParaShape의 해당 비트를 직접 소비하지 않습니다. 이 PR의 판정 범위는 파일 속성 보존입니다. 한컴에서 다시 조판한 쪽수나 시각 정확도는 별도로 입증한 것으로 표시하지 않습니다.

| 항목 | 근거와 범위 | 판정 |
| --- | --- | --- |
| 구현 근거와 일반성 | 사양 비트와 한컴 저장 쌍둥이, enum 양방향 대응 | 확인 |
| 측정·배치 일관성 | 문단 lineWrap의 측정/배치 소비 변경 없음 | 적용 안 됨 |
| 분할·이어받기 계약 | 저장 속성 보존만 변경; 분할 경로 변경 없음 | 적용 안 됨 |
| 줄 소속과 점유 높이 | 현재 renderer의 줄 소속을 변경하지 않음 | 적용 안 됨 |
| 사례와 증거의 독립성 | 한컴 원본 두 벌의 전제 및 재파싱 검사 통과 | 확인 |
| 기준값 변경 | 출력 좌표 기준값 변경 없음 | 확인 |
| 주장과 검증 범위 | 네 계약 통과; 전체 통합 검증 미완료 | 통합 보류 |

## 검증 입력과 실제 결과

두 입력 `samples/hwpx/mel-001.hwpx`, `samples/hwpx/hancom-hwp/mel-001.hwp`는 이미 커밋되어 있습니다. 검증 head·입력 SHA-256·실행 명령·전체 focused 실패 목록은 [단계1 검증 증거](../assets/planet6897_20261004/stage1_focused_validation.json)에 기록했습니다. 로그는 ignored `output/pr-review/planet6897-20261004/focused-nextest.log`에만 보관합니다.

관련 묶음 결과는 65개 중 63 PASS / 2 FAIL이며, 실패 두 건은 #7547의 표 분할 계약입니다. #7542 자체 네 검사는 모두 통과했습니다. 새 회귀 또는 메인터너 코드 보정은 추가하지 않았습니다.

## 다음 단계와 contributor 후속 계획

통합 전체 검증 완료 후 최종 수용 판정을 갱신합니다. 병합 후에는 기여자의 파일 속성 왕복 보정이 한컴 독립 입력 네 계약을 통과했음을 한국어 존댓말로 원 PR에 설명하고 정확한 통합 merge SHA를 안내합니다. 현재 comment·close·통합 merge는 수행하지 않았습니다.
