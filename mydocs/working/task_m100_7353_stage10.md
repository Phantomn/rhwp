# Task #7353 — V2 가로 병합 셀의 공통 grid

- Issue: #7353. 선행 절편: `5c975d0f9` — [stage9](task_m100_7353_stage9.md).
- 범위: fresh IR의 colspan을 중첩 분할·반복 제목·실제 출력에 연결한다. rowspan은 미지원 유지.
- 상태: 구현·선택 회귀·범위별 lint 완료. Legacy/기본 엔진·baseline은 변경하지 않는다.

## 규칙과 입력 근거

[HWP 5.0 표 80](../tech/한글문서파일형식_5.0_revision1.3.md)은 셀의 열 주소·열 병합 수와
셀 폭을 별도로 정의한다. 셀 폭은 병합된 셀 전체의 폭이며 덮인 각 열의 폭이 아니다.
병합 셀의 문단·컨트롤은 하나의 소유 범위로 구성하고 가용 폭은 전체 셀 폭에서 유효 안 여백을 뺀다.

입력은 합성 fresh IR다. 제목200px, 본문100+100px, 마지막40+60+100px로 선언하여
공유 열 경계는0/40/100/200px라는 독립 기하 불변식으로 검사한다. 일부 행에서만 드러나는
열 경계를 다른 행의 병합 셀에 억지로 균등 배분하지 않는다. 모든 행에서 숨겨진 경계는 미정으로
남겨도 실제 셀의 배치에 필요하지 않으므로 지원한다.

셀 주소 중복·중첩·구멍,0 span/폭, 범위 밖 주소, 행간 동일 열 경계의 불일치, 서로 다른 행에서
관측한 경계의 역전/겹침은 준비 단계에서 거부한다. 폭을 평균내거나 출력 clamp로 숨기지 않는다.
세로 병합, 저장 LineSeg, TAC/어울림·테두리 등 기존 미지원 경계는 유지한다.

## 공통 결과와 소비 위치

1. `grid.rs::resolve`: 원래 정수 HWP 단위에서 행의 연속 덮음과 공유 경계 정합을 확인한다.
   물리 셀별 `CellTrack {column, span, left, width}`와 grid 순서의 IR 참조를 만든다.
2. `ir.rs::bind_table`: 같은 track 폭에서 안 여백을 빼 문단을 구성한다. colspan으로 셀이나
   문단을 복제하지 않는다. 제목 반복은 해당 행 전체의 실제 셀이 제목인 경우를 지원한다.
3. `content.rs::from_grid_rows`: 동일 track 폭과 구성 폭을 검사하여 불변 계획을 보관한다.
   기존 명시적 flow API는 span1 track을 만들어 같은 소비 경로로 진입한다.
4. `fragment.rs::fit_rows`: cursor는 물리 셀별로 진행하고, 실제 배치에는 track의 논리 열 주소·
   병합 수·x·폭을 사용한다. 내용 컷·패딩·남은 최소 높이와 요구/예약 높이 계산은 기존 공통 경로다.
5. `text.rs::build_node`: 논리 열 주소로 원본 payload를 조회하고 최종 `TableCellNode.col_span`을
   보존한다. paint에서 열 폭을 다시 나누거나 셀 개수에 맞춰 주소를 바꾸지 않는다.

일반/반복 제목/중첩 continuation이 모두 이 track을 소비한다. `Never`, `BetweenRows`,
`WithinCells`의 기존 높이·이월 규칙을 유지한다. rowspan 소유 유닛·세로 병합 분할은 비해당이며
이 절편으로 구현됐다고 보고하지 않는다. 공개 실험용 `CellPlacement`에 `column_span`을 추가했다.

## 독립 기대값과 검사

정식 source: `tests/cases/issue_7353_table_v2_colspan.rs`.

- 제목 colspan3: x20/w200, 본문 colspan2: x20/w100, 다음 논리 col2: x120/w100.
  다음 페이지 제목을 반복하고 마지막 행은 x20/60/120, 폭40/60/100으로 출력한다.
- 부모 colspan2 폭212/여백5·7 → 안쪽200에서 같은 자식 표를 재귀 분할한다.
  뒤 host/after와 내용 종료를 검사한다.
- 직접 구성기에 전달되는 병합 폭100 - 여백3·7 =90, 실제 줄 x23/w90을 검사한다.
- 72/96/144 DPI에서 같은 전체 폭의 미병합 셀 대조군과 실제 텍스트 줄 구성·높이가 동일하다.
- 병합 셀 내부3줄과 짧은 형제 셀: 제목은 반복하되 형제 본문은 재출력하지 않고 셀 영역은 유지한다.
- 입력 배열 역순, 전체 병합으로 숨은 경계, 예산 sweep,3가지 split policy, 합성 HWPX 재파싱,
  모순 경계/구멍/중첩/rowspan 거부를 검사한다.

## 검증 기록

기준 소스는 `5c975d0f9`, 고정 policy base는
`7a95e46e025470a4d7a7b59ad68ec02958bda738`이다. review worktree와 Cargo target은 앞 절편과 같다.
수정 전6건은 빌드·실행되었으며 정상 colspan5건이 명시적 미지원으로 실패하고 거부 검사1건은 통과했다.
이는 기존 Legacy 조판 결함5건이라는 뜻이 아니다.

증적 디렉터리: `output/7353/r10/`.

| 검사 | 결과 / 증적 |
| --- | --- |
| 수정 전 최초6건 | 1 PASS / 5 FAIL, `before.log` — 지원하지 않던 colspan 입력 거부 |
| 최종 신규 계약 | **10 PASS**, `after-final-capture.log` |
| 기존 V2 geometry/nested/text/nested_text/session/ir_text/headers | **68 PASS**, `selected-tests.log` |
| Legacy #5301 안 여백 대조군 | **3 PASS**, 같은 로그 |
| 합계 | **81 PASS**. 전체 저장소 회귀/CI 결과가 아님 |
| Native / WASM library Clippy | **PASS**, `clippy-native.log`, `clippy-wasm.log` |
| 신규 case를 포함하는 integration suite Clippy | **PASS**, `clippy-tests.log` |
| fmt / 고정 base manifest 정책 | **PASS**, `fmt.log`, `policy.log` |
| 변경 문서2개 상대 링크 / diff whitespace | **PASS** |
| source 동일성 | `source.sha256` 20개 파일: product/review 동일 |
| 산출물 | `preview.sha256`: SVG/PNG 5쌍과 합성 HWPX |

명령은 review worktree에서 `node scripts/rust-test-suite-manifest.mjs --prepare` 후
각 case에 `node scripts/run-rust-test.mjs <case> -- --cargo-profile release-test
--target-dir /home/edward/mygithub/rhwp/target/pr-review --no-fail-fast`를 적용했다.
새 계약의 캡처 실행에는 `ISSUE7353_PREVIEW_DIR`로 위 증적의 `preview` 경로를 지정했다.
새10건 중 추가4건은 수정 전 실행하지 않았으므로 전후 결함 검출 증거로 주장하지 않는다.

lint는 review worktree에서 다음을 순차 실행했다. 이는 개발 절편의 범위별 검사이며
push/PR 직전 필수인 workspace build·all-targets Clippy 전체 묶음의 대체가 아니다.

```sh
cargo clippy --locked -p rhwp --lib --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo clippy --locked -p rhwp --lib --target wasm32-unknown-unknown --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo clippy --locked --test regression_suite_013 --target-dir /home/edward/mygithub/rhwp/target/pr-review -- -D warnings
cargo fmt --all -- --check
node scripts/rust-test-suite-manifest.mjs --check --base-ref 7a95e46e025470a4d7a7b59ad68ec02958bda738
```

WASM library Clippy는 WASM 실행·시각 검증 결과가 아니다. source-side `#[cfg(test)]`는 변경하지 않았다.

최초 수정 후 실행에서 좌표60px와 부동소수 계산값60.00000000000001의 exact 비교1건이
실패했다(`after.log`). 좌표 assertion만 기계 정밀도(`8 * f64::EPSILON * max(1, |expected|)`)
범위 비교로 변경했다. 제품 좌표·기준값·baseline·실패 허용 목록은 변경하지 않았다.
로컬 nextest0.9.137에서 권장0.9.140 및 JUnit 설정 키 경고가 있으나 위 검사는 모두 실행됐다.

### 직접 출력 확인

`rsvg-convert -b white`로 SVG를 PNG로 변환하여 5장을 직접 열었다.
기본2쪽은 title/left/right → 반복 title/A/B/C, 중첩3쪽은 같은2쪽 흐름 뒤 host/after가 이어진다.
중첩 표의5px 왼쪽 안 여백 이동과 내용 누락·중복이 없는 것을 확인했다. 정확한 셀 주소·폭·
점유 높이는 같은 실행의 최종 RenderTree assertion으로 확인한다. 현재 테두리 없는 실험용
출력이므로 셀 외곽선 정확성이나 실제 한컴 조판 일치를 입증하지 않는다.

대표 산출물(저장소 루트 기준):

- `output/7353/r10/preview/colspan-0.png`, `colspan-1.png`
- `output/7353/r10/preview/nested-colspan-0.png`부터 `nested-colspan-2.png`
- `output/7353/r10/preview/fresh-colspan.hwpx` — 테스트가 생성한 합성 입력이며 한컴 생성본이 아님

## 남은 범위

실물 한컴 PDF와 Native/fresh WASM Visual Sweep, 문서 전체 선택·Studio 연결은 미검증이다.
합성 기하·출력 검사의 통과를 한컴 시각 일치로 승격하지 않는다. 전체 CI/workspace 검증과
원격 push/PR/이슈 종료는 이번 절편에 포함하지 않는다.
