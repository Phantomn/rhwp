---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-05
---

# PR #7543 기여자 변경 검토

## 최종 판정

**메인터너 보정 후 수용 가능** — TAC 개체 소유 문단의 저장 줄 배치의 개별 범위와 통합 후보의 최종 로컬 검증을 완료했습니다. 원 PR 직접 병합이 아니라 이 문서에 기록한 체리픽 통합 후보를 대상으로 합니다. #7445 이관 문서의 전체 피델리티 수용을 주장하지 않습니다. 원격 통합 PR의 최신 head CI·보호 요건 통과 전에는 병합하지 않습니다.

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

| 항목 | 현재 근거와 범위 | 판정 |
| --- | --- | --- |
| 구현 근거와 일반성 | TAC 개체를 소유하는 문단의 저장 줄 배치 | 개별 범위 확인 |
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

## 최종 통합 로컬 검증 — 2026-10-05

전체 회귀 head `aa10d4093f60c4e0c12aa186f2464861acbf89ea`, 최종 lint/build·fresh WASM·Skia head `42ef60c96`(테스트 모듈 순서만 정리), base `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`입니다. 전체 nextest는 **10,318 PASS/0 FAIL/50 skip**,563.885초, threads=8로 완료했습니다. Format·Native/WASM32 Clippy·workspace build·workspace all-target Clippy·manifest/unit tier 정책을 통과했습니다. fresh WASM은 Mac 로컬 `--no-opt` 대체 빌드이며 Docker 최적화 검증으로 보고하지 않습니다. Skia lib·그림 placeholder·직접 PDF 출력 범위도 실제 실행했습니다. Studio 전체1,815 PASS/0 FAIL/2 skip와 TypeScript/Vite build도 완료했습니다.

[정확한 head·명령·결과·원 source CI·개별 시각 범위](../assets/planet6897_20261004/final_validation.json)를 참고합니다. 시각 자료는 각 단계의 생산 code head와 입력/PDF hash로 고정되어 있으며, 이후 변경은 테스트·문서 범위입니다. 최종 fresh WASM hash와 기존 단계의 hash, 생산 본문 동일성 증적을 함께 기록합니다. 대용량 미달 문서는 #7445의 기존·추가 이관 기록으로 추적하고 그 문서 전체의 정확성을 승인하지 않습니다. 위의 “통합 검증 미완료/대기”는 해당 단계 작성 시점의 기록이며 최종 로컬 판정은 이 절을 따릅니다. 통합 CI는 아직 미실행입니다.

## Merge 후 contributor PR comment 계획

기여자의 TAC 개체 소유 문단의 저장 줄 배치 결과와 이 문서에 적은 추가 보정의 원인·범위를 한국어 존댓말로 구분해 설명합니다. 실제 통합 merge SHA·CI URL·Visual Sweep 정본과 이관 잔여 범위를 안내하고, 통합 대체 병합으로 원 PR을 close합니다. UTF-8 `--body-file`로 게시 후 API로 본문·close 상태를 재확인하며 중복 댓글은 게시하지 않습니다. [대표 PNG](../assets/planet6897_20261004/7543-stage6/native_review_p001.png)는 실제 merge SHA에 고정한 raw URL로 표시합니다.

### 최종 출력 provenance 재확인

최종 Native CLI(`native-skia`)와 실제 Chrome/fresh WASM으로 수용 범위20개 target·104쪽씩을 다시 내보냈습니다. **두 경로 최저90.80846%,90% 미만0쪽**입니다. SVG가 byte-identical인 쪽의 raster만 재사용하고, 달라진 SVG·글꼴 정책 블록은 새 raster로 비교했습니다. [전쪽 수용 범위 TSV·입력/PDF hash·현재 SVG hash와 재사용/재출력 구분](../assets/planet6897_20261004/final-visual/results.json). #7445 이관 문서의 전체 수용 주장은 아닙니다. 자산/속성 기여는 원래의 제한된 검토 범위를 유지하며 Studio crop은 동일한 최종 WASM의 실제 Canvas2D/CanvasKit 증적을 유지합니다.
