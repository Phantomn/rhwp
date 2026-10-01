# Task #7353 — W3 최종 통합 검증

- 승인: 2026-10-01, 다음 진행 승인에 따른 로컬 통합 검증.
- 구현 head: `9e059480c` (WASM/Studio 기본 V2). W3는 stage22에 기록했다.
- 정책 base: fetch로 확인한 `02530b9ed567a44663edb26c65fb565c4a79f00d`.
- 원격 push·PR·devel 병합 및 새 미지원 기능 구현은 범위 밖이다.
- 기존 사용자 skill/config 미추적 파일은 보존한다.

## 실행 계획과 증적

`output/7353/wasm-product/final/run-gates.sh`에서 각 명령을 순차 실행하고,
첫 실패에서 후속 게이트를 중단한다. target은 기존 공유
`/home/edward/mygithub/rhwp/target/pr-review`, Cargo jobs=2를 사용한다.
명령별 로그와 종료 코드는 같은 디렉터리에 기록한다.

전체 순서: suite 준비 → release build/lib → nextest 전체 → Native Skia 3종 →
fmt/diff → Native/WASM/workspace lint와 build → 정책 base 비교 → doc test →
Studio TypeScript/test → 표준 Docker WASM build.

## 실행 결과

| 게이트 | 결과 | 증적 |
| --- | --- | --- |
| suite 준비 | 1,471 sources / 48 integration targets 준비 완료; 파생 파일은 커밋하지 않음 | `prepare.log` |
| release build | PASS, 23분 5초 | `release-build.log` |
| release lib | 4,055 PASS / 13 ignore / 0 FAIL (rhwp 3,873 + workspace lib 182) | `release-lib.log` |
| 전체 nextest | 10,878 PASS / 50 skip / 0 FAIL, 실행 600.367초 | `nextest.log` |
| Native Skia lib | 4,112 PASS / 13 ignore / 0 FAIL | `skia-lib.log` |
| Skia placeholder / 직접 PDF | 각각 2 / 4 PASS; suite의 나머지는 선택 필터로 제외 | `skia-placeholder.log`, `skia-pdf.log` |
| fmt / diff / Native·WASM lib Clippy / workspace build | PASS | 각 명령 로그 |
| workspace all-target Clippy | 최초 FAIL: 테스트의 `clippy::replace_box` 1건 | `clippy-workspace.log` |
| lint 정정 후 제품 계약 | 15 PASS / 0 FAIL | `product-contract-recheck.log` |
| 최종 3종 Clippy·workspace build·fmt | PASS | `*-final.log` |
| base 고정 manifest / unit tier 정책 | PASS | `manifest-final.log`, `unit-policy.log` |
| doc test | 8 PASS / 3 ignore / 0 FAIL | `doctest.log` |
| Studio TypeScript / unit tests | PASS; 1,767 PASS / 2 skip / 0 FAIL | `studio-ts.log`, `studio-tests.log` |
| 표준 Docker WASM | 네트워크 우회 재시도 PASS, wasm-opt 포함 7분 47초 | `wasm.log`, `wasm-network-retry.log` |
| 최종 Native/fresh WASM 제품 출력 | 본문 2쪽 + 분할 셀 3쪽 tree/layer/SVG 일치, 실제 Canvas·편집 API PASS | `native-*.log`, `wasm-*.log`, `body/`, `table/` |
| 일반 Studio CDP | 키 입력·undo/redo·선택·저장/재열기·새 문서·명시적 Legacy PASS | `studio-cdp.log`, `studio/result.json` |
| E2E manifest | 139/139 PASS | `e2e-manifest.log` |

최초 all-target lint 실패에서 runner는 후속 게이트를 중단했다. 정정은
`tests/cases/issue_7353_product_v2.rs`의 SectionDef 복사에서 새 Box를 할당하지 않고
기존 Box의 내부 값에 대입하도록 한 줄 변경한 것이다. 제품 코드·입력값·assertion은 불변이다.
전체 회귀/Skia 실행은 `9e059480c`의 결과이며, 이후 테스트 코드 한 줄 정정에는
해당 제품 계약 전수와 lint를 재실행한다. 전체 회귀를 재실행했다고 보고하지 않는다.

테스트 한 줄 정정은 파일 크기 기반 파생 suite 배정도 바꾼다. 최초 manifest 검사는
26개 harness drift로 실패해 후속 실행을 중단했다. 전후 suite 소스 1,451개가 동일하며
중복·누락이 없음을 `harness-drift-evidence.log`로 확인하고 `--prepare`를 다시 수행했다.
예외 target 20개 및 테스트 원본은 유지한다. 재생성 후 최종 lint·base 고정 정책은 PASS다.
파생 파일은 stage/commit하지 않는다.

Docker 최초 실패는 `all predefined address pools have been fully subnetted`로,
빌드 전에 Compose 기본 네트워크 생성이 거부된 환경 문제다. 기존 `rhwp_default`는
연결 컨테이너 0개임을 확인했고, output 아래 Compose override로 external 재사용한다.
표준 wasm 서비스·이미지·캐시·locked wrapper는 그대로 사용한다. 산출물 소유권만 실제
host UID/GID 1002로 맞추며 `.env.docker`와 전역 Docker 네트워크는 변경/삭제하지 않는다.

환경: Rust 1.93.1, nextest 0.9.137. 저장소 권장 nextest 0.9.140보다 낮다는 경고와
미사용 `ci-duration-observation.junit.report-skipped` 키 경고가 있다. 기본 프로파일의
실제 테스트 전수 실행은 성공했으며, 원격 CI 동등 환경이라고 주장하지 않는다.

기존 W3 대표 CDP·fresh WASM 시각 검증과 전체 통합 통과를 구분한다.
실패를 baseline 완화 또는 ignore로 숨기지 않는다.

## 최종 패키지와 시각 증적

- 제품 source는 `9e059480cdeca5711370ab628afa2fc18d3e8e53` 그대로다. 이후 변경은
  테스트 Box 대입과 E2E 출력 경로 환경변수 `RHWP_V2_E2E_OUT`뿐이다. 기존 W3 증적을
  덮어쓰지 않도록 이번 증적을 `final/`에 분리했다.
- 최적화 WASM SHA-256:
  `a3e9f36487204f893cf4b454b9cb48b621109cb7e876f9222e75bc5c51f2322c`.
  Studio `http://localhost:7700`이 제공하는 WASM 응답의 해시도 동일하다
  (`studio-served-wasm.json`). `pkg/`는 host edward 소유로 유지된다.
- `verify-final.sh`가 Native 기준을 현재 테스트에서 다시 출력한 뒤 새 WASM의
  tree/layer/SVG와 비교하고 실제 Canvas를 캡처했다. `sweep.py`는 정상 한컴 저장본 및
  대응 PDF로 각 backend의 compare·standalone overlay·review를 새로 생성했다.
  입력은 `tests/fixtures/issue7353_body_frame_review/portrait-saved.hwp` 및
  `tests/fixtures/issue7353_host_owner_review/split-saved.hwp`, 기준은 각 폴더의
  `portrait-2020.pdf`, `split-2020.pdf`다.
- 직접 판독: 본문 빈 줄과 ALPHA–CHARLIE / DELTA–AFTER 순서, 분할 표 2쪽 ROW01–19와
  3쪽 ROW20–25 / AFTER, 외곽·이어받기 위치, Studio EDIT 입력·선택·3쪽 표시를 확인했다.
  글꼴 형태와 테두리 색 차이는 남는다. 이를 한컴 피델리티 완전 일치로 보고하지 않는다.
- 실제 Studio CDP는 Windows Chrome 153.0.8010.48, 제품 API Canvas 캡처는 Linux Chrome
  152.0.7977.54다. beforeunload는 보존한 테스트 문서의 의도된 전환에서만 처리했고,
  사용자 탭·변경사항 보호는 유지했다. uncaught browser error는 0이다.

대표 자료:

- [본문 1쪽 review](../../output/7353/wasm-product/final/body/visual/canvas/review/review_001.png)
- [표 2쪽 compare](../../output/7353/wasm-product/final/table/visual/canvas/compare/compare_002.png)
- [표 2쪽 standalone overlay](../../output/7353/wasm-product/final/table/visual/canvas/overlay/overlay_002.png)
- [표 2쪽 review](../../output/7353/wasm-product/final/table/visual/canvas/review/review_002.png)
- [표 3쪽 review](../../output/7353/wasm-product/final/table/visual/native/review/review_003.png)
- [Studio 셀 편집 화면](../../output/7353/wasm-product/final/studio/cell-edited.png)

코멘트: 표 2쪽 Canvas의 내용 픽셀 중심 자동 일치율 보조값 = 약 14.26%.
높을수록 좋음: 기준 PDF와 rhwp PNG가 더 비슷함
낮을수록 나쁨/검토 필요: 잉크 위치나 형태 차이가 큼
단, 사람 판정 정확도가 아니라 내용 픽셀 중심 자동 일치율 보조값입니다

## 종료 경계

WASM/일반 Studio 기본 V2 전환의 승인된 로컬 통합 검증을 마쳤다. 전체 회귀의 제품 실패는
없었고, 테스트 lint 1건은 정정 후 영향 계약·3종 lint를 재검증했다. 산출물 drift와 Docker
네트워크 환경 문제도 해소했다. 새로운 조판 규칙·baseline/ignore 변경은 없다.

이 결과는 전체 문서 수용, 복잡한 중첩 객체 편집, IME 전수, 암호 입력 UI 또는 OS 저장창
자동화 완료를 뜻하지 않는다. 암호는 제품 API, 저장은 bridge export와 file-input 재열기로
검증했다. 최종 메인테이너 시각 승인·원격 게시/CI·devel 병합·0.9.0 릴리즈는 별도 경계이며,
이번 실행에서는 원격 push/PR/merge를 하지 않았다.

## 메인테이너 승인 및 게시

2026-10-01 메인테이너가 W3 시각 확인 완료와 커밋·push를 승인했다. W3 시각 승인 대기를
종료한다. 게시 대상은 `upstream/refactor/0.9.0`이며 PR 생성·devel 병합·릴리즈는 포함하지 않는다.
게시 직전 fetch 결과 정책 base `02530b9ed567a44663edb26c65fb565c4a79f00d`와 원격 기준점
`3b2390949e69cff4f6c35ffd7353a8ecf02de2a7`은 불변이다. 검증 head `ce1753ce2` 이후 변경은
이 승인 기록뿐이므로 위 검증을 재사용한다. 미추적 사용자 skill/config 파일은 커밋하지 않는다.
후속 V2 미지원 구현은 현재 코드의 실제 거부 경계를 조사해 유한한 기능 묶음으로 계획하며,
W3에서 단순히 미검증인 UI 경로와 구별한다.
