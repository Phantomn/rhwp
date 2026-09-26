# #7353 장식 중첩·셀 테두리 우선순위 대조군

원본 #6923 전체의 통과 자료가 아닌 규칙별 독립 한컴 출력이다.

## 생성과 출처

`create.rs`를 저장소 루트에서 실행한 HWPX를 한컴 MCP engine2020으로 정상 저장한
`*-saved.hwp`, 그 **저장 HWP**를 PDF로 변환한 `*-2020.pdf`를 짝으로 보존한다.
Hancom11.0.0.9136/direct32bit, 전처리none이다. 모든 저장 LineSeg를 지운 뒤 한컴에서
새로 계산했으며, 다운로드한 HWP의 줄 메트릭·좌표를 수동 편집하지 않았다.
명령은 `mydocs/manual/mcp_hwp2024Convert_usage.md`의 start/status/download 절차다.

| 이름 | HWP job | PDF job |
|---|---|---|
| outline-clean | cddf2c1d-9ca9-40fb-99e7-656c74364015 | 2b23b114-97de-427a-8c4e-151ecd594d4b |
| zones | ad03c61f-893f-41d3-8911-40af2b5c6522 | 5b7ab48b-d16d-481e-a659-4ea5ebb92e72 |

- outline-clean: 기존 `../issue7353_double_review/grid-7-saved.hwp`의 2×2 자식 표에
  표 전체 빨강 1.5mm 실선, 셀별 검정 0.12mm 실선을 설정한다. 좌상단 셀의 위쪽과
  우하단 셀의 아래쪽만 None이다. 부모/문단/페이지 스타일은 변경하지 않는다.
  PDF1쪽에서 **빨강 선은 없고 두 구간은 열린 채**로 보인다. 자식30000×8000HU,
  각 셀15000×4000HU, 부모와 AFTER CELL을 보존한다.
- zones: 기존 `../issue7353_zone_review/zone-lines-saved.hwp`의 파랑/빨강 구역에
  동일 장식의 안쪽 구역(0-based행3..6, 열0..1)을 추가한다. PDF1쪽 행4의 위와
  행7의 아래가 빨강이며, 바깥 빨강 테두리/파랑 배경/노랑 셀도 유지된다.
  행1..19/20..24의 두 조각과 마지막 AFTER ZONE TABLE 위치는 이전 기준과 같다.
- 제목의 실제 혼합 이중선/실선/None 사례는 기존
  `../issue7353_double_review/title-saved.hwp`와 대응 PDF를 재사용한다.
  가운데 공백 셀의 위/아래에 표 전체 테두리를 덧그리지 않아야 한다.

## 실패 대조군과 한계

첫 `outline-input.hwpx` 생성 시 모든 borderFill을 변경해 문단·페이지에까지 불필요한
테두리가 생겼다. 이 입력·HWP·PDF는 `output/7353/r19/decoration-priority/outline-*`에
진단용으로 보존했으며 시각 판정 근거로 사용하지 않는다. 위 outline-clean은 자식 셀
fill만 복제하도록 수정한 별도 입력이다. 원본 #6923이나 실패 저장본을 덮어쓰지 않았다.

같은 불투명 단색/동일 선 장식의 합성만 검증한다. 서로 다른 구역 효과의 우선순위,
중첩 대각선/그라데이션/이중선 구역은 미지원이다. 셀 borderFill=0(참조 없음)을
명시적 all-None과 동일시하지 않으며, 전자는 table outline fallback 미검증 상태다.
페이지 분할에서 셀 테두리 적용은 합성 계약으로 별도 검사하며, 위 intact 표 PDF를
모든 분할 유형의 한컴 정답으로 확장하지 않는다. 글꼴 외형과 raster antialias 차이는
테두리 소유·위치·배경·후속 내용의 검증과 구분한다.
