---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-04
---

# PR #7545 기여자 변경 검토

## 최종 판정

**머지 보류** — 체리픽 적용 후 통합 head의 focused·전체 회귀와 필수 시각/구조 검증 진행 중. 원 source의 CI·PNG를 통합 검증 통과로 취급하지 않습니다.

## 접수·범위

- 원 PR: https://github.com/edwardkim/rhwp/pull/7545
- 기여자: planet6897. 제목: fix(layout): 셀 내용 폭을 한/글 저장 격자(4 HWPUNIT)로 내려 줄을 나눈다 (#7412)
- 원 head: `044027a9030f6893b8369be0e64efbb0ec410d57`. base: devel.
- 검토 branch: `review/planet6897-20261004`. 통합 base: `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`. 적용 단계 head: `fe0e6b9a772bd783d8c5b09bf05721d973e9e7d4`.
- 원 PR reviewer: jangster77 지정
- 경로: collaborator_external_pr 9.1.1; intake_and_review, local_validation, visual_fixture_evidence, multi_pr_update_branch, post_merge.
- 작성 시점 CI/mergeability는 참고값이며 수용·push 전에 최신 source head를 다시 확인합니다.

## 원 기여와 체리픽 출처

| 원 commit | 통합 commit/처리 | 비고 |
| --- | --- | --- |
| `044027a9030f6893b8369be0e64efbb0ec410d57` | `964829525d8c6ea61a31d0f64a3e90f6afa85cbe` | layout_frame::origin_is_authoritative 필드 보존; rows 주석은 셀 폭 격자 근거로 갱신 |

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

## Stage 16 메인터너 보정 — HWPX 구판 여백 단위와 첫 조각 원점

**계속 보류**입니다. 사용자 요구에 따라 HWP 121쪽은 원 PR 정본 93.56018% 이상, HWPX 121쪽은 90% 이상을 해제 조건으로 유지합니다. HWPX를 #7445로 이관하거나 회귀를 삭제하지 않습니다.

- 동일 HWPX를 한컴 engine 2020으로 PDF와 HWP로 각각 변환했습니다. 새 PDF도 156쪽이며, HWP 재저장의 문단 내어쓰기 값은 xmlVersion 1.2 case 값과 같습니다. 현재 파서가 여기에 최신 판본의 2배 정규화를 적용하던 단위 오류를 보정했습니다. 직렬화도 같은 패키지 판본의 역단위를 적용합니다.
- 저장 줄 없는 표 전용 호스트의 첫 조각은 예산에 예약된 앞 여백·오프셋을 확정 배치 원점으로 전달합니다. 픽셀 보정 상수를 추가하지 않았습니다.
- 기존 여백 왕복 검사 2개와 #7412 줄 소속 검사 2개: 4 PASS / 0 FAIL. 검사 수는 늘리지 않았습니다. HWPX의 규제 개요는 독립 PDF 120쪽에 대응하므로 기존의 잘못된 121쪽 전제를 바로잡았습니다.
- 새 WASM은 로컬 대체 빌드입니다. Native/fresh WASM HWP 121쪽은 91.73087%, HWPX 121쪽은 86.09215%입니다. 두 값 모두 이번 사용자 해제 조건을 충족하지 못했습니다.
- HWPX 출력과 PDF는 이제 모두 156쪽이지만, 전체 156쪽 TSV에는 4쪽 조문 대비표의 줄바꿈에서 시작하는 내용 대응 차이가 남아 있습니다. 쪽수 일치만으로 정상 페이지 소유를 주장하지 않습니다.
- 코드·바이너리·WASM 해시와 전체 TSV, 새 121쪽 PNG: [Stage 16 증적](../assets/planet6897_20261004/7545-stage16/results.json). 로그와 진단 변환본은 ignored output에만 보존합니다.

다음 보정은 4쪽 저장 줄의 너비·내어쓰기와 120~121쪽 표 이어받기의 내용 소유 및 글줄 폭을 독립 PDF와 대조합니다. HWP 121쪽의 원 PR보다 낮은 일치율을 허용하거나 Visual Sweep 수치를 인위적으로 변경하지 않습니다.

## Stage 17 메인터너 보정 — 자연 공백·숫자 폭과 물리 프레임 경계

### 원인과 구현

- `한양중고딕` 공백의 기존 `550/1024em`은 양쪽 정렬에서 늘어난 간격을 자연 폭으로 사용했다. 독립 한컴 2024 HWP PDF 122쪽과 재산출한 한컴 2020 HWPX PDF 121쪽의 같은 말미 문장은 자연 공백이 약 `0.5em`임을 확인한다.
- 두 PDF의 Type3 `T6` `/Widths`는 숫자 0~9를 모두 `500/1000em`으로 선언한다. 기존 COM 측정 표의 `509/1024em`을 `512/1024em`으로 보정했다. 공백과 숫자 근거는 [독립 PDF 숫자 폭](../assets/planet6897_20261004/7545-stage17/pdf-glyph-width-evidence.json)과 [말미 줄 공백](../assets/planet6897_20261004/7545-stage17/pdf-natural-space-evidence.json)에 기록했다. 빈 공백 글리프의 선언 `/Widths`와 실제 줄의 이동량은 구분한다.
- 공백만 보정한 중간 후보는 수치는 높아졌지만 `시ㆍ도조 / 례로` 줄 소속 검사를 실패했다. 숫자 폭을 보정한 뒤에도 물리 프레임에 추정용 초과 여유를 더해 4 HU 격자 밖 글자를 허용했다. 확정된 물리 구간의 채움·온전한 낱말 회피·말미 재생은 같은 경계 판정을 소비하도록 수정했다. 추정 폭을 쓰는 기존 scalar 경로와 유효한 저장 줄 재사용은 별도 계약을 유지한다.
- 생산 결과는 `layout_paragraph_in_frame_impl`의 실제 구간 채움으로 만들어지는 줄이다. 셀 측정과 실제 배치가 모두 `recompose_cell_lines_in_frame`을 통해 같은 결과를 소비한다. 문서 ID·페이지 번호·임의 좌표를 구현 조건으로 사용하지 않았다.
- 기존 `#3820` 검사에서 실패한 고정 픽셀 위치는 원본 용지의 본문 하단, 표 안 문단 포함, 앞쪽 소유 문단의 중복 방지, 빈 이어받기 칸 뒤 다음 행의 배치 관계로 교체했다. 새 검사 함수·새 픽스처는 추가하지 않았다.

### 검증 결과

| 대상 | 보정 전 | Native | 새 WASM | 판정 |
| --- | ---: | ---: | ---: | --- |
| HWP 121쪽 | 91.73087% | 96.53486% | 96.53486% | 원 PR PNG 93.56018%, exact source 재빌드 93.77918%보다 높음 |
| HWPX 121쪽 | 86.09215% | 93.93579% | 93.93579% | 요청한 90% 이상 충족 |
| HWPX 120쪽 | 91.69543% | 95.71513% | 95.71513% | 기존 줄 소속 회귀와 시각 기준 충족 |
| 대조군 `76076` 35쪽 | Stage 16 전 92.95975% | 94.21576% | 이 단계에서는 미실행 | 기존 주요내용 줄 소속 유지 |

- `cargo nextest run --locked --cargo-profile release-test --target-dir target/pr-review --lib --test regression_suite_014 --test regression_suite_023 --test-threads 8 --no-fail-fast`의 관련 이름 필터로 **96 PASS / 0 FAIL**. 공백·숫자 폭, `#7412`의 HWP/HWPX 줄 소속, `#3820` 소유 관계, 기존 줄 채움·condense 검사를 함께 실행했다.
- `cargo fmt --all -- --check`, `git diff --check` 통과. 새 WASM은 로컬 `wasm-pack --no-opt` 대체 빌드이며 Docker 최적화 빌드 통과로 보고하지 않는다.
- 독립 재산출한 156쪽 `pdf/issue1891/80168_regulatory_analysis-hwpx-2020.pdf`가 기존 PDF를 대체한다. Creator `Hwp 2020`, 변환 job `15cd9992-51c1-4364-870d-c6e77a2de755`. 원 HWP/HWPX는 보존했다.
- [Native·WASM TSV와 hash](../assets/planet6897_20261004/7545-stage17/results.json), [HWP review PNG](../assets/planet6897_20261004/7545-stage17/grid-hwp-p121-native-review.png), [HWPX review PNG](../assets/planet6897_20261004/7545-stage17/grid-hwpx-p121-native-review.png)를 커밋 증적으로 보존한다. 로그는 ignored `output/pr-review/planet6897-20261004/stage17-*`에만 있다.

### 남은 검토와 승인 범위

이번 **121쪽 보정은 검증 완료**다. 전체 PR 승인으로 확대하지 않는다. HWP 120쪽은 기존과 같은 85.38611%이고, 대조군 36쪽도 74.51284%로 남는다(Stage 16 전 후보에서도 74.71294%였다). 기존 구조 회귀의 행·문단 소유 통과를 이 페이지의 시각 정확도 통과로 보고하지 않는다. HWPX 쪽수는 기준과 같은 156쪽이지만 전체 내용 대응을 보장하지 않으며, Stage 17 전체 페이지 TSV와 통합 전체 nextest는 아직 완료하지 않았다. 이 HWPX 문서나 회귀 검사를 `#7445`로 이관·제거하지 않는다.
