---
kind: report
status: active
canonical: mydocs/manual/pr_review_workflow.md
last_verified: 2026-10-02
---

# PR #7539 리뷰 — Studio Enter 뒤 새 쪽 캐럿·스크롤

## 최종 판정

**머지 보류 — 로컬 검증은 완료했으며 최신 원격 head CI·타인의 리뷰·별도 병합 승인이 남았다.** 작성자 self-review이고 GitHub Approve는 제출하지 않았다.

## 접수 정보

| 항목 | 값 |
| --- | --- |
| PR·작성자·base | [#7539](https://github.com/edwardkim/rhwp/pull/7539) / postmelee / devel |
| 시작 base | 선행 #7487 merge `83bf0f3c840c9afc1de584c17b167610fd9caf3b` |
| 검증 source | `3f66b618c0842221981eb3353a818b696315ee53` |
| 게시 candidate | `43fb71ef01609ec50ace7758f506898ad4836a08` — 같은 source에 증적·문서만 추가 |
| 관련 이슈 | #7486 참조만 사용; 표 뒤 Enter 잔여 문제로 OPEN 유지 |
| 경로 | collaborator 본인 PR; reviewer 지정 없음; review·review_impl와 오늘할일을 동일 PR의 문서 후속 commit으로 포함 |
| 생성 시점 원격 참고값 | non-draft, MERGEABLE/BLOCKED, CI 실행 중. 병합 판단 때 재조회 필요 |

## 변경과 검토 범위

본문 Enter command의 비동기 layout 완료 뒤 기존 one-shot 캐럿 reveal을 예약한다. `splitParagraph`를 허용 목록에 추가하고 `executeOperation(command)`가 예약하도록 했다. 기존 Undo/Redo의 history type 예약도 이를 사용한다. `insertText` 및 셀·머리말·각주 Enter는 대상에서 제외했다.

실제 생산·소비 호출 경로, 원인과 반례는 [구현·검증 보고서](../../report/task_m100_7486_report.md)에 연결했다. 엔진 조판·문단 소유·쪽수·저장 줄 정보·baseline을 바꾸지 않았으며 #7487의 원 기여 코드를 다시 수정하지 않았다.

## 검증과 증적

- 루트 wrapper의 source head fresh dev WASM과 root pkg/public hash 일치를 확인했다. generated JS/WASM은 제출에서 제외했다.
- TypeScript·production build PASS; 전체 Studio/package 계약 1,816 PASS / 0 FAIL / 0 skipped.
- 정식 E2E 160/200/300% × 100% 한 쪽/66% 두 쪽 보기의 실제 Enter·Undo·Redo: 수정 전 15 FAIL, 수정 후 84 PASS / 0 FAIL. 최종 source에서 재실행했다.
- Ctrl+Enter 정상 대조군 6 PASS, 기존 편집 Undo 계약 PASS, E2E manifest 147개/147행 일치.
- 사용자 Chrome UI의 200%·33번째 Enter에서 추가 입력 없이 새 쪽 캐럿과 스크롤 이동을 확인했다.
- 입력은 E2E 코드 생성이며 파일로 소비한 HWP/HWPX/PDF가 없다. 원점 투영·viewport 계약을 검증했고 한컴 PDF의 caret 좌표와 일치한다고 주장하지 않았다.
- [검증 JSON](../../working/assets/issue7486-studio-caret/validation.json)에 source SHA, 전체 18개 전후 상태, WASM·대표 PNG 해시를 포함했다.

전후 PNG 4개는 직접 판독했고 PR 본문에 정확한 head의 실제 Markdown 이미지로 표시했다. 원점 잔차를 수치·화면으로 함께 설명했다. 세부 명령·로그 위치·한계는 보고서에 보존했다. DOM overlay 변경이므로 Native/WASM 문서 Visual Sweep과 90% silhouette gate는 비해당이다. Rust 전체 회귀·lint 및 release WASM은 Studio 단독 범위라 생략했다.

## 공통 원칙 검토

| 항목 | 판정 | 근거 |
| --- | --- | --- |
| 구현 근거와 일반성 | 충족 | 완료된 쪽 원점에 표시하는 UI 계약; 특정 spacing/문서 조건·clamp 없음 |
| 측정·배치 일관성 | 비해당 | 엔진 측정·배치 미변경; UI 투영은 완료된 VirtualScroll과 page-local rect를 사용 |
| 분할·이어받기 계약 | 비해당 | SplitParagraph의 내용·컷·flush를 바꾸지 않고 화면 표시 완료 예약만 추가 |
| 줄 소속과 점유 높이 | 비해당 | LineSeg·높이·조판 함수 미변경 |
| 사례와 증거의 독립성 | 충족 | 합성 fixture setup과 실제 키 입력을 분리; source baseline FAIL/수정 PASS; 논리 커서·완료 원점·viewport 검사 |
| 기준값 변경 | 비해당 | baseline/golden/허용치 변경 없음 |
| 주장과 검증 범위 | 충족 | 6개 조합·18개 Enter/Undo/Redo 상태, 제외 경로 unit, 사용자 Chrome 확인; IME/iOS·HF 실사용은 미검증으로 구분 |

## 병합 후 계획

별도 승인이 있을 때 최신 head·필수 CI·mergeability를 재확인한다. 병합 후 merge SHA 고정 이미지와 검증 범위를 comment에 남기고 #7486을 열어 둔다. 표 뒤 Enter를 해결한 것으로 쓰지 않는다. 이번 승인 범위는 PR 생성까지이며 새 PR 병합·self-Approve는 수행하지 않는다.
