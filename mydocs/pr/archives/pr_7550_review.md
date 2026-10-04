---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-05
---

# PR #7550 기여자 변경 검토

## 최종 판정

**메인터너 보정 후 수용 가능** — 영향 29쪽과 현대 HWPX 반례의 Native/fresh WASM 시각 검증을 충족했습니다. 미달 HWP5 대조군의 incoming 비교 검사 한 건은 #7445로 분리합니다. 고정 px 검사는 원본 장평 반영 글자 크기와 소유 글줄 관계로 수정했습니다. 최종 통합 회귀·Rust lint·최신 CI가 남습니다.

## 접수·범위

- 원 PR: https://github.com/edwardkim/rhwp/pull/7550
- 기여자: planet6897. 제목: 수정(렌더): 저장 줄 사다리가 증언하면 계보 신호 없는 HWPX 쌍둥이도 HFT ASCII 를 반각으로 잰다 (#7051)
- 원 head: `51b4a150cd6959866e13f8b9822a94ab4e6d4fef`. base: devel.
- 검토 branch: `review/planet6897-20261004`. 통합 base: `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`. 적용 단계 head: `fe0e6b9a772bd783d8c5b09bf05721d973e9e7d4`.
- 원 PR reviewer: jangster77 지정
- 경로: collaborator_external_pr 9.1.1; intake_and_review, local_validation, visual_fixture_evidence, multi_pr_update_branch, post_merge.
- 작성 시점 CI/mergeability는 참고값이며 수용·push 전에 최신 source head를 다시 확인합니다.

## 원 기여와 체리픽 출처

| 원 commit | 통합 commit/처리 | 비고 |
| --- | --- | --- |
| `6f19bec704cc0edf83e214131f115e99481e671c` | `cdad37983d1967ac9529eb85bb58b25eb3fba70a` | text_measurement::layout_half_space=false 유지; HFT 반각 증인 필드 소비 |
| `51b4a150cd6959866e13f8b9822a94ab4e6d4fef` | `616f3f7dfc6e800aab86c5c3cee0acaa8e608082` | applied |

## 조판 원칙과 검증 현황

| 항목 | 현재 근거와 범위 | 판정 |
| --- | --- | --- |
| 구현 근거와 일반성 | HFT 영향 29쪽·정상 6쪽; 미달 HWP5 twin은 #7445 | 개별 범위 확인 |
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

## 통합 head focused 검사 진행 기록

- code head `072048a8ae2d19b1140c6b675ebfc234961b84a1`에서 `issue_7051_hwpx_twin_stored_ladder_witness`: **3 PASS / 0 FAIL**. `cargo nextest run --locked --cargo-profile release-test --target-dir target/pr-review --test-threads 8 --no-fail-fast`로 해당 suite와 이름 필터를 지정했습니다.
- 로그는 ignored `output/pr-review/planet6897-20261004/stage18-full/`에 보존합니다. 검사 통과를 시각 정확도 승인으로 확대하지 않습니다. Native/fresh WASM 직접 시각 검증과 고정 px assertion의 독립 근거 검토는 계속 진행합니다.

## Stage 19 직접 시각 검증과 회귀 보정

- 생산 code head `072048a8ae2d19b1140c6b675ebfc234961b84a1`; 이후 문서 commit은 생산 소스가 같습니다. 원본 HWPX의 변경 영향 29쪽을 직접 비교했으며 Native/fresh WASM 모두 **최저 92.22769%, 미달 0쪽**입니다. 489·680쪽 review PNG도 직접 열어 본문·표시 위치를 확인했습니다. 전체 763쪽 통과 주장으로 확대하지 않습니다.
- 기준은 커밋된 한컴 2024 HWP5 분할 PDF 3개를 순서대로 합친 763쪽입니다. 결합본은 ignored output에만 두며 원래 분할 PDF와 source 입력은 보존합니다. `samples/hwpx/exam_kor.hwpx`·`pdf/exam_kor-2022.pdf` 6쪽 정상 반례는 Native/fresh WASM **98.44675%**입니다.
- `DocumentCore` 로드가 저장 줄의 HFT ASCII 반각/비례 증인 수를 판정하고 `SourceProvenance.hft_ascii_halfwidth_witnessed → ResolvedStyleSet.hft_ascii_halfwidth → resolved_to_text_style`로 전달합니다. 공통 텍스트 측정과 최종 렌더 노드가 같은 전진폭을 소비합니다. HWP3의 문단 간격 등 다른 보정은 이 별도 플래그로 켜지지 않습니다. 현대 반례에서 비례폭 유지도 확인했습니다.
- 기존 incoming 시험의 `6.02px`, `566.9px`, `24px` 절대값·크기 선택 조건을 없애고, TABLESPACE 런의 소유 글줄 포함·장평 반영 글자 크기 대비 반각, 현대 큰 숫자의 비례폭으로 수정했습니다. 중간 후보에서 현대 글자의 원본 장평 90%를 빠뜨린 실패를 진단하고, 실제 source style의 장평을 반영했습니다. 허용 폭을 올려 실패를 숨기지 않았습니다.
- HWP5 쌍둥이 489쪽은 양쪽 **37.61557%**이며 첫 문장 말미·세로 배치 차이가 있습니다. [#7445 후속 기록](https://github.com/edwardkim/rhwp/issues/7445#issuecomment-5981169881)으로 이관하고 `hwpx_twin_measures_like_the_hwp5_twin` 한 건만 수용 대상에서 제거했습니다. 원본·PDF와 미달 PNG는 보존합니다. HWPX 본 대상과 현대 반례의 검사 2개는 유지합니다.
- [Native/WASM TSV·provenance·대표 PNG](../assets/planet6897_20261004/7550-stage19/results.json)를 보존합니다. 로컬 fresh WASM은 `--no-opt` 대체 빌드이며 Docker 최적화 검증으로 표시하지 않습니다. 전체 통합 검증과 최신 source/CI 확인 후 최종 판정을 확정합니다.

보정 후 관련 nextest는 **2 PASS / 0 FAIL**입니다. format·Markdown 링크·메타데이터와 검사 함수 수 변경에 따른 generated harness 재생성/manifest 검사를 수행했습니다. generated 파일은 커밋하지 않습니다. Rust Clippy와 전체 nextest는 최종 통합 gate로 남습니다.
