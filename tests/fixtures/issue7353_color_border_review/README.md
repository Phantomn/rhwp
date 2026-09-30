# #7353 색이 다른 공유 셀 실선

## 입력과 독립 기준

- 원본: `samples/issue4090/156492236_규제샌드박스_min.hwpx`.
- `input.hwpx`: 원본 Document의 section0 문단을 처음2개로 truncate한 추출본.
  표/셀/문단/줄/BorderFill 속성을 바꾸지 않았다. 원본은 보존했다.
- `saved.hwp`: 추출본을 한컴에서 정상 저장한 입력.
- `shared-2020.pdf`: 위 saved.hwp를 한컴이 출력한1쪽 PDF.
- `variant.hwpx`: saved.hwp를 파싱한 뒤 다음 두 셀의 BorderFill을 찾아 변경한
  합성 대조군. (row4,col4)의 오른쪽은 회색0.10mm, 아래는 검정0.12mm,
  (row4,col5)의 왼쪽은 검정0.12mm다. 같은 BorderFill을 참조하는 다른 셀에도
  적용된다. 크기/줄/셀 배치는 변경하지 않는다.
- `variant-2020.pdf`: variant.hwpx를 직접 한컴에서 출력한 독립 기준이다.
  재저장 HWP라고 부르지 않으며, 원본 충실도 증거와 구별한다.

환경: MCP engine2020 / Hancom11.0.0.9136 / hwp-managed-direct-dll-host,
input_preprocess=none,2026-09-28. HWP job
`22824a7d-4f57-4796-a6cb-61700d31cd4f`, PDF job
`1eec963c-e00d-46b0-9fb2-f5ebe1dc5529`, variant PDF job
`d2412b97-84ef-43e9-8192-e4a5b5693782`.
생성 코드/상태 원장: `output/7353/r19/shared-color/{extract,variant}.rs`,
`*-job.json`, `*-status.json`, `*-download.json`.

## 관측값

`mutool draw -F trace`로 확인한 shared PDF의 공통 세로 경계는
x368.562pt, top-down y157.628~185.437pt부터 반복된다. 동일 중심선에
검정0.36pt를 먼저, 회색0.24pt를 나중에 stroke한다. 즉 큰 폭/특정 색의
한 선으로 선택하는 규칙이 아니다. 두 선의 선언 폭0.12/0.10mm는 기존
600dpi pen 격자 규칙으로 위 값이 된다.

variant PDF의 같은 세로 경계는 회색0.24pt 다음 검정0.36pt다. 가로 경계
y185.437pt도 위 셀의 검정0.36pt 다음 아래 셀의 회색0.24pt로 출력된다.
동일 시작 행/열에서는 왼쪽/위 셀 → 오른쪽/아래 셀 순서다. 실제 소유 순서는
원본 셀 시작(row,col)이며, 입력 벡터 순서·검정 우선·굵은 선 우선으로
일반화하지 않는다. Cell colors와 zone 우선순위/복합 선종은 별개다.

추가 `span.hwpx`는 정상 저장본의 (row4,col5)를 rowspan2로 늘리고
(row5,col5)를 제거해 만든 합성 병합 대조군이다. 위 셀 height에 아래 셀 height를
합했고 아래 셀의 문단은 이 대조군에서 제거했다. 원본 보존 자료와 혼용하지 않는다.
직접 한컴 PDF `span-2020.pdf`(job `3b6cb461-b210-4a92-99f2-e2fbee2373d6`)는
x368.562pt의 첫 구간(y157.628~185.437pt)에서 검정→회색, 다음 구간
(y185.437~213.247pt)에서는 회색→검정을 출력한다. 오른쪽 병합 셀의 시작 행이
그 다음 왼쪽 셀보다 빠르기 때문이다. 이 반례는 무조건 왼쪽 선을 먼저 그리는
규칙과 source(row,col) 순서를 구별하며, 새 정식 계약에 포함했다.

정식 계약은 최종 Line 좌표·굵기·색·paint 순서, 셀 벡터 역순,
본문/표 외곽/후속 표 기하 보존을 검사한다. 분할 합성 계약은 별도
`issue_7353_table_v2_split_borders.rs`에서 내용 보존과 각 조각의 실제
끝점/두 펜을 확인한다. 가로선 교차점 cap 길이·글꼴 외형 완전 일치나
원본 전체17쪽의 한컴 시각 일치를 이 계약으로 주장하지 않는다.

## SHA-256

| 파일 | SHA-256 |
| --- | --- |
| input.hwpx | 0071379b0df95ac071b29cd24fd692bb0357174194875724776c63e25f6b6f02 |
| saved.hwp | 87e9c0804bbedbe035766b925cba28322cf1a238f4b8fc60a48c6cf4c9ee1f83 |
| shared-2020.pdf | 0480b42473ade7ec2dfdc19bb12191bb5326215ac65eb233ea889fa599003f4c |
| variant.hwpx | 694d675b666741ef85a67f4934d1546f5ed8298c377d8190c9cfb720e76404fd |
| variant-2020.pdf | b9f4705f2e4508ca3d3b4aa47644269380cf0030f66b7edbf36ecf16308aeac2 |
| span.hwpx | 3e979d85587741acb3128e5de7bce61dba728e1eab32b8a6d329c3af31623a87 |
| span-2020.pdf | a82dfc0f8120a04d465a2acbe35c3cb9f15ac40af712426aebd330b4c3607fb4 |
