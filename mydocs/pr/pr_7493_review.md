---
kind: report
status: active
last_verified: 2026-10-02
---

# PR #7493 리뷰 — Studio 수정 모드·IME·Undo

## 최종 판정

머지 보류 — 수정 모드의 Unicode 위치와 IME 종료 조각 수명에서 반례가 실행으로 실패했다.

검토일: 2026-10-02. 작성자: semanticist21. 대상: devel.
기준 devel: `e5098bc91be44a49367a7f2895a14fcd4f4c2c7f`.
누적 진단 head: `c6ef30ea943308c37e5d68c8304dfdabdd7b8f74`.

누적 실행 명령·로그·제한은 [일괄 검토 기록](pr_semanticist21_20261002_review_impl.md#누적-검증-결과)에 연결한다.
원 PR의 exact-head 녹색 CI와 누적 진단 head의 결과는 별개다. 누적 head는 7건을 포함하며
원 PR 또는 최종 수용 그룹의 전체 CI 통과로 간주하지 않는다. 메인터너 source/test 보정은 없다.

원 PR code head: `d29d483e4f5fc759c067e33766700bcad87133ed`.
[원 PR](https://github.com/edwardkim/rhwp/pull/7493) · [exact-head Build & Test](https://github.com/edwardkim/rhwp/actions/runs/36847784882/job/110327871695).
CI 집계 실패·진행 중 없음(확인 당시). 원 PR head는 최초 접수 이후 바뀌지 않았다.
Reviewer edwardkim 지정. 원격 GitHub 승인 이벤트는 아직 게시하지 않았다.

## 범위와 검증

#7489 종료 제안. Studio 입력·InsertTextCommand·fragment/Undo만 변경하며 Rust 조판 수정은 없다.
누적에 6개 기능 commit을 적용했다. TypeScript PASS, Studio 전체 1813 PASS / 2 skip / 0 FAIL.
작성자의 overwrite runner를 직접 실행한 25개 시나리오는 PASS했다.
Node 24.15.0에서 `allowedNodeEnvironmentFlags` 기반 지원 판단 때문에 정식 wrapper가
실제 실행 가능한 `--experimental-transform-types` 테스트를 skip했다. 직접 runner를 실행해 보완했다.

## 추가 반례 — 실행 결함과 검토상 우려

실제 TypeScript handler/command/history를 작성자의 scalar-index mock core와 실행했다.
이는 브라우저/WASM 사용자 여정 검증이 아니라 재현 가능한 명령 계층 반례다.

| 반례 | 기대 | 관측 |
| --- | --- | --- |
| abcd에서 수정 모드로 😀 다음 X 입력 | 😀Xcd | 😀bXd |
| 첫 😀 뒤 caret을 scalar 1로 맞춰 X 입력 후 한 묶음 Undo | abcd | 😀bcd |
| IME 조각 생성 후 실제 deactivate 호출 | 조각 해제 또는 복원 | 보관 조각 1개 잔류 |

Core의 `get_text_range_native`는 Unicode scalar 단위다.
`command.ts`의 703·713·764줄 등은 JS UTF-16 `text.length`로 replay/caret/merge 경계를 정한다.
기존 삽입 명령의 위치 가정이 새 수정 모드에서 잘못된 덮어쓰기 대상으로 연결된다.
두 번째 반례는 Undo에서 먼저 실패했으므로 Redo 실패를 실행했다고 주장하지 않는다.
`input-handler.ts:4421`의 deactivate는 fragment ID를 null로 만들지만 저장소의 조각을 해제하지 않는다.
`dispose`의 같은 처리도 코드상 우려이지만 이번 실행 반례는 deactivate만 확인했다.

## 해제 조건과 범위

기여자가 scalar/UTF-16 경계를 일관되게 처리하고 명령 병합·Undo/Redo·IME 중 문서 종료 수명을
검증해야 한다. wrapper의 skip 검출도 고쳐 정식 테스트에서 실행되게 해야 한다.
실제 browser/WASM 편집 검증은 아직 미실행이므로 UI 전체 정상 판정은 하지 않는다.
소스에 임의 메인터너 보정을 넣거나 기존 필드 Redo 문제까지 이번 결함으로 합치지 않았다.
보류 근거 comment는 작업지시자의 승인 뒤 게시했다.

## 승인 후 게시 기록

2026-10-02 작업지시자의 댓글 게시 승인 후 [보류 사유 comment](https://github.com/edwardkim/rhwp/pull/7493#issuecomment-5944041776)를 게시했다.
게시 직전 원 head가 그대로 OPEN임을 확인하고 API 재조회로 한글 본문·BOM/치환 없음 및
작성 문안과의 일치를 확인했다(파일 끝 개행만 정규화). 코드 변경·push·GitHub 승인·merge 없음.
