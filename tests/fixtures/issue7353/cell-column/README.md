# #7353 셀 첫 문단의 일반 1단 정의

## 입력과 독립 기준

원본 `samples/86712_regulatory_analysis.hwp`의 본문 p13 표에서 r6c2와 r30c1을
분리했다. 각각 자식 TAC 표를 포함한 셀과 `15.규제정비 / 계획` 텍스트 셀이다.
두 셀의 첫 문단에는 `MultiColumn`과 일반 1단·동일 너비·단 간격0의 `ColumnDef`가
있다. 원본과 원본 첫14문단의 한컴 재저장본에서 동일하게 관측했다.
첫14문단 기준 PDF2쪽에는 r6c2 자식 표가 앞뒤 행 사이에, 3쪽에는 r30c1 문단이
셀 안에 놓인다. 이 정의를 무조건 다음 쪽 이동으로 해석할 근거가 아니다.

`output/7353/r19/cell-break/create.rs`는 두 셀을 각각 같은 너비/높이/여백의
1x1 부모 표에 넣고 영어 설명 제목을 추가한다. 셀 내용·자식 표·글자모양·단 정의는
보존하며, 저장 LineSeg는 제거하고 한컴에서 정상 재조판/저장한다. 부모 표의 구역 내
위치는 문단 기준0, 왼쪽 정렬로 설정했다. 원본 전체 조판 또는 원본 표의 rowspan/
페이지분할 증거가 아니라 초기 단 정의의 독립 대조군이다.

- HWPX→HWP: MCP2020 job `66cc7cbc-6698-46f6-bc2c-7a5ba7712f09`
- 그 HWP→PDF: MCP2020 job `79347e8c-8520-4970-a144-50839923e079`
- 둘 다 succeeded, Hancom11.0.0.9136, PDF1쪽. 영수증은 위 output 경로의
  `save-v2-*`/`pdf-v2-*` JSON에 보존했다.
- 초기 생성본 `cells-input.hwpx`는 복제한 제목의 raw Page 비트를 남겨 첫 문단이
  Page로 저장되는 오류가 있었다. 실패본과 로그를 보존하고, v2 생성에서 제목의
  `raw_break_type`만 함께 초기화했다. 이 오류는 제품 결함 재현으로 세지 않는다.

## 독립 좌표 계약

`mutool draw -F trace`의 PDF path 좌표는 pt이며 y 변환은 `841-y`다.
96dpi에서는 각 좌표를4/3배 한다. 순서대로 부모/자식/두 번째 부모 표의
왼쪽·위·오른쪽·아래 좌표는 다음과 같다.

- `(58.049,758.89,408.62,699.914)`
- `(63.086,753.735,399.505,705.069)`
- `(58.049,673.064,151.239,626.315)`

뒤 제목·규제정비·계획의 기준선은 trace의 각각 y1295/1547/1720에
0.119869pt 변환을 적용한다. `initial_cell_column_saved_text_and_nested_table_match_independent_pdf_origins`
테스트는 실제 최종 Table bbox와 TextLine 기준선을0.5px 이내로 대조한다.
자식 표 두 행의 문자열이 한 번씩 표시되는지도 검사한다.

다른 `initial_cell_*` 계약은 명시적 정의와 암묵적 단일 셀 영역의 최종 좌표가
같은지 확인한다. 저장 재사용과 셀 내용 재조판을 별도로 실행하며, 첫 빈 문단도
다음 줄을 전진시켜야 한다. 다단·중간 선언·Page/Column/Section 등은 거부한다.
이 합성/변형 계약을 한컴 시각 일치 증거로 승격하지 않는다.

## 한계와 수정 전후

- 이전 Native 바이너리: 같은 정상 저장본에서 본문 p1의
  `explicit paragraph page/column break`로 거부(`before-v2.log`).
- 변경 후: 같은 입력에서1쪽 생성. Native/fresh WASM 비교는 `review/`에 보존.
- 원본 첫14문단은 이후 `stored body anchor outside body` 제한에서 멈춘다.
- 한컴 저장 전 HWPX는 `nested anchor, TAC, wrap or outer margin`에서 거부된다.
  정상 저장 HWP의 통과를 이 HWPX/원본 전체 통과로 보고하지 않는다.
- 폰트 외형·굵기 차이는 남는다. 실제 다단 조판과 셀 중간 단 전환은 지원하지 않는다.

## SHA-256

- cells-v2-input.hwpx: `f901380b979c508d5eddab174e985c7582ca5d6e6ada98b9af3fc178eea56457`
- cells-v2-saved.hwp: `e5bc4998080e267693fd601819bb61f94ac6e915500ff1af0f9724f13ab7ca9a`
- cells-v2-2020.pdf: `398ba532b03ca236bdce6572f0b31d6201a3d4dd8915350651ab2a6bc33c9a1d`

## 후속 문단의 단 기준 표: cell-column-saved.hwp (2쪽)

`output/7353/r19/next-172/generate-cell.rs`가 같은 원문 p172의 cell80 안에 있는
1x1 표 전체(25문단, 내부3x12 및5x4 표)를 분리한다. 본문 첫 위치에는 원문 p173의
빈 문단을 두고, 셀에서 본문으로 올린 외부 carrier의 ColumnDef/MultiColumn만
제거한다. 본문 단은 새 첫 문단이 소유한다. **내부** 첫 문단의 단 정의와 모든
문단/표 속성·텍스트는 보존한다. 저장 좌표를 수동 작성하지 않고 한컴에서 재저장한다.

초기 `cell-story`/`cell-anchor` 시도는 본문 첫 carrier의 구조 컨트롤 및 두 번째
본문 단 구역 때문에 현 V2 Document 진입이 거부했다. 원본/로그는 output에 남긴다.
최종 대조군은 이 분리 절차를 명시한 정상 생성본이며 원본 전체 통과의 증거가 아니다.

- HWPX→HWP job `ee5e436a-1963-476c-80e4-ceb2215f6013`.
- 그 HWP→PDF job `f4053821-1035-4bf4-a254-4a89eca0cac8`.
- Hancom2020 11.0.0.9136, input_preprocess none, PDF2쪽.
- 입력 HWPX SHA256 `d8568552002dfba2f2daa540fb6e3b4c8119e91db1e71946f9a38b9f1c4b5f66`.
- HWP SHA256 `87d424a0ac009a0f3ddd8471477381da7ae944a23d31f57d5ae0d2e50c827642`.
- PDF SHA256 `03432af97a8516ec9361f7aefc36a8bb04ddbdbdf7240869152df3c9b7557a52`.

독립 PDF trace에서1쪽 내부표 위/아래는 y189.752/87.744pt,
2쪽은761.047/606.536pt이다. 왼쪽 수직선은x59.488pt.
y는841에서 빼고 dpi/72를 곱한다. 인쇄 배율 .119869/.12의 잔차와0.2px 외에는
허용값을 늘리지 않는다. `issue_7353_cell_column_insets`는96/144dpi의 실제 최종
Table/host 좌표·모든 내용1회 보존·뒤 문단·종료를 검사한다.

단 기준 표 x는 셀 원점+개체 왼쪽 바깥여백283HU다. 글줄 원점은 문단 왼쪽50HU를
사용한다. 문단 위 기준 표는 문단 앞 간격100HU **전** 원점을 사용하고 host 글줄은
그 간격 **후** 원점이다. 따라서 표 위와 host 위 차이는283-100HU.
앞 간격을 표 원점에도 더한 중간 구현은2쪽 PDF 좌표와 어긋났으며 수정했다.

수정 전 정적 Native probe는 같은 정상 HWP에서 `stored excluded cell anchor`로
실패(`column-before.log`). 중간 앵커 문맥만 연결한 라이브러리에서는 새 정식3검사 중
2FAIL/1PASS(`test-initial-anchor.log`), 최종은3PASS다. 중간 라이브러리를 작업 시작 전
상태로 부르지 않는다. 단 정의 부재/다단/저장 host x 불일치 반례는 계속 거부한다.
합성 경계 검사는 host 점유 끝32px가 아닌20px만 들어가는31px 예산의 실패,
32px 수용 후3px간격 이어받기와 다음10px줄의 보존을 별도로 확인한다.
