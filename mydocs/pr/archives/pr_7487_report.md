# PR #7487 사전 판단 보고 — 엔진 보정과 Studio 후속 범위

## 수용 판단

현재 원격 head는 **머지 보류**다. 원 기여자의 마지막 빈 쪽 보존 변경 위에, 앞단의 빈 문단 흡수가 새 문단 owner를 지우지 않도록 로컬 보정했다. [상세 리뷰](pr_7487_review.md)에 원인·정식 회귀·전체 검증·스크린샷·남은 차이를 연결했다.

| 계보 | SHA·역할 |
| --- | --- |
| 원 기여 | `b28130e1bcea509d9969088c7e75a0e23e4974b2`, author semanticist / @semanticist21 |
| collaborator 보정 | `45863eb2b238929c22ecc606af801b6273dc8482`, author @postmelee, 원 head를 유일 parent로 보존 |
| 검토 기준 devel | `02530b9ed567a44663edb26c65fb565c4a79f00d` |
| code 후보 merge tree | `247d8bba8f2373ad90a192deba83f37a861d9e42`, 텍스트 충돌 없음 |
| 초기 체리픽 검토 기록 | `b28130e...` → `d10a64a0c9d4f85e1cad726feb7158585aa277a9`; 로컬 backup ref만 보존, 원격 통합하지 않음 |

보정은 승인된 contributor source 직접 보정 경로(9.3.1)다. 원 commit의 author·내용·credit을 유지하고 별도 보정 commit과 기록 commit을 추가한다. 다른 작업 branch나 별도 문서 PR로 보내지 않는다.

## 완료·제한 범위

| 범위 | 상태 |
| --- | --- |
| 빈 문단 Enter 경계 owner 소실 | 로컬 수정 및 200% Enter33 / 300% Enter22 전후 회귀 PASS |
| 본문 시작 좌표·저장 재열기·90회 연속 입력 | 정식 테스트 PASS |
| 필수 lint·전체 Native·Native Skia·fresh WASM | 실제 실행 PASS; 개별 명령·횟수는 상세 리뷰에 기록 |
| Chrome 실제 입력 | 새 쪽 즉시 생성 확인; 100%/66%에서 남은 Studio 지연 재현 |
| 기존 p122 저장 문서 시각·geometry | Native/fresh WASM control gate passed, 그림 geometry delta=0 |
| 새 합성 Enter 문서의 한컴 기준 출력 | 미검증, p122 control로 대체하지 않음 |
| Studio caret/scroll refresh | 별도 후속 수정 범위, 이번 코드에 포함하지 않음 |
| 표 뒤 Enter | 미해결 범위 유지, 이번 보정의 해결 주장에 포함하지 않음 |
| 원격 push·review·merge | 모두 미수행 |

## 게시·병합 준비 순서

1. 사용자에게 두 commit의 실제 diff·검증·리뷰 문서를 제시하고 source push만 승인받는다.
2. PR head·contributor ref·maintainerCanModify를 재확인하고 승인 범위만 정상 push한다.
3. 보정이 포함된 새 head의 Full CI·CodeQL·필요한 Render Diff와 branch protection을 확인한다. 과거 source CI 또는 review-only fast-pass를 새 코드에 적용하지 않는다.
4. 작업지시자의 시각 판정과 한컴 대조 미검증의 처리 범위를 확인하고 최종 판정을 갱신한다. PR 본문 asset은 head SHA 고정 raw URL로 표시한다.
5. 확정된 review 문안을 제시해 별도 게시 승인을 받는다.
6. merge는 별도 승인 뒤 진행하고 최종 merge SHA·실제 시각·CI 증적을 후속 comment에 남긴다.

이 문서는 사전 판단이다. 미래 merge SHA·merge 시각·이슈 종료를 완료 사실로 적지 않는다. [#7486](https://github.com/edwardkim/rhwp/issues/7486)은 부분 해결이므로 OPEN을 유지한다.

## Contributor review 문안 초안

아래는 게시 전 초안이며 review event는 아직 선택·제출하지 않았다. 보정 push와 새 CI 후 exact head·URL을 넣고 작업지시자에게 다시 확인받는다.

> Enter로 넘친 빈 문단의 끝 쪽을 보존하는 변경을 확인했습니다. 추가 경계 검증에서 줄간격 200%의 33번째 Enter와 300%의 22번째 Enter는 앞단의 빈 문단 흡수에서 쪽 소유를 잃는 것을 확인하여, 승인된 범위에서 같은 저장 줄 overflow 판별을 앞단에도 공유하는 보정 commit을 준비했습니다.
>
> 보정 후 두 경계에서 새 쪽과 문단 소유가 즉시 생기고, 본문 시작 좌표 및 HWPX 저장·재열기 검사가 통과했습니다. 전체 Native 회귀 10,273개, 필수 Clippy 세 경로, Native Skia 및 fresh WASM도 통과했습니다. 실제 Chrome 입력에서 새 쪽 생성은 확인했으며, 별도로 캐럿·스크롤 갱신이 늦는 기존 Studio 문제가 남아 있는 것도 확인했습니다.
>
> 이번 범위는 빈 문단의 엔진 쪽 소유 보존입니다. Studio 갱신과 표 뒤 Enter까지 해결된 것으로 보고하지 않으며 #7486은 계속 열어 두겠습니다. 원 기여와 추가 보정, 같은 source SHA의 검증 및 남은 한컴 대조 미검증을 리뷰 기록에 구분했습니다.
