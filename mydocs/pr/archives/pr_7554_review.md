---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-05
---

# PR #7554 기여자 변경 검토

## 최종 판정

**메인터너 보정 후 수용 가능** — 영향 88·89쪽과 정상 대조군 전체 8쪽의 Native/fresh WASM 증적을 충족하고 회귀를 원본 저장 줄·본문 영역 관계로 보완했습니다. 기존 #7445 이관 범위인 대용량 문서 전체의 잔여 차이는 별도 추적합니다. 최종 통합 회귀·Rust lint·최신 CI가 남습니다.

## 접수·범위

- 원 PR: https://github.com/edwardkim/rhwp/pull/7554
- 기여자: planet6897. 제목: fix(layout): 저장 사다리 경계 다섯 곳을 정본 자리에 맞춘다 — 1480000-201900042 88·89쪽 (#6761, #7345, #7351)
- 원 head: `f67d0882a4373ecba4e8de09a5cd548f204bd05f`. base: devel.
- 검토 branch: `review/planet6897-20261004`. 통합 base: `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`. 적용 단계 head: `fe0e6b9a772bd783d8c5b09bf05721d973e9e7d4`.
- 원 PR reviewer: jangster77 지정
- 경로: collaborator_external_pr 9.1.1; intake_and_review, local_validation, visual_fixture_evidence, multi_pr_update_branch, post_merge.
- 작성 시점 CI/mergeability는 참고값이며 수용·push 전에 최신 source head를 다시 확인합니다.

## 원 기여와 체리픽 출처

| 원 commit | 통합 commit/처리 | 비고 |
| --- | --- | --- |
| `61204678abd0b3e8584da3dad83df4f68c9e6adf` | `0b62cca7f6437e77327ae002cd346ee70dd913c0` | 최신 devel stored_first_margin_is_page_relative 공통 helper와 #6797 TAC 도형 반례 제외 조건 보존; layout/typeset 공동 소비 유지 |
| `f67d0882a4373ecba4e8de09a5cd548f204bd05f` | `f295dd7ca33474f7fb00b1cf3107c7f081037a02` | applied |

## 조판 원칙과 검증 현황

| 항목 | 현재 근거와 범위 | 판정 |
| --- | --- | --- |
| 구현 근거와 일반성 | 저장 줄 간격·개체 소유 및 88–89쪽; 전체 잔여는 기존 #7445 | 개별 범위 확인 |
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

- code head `072048a8ae2d19b1140c6b675ebfc234961b84a1`에서 `issue_6761_stored_ladder_boundaries`: **5 PASS / 0 FAIL**. `cargo nextest run --locked --cargo-profile release-test --target-dir target/pr-review --test-threads 8 --no-fail-fast`로 해당 suite와 이름 필터를 지정했습니다.
- 로그는 ignored `output/pr-review/planet6897-20261004/stage18-full/`에 보존합니다. 검사 통과를 시각 정확도 승인으로 확대하지 않습니다. Native/fresh WASM 직접 시각 검증과 고정 px assertion의 독립 근거 검토는 계속 진행합니다.

## Stage 20 전체 TSV·정상 대조군과 회귀 보정

- 생산 코드 `072048a8ae2d19b1140c6b675ebfc234961b84a1`; 이후 commit은 문서·다른 검사 변경이며 생산 코드가 같습니다. 103쪽 전체 Native/fresh WASM TSV를 산출했고 미달은 양쪽 **24쪽, 최저 17.93315%**입니다. 이미 #7445에 이관됐던 전체 피델리티의 [현재 검증 결과](https://github.com/edwardkim/rhwp/issues/7445#issuecomment-5981264035)를 갱신했습니다. 전체 쪽수 일치·전쪽 시각 승인을 이 PR의 새 회귀에 포함하지 않습니다.
- 직접 수정하는 **88쪽 92.67463%, 89쪽 95.97303%**는 두 backend에서 같습니다. Native의 88·89쪽 review PNG를 직접 열어 그림·캡션의 쪽 소속과 순서를 확인했습니다. `issue2004_cell_image_stack.hwp` 대조군은 PDF·rhwp 모두 실제 **8쪽**이며, 전체 최저 **96.40794%, 미달 0쪽**, 표 조각 4쪽 **99.95278%**입니다. 앞선 계획의 4쪽이라는 표현은 검사 대상 페이지이며 문서 전체 쪽수가 아닙니다. 파일명과 달리 기준 PDF Creator는 Hwp 2022이며 정상 출력이므로 재사용했습니다.
- `127.84px`, `80.8px`, `BODY_BOTTOM_PX`·고정 gap 범위를 없애고, 표와 앞 글줄 사이의 저장 줄간격+표 바깥여백, 두 문단의 원본 vpos 차이, 용지의 하단·꼬리말 여백, 빈 후속 줄의 원본 pitch, 저장 bl/lh 비율을 검사합니다. 픽셀 반올림 대신 저장 4 HU 격자 오차만 허용합니다. 전체 103쪽 고정 assertion을 제거했으며 그림·캡션의 유일 소유, 다음 쪽 첫 문단, 앞 소제목 비침범은 유지합니다. 새 함수·픽스처를 추가하지 않았습니다.
- 보정 후 해당 검사 **5 PASS / 0 FAIL**입니다. 측정·실제 배치의 stored-first-margin 공통 helper와 #6797 글자처럼 도형 제외 조건은 유지했습니다. 앞 문단/그림/후속 저장 줄의 원점을 실제 스냅된 cursor로 대조하는 경로와 표 조각의 host 여백 처리를 정상 대조군에서 검증했습니다.
- [전체 TSV·provenance·대표 PNG](../assets/planet6897_20261004/7554-stage20/results.json)를 보존합니다. 로그는 ignored `output/pr-review/planet6897-20261004/stage20-7554/`입니다. Rust lint·통합 전체 nextest·최신 source head/CI를 완료한 뒤 최종 통합 판정을 확정합니다.
