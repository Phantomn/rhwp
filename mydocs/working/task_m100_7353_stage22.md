# Task #7353 — W3 WASM/Studio 기본 V2 전환

- 승인: 2026-10-01, W3 및 준비된 CDP 사용. W2는 `8c835ef2b`로 보존했다.
- 브랜치: `refactor/0.9.0`. 원격 push/PR/devel 병합은 범위 밖.
- 상태: 기본 V2 전환 및 대표 CDP 제품 여정 통과. 최종 통합 게이트 잔여이며,
  전체 CI·제출 완료나 모든 문서의 편집 지원으로 보고하지 않는다.

## 변경 경계

WASM constructor·암호 열기·빈 문서는 V2를 기본으로 선택한다. Native/CLI 기본은 유지한다.
명시적 Legacy 재열기/암호 열기/빈 문서 생성 API를 유지하며 자동 fallback은 없다.
일반 Studio와 `?typeset=v2`는 `main.ts`의 편집 경로를 사용한다. 기존 읽기 미리보기는
`?typeset=preview` 또는 전용 `table-v2` 개발 모드로 구분한다.

생산→소비: WASM 제품 진입의 엔진 선택 → DocumentCore의 동일 IR/엔진별 확정 결과 →
일반 WasmBridge → CanvasView·InputHandler·history → 기존 제품 출력/조회. 새 표 컷/높이 규칙은
추가하지 않는다. 복구·암호·새 문서 경로와 실패 시 기존 문서 보존을 함께 확인한다.

## 검증 환경

- CDP: `http://localhost:19222`, Windows Chrome `153.0.8010.48` 연결 확인.
- 테스트가 만든 탭만 닫고 사용자 기존 탭은 보존한다.
- fresh WASM: `output/7353/wasm-product/w3/wasm-build.log`; Studio는 task worktree의 `pkg/`를 사용.
- 이전 W2 증적은 전환 전 대조군이며 W3 UI 성공 증거로 재사용하지 않는다.

## 실제 UI에서 발견한 연결 결함

1. 본문 한 글자 입력: `replace_body_text_local_native`가 V2에도 Legacy의
   unchanged-flow 캐시 갱신을 적용해 `composed[0]`에서 panic. 정식 회귀
   `product_v2_local_keyboard_edit_publishes_current_generation`을 수정 전 실행해 같은
   panic을 확인했다(`w3/local-edit-before.log`). V2는 새 hosted 결과를 확정하고
   UI에 full refresh를 요구한다. Legacy 최적화와 줄 높이 규칙은 바꾸지 않는다.
2. 셀 지연 입력: V2의 미확정 generation은 0쪽으로 노출되지만 Studio의 Legacy 지연
   경로가 이를 정상 페이지 목록으로 수용했다. 입력 뒤 core=3 / view=0 / canvas=0
   (`w3/cell-paint-before.log`). WasmBridge의 V2 셀 삽입·삭제·교체는 즉시 확정 API를
   사용한다. 수정 뒤 core=3 / view=3 / canvas=3 및 편집 텍스트·선택 표시를 직접 확인했다.
   Core의 지연 API/이전 generation 비공개 계약은 유지한다.
3. WASM `createEmpty`의 Legacy 최소 stub에는 V2에서 필요한 기본 스타일/구역 정보가
   없어 0쪽이었다. V2 생성은 이미 검증된 내장 빈 문서 경로를 사용한다. Native/명시적
   Legacy의 기존 stub 계약은 보존한다. Studio 새 문서는 원래도 `createBlankDocument`로
   교체하므로 UI 통과만으로 이 별도 API를 통과 처리하지 않았다.

"나가기" 확인창은 편집한 테스트 문서에서 의도적으로 `?typeset=legacy`로 이동하는
beforeunload 경계다. E2E는 `new-edited.hwp`를 보존한 후 해당 테스트 탭의 전환에만
dialog handler를 등록한다. 실제 사용자 변경사항 보호 기능은 유지한다.

## 검증 범위

- 최종 source Native 제품 계약: 15건 통과, 기존 hosted-section 대조: 31건 통과.
  본문 local 편집의 수정 전 FAIL / 수정 후 PASS 포함(`native-tests.log`, `host-regression.log`).
- 일반 Studio CDP: 본문 2쪽·분할 셀 3쪽에서 file input 열기 → 실제 키 입력 →
  Ctrl+Z/Y → Shift+Left 선택 → bridge HWP export → file input 재열기, 새 문서 입력,
  명시적 Legacy 전환을 검사한다. `studio-cdp.log`, `studio/result.json` 및 편집 화면 PNG.
- 저장은 Studio bridge export이며 OS 저장 대화상자 자동화는 아니다. 암호 경계는
  WASM 제품 API에서 별도로 검사하고 암호 입력 UI 전체를 검사했다고 하지 않는다.
- 전체 회귀·3종 Clippy·Native Skia·최종 정책 검증은 아직 완료하지 않았다.

## 최종 fresh WASM·직접 판독

- source: `8c835ef2b` 위 W3 미커밋 diff. 변경 파일 hash는
  `output/7353/wasm-product/w3/source-sha256.txt`에 고정했다.
- WASM SHA-256: `95d70b5c3589cf16a6329bb292b4773536a3cea98cd7ce0b78bd8fe9f93fa2ec`.
  실제 Studio Vite는 task `pkg/`를 읽는다. 이전 소유권이 다른 패키지는 삭제하지 않고
  같은 상위 폴더의 `pkg-before-w3/`로 보존했다.
- 최종 CDP: Windows Chrome 153.0.8010.48, `http://localhost:19222`,
  `VITE_URL=http://localhost:7700 node e2e/product-v2.test.mjs --mode=host`.
  `expected navigation guard, got beforeunload` 뒤 Legacy 전환 및 무예외 검사가 통과했다.
- 기본 constructor/empty/password, 명시적 Legacy, 편집/소유/저장 API 검사:
  `node scripts/verify-product-v2-wasm.mjs --pkg pkg --input <fixture> --out <w3 또는 w3/table> --default-v2 --edit 또는 --edit-cell`.
  Native/fresh WASM의 tree·layer·SVG 5쪽 일치. 암호·빈 문서도 fresh bundle에서 통과했다.
- Studio 단위 검사 1,767 통과 / 2 skip / 실패 0; TypeScript, E2E manifest, fmt,
  Native Clippy 및 WASM-lib Clippy 통과. workspace/all-target Clippy는 아직 미실행.
- 시각 산출: `output/7353/wasm-product/w3/sweep.py`가 정상 한컴 저장본과 대응 PDF를
  이용해 Native SVG·fresh WASM SVG·실제 Canvas의 compare/standalone overlay/review를 생성했다.
  원본은 `tests/fixtures/issue7353_body_frame_review/portrait-saved.hwp`와
  `tests/fixtures/issue7353_host_owner_review/split-saved.hwp`, 대응 기준은 각 폴더의
  `portrait-2020.pdf`, `split-2020.pdf`다. 편집본의 한컴 재출력 일치를 주장하지 않는다.

대표 증적:

- [Studio 본문 편집 화면](../../output/7353/wasm-product/w3/studio/body-edited.png)
- [Studio 셀 편집·선택 화면](../../output/7353/wasm-product/w3/studio/cell-edited.png)
- [표 2쪽 compare](../../output/7353/wasm-product/w3/table/visual/canvas/compare/compare_002.png)
- [표 2쪽 standalone overlay](../../output/7353/wasm-product/w3/table/visual/canvas/overlay/overlay_002.png)
- [표 2쪽 review](../../output/7353/wasm-product/w3/table/visual/canvas/review/review_002.png)
- [CDP HTML 결과](../../output/e2e/product-v2-report.html)

직접 판독: 본문 빈 줄/ALPHA–CHARLIE/DELTA–AFTER, 표 앞 BEFORE,
2쪽 ROW01–19와 3쪽 ROW20–25 및 뒤 AFTER의 순서·외곽·이어받기 위치를 확인했다.
Studio 편집 후 EDIT 텍스트와 선택 표시, 용지 표시가 유지된다. 기준 PDF 대비 글꼴과 테두리
색 차이는 남는다. 기계 일치율 또는 Native/WASM 일치를 한컴 피델리티 완전 통과로 승격하지 않는다.

코멘트: 표 2쪽 Canvas의 내용 픽셀 중심 자동 일치율 보조값 = 약 14.26%.
높을수록 좋음: 기준 PDF와 rhwp PNG가 더 비슷함
낮을수록 나쁨/검토 필요: 잉크 위치나 형태 차이가 큼
단, 사람 판정 정확도가 아니라 내용 픽셀 중심 자동 일치율 보조값입니다

다음은 승인 경계의 최종 통합 게이트다. 복잡한 객체/중첩 셀 편집, IME 전체, 암호 입력 UI,
OS 저장창은 이 대표 여정만으로 검증 완료라고 하지 않는다. 새 미지원 속성 구현은 자동 편입하지 않는다.
이번 절편은 원격 push·PR·devel 병합을 수행하지 않았다.
