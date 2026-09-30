# 바탕쪽·본문 TAC 도형의 공통 호스트 대조군

`create.rs`가 앞 절편의 절대 표/2단 입력에 바탕쪽과 본문 도형을 추가한다.
본문 LineSeg는 제거한 뒤 **한컴이 정상 저장하면서 다시 생성**한다.
원본 #7008 전체 지원 증거가 아니라 기능 경계를 읽기 쉽게 만든 제어 문서다.

## 생성 및 독립 기준

- `input.hwpx` → `saved.hwp`: job `1ee75387-4c43-49a6-b8f7-77afd8b80032`.
- 동일 `saved.hwp` → `reference-2020.pdf`: job `721cd1ff-9a4c-426c-86e2-7ce4592892fb`.
- 2026-09-30, 한컴 runtime11.0.0.9136/engine2020, 전처리 없음, 3쪽.
- 상태/다운로드 기록: `output/7353/r19/section-scope/master-{save,pdf}-*-v3.json`.

최초 2400HU TAC 도형/1800HU 고정 피치 입력은 공통 문자 배치의 줄 축소 때문에
V2가 거부했다. `tall-unsupported.hwp` 및 `master-tall-draft/`로 보존한다.
그 범위를 수용하도록 조건을 완화하지 않았으며 정식 테스트가 명시적 거부를 보호한다.
다음 1200HU 도형 초안은 한컴이 textbox 내부 여백으로 current_height를1202로 늘렸지만
common.height는1200을 유지하는 별도 미지원 사례다. `master-textbox-draft/`에 보존했다.
최종 입력은 **작성 단계에서 도형 높이를1600HU로 지정**한다. 저장 LineSeg를 조작하거나
초안의 정상 출력 일치를 주장하지 않는다. 두 초안은 이번 완료 범위가 아니다.

## 판독점과 기대값

단위 HU. 용지/다단/절대 표/빈 줄은 앞 절편의 `issue7353_host_absolute_review`와 같다.

- 1쪽: 바탕쪽 없음. 위 중앙/아래 오른쪽 표와 의도된 빈 줄·2단 본문은 유지.
- 2쪽: 양쪽 바탕쪽 `BASE MASTER`, 외곽 `(2400,27000,14000,3000)`.
- 3쪽: 홀수 바탕쪽 `ODD MASTER`, 외곽 `(18000,27000,14000,3000)`.
- 바탕쪽 글상자는 원문 위/아래 여백을 뺀 높이에서1200HU 텍스트 줄이 가운데에 온다.
  기준값은 이 정렬 불변식과 동일 HWP의 PDF이며 Legacy 결과를 정답으로 사용하지 않는다.
- 2쪽 왼쪽: LINE16~18, AFTER, `LEFT [BOX] RIGHT`, TAIL01~03.
  도형 소유 줄 원점 y15200/높이1600, 도형 폭4500. TAIL01은 y17000.
- 2쪽 오른쪽: TAIL04~11. 3쪽 왼쪽: TAIL12~14/END(y13400).
  바탕쪽은 본문 공간을 소비하거나 본문/도형을 복제하지 않는다.

`tests/cases/issue_7353_hosted_master.rs`는 정상 저장본/HWPX 왕복/96·192dpi,
최종 외곽·텍스트 줄·뒤 문단, 기존 문서 호스트의 바탕쪽 선택을 검증한다.
바탕쪽 제거/첫 쪽 표시/도형 단독 줄/미지원 속성 변형은 합성 계약이며 별도 한컴 일치 증거가 아니다.

남는 차이: fallback 글꼴의 외형·미세 폭, PDF custom-paper399pt 대 HWP400pt의
인쇄 transform 때문에 발생하는 하단 약0.75pt 차이. 출력/이미지 좌표 보정은 하지 않는다.

## SHA256

```text
b58d09e9e1b8f21d705c63b8f0c22b31e4c23833ac1261134d2388a79e9ea3d6  input.hwpx
fadcc0c7ce9b0a9de40f2f720623451678f2434ca80a4ab0bccda1ed632ab58a  saved.hwp
ba47ae3cf39bc4982bc86c944204312ef75887cbf9107b18a04536a65ba6a1d9  reference-2020.pdf
```
