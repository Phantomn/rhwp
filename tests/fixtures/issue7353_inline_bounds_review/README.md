# 실제 자식 표 높이 예약과 Dot 셀 테두리

2026-09-28 생성한 정상 한컴 저장본과 같은 HWP의 독립 PDF다.
원본 전체 문서의 통과 증거가 아니라 문단30 표의 분리 대조군이다.

## 생성과 출처

- `table-input.hwpx`: `samples/issue4090/156492236_규제샌드박스_min.hwpx`에서
  section0/paragraph30만 남겼다. 부모 2행3열, 내부 표3개, 스타일·내용·용지와
  테두리 속성을 유지했다. 원본 파일은 변경하지 않았다.
- `dot-input.hwpx`: `issue7353/cell-dash/pen-catalog-saved.hwp`의16개 표준 펜
  폭과 표를 유지하고 Dash 테두리만 Dot으로 바꿨다. 실제 원본 문서가 아니라
  펜 속성의 독립 출력 관측용 카탈로그다.
- HWPX를 한컴으로 HWP 저장한 뒤 그 HWP를 다시 PDF 인쇄했다.
  engine2020, Hancom11.0.0.9136, `hwp-managed-direct-dll-host`, 전처리 없음,
  `hancom2020_pdf_driver_one_up`이다. 줄 메트릭을 수동으로 조작하지 않았다.

| 대상 | HWP 저장 job | 해당 HWP의 PDF job |
| --- | --- | --- |
| table | 839a80d8-4d81-4319-921f-19522e4aebeb | bd6af2d5-6b11-48ec-bc39-68bc7891d326 |
| dot | 4dd30620-27f3-4d54-b470-42c559cbf873 | 6db727a6-8cec-4265-9ad0-e7f0443db606 |

생성 코드는 `output/7353/r19/inline-bounds/{extract,dot-catalog}.rs`, MCP 응답은
같은 폴더의 `*-job.json`, `*-download.json`이다.

## 독립 기대값과 검증 범위

table의 투자유치·매출증가·고용증가 표는 각각 합계까지 온전히 배치되어야 한다.
각 합계 `4조 8,837억원`, `1,561억원`, `6,355명` 및 하단 벤처투자 주석을
중복·누락 없이 보존한다. 부모 Dot 외곽, 배경, 자식 표3개의 행과 외곽을 PDF와
직접 비교한다. 폰트 대체에 따른 글립 외형 차이는 남는다.

원본의 왼쪽 자식 표 측정155.40000000000003px와 기존 예약155.40000000000001px는
같은 HU 기하를 계산한 부동소수점 표현 차이다. bind에서 유효성을 확인한 자식의
실제 끝점을 예약해야 한다. 임의 허용 오차를 늘리거나 셀 경계를 감추지 않는다.
합성 경계 계약은0.1+0.2와0.3의 차이를 이용한다. 실제 끝점보다1ULP 작은 예산은
거부하고 정확한 예산은 수용하며, 다음 문단은 다음 조각에 정확히 한 번 남는다.
이는 한컴 픽셀 관측이 아닌 산술·소유권 불변식 테스트다.

Dot PDF의 실제 stroke_path를 `mutool draw -F trace`로 추출했다.
`read-pens.mjs`와 `dot-pens.json`은 가로/세로 각각의 반복 획·공백을 기록한다.
아래는 두 축에서 관측한600dpi 프린터 단위 `(획, 공백)`이며 폭 인덱스0..15순이다.
표준 폭별 펜 정의이지 문서 ID별 보정값이 아니다.

```text
(3,4) (4,6) (5,7) (7,10) (9,13) (10,15) (14,21) (17,25)
(21,31) (24,36) (35,52) (52,78) (69,103) (104,156) (139,208) (173,259)
```

정식 계약은 `issue_7353_inline_bound_envelope.rs`, `issue_7353_cell_dot.rs`에 있다.
펜16종은 선택 표 경로에서96/192dpi 최종 Line 노드의 획 길이·주기를 검사한다.
실제 문단30 표는 DocumentV2의 부모/자식 최종 상자와 내용을 검사한다.
`issue_7353_table_v2_borders.rs`의 Dot 분할 계약은 제목 반복과 중첩/일반 표의
모든 조각에서 내용·상자 보존,0.64px 이하 획, 동일 획 중복 부재를 검사한다.
카탈로그 전체 DocumentV2는 기존 `stored text requires intact single-segment rows`
거부가 남는다. 선택 표 펜 검증을 카탈로그 본문 조판의 통과로 보고하지 않는다.
실제 표 한 쪽에서 Native/fresh WASM 직접 시각 비교하며 카탈로그16종 전체의
WASM 시각 일치까지 주장하지 않는다. 분리본 통과는 원본 전체 통과가 아니다.

```text
abfa5e9b5825bb5a38a268395a69acdaaf0dca787077f2407ff5ecdb6d637c39  dot-2020.pdf
742586339e6b4adef9f9f1f7aeed7abace26ea95a60deec8b7b4d93b445a32bf  dot-input.hwpx
0d4597c1816a1d850b2b0baf55dc95869a2e14239fd6af60edda801043ba1cc7  dot-saved.hwp
7c4dc151137ee34bf324ac86691f171471284e92987442bca30f7d2b560b5d33  table-2020.pdf
0933ad7a9118e18590f8f817e4feff957e2f45da5f26cfbc3c2f2989d053ad91  table-input.hwpx
196995946ed50e11ea932c2ba993dde3b00d85f6b3ca6adce1e57a2f29e161dc  table-saved.hwp
```
