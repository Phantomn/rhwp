---
kind: decision
status: active
canonical: mydocs/troubleshootings/README.md
last_verified: 2026-10-01
---

# #7353 — V2 리팩토링 접근 실패와 백지화

## 결정과 적용 범위

2026-10-01 작업지시자는 이번 접근의 잘못을 트러블슈팅으로 기록하고 **접근 방식 자체를
백지화**하도록 지시했다. 개별 거부 조건을 하나씩 해제하거나 누락 기능 목록을 순회하는
방식으로 기존 계획을 계속하지 않는다. 이는 완료·일시 보류가 아니라 기존 실행계획의 폐기다.

- 수행계획·구현계획의 다음 절편, 자동 승인, 기존 공정률과 완료 예상은 더 이상 실행 근거가 아니다.
- 원래 목적(규칙에 따른 일관된 표 조판과 책임 분리)은 남지만, 대체 설계·일정·재사용 범위는 미정이다.
- 기존 코드·테스트·샘플·시각 승인·실행 증적은 실패 경위와 재검토 자료로 보존한다.
  보존은 해당 구조의 채택이나 새 설계에서의 재사용 확정을 뜻하지 않는다.
- 이번 문서화로 코드 rollback, WASM 기본값 복원, Legacy 파일 삭제, 브랜치 삭제,
  원격 push, devel 병합 또는 이슈 종료를 수행하지 않는다. 기존 WASM의 로드 결함은 남는다.
- 이전의 개별 시각 판정은 당시 입력·페이지·빌드 범위의 증거로 유지한다.
  이를 취소하지도, 제품 전체 대체 완료의 근거로 확대하지도 않는다.

## 잘못된 방향

문단 안에 텍스트와 여러 컨트롤이 선언되는 Document IR의 전체 문서 처리 책임을 새 구조로
옮겨야 했으나, 실제 진행은 **제한된 조합만 수용하는 별도 V2 엔진의 허용 범위를 절편마다
늘리는 개발**로 바뀌었다. 독립 경로에서 실험한다는 선택과, 그 제한된 수용 모델을 제품
대체 구조로 계속 확장한다는 선택은 다르다. 후자가 이번에 폐기하는 접근이다.

기존 잘못된 예외까지 복제하거나 Legacy 출력을 무조건 정답으로 삼아야 한다는 결론은 아니다.
정상적인 문단·컨트롤 결합을 처리하는 책임의 이전과, 규칙이 잘못된 기존 동작의 교정을
구분하지 못하고, 기능 수용 절편을 늘리는 것으로 두 문제를 대신한 것이 실패다.

| 실패 축 | 확인한 근거 | 잘못된 판단 |
| --- | --- | --- |
| 문서 단위 책임의 누락 | 쪽 번호와 TAC 표가 같은 문단에 있으면 V2가 명시적으로 거부 | 각 기능의 개별 지원을 정상 조합의 지원으로 확대 |
| 최초 종단 목표의 상실 | stage19의 최초 원본이 바로 #6923 샘플인데 현재 제품 진입점에서 여전히 거부 | 선택 표·부분 문서의 진전을 원본 전체 대체의 진전으로 사용 |
| 검증 경로 불일치 | #6923 기존 회귀는 `DocumentCore::from_bytes`를 사용하며 이 API는 Legacy를 선택 | 기존 회귀 통과만으로 V2에서 같은 문서가 열린다는 보증 불가 |
| 제품 전환과 기능 대체 혼동 | WASM의 V2 선택·제한된 Studio 여정 검증 뒤에도 원본 로드 불가 | 기본값 전환·의존 분리를 전체 지원 능력과 혼동 |
| 작업 운영 실패 | 장기간 절편 추가·반복 승인과 작업지시자의 반복적인 완료 시점 지적 | 문서 전체 대체의 완료 기준 대신 절편 수·부분 통과로 진척 설명 |

위 표는 에이전트의 설계·구현·검증 운영 실패를 기록한다. 메인테이너가 개별 화면을 통과시켰기
때문에 발생한 문제가 아니다. 부족한 종단 증거를 부분 검증으로 대신하지 않을 책임은 구현자에게 있다.
과거 모든 테스트가 잘못됐거나 모든 V2 기능이 무효라는 주장도 아니다.

## 확인된 반례와 재현 근거

- 브랜치: `refactor/0.9.0`
- 진단 코드 HEAD: `43eeb3380f95e2bce81b7ac2dddb513f2f6caa74`
- 입력: [148738070_wrapper_table_stored_page_frame.hwp](../../tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp)
- 입력 SHA-256: `41f8f0349840a72476606a45843caf01b12013b5e536fca5f6214ff76872ceb3`
- 기존 독립 기준: [한컴 PDF](../../tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame-2020.pdf), 7쪽.
- 사용한 기존 fresh WASM SHA-256: `4f2a880a8d95abe04b622f97fbcf4d982eb6ff8b689046189659b02c89f7c534`

같은 원본·같은 WASM에서 `HwpDocument.openWithTypesetter`로 비교했다.
Legacy는 열리고 `pageCount=7`, V2는 다음 오류로 열리지 않는다.

```text
렌더링 오류: 렌더링 오류: V2 section 0: table_v2 host:
Paragraph { index: 0, cause:
UnsupportedHost("body page-number declaration with table controls") }
```

파싱된 첫 문단 `s0:pi=0`은 빈 텍스트와 저장 LineSeg 1개를 가지며,
`ci=0 SectionDef`, `ci=1 ColumnDef`, `ci=2 PageNumberPos`, `ci=3 TAC 5×4 표`가 공존한다.
문제가 제기된 본문 1×1 wrapper는 `pi=5`이고 내부 문단은 87개다.
현재 실패는 wrapper 페이지네이션에 도달하기 전의 문단 수용 단계다.

재현 명령(해당 WASM 빌드가 있는 작업 트리 루트에서 실행):

```bash
node --input-type=module <<'JS'
import fs from 'node:fs';
import { initSync, HwpDocument } from './pkg/rhwp.js';
initSync({ module: fs.readFileSync('pkg/rhwp_bg.wasm') });
const input = fs.readFileSync(
  'tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp');
for (const engine of ['v2', 'legacy']) {
  try {
    const doc = HwpDocument.openWithTypesetter(input, engine);
    console.log(engine, doc.getDocumentInfo());
    doc.free();
  } catch (error) {
    console.log(engine, String(error));
  }
}
JS
```

이 재현은 로드 결함의 증거다. Legacy의 7쪽이라는 수치만으로 조판 정확성을 판정하지 않으며,
V2가 열리지 않았으므로 이 실행의 시각 통과도 없다. 이 거부 이후의 실패는 미검증이다.

## 코드·기록 연결

- [host_section.rs](../../src/renderer/table_v2/host_section.rs): 위 HEAD의 253–266행에서
  `PageNumberPos` 처리 시 같은 문단의 TAC/비-Square 표를 거부한다. 689–752행은
  `HostedParagraph`의 소유 줄이 실제 페이지에 수용됐는지로 선언을 활성화·소비한다.
  거부문만 지우는 것은 이 소유/배치 계약의 완성을 입증하지 않는다.
- [기존 #6923 회귀](../../tests/cases/issue_6923_wrapper_table_stored_page_frame.rs)의 `core()` →
  [DocumentCore 생성](../../src/document_core/commands/document.rs)의 `from_bytes()` →
  `TypesettingEngine::Legacy`. 이 사실은 해당 테스트에 대한 것이며 전체 회귀의 엔진을 일반화하지 않는다.
- [stage19](../working/task_m100_7353_stage19.md)의 첫 종단 목표에는 동일 원본이 이미 있다.
  이번 문서를 새로 제공받아야만 알 수 있었던 요구사항이 아니다.
- [stage25](../working/task_m100_7353_stage25.md)의 공통 paint/상태 분리와 5종 15페이지 검증은
  그 범위의 증거다. #6923 원본 수용이나 Legacy 전체 제거를 증명하지 않는다.
- [폐기된 구현계획](../plans/task_m100_7353_impl.md)은 제한된 수용부터 절편 확장,
  기본값 전환, 의존 제거까지의 의사결정 이력을 보존한다.

## 백지화 이후의 경계와 재발 방지

1. 이번 오류를 다음 패치 대상으로 삼아 기존 절편 작업을 재개하지 않는다.
   전수 누락 목록을 작성해 하나씩 고치는 작업도 이번 결정의 이행으로 간주하지 않는다.
2. 기존 계획에 새 절편을 추가하거나 새 공정률을 계산하지 않는다. 새 방향은 별도 명시적
   합의가 필요하며, 이 문서는 대체 설계나 구현 승인이 아니다.
3. 향후 검토에서는 **기능 보존/규칙 교정**, **선택 경로 연결/실제 기능 대체**,
   **부분 검증/제품 종단 검증**을 구분한다. Legacy와 새 경로 중 무엇을 실행했는지 명시한다.
4. 원본의 정상 컨트롤을 지우거나, 실패를 거부 테스트로 고정하거나, fallback·빈 성공으로
   숨기거나, 기대값을 완화하는 것은 리팩토링 완료가 아니다.
5. 문단 중심의 전체 처리 책임·종료 조건이 합의되기 전에는 기존 V2 구조와 부분 성과를
   새 설계의 전제로 고정하지 않는다. 재사용 가능성도 별도 판단한다.

문서화 완료와 결함 해결은 구분한다. **이번 조치는 접근 폐기와 이력 보존이며, 제품 결함 해결은 아니다.**
