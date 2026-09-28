# 본문 누름틀 종료 빈 줄 — #7353

원본: `samples/issue6601/36331407_side_by_side_tac_tables.hwpx`.
본문 문단2의 ClickHere 시작 ID1561678090을 문단34가 닫는다. 종료 문단은 빈 문자열이지만
저장 줄 높이1600HU, 뒤 간격1280HU를 가진다. 종료 마커가 보이지 않는다는 이유로
문단을 제거하거나 높이를0으로 만들지 않는다.

## 입력 생성과 판정 범위

`python3 tests/fixtures/issue7353_body_end_review/create.py`로 `after.hwpx`를 만든다.
원본 모든 문단을 보존하고 문단33의 서식에 새 텍스트를 넣은 후속 문단을 덧붙인다.
새 문단의 저장 LineSeg는 제거한다. 원본 종료 문단의 저장 정보는 바꾸지 않는다.
이 입력은 **원본이 아닌 가시 후속 문단을 추가한 대조군**이다.

`--bounded`는 원본 문단0,1,2,3,32,33,34와 새 후속 문단만 남긴 `bounded.hwpx`다.
남긴 문단 내부·표·저장 줄과 다른 ZIP 리소스를 바꾸지 않는다. 2쪽의 `붙임 2` 다음
빈 줄, `후속 문단: 누름틀 종료 빈 줄 다음`의 시작 위치를 비교한다.
원본 전체의 인증 자료가 아니라 누름틀 종료 빈 줄의 독립 판정 자료다.

`--before`의 `before.hwpx`는 원본 앞34문단(종료 문단 제외)을 남긴 진단본이다.
수정 전 V2에서도 실행되며 전체 원본과 같은 입력이라고 부르지 않는다.
기존 바이너리의 이 prefix 출력과 수정 후 전체 원본의 가시 SVG는4쪽 모두 동일하다.

## 독립 기준 PDF

2026-09-28 KST, 각 HWPX를 한컴 MCP `start → status → download`로 변환했다.
engine2020 / Hancom11.0.0.9136 / managed direct DLL / 32bit / preprocess none.
rhwp가 만든 PDF가 아니다. 영수증: `output/7353/r19/body-end/*pdf-*.json`.

| 입력 | PDF | 쪽 | 작업 ID |
|---|---|---:|---|
| after.hwpx | after-hwp2020.pdf |4| e72d0dd0-7f0a-4ed1-b8a1-33269e4df16e |
| bounded.hwpx | bounded-2020.pdf |2| 064d106b-ed6c-4552-9538-b56a99ca86cd |

SHA256:

- after.hwpx: `4e74fb828276a642e4b631ecd802cae3623503ab15823238a6d57f783c48111d`
- after-hwp2020.pdf: `8e27f437f4cb5a59bdc8e3c78a1a4731c58de6e90ffc9be131246b53e91621c2`
- bounded.hwpx: `5f1b127b59d2e6bc180caffb2db50b994a267be8ff2d8cf048bf1248052623e2`
- bounded-2020.pdf: `fd478d353a69c027693533f7d628368bce3e80f70b0027dfb589a4a4975c769b`

## 검증과 남은 차이

`tests/cases/issue_7353_body_field_end.rs`는 원본 저장 메트릭을 기준으로 종료 빈 줄의
실제 높이와 다음 줄 원점(2880HU 차이), 종료 후 완전 소진, 잘못된 ID·중복·교차 종료의
거부를 검사한다. 계약의 AFTER FIELD는 저장 메트릭을 가진 합성 후속 문단으로,
이 폴더의 한컴 PDF 대조군과 구분한다.

전체 after4쪽은 한컴보다 기존 붙임 두 줄 및 후속 문단이 약8.6px 아래에 있다.
수정 전 prefix 출력에도 존재하며 이번 종료 처리로 이동한 것이 아니다.
3쪽의 별표 문단 재조판 차이와 대체 글꼴 외형 차이도 남는다.
bounded2쪽의 종료 빈 줄·후속 문단 위치 비교를 전체 문서 일치로 확대하지 않는다.
Native/fresh WASM과 직접 판독 결과는 stage19 및 `body-end/` 증적에 연결한다.
