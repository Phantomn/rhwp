# PR #7518 메인터너 보정 계획

사용자는 2026-10-04에 그림 띠 2쪽의 잔여 문제도 메인터너가 해결하는 경로를 지시했습니다.
`re_review_required`에서 기여자 재작업만 요청하는 일반 절차 대신 이 명시 지시를 적용합니다.
초기에는 90% 예외 없이 검증했고, 이후 아래 사용자 시각 판정에 따라 76076 33·34쪽의
85% 수준 출력을 수용합니다. 자동 측정값과 전역 기준은 바꾸지 않습니다.

- 원 기여자 head: `02845752f76d5539c74df135d950ba5757bd1792` (`kidsnote/rhwp`, `fix/trailing-space-line`).
- 현재 통합 base: `1d6bc70767fad365b07afe4ef57972d23b140f2b` (`upstream/devel`, #7567 포함).
  아래 이전 회차의 `8497729b4fb0e071c484fc5740f9bb2400bed437` 검증은 역사 기록입니다.
- 현재 코드 통합 후보: `3d9239eeec32fc60ee188c3f3bc0d9ec5094ec3a`.
  이전 통합 코드 `3d23545846942137256bc95afbdd8c6390fade42`의 결과는 아래 회차별로 구분한다.
  최초 후보 `55a2800aadc32fc85ed4aa3e8e17f6b169f3b0b0`, tree `40c2a2d91e2b2023c3110f5635ac95cca2e53820`는 보존합니다.
- 검토 PDF·진단 입력·대표 PNG 보존: `5eb671068d74e2757daf12925017f5f6cdbdd38a`, `d8b98bd325a8233b40430e36a5c3db10283e9889`.
- 유지할 작업공간: `/tmp/rhwp-pr7518-review-20261004`, 현재 `integration/pr7518-maintainer-20261004`.
  이전 `review/pr7518-20261004`는 checkpoint `44ca6f0f9`로 보존합니다. 주 작업공간의 기존 #7494 branch와 #7353 worktree는 보존합니다.
- 처리 경로: [`collaborator_external_pr.md` 9.1.1](../manual/pr_review/collaborator_external_pr.md#911-기본-작업공간-devel-기반-체리픽-통합-검토)의 별도 통합 PR입니다. 원 기여자 이력은 재작성하지 않습니다.
  현재 통합 후보에는 devel의 추가 보정과 독립 자료가 있으므로 이 이력을 기여자 fork에 그대로 push하지 않습니다.
  사용자가 명시한 경로에 따라 본 저장소에 통합 PR을 만들고, 그 PR이 merge된 뒤 원 PR #7518에
  통합 링크를 남겨 close합니다. 지금은 local 수정·검증 단계이며 원 PR을 먼저 닫지 않습니다.

## 보정 전 증거와 가설

원래 수정은 focused 4/4 PASS입니다. 현재 devel 코드(최종 #7563 code와 동일)에서는 같은 실제 출력 조건이
4/4 FAIL이며, TAC 문서 2→1쪽·그림 띠 3→2쪽의 개선을 확인했습니다.
Native 전쪽 실루엣은 TAC 98.65641%, 그림 띠 p1 98.12095%, p2 62.05279%입니다.
그림 띠 두 쪽의 render tree는 이전 원 #7518 exact-head 결과와 동일합니다.

한컴 2020 PDF의 그림 띠 p2는 바깥 표 y=58.50, 행 5 아래끝=243.41, 내부 표 x=216.84/y=128.34,
다음 행 아래끝=275.54, 전체 아래끝=521.67px입니다(96dpi).
보정 전 rhwp는 바깥 표 y=56.7, 행 5 아래끝=180.6, 내부 표 x=230.9/y=87.9,
다음 행 아래끝=212.7, 전체 아래끝≈460.1px입니다.

현재 가설은 내용 컷으로 표현하지 않은 선언 행의 남은 물리 공간과 마지막 문단 기준 floating 표의
오프셋·여백을 continuation이 같은 원장으로 소비하지 않는다는 것입니다. 좌표 이동 상수를 추가하지 않습니다.

## 구현·검증 순서

1. 원본 row 5의 선언 높이·실제 유닛·첫 조각 예약 높이·다음 조각 요구 높이와 부분 배치의 최종 정렬을 추적합니다.
2. 측정/컷/예약/배치가 공통 물리 높이와 표 원점을 소비하도록 승인된 범위의 보정 commit을 만듭니다.
3. 원본 `tests/cases`에 실제 마지막 경계·내부 표·다음 행 좌표 검사를 추가합니다. 보정 전 FAIL / 보정 후 PASS와
   정상 대조군을 확인합니다. 합성 페이지 예산 입력은 독립 한컴 출력이 없는 계약 진단과 구분합니다.
4. 영향 페이지 Native 직접 sweep으로 먼저 방향을 확인합니다. 이후 필요한 lint·회귀·fresh WASM·CDP를 순차 실행합니다.
5. 검증한 head의 review·본문·대표 이미지·남은 차이를 준비한 뒤 승인된 원격 작업을 수행합니다.

## 이전 검증 결과 — 720ebcd60

검증한 code head는 `720ebcd6005f330fa5d7aa22ab7fa9c816965e81`입니다.
일반 행의 선언 물리 tail, 최초 소유의 문단 lead/바깥여백, 전체·부분·재귀 내부 표의 공통 원점과
child cut의 실제 패딩을 보정했습니다. scratch LayoutEngine에도 실제 PageLayoutInfo를 캐시 준비 전에
공급했습니다. NO_LS만으로 canonical 투영을 강제하던 중간 가정은 제거했습니다.

- Focused 21개와 전체 nextest 10,278개가 통과했습니다(전체 50 skip).
- fmt, Native/WASM/workspace-all-targets Clippy, workspace build, Skia 세 검증과 고정 base 정책 검사도 통과했습니다.
- 원본 세 쪽의 Native/fresh WASM 실루엣은 각각 98.65641%, 98.12095%, 92.96635%로 정상 90% gate를 통과했습니다.
- 요청에 따른 CDP 재검증은 캐시를 끈 새 탭에서 11/11 PASS였습니다. 응답 WASM과 pkg/Studio 파일 해시가 일치했습니다.
- 최종 같은 검사로 원 통합은 0/9 PASS, 중간 보정은 6/9 PASS, 최종 보정은 9/9 PASS였습니다.

당시에는 통합 조건이 미충족이었다. 이후 사용자의 지시에 따라 추가 보정을 계속한다.
추가 공개 대조 문서 p33/p34의 Native 점수는 80.90046%/52.62315%이며,
합성 신규 렌더링 회귀 후보에는 독립 기준 PDF와 필수 Native/fresh WASM 시각 증거가 부족합니다.
추가 대조 WASM은 반복 글꼴 임베딩 저장 중 공간 부족으로 미완료입니다. 같은 원인으로 중단된 Skia 빌드는
제가 만든 실패 SVG만 정리한 뒤 재실행해 통과했습니다. 상세 실패 로그도 보존했습니다.
코드·검사 후보를 유지하고, 부족한 증거를 계약 검사 통과로 대신하지 않습니다.

[최종 리뷰](pr_7518_review.md)에 실제 생산/소비 경로·입력 해시·검증 결과·직접 확인한 PNG와 보류 해제 조건을 연결했습니다.
원격 push·PR 생성·comment·merge는 수행하지 않았습니다. 보류 해제 뒤 검토 가능한 후보로 다시 제시합니다.

## 추가 보정 착수 — 사용자 지시 이후

보류 판정을 종료점으로 삼지 않고 추가 대조 문서와 합성 경계의 독립 출력까지 해결한다.
공식 비동기 한컴 client 0.9.0의 `start → status → download`로 합성 원본 13개를
engine 2020에서 변환했고 입력·PDF 해시와 job ID를 로컬 증적에 보존했다.
원격 endpoint·토큰과 글꼴 파일은 공개 증적에 포함하지 않는다.

- 용지 높이만 500~700px로 줄인 입력은 portrait 방향과 저장 폭/높이가 모순된다.
  한컴 PDF가 가로·세로를 바꾼 실패 원본과 PDF를 보존하고, 여섯 대조군을 별도
  `valid_orientation/`에 생성한다. 저장 폭/높이를 교환하고 landscape/attr bit 0을 켜며,
  rhwp의 유효 용지와 본문 크기는 동일함을 검사한다. 원문/줄/개체 속성은 바꾸지 않는다.
- p34 첫 가시 문단은 원래 child 문단 9이며 앞 간격 1000 HU를 가진다. CellUnit은
  이 간격을 첫 줄에 예약하지만 1×1 continuation 배치는 column-top에서 버린다.
  컷이 첫 줄을 소유한 재조판 문단의 간격을 측정과 배치가 함께 소비하게 한다.
  문단 중간의 continuation, 원 셀 첫 문단, 유효 저장 줄 경로는 별도로 대조한다.
- 2024 기준의 한양중고딕은 Type 3이고 현재 SVG 임베더는 HCR Dotum을 우선한다.
  H2GTRM.TTF를 실제 공급해도 기존 선택 정책은 HCR을 유지했다. 파일 부재만으로
  설명하지 않고 PDF 프로그램/선택 정책과 줄·표 위치 차이를 구분해 검증한다.

진행 중 결과는 최종 통과로 승격하지 않는다. 코드 변경 뒤에는 최종 head에서 필수
회귀·lint·fresh WASM·CDP와 Native/WASM 직접 시각 검증을 다시 실행한다.


### 문단 간격과 빈 host 점유 보정

빈 재조판 block-table host는 글줄과 표의 합이 아니라 같은 원점의 점유 합집합을
사용한다. 별도의 실제 빈 문단은 보존한다. `reflow_block_table_host_occupied_height`
생산 결과를 HeightMeasurer의 행/rowspan/MeasuredCell 높이와 전체 셀 배치가 소비하며,
MeasuredCell의 줄 메트릭은 재귀 CellUnit 원장과 구분해 유지한다. 조정 가능한 다중
재조판 행은 캐시 object 높이를 최소 행 높이로 확대하지 않는다.

기존 generic fit이 내용 하한 때문에 균일 축소를 거부한 경우에도 기존 tail-only fit을
소비해야 한다. `fit_measured_table_to_declared_height_with_outcome`이 이 거부 이유를
반환하고 typeset이 원래 fit 범위 안에서 tail-only 경로를 호출한다. paint가 사용하는
같은 tail fit과 예약 높이를 맞췄으며 fit 범위나 회귀 기대값은 완화하지 않았다.

독립적인 원본 HWP의 fresh Hancom 2024 PDF에서 p34 마지막 행은 147.6784px이다.
기존 165.5px가 새 배치에서 147.9px가 되었고 별도 13pt 빈 문단은 유지된다.
첫 문단 간격 및 host 회귀는 기존 720ebcd 코드에서 각각 의도한 좌표 원인으로
실패했다(`spacing-negative-720.log`, `host-negative-720.log`). 수정 후 11개 경계와
기존 #2308 5개/#3128 2개를 합친 18개가 모두 통과했다.
증적: `output/pr-review/pr7518-20261004/logs/host-occupancy-fit-outcome.log`.
다음 단계는 내용 완료 컷과 남은 물리 tail의 정렬, 유효한 follower 흐름 입력 검증이다.


### 완료 내용 컷의 정렬

끝 컷 벡터의 유무 대신 배치에서 사용하는 `cell_cut_window`와 실제 CellUnit 수로
남은 내용이 완전히 소비됐는지 확인한다. 선언 물리 tail이 다음 쪽에 남아도 이 조각에
남은 내용 전체가 들어가면 원래 세로 정렬을 보존하고, 실제 내용 컷은 Top을 유지한다.
독립 Hancom 2020 `terminal-tail` p2 내부 표 top=537.17px와 가운데 점유 불변식을
정식 회귀에 추가했다. 기존 720ebcd에서는 이 좌표 검사로 FAIL, 수정 후 19개
경계/정상 대조군이 PASS다. Native 직접 review p2에서 외곽과 내부 표의 정렬을
확인했으며 3쪽 모두 gate PASS(최저 90.87076%, p2 99.12%)다. 아직 최종 head의
WASM 증적이 아니며 다음 단계에 다시 검증한다.

### nested-split 2쪽 공백 셀과 3쪽 그림 이어받기

사용자가 제공한 한컴 화면과 독립 2020 PDF를 기준으로 먼저 이 경계를 보정했다.
입력은 `valid_generated/nested-split.hwp`이며 원래 `valid_orientation`의 NO_LS
대조군도 같은 정식 검사를 실행했다. 원 입력의 저장 줄이 무효였다는 가정은 하지 않는다.
Hancom 저장본에서 행 5 및 재귀 자식의 LineSeg만 제거한 입력의 생성·변환 출처는
`tests/fixtures/pr7518_review_page_budget/valid_generated/provenance.json`에 연결했다.

실제 원문 행 4 / 열 1은 문단 공백 51개와 세 개의 **non-TAC Square 그림**이다.
그림은 Para/Top, flow_with_text이며 두 개는 유효한 signed 음수 오프셋을 가진다.
저장 줄이 있는 경우 이 음수 오프셋을 근거로 공통 띠를 해체해 높이를 합산하던
가정을 제거했다. 실제 약 247px인 띠를 분할 장부가 약 729px로 예약하던 결함이다.

생산·소비 경로는 다음과 같다.

| 단계 | 실제 경로와 계약 |
| --- | --- |
| 공통 그림 띠 | `float_placement::parallel_cell_float_band_height` / `parallel_cell_picture_band_height`: signed 구간이 겹치는 동일 문단의 띠. 명시 개행·가시 글자·TAC·다른 세로 띠는 이 단일 host 계약에 들어가지 않는다. |
| 전체 측정 | HeightMeasurer와 layout의 셀 visual bottom이 같은 picture band를 소비한다. |
| 내용 컷 | `cell_units_uncached`: host 줄 소유와 모든 그림 control을 소유한 불가분 띠를 분리한다. 둘이 공유하는 줄 전진량을 그림 띠로 옮겨 전체 점유를 중복하지 않는다. |
| 앞 조각 예약 | `parallel_picture_row_opening_height`를 rowspan ordinary scanner가 조회한다. 줄만 소비하고 그림이 남은 컷에 원본 셀 최소 높이 7026 HU를 보존하며 실제 잔여 예산을 검사한다. |
| 이월 | continuation `fragment/emit.rs`가 같은 컷을 조회하고 남은 그림 유닛 높이 전체를 이월한다. 내용 상자에서 빈 물리 opening을 빼던 후속 덮어쓰기를 적용하지 않는다. |
| 실제 배치 | partial table은 같은 parallel picture owner의 문단 시작 원점을 사용한다. 그림을 문단 조판 완료 후의 원점에서 시작하지 않는다. |

독립 PDF의 p2 blank rules는 326.97/420.48px, p3 picture-cell rule은 304.63px,
그림의 실제 보이는 상단은 60.32px이다. Native는 각각 326.1/419.8px,
303.7px, 58.6px이다. p3의 다음 행은 timeline 문단 9개를 소유하고 p4에 반복하지 않는다.
`a_deferred_picture_band_keeps_the_blank_cell_and_the_next_row_on_its_page`는
이 최종 좌표·소유·그림 하단·그림 전체 1회 배치를 검사한다. 이전 immutable CLI는
p2 셀 부재로 FAIL, 보정 CLI는 원래 NO_LS/저장 주변 프레임 두 입력에서 PASS다.

### 마지막 실제 빈 줄의 물리 셀 윤곽

p6의 빈 셀도 실제 원본 문단 두 개의 소유다. GDB로 기존 코드의 실제 CellUnit
14/15를 확인했다: p12는 17.06667px, p13은 10.66667px, 둘 다 vis=0..1이고
empty_spacer=true다. 이어받기 walker가 글자가 없다는 이유로 이 양수 공간을
무높이로 소비해 기존 p6 윤곽을 없앴다. 진단 로그는 `nested-gdb-tail-units.log`에 보존했다.

`empty_unit_owns_flow_line_box`는 실제 재조판한 빈 줄과, 다음 문단이 NO_LS일 때
정확한 직전 저장 슬롯으로 원점이 입증된 빈 줄을 보존한다. ordinary/block walker와
예약 패딩이 같은 소유 판정을 소비한다. 원래의 저장 overlay 판정은 별도로 유지한다.
두 줄과 안 여백의 합 `(800+480+800+282)/75=31.49333px`가 p6에 남으며
독립 PDF의 31.44px과 일치한다. 새 정식 검사 `trailing_empty_paragraphs_keep_their_continued_cell_outline`은
이전 CLI에서 5쪽으로 FAIL, 수정 후 6쪽과 실제 셀 높이로 PASS다.

위의 code는 `c90bc800d` 이후 **미커밋 보정**이다. renderer diff SHA와 입력·PDF·CLI SHA,
명령 및 판정은 `output/pr-review/pr7518-20261004/nested-tail-p1-p6/capture-provenance.json`에 고정했다.
정식 전후 로그는 같은 output의 `logs/picture-current-{positive,negative}.log`와
`logs/empty-tail-current-{positive,negative}.log`다. 이전 Native 네 쪽 직접 판독은
p2 99.72190%, p3 98.78619%, p4 97.68812%로 개선됐지만 최종 새 코드에서 전쪽을 다시 캡처했다.

**현재 범위의 결과와 남은 문제를 구분한다.** p2 공백 셀/p3 그림 및 다음 문단/p6 빈 줄
경계는 정식 검사 PASS다. 전쪽 Native 직접 비교에는 p4 자식 표 아래 물리 경계와
p5 이어받기 시작/후속 행의 약 5px 차이가 남고 p5는 89.20899%다. 전체 gate는
`re_review_required`이며 90% 예외·허용치 완화는 적용하지 않았다. 추가 focused 집합은
12 PASS / 4 FAIL이다: auto-row/mixed-cell의 조각 소유와 page budget,
valid follower의 nested replay 주장까지 독립 출력과 추가 대조가 필요하다.
이는 통합 완료·승인 증거가 아니다.

저장소 루트에서 `CARGO_TARGET_DIR=/home/edward/mygithub/rhwp/target/pr-review scripts/wasm-pack-locked.sh --target web --out-dir pkg`로 fresh WASM을 만들었다.
`pkg/`, Studio `public/`, 캐시를 끈 CDP 브라우저의 실제 응답 SHA-256은 모두
`493b7c110cff3e11397e4eff739ab112be8f6986b883f94554c9054d7c3ff023`이다.
현재 renderer diff SHA는 `b7ee00e7736a10de57321c7b48aa9283d23655e874ed89a8c087961c98148673`다.

fresh WASM Visual Sweep의 1–6쪽 export/compare/standalone overlay/review를 다시 생성했다.
`nested-tail-wasm-p1-p6/summary.json`과 그 아래 `run_manifest.json`에 명령·입력·PDF·source를 연결했다.
p2/p3/p6 review와 p3 standalone overlay를 직접 확인했다. Native와 같은 p2 99.72190%,
p3 98.78619%, p5 89.20899%, p6 100%이며 전쪽 gate는 여전히 `re_review_required`다.

사용자가 준비한 Chrome CDP에서 별도 작업용 Studio 7718과 새 탭을 사용했다.
`valid_orientation`/`valid_generated` 두 입력의 실제 browser render tree로
p2 빈 opening, p3 전체 그림 및 다음 9문단, p4 문단 재등장 부재, p6 빈 셀을 검사했다.
결과는 **16/16 PASS**이며 `cdp-nested-picture/result.json`과 `*-trees.json`에 남겼다.
실제 페이지 오프셋으로 이동한 `valid_generated-page-2.png`/`valid_generated-page-3.png`도
직접 확인했다. 이 CDP 통과는 위 추가 focused 4 FAIL을 해소하는 증거가 아니다.
현재 코드의 `cargo fmt --all -- --check`, Native Clippy `-D warnings`, `git diff --check`는 PASS다.
WASM Clippy·workspace/all-targets·최종 전체 회귀 등 제출 전 검증은 아직 완료되지 않았다.


### 사용자 재검토: 3쪽 그림 상단 여백

사용자 재검토에서 3쪽 그림의 top padding 차이가 지적되어 셀 테두리와 본문 원점을
각각 추적했다. 앞선 약 2px 절대좌표 검사는 이 원점 차이를 검출하지 못했다.
원본 outer table의 바깥 위 여백과 기본 셀 top padding은 각각 141 HU다.
기존 최종 좌표는 본문/셀 상단 56.7px, 그림 상단 58.6px였고, 그림과 셀 사이에는
실제로 약 1.88px 안여백이 있었다. 독립 PDF는 셀 상단 58.40px, 그림 상단 60.32px다.
빠진 값은 공백 host 조각 뒤에서 그림 띠를 이어받는 **표 프레임의 바깥 위 여백**이다.

`parallel_picture_row_opening_height`의 소유 컷을 budget과 partial paint가 함께 조회한다.
본문 최상단에서 이어받는 ordinary cut이며 실제 그림 continuation 높이가 전달된 경우
새 바깥 프레임을 열고, 기존 셀 top padding을 이어서 적용한다. 기존 top margin 경로와
중복 예약하지 않는다. 첫 조각·block cut·중첩 셀·쪽 중간 조각은 이 새 프레임 경로에
포함하지 않는다.

생산·소비 경로: `table_layout.rs::parallel_picture_row_opening_height`의 host/band 소유 컷
→ `fragment/budget.rs`의 `host_before_overhead`와 행 예산
→ `fragment/emit.rs`의 실제 흐름 전진
→ `table_partial.rs`의 table `y_start`
→ 기존 `cell_y + pad_top` 및 `para_y_before_compose`
→ 최종 `fragment_owned_square_flow` picture anchor. 셀 padding을 두 번 더하지 않는다.

진단 CLI `rhwp-picture-top-frame-probe`의 Native 직접 review/overlay에서
셀 상단 58.6px, 그림 상단 60.5px을 확인했다. 두 독립 입력
`valid_orientation`/`valid_generated`의 p2/p3 sweep은 각각 최저 99.65660%다.
p2 render tree JSON은 수정 전 `nested-tail-p1-p6`와 바이트까지 동일하다.
증적은 `picture-top-frame-native`/`picture-top-frame-original-native`이며, fresh WASM과
정식 상대 여백 검사 결과는 아래에 이어 기록한다. 이는 앞서 남은 p4/p5 차이와
추가 focused 4 FAIL을 해결했다는 판정이 아니다.


현재 보정의 fresh WASM 및 회귀 결과를 다음과 같이 고정한다.

- renderer diff SHA: `f0ea459498b8494dc6e53bbabedf9ffcb817abbb36b98fc205a1d9b11b92faa7`
- fresh WASM SHA: `15e9fe49972759bf0049621c9cb4aae0e428fc1fbfde139ec5d74ace49d7548d`.
  root wrapper 성공 후 pkg/public/CDP 실제 응답이 모두 일치한다.
- `picture-top-frame-wasm` 및 `picture-top-frame-original-wasm`: 두 입력의 p2/p3
  Native/fresh WASM 범위 모두 gate PASS, p2 99.72190%, p3 99.65660%.
  p3 review/standalone overlay와 실제 CDP p3 캡처를 직접 판독했다.
- `cdp-picture-top-frame/result.json`: **20/20 PASS**. 각 입력에서 본문→표 프레임의
  원본 바깥 위 여백과 셀→세 그림의 원본 안여백을 각각 검사했다.
- 이 시각 선행 조건을 확인한 뒤 기존 정식 picture case를 보강했다. HWP parser로
  읽은 실제 source table/cell padding을 사용해 두 상대 간격을 검사한다.
  보정 전 immutable `rhwp-nested-tail-frame-probe`에서 outer-top 관계로 FAIL,
  새 `rhwp-picture-top-frame-probe`에서 두 입력 모두 PASS다.
  로그: `picture-top-frame-formal-{negative,positive}.log`.
- 새 source로 prepare한 `regression_suite_020`의 focused 16개는 **12 PASS / 기존 4 FAIL**.
  이외의 실패를 해결했다고 보고하지 않는다. `picture-top-frame-focused-formal.log`에 보존했다.
- fmt/Native Clippy/diff check PASS. 최종 제출용 세 Clippy·전체 회귀 등은 여전히 남아 있다.

입력/PDF/source/CLI/WASM 해시, 명령·manifest 및 테스트 전후 결과는
`output/pr-review/pr7518-20261004/picture-top-frame-provenance.json`에 연결했다.
전체 Native render tree를 대조하면 p3만 달라지고 p1/p2/p4/p5/p6는 이전 산출물과
바이트까지 동일하다. 따라서 이전 p5 gate 미달과 추가 4 FAIL은 남은 문제다.
이번 수정의 판정은 **3쪽 그림 원점의 빠진 바깥 위 여백 복원과 셀 안여백 유지 충족**,
PR 전체 승인·통합은 미완료다.


### 별도 통합 PR 준비 — 사용자 판정

2026-10-04 사용자가 수정 후 3쪽 그림과 2쪽 공백 셀의 **시각 판정: 통과**를
확정하고 이전과 같은 별도 PR 준비를 지시했다. 이 판정은
`picture-top-frame-provenance.json`의 두 입력 p2/p3 Native/fresh WASM 및
CDP 20/20 결과에 연결한다. p4/p5, 다른 fixture와 추가 focused 4 FAIL의
통과 판정으로 확대하지 않는다. 원 contributor head를 보존하고 최신 devel 기반
통합 후보에서 필요한 제출 검증과 최종 PR 본문·head 고정 시각 asset을 준비한다.
원격 push/PR 생성/원 PR close/merge는 아직 수행하지 않았다.

### 별도 통합 경로 확정과 최신 base 재검증

사용자는 권한 문제로 기존 PR에 메인터너 작업을 직접 반영하는 대신, **기여자 변경과
메인터너 보정을 담은 별도 PR을 merge한 후 원 PR #7518을 닫는 방법**이라고 명시했다.
통합 PR은 `edwardkim/rhwp`의 `devel`을 대상으로 한다. 원 PR은 통합 완료 전까지 열어 두고,
완료 후 merge된 통합 PR·기여자 credit·merge SHA로 고정한 증적 링크를 남겨 close한다.
원 #7518의 archive 검토 기록은 같은 통합 PR에 포함하고 통합 PR 번호용 중복 review 문서는 만들지 않는다.

최신 base `1d6bc70767fad365b07afe4ef57972d23b140f2b`에는 #7567의 RowBreak 변경이 포함되어 있다.
이 base에서 기여자 `b2bb249bcc00b0a8301075b62a8810d500f11de2`와
`02845752f76d5539c74df135d950ba5757bd1792`를 순서대로 cherry-pick했다.
각 결과는 `38fa6d211`, `ff732a5a6`이며 원 author를 보존했다.
이전 checkpoint `44ca6f0f9`와 최신 base의 `git merge-tree --write-tree`는 충돌 없이
tree `783c47a9cb7668b72f2e93821870ac88633cd225`를 만들었다. 같은 최종 tree의 추가 보정을
별도 commit `3d23545846942137256bc95afbdd8c6390fade42`로 통합했다.

해당 source의 Native 빌드·Native Clippy·WASM Clippy는 PASS다.
immutable CLI `output/pr-review/pr7518-20261004/rhwp-integration-3d2354584`로
`valid_generated/nested-split.hwp` 전쪽 Native 실루엣을 재실행했다.
p2 99.72190%, p3 99.65660%로 승인된 영역의 결과는 유지되며, p5는 89.20899%다.
focused probe는 12 PASS / 4 FAIL이고 최종 suite 재링크·전체 회귀·최신 source의 fresh WASM 검증은 남아 있다.
따라서 통합 브랜치 생성 완료를 PR 제출 준비 완료로 판정하지 않는다.

후속 진단에서 `nested-auto-row`와 `nested-mixed-cell`의 Native UNIT 소유는 p4 19개,
p5/p6 각각 20개였다. 같은 입력의 독립 PDF는 p4 18개, p5/p6 각각 19개다.
continuation 첫 UNIT의 Native 원점 60.5px과 PDF 66.24px도 다르다.
내용 줄 간격보다 실제 조각의 프레임·여백 예약을 먼저 추적할 근거로 기록한다.
`terminal-follower`는 자식 행 0–2와 행 3이 각각 다른 쪽에 있으므로 자식 fragment 수 2개만으로
재방출을 단정하지 않는다. 다만 전쪽 Native 실루엣에서 p3 74.08920%가 남아 있어,
기존 기대값 변경이나 전체 통과 판정의 근거로 사용하지 않는다.
증적은 `output/pr-review/pr7518-20261004/integration-*-tree`,
`integration-nested-split-native`, `integration-follower-native`와 `logs/integration-*.log`다.

### 사용자 전쪽 판독 — nested-split 페이지 분리 수용

2026-10-04 사용자가 현재 통합 후보 `3d23545846942137256bc95afbdd8c6390fade42`의
`valid_generated/nested-split.hwp` 전체 1–6쪽 PNG를 확인한 뒤,
“페이지 분리 처리는 한컴과 거의 동일하게 되어 있습니다. 앞쪽 페이지네이션 버그를 수정하면서
자연스럽게 해결되었네요”라고 판정했다. 해당 샘플의 페이지 분리는 사용자 시각 판정으로 수용한다.
앞선 브리핑의 4→5쪽 차이를 이 샘플의 페이지 분할 미해결 판정으로 계속 사용하지 않는다.

대조한 출력은 `output/pr-review/pr7518-20261004/all-pages-user-review/nested-split`의
`rhwp_png`, `pdf_png`, `review`, `overlay`이며 각각 6쪽을 새로 생성했다.
source SHA·입력/PDF hash·실행 조건은 같은 디렉터리의 `run_manifest.json`에 고정되어 있다.
사용자 판정은 페이지 분리의 수용이며 완전한 픽셀 일치 주장과 구분한다.
p5 실루엣 89.20899% 및 자동 `re_review_required`는 원래 측정값으로 보존한다.
자동 높이·단일 셀·terminal-follower 대조군의 결과 및 focused 4 FAIL을 이 판정으로
통과 처리하지 않으며, 다른 샘플의 실제 분할 차이와 검사 가정의 오류를 별도로 확인한다.

### 다음 보정: 76076 33쪽의 빈 host 표 원점

사용자가 지정한 원본 `samples/76076_regulatory_analysis.hwp`와 동일 대응 PDF
`samples/issue1891/76076_regulatory_analysis-2024.pdf`의 33·34쪽을 source `3d2354584`로
다시 출력했다. `regulatory-spacing-current-native/regulatory-spacing-current`에 Native
compare/overlay/review·render tree·입력/PDF hash를 보존했다. nested-split 1–4쪽도
같은 source의 `all-pages-user-review`로 다시 직접 확인했다.

원본 323·324번 문단은 저장 LineSeg가 없는 빈 host의 비-TAC TopAndBottom, Para/Top,
offset 0 표다. 두 표의 위·아래 바깥여백은 각각 566HU, 선언 본체 높이는 1300HU다.
325번 문단의 뒤 큰 표도 같은 host 계약이며 바깥 위 여백은 141HU다.
앞 일반 문단의 baseline은 Native 158.6px / PDF 158.72px로 일치하지만,
323·324 표 문자의 PDF baseline 196.00/228.48px에 비해 Native는 각각 약 7.55px 이르다.
큰 표의 PDF 상단 괘선은 240.217px, Native 상단은 238.5px다.
PDF font bbox의 yMin은 실제 glyph top과 다르므로 이 진단에는 PDF text origin과
괘선 path를 96dpi 좌표로 변환해 사용했다.

원인 경로는 `table/host_spacing.rs::resolve`가 outer-top을 before로 예약하고,
`format_table`·block fit이 본체+before+after를 소비하지만 빈 NO_LS host에는 확정
`ParagraphFloatPlacement`가 없어 full paint의 문단 기준 원점이 outer-top을 생략하는 것이다.
partial 경로도 확정 원점이 없으면 다른 프레임 술어로 재해석한다.
기존 `layout.rs`의 1×1 RowBreak 흐름 끝 보정은 표를 실제로 옮긴 뒤 top을 다시 더할 수 있다.

보정 범위는 단일 표만 가진 빈 NO_LS host의 비-TAC, Para/Top, TopAndBottom, offset 0
블록 표다. 예약된 before·본체·after로 하나의 확정 원점/점유 끝을 만들고 whole fit,
첫 fragment, paint 및 뒤 흐름이 소비하게 한다. 일반 텍스트·공백 host, TAC, Square,
Page/Paper 기준, 저장 LineSeg host는 이 경로로 승격하지 않는다.
선행 입력 그대로 수정 전후 최종 원점·흐름 끝·뒤 표 소유 및 33·34쪽 직접 출력을 확인한다.

### 76076 빈 host 표 원점 보정 — 3d9239eee

입력은 위 실제 원본과 기존 한컴 2024 PDF 그대로이며 수동 LineSeg나 좌표를 추가하지 않았다.
보정 코드 SHA는 `3d9239eeec32fc60ee188c3f3bc0d9ec5094ec3a`다.
`from_empty_reflow_host`가 기존 `host_spacing`의 before·측정 본체·after로 표 원점과
점유 끝을 만든다. text가 빈 sole-table host의 재조판, Para/Top, 자리차지, offset 0에 적용한다.
공백을 가진 host는 빈 host로 합치지 않고, 저장 앵커·TAC·어울림·절대 기준·명시 offset의
기존 계약도 이 분기에 포함하지 않는다.

| 실제 경로 | 확정 결과의 소비와 후속 분기 |
| --- | --- |
| whole fit | `block/entry.rs`는 formatted before/body/after의 placement를 fit 하단과 함께 기록하고 `typeset.rs::place_table_with_text`는 그 occupied_bottom을 전진시킨다. |
| 첫 RowBreak 조각 | `block/prepare.rs`는 빈 host의 원점을 텍스트 줄 앵커로 다시 환산하지 않고 첫 top을 중복 열지 않는다. 별도 host 글줄도 예약하지 않는다. |
| 이월·이어받기 | `continuation/fragment/budget.rs`는 같은 frame에서는 확정 top을 소비하고 새 frame에서는 현재 높이와 해당 조각 overhead로 옮긴다. 같은 첫 조각의 1×1 top을 다시 더하지 않는다. `fragment/emit.rs`가 실제 수용한 조각 높이로 점유 끝을 확정한다. |
| 전체·부분 paint | `layout_table`/`layout_partial_table`에 확정 원점을 전달한다. `layout.rs`의 이전 빈 1×1 RowBreak 끝점 공식 대신 확정 occupied_bottom을 소비하여 top을 재가산하지 않는다. 위 캡션은 확정 외곽 상자의 내부로 배치한다. |

다음 값은 96dpi 좌표다. PDF의 text origin과 괘선 path를 기준으로 사용했으며,
font bbox yMin을 글자 기준선으로 대신하지 않았다.

| 실제 출력 검사 | 수정 전 Native | 수정 후 Native | 독립 PDF |
| --- | ---: | ---: | ---: |
| 323 표 문자 baseline | 188.360 | 195.907 | 196.000 |
| 324 표 문자 baseline | 220.787 | 228.333 | 228.480 |
| 325 큰 표 시작 괘선 | 238.5 | 240.4 | 240.217 |

두 작은 표의 baseline은 PDF 대비 0.25px 이내 검사를 수정 전 FAIL / 수정 후 PASS했다.
실제 표 사이 진행량은 `(566 + 1300 + 566) / 75`px로 유지하며,
빈 325번 host의 별도 TextLine은 생성하지 않는다. 일반 앞 문단 원점은 바뀌지 않았다.
이는 `output/pr-review/pr7518-20261004/regulatory-spacing-diagnosis.json`의 실제 출력 진단이며
정식 회귀 테스트 통과나 전체 한컴 일치 증거로 승격하지 않는다.

immutable Native CLI `rhwp-regulatory-empty-host-probe2`의 SHA-256은
`8827ee3867e3b5c0cb7aaada30321ed5dce89e298f56139e9f28416c098c4220`다.
`regulatory-spacing-fixed-native/regulatory-spacing-fixed`에 33·34쪽의
compare/standalone overlay/review와 exact-source `run_manifest.json`을 새로 산출했다.
33쪽 review와 overlay를 직접 확인했다. Native p33 85.07562%, p34 84.06333%로
자동 gate는 아직 `re_review_required`다. 아래 긴 셀의 글꼴·가로폭·줄바꿈과 표 하단 차이는 남아 있다.
90% 예외·golden/래칫 갱신을 적용하지 않으며 새 정식 렌더링 회귀도 추가하지 않는다.

`nested-split-spacing-control-native/nested-split-fixed`의 1–4쪽 review를 모두 직접 확인했다.
2쪽 빈 셀 윤곽, 3쪽 그림 패딩·그림 뒤의 다음 행 소유, 4쪽 이어받기 위치가 유지되며
gate는 PASS다. Native 1–6쪽 render tree는 앞서 사용자 수용한 `3d2354584` 출력과 모두 동일하다.
현재 CLI를 소비하는 기존 focused 실제 출력 probe는 12 PASS / 4 FAIL로 이전과 동일하다.
이 실행은 library를 새로 링크한 정식 전체 회귀로 대신하지 않는다.

같은 source의 fresh WASM wrapper는 성공했고, 루트 pkg/Studio public/실제 CDP 응답 WASM의
SHA-256은 모두 `ad03963e38cd85939f14eaa9c80099f5060d8642a77c4359ee582487b0091f40`이다.
`regulatory-spacing-fixed-wasm/regulatory-spacing-fixed`의 33·34쪽 review와 33쪽 overlay를
직접 확인했다. Native와 같은 85.07562%/84.06333%이며 90% gate는 미충족으로 보존한다.
`nested-split-spacing-control-wasm/nested-split-fixed`의 1–4쪽 review와 3쪽 overlay도
직접 확인했다. 두 backend 모두 각 쪽 97.84546%, 99.72190%, 99.65660%, 97.68812%로 PASS다.
CDP는 캐시를 끈 새 탭에서 실제 새 WASM을 받은 해시와 33·34쪽 표 좌표, source 여백과
한 번의 흐름 전진, 빈 split-host 글줄 비생성을 확인해 8/8 PASS했다.
명령과 출력은 `logs/regulatory-spacing-fixed-{native,wasm}.log`,
`logs/nested-split-spacing-control-{native,wasm}.log`, `cdp-regulatory-spacing/result.json`에 연결한다.

원 기여의 `issue_7500_no_lineseg_trailing_whitespace_line`을 현재 source로 새로 링크해 4/4 PASS했다.
`logs/regulatory-spacing-7500-current.log`에 원 TAC 표의 쪽 소속·앞 공백 뒤 위치·다음 큰 표와의
앞뒤 관계 및 그림 세 장의 첫 쪽 소속을 보존한 결과가 있다.
기존 `issue_7518_reflow_row_physical_frame`도 현재 source로 새로 링크해 실행했다.
`logs/regulatory-spacing-7518-current.log`의 결과는 12 PASS / 4 FAIL이며,
자동 높이·단일 셀·terminal-follower의 앞선 실패 네 가지가 그대로 남아 있다.
이번 76076 원점 보정으로 이 대조군까지 해결했다고 보고하지 않는다. 전체 회귀는 아직 재실행하지 않았다.
fmt, Native/WASM/workspace-all-targets 세 Clippy, workspace build는 순차로 PASS다.
policy는 현재 통합 base `1d6bc70767fad365b07afe4ef57972d23b140f2b` 대비 PASS다.

검증 중 fetch에서 최신 devel이 `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`(#7568)로
전진한 것을 확인했다. 최신 base 대비 manifest check는 이 base가 옮긴 옛 integration source
네 파일을 새 루트 source로 판단해 FAIL했다. 파일을 삭제하거나 정책을 완화하지 않았다.
`git merge-tree --write-tree upstream/devel HEAD`의 read-only 시험은 height_measurer,
table_layout, table_partial, block/prepare, fragment/emit의 다섯 content conflict를 검출했다.
이 base는 아직 작업 branch에 통합하지 않았으며 현재 출력 증거는 기존 base 위의 위 SHA에 한정된다.
최신 base 통합·충돌 해결·필수 재검증을 PR 제출 준비 완료와 혼동하지 않는다.

### 사용자 시각 판정 — 76076 33·34쪽 수용

2026-10-04 사용자는 위 Native/fresh WASM PNG를 제시한 뒤 다음과 같이 판정했다.

> 가이드레일 기준 90%에 미치지 못하지만 85% 수준에서 픽셀 일치시키는 수준이면 조판 허용치로는 수용할 수 있습니다.
> 시각 판정 통과로 진행시킵니다.

검토 source는 `3d9239eeec32fc60ee188c3f3bc0d9ec5094ec3a`이며 판정 대상은 위 76076
33·34쪽의 표 원점 보정과 그 비교 출력이다. Native/fresh WASM 모두 p33 85.07562%,
p34 84.06333%의 원 점수를 유지한다. 이 지표는 엄격 픽셀 일치율이 아니라 2px 이웃 관용
내용 실루엣 일치율이며 사용자 직접 판독에 따른 이번 수용과 구분해 기록한다.
자동 `re_review_required`를 PASS로 편집하거나 global threshold·golden·래칫을 완화하지 않는다.
사용자 지시가 이 작업의 일반 90% 시각 게이트보다 우선하므로 같은 수용을 재확인하지 않고
최신 base 통합과 정식 회귀·lint·새 head 출력 검증을 계속한다.
다른 대조군의 누락·중복·불필요한 빈 쪽이나 기존 검사 실패까지 이 판정으로 통과 처리하지 않는다.

### 최신 devel 통합과 사용자 시각 판정 적용

최신 base `731de9e1b4bb946d76f35108ed7e186ebe4ebecb`(#7568)를 실제 작업 branch에
통합했다. 위 다섯 conflict는 merge commit `d76cd8075c5e98f10da1b48f3af72155d9f3d40d`에서
해결했다. contributor의 원 code·asset commit과 이전 메인터너 보정 history를 유지했다.
`checkpoint/pr7518-before-7568-20261004`에 통합 전 상태도 보존했다.

| 충돌 경로 | 최종 소비 계약 |
| --- | --- |
| `height_measurer.rs`, `table_layout.rs` | 재조판 nested host는 cut과 같은 cell-unit 원장을 측정한다. 최신 devel의 NO_LS TAC host 뒤 간격도 그 원장에 포함한다. 그 밖의 collapsed/all-NO_LS 경로는 최신 처리를 유지한다. |
| `table_partial.rs` | 최신 저장 block-reset opening과 재조판 complete-content/physical-tail의 정렬 소유를 각 실제 cut 경로에서 유지한다. |
| `block/prepare.rs` | 저장 body-filling frame의 새 outer-top은 유지하고 확정 빈 재조판 host의 top은 중복 열지 않는다. |
| `fragment/emit.rs` | 빈 그림 띠의 물리 높이, 재조판 행의 physical tail, 저장 frame의 남은 높이는 각각의 실제 호출 분기로 이월한다. |

같은 원본 HWP/PDF·글꼴 경로로 Native와 fresh WASM의 76076 33·34쪽을 다시 산출했다.
source SHA는 위 merge code이며 renderer diff는 없다. 이후 test-only commit
`f961b773935d823859efca682c6b2f1b926b6acb`는 renderer·WASM을 변경하지 않는다.
Native CLI SHA-256은 `200ada66a6f0e582d58d4f28fd41acb308bd94716db78c98eea195a63b38d33d`,
pkg/Studio public/캐시를 끈 CDP 실제 응답 WASM은 모두
`7353d24cde4f554b6bef0bb14e5053e5c9004db093058ac974290642f221b6f8`다.

| 새 직접 출력 | Native | fresh WASM | 판독 |
| --- | ---: | ---: | --- |
| 76076 p33 | 85.78689% | 85.78689% | 작은 두 표·다음 표의 보정 원점 유지, 사용자 시각 수용 범위 |
| 76076 p34 | 86.67962% | 86.67962% | 최신 outer-top 보존으로 기준 괘선에 가까워짐, 사용자 시각 수용 범위 |
| nested-split p1–p4 최저 | 97.68812% | 97.68812% | 2쪽 빈 셀 윤곽·3쪽 그림 안여백과 다음 행·4쪽 이어받기 유지 |

각 backend의 해당 review PNG를 모두 열어 직접 비교했고, 76076 p33과 nested-split p3의
standalone overlay도 직접 확인했다. 76076 자동 gate의 `re_review_required`는 그대로 보존한다.
일반 90% 조건을 코드로 낮추지 않고 이번 사용자 명시 수용을 해당 비교 범위에 적용한다.
증적 root는 `output/pr-review/pr7518-20261004/` 아래
`integration-7568-regulatory-{native,wasm}/regulatory-integration`과
`integration-7568-nested-{native,wasm}/nested-integration`다.
각 `run_manifest.json`에 source·입력·PDF hash와 실행 인수를 고정했다.
명령 원문과 점수는 `logs/integration-7568-{regulatory,nested}-{native,wasm}.log` 및
각 root의 `summary.json`에 있다.

CDP `cdp-regulatory-integration/result.json`은 새 WASM 제공 해시, source outer-top,
작은 표 사이 before/body/after 한 번 소비, 뒤 큰 표의 인접 여백, 빈 split-host 글줄
비생성, p33/p34 Native/WASM 표 좌표 일치, 브라우저 오류 부재를 확인하여 8/8 PASS다.
실행은 `VITE_URL=http://localhost:7718 CHROME_CDP=http://localhost:19222 node
output/pr-review/pr7518-20261004/cdp-regulatory-integration.mjs`이며 새 탭만 사용했다.

정식 source `tests/cases/issue_7518_reflow_row_physical_frame.rs`에
`empty_reflow_table_hosts_consume_the_source_margins_once`를 추가했다. 실제 source 속성으로
앞 subtitle의 줄 점유·첫 표 outer-top·인접 표의 bottom/top 여백 관계와 325 host 글줄
비생성을 검사하며 절대 픽셀 원점을 고정하지 않는다. 같은 compiled formal harness에서
수정 전 immutable CLI `rhwp-integration-3d2354584`는 첫 host 위 여백 누락으로 FAIL,
현재 `rhwp-integration-d76cd8075`는 PASS다. 이는 환경·빌드 실패가 아니다.
증거는 `logs/integration-7568-empty-host-{before,after}.log`다.

현재 source를 새로 링크한 focused 결과는 아래와 같다. 실행 명령은
`node scripts/run-rust-test.mjs --cargo-test <case> -- --target-dir
/home/edward/mygithub/rhwp/target/pr-review`이며 case별 로그를 보존했다.

| 실제 검사 | 결과 | 증적 |
| --- | --- | --- |
| `issue_7418_host_text_and_split_row_geometry` | 7/7 PASS: stored/synthesized host·TAC 간격·continuation outer-top 등 정상 대조 | `logs/integration-7568-focused-7418.log` |
| `issue_7422_recomposed_cell_frame_uses_paragraph_margins` | 1/1 PASS: 재조판 셀의 본래 문단 프레임 | `logs/integration-7568-focused-7422.log` |
| `issue_7500_no_lineseg_trailing_whitespace_line` | 4/4 PASS: 원 TAC 줄·그림 소속 | `logs/integration-7568-focused-7500.log` |
| `issue_7518_reflow_row_physical_frame` | 13 PASS / 기존 4 FAIL; 새 여백 검사는 PASS | `logs/integration-7568-focused-7518.log` |

기존 실패 중 auto/mixed의 마지막 outer-row fragment 개수와 follower의 nested-table
fragment 개수는 그 자체로 내용 중복을 입증하지 않는다. 실제 유닛 소유로 대조할 대상이다.
한편 auto 대조군은 현재 Native 7쪽 / 동일 입력 한컴 PDF 8쪽이며 4·5·7쪽 직접 비교에서
UNIT의 쪽 소속과 프레임 여백 차이가 확인된다. `integration-7568-auto-native/auto-integration`
및 `integration-7568-auto-tree`에 현재 출력과 원본을 보존했다. p5 88.51980%, p7 73.02188%를
76076 사용자 수용으로 통과 처리하지 않는다. mixed의 마지막 빈 쪽도 별도 원인 대상으로 남긴다.
작은 영향 경계가 아직 해결되지 않아 비용이 큰 전체 회귀를 먼저 반복하지 않았다.

최신 base 대비 manifest check는 통합 전의 옛 네 source 이동 문제를 해소하여 PASS했다.
test-only commit의 필수 fmt·Native/WASM/workspace-all-targets 세 Clippy·workspace build·manifest
순차 검증은 모두 PASS했다. `integration-7568-lint.sh` 및 `logs/integration-7568-lint.log`에
명령과 완료 결과가 있다. 같은 base의 source-unit tier check도 PASS다.
이 결과를 전체 회귀·최종 PR 제출·remote 통합의 완료로 대신하지 않는다.
