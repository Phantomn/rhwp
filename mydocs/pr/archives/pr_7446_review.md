---
kind: snapshot
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-09-27
---

# PR #7446 리뷰 — #7443 후속 표 식별·캐시 보정

## 최종 판정

**머지 보류 — 로컬 검증은 통과했으며 최신 PR head의 GitHub CI 완료를 기다린다.**
원 PR #7443은 이미 승인·병합됐다. 이 판정은 후속 PR #7446에만 적용한다.
CI 완료 뒤 최신 head·mergeability와 사용자 병합 승인을 확인해야 한다. 이번 사용자 요청은 후속 PR 생성까지다.

## 접수 정보

- [후속 PR #7446](https://github.com/edwardkim/rhwp/pull/7446), 작성자 @postmelee, base `devel`.
- base: `013bc846fca3434ccb1e4167744bed9440c6da6a`; code candidate: `9e1cffccfc9401262fff806f16b388180d5a663c`.
- code commit은 이전 로컬 보정 `7af493cb68a6dd94f6dc087e723d44b0738b7939`의 cherry-pick이다.
  전체 tree `98aee4a4a45a1b4d9e11582fb644845f9f9f6d77`가 동일하다. 이후 commit은 리뷰·증적 기록이다.
- base route: `collaborator_self_merge`; modifiers: `intake_and_review`, `local_validation`.
  원 PR 병합 후 처리는 `post_merge`를 적용했다. 자신에게 GitHub APPROVE를 제출하거나 reviewer를 지정하지 않는다.

## 변경과 검증

다른 구역의 동일 번호 표를 선택 대상으로 받지 않도록 section을 비교하고, 같은 중첩 표 안의 leaf 셀·문단
이동에는 bbox cache/failure memo를 재사용한다. 모든 조상 경로와 마지막 control은 구분한다.
세 회귀 테스트는 보정 전 FAIL/후 PASS였다. 기존 13개와 관련 cache 검사까지 21개도 앞선 검토에서 통과했다.
Rust grid 검사는 문단 불일치를 skip하지 않고 실제 bbox 페이지·구역·안쪽 표 경로를 확인한다.
모든 좌표가 정확한 leaf 셀과 일치해야 한다는 가정은 기존 텍스트 우선 계약과 다르므로 넣지 않았다.

최종 code candidate에서 다음을 재실행하고 통과했다.

- `cargo fmt --all -- --check`, native·WASM32 lib·workspace all-target Clippy(`-D warnings`), workspace build.
- `node scripts/rust-test-suite-manifest.mjs --check --base-ref 013bc846fca3434ccb1e4167744bed9440c6da6a`.
  파생 suite는 PR에 포함하지 않았다. source-side unit test 변경은 없어 unit-tier 검사는 비해당이다.
- `node scripts/run-rust-test.mjs issue_7442_nested_cell_hit_test -- --cargo-profile release-test --target-dir target/pr-review`: 7/7.
- `npm --prefix rhwp-studio test`: 1,803/1,803. TypeScript 포함 `npm --prefix rhwp-studio run build`와 E2E manifest 검사도 통과했다.
- 실제 Chrome E2E: F5 선택·드래그·부모 모델 보존·Undo·빈 영역 hit·일반 드래그·블록/개체 선택 PASS.

[명령별 결과·해시](../assets/pr7443_review/followup-validation.json)와
[원 PR 리뷰의 검증 결과](pr_7443_review.md#검증-결과)를 근거로 삼는다.
이전 동일 tree의 전체 Rust 결과 및 로컬 파일명 문제에 대한 재검사 내역은 원 PR 리뷰에 그대로 보존했다.
이번 diff는 Rust 제품 소스·snapshot·baseline을 바꾸지 않는 test helper와 Studio 변경이므로
전체 Rust 회귀를 중복 실행하지 않고 해당 lint·focused 및 Studio 검증을 수행했다.

## 입력·조판 원칙·미검증

[원본 HWP](../../../samples/basic/issue1994_behindtext_table_20200830.hwp)는 검토 commit에 포함되어 있고
실행 바이트의 SHA-256 `8e7a95cf591944bff56050879fa90251921ec57e28eac66d40c6fb8ad103016f`가 일치한다.
Rust/Cargo 입력이 기존 fresh WASM 빌드와 동일하며 `pkg`/Studio public WASM SHA-256도 일치했다.
WASM `91d8dfb9e44560338b1915fe1e6554b15278fe819b11ebe58feb8fdcc052909a`를 재사용했다.

조판 원칙·Native Skia·Visual Sweep: **비해당**. 이미 생성된 hit의 식별과 bbox cache key를 바꾸며
레이아웃·측정·배치·paint·분할을 생성하는 코드를 변경하지 않는다.
[비교 영상](../assets/pr7443_review/f5-resize-before-after.mp4)은 원 PR 이전과 보정 포함 버전의 UI 증거이며,
후속 보정만의 시각 변화 또는 한컴 출력 일치 증거는 아니다. 두 번째 7×4 중첩 표 문서는 미특정·미검증이다.

## 원 PR 후속 처리와 유지 범위

[원 PR 최종 코멘트](https://github.com/edwardkim/rhwp/pull/7443#issuecomment-5855597661)와
[이슈 종료 확인 코멘트](https://github.com/edwardkim/rhwp/issues/7442#issuecomment-5855597818)를 게시하고 본문을 API로 확인했다.
원 기여자의 commit·fork branch는 보존한다. 로컬 devel은 merge SHA로 동기화했다.

이 후속 PR은 아직 진행 중이므로 현재 기본 작업공간, `codex/pr-7443-followup`, 기존 보정 참조 branch,
`output/pr-review/7443/before` 비교 worktree, 원 head 비교 서버와 검증 로그를 유지한다.
후속 PR 종료 시 소유 산출물을 정리하며 공유 `target/pr-review`와 다른 작업의 worktree는 건드리지 않는다.
