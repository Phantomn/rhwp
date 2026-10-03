---
kind: report
status: active
last_verified: 2026-10-04
---

# PR #7563 리뷰 — TAC 공통 줄과 Square 문단 흐름 보정

## 최종 판정

**머지 보류.** 승인된 부분 보정을 공개했으며, 최신 head의 GitHub Actions와 1,000줄 초과 변경의 별도 검토가 남았습니다. 작업지시자 승인은 push·Open PR 생성에 대한 승인이고 merge 승인은 아닙니다. 50% 페이지 수용은 이번 부분 제출의 해결 범위 밖이며 미검증·미구현으로 명시했습니다.

## 접수와 범위

- [PR #7563](https://github.com/edwardkim/rhwp/pull/7563), edwardkim, base `devel`; reviewer를 지정하지 않은 self-review 기록입니다.
- 최초 공개 head `659c940bd00b6c684493ba71781dda97fca726c7`, 검증 code head `0373fb43c4189a138482f72ebbb5ecb0a081b4a5`, 고정 검증 base `6b3faf77d8085441f9f26d88d65a49791e910352`입니다.
- 원 기여자 davindev의 [#7482](https://github.com/edwardkim/rhwp/pull/7482) head `kidsnote/rhwp@9bc8478d5ddf43dc87c06278b8c0deb8c7640959`를 조상으로 보존했습니다. fork push403 때문에 원 저장소의 `fix/pr7482-shared-tac-rows`로 공개했습니다.
- 이번 PR은 공통 TAC 줄 구성·Square 후속 제외 영역·legacy 후속 커서 소유권의 부분 보정입니다. 원 PR의 approve·close·merge, #7518, 관련 이슈 전체 종료는 수행하지 않았습니다.

## 검증과 조판 근거

[원 PR의 상세 검토](../pr_7482_review.md)에 생산→측정/예산→실제 배치 호출 경로, 독립 기대 관계, 수정 전 FAIL/후 PASS, 적용/비적용 경계를 연결했습니다. 입력 생성·독립 PDF job/hash는 [fixture README](../../../samples/issue7482/README.md)와 [provenance](../../../samples/issue7482/provenance.json)에 있습니다. 원본/수동 NO_LS 변형을 구분하며, 50% 진단을 정상 저장본의 일반 규칙 증거로 승격하지 않았습니다.

- fmt·Native/WASM/workspace all-target Clippy·workspace build·base 고정 manifest/unit-tier PASS입니다.
- focused24/24(정식 TAC/Square/legacy15 + #6970 경계9), 전체 nextest10,265/10,265 PASS(50 skipped,792.835s)입니다.
- Native Skia lib workspace4,109 PASS(rhwp3,927;13 ignored), missing picture2/2, direct PDF4/4 PASS입니다.
- root wrapper fresh WASM SHA-256 `91654b0814b6cadc38225cf9eaab48ef1e3a35d9a7ad2b08cd7a02761ed4d95b`와 pkg/public/실제 browser 응답이 일치했습니다. CDP18문서/45검사 PASS,pageErrors0입니다. 추가 #6970 원본은 기존 fixture bytes를 CDP 응답으로 공급해 실제 loadHwpFile/WASM/canvas를 확인했습니다.
- 후행 기록 commit은 mydocs만 바꾸며, 검증한 Rust source/test/fixture/baseline에 변경을 추가하지 않습니다. local 성공은 최신 remote CI를 대신하지 않습니다.

## 시각 증적과 남은 차이

Native/fresh WASM 각각22문서27쪽 review PNG·대표 standalone overlay를 직접 판독했습니다. 최신 PR 본문에 정확한 head SHA의 대표28PNG를 Markdown 이미지로 표시합니다. [PNG와 입력/PDF hash](../assets/pr7482_shared_rows_20261004/provenance.json), [실행 집계](../assets/pr7482_shared_rows_20261004/validation-summary.json)를 사용합니다.

같은 줄/너비 부족/개행의 표·텍스트 순서와 뒤 문단, Square 옆22문단과 p2 큰 TAC의 단일 소속, #6970 p3 오른쪽 글줄의 독립16px 간격을 확인했습니다. 마지막 경계는 immutable c8에서1run FAIL하고 현 source에서 PASS했습니다. 전역gate·기존baseline/golden·래칫은 완화하지 않았으며, 사용자90% 예외는 #7482를 보정하는 이번 부분 제출에만 적용합니다. distribution p3 85.83661%,masked p2 43.1166%,regulatory HWP/HWPX p38 77.99388/78.00141%,form p2 65.49734%,#6970 p3 74.11148%의 raw gate와 남은 차이를 보존합니다. 완전 시각 일치로 판정하지 않습니다.

## 후속 조건

최신 head의 CI와 mergeability를 다시 확인하고 대형 변경의 별도 검토 및 작업지시자의 merge 승인을 받아야 합니다. 원 PR과 관련 이슈의 종료는 부분 해결 범위에 맞춰 별도로 판단합니다. merge 승인 시에는 merge SHA·실제 CI·해당 SHA 고정 PNG로 후속 기록을 준비하며, 아직 contributor comment를 게시하지 않았습니다.
