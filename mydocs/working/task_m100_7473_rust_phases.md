# 실제 Rust 수정 후 release 컴파일 단계 실험

Issue: #7473. 기준: upstream/devel `0e8fd49fb868da0d47ac1294dcbbda81f0211233`.
PR #7474는 Draft/head 그대로 둔다. 이 브랜치는 production 병합 후보가 아니다.

- `src/lib.rs`의 WASM version 반환값에 표식을 실제 추가하고 finally로 복원한다.
- Ubuntu 24.04 / Rust 1.93.1 / release / default features / opt-level 3 / CGU 1을 유지한다.
- 의존성을 seed한 뒤 같은 runner에서 dual(rlib+cdylib)과 cdylib-only를 ABBA/BAAB로 대조한다.
- 2개 독립 runner에서 각각 앞 2개는 root에만 time-passes와 링커 실행시간 계측, 뒤 2개는 비계측 대조다.
- 진단은 같은 stable compiler에 한해 RUSTC_BOOTSTRAP=1로 time-passes를 연다. 제품 빌드 정책 변경이 아니다.
- Cargo 총시간, 실제 rustc 인자, nested compiler 시간, 실제 rust-lld 프로세스 시간을 구분한다.
- 후보는 `cargo rustc --crate-type cdylib`다. manifest의 release 최적화 조건은 낮추지 않으며 실제 LTO 인자의 차이를 공개한다.
- 후보의 이득이 확인되면 wasm-bindgen/wasm-opt를 생략하지 않은 package 및 실제 브라우저 Canvas 대조를 수행한다.
- raw/최적화 후 크기, 출력, 검사 범위와 미검증 범위를 기록한다. Native Rust rlib 계약을 제거하는 변경은 제안하지 않는다.
- 코드/실행 기록을 먼저 커밋한 뒤 결과 보고를 별도 커밋한다. 새 PR·댓글·캐시 생성·생산 workflow 변경은 하지 않는다.

## 첫 대조 뒤의 범위 보완

실행 36512273477에서 순수 cdylib는 실제 LTO 적용을 켜면서 9–12% 느려졌다.
본체 기준선의 실제 linker는 약 0.13초였고, LLVM 구간이 176–179초였다.
후속 원인 분리에서는 cdylib만 선택하면서 명령 범위의 `--config profile.release.lto=false`로
현행 dual WASM의 **실효 cross-crate LTO 없음**을 고정한다. opt-level=3/CGU=1/wasm-opt 조건은 유지한다.
이는 nominal release 설정을 문자 그대로 유지하는 안과 구분한다. manifest를 전역 변경하거나
Native release LTO를 끄는 안은 아니다. 실제 옵션·속도·최종 산출물과 계약 차이를 공개한 뒤 채택 가능성을 판단한다.

## 계측 교란 보정

첫 계측에서 BOOTSTRAP에 의해 root 심볼 이름이 v0가 된 것을 raw WASM name section으로 확인했다.
첫 실행의 비계측 성능 대조는 유효하지만 계측 수치는 참고값으로 둔다.
후속 run 36514395974는 진행 중 취소하고, 계측에 `-Csymbol-mangling-version=legacy`를 명시한다.
이후 계측/비계측은 모두 같은 실제 수정 소스(marker 1)로 비교해 raw WASM hash 일치 여부도 검사한다.
비계측에는 BOOTSTRAP이나 진단/이름형식 override를 추가하지 않는다.

## 완료 결과

실효 LTO 조건을 맞춘 두 runner의 비계측 본체 빌드는 14.25–14.42% 단축됐다.
opt-level 3 / CGU 1 / wasm-opt를 유지했고, 기본 Canvas 3개와 API 호환성은 통과했다.
확장 Canvas는 양쪽 모두 같은 1개 fixture가 실패했으며, PNG 33개는 현행/후보 사이 동일했다.
일반적인 전체 CI 개선이나 production 적용 완료로 확대하지 않는다.
보정된 최종 수치, nominal/effective LTO 차이, 재현 코드와 남은 검증은
[최종 보고서](../report/task_m100_7473_rust_phases.md)에 기록했다.
