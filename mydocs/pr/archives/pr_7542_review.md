---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-05
---

# PR #7542 기여자 변경 검토

## 최종 판정

**승인** — lineWrap 파일 속성 세 경로 왕복 보존의 개별 범위와 통합 후보의 최종 로컬 검증을 완료했습니다. 원 PR 직접 병합이 아니라 이 문서에 기록한 체리픽 통합 후보를 대상으로 합니다. #7445 이관 문서의 전체 피델리티 수용을 주장하지 않습니다. 원격 통합 PR의 최신 head CI·보호 요건 통과 전에는 병합하지 않습니다.

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

## 최종 통합 로컬 검증 — 2026-10-05

전체 회귀 head `aa10d4093f60c4e0c12aa186f2464861acbf89ea`, 최종 lint/build·fresh WASM·Skia head `42ef60c96`(테스트 모듈 순서만 정리), base `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`입니다. 전체 nextest는 **10,318 PASS/0 FAIL/50 skip**,563.885초, threads=8로 완료했습니다. Format·Native/WASM32 Clippy·workspace build·workspace all-target Clippy·manifest/unit tier 정책을 통과했습니다. fresh WASM은 Mac 로컬 `--no-opt` 대체 빌드이며 Docker 최적화 검증으로 보고하지 않습니다. Skia lib·그림 placeholder·직접 PDF 출력 범위도 실제 실행했습니다. Studio 전체1,815 PASS/0 FAIL/2 skip와 TypeScript/Vite build도 완료했습니다.

[정확한 head·명령·결과·원 source CI·개별 시각 범위](../assets/planet6897_20261004/final_validation.json)를 참고합니다. 시각 자료는 각 단계의 생산 code head와 입력/PDF hash로 고정되어 있으며, 이후 변경은 테스트·문서 범위입니다. 최종 fresh WASM hash와 기존 단계의 hash, 생산 본문 동일성 증적을 함께 기록합니다. 대용량 미달 문서는 #7445의 기존·추가 이관 기록으로 추적하고 그 문서 전체의 정확성을 승인하지 않습니다. 위의 “통합 검증 미완료/대기”는 해당 단계 작성 시점의 기록이며 최종 로컬 판정은 이 절을 따릅니다. 통합 CI는 아직 미실행입니다.

## Merge 후 contributor PR comment 계획

기여자의 lineWrap 파일 속성 세 경로 왕복 보존 결과와 이 문서에 적은 추가 보정의 원인·범위를 한국어 존댓말로 구분해 설명합니다. 실제 통합 merge SHA·CI URL·Visual Sweep 정본과 이관 잔여 범위를 안내하고, 통합 대체 병합으로 원 PR을 close합니다. UTF-8 `--body-file`로 게시 후 API로 본문·close 상태를 재확인하며 중복 댓글은 게시하지 않습니다. 배치 변경 없는 자산/파일 속성 기여로 시각 렌더 승인 주장은 하지 않습니다.
