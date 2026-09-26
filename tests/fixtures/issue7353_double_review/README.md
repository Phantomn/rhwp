# #7353 표 이중선 독립 대조군

이 자료는 원본 #6923 전체의 통과 자료가 아니다. 한컴 정상 저장·PDF 출력으로
동일 굵기 이중선의 모서리/공유 경계/교차점을 확인하는 별도 규칙 대조군이다.

## 입력과 생성

- `create.rs`: 원본 `../issue6923/148738070_wrapper_table_stored_page_frame.hwp`의
  s0/p5/t0/c0/p7 제목 문단과 `../issue7353_tac_space_review/space-input.hwpx`를 사용한다.
- `title-input.hwpx`: 제목 자식 내용을 유지하고 1×1 회색 부모 표에 넣는다.
- `grid-{0,3,7,11}-input.hwpx`: 같은 스타일의 2×2 자식 표를 새로 구성한다.
  자식 30000×8000 HWPUNIT, 셀 15000×4000, 부모 높이16000이며, 글자는
  `W{굵기ID} R{행} C{열}`이다. 모든 저장 LineSeg를 지우고 한컴에서 다시 저장했다.
- generator의 `common.margin=Default`는 table 전용 바깥여백을 변경하지 않는다.
  실제 저장본의 바깥여백은 각141 HWPUNIT이다. 저장본을 너비 검사에 맞춰 수정하지 않았다.
- `*-saved.hwp`: 각 입력 HWPX를 한컴 MCP `engine2020`, 전처리none으로 변환했다.
- `*-2020.pdf`: 반드시 대응 saved HWP에서 같은 엔진으로 변환했다.
  관측 backend는 Hancom11.0.0.9136/direct32bit이다.

재생성은 저장소 루트에서 현재 라이브러리에 연결해 `create.rs`를 실행하고,
`mydocs/manual/mcp_hwp2024Convert_usage.md`의 비동기 start/status/download 절차로
HWPX→HWP→PDF 순서를 따른다. generator 산출 경로는 `output/7353/r19/double/`다.
자격증명은 저장소 밖 환경 파일에 있으며 fixture에 포함하지 않는다.

| 자료 | HWP job | PDF job |
|---|---|---|
| title | fcaae4ac-7466-4914-9857-a28d0f7ca043 | 511091c0-e353-4491-acd1-f6f27a047958 |
| grid-0 | fff7e920-ab6f-4128-b76d-1c7f84352082 | f799ba19-0e60-4a33-b1b2-450af55f0834 |
| grid-3 | d49a3045-78be-41cf-aece-bd20ed251782 | 3683c6c3-e93d-4500-8a1b-866629aca5c5 |
| grid-7 | e3f2622f-f72a-4599-a258-06755209cd40 | 96c561a9-c7ce-49f8-bdc0-cd7d87ff5f2e |
| grid-11 | 554d9324-6247-44ea-b13a-1fdf563eb88c | eda631d9-6bd0-4029-bd58-deb26c949ddc |

## 독립 관측과 제한

PDF vector의 굵기ID0/3은 pen0.12pt, 중심 간격약0.36pt,
ID7은0.36pt/1.08pt, ID11은약1.079pt/3.238pt다.
선:빈간격:선=1:2:1, 600dpi pen 격자와 일치한다. T/교차점의 안쪽 선은
빈 간격을 가로지르지 않는다. PDF의 약1픽셀 단위 끝점 반올림과 폰트 외형은
V2의 연속좌표/대체 글꼴과 분리해 판단한다.

이중선 절편 당시 `title-saved.hwp`는 `V2 table/cell outline disagreement`,
원본 #6923은 `V2 overlapping zone decorations`로 미지원이었다.
후속 [장식 우선순위 대조군](../issue7353_decoration_priority_review/README.md)에서
입력 변경 없이 제목 출력과 동일 장식 구역 중첩을 구현했다. 원본 전체는 이후의
저장 줄 구성 미지원으로 아직 완료하지 않았다. 제목 표의 선이나 원본 입력을 바꿔
수용 조건에 맞추지 않는다.
다른 굵기·색·선 종류가 만나는 접점, Double zone perimeter는 별도 검증 전까지
명시적으로 미지원이며, 이 네 대조군의 통과를 해당 경로의 통과로 확장하지 않는다.
