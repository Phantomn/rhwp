# #7473 동일 소스 WASM 패키지 재사용 실험

Issue: #7473. 사용자 요청에 따라 반복 빈도 → 좁은 재사용 계약 → 새 runner 실측 순서로 진행한다. #7474는 Draft/head를 유지한다.

## 관측과 범위

2026-08-29 이후 조회 가능한 Render Diff 965 run 중 본 조사용 ref 15개 run을 제외했다. 일반 950 run / 1,010 attempts에서 같은 run의 실제 WASM 성공 빌드 반복은 1회, 다른 run의 동일 head까지 포함하면 최대 6회다. attempt Jobs API의 복사된 이전 job은 실행 시각으로 중복 제거했다. 같은 head가 같은 checkout merge SHA라는 보장은 없어 6회는 상한이다.

현재 상시 도입의 경제성은 낮다. production workflow 변경이나 새 PR로 확대하지 않고, 이 실험 ref에서 같은 run의 producer → 새 runner consumer에 한정한 최종 package 재사용을 검증한다. PR과 trusted branch 사이의 공유는 허용하지 않는다. GitHub cache를 추가하지 않으며 작은 임시 Actions artifact를 쓴다.

## 계약과 검증

- 같은 repository/run/source SHA·tree, release/web/기본 feature, locked 입력과 Rust/wasm-pack 실행 파일을 고정한다.
- wasm-bindgen/wasm-opt 버전과 실제 옵션은 빌드 증거로 확인하고 허용한 계약과 맞지 않으면 재사용하지 않는다.
- producer job이 전달한 불변 artifact ID만 내려받는다. 패키지 파일 목록·크기·SHA-256 및 build manifest를 검증한 뒤 pkg와 Studio public 두 소비 위치를 반영한다.
- 누락·내용 손상·다른 SHA·profile·도구·출처는 miss로 처리해 정상 빌드한다. 임의 경로나 symlink를 설치하지 않는다.
- consumer에서도 실제 브라우저 WASM 응답 해시와 Canvas 회귀 검사를 실행한다. producer/consumer PNG와 결과를 비교한다. PDF/native 검증이나 production 전체 Render Diff를 실행한 것으로 보고하지 않는다.
- 로컬 계약 검증은 실제 helper를 호출하는 임시 package 테스트로 하고 workflow는 actionlint로 검사한다.
- 작은 실험의 결과를 바탕으로 재사용 가능성, 복원 부대 비용과 도입/보류 결론을 기록한다. 비용 수요가 낮으면 실험 성공만으로 상시 활성화하지 않는다.

## 운영 경계

실험 branch만 push하고 기존 Render Diff dispatch entrypoint를 이 ref에서만 대체한다. production trigger/check/permission은 변경하지 않는다. contents:read, 외부 action SHA 고정, artifact 보존은 3일. 원상복구는 실험 ref를 사용하지 않는 것이며 사용자 소스나 #7474에 영향을 주지 않는다.

## 구현 전 로컬 검증

실제 helper를 호출하는 unittest 13개(여러 입력 경계 subtest 포함), actionlint, Python/Node 구문, git diff --check 통과. 정상/손상 consumer는 같은 불변 producer artifact ID만 사용한다. 기존 #7474의 빌드 계측 및 브라우저 응답 관찰 코드를 재사용했으며 Rust 제품 코드는 변경하지 않았다.

원본 checkout 로그까지 대조한 결과 동일 head 6회 상한 중 실제 같은 checkout SHA 반복은 **4회**였다. 나머지는 PR merge SHA가 달랐다. 같은 run 안 반복은 여전히 1회다. 상시 도입을 보류하는 판단은 유지한다.

## 완료

최종 code SHA `cef17e8de83049046638dad93331b13bf1ce16b6`, Actions run `36464037905`의 producer/hit/corrupt 3개 job 모두 성공했다. 로컬 최종 unittest는 잘못된 build manifest 형식 대조를 포함해 14개 통과. 정상 패키지 준비 step 430→3초, 손상 fallback 및 3개 문서 Canvas/실제 응답 hash/PNG 동등성을 확인했다. 반복 수요가 작아 production 도입은 보류하며 #7474를 변경하지 않았다. [결과 보고서](../report/task_m100_7473_package_reuse.md)에 수치·증거·한계를 기록했다.
