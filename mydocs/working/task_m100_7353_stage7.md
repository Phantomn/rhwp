# Task #7353 — 선택한 표의 독립 V2 미리보기 세션

- 선행 절편: `a674c5690` — [stage6](task_m100_7353_stage6.md).
- 범위: 문서 IR의 표 선택·서식 snapshot과 재개 가능한 표 내부 미리보기.
- 상태: R3 진행 중. 문서 전체 엔진 전환이나 Studio 개선 완료가 아니다.

## 구현과 비적용 경로

`TablePreviewSession::from_document`는 section/paragraph/control 주소로 표를 선택하고
문서의 서식을 해석하여 `PreparedTextTable`을 만든다. 저장 LineSeg·미지원 셀 컨트롤은
기존 수용 경계에서 오류로 반환한다. 원본을 수정하거나 저장 정보를 지워 통과시키지 않는다.
이미 명시적으로 구성한 중첩 흐름은 `TablePreviewSession::new`로 동일 세션에 연결한다.

용지/본문 영역은 호출자가 pixel 단위로 지정한다. 이것은 선택한 **표 내용만**의 미리보기이며,
host 문단의 앵커, 문서 본문의 현재 위치, 구역의 단·머리말·꼬리말을 해석하지 않는다.
실물 문서의 페이지 번호나 위치를 재현하는 API로 사용하면 안 된다. Legacy와 DocumentCore,
기존 `typeset/table` 및 `layout/table_*`의 실행 경로는 변경하지 않았다.

세션은 폭·DPI·내용 snapshot을 고정한다. 원본 편집 뒤 다시 계산하려면 새 세션을 만든다.
`next_page`가 성공하면 fragment와 동일한 `PageRenderTree`를 반환하고 이어받기 상태를 확정한다.
오류는 상태를 전진시키지 않는다. 첫 페이지의 남은 영역에 들어가지 않으면 온전한 다음 페이지에
한 번만 재질의한다. 그래도 들어가지 않는 원자 유닛은 요구 폭/높이와 가용 영역을 오류로 반환한다.
폭 부족, 잘못된 viewport, 출력 페이지 한도도 명시적 오류이며 Legacy fallback은 없다.
첫 영역을 건너뛴 경우 가짜 빈 출력을 만들지 않고 반환 page index로 전이를 표시한다.

## 값과 상태의 실제 소비 경로

`session.rs:176` 서식 해석 → `text.rs:72` 공통 문단 구성과 불변 plan/paint 생성 →
`session.rs:207` 가용 영역 → `text.rs:139`의 기존 cursor fit →
`session.rs:212` 동일 fragment의 `append_to` → `text.rs:182` 확정 좌표로 노드 구성 →
`session.rs:215` 성공한 continuation/페이지 번호 확정.

내용 컷·빈 밴드·패딩·자식 재개는 기존 V2 cursor가 계산한다. 세션은 높이를 재계산하거나
clamp하지 않는다. 실제 수용한 조각만 출력하며 paint 성공 이전에는 cursor를 소비하지 않는다.
부분 출력 뒤 다음 자식이 과대인 경우에도 이를 정상 종료로 바꾸지 않고 후속 내용을 보존한다.
rowspan·반복 제목·캡션·각주 및 문서 앵커는 이번 경로 비해당이며 지원 완료로 세지 않는다.

## 독립 기대값과 정식 검사

`tests/cases/issue_7353_table_v2_session.rs`의 9건은 수동 작성한 fresh Document IR/명시적
중첩 흐름 계약이다. 한컴에서 생성한 저장 문서나 기준 PDF의 증거가 아니다.

- 96 DPI, 글자900 HU(9pt)=12px, 고정 줄간격2700의 IR 단위=18px 전진을 입력으로 지정한다.
  셀 여백 왼쪽375/오른쪽525/위225/아래300 HU는 5/7/3/4px이다.
  alpha/beta/gamma의 총높이61(3+18×3+4), 예산22에서 조각21/18/22를 검사한다.
  실제 줄 x25, y33/30/30, 상자 높이12와 각 페이지의 표 점유 높이를 함께 검사한다.
- 나누지 않는 두 문단 표43은 첫 남은 예산10을 건너뛰고 새 영역60에 배치한다.
  온전한 예산22에도 못 들어가면 요구43 오류가 반복 질의에도 동일하며 cursor는 소비되지 않는다.
- 부모 상1 + 자식43 + 뒤 문단18 + 하2 =64. 예산22의 중첩 조각22/22/20과
  자식/뒤 문단의 실제 위치·내용 순서를 검사한다.
- 앞 문단18을 출력한 뒤 원자 자식43이 실패하면 출력 수는1로 유지한다. 더 큰 viewport로
  새 세션을 만들면 before/alpha/beta/after가 각각 한 번, 총79로 보존된다.
- 원본 내용/글자 크기를 편집해도 시작한 세션에는 반영되지 않는다. 새 세션에서만 새 글자18px와
  줄 전진24px를 사용한다. 정상 완료와 페이지 한도 초과를 구별하고 빈 후속 페이지를 만들지 않는다.
- 잘못된 선택, 비정상 viewport/DPI, 저장 줄 정보 거부를 검사한다. 저장 줄 테스트는 수동
  메타데이터의 부정 입력 검사이며 정상 저장 정보의 수용 근거가 아니다.

기존 구현에는 이 세션 API가 없으므로 컴파일 불가를 수정 전 결함 FAIL로 세지 않는다.
기존 fixture/assertion/baseline/ignore는 수정하지 않는다.

## 검증 기록

- 검증 소스: `a674c5690` + 이번 변경. [source.sha256](../../output/7353/r7/source.sha256)의
  15개 파일이 제품/review worktree에서 바이트 단위로 일치함을 확인했다.
- 실행 worktree: `/home/edward/mygithub/rhwp-review-7353`; 고정 target:
  `/home/edward/mygithub/rhwp/target/pr-review`. 파생 harness는 review에서만 준비한다.
- 신규 세션9건: [최종 로그](../../output/7353/r7/session-tests-final.log) PASS.
- 기존 V2 기하8/중첩12/텍스트7/중첩 출력9와 Legacy #5301의3건:
  [선택 회귀 로그](../../output/7353/r7/selected-tests.log) **39건 PASS**, 신규 포함 총48건 PASS.
- `cargo clippy --locked -p rhwp --lib --target-dir <고정 target> -- -D warnings`:
  [Native PASS](../../output/7353/r7/native-clippy.log).
- 동일 명령에 `--target wasm32-unknown-unknown`을 추가한 lib lint:
  [WASM PASS](../../output/7353/r7/wasm-clippy.log).
- `cargo clippy --locked --test regression_suite_003 --target-dir <고정 target>
  -- -D warnings`: [최종 검사 target PASS](../../output/7353/r7/test-clippy.log).
- `cargo fmt --all -- --check`: [PASS](../../output/7353/r7/fmt.log).
  manifest 준비 후 `node scripts/rust-test-suite-manifest.mjs --check --base-ref
  7a95e46e025470a4d7a7b59ad68ec02958bda738`: [정책 PASS](../../output/7353/r7/policy-check.log).
- 변경 Markdown 두 파일의 상대 링크 검사와 `git diff --check` PASS.

제품의 `src/**` test module 변경은 없으며, 새 integration source만 `tests/cases/`에 둔다.
nextest 설치0.9.137/권장0.9.140 및 `report-skipped` 미지원 경고는 로그에 남기고 환경을 변경하지
않았다. 최종9건 이전의8건 실행은 중복 합산하지 않는다.

실행 명령은 `node scripts/run-rust-test.mjs <case> -- --cargo-profile release-test
--target-dir /home/edward/mygithub/rhwp/target/pr-review --no-fail-fast`이다.
새 세션은 `ISSUE7353_PREVIEW_DIR=.../output/7353/r7/preview`를 지정해 실제 출력 SVG를 남긴다.
`rsvg-convert -b white <svg> -o <png>`로 변환한 세 페이지의 alpha/beta/gamma 분리를 직접
확인했다. [출력 hash](../../output/7353/r7/preview.sha256)를 보존한다.

![첫 조각](../../output/7353/r7/preview/session-0.png)

![두 번째 조각](../../output/7353/r7/preview/session-1.png)

![마지막 조각](../../output/7353/r7/preview/session-2.png)

이 합성 출력은 위치/분할의 보조 확인이다. 글꼴·자간·표 테두리의 한컴 일치 판정이 아니다.
문서 전체 선택 경로, 실물 기준 PDF와의 Native/fresh WASM Visual Sweep 및 Studio 검증은
미검증이다. WASM lib lint도 fresh WASM 실행을 대신하지 않는다. 전체 workspace CI 게이트와
전체 회귀는 이번 선택 검증에 포함하지 않는다. 원격 push/PR은 수행하지 않는다.

## 남은 연결

문서 IR의 자식 컨트롤 소유·앵커와 저장 줄 수용, 실제 문서 세션의 엔진 선택/페이지 상태,
WASM 출력 연결은 남아 있다. 이번 table-local session을 그 구현 완료로 보고하지 않는다.
