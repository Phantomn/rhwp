---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-04
---

# PR #7547 기여자 변경 검토

## 최종 판정

**머지 보류** — 체리픽 적용 후 통합 head의 focused·전체 회귀와 필수 시각/구조 검증 진행 중. 원 source의 CI·PNG를 통합 검증 통과로 취급하지 않습니다.

## 접수·범위

- 원 PR: https://github.com/edwardkim/rhwp/pull/7547
- 기여자: planet6897. 제목: 수정(조판): 나눔 표 일반 행을 쪽 경계에서 자르고, HWP5 문단 기준 표의 이어받은 조각에 바깥 위 여백을 연다 (#5585)
- 원 head: `ad09a2599907d782ac452b3abff91760698e0dab`. base: devel.
- 검토 branch: `review/planet6897-20261004`. 통합 base: `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`. 적용 단계 head: `fe0e6b9a772bd783d8c5b09bf05721d973e9e7d4`.
- 원 PR reviewer: jangster77 지정
- 경로: collaborator_external_pr 9.1.1; intake_and_review, local_validation, visual_fixture_evidence, multi_pr_update_branch, post_merge.
- 작성 시점 CI/mergeability는 참고값이며 수용·push 전에 최신 source head를 다시 확인합니다.

## 원 기여와 체리픽 출처

| 원 commit | 통합 commit/처리 | 비고 |
| --- | --- | --- |
| `0bcbe595176f8afef0225bec7bd1ac9751fae332` | `616208c36b3be8ed6876653654b77101c298cccd` | applied |
| `ad09a2599907d782ac452b3abff91760698e0dab` | `bc7664ba9e5c6c079cdc4a511e83cad37ff4928f` | applied |

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

## 단계1 실제 통합 검증과 보정 분석 (2026-10-04)

전체 focused 65개 중 #7547의 새 회귀 3개는 쪽수 1개 PASS / 내용 컷·종료 여백 2개 FAIL입니다. 전체 17쪽 Native/fresh WASM TSV는 15쪽 41.14622%, 16쪽 32.75887%만 미달이고 나머지 15쪽은 95.37132% 이상입니다. 원문·PDF·통합 출력의 쪽수는 17쪽입니다. 새 회귀의 기대값을 낮추거나 지우지 않았습니다.

- [전체 fresh WASM TSV](../assets/planet6897_20261004/7547-stage1/wasm_silhouette.tsv), [WASM export·빌드 provenance](../assets/planet6897_20261004/7547-stage1/wasm_export_manifest.json). Native와 동일한 두 미달을 재현했습니다. Mac 로컬 `--no-opt` 빌드이며 Docker 최적화 통과가 아닙니다.
- [전체 Native TSV](../assets/planet6897_20261004/7547-stage1/native_silhouette.tsv), [실행 환경](../assets/planet6897_20261004/7547-stage1/native_run_manifest.json), [전체 export 쪽수](../assets/planet6897_20261004/7547-stage1/native_export.json).
- [15쪽 review PNG](../assets/planet6897_20261004/7547-stage1/native_review_p015.png), [16쪽 review PNG](../assets/planet6897_20261004/7547-stage1/native_review_p016.png)를 직접 판독했습니다. 15쪽의 첫 빈 밴드가 없고 `【구글】` 행이 먼저 나타납니다. 16쪽의 표와 다음 문단도 위로 당겨집니다. 통합 출력의 문제로 보류합니다.
- 원문 pi=169 행 3 양쪽 셀 선언 높이는 각각 14721HU입니다. 첫 조각 높이 덮어쓰기(`source_complete_frame_last_row`)를 의심한 첫 시도는 두 실패를 해결하지 못해 코드 변경을 철회했습니다. 단계2는 35개 중 33 PASS / 동일 2 FAIL입니다.
- 추가 `RHWP_TABLE_DRIFT=1` 진단에서 측정·컷 행 높이 10개는 모두 일치합니다. 첫 조각 예산은 481.6px인데 저장 프레임 초과 허용 24.3px 때문에 495.8px의 전체 행을 받아 endCut 없이 끝납니다. 실제 paint는 첫 프레임에 맞추어 행 3을 178.9px로 잘라 남은 선언 밴드를 버립니다. 이 초과 수용이 신규 일반 행 컷보다 먼저 실행되는 것이 직접 원인입니다.
- 보정 계획: 기존 `ordinary_band_row_shape`로 판별하는 비병합·글줄만 포함한 선언 일반 행은 저장 프레임 초과 허용으로 통째 수용하지 않고 기존 내용 컷 판정에 맡깁니다. 실제 예산에 들어가는 전체 행, 중첩 개체·rowspan·선언보다 큰 내용은 기존 경로를 유지합니다. 파일명·쪽 번호 분기는 추가하지 않습니다.
- 보정 후 #5585와 기존 #6123·#2439·#7379·#7234 경계 회귀를 실행하고 전체 17쪽 Native/fresh WASM TSV 및 대표 PNG를 재산출합니다. 아직 보정 후 결과와 수용 여부는 미확정입니다.

## 단계3·4 결과와 정상 반례 보류

- 초과 수용보다 일반 밴드 컷을 우선한 단계3: 기본 관련 35/35 PASS. Native 17쪽 중 16쪽은 98.90594%로 복구됐지만 15쪽은 69.32327%로 여전히 미달입니다. 대표 PNG를 직접 확인했고 앞의 blank band와 뒤 모든 행이 기준보다 아래로 밀립니다. 검사 통과를 시각 완료로 표시하지 않습니다.
- 추가 정상 반례 #1133(HWP/HWPX 각각 3쪽): 보정 전 fresh WASM 전쪽 최저 97.99746%/98.00378%, 3쪽 100%입니다. 단계3 Native는 두 벌 모두 3쪽 39.49135%로 악화됐고 기존 내용 조각 계약도 깨집니다. 이 회귀는 정상 기준이 있어 삭제하지 않습니다.
- 단계4에서 전체 내용의 높이·정렬 후 끝이 예산에 들어가는 조건도 확인했으나 39개 중 37 PASS / #1133 2 FAIL입니다. source 파일 hash와 측정 범위는 [단계4 결과](../assets/planet6897_20261004/7547-stage4/results.json)에 기록했습니다. 아직 **메인터너 보정 미완료 / 머지 보류**입니다.
- 다음 분석: 위 정렬의 빈 꼬리 밴드와 가운데 정렬의 닫힌 원본 프레임은 같은 높이 축소로 취급할 수 없습니다. 정상 반례의 원본 정렬·물리 프레임 소유를 보존하고 #6929의 빈 밴드도 실제 첫 조각과 동일한 높이로 이어받게 합니다. 코드 보정 전후의 Native/fresh WASM을 다시 비교합니다.
