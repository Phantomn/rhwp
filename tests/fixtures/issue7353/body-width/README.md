# #7353 본문보다 넓은 절대 너비 자리차지 표

## 생성과 독립 기준

`samples/86712_regulatory_analysis.hwp` p13 표의 첫 네 행을 분리했다.
원본의17열 그리드·셀 내용·너비49204HU·좌우 바깥여백141HU·문단 기준
자리차지(TopAndBottom)·일반 본문 폭48190HU를 유지했다. 셀 내용과 첫 열의
4행 병합도 유지한다. 제목 뒤 표, 그 뒤 `AFTER TABLE / ORIGINAL BODY WIDTH`
문단을 놓고 저장 LineSeg를 제거한 후 한컴에서 정상 재조판·저장했다.
원본31행 전체의 분할·이중선 처리를 입증하는 샘플은 아니다.

생성기: `output/7353/r19/body-width/create.rs`.

- HWPX → HWP: MCP2020 job `59ac952c-bd03-4a07-979f-19eb9cc3f4c1`
- 그 HWP → PDF: MCP2020 job `3cf35d8f-7e36-4604-bb0e-d5e3249c763f`
- 둘 다 succeeded, Hancom11.0.0.9136, PDF1쪽. 영수증은 같은 output 경로에 보존.

원본 첫14문단의 한컴 정상 저장본과 PDF도 표가 본문 오른쪽을 넘고 용지 안에
남는 배치를 보여준다(`page-break/break-saved.hwp`, `cell-break/break-2020.pdf`2쪽).
이 분리본은 그 가로 배치를 검토하면서 원본의 별도 미지원 이중선 접합을 분리한다.

## 계약과 관측

PDF trace의 표 path: 왼쪽58.049pt, 오른쪽549.784pt,
위758.890/아래625.955pt(y 변환841−y). 뒤 문단 글자 기준선은
1912×0.119869pt. 96dpi 계약은4/3배, 정상 PDF 좌표 정밀도를 고려한0.6px
이내로 최종 Table bbox와 뒤 문단 기준선을 검사한다. 입력 자체의 절대 너비와
원점(좌여백5669HU+표 바깥여백141HU)도 별도 검사한다.

`tests/cases/issue_7353_body_exclusion.rs`의 추가 계약:

- 정상 저장본의 표 너비·외곽, 원래 본문 너비를 가진 뒤 문단.
- 본문 폭은 유지하고 용지 오른쪽 여백만 변경: 정확한 맞춤/1HU 및141HU 부족.
  뒤 두 경우의 거부는 이번 V2 지원 경계이며 일반 편집기의 절대 규칙이라는 뜻이 아니다.
- 셀 내부에서는 별도 가로 lane을 주더라도 부모 content box를 넘지 못한다.
- 이어받기 변형: 첫 열의4행 병합을 행별 셀로 분리하고 원래 글자는 첫 셀에만
  남긴다. 추가 셀은 유효한 빈 문단을 가지며, 첫 열의 줄은 재조판한다.
  본문 높이8000/14000HU에서 RowBreak 조각의 너비·원점·본문 하단·셀별 내용
  무누락/무중복·host 및 뒤 문단의 단일 소비를 검사한다. 한컴 일치 증거가 아니다.

원래4행 병합은 RowBreak에서 통째로 이동한다. CellBreak로만 바꾼 초기 테스트는
기존 `rowspan cell-internal cuts` 미지원에 걸렸으므로 위 비병합 변형과 구분한다.
빈 셀의 문단/글자모양이 빠진 초기 변형 오류도 제품 결함 증거로 세지 않는다.

수정 전 Native 바이너리는 정상 저장본에서 `stored body anchor outside body`로
거부(`before.log`, 보존 바이너리 `cell-break/probe`와 그 `source.sha256`).
수정 후 같은 입력은1쪽 생성한다. 원본 첫14문단은 다음 제한
`V2 mixed double-border junction`에 걸린다(`full.log`). 일반 어울림·TAC·셀 overflow·
용지 밖 개체·nonzero offset 본문 anchor까지 지원했다고 보고하지 않는다.

## SHA-256

- wide-input.hwpx: `1be11109cca2c7c99f7a529b52d7a67f493dcc6ec0db3d9a5bb3950c13389e1c`
- wide-saved.hwp: `48c1748e7161ef568111fa7c8aef689d0665485bd7141185ce809645739206dd`
- wide-2020.pdf: `e69a398cc01c57055e0ec79ef7fac103a6dc9a7ad0e4fb9486b551e1c2f995c6`
