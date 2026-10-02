---
kind: report
status: active
last_verified: 2026-10-02
---

# PR #7508 리뷰 — 첫 문단 클립보드 구조 슬롯 해제

## 최종 판정

머지 보류 — 기능상 수용 후보이며 보류 PR을 제외한 최종 코드 후보 검증을 기다린다.

검토일: 2026-10-02. 작성자: semanticist21. 대상: devel.
기준 devel: `e5098bc91be44a49367a7f2895a14fcd4f4c2c7f`.
누적 진단 head: `c6ef30ea943308c37e5d68c8304dfdabdd7b8f74`.

누적 실행 명령·로그·제한은 [일괄 검토 기록](pr_semanticist21_20261002_review_impl.md#누적-검증-결과)에 연결한다.
원 PR의 exact-head 녹색 CI와 누적 진단 head의 결과는 별개다. 누적 head는 7건을 포함하며
원 PR 또는 최종 수용 그룹의 전체 CI 통과로 간주하지 않는다. 메인터너 source/test 보정은 없다.

원 PR code head: `b53d3621be516ed8f918b321a0e0b01f09f8ff19`.
[원 PR](https://github.com/edwardkim/rhwp/pull/7508) · [exact-head Build & Test](https://github.com/edwardkim/rhwp/actions/runs/36949216335/job/110664169785).
CI 집계 실패·진행 중 없음(확인 당시). 원 PR head는 최초 접수 이후 바뀌지 않았다.
Reviewer edwardkim 지정. 원격 GitHub 승인 이벤트는 아직 게시하지 않았다.

## 범위와 소비 경로

#7506 종료 제안. 기능 commit `b53d3621`를 누적 적용했다.
`strip_structural_controls_for_text_clipboard`가 구역/단 정의의 선행 8-slot을 걷고
char_offsets·char_shapes·range tags·markpen·LineSeg start/char_count를 같이 옮긴다.
누름틀 field_ranges는 scalar 축이라 같은 stream shift를 적용하지 않고 필요 시 offsets를 재구성한다.
복사본 → paste_internal의 merge_from → 재조판 → 최종 text layout의 글자 보존을 확인했다.
일반 페이지네이션/renderer 자체는 바꾸지 않는다.

## 검증 결과와 제한

- focused 1/1 PASS: 첫 문단 두 문단 복사, 서식의 ‘가나’ 소속과 전체 표시 글자.
- fresh WASM 실제 브라우저: 마지막 문단의 IR text와 최종 getPageTextLayout가
  둘 다 ‘마바사아가나다라’로 PASS.
- e509 기준 merge simulation clean. 원 exact-head Full CI 성공.
- 모든 혼합 field/남은 컨트롤 조합이 검증됐다는 주장은 하지 않는다. 한컴 픽셀 비교나 Studio UI 검증도 아니다.

## 다음 조건

기능 검토상 수용 후보로 분리한다. 보류 PR의 효과를 포함한 누적 c6ef 결과를 최종 그룹 결과로
승격하지 않고 선별 통합 후보 검증·작업지시자 승인을 진행한다. 원격 게시/merge 미실행.

