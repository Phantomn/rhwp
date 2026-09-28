# 동일 소스 최종 WASM 패키지 재사용 조사·실험

**판정: 한정된 재사용·손상 fallback의 구현과 CI 검증은 성공했다. 다만 최근 한 달의 실제 동일 checkout 반복 기회가 4회뿐이어서 production 상시 도입 PR은 현재 보류하는 것이 타당하다.**

Issue #7473. #7474는 기존 Draft/head를 유지한다. 실험 branch `codex/wasm-package-reuse-7473`은 production에 병합할 workflow가 아니다.

## 1. 실사용 기회 조사

2026-08-29 이후 조회 시점까지 Render Diff API의 965 run을 조사했다. 본 세션의 조사·비교 ref 15 run을 제외하면 950 run / 1,010 attempts다. 전체 날짜 범위의 API 페이지와 반복 후보의 128개 attempt Jobs 응답을 보존했다.

| 분류 | 관측 |
|---|---:|
| 일반 run / attempts | 950 / 1,010 |
| 동일 head 반복 후보 run | 68 |
| 후보에서 실제 실행한 WASM 빌드 | 59 (모두 성공) |
| 동일 head의 추가 성공 빌드 | 최대 6 |
| 원본 checkout 로그까지 같은 SHA인 추가 빌드 | 4 |
| 같은 workflow run 내부의 추가 빌드 | 1 |

`59`는 반복 후보 안의 빌드 수이고 모든 CI의 WASM 빌드 총수가 아니다. 재실행 attempt에 복사되어 표시된 이전 job은 job/step 실행 시각으로 중복 제거했다. 같은 PR head라도 base 변화에 따라 checkout merge SHA가 다르므로 단순 head 집계의 6회를 재사용 가능한 횟수로 단정하지 않았다. 남은 4회도 tool/image/모든 환경의 일치까지 입증한 적중률은 아니다. 삭제된 run/만료된 로그와 다른 workflow는 포함하지 않는다.

같은 run 반복 사례: [34465225005](https://github.com/edwardkim/rhwp/actions/runs/34465225005)의 attempt 1/2. checkout `7f35f9714680e160cb133cf876abc1d19b498105`가 동일했다. 당시 dev 빌드이며 새 release 경로의 전체 절감량으로 환산하지 않는다.

현재 수요만으로 상시 패키지 저장·검증·복원·유지보수 비용을 정당화하기는 어렵다. 기능적 가능성과 새 runner에서의 실측은 아래 한정 실험으로 확인하고 도입 판단과 구분한다.

## 2. 구현한 좁은 계약

- GitHub cache를 추가하지 않고 **같은 run의 producer가 넘긴 불변 artifact ID**만 소비한다. 임의 이름, 다른 run, PR→trusted branch 공유는 없다. contents:read, 외부 action SHA 고정, artifact 3일 보존.
- producer는 locked release/web 기본 feature 패키지를 생성하고 실제 Canvas 검증을 성공한 뒤 내보낸다. repository/run/workflow/event, checkout SHA/tree, lock hash, Rust/Cargo/wasm-pack 및 환경·이미지가 같아야 한다.
- wasm-bindgen 0.2.127 및 wasm-opt 117의 실제 버전·실행 파일 hash·명령 인자를 기존 실행 증거에 고정했다. 다른 버전은 자동 수용하지 않는다.
- 패키지 7개 파일의 목록·크기·SHA-256, 빌드 성공/clean source 증거를 검사한다. 64 MiB 상한, symlink 거절. 전체 검증 뒤에만 root pkg와 Studio public 두 경로에 반영한다.
- 손상·누락·다른 계약은 miss가 되어 정상 locked release 빌드를 실행한다. 모든 경로에서 브라우저가 실제 받은 WASM 응답 bytes를 검증하고 Canvas 회귀 검사를 새로 실행한다.
- helper 단독의 checksum은 서명을 대신하지 않는다. 출처 경계는 GitHub의 같은 run·불변 artifact ID 전달과 읽기 권한이며, checksum은 그 bundle의 일관성 검사다. cross-run 신뢰·PR artifact 승격·일반 목적 캐시는 구현하지 않았다.

[artifact ID 다운로드](https://github.com/actions/download-artifact)와 [불변 artifact](https://github.com/actions/upload-artifact)의 공식 계약을 확인했고, 실험 action은 각각 v6의 정확한 commit에 고정했다.

## 3. 로컬 검증

실제 helper를 호출하는 14개 unittest(여러 입력 경계 subtest 포함), actionlint, Python/Node 구문, git diff --check 통과. identity 거절을 제거한 임시 오구현에서는 경계 테스트가 의도대로 실패했다(`negative-contract-test.log`). 제품 Rust 코드는 바꾸지 않았다.

정상/변경 SHA·profile·repository·run·tool, 손상 JS/WASM·manifest, 빠진/추가 파일, symlink, 크기 상한을 확인했다. 실제 cold/복원/손상 fallback 및 브라우저 검증은 CI 결과를 별도로 기록한다.

## 4. 실행

- 최종 코드 실행: https://github.com/edwardkim/rhwp/actions/runs/36464037905
- 초안 실행 36463795798은 잘못된 manifest 형식의 fallback을 보완하려고 준비 단계에서 취소했다. 완료 표본으로 사용하지 않는다.
- producer cold 1개 + 정상/손상 consumer 2개, Ubuntu 24.04·Rust 1.93.1·Chromium 1660786. source와 tool을 같게 하되 독립 VM을 사용한다.
- 기존 #7474의 계측 및 실제 WASM 응답 관찰 코드를 재사용했다. 새 비교는 Canvas 기본 3개 fixture의 첫 페이지만 다루며 Native/PDF/readiness 전체 CI나 한컴 기준 출력과의 정합성 검증을 대신하지 않는다.

## 5. 최종 결과

검증 SHA: `cef17e8de83049046638dad93331b13bf1ce16b6`. producer, 정상 consumer, 손상 consumer **3개 job 모두 성공**했고 세 boot ID가 서로 달랐다. GitHub API의 step 시간은 초 단위, helper의 wall은 monotonic이다.

| 경로 | 빌드 step | 다운로드 step | 검증·설치 step | Canvas step | 전체 실험 job |
|---|---:|---:|---:|---:|---:|
| cold | 430초 (success) | 0초 | 0초 | 12초 | 540초 |
| hit | 0초 (skipped) | 1초 | 2초 | 12초 | 113초 |
| corrupt | 429초 (success) | 0초 | 2초 | 12초 | 538초 |

- 정상 재사용: 패키지 준비 step 합계 **430초 → 3초**. 검증·설치 내부 실측은 **1.371초**, 다운로드 API step은 1초다. Rust target directory가 생기지 않았고 normal build step이 skipped였다.
- cold wrapper 내부 실측 **428.440초**: 준비 2.691초, Cargo 300.260초, bindgen 설치/실행 4.962초, wasm-opt/마무리 120.513초. 구간은 로그 경계 근사이며 전용 프로세스 CPU 시간이 아니다.
- 손상 대조: 내려받은 WASM 첫 byte를 바꿨다. `package checksum or size mismatch`로 거절하고 정상 release 빌드를 실행했다. fallback wrapper 내부 실측 **429.485초**. 잘못된 bytes를 브라우저로 보내서 통과한 것이 아니다.
- 전체 실험 job은 cold 540초 / 정상 hit 113초였다. n=1 대조이고 Native/PDF/readiness가 빠진 실험 job이므로 실제 #7474 전체 CI에 79% 개선이 적용됐다는 주장은 하지 않는다.
- 전달 package는 **4,337,399 bytes(약 4.14 MiB)**. producer의 identity sealing은 API 해상도에서 0초, upload는 1초였다. 0초는 비용이 없다는 뜻이 아니다. 생산·저장 비용은 consumer 절감과 별도로 남는다.

### 검증 결과와 시각 범위

세 경로 모두 `basic/KTX.hwp`, `biz_plan.hwp`, `tac-case-001.hwp`의 첫 페이지 **3/3 통과**. 각 경로의 legacy/layer/diff PNG 9개, 총 27개를 비교했으며 파일명이 대응하는 PNG는 모두 byte 동일했다. Canvas 결과 JSON도 동일했다. 대표 KTX cold/hit 이미지를 직접 열어 지도·표 위치를 대조했고, 복원한 biz_plan/tac 첫 페이지도 확인했다. 기준값과 허용치는 바꾸지 않았다.

브라우저 초기화에서 실제 수신한 WASM 응답은 각 job 1건씩 총 3건 모두 manifest와 일치했다. JS/WASM의 hash는 정상 복원과 손상 후 재빌드 모두 cold와 같았다. 최적화 WASM SHA-256:

`d4f9a862800dad6ac2eb56b6d452a45c385a941bbcc84a0a9e78a273bd4be095`

재사용한 `build.json`의 428.440초는 **producer의 원래 생성 시간**이다. consumer 시간으로 다시 집계하지 않는다. consumer의 비용/재사용 여부는 별도 `reuse.json`과 step 시간에 있다.

## 6. 도입 판단과 남은 범위

**현재는 production 자동 재사용 PR을 만들지 않는다.** 같은 소스 반복 시 기술적 효과는 크지만, 실제 반복 기회가 드물다. 이 개선은 소스가 바뀌는 일반 PR의 release 빌드를 빠르게 만들지 않는다. 현재 단일 Canvas job을 producer/consumer로 나누기만 하면 최초 빌드는 그대로이고 job/전송 비용만 추가된다.

이번 구현은 같은 run의 순차 job 사이 전달을 검증한 프로토타입이다. 이전 attempt artifact 검색·선택, cross-run/cross-PR 공유, main/devel→PR 신뢰 경로, 임의 feature/환경에 대한 범용 캐시 정책은 구현하거나 검증하지 않았다. 한정 실험의 read-only token, 검증한 환경과 도구 범위를 일반적인 공급망 보안 보증으로 확대하지 않는다. 재시도나 동일 release package를 소비하는 CI job이 늘어날 때 다시 검토할 수 있다.

다음 성능 기여의 우선순위는 **소스가 달라져도 많은 PR에 적용되는 의존성 캐시 공급 복구 또는 변경된 본체 release 빌드 비용 분석**이 더 높다. #7474의 배포용 WASM 시각 검증 가치는 이 재사용 도입 여부와 별도로 판단한다.

#7474는 OPEN/Draft, head `0c4a2f9812732bdfff3094caa6c624a794c86b8a` 그대로다. production workflow와 기존 Cargo 캐시를 변경하지 않았고 새 PR·댓글·병합도 수행하지 않았다. 실험 artifact는 3일 보존하며 전체 원자료는 로컬에도 내려받았다.

## 7. 증거 위치

- `render-runs.json`, `attempts/`, `audit.json`: 조사 범위·중복 제거·실제 빌드 이력.
- `repeated-checkouts.json`, `repeat-logs/`: 동일 head와 실제 checkout SHA의 구분.
- `36464037905/{run,jobs,summary}.json`, job별 `.log`: 최종 실행·attempt·시간.
- `36464037905/artifacts/`: package 전체, build/consumption/reuse manifest, Canvas 결과 및 PNG.
- `metrics.json`, `artifacts-final.json`, `pr-after.json`: 동등성 판정·artifact 크기·Draft 유지.
- `negative-contract-test.log`: identity 검사를 제거한 오구현의 예상 실패.
- 실험 소스: [고정 SHA의 helper](https://github.com/edwardkim/rhwp/blob/cef17e8de83049046638dad93331b13bf1ce16b6/scripts/experiments/wasm_package_reuse.py), [workflow](https://github.com/edwardkim/rhwp/blob/cef17e8de83049046638dad93331b13bf1ce16b6/.github/workflows/render-diff.yml).

공유용 숫자 정본: [측정·검증 집계](assets/issue-7473/package-reuse/metrics.json), [반복 빈도 집계](assets/issue-7473/package-reuse/audit-summary.json). 위 전체 원자료는 로컬 저장소 `output/issue-7473/wasm-package-reuse/` 아래에 있으며 CI artifact 링크는 보존 기간 동안 사용할 수 있다.
