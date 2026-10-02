---
kind: report
status: active
last_verified: 2026-10-02
---

# PR #7497 리뷰 — HTML 인라인 그림 붙여넣기

## 최종 판정

머지 보류 — 기능 검토는 수용 권고이나, 보류 PR을 제외한 최종 통합 후보의 검증이 아직 없다.

검토일: 2026-10-02. 작성자: semanticist21. 대상: devel.
기준 devel: `e5098bc91be44a49367a7f2895a14fcd4f4c2c7f`.
누적 진단 head: `c6ef30ea943308c37e5d68c8304dfdabdd7b8f74`.

누적 실행 명령·로그·제한은 [일괄 검토 기록](pr_semanticist21_20261002_review_impl.md#누적-검증-결과)에 연결한다.
원 PR의 exact-head 녹색 CI와 누적 진단 head의 결과는 별개다. 누적 head는 7건을 포함하며
원 PR 또는 최종 수용 그룹의 전체 CI 통과로 간주하지 않는다. 메인터너 source/test 보정은 없다.

원 PR code head: `64f76e37b31bad9c0dedcb9bde67cc7a8f70eb58`.
[원 PR](https://github.com/edwardkim/rhwp/pull/7497) · [exact-head Build & Test](https://github.com/edwardkim/rhwp/actions/runs/36841040332/job/110330389788).
CI 집계 실패·진행 중 없음(확인 당시). 원 PR head는 최초 접수 이후 바뀌지 않았다.
Reviewer edwardkim 지정. 원격 GitHub 승인 이벤트는 아직 게시하지 않았다.

## 수용 권고 범위

#7496 종료 제안. 3개 기능 commit을 누적 적용했다.
본문 p/li/최상위 span의 data URI img를 글자처럼 취급하는 8-slot 컨트롤로 만들고,
char_offsets·서식 시작·문단 병합 및 저장/재열기에 보존한다. bullet을 위해 갭을 재구성하지 않는다.
셀 붙여넣기는 기존 text-only 정책을 유지하고 미사용 그림 데이터의 추가를 되돌린다.
조판 알고리즘·renderer·golden 변경은 없다. 입력 IR의 글/그림 순서 보존 변경이다.

## 검증 결과와 제한

- focused 8/8 PASS: 글 사이 그림, 반복/그림만 있는 항목, 서식·컨트롤 앵커, HWP/HWPX 재열기.
- fresh WASM 실제 HeadlessChrome 152: 글 ‘앞뒤’, 최종 SVG의 image 존재,
  Canvas 렌더 호출 및 HWP/HWPX 재열기 모두 PASS.
- e509 기준 merge simulation clean, 원 exact-head Full CI 성공.
- 중첩 span 그림과 모든 HTML 요소를 지원한다는 판정은 아니다. 원 PR의 잔여 범위를 유지한다.
- 브라우저 결과는 API smoke이며 Studio UI·독립 한컴 픽셀/좌표 일치 판정이 아니다.
  렌더러 규칙 수정이 없어 신규 조판 회귀의 PDF 점수 게이트는 이 변경의 주장과 비해당으로 분리한다.

## 다음 조건

기능상 수용 후보로 선정한다. 누적 진단은 보류 PR 4건도 포함하므로 승인/통합 완료가 아니다.
선별한 수용 그룹에 대한 코드 후보와 검증·CI를 고정한 뒤 최종 승인한다.
원 head CI 재사용은 원 head에만 적용하며 누적 c6ef의 Full CI로 보고하지 않는다.
원격 review/comment/push/merge는 아직 수행하지 않았다.

