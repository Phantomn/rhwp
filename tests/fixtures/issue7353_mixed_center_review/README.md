# 혼합 글자 크기와 문단 세로 가운데 정렬

2026-09-28 생성한 정상 한컴 저장본과 같은 HWP의 독립 PDF 출력이다.
원본 #7158 전체 문서의 통과 증거가 아니라 해당 문단/표의 대조군이다.

## 입력과 생성

- `mixed-input.hwpx`: `issue7353_body_frame_review/portrait-saved.hwp`의 용지와
  글꼴을 재사용하고 본문을 `ABC xyz` 두 문단으로 교체했다. ABC/공백24pt,
  xyz12pt, 첫 문단은 attr1의 세로정렬 bits20..21=2(CENTER), 다음 문단은
  BASELINE이다. Fixed 줄간격3600URC이며 수동 LineSeg를 넣지 않았다.
- `table-input.hwpx`: `samples/issue4090/156492236_규제샌드박스_min.hwpx`의
  section0/paragraph21만 남긴 분리본이다. 표의41개 셀, 글자/문단 스타일,
  테두리와 내용을 보존하고 한컴에서 다시 저장했다. 원본 자체는 변경하지 않았다.
- 각 `*-saved.hwp`를 한컴으로 인쇄한 것이 대응 `*-2020.pdf`다.
  engine2020, Hancom11.0.0.9136, `hwp-managed-direct-dll-host`, 전처리 없음,
  `hancom2020_pdf_driver_one_up`으로 각각1쪽을 생성했다.

| 입력 | HWP job | 해당 HWP의 PDF job |
| --- | --- | --- |
| mixed | 4fc93dfd-d9fe-471e-990a-d7ff144a1713 | 8fc199ba-607d-4fb6-8774-9e9b9687805d |
| table | b557f535-3ac3-4f5f-8228-b75e280599f1 | 0dad0105-44b4-47d0-a0e5-88d3b482338e |

생성 코드와 MCP 응답은 `output/7353/r19/mixed-center/{create,extract}.rs` 및
같은 폴더의 `*-job.json`, `*-download.json`에 있다.

## 독립 기대값

mixed 정상 저장 줄의 text_height=2400HU, CENTER reference=1200HU다.
각 글자는 자신의 em 중심을 줄의 중심에 맞춘다. 기존 공통 nominal glyph
baseline을 사용하면96dpi에서 큰 글자 기준선27.2px, 작은 글자21.6px다.
PDF에서 CENTER의 큰/작은 기준선 y=35.41416/31.21248pt이며 차이는4.20168pt
(장치 반올림을 포함해 약5.60px)다. BASELINE 대조 문단에서는 둘 다53.42136pt다.
줄 원점은20/44px, 줄 상자는32px이고 겹치는 줄간격을 임의로 늘리지 않는다.

table의 `135 (21%)`는12pt/10pt다. 저장 줄 높이1200HU, 중심600HU에
각 글자 em을 정렬하므로 기준선은1020/950HU다. 모든41개 셀·42개 저장 줄
(빈 셀 포함), 표 외곽, 주변 행이 보존되어야 한다.
이1020/950은 rhwp 공통 nominal baseline의 기대값이지 한컴 글꼴의 정확한 잉크
기준선 수치를 재인용한 값이 아니다. 표 PDF의 `135`/`(21%)` 기준선은
120.22861/119.629268pt(차이 약0.799px)이며 rhwp nominal 차이0.933px와 약0.134px
차이가 남는다. 서로 다른 굵기/글꼴의 세부 baseline 메트릭 일치까지 주장하지 않는다.

정식 계약은 `tests/cases/issue_7353_table_v2_document_flow.rs`와
`issue_7353_table_v2_text.rs`에 있다. fresh/stored 셀의11/12px 수용 경계,
크기 순서 반전, 다음 조각의 AFTER 보존도 검사한다. super/subscript CENTER,
개체 혼합 CENTER, 전체 원본의 다른 오류는 이번 지원 범위가 아니다.
대체 글꼴의 폭·굵기 차이는 남는다.

```text
b29fb50d20d7a92b524bc16349d3932aa7a18e3073d3d2c169fe5fe5f4801a53  mixed-2020.pdf
d38eca83186583d0ded39b9f666278a8cb00cbb8411e52f31ac2555a0f07fec5  mixed-input.hwpx
11b761b16dfd76e4f601223ebaee258f1df1d999844874f557f3c8154175f452  mixed-saved.hwp
ffcb5a9c0d837ad8e737a72196028a8102c866f7f31f48c53647e6cf8db5cb70  table-2020.pdf
e09312829edab17edddabe85dcab8918804ec7c2ed17f516ad87aff57adb9d62  table-input.hwpx
74ba4adb10ce12fa266d10e7b262254bdb22c7704c7a5e20479d0f3cb9ebb1ac  table-saved.hwp
```
