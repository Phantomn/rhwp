# Task #7353 — U1 쪽 문맥·구역 상속

- 승인: 2026-10-01, 구현계획 5.5 U1. 시작 head `99cdb22f6`.
- 범위: 쪽번호 시작/계속, 텍스트·쪽번호 필드 머리말/꼬리말의 선언·홀짝·상속·감춤.
  각주/미주, 바탕쪽 확장, 내부 표/복합 도형은 제외한다. 자동 Legacy fallback은 없다.
- 독립 근거: HWP5 사양 표129의 쪽번호 0=앞 구역에 이어/n=임의 시작,
  표140/141의 머리말·꼬리말 문단 목록/영역/홀짝 적용. 정상 한컴 원본
  `samples/issue6864/header-justify.hwp`와 `pdf/issue6864-header-justify-2020.pdf`를 재사용한다.
  복수 구역 작성 대조군은 생성 입력과 한컴 저장/출력을 구별해 보존한다.

## 구현 전 경로와 반례

`SectionDef.page_num → HostedDocumentLayout::prepare(first_number) →
HostedSectionLayout::prepare → typeset_hosted_section fit의 page_number →
PageContent.page_number → PageNumberStory/셀 page field/최종 RenderTree`.
현재 section의 `page_num > 1` 거부와 PageNumberStory의 시작번호 가산이 제품 번호 전달과
충돌한다. 번호 원점과 물리 인덱스를 분리하고 최종 확정 번호를 그대로 소비시킨다.

`Header/Footer IR → 문서 소유 선언 상태 → 실제 수용한 본문 줄의 선언 활성화 →
V2 TextComposer의 줄/ParagraphEnd → Header/Footer 노드 → Native/WASM/제품 조회`.
Legacy 머리말/꼬리말 재측정은 사용하지 않는다. 불변 payload의 실제 줄 끝으로 내용 높이를
정하고 그 결과로 세로 정렬/최종 원점을 정한다. 구역별로 선언 상태를 초기화하지 않는다.

반례: 시작번호가 1이 아닌 구역, 이어받기/재시작, 홀짝과 양쪽 선언 혼재, 뒤 페이지 선언의
조기 활성화, 구역 첫 쪽 감춤 뒤 다시 표시, 상속된 원본 구역 소유, 빈 줄과 여러 줄 콘텐츠,
지원 밖 내부 표 거부 시 부분 결과 미공개. 편집/저장/재열기는 읽기와 별도 검사한다.

## 상태

U1 구현·집중 검증·fresh WASM·실제 Studio 편집 확인까지 수행했다.
2026-10-01 메인테이너가 제시된 U1 시각 검증 묶음을 통과로 판정했다.
U2/U3는 시작하지 않았고 구현 코드는 아직 미커밋이다.
필수 Rust lint 묶음은 통과했다. 전체 nextest는 10,888 PASS / 2 FAIL / 50 skip이며,
실패 두 건은 아래에 분석했다. 전체 통합 통과는 아니며 후속 Skia/doc 게이트는 중단했다.

## 구현 결과와 실제 소비 경로

- `host_stories.rs::StoryContext`를 문서가 소유하고 구역 사이에 전달한다.
  `owner_line → HostedParagraph.fragment.contains_line → 선언 활성화 →
  ActiveHeaderFooter.active(확정 표시 번호)`로 실제 수용한 줄에서만 선언을 반영한다.
  뒤쪽 선언이 앞쪽 쪽에 조기 적용되지 않도록 정식 계약으로 보호했다.
- `PageNumberStory::render_number`는 확정된 표시 번호를 그대로 소비한다.
  명시 시작 번호를 다시 더하지 않으며 기존 셀 쪽번호 필드와 같은 번호를 사용한다.
- `TextComposer → ParagraphEnd.occupied_end/next_origin → 선언 영역 내 정렬 →
  최종 Header/Footer 자식 좌표`를 사용한다. 본문 페이지 skeleton에 전달하는 HF 참조는
  비워 Legacy HF paint를 실행하지 않고 V2 노드를 추가한다. 원본 section 소유 주소는
  노드에 보존하고, 편집 preview도 같은 story renderer를 사용한다.
- 편집 명령이 Legacy reflow 행을 저장된 원본 행처럼 clean 처리하던 경로를 분리했다.
  V2는 입력을 invalidate하고 공통 composer에서 새 줄을 구성한다.
  `edit-probe.log`의 dirty=false/구현용 row tag/RenderError에서
  `edit-probe-after.log`의 dirty=true/rows=[]/정상 render로 바뀌었다.
  소스 문서 IR을 paint 중 수정하지 않는다.
- 이번 수정은 본문/셀 fragment의 컷·예약 높이·rowspan 알고리즘을 바꾸지 않는다.
  이어받기 관련 변경은 선언 소유 줄과 구역 문맥이며, 본문·셀 정상 대조 15건을 실행했다.

## 입력과 검증 증적

증적 루트는 `output/7353/u1/`이다.

| 검사 | 실행 및 결과 |
|---|---|
| 정식 Rust 계약 | `node scripts/rust-test-suite-manifest.mjs --prepare` 후 `CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/home/edward/mygithub/rhwp/target/pr-review cargo test --locked --profile release-test --test regression_suite_012 --test regression_suite_025 issue_7353_product -- --nocapture`: 신규 12 + 기존 15 PASS, 0 FAIL. 신규에는 환경변수 지정 시 실행하는 증적 helper 2건이 포함되어 동작 계약은 10건. `regression-final.log` |
| 수정 전후 | 정상 원본 header가 미지원으로 거부되던 `before.log`와 구현 후 정식 원본 계약 PASS. 초기 합성 입력 일부는 작성 방식도 정정했으므로 동일 입력 red/green 근거로 세지 않는다. 편집 reflow 결함은 위 probe의 동일 입력 전후를 별도로 보존 |
| 정상 한컴 대조 | `tests/fixtures/issue7353_product_stories/README.md`의 생성 입력 → 한컴 저장 HWP → 동일 HWP의 PDF. 6쪽 번호·홀짝·감춤, 영역 원점 y=20/40/160px를 독립 속성과 PDF로 확인. 정상 원본 header의 PDF x=85.033915pt도 검사 |
| fresh WASM | `docker compose --env-file .env.docker -f docker-compose.yml -f output/7353/wasm-product/final/compose-network.yml run --rm --no-deps wasm`: 성공, `wasm-final-fixed.log` |
| Native/WASM 제품 비교 | `node scripts/verify-product-v2-wasm.mjs --pkg pkg --input <동일 HWP> --out output/7353/u1/{header,stories} --default-v2 --hf-addresses`: 각각 1쪽/6쪽 SVG·RenderTree·Canvas layer 일치. `wasm-header.log`, `wasm-stories.log`. HF의 usize 주소만 Native64→WASM32로 대응하며 좌표·글리프·일반 ID는 변경하지 않음 |
| 실제 Studio | `CHROME_CDP=http://localhost:19222 VITE_URL=http://localhost:7700 node ../output/7353/u1/studio.mjs --mode=host` (cwd rhwp-studio): 일반 파일 열기 V2, 키보드 편집, undo/redo, 선택 영역, 상속 표시, HWP 저장/재열기 PASS. `studio-final.log`, `studio/result.json`, `studio/header-edited.png`, `studio/edited.hwp` |
| 형식 | `cargo fmt --all -- --check`, `git diff --check`: PASS |

Studio 첫 실행의 키 입력 실패는 모드 전환 직후 포커스 타이밍이었다. 전환 완료 뒤 다시
focus한 실행에서 편집·undo/redo가 통과했다. 이 결과를 위해 Studio 제품 코드는 바꾸지 않았다.

## 시각 검증 준비

`/home/edward/mygithub/rhwp/venv/bin/python output/7353/u1/sweep.py`는 canonical
Visual Sweep의 비교/overlay/review 기능과 webfont rasterizer로 Native/fresh WASM SVG 및
실제 Canvas 캡처를 동일 PDF에 대조한다. 원본 header 1쪽과 작성 대조본 1~6쪽을 산출했다.

- 원본: `header/visual/wasm/review/review_001.png` — TEST TEXT의 시작 위치와 자연 폭.
- 대조 1쪽: `stories/visual/wasm/review/review_001.png` — 시작 번호 7, 머리말·ODD.
- 대조 3쪽: `stories/visual/wasm/review/review_003.png` — 다음 구역 번호 9와 상속.
- 대조 5쪽: `stories/visual/canvas/review/review_005.png` — 번호 3 재시작, 머리말만 감춤.
- 대조 6쪽: `stories/visual/canvas/review/review_006.png` 및
  `stories/visual/wasm/overlay/overlay_006.png` — 머리말 복귀와 EVEN.

위 대표 review와 standalone overlay를 직접 열어 본문 빈 줄, 머리말/꼬리말 영역,
번호·상속·감춤을 확인했다. 글꼴/래스터 차이는 남으며 특히 희소 페이지의 낮은 ink match를
자동 실패 또는 자동 통과로 해석하지 않았다. 2026-10-01 메인테이너가 표시 번호
`7, 8, 9, 10, 3, 4`의 시작·계속·재시작 구성을 확인한 뒤 “시각 판정 : 통과”로 판정했다.
이 승인은 제시한 U1 검증 범위에 한정하며 아래 미지원·미검증 범위를 해소한 것으로 보지 않는다.

## 검증 소스와 남은 범위

- 기반 head: `99cdb22f626566c5d20e284de214ca8992b3f7e2` + 이번 미커밋 변경.
- 변경 production Rust 8개 파일을 경로 순으로 `경로 + NUL + 내용 + NUL` 연결한 SHA256:
  `51f2ae0fde5cafd1abcad3379cf0264007b2e00943b62954473e9cae926a2aaf`.
  대상은 `header_footer_ops.rs`, queries `rendering.rs`, table_v2의
  `cell_page_field.rs`, `host_document.rs`, `host_section.rs`, `host_stories.rs`,
  `mod.rs`, `page_number.rs`다.
- `pkg/rhwp_bg.wasm` SHA256:
  `a04bf0f774ea6373dd269735c1bd814dba82d3036487a7f72d57ffec32198784`.
- 정상 HWP SHA256: `5ae9547dbccad4cdcb5b96702f5993a5bbdf259a143ff8e355b936cda9697770`.
- 기준 PDF SHA256: `9c75674d41afea86c9bb3c027b655291a389cbff1eb5390090c207e8ad4d5d63`.

현재 명시 미지원: HF 내부 표/도형·세로쓰기·복합 필드, 새로운 번호 형식,
fresh 문단 중간 텍스트 뒤 선언, 각주/미주·바탕쪽 확장. 이번 지원은 텍스트 및
단일 십진 쪽번호 placeholder에 한정한다. HF 가운데/아래 밴드 정렬의 전용 독립 시각
대조는 미검증이다. U2/U3로 범위를 넓히지 않는다.

시각 판정과 전체 검증 승인을 구분한다. 아래 별도 승인으로 전체 회귀/필수 lint를 실행했으며,
계약 정정·재검증 후 커밋·게시 절차가 남아 있다. push 승인은 이번 검증 승인에 포함하지 않는다.

## 승인된 전체 회귀·lint 실행

2026-10-01 메인테이너가 전체 회귀·필수 lint 실행을 별도로 승인했다. 커밋·push는
포함하지 않는다. fetch한 정책 base는 `02530b9ed567a44663edb26c65fb565c4a79f00d`로
기존 값과 같으며 작업 브랜치에 devel을 병합하지 않는다.

`output/7353/u1/gates/run.sh`로 fmt → Native/WASM/workspace Clippy와 workspace build →
base 고정 manifest → release-test 전체 nextest → Native Skia 3종 → doctest를 순차 실행한다.
첫 실패에서 후속 명령을 중단하며 원장·기대값·ignore는 변경하지 않는다.
이번 준비 결과는 1,472 sources / 28 suites + 20 exceptions = 48 integration targets다.
기존 공유 target `/home/edward/mygithub/rhwp/target/pr-review`를 재사용하며
host 16 CPU / 가용 RAM 약 21GiB에 Cargo jobs=2, nextest threads=4로 제한했다.

현재 source-side `#[cfg(test)]` 변경은 없으며 unit-tier 추가 게이트는 비해당이다.
Studio source는 변경하지 않았고 동일 production Rust digest의 fresh WASM/Studio/시각 검증은
위 결과를 재사용한다. 이후 production source가 바뀌면 해당 증적은 다시 검증한다.
전체 통과로 판정하지 않으며 실행 결과는 다음과 같다.

| 게이트 | 결과 | `output/7353/u1/gates/` 증적 |
|---|---|---|
| fmt / fmt check | PASS; production digest 불변 | `fmt.log`, `fmt-check.log` |
| Native root Clippy | PASS | `clippy-native.log` |
| WASM32 lib Clippy | PASS | `clippy-wasm.log` |
| workspace build | PASS | `workspace-build.log` |
| workspace all-target Clippy | PASS | `clippy-workspace.log` |
| base 고정 manifest 정책 | PASS | `manifest.log` |
| 전체 release-test nextest | **10,888 PASS / 2 FAIL / 50 skip**, 78 binaries. 빌드 11분, 실행 663.451초. 종료 코드 100 | `nextest.log` |
| Native Skia 3종 / doctest | 앞 게이트 실패에 따라 미실행 | `status.log`의 nextest 종료에서 중단 |

기존 W3 전체 결과 10,878 PASS / 50 skip 대비 U1 테스트 12개가 추가되어 실행 대상은
10,890건이다. U1 신규 12개는 모두 PASS이며 기존 거부 계약 2개에서 실패했다.
기존 W3 결과는 비교 자료이지 이번 소스의 미실행 Skia/doc 게이트를 대신하지 않는다.
환경은 Rust 1.93.1 / nextest 0.9.137이다. 권장 nextest 0.9.140 미만 경고 및
미사용 JUnit 설정 키 경고는 환경 차이로 기록하며 원격 CI 동등 환경이라고 주장하지 않는다.
전체 테스트 완료 뒤 실행기를 중단했고 새 구현·테스트 변경, commit/push는 하지 않았다.

### 검출한 기존 거부 계약과 U1 지원 확대의 충돌

- `issue_7353_hosted_page_number::unsupported_format_or_mid_paragraph_declaration_remains_explicit`
  (`tests/cases/issue_7353_hosted_page_number.rs:158`): kind=0은 실제 문단 중간 선언이 아니라
  선두의 동일 PageNumberPos 2개다. 첫 문자 offset=16, 저장 줄 시작=0이며 두 제어는
  0/8 슬롯을 소유한다. 종전 `validate_body_entry`는 앞에 다른 PageNumberPos가 있다는
  이유로 거부했지만 U1 `owner_line`은 실제 소유 줄을 판별하여 수용한다.
  같은 테스트의 kind=1(format=1)은 별도 probe에서 계속 명시 거부됨을 확인했다.
  `gates/probe-declaration.rs` / `.log`에 동일 입력별 관측을 보존했다.
- `issue_7353_hosted_master::unsupported_first_page_flags_remain_explicit`
  (`tests/cases/issue_7353_hosted_master.rs:215`): `hide_header=true`의 무조건 거부를 기대한다.
  U1은 이 속성을 지원하며 정상 한컴 6쪽 대조본의 5쪽 감춤/6쪽 복귀 계약과 시각 승인이 있다.

두 실패는 이번 지원 확대 시 기존 거부 계약을 함께 갱신하지 못한 항목이다. 표시상의
회귀가 실행으로 확인된 것으로 보고하지 않는다. 이번 검증에서는 production 코드,
원본 테스트·baseline·ignore를 수정하지 않았다.

권고 정정은 기존 거부 assertion을 단순 삭제하는 것이 아니다. 선두 복수 선언은 소유 줄과
실제 쪽번호 위치/본문 불변을 확인하는 수용 계약으로 분리하고, 정말 미지원인 번호 형식 및
fresh 텍스트 뒤 선언은 거부 계약으로 유지한다. 첫 쪽 HF 감춤은 해당 속성 수용과 실제
감춤/복귀를 보호하며 바탕쪽의 기존 정상 대조를 유지한다. 정정 후 해당 계약과 전체 회귀,
남은 Native Skia·doc 게이트를 재검증한다.

### 우선순위 변경

2026-10-01 메인테이너가 미지원 기능에 의한 V2 문서 로드 실패 해소를 최우선으로 지시했다.
후속 작업은 구현계획 5.6을 따른다. 위 계약 2건의 정정과 전체 검증 반복을 먼저 진행하지 않는다.
보고된 공통 host control 거부 오류는 실제 샘플을 확인해 원인 컨트롤을 식별해야 하며,
현재 문자열만으로 복수 표 결함이라고 단정할 수 없다. 기존 U1 작업과 검증 증적은 보존한다.

이후 메인테이너는 `src/renderer/layout/table_layout.rs`를 더 이상 사용하지 않고 삭제해도
Studio가 정상 동작하는 것을 최우선으로 명시했다. 구현계획 5.7이 최신 우선순위다.
코드 조사에서 Legacy 타입/캐시·typeset의 직접 참조와 V2의 공통 LayoutEngine 컴파일 결합이
남아 있음을 확인했다. 기본값 V2 전환을 의존성 제거 완료로 보고하지 않는다.
이번 기록에서는 코드·파일 삭제를 수행하지 않았으며, 기존 U1 검증 범위는 그대로 보존한다.
