---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-05
---

# PR #7534 기여자 변경 검토

## 최종 판정

**머지 보류** — 체리픽 적용 후 통합 head의 focused·전체 회귀와 필수 시각/구조 검증 진행 중. 원 source의 CI·PNG를 통합 검증 통과로 취급하지 않습니다.

## 접수·범위

- 원 PR: https://github.com/edwardkim/rhwp/pull/7534
- 기여자: planet6897. 제목: fix: relSz 는 글리프에만·줄 상자는 선언 크기로, 글뒤 그림 뒤 TAC host 다음 문단의 저장 top 을 지킨다 (#7398 #7431)
- 원 head: `032c778f47e8742f00a6cd91ab436e6abedb4191`. base: devel.
- 검토 branch: `review/planet6897-20261004`. 통합 base: `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`. 적용 단계 head: `fe0e6b9a772bd783d8c5b09bf05721d973e9e7d4`.
- 원 PR reviewer: jangster77 지정
- 경로: collaborator_external_pr 9.1.1; intake_and_review, local_validation, visual_fixture_evidence, multi_pr_update_branch, post_merge.
- 작성 시점 CI/mergeability는 참고값이며 수용·push 전에 최신 source head를 다시 확인합니다.

## 원 기여와 체리픽 출처

| 원 commit | 통합 commit/처리 | 비고 |
| --- | --- | --- |
| `685c82b12f58976c16bf4b6f9326cae6b0401a37` | `02a3982fe14a64c3e4f1aed6be50f55beae2421c` | applied |
| `07dca362e40e34d0cb17734a027ccd3742a472d7` | `4c5372e94fa9ab2e1d54f82360bd7e77b16d0d24` | applied |
| `bcc058a312a6a78bdf56adb2b4b5e7627b4d48e1` | `6a2377e21965cdb549d9dfce04439382bb554da5` | applied |
| `6a1aeab5b7e241bce9aecc53889d8432fc52f8fc` | `d466b468463ab8615f03268d521550521b47009e` | applied |
| `032c778f47e8742f00a6cd91ab436e6abedb4191` | `e6d71387606a38b9b91d1b63ebf31ab0cfe2b948` | applied |

## 조판 원칙과 검증 현황

| 항목 | 현재 근거와 범위 | 판정 |
| --- | --- | --- |
| 구현 근거와 일반성 | 공백의 측정 폭과 잉크 점유 분리·저장 줄 소유 | 개별 범위 확인 |
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

## 단계7 통합 직접 검증과 남은 보류

- 생산 code candidate `c8c5f535cd3ebf331cadc2714c68af9fdc663a4d`의 Native/fresh WASM으로 원문 `samples/exam_eng.hwp`와 독립 한컴 2022 `pdf/exam_eng-2022.pdf`를 전체 8쪽 대조했습니다. 저장 product도 한컴 2022입니다. 두 출력 모두 최저 93.01297%(2쪽), 미달·누락 0, 쪽수 8쪽 일치입니다. [실행 결과](../assets/planet6897_20261004/7534-stage7/results.json)·Native/WASM TSV 및 manifest를 보존했습니다.
- [2쪽](../assets/planet6897_20261004/7534-stage7/native_review_p002.png)·[7쪽](../assets/planet6897_20261004/7534-stage7/native_review_p007.png)을 직접 열었습니다. 원 기여의 줄 진행·답지 시작 복구는 확인됐지만 2쪽 Woman/Man/Sophia의 NBSP 공백에 사각형 잉크가 나타납니다. 90% 통과만으로 이 출력 결함을 승인하지 않습니다.
- 기존 `test_521`은 SVG에서 595..600px 영역을 찾고 24px 간격을 고정했습니다. 8쪽 시각 근거를 확인한 뒤 동일 기존 시험을 표 host pi104·다음 답안 pi105의 소유 관계 및 원문 저장 vpos에서 선언 표 높이를 뺀 간격으로 변경했습니다. 새 테스트는 추가하지 않았습니다. 변경된 기존 unit 1/1 PASS, 새 기여 회귀 5/5 PASS, unit tier 검사 PASS(개수 증가 0)입니다.
- 다음 보정은 NBSP 잉크입니다. 문서에는 NBSP가 저장되어 있고 현재 HY신명조 글꼴 cmap은 U+00A0을 `uni0080`(윤곽선 2개)으로 매핑합니다. 공백이 실제 글리프처럼 그려지는 경로를 대조해 저장 폭·밑줄을 보존하며 잉크만 그리지 않도록 검토합니다. 메인터너 보정과 재출력 전 **개별 시각 보류**입니다.

## 단계8 공백 잉크 보정 후보

NBSP를 포함해 Unicode 공백만으로 구성된 클러스터는 SVG의 본문·그림자, Canvas 기본 글리프, Skia 텍스트 pass에서 잉크를 그리지 않도록 맞췄습니다. 이미 계산된 글자 전진폭·뒤 글자 위치·밑줄과 배경 장식은 같은 경로에 남습니다. 입력·폰트 파일·메트릭·비교 지표를 바꾸지 않았습니다.

기존 SVG unit 및 변경한 `test_521` **51/51 PASS**입니다. [소스·폰트 hash와 결과](../assets/planet6897_20261004/7534-stage8/results.json)를 기록했습니다. Native/fresh WASM 재출력과 native-skia 기능 검증 전에는 보정 후보로 보류합니다.

## 단계8 보정 후 판독 완료 — 개별 보류 해소 / 통합 검증 대기

- 생산 code candidate `4cfe96e52`에서 기존 SVG/관계 unit 51/51 PASS, native-skia 기능을 켠 관련 unit 100/100 PASS입니다. Native 기능 빌드와 새 WASM 빌드 모두 성공했습니다. 이후 실행 head `4de143ddb`까지 생산·테스트 변경이 없음을 git diff로 확인했습니다. [소스·build·로그 hash와 현재 결과](../assets/planet6897_20261004/7534-stage8/results.json)를 보존했습니다.
- 전체 8쪽 Native/fresh WASM TSV가 페이지별로 같으며 최저 **95.29650%(4쪽)**, 2쪽 **96.11420%**, 7쪽 95.84947%입니다. 미달·누락 0, 원문/PDF/출력 모두 8쪽입니다. 새 회귀를 추가하지 않고 기존 `test_521`의 저장 관계 검사만 고쳤습니다.
- [보정 2쪽 review](../assets/planet6897_20261004/7534-stage8/native_review_p002.png)와 [7쪽 review](../assets/planet6897_20261004/7534-stage8/native_review_p007.png)를 직접 열었습니다. 듣기 답안 NBSP의 사각형 잉크가 사라지고 밑줄·내용·뒤 문단 위치는 유지됩니다. 오른쪽 편지 및 답안 시작, 양 단 줄 소속이 유지됩니다. 2쪽 엄격 픽셀 18.70%와 관용 실루엣 96.11%를 구분합니다.
- [Skia 실제 2쪽 PNG](../assets/planet6897_20261004/7534-stage8/skia_page002.png)와 fresh WASM 2쪽 raster도 직접 열어 같은 NBSP 잉크/밑줄 관계를 확인했습니다. Skia PNG는 2쪽 진단이며 전체 8쪽 Skia 실루엣 검증으로 주장하지 않습니다. 도구 라벨과 본문은 판독 가능합니다.
- 앞서 수용한 #7547 대상 17쪽, 정상 반례 두 벌 각 3쪽, #7543 대상 1쪽까지 **32쪽 전체를 Native/fresh WASM으로 재검증**했습니다. 각각 최저 95.23424% / 97.99734% / 98.00366% / 99.41288%, 미달·누락 0으로 유지됩니다. 원 기여의 줄 높이·TAC host 수정과 메인터너의 공백 잉크/좌표 회귀 보정으로 **개별 발견 보류는 해소**했습니다. 최종 통합 검증 전 머지 보류를 유지합니다.
- merge 후 contributor 설명에는 두 원 기여와 독립 저장 관계 검사, 공백 글리프 보정의 필요·범위·반례, 정확한 CI/merge SHA 및 위 PNG를 함께 기록합니다. 아직 게시·close하지 않았습니다.
