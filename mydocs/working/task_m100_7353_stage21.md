# Task #7353 — W2 편집·조회·저장 일관성

- 승인 범위: 구현계획 §5.4 W2. W1을 로컬 커밋 `a2289c9b2`로 보존한 뒤 시작했다.
- 브랜치: `refactor/0.9.0`. 원격 push, Studio 기본값 변경, devel 병합은 하지 않는다.
- 정본: [구현계획](../plans/task_m100_7353_impl.md#54-2026-10-01--wasm-제품-조판을-v2로-전환).
- 상태: W2 대표 제품 편집 경로 구현·집중 검증 완료. W3 기본 전환/Studio UI/최종 통합 게이트는 잔여.

## 원인과 구현 경계

1. V2 fit에는 셀/문단/컨트롤 소유가 있지만 paint가 이를 제품 조회 메타데이터로 전달하지 않았다.
   셀 커서가 글줄을 찾지 못해 셀 전체 360px 높이를 반환하고 hit-test는 본문으로 반환했다.
   `TextPaint`는 IR cell 순서와 확정 line/nested-table owner를 보존하고,
   `ownership::bind`가 최종 host tree에 section·전체 CellPath를 연결한다. 좌표로 소유를 추측하지 않는다.
2. 셀 지연 편집의 `mark_section_pagination_dirty`는 V2 packet을 무효화하지 않았다.
   새 IR에 이전 packet이 노출됐다. 이 경계에서 V2 전체 결과를 무효화하고 Legacy focused-cell
   patch/cursor 보정 경로를 사용하지 않는다. 재페이지네이션 전에는 출력이 명시적으로 실패한다.
3. `end_batch_native`/snapshot 복원은 조판 실패를 성공으로 보고할 수 있었다.
   준비 결과를 확인하고 오류를 전달한다. 실패한 IR은 보존하며 페이지 수 0/출력 오류 상태로 둔다.
   자동 Legacy 재시도나 묵시적 IR rollback은 하지 않는다. 호출자는 저장한 snapshot으로 복구한다.
4. 문서 host가 styles를 자체 재해소하여 제품 font environment를 무시했다.
   현재 IR·폰트 환경으로 생성한 style 결과에 V2 source-unit 검증을 적용하고 본문/표의 fit와 paint에
   같은 결과를 전달한다. `document_mut`로 DocInfo가 바뀌는 경우에도 새 세대에서 재해소한다.
5. 저장 검증의 재열기는 원래 선택 엔진을 보존한다. Legacy picture-band shadow가 V2 core에
   Legacy pagination/composed를 이식하지 않도록 해당 최적화는 V2에서 사용하지 않는다.

생산→소비 추적: IR cell index/line owner → `TextPaint::build_node`의 실제 배치 노드 →
`HostedSectionLayout::render_page`의 소유 연결 → cursor/hit-test/selection의 동일 TextRun.
기하 fit·컷·예약 높이는 변경하지 않았다. Legacy cell-unit fast query는 V2에서 사용하지 않는다.
구역/표의 기존 미지원 조판 속성은 여전히 거부한다.

## 집중 계약과 현재 증적

정식 계약은 `tests/cases/issue_7353_product_v2.rs`에 유지한다.
증적 root: `output/7353/wasm-product/w2/`.

- `before.log`: 최초 셀 커서 height360/x20, 본문 hit-test를 관찰했다. 당시 약한 page-only
  assertion은 통과했으므로 이 로그를 실패 테스트로 부르지 않는다. 후속 계약은 정상 저장본의
  9pt=12px 글자 높이와 page20px+cell padding5px=25px 및 cell path를 검사한다.
- `state-before.log`: 지연 편집 후 옛 결과 노출 및 실패 batch 성공 반환 **2건 FAIL**을 재현.
- `state-after.log`: 수정 후 기존 6건 + W2 6건 **12 통과**. 글자 삽입·snapshot undo/redo·HWP 저장/재열기,
  실제 커서/hit-test, 30줄 추가에 따른 페이지 증가와 모든 추가 유닛 1회 출력,
  지연 편집 상태, batch 거부/복구, 폰트 환경 변경/복원·DPI 변경/복원을 검사한다.
- 최종 `native-tests.log`: **13 통과**. 셀 Enter 후 새 문단 소유/커서/hit-test 계약과
  페이지 증가 뒤 실제 마지막 marker의 페이지를 가리키는 커서 검사를 추가했다.
  `host-regression.log`의 기존 host 계약은 **31 통과**(마지막 Legacy focused-cell guard 추가 전;
  그 guard는 이 읽기 전용 대조군 경로에 적용되지 않음).
- fresh WASM 제품 검사 **본문 2쪽 + 표 3쪽 PASS**, 전체 페이지의 Native/WASM tree·layer·SVG 동일.
  `browser-manifest.json`, `table/browser-manifest.json`, 각 `editing.json`에 결과를 보존했다.
  표 편집은 `--edit-cell`로 명시한다. 초기 스크립트의 SVG 공백 기반 자동 식별이 표를 본문으로
  오인한 결과는 셀 검증으로 인정하지 않고, 명시적 셀 여정으로 재실행했다.
- 실제 셀 검사: 삽입 후 cursor x25/height12/2쪽, hit의 cell path 보존, 30줄 추가 후 3→4쪽,
  마지막 `ADDED29`는 3쪽이며 커서/선택도 3쪽. Enter 후 새 셀 문단 index1, 커서 y41.6,
  선택 y41.0 및 snapshot 원복을 확인했다. 일반 커서와 hit/선택의 0.6px 위상 차이는 남아 있으며
  모든 query의 수직 원점이 완전히 동일하다고 주장하지 않는다.
- `cargo fmt --all -- --check`, Native lib Clippy 및 WASM lib Clippy `-D warnings` 통과.
  전체 workspace/all-target Clippy·전체 회귀·정책 게이트는 W3 안정 통합 시점에 수행한다.

### 실행과 source 고정

검증 source는 `a2289c9b2` 위 W2 변경이다. 변경 Rust source는 `source-sha256.txt`에 고정했고
fresh WASM 생성 뒤 전부 일치함을 확인했다. 이후 제품 source 변경 없음(정식 테스트와 스크립트 보강만 수행).
WASM SHA256: `8d47b73df3de6c788e681df484e8ade48e29543dedf541a8dafd6f4d0a10d0bd`.

```bash
RHWP_V2_PRODUCT_EVIDENCE=output/7353/wasm-product/w2 cargo test --locked --jobs 2 \
  --target-dir /home/edward/mygithub/rhwp/target/pr-review \
  --test regression_suite_022 issue_7353_product_v2 -- --test-threads=4
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/home/edward/mygithub/rhwp/target/pr-review \
  scripts/wasm-pack-locked.sh --target web --out-dir output/7353/wasm-product/w2/pkg --no-opt
node scripts/verify-product-v2-wasm.mjs --pkg output/7353/wasm-product/w2/pkg \
  --input tests/fixtures/issue7353_body_frame_review/portrait-saved.hwp \
  --out output/7353/wasm-product/w2 --edit
node scripts/verify-product-v2-wasm.mjs --pkg output/7353/wasm-product/w2/pkg \
  --input tests/fixtures/issue7353_host_owner_review/split-saved.hwp \
  --out output/7353/wasm-product/w2/table --edit-cell
/home/edward/mygithub/rhwp/venv/bin/python output/7353/wasm-product/w2/sweep.py
```

브라우저 명령의 환경: `VISUAL_SWEEP_CHROME=/home/edward/.cache/puppeteer/chrome/linux-152.0.7977.54/chrome-linux64/chrome`.
`--no-opt` 진단 빌드이며 Studio의 배포 asset을 교체한 것이 아니다.

### 직접 시각 확인

동일 정상 저장본과 독립 한컴 PDF를 사용했다. 표 입력은
`tests/fixtures/issue7353_host_owner_review/split-saved.hwp`, 기준은 `split-2020.pdf`;
본문 입력은 `tests/fixtures/issue7353_body_frame_review/portrait-saved.hwp`, 기준은 `portrait-2020.pdf`.
새 Native/fresh WASM/실제 Canvas 출력의 compare·standalone overlay·review를 생성했다.
대표 표 2쪽:

- [compare](/home/edward/mygithub/rhwp-task-7353/output/7353/wasm-product/w2/table/visual/canvas/compare/compare_002.png)
- [standalone overlay](/home/edward/mygithub/rhwp-task-7353/output/7353/wasm-product/w2/table/visual/canvas/overlay/overlay_002.png)
- [review](/home/edward/mygithub/rhwp-task-7353/output/7353/wasm-product/w2/table/visual/canvas/review/review_002.png)
- [편집한 실제 Canvas](/home/edward/mygithub/rhwp-task-7353/output/7353/wasm-product/w2/table/browser/edited-canvas.png)

직접 확인 범위: 본문 두 쪽의 빈 줄·줄 순서, 표 앞 문단/2쪽 ROW01–19/3쪽 ROW20–25 및 뒤 문단
`AFTER`, 외곽 위치. 승인된 W1과 원본 SVG 5쪽도 byte 동일하다. 글꼴/테두리 색의 기존 차이는 남는다.
편집 Canvas에는 `EDIT ROW 01`이 실제로 나타난다. 편집 후 별도 한컴 정답지를 만든 것은 아니므로
이 캡처는 제품 편집 연결 증거이며 한컴 편집 결과 일치 증거로 확대하지 않는다.

코멘트: 표 2쪽 Canvas의 내용 픽셀 중심 자동 일치율 보조값 = 약 14.26%.
높을수록 좋음: 기준 PDF와 rhwp PNG가 더 비슷함
낮을수록 나쁨/검토 필요: 잉크 위치나 형태 차이가 큼
단, 사람 판정 정확도가 아니라 내용 픽셀 중심 자동 일치율 보조값입니다

본문 Native/WASM SVG 2.53–4.06%, Canvas 8.67–9.52%; 표 Native/WASM SVG 2.05–4.32%,
Canvas 12.14–16.94%. 이를 자동 시각 통과로 해석하지 않는다. 최종 메인테이너 판정은 별도이며,
Studio history UI 동작도 이번 제품 API 검사로 대신하지 않는다.

## 남은 경계

- 일반 Studio/worker의 기본 V2 선택, 새 문서/암호/복구 진입, UI history 여정은 W3다.
- 중첩 셀 경로 및 복잡한 객체 편집, fresh wrap, exact/supplemental font metrics 등은
  단순 셀 입력 계약의 통과로 지원 완료라고 주장하지 않는다. 기존 V2 admission을 유지한다.
- 현재는 로컬 집중 검증이며 전체 CI·최종 제출 게이트 통과 또는 릴리즈 완료가 아니다.
