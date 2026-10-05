---
kind: report
status: active
last_verified: 2026-10-05
---

# PR #7599 리뷰 — 편집 문단 들여쓰기와 TAC prefix 보정

## 최종 판정

**메인터너 보정 후 수용 가능** — #7490 해결 범위의 로컬 검증 완료.
사용자 승인으로 별도 통합 PR을 push·등록했다. 최신 원격 head CI·보호 조건·merge는
미검증이며 통합 완료를 주장하지 않는다. 원 #7491은 열린 상태다.

## 접수 정보

| 항목 | 값 |
| --- | --- |
| 통합 PR·작성자·base | [#7599](https://github.com/edwardkim/rhwp/pull/7599) / edwardkim / devel / Open, non-draft |
| 기여자 원 PR·head | [#7491](https://github.com/edwardkim/rhwp/pull/7491) / semanticist21 / `c4367ec03a28369cc6f26b17eca46553ac61514c` / OPEN, DIRTY |
| 최초 등록 head | `e97473e7f34a771dc6716963d5f44d62a5fd2b2a` |
| 최종 production·전체 회귀 head | `c3e99c204133f661534e837aa8bb00b16e4bb99a` / `dbe8431cb240d056411b8136503e3ebef3d54bba` |
| 검증 base | `cdba77b609c399fdef26a6c9e637716aa32c2177` |
| 관련 이슈 | [#7490](https://github.com/edwardkim/rhwp/issues/7490) 종료 범위; #7491 통합 참조 |
| reviewer | 작성자 자체 검토 경로; reviewer 지정 없음. 변경 규모에 따른 별도 코드 검토와 merge 전 조건 확인은 남아 있음 |

등록 번호 기록을 위한 이 후행 commit은 `mydocs/` 문서만 바꾼다. production/test/fixture와
실행 증적은 이전 검증 head와 같다. 최신 제출 SHA는 GitHub PR head가 정본이며,
본문의 실제 Markdown 이미지 raw URL을 그 SHA에 고정한다.

## 변경과 검토 범위

원 기여자의 네 commit을 author 및 원 SHA가 남는 cherry-pick으로 수용했다.
편집 재조판의 첫 줄·후속 줄 들여쓰기, 표 앞 텍스트·공백 줄의 점유, 바깥 여백을 포함한
TAC 너비·높이, 페이지 이월 뒤 실제 새 단 원점 소비를 보정한다. 재구성 HWP의 누락된
기본 1단 선언은 실제 1단 명령 및 독립 한컴 출력으로 검증했다.

[원 PR의 상세 검토](pr_7491_review.md)에 조판 원칙의 충족/미충족/미검증/비해당,
값의 생산→측정→배치, 컷 소유·요구/예약 높이·이월·paint 경로, 독립 기대값과 전후
실행을 연결했다. 실제 호출 경로와 21개 정식 회귀 검사를 대조했으며 helper 단위 성공으로
최종 위치를 대신하지 않았다. 테스트 baseline/golden/래칫 허용치를 완화하지 않았다.

## 검증 입력과 결과

[fixture 설명](../../tests/fixtures/pr7491_edited_indent/README.md),
[입력 생성·PDF 출처/해시](assets/pr7491/input-provenance.json),
[검증 source·명령·산출 해시](assets/pr7491/visual-validation.json)가 정본이다.
실제 명령 생성 HWP와 한컴 2020 PDF, 정식 회귀 원본은 최초 등록 commit에 포함했다.

- 전체 Rust **10,379 PASS / 0 FAIL / 50 SKIP**, focused **21/21 PASS**.
- fmt·Native/WASM32/workspace all-target Clippy·workspace build·고정 base 정책: PASS.
- Native Skia lib **4,109 PASS / 13 ignored**, placeholder **2 PASS**, 직접 PDF **4 PASS**.
- TypeScript·E2E manifest **149개**: PASS. 실제 fresh WASM CDP에서 편집·undo/redo·병합 undo·저장 재개방과 TAC 이월 좌표 검사: PASS.
- 정확한 수정 전 checkout `41e1be0cf7d3c5946a5e76811da49fd214b64136`에서 경계 계약 FAIL,
  수정 후 표 2쪽 y=69.92px·본문 내부·앞줄 소유 1/0개 PASS.
  [독립 한컴 y=69.844px 및 전후 기록](assets/pr7491/tac-page-handoff-evidence.json).
- 기본 ColumnDef에 따른 정확한 8-unit 구조 이동 **51행**만 IR baseline에 추가;
  기존 583행과 11개 공개 입력의 모든 painted box는 보존했다.
  [개별 노드 증거](assets/pr7491/serializer-default-column-normalization.json).

## 시각 증적과 남은 차이

Native/fresh WASM 각각 9개 입력 14쪽, **총 28쪽 모두 90% gate PASS**,
최저 **93.42616%**다. 최종 review/standalone overlay를 직접 확인했고 글꼴 예외는 쓰지 않았다.
대표 PNG는 [증적 디렉터리](assets/pr7491/)에 커밋했으며 PR 본문에 최신 제출 SHA 고정
raw URL의 실제 Markdown 이미지 7개를 표시한다. 작은 글자·괘선 차이와 직접 판독 범위는
[기존 검토 기록](pr_7491_review.md)에 남겼다.

추가 셀 성장 입력에는 **Enter 20의 과도한 높이, 성장 셀 저장 재개방의 페이지 수,
뒤 각주/rowbreak 위치**가 남아 있다. 이전 코드에서도 실행으로 확인한 결함이며 전체
페이지 일치로 수용하지 않는다. Enter 8은 live 표 이월·본문 점유 계약을 통과한 범위다.
보조 영역의 들여쓰기는 Native 최종 좌표 계약이며 한컴 PDF/Studio UI 일치 증거는 미검증이다.

## Merge 후 contributor PR comment 계획

통합 merge와 원 PR comment·close는 아직 수행하지 않았다. 후속 승인 범위에서 정확한
merge SHA·최신 CI URL·원 기여 네 commit·보정 이유와 해결 범위·남은 차이를 한국어
존댓말로 설명하고, 같은 대표 PNG를 merge SHA 고정 raw URL로 원 #7491에 게시할 계획이다.
UTF-8 본문 파일과 API 재조회로 게시 내용을 확인한 뒤 통합 대체 병합에 따른 종료를 처리한다.
