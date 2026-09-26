# #7353 재조판 들여쓰기 대조 문서

`fresh-input.hwpx`는 `create.rs`로 작성한 **LineSeg 없는 입력**이다. V2는 이 파일을 직접
재조판한다. `fresh-saved.hwp`는 같은 입력을 한컴에서 정상 저장한 독립 관측 자료이며,
그 줄 정보를 V2 입력에 주입하지 않는다. `fresh-2020.pdf`는 그 HWP의 한컴 출력이다.
#6923 원본의 완성/시각 통과 증거가 아니다.

## 생성과 출처

- 작성 출발점: `../issue7353_indent_review/indent-input.hwpx`.
- 명시적 줄바꿈을 가진 들여쓰기/내어쓰기·Left/Center/Right 문단과 실제 빈 문단을 작성했다.
- fresh body 경로가 지원하는 문단 기준 offset0/바깥여백0인 표를 새로 작성했다.
  초기 offset 보존 진단 입력은 `output/7353/r19/fresh-indent/draft-anchor.hwpx`에 남겼고,
  `nested anchor, TAC, wrap or outer margin` 거부를 우회하거나 저장 캐시를 수정하지 않았다.
- 선언 셀 최소 높이28000HU, 마지막 문단 아래 간격0. 셀 끝 아래 간격 문제와 분리한 대조군이다.
- MCP HWP 저장 job: `1d9d503b-8d6b-4427-9092-a4e2c3d7dbf1`.
- MCP PDF job: `c1fb4f2b-c07f-47bc-9032-e0f3dccd02c5`.
- engine2020, Hancom11.0.0.9136, direct DLL32bit, preprocess none,
  font verified2/failed0, PDF print method0/one-up, 1쪽.

SHA-256:

| 파일 | SHA-256 |
| --- | --- |
| fresh-input.hwpx | f27b70b620698c0a39a8287558e14b466b2079f02ae5d464906200e777498cce |
| fresh-saved.hwp | 4fbf05ee9f0dd5178951c1cf68ad90e4b893c998a1042edbd09c048d2cca34d3 |
| fresh-2020.pdf | 7ec51ee78279f5de097aca335c1350a0571c9e3bd330f7d4ecbda8446026738f |

## 독립 기대값과 범위

원본 속성은 문단 양쪽 여백500HU, 들여쓰기 절댓값1500HU(15pt), 셀 padding283HU다.
한컴은 양수 문단 첫 줄, 음수 문단 후속 줄에 bit20을 기록한다. 실제 빈 문단도 양수 첫 줄이다.
한컴 저장본의 줄 시작 y는0,1760,3520,5680,7440,9200,10960,13120,14880,16640,
18800,20560,22320HU다. 마지막 두 본문 줄은28000,29760HU에서 시작한다.

정식 `issue_7353_table_v2_document_flow` 검사는 원본 무저장 상태와 한컴 저장 상태를 구분하고,
실제 최종 줄 좌표·텍스트·빈 줄·표 높이·후속 본문 위치를 검사한다.
폭은 입력 선언32000 - 셀 좌우566 - 문단 좌우1000 =30434HU에서 inset을 뺀다.
한컴 저장 segment 폭30432HU와 **2HU(96dpi에서 약0.027px) 차이**가 남는다.
이 차이를 숨기려고 저장 폭으로 clamp하지 않는다. 폰트 외형 차이와도 별개다.

짧은 명시 개행 문서는 폰트 폭에 따른 자동 줄바꿈 오차 없이 inset·정렬을 직접 판독하기 위한
대조군이다. 폭 부족 자동 줄바꿈·중첩·페이지 이어받기는 별도의 합성 정식 계약으로 검사하며,
그 통과를 한컴 출력과의 일치로 바꾸어 주장하지 않는다.
