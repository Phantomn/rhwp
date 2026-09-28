---
kind: task
status: active
issue: 7473
---

# #7473 PR Render Diff의 배포 WASM 검증과 비용 측정

Issue: #7473. 기준 소스: `d6cf1605327ced1c276f17a529192a743a766209`.

## 분석과 구현 범위

PR Render Diff만 `wasm-pack build --target web --dev`에서 `--release`로 전환한다.
Full Renderer Sweep·npm·Pages는 이미 release이며, frontend package gate와 로컬 wrapper는
기존 빠른 경로를 유지한다. Cargo 프로필·캐시 정책·렌더링 구현·시각 기준값은 변경하지 않는다.
운영 등급은 O3(job command)다. 기존 job 이름·required check·fork 권한을 유지한다.

검증 입력은 Vite의 `@wasm/rhwp.js` alias → 루트 `pkg/rhwp.js`의 기본 init →
`pkg/rhwp_bg.wasm`이다. `public/` 사본이나 이전 pkg의 통과를 배제하기 위해 빌드 manifest의
SHA-256과 각 Canvas/PDF 검사 페이지가 실제 수신한 WASM 응답 bytes를 비교한다.

측정 script는 기존 `wasm-pack-locked.sh`를 사용해 metadata/build의 lockfile 고정과
Studio public 사본 동기화를 유지한다. 동일 release 프로필/wasm-opt를 사용하며 로그 수신 시각으로 Cargo·bindgen 준비/실행·
wasm-opt/마무리를 구분한다. 이 값은 프로세스 독점 CPU 시간이 아닌 로그 경계의 wall 근사다.
실제 child 명령 로그에서 도구 경로·옵션을 얻고 버전과 executable hash를 보존한다.
release에서 wasm-opt 실행 명령이 관측되지 않으면 검증을 실패시킨다.

## 검증 계획

- workflow actionlint·기존 계약 검사, 측정 실패/최적화 누락 검사, 해시 일치/불일치/응답 누락 검사.
- 동일 소스·Chrome·fixture에서 fresh dev/release WASM의 Canvas/PDF 결과 대조.
- Studio package 검증(TypeScript·unit·production build) 및 실제 브라우저 소비 증거.
- 기존 Actions 5건은 모두 cache miss이며 서로 다른 소스이므로 운영 참고치로만 사용한다.
  후보 PR의 동일 조건 Actions 실행 전에는 CI 증가분과 이슈 완료를 확정하지 않는다.

## 결과

로컬 구현과 기본 시각 비교를 수행했다. 전체 완료 판정은 보류한다.
원격 push·PR 생성·새 Actions 실행은 아직 수행하지 않았다.

### 실행 조건과 결과

WASM/관측 helper의 검증 commit은 `f818d92985a6668b53f9d1b104af6a4fc362d982`이며
Rust·Studio 제품 소스·Cargo lock은 base와 동일하다. 그 뒤 변경은 수동 CI 비교 입력·검사 배선·문서다.
manifest의 `source_dirty=true`에는 wrapper가 동기화한 tracked `public/rhwp.js`가 포함된다.
이 생성 사본은 source 변경으로 제출하지 않는다. 두 빌드 모두 실행한 wrapper로 생성했고
`pkg/`와 public WASM hash 일치를 확인했다.

macOS / Rust 1.93.1 / wasm-pack 0.15.0 / wasm-bindgen 0.2.127 /
wasm-opt 117 `-O` / Chrome 153.0.8010.53 / jobs=6 / `target/pr-review`.
Docker CLI는 있으나 daemon에 연결할 수 없어 native host에서 진단했다.
GitHub의 Ubuntu·고정 Chromium 조건과 동일한 검증으로 취급하지 않는다.

| 검사 | 결과 | 명령·증거 |
| --- | --- | --- |
| Workflow | 183 통과, actionlint 통과 | `python3 -m unittest discover -s scripts/tests -p 'test_*workflow.py'`; `actionlint .github/workflows/render-diff.yml` |
| 경로 분류 | 44 통과 | `node --test scripts/tests/ci-impact-classifier.test.cjs` |
| Studio | TypeScript·production build 통과, 1,808 tests 통과 / 기존 skip 2 | `npx --prefix rhwp-studio tsc --project rhwp-studio/tsconfig.json --noEmit`; `npm --prefix rhwp-studio test`; `npm --prefix rhwp-studio run build` |
| 기존 job 계약 | renderer-contract·font-coverage·native-diff self-test·E2E manifest 통과 | Render Diff의 기존 script syntax 및 계약 명령 실행 |
| Native CLI | dev native-skia 빌드 통과 | `CARGO_TARGET_DIR=target/pr-review CARGO_BUILD_JOBS=6 cargo build --locked --features native-skia --bin rhwp` |
| Canvas | dev/release 각 3/3 통과 | `npm --prefix rhwp-studio run e2e:render-diff:ci`, 기본 3 fixture 각 p1 |
| Direct PDF | dev/release 각 3/3 통과 | 같은 runner, `RHWP_RENDER_DIFF_DIRECT_PDF_GATE=1`, 허용비 0.02 유지 |
| PDF 참고 비교 | 양쪽 4 warning / 0 error, 동일한 내용 | 원래 report-only 결과를 보존. 통과로 바꾸지 않음 |
| 프로필 간 출력 | 저장 PNG 27/27 byte 동일; Canvas 결과 JSON·PDF summary 동일 | `output/issue-7473/visual-comparison.json` |
| provenance 음성 대조 | 실제 release 응답에 dev manifest를 주면 exit 1 | `output/issue-7473/stale-negative/`: expected dev hash와 실제 release hash가 다름을 기록 |

대표 PNG의 같은 페이지를 직접 확인했다: KTX p1의 지도·우측 시간/요금표,
사업계획서 p1의 제목·구분선·회사명, tac-case p1의 글줄 사이 표와 뒤 텍스트,
kps-ai p1의 제목·연도·로고가 dev/release 사이에서 같은 위치와 형태다.
이 비교는 프로필 간 동등성 검증이며 한컴 출력과의 일치를 주장하지 않는다.

원시 결과는 `output/issue-7473/{dev,release}-visual/artifacts/` 및
`{dev,release}-final/{build.json,build.log,consumption/}`에 있다.
처음 기본 inspector buffer로 큰 dev WASM을 읽을 때 본문이 evict된 실패도 보존했다.
수정 후 독립 CDP session에 산출물 크기를 반영한 buffer를 설정하여 두 프로필에서 실제 응답을 읽었다.

| 산출물 | dev | release |
| --- | --- | --- |
| WASM bytes | 41,287,538 | 11,358,612 |
| WASM SHA-256 | `9721e0174b75777e854e0bcbb9cabfa3c9a318a8379d3175f1b7a3facdcf81cf` | `787f8b03fea6d3ee38f08a9285f978faa16109db0e5f9d1d250f874bbe2d16f7` |

로컬 release 1회 빌드 wall 338.147초 중 로그 경계로 Cargo 308.346초,
bindgen 준비/실행 0.985초, wasm-opt/마무리 27.881초를 관측했다.
이는 새 release target의 빌드이며 dev의 warm 빌드와 비용 비율을 비교하지 않는다.
호스트 부하도 통제하지 않았다. 시각 runner 1회 wall은 dev 60.329초 / release 69.616초지만
반복이 없으므로 성능 개선·회귀의 근거로 쓰지 않는다. CI 증가분은 아직 미측정이다.

### readiness 실패와 dev 대조

`python3 scripts/renderer_baseline.py --profiles screen --browser-mode headless --readiness-only`
실행은 release/dev 모두 8개 중 7개 통과, `table-border-style` 하나 `visualParityFailed`로 exit 1이다.
동일 macOS Chrome의 CanvasKit software fallback 경로에서 diff ratio가 양쪽 모두 0.006725이며,
readiness PNG 16/16이 byte 동일하다. 실패 페이지도 두 프로필에서 직접 비교했다.
Canvas2D/CanvasKit 사이의 글자 굵기·세로 글줄 모양 차이는 두 프로필에 공통으로 보인다.
따라서 관측한 실패를 release 전환 회귀로 분류하지 않지만, renderer 간 차이 자체의 근본 원인은
이번 CI 작업에서 해결하거나 한컴 기준으로 판정하지 않았다. 기준값도 완화하지 않았다.

dev 대조에서 `--readiness-only --filter` 조합이 지원되지 않아 처음 명령은 실행 전 실패했으며,
이후 filter 없이 동일 8개 집합 전체를 실행했다. 이 재실행 결과만 비교했다.
증거: `output/issue-7473/{readiness,dev-readiness}/baseline-report.md`,
`readiness-comparison.json`. 후보 Ubuntu CI의 readiness 및 비용은 아직 미검증이다.

### 남은 검증과 후속 순서

1. 로컬 dev/release 공통 readiness 실패를 명시하고 Ubuntu 후보 CI 결과로 적용 범위를 확인한다.
2. 승인 후 branch push·draft PR로 최신 head의 required checks를 실행한다.
3. 동일 ref SHA의 수동 `wasm-profile=dev` / `release`를 각각 3회 실행한다.
   runner 이미지·도구·fixture·폰트·캐시 조건이 맞는 실행끼리만 비교한다.
   Rust/bindgen/opt는 build.json의 근사 구간, 시각/전체 job은 attempt별 API에서 구분한다.
4. 실행 결과와 비용이 확인될 때 이슈 완료 여부를 결정한다. 반복 opt 비용이 크다면
   캐시 무효화·결과 동등성 비용을 함께 따져 별도 이슈/PR로 제안한다.

렌더링 source·baseline·cache 구현 변경은 없으므로 Rust 전체 lint/integration 및
새 조판 Visual Sweep은 이 변경의 제품 수정 검증에 비해당이다. 실제 CI visual/readiness job은
이번 변경 범위에 해당하므로 실패·미실행을 면제하거나 PASS로 바꾸지 않는다.

## 기존 GitHub Actions 운영 기준선

2026-09-28 조회. 모두 PR 이벤트, attempt 1, ubuntu-latest, 성공한 Canvas visual diff job이다.
캐시 복원 로그는 모두 miss였다. 서로 다른 소스·runner 인스턴스이므로 release 전환의 대응군은 아니다.
스킵된 실행은 제외했다. 단계 시간과 job 시간은 attempt별 jobs API의 시작/종료 시각 차이다.

| 실행 | source SHA | WASM dev | Native CLI | Canvas/PDF | 전체 job |
| --- | --- | ---: | ---: | ---: | ---: |
| [run 36387655547](https://github.com/edwardkim/rhwp/actions/runs/36387655547/job/108816405031) | `f1f67db89937` | 116s | 120s | 55s | 465s |
| [run 36388039922](https://github.com/edwardkim/rhwp/actions/runs/36388039922/job/108817607554) | `8474698c1d89` | 121s | 124s | 57s | 475s |
| [run 36390631636](https://github.com/edwardkim/rhwp/actions/runs/36390631636/job/108825557393) | `fe6e8744d54f` | 118s | 125s | 57s | 486s |
| [run 36391429790](https://github.com/edwardkim/rhwp/actions/runs/36391429790/job/108827984240) | `20b388d9c14f` | 81s | 84s | 38s | 356s |
| [run 36398122815](https://github.com/edwardkim/rhwp/actions/runs/36398122815/job/108849365163) | `492aef1eefd0` | 121s | 125s | 53s | 495s |

기본 fixture/페이지/Direct PDF 조건이며 WASM dev 81–121초, 시각 검사 38–57초, 전체 356–495초다.
로그·원본 API 응답은 로컬 `output/issue-7473/ci-baseline/`에 보존했다.
