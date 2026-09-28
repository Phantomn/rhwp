# #7353 저장 SQUEEZE 셀의 한 줄 재현

## 입력과 기준 출력

기존 정상 `issue7353_tac_noop_review/noop-saved.hwp`에서 `create.rs`로 자식 표 너비만
10000HU로 줄이고 셀의 `line_wrap=1(SQUEEZE)`, 문구를
`ONE TWO THREE FOUR FIVE`로 지정했다. 변경된 문단/host의 LineSeg는 지우고
`squeeze-input.hwpx`를 만들었다. 부모·앞뒤 문단·글꼴·용지 정보는 보존했다.
한컴에서 정상 저장한 `squeeze-saved.hwp`를 수정 없이 V2와 PDF 변환에 사용한다.
이것은 #6601 원본을 대체한 파일이 아니라 긴 줄의 압축을 검증하는 독립 대조군이다.

- HWP MCP job: `139644b8-4ce5-475d-b5eb-6f9dd29bd6fd`
- PDF MCP job: `d988a3f0-1e92-4e01-8312-e42d20ee0b38`
- engine2020, Hancom11.0.0.9136, managed-direct-dll-host, input_preprocess none
- font_scope session0 verified(mapped/registered2, failed0), PDF1쪽

| 파일 | SHA256 |
| --- | --- |
| squeeze-input.hwpx | b7fa2919634f96156077a59b11644a981a61122e6a8681ce48774e43cb4c8ca9 |
| squeeze-saved.hwp | 6f18c6a7b6e1e119dc08fa4e5ccab0a6cf3244e3feb8bde810f81fde6df53c8d |
| squeeze-2020.pdf | d29f1393c2ad9da0a89793042e83235838878ff28ac2268badf4055e46988b2e |

## 독립 관측과 검사

저장 셀 폭10000HU, 좌우 안여백283HU, 저장 줄 폭9432HU(정수 양자화), 줄1개.
부모 y8787, 안여백283, 앞 문단 advance1760 → 자식 표 y10830HU.
자식 높이5000HU, 내용 y11113HU. CELL AFTER y16490HU, 뒤 본문 y27354HU.

한컴 PDF trace의 자식 clip은 x149.574..249.410pt, y108.182..158.195pt.
문구의 모든 glyph는 같은 baseline이며 첫 glyph의 x는152.3306pt,
baseline은120.4147pt다. 셀 안쪽에 한 줄을 유지하며 여백을 확장하지 않는다.
PNG에서 강하게 압축한 glyph의 형태·겹침은 완전 일치하지 않는다. 인쇄 축척·대체 글꼴 및
backend의 자간 표현을 구분해 조사하지 않았으므로 글꼴만의 차이로 단정하지 않는다.
이 차이를 표 기하/한 줄 유지/내용 보존 계약과 구분한다.

정식 `issue_7353_table_v2_document_flow` 검사는 저장 HWP 및 HWPX 직렬화·재파싱에 대해
실제 표 크기·내용 한 줄·안여백·음수 자간·후속 문단을 확인한다. in-memory dirty,
LineSeg 제거, KEEP/알 수 없는 값, 세로쓰기는 거부한다. dirty flag는 파일 속성이 아니므로
직렬화로 재현하지 않고 실제 Document IR 경로로 검사한다.

이번 지원은 유효한 저장 plain-text SQUEEZE 줄이다. 편집 후 fresh SQUEEZE 재조판,
KEEP, 셀 내부 개체 혼합, 임의 글꼴에서의 개별 glyph 위치 일치는 입증하지 않는다.
