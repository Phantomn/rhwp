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

검증 진행 중. 원격 push·PR 생성·새 Actions 실행은 아직 수행하지 않았다.
