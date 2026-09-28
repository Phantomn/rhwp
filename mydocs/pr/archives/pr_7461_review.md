# PR #7461 통합 검토 — #7458 install-action 갱신

## 최종 판정

**머지 보류 — 최신 PR head의 GitHub Actions 완료 대기.** 로컬 검증은 통과했다.
최신 required checks와 mergeability 확인 및 별도 병합 지시가 완료 조건이다.

## 접수와 적용

- 통합 PR: [#7461](https://github.com/edwardkim/rhwp/pull/7461), 작성자 jangster77의 self-review, reviewer 미지정.
- 원 PR: [#7458](https://github.com/edwardkim/rhwp/pull/7458), Dependabot 작성.
- 고정 base: `ba4fa3ccce2f1e2fe711a510bf36e04772451a4b` (`upstream/devel`).
- 원 source: `753565b0b1e7cd1ed861f6d0bf68b588227f5b45`.
- 체리픽·검증 후보: `1231f74fbb95c49ee14b07bcbc4892f5e4f4453b`.
- 작업 branch: `integration/dependabot-7458-20260928`.
- `devel`을 fast-forward 동기화하고 새 branch에서 `git cherry-pick -x`를 실행했다.
  충돌 없이 원 저자를 보존했다. 원 PR 종료·GitHub review·merge는 수행하지 않았다.
- 기본 경로: collaborator_self_merge. 보조: intake_and_review, review_template,
  local_validation, review_only_fast_pass, rework_and_exceptions. 공통 라우터·선택표와
  해당 문서, github_operations, docs_and_git_workflow를 읽고 적용했다.

## 변경과 검증

`build-nextest-archives.yml`, `run-nextest-archives.yml`의 install-action SHA 두 곳만
v2.87.15에서 v2.87.20으로 교체했다. 입력 `tool: nextest`, permission, trigger,
runner, job 이름, required check 배선은 변경하지 않았다.

검증 환경: Ubuntu 개발 서버, 위 체리픽 후보 SHA.

| 검사 | 실행·결과 |
| --- | --- |
| 공백 | `git diff --check upstream/devel...HEAD` 통과 |
| workflow 구문·ShellCheck | `actionlint .github/workflows/build-nextest-archives.yml .github/workflows/run-nextest-archives.yml` 통과 |
| archive 계약 | `python3 -m unittest discover -s scripts/tests -p 'test_nextest_archive_workflow.py'` 15 tests 통과 |
| 계약 테스트 CI 배선 | `python3 -m unittest discover -s scripts/tests -p 'test_workflow_contract_wiring.py'` 3 tests 통과 |
| Action SHA | 공식 tag API `repos/taiki-e/install-action/git/ref/tags/v2.87.20`의 commit이 `9983c65e42da123ff25d1f78505eb6de315aa172`과 일치 |

조판 원칙·Visual Sweep·HWP/HWPX/PDF 입력 커밋 확인은 **비해당**이다.
제품 Rust·WASM·Studio·fixture·baseline 변경이 없다. 제품 전체 로컬 빌드는 비해당이며
새 Action의 GitHub runner에서의 실제 nextest 설치·archive 실행은 원격 CI에서 확인해야 한다.

## CI와 후속 조건

최초 후보의 [CI run](https://github.com/edwardkim/rhwp/actions/runs/36383960201)은
PR 생성으로 시작됐으며 기록 시점에 진행 중이었다. 이 후행 문서 commit까지 포함한 최신
head가 최종 CI 확인 대상이다. workflow 변경 PR이므로 review-only 재사용을 가정하지 않는다.
source와 이 기록을 같은 PR에 포함한다. 이후 병합·원 PR 종료·코멘트는 별도 지시 범위다.
되돌리기는 위 체리픽 commit의 revert로 이전 Action SHA를 복원한다.
