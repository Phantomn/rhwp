---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-05
---

# PR #7554 기여자 변경 검토

## 최종 판정

**메인터너 보정 후 수용 가능** — 저장 줄·개체 소유·88~89쪽 보정의 개별 범위와 통합 후보의 최종 로컬 검증을 완료했습니다. 원 PR 직접 병합이 아니라 이 문서에 기록한 체리픽 통합 후보를 대상으로 합니다. #7445 이관 문서의 전체 피델리티 수용을 주장하지 않습니다. 원격 통합 PR의 최신 head CI·보호 요건 통과 전에는 병합하지 않습니다.

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
| 주장과 검증 범위 | 개별 범위와 최종 로컬 검증 통과; 최신 head 통합 CI 완료 전 병합 불가 | 통합 CI 대기 |

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

## 최종 통합 로컬 검증 — 2026-10-05

전체 회귀 head `aa10d4093f60c4e0c12aa186f2464861acbf89ea`, 최종 lint/build·fresh WASM·Skia head `42ef60c96`(테스트 모듈 순서만 정리), base `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`입니다. 전체 nextest는 **10,318 PASS/0 FAIL/50 skip**,563.885초, threads=8로 완료했습니다. Format·Native/WASM32 Clippy·workspace build·workspace all-target Clippy·manifest/unit tier 정책을 통과했습니다. fresh WASM은 Mac 로컬 `--no-opt` 대체 빌드이며 Docker 최적화 검증으로 보고하지 않습니다. Skia lib·그림 placeholder·직접 PDF 출력 범위도 실제 실행했습니다. Studio 전체1,815 PASS/0 FAIL/2 skip와 TypeScript/Vite build도 완료했습니다.

[정확한 head·명령·결과·원 source CI·개별 시각 범위](../assets/planet6897_20261004/final_validation.json)를 참고합니다. 시각 자료는 각 단계의 생산 code head와 입력/PDF hash로 고정되어 있으며, 이후 변경은 테스트·문서 범위입니다. 최종 fresh WASM hash와 기존 단계의 hash, 생산 본문 동일성 증적을 함께 기록합니다. 대용량 미달 문서는 #7445의 기존·추가 이관 기록으로 추적하고 그 문서 전체의 정확성을 승인하지 않습니다. 위의 “통합 검증 미완료/대기”는 해당 단계 작성 시점의 기록이며 최종 로컬 판정은 이 절을 따릅니다. 통합 CI는 아직 미실행입니다.

## Merge 후 contributor PR comment 계획

기여자의 저장 줄·개체 소유·88~89쪽 보정 결과와 이 문서에 적은 추가 보정의 원인·범위를 한국어 존댓말로 구분해 설명합니다. 실제 통합 merge SHA·CI URL·Visual Sweep 정본과 이관 잔여 범위를 안내하고, 통합 대체 병합으로 원 PR을 close합니다. UTF-8 `--body-file`로 게시 후 API로 본문·close 상태를 재확인하며 중복 댓글은 게시하지 않습니다. [대표 PNG](../assets/planet6897_20261004/7554-stage20/labeling-p088-native-review.png)는 실제 merge SHA에 고정한 raw URL로 표시합니다.

### 최종 출력 provenance 재확인

최종 Native CLI(`native-skia`)와 실제 Chrome/fresh WASM으로 수용 범위20개 target·104쪽씩을 다시 내보냈습니다. **두 경로 최저90.80846%,90% 미만0쪽**입니다. SVG가 byte-identical인 쪽의 raster만 재사용하고, 달라진 SVG·글꼴 정책 블록은 새 raster로 비교했습니다. [전쪽 수용 범위 TSV·입력/PDF hash·현재 SVG hash와 재사용/재출력 구분](../assets/planet6897_20261004/final-visual/results.json). #7445 이관 문서의 전체 수용 주장은 아닙니다. 자산/속성 기여는 원래의 제한된 검토 범위를 유지하며 Studio crop은 동일한 최종 WASM의 실제 Canvas2D/CanvasKit 증적을 유지합니다.
