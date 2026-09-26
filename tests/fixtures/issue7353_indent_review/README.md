# #7353 저장 문단 들여쓰기 독립 대조

`indent-saved.hwp`는 LineSeg를 수작업하지 않은 한컴 정상 저장본이다.
`indent-2020.pdf`는 그 HWP의 한컴 출력이다. 원본 #6923 전체 통과 자료가 아니다.

## 생성과 독립 근거

- `create.rs`는 승인된 anchor 대조군의 페이지/표 구조를 재사용하고 저장 줄을 비운
  HWPX를 작성한다. 한 셀에 첫 줄 들여쓰기/내어쓰기와 Left/Center/Right 문단을 배치한다.
- ParaShape 좌우 여백1000, 들여쓰기±3000은 각각 실제500HU/1500HU다.
  저장 줄은 모두 cs500/sw30432이며, 첫 줄 들여쓰기는 첫 줄, 내어쓰기는2~4줄에
  `TAG_INDENTATION`을 기록한다. cs/sw 자체에는1500HU가 들어 있지 않다.
- PDF 첫 문단 첫 줄 x62.442pt/후속 줄47.461pt, 둘째 문단 첫 줄47.461pt/후속 줄62.442pt.
  15pt 차이는 문단 값의1500HU와 대응한다. PDF 수치의 소수 차이는 출력 드라이버의 raster grid다.
- 넷째 문단의 보이는 끝은 약351.50pt로 같고, 셋째 문단은 줄별 가용 상자 안에서 가운데 정렬된다.
- HWP:1쪽, 표 x3969/y8787/폭32000/높이29266HU, 뒤 문단 y38620HU(본문 원점5669+저장32951).
  글꼴 대체 외형은 조판 판정과 구분하며 저장된16개 줄의 텍스트 범위를 보존한다.
- 생성: 라이브러리에 `rustc --edition=2021 create.rs --extern rhwp=… -L dependency=…`로
  연결한 실행 파일을 repository root에서 실행한다. canonical MCP async 절차로 HWPX→HWP→PDF.
  저장된 HWP를 다시 편집하거나 LineSeg를 덮어쓰지 않는다.
- HWP job `04bbc154-8440-4f19-9abd-02594a126390`, PDF job
  `350a7485-b873-423f-8875-ba3a6192dbef`; engine2020/Hancom11.0.0.9136,
  direct-DLL32bit, preprocess none, session font verified2/failed0, one-up printMethod0.

| 입력/출력 | SHA-256 |
| --- | --- |
| indent-input.hwpx | `926a60c80d74a03315c78a4a1d7dc6899fbdb700f9f7703adac5748ae71b9b2b` |
| indent-saved.hwp | `c16b1f3747e22805f37122fc6255c9783a061c5429f33c2c26eeb33400c60c74` |
| indent-2020.pdf | `a06b1828da80bb8db28ad07007ad0d3adc0a136aaaa3e59981ee29d92127b5bf` |

## 미통과 대조 보존

`diagnostic/`는 마지막 문단도 아래 간격800(실제400HU)을 갖는 먼저 생성한 정상 저장본이다.
V2는 이 간격을 셀 끝에도 포함해 한컴보다 표 하단과 뒤 문단이400HU=5.333px 내려간다.
이는 들여쓰기 처리로 해결한 문제가 아니다. 이번 정상 대조는 마지막 문단 아래 간격만0으로
작성해 한컴에서 다시 저장/출력했다. 진단 원본을 바꾸거나 새 대조 통과로 덮지 않는다.
`create --terminal-gap`으로 이 편집 속성 조합을 재생성할 수 있다(한컴 저장 파일 바이트는 별도).

- 진단 HWP job `aaa4886e-6465-4199-a2be-726340186f73`, PDF job
  `647aeb1e-ba20-4aaa-8745-a36b37819596`; 위와 같은 엔진/폰트/전처리 설정.
- 진단 HWPX SHA `4a26199c3f92f71f58ebaa426b4f9c8c5fbd2c4e71be6ac005c861797c65ebdf`.
- 진단 HWP SHA `ccece1a559d5e70743558b112393df8387f63aec3d0ac63a721130666c777326`.
- 진단 PDF SHA `8ec7cf3f488ad585915c8838cd5f2bb6cb894930b4ee717f28ac3400f106b254`.

실행 기록은 `mydocs/working/task_m100_7353_stage19.md`의 저장 문단 들여쓰기 절에 연결한다.
