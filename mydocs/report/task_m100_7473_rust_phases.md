# 실제 Rust 수정 후 WASM release 본체 빌드 분석

Issue: #7473. PR #7474는 Draft로 유지했다. 이번 브랜치는 실험과 증거이며 production 적용 PR이 아니다.

## 결론

**현행 WASM의 실제 최적화 조건을 유지한 후보는 본체 반복 빌드를 33.2–35.9초(14.25–14.42%) 줄였다.** 동일 수정 소스로 두 독립 runner에서 순서를 반대로 비교했다. 아래 release 후처리·브라우저 검증 범위와 함께 해석한다.
실제 외부 링커는 약 0.127초이며, 비용은 주로 LLVM 코드 생성/최적화와 Rust 라이브러리 메타데이터 생성에 있었다.

- 현재 release 프로필을 문자 그대로 둔 `cdylib` 단독 빌드는 fat LTO를 새로 켜서 **8.94–11.83% 느려졌다**. 속도 개선안으로 채택하지 않는다.
- 후속 후보는 WASM에 필요한 `cdylib`만 만들되, 현행 dual WASM의 **실효 cross-crate LTO 없음**을 명령 범위에서 고정했다.
- opt-level=3, codegen-units=1, default features, wasm32 target, incremental=0, 최종 wasm-opt 117 `-O`를 유지했다.
- `lto=false` override를 사용하므로 **Cargo.toml의 release 설정 문구까지 동일한 안은 아니다**. Native release 설정이나 Rust 소비자에게 필요한 rlib를 전역 변경하는 안도 아니다.

## 실제 수정과 비교 조건

`src/lib.rs`의 `version()` 반환값에 `-phase-probe-1`을 추가한 같은 수정 소스를 비교했다.
기준 커밋 `0e8fd49fb868da0d47ac1294dcbbda81f0211233`, 계측 실행 SHA `99473fbc7d80f237e2ec3e9f952752431232bb9d`.
후보의 실제 명령은 다음과 같다. 최종 wasm-bindgen/wasm-opt는 이 명령 뒤에 별도로 실행했다.

```sh
cargo rustc --locked -p rhwp --lib --release \
  --target wasm32-unknown-unknown --target-dir target/pr-review \
  --crate-type cdylib --config profile.release.lto=false
```

원본/변경 소스 SHA와 교체 표현식, 실제 rustc 인자와 raw WASM 해시는 각 실행의 JSON에 보존했다.
최종 Node 실행에서도 이 변경 반환값을 직접 확인한다. 파일 시각만 바꾼 이전 캐시 진단과 구분한다.

두 독립 Ubuntu 24.04 runner / Rust 1.93.1을 사용했다. 둘 다 AMD EPYC 7763 / 4 vCPU, 이미지 20260920.314.1이며 boot ID가 다르다. 한쪽은 ABBA, 다른 쪽은 BAAB 순서다.
첫 seed는 의존성 준비이며 비교 표에서 제외했다. 앞 2회는 root rustc 단계 계측, 뒤 2회는 진단 옵션 없는 대조다.
Cargo의 compiler-artifact로 본체 외 재컴파일 여부를 확인했다.

## 본체 비용 분해

보정된 계측 빌드의 벽시계 시간(초)이다. 행은 겹치지 않게 집계했다.

| 구간 | 현행 rlib+cdylib | 후보 cdylib / 실효 LTO 고정 |
|---|---:|---:|
| 프런트엔드·메타데이터·기타 | 53.036–65.594 | 27.152–30.092 |
| 코드 생성·LLVM 완료 대기 | 175.269–184.560 | 168.081–191.518 |
| 최종 링크와 파일 처리 | 0.310–0.333 | 0.191–0.204 |
| rustc 내부 total | 228.615–250.487 | 195.424–221.814 |
| 위 링크 구간 중 실제 rust-lld 프로세스 | 약 0.127 | 0.113–0.123 |

세부 참고: 현행 type check 10.989–12.157초,
borrow check 9.664–10.452초,
`generate_crate_metadata` 25.925–36.150초,
`LLVM_passes` 172.468–181.371초.
이 세부 값은 상위 구간에 포함되므로 표에 더하지 않는다. 모노모피제이션 작업이 메타데이터/코드 생성 구간 사이에서 이동하므로 메타데이터 행 차이를 그대로 순절감으로 계산하지 않는다.

`codegen_crate + finish_ongoing_codegen`을 코드 생성 구간으로 묶었다. `LLVM_passes`는 내부 원인 확인용이다.
`link`는 rustc의 파일/아카이브 처리까지 포함하고, 별도 wrapper가 실제 rust-lld 프로세스 시간을 기록한다.
나머지 total은 파싱·매크로·이름 해석·타입/borrow 검사·메타데이터·정리 비용이다.
타이머 경계는 동일 버전 [passes.rs](https://github.com/rust-lang/rust/blob/1.93.1/compiler/rustc_interface/src/passes.rs)와
[queries.rs](https://github.com/rust-lang/rust/blob/1.93.1/compiler/rustc_interface/src/queries.rs)에서 대조했다.

## 진단 옵션 없는 반복 빌드

| runner 순서 | 현행 Cargo(초) | 후보 Cargo(초) | 후보−현행 | 재컴파일 범위 |
|---|---:|---:|---:|---|
| abba | 230.247 | 197.038 | -33.209 (-14.42%) | 본체 1개씩 |
| baab | 252.203 | 216.259 | -35.944 (-14.25%) | 본체 1개씩 |

n=2의 같은 runner 내 비교다. 일반적 고정 절감률이나 전체 Render Diff job 개선율로 확대하지 않는다.
첫 후보 빌드는 설정 전환으로 173개 target이 재컴파일됐고 Cargo 총시간이 본체 rustc 시간보다 약 53–56초 길었다. 이 준비 실행은 위 반복 빌드 비교에서 제외했다. 비계측 비교에서는 양쪽 모두 rhwp 본체만 재컴파일했는지 위 표와 원시 목록으로 확인한다.
다른 runner·이전 source SHA·dev/release-test 수치를 합쳐 성능 개선율을 만들지 않았다.

## release 후처리와 기능 검증

위 실행의 **비계측** `4-dual`, `3-cdylib-baseline-lto` raw WASM을 입력으로 사용했다.
이미 측정한 Cargo를 검증 runner에서 다시 빌드했다고 주장하지 않는다.
이전 wasm-pack 0.15.0 release에서 관측한 것과 동일한 wasm-bindgen 0.2.127 `--target web --typescript`,
wasm-opt 117 `-O`를 실행하고 실제 도구 바이너리 SHA-256을 확인했다.

| 조건 | 최적화 후 WASM(bytes) | bindgen(초) | wasm-opt(초) | Canvas 명령(초) |
|---|---:|---:|---:|---:|
| dual | 11,341,645 | 2.814 | 117.512 | 31.050 |
| cdylib-baseline-lto | 11,303,194 | 2.756 | 114.161 | 30.317 |

후처리 시간은 36516437863의 별도 runner n=1 비교다. Canvas 시간은 그 **동일 최적화 산출물**을 해시 확인 후 읽은 36517281262의 실측이며 Rust/wasm-opt를 재실행하지 않았다. 앞 Cargo 시간에 더해 전체 CI 실측이라고 표현하지 않는다.

첫 브라우저 시도는 public/samples에는 있지만 Vite /samples middleware가 읽는 루트 samples 경로에는 없는 BlogForm 파일 때문에 양쪽 모두 HTTP 404였다. 재검증에서는 11개 원본 SHA-256을 확인하고 누락된 경로 5개에 동일 바이트만 복사했다. 기존 경로 6개는 내용 일치를 검사하고 덮어쓰지 않았다. production Vite 코드는 변경하지 않았고 원본 문서·기준값·허용 오차도 바꾸지 않았다.

- Node에서 수정된 version 반환값 실행, JS exports / WASM imports·exports 일치: **확인**.
- TypeScript 선언은 `InitOutput`의 `version`/`init_panic_hook` 두 readonly 속성의 순서만 달랐다. TypeScript 7.0.2의 양방향 대입 검사는 통과했고, 반환형을 boolean으로 바꾼 음성 대조는 TS2322로 양쪽 방향에서 실패했다. 나머지 선언 바이트는 같았다. 파일 바이트 동일성과 타입 호환성을 구분한다.
- 기본 Canvas 3개는 두 조건 모두 통과했다. 확장 11개 fixture 첫 페이지에서 **양쪽 10/11 통과**, 결과 JSON 동일: **확인**. `BlogForm_BookReview.hwp`는 두 조건 모두 legacy/layer 사이 571/440,960 픽셀(0.12949%) 차이로 기존 0.05% gate를 넘었다. 따라서 브라우저 workflow는 failure이며 전체 검증 성공이라고 주장하지 않는다.
- 현행/후보 사이 동일 PNG **33개**, 다른 PNG **0개**. 실패한 BlogForm의 legacy/layer/diff PNG까지 양쪽 동일했다. KTX, BlogForm, biz_plan 대표 PNG를 직접 열어 문서 내용도 확인했다. 이는 후보가 새 차이를 만들지 않았다는 증거이며 한컴 기준 출력 일치 증거는 아니다.
- 초기화 때 브라우저가 실제 받은 WASM의 SHA-256을 manifest와 대조했다.
- Native/PDF/readiness 전체 gate, 전체 문서의 한컴 출력 일치, 일반 런타임 성능은 이번 범위가 아니다.

## 첫 후보와 계측 보정

Cargo 1.93.1의 [LTO 선택 로직](https://github.com/rust-lang/cargo/blob/083ac5135f967fd9dc906ab057a2315861c7a80d/src/cargo/core/compiler/lto.rs#L109)은
모든 crate type이 LTO를 지원할 때만 적용한다. `rlib+cdylib`는 `lto=true`여도 실제 `-C lto`가 빠지고,
cdylib만 선택하면 이를 적용한다. 처음 후보는 메타데이터를 줄이는 동시에 fat LTO를 켰으므로 느려졌다.
이를 'rlib 파일 하나 없애면 무조건 빠르다'로 해석하지 않는다.

초기 계측은 `RUSTC_BOOTSTRAP=1` 때문에 Rust 1.93.1 기본 심볼 이름이 legacy에서 v0로 바뀌었다.
처음 실행의 **비계측 성능 대조는 유지**하되 계측 분해값을 최종 표로 사용하지 않았다.
후속 run 36514395974는 각 runner 약 4분 진행 후 취소했다. 이후 계측에만 legacy를 명시하고
같은 수정 소스로 계측/비계측 raw WASM 해시를 대조했다. 각 조건별 4개 산출물(2 runner × 계측/비계측)이 모두 바이트 단위로 일치했다. 작은 smoke에서도 해시 동일성을 확인했다.
수정된 진단은 같은 compiler의 time-passes를 쓰며 제품에서 BOOTSTRAP을 사용하는 제안이 아니다.

## 기여 후보와 남은 조건

1. **WASM 전용 산출물 선택을 명시하는 작은 변경**이 대상이다. 루트 crate의 Native/Rust rlib 계약은 유지한다.
2. 현행 effective no-LTO를 유지할지, manifest의 fat LTO 의도를 WASM에도 적용할지 메인테이너가 선택할 수 있도록 실측을 공개한다.
   후자는 이번 단순 cdylib 대조에서 속도 개선이 아니었다.
3. production 통합 시 wasm-pack Cargo 호출/후처리/Studio 동기화 경로를 명시적으로 연결하고,
   캐시 키에 crate type과 실제 profile override를 포함한다. 명령만 바뀌어도 같은 release 캐시라고 취급하지 않는다.
4. 실제 PR 전 최종 command 경로의 cold/changed-source 비용과 기존 release 검증 gate를 실행한다.
   이번 조사만으로 production 배포 완료나 #7474 병합 준비를 선언하지 않는다.
5. LLVM 비용은 여전히 남는다. 큰 생성 데이터만의 분리나 링커 교체보다, 실제 코드 생성량을 줄이는 crate 경계가 다음 구조적 연구 대상이다.
   그 경우 cross-crate 최적화와 런타임 영향도 별도로 검증해야 한다.

## 재현과 증거

- [첫 대조 36512273477](https://github.com/edwardkim/rhwp/actions/runs/36512273477): 단순 cdylib + fat LTO 대조.
- [보정된 대조 36514652526](https://github.com/edwardkim/rhwp/actions/runs/36514652526): 현행 effective LTO 고정, 심볼 이름 보정.
- [후처리 36516437863](https://github.com/edwardkim/rhwp/actions/runs/36516437863): postprocessing/API 완료, 최초 Canvas는 fixture 404로 실패. 코드 SHA `0bea9faa7a4d0e2cc8aefe91c608d5b592ebafd0`.
- [브라우저 대조 36517281262](https://github.com/edwardkim/rhwp/actions/runs/36517281262): 기본 3개 통과, 확장 11개 중 양쪽 1개 동일 실패, PNG 33개 동일. 코드 SHA `6f6767b71453f7d0f5d4de8f85fa04ab01eeb174`.
- `scripts/experiments/rust_build_phases.py`, `validate_rust_phases.py`, `validate_rust_phases_browser.py`, `compare_wasm_declarations.mjs`: 독립 실험 ref의 재현·판정 코드.
- [반복 수치와 단계별 시간](assets/issue-7473/rust-phases/measurements.json), [계측 동등성](assets/issue-7473/rust-phases/aggregate-verification.json), [후처리](assets/issue-7473/rust-phases/postprocessing-summary.json), [브라우저 결과](assets/issue-7473/rust-phases/browser-summary.json), [선언 호환성](assets/issue-7473/rust-phases/declarations-check.json): 저장소에 보존한 주요 증거.
- 전체 원시 자료는 위 Actions artifact와 조사 작업공간의 `output/issue-7473/rust-build-phases/`에 보존했다. Actions artifact 보존 기간과 무관하게 주요 측정값·해시는 위 추적 파일에 남긴다.
- 원시 자료: 각 build.json, source.json, rustc.json, rustc.stderr, linker.json, cargo.jsonl, raw WASM,
  최적화 도구 버전/해시, API/브라우저 소비 기록, 결과 JSON과 PNG.

실험 브랜치의 render-diff.yml은 dispatch 전용 임시 하니스다. 기존 production workflow를 교체하는 PR이 아니다.
새 PR·댓글·병합·production cache 변경은 수행하지 않았다.

## 실제 wrapper 통합 후속 결과

[2026-09-30 후속 보고서](task_m100_7473_wasm_wrapper.md): 같은 runner에서 wasm-opt를 포함한
전체 release 패키징 354.324 → 320.743초(−9.48%, n=1), 실제 Canvas/PDF/readiness 대조와 적용 후보.
