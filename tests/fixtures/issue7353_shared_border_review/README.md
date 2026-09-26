# #7353 공유 실선 굵기 — 원래 테두리 보존 표

- 원본: `tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp`.
- 원본 physical PDF p6의 시장점유율 표 및 앞뒤 문단(0-based21~25)을 추출했다.
- `table-input.hwp`: 아래 생성 코드로 문단만 분리한 입력이다. 표/셀/BorderFill/문자·문단
  속성/저장 LineSeg를 삭제·통일하지 않았다. 문서 전체의 페이지 흐름 증거는 아니다.
- `table-saved.hwp`: 이 입력을 한컴에서 정상 재저장한 파일이다.
- `table-2020.pdf`: **위 정상 저장 HWP**에서 한컴이 생성한1쪽 PDF다.

```rust
let mut d = rhwp::parse_document(&std::fs::read(
    "tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp"
).unwrap()).unwrap();
d.sections[0].paragraphs = d.sections[0].paragraphs[21..26].to_vec();
let input = rhwp::serializer::cfb_writer::serialize_hwp(&d).unwrap();
```

생성 환경: 2026-09-26 MCP engine2020, Hancom11.0.0.9136, direct32bit,
input_preprocess=none. 이전 CENTER 조사 때 생성된 동일 입력/출력을 재사용한다.
HWP job `38815ab8-bcef-4a0a-b8a4-d22acce92d04`,
PDF job `2dfd02c5-fefe-4f84-9617-7a1e768bb0bf`.
원장: `output/7353/r19/stored-paint/table-{status,pdf-status}.json`.

| 파일 | SHA-256 |
| --- | --- |
| table-input.hwp | cc18a209516e74606c3d5ea0b9fcc25236ed0190b5db2b7308a514dd75f36ccc |
| table-saved.hwp | d88085622c3b3215f37bf10ad64ed8dd6b71a45b2790a58c552ad5d556448070 |
| table-2020.pdf | a9cfc5d90a400c5d4cd335d5581f479f8b0c620d730814f685c163a1480d886b |

독립 기준은 PDF의 공유 경계 stroke다. `mutool draw -F trace table-2020.pdf`에서
제목/본문 경계 y=671.386pt(bottom-up)는 순위 열에0.24pt 실선을, 제조사 열부터
나머지7열에는 같은 중심선의0.24/0.48pt 실선을 갖는다. HWP width0/2의
600dpi 격자 투영이다. 같은 색 불투명 실선의 합집합은 큰 굵기의 실선으로 표현되지만
짧은 굵은 구간을 전체 병합 셀 경계로 확장해서는 안 된다.

이번 계약은 테두리 결합이며 폰트 외형·상대크기 완전 일치나 원본 전체 문서 수용은
주장하지 않는다. 색·선종 우선순위와 복선 혼합은 이 사례로 일반화하지 않는다.

## 시각 판정 입력과 별도 실패 입력의 구분

`table-input.hwp`와 이를 **직접** 출력한 `table-input-2020.pdf`의 큰 위치 차이는
미해결 증거로 보존한다. PDF job `d0cbe292-9833-4a50-ae60-5201f6f16910`, 같은 환경,
SHA-256 `193d717c5704225a9ca3214f16162c439cb548435d1752590901cfd43925e0cb`.
또한 HWP 직접 재저장본 `table-saved.hwp`는 host LineSeg 폭42520HU가
표46149HU보다 작아서 V2 문서 수용 단계에서 거부된다. 이 두 자료의 한계를
실선 지원으로 해결했다고 보고하지 않는다.

최종 시각 판정은 **`reflow-saved.hwp`와 `reflow-2020.pdf`**를 쓴다.
위 추출 Document의 모든 문단(셀 문단 포함)의 `line_segs`만 비운 후
`serialize_hwpx`로 `reflow-input.hwpx`를 만들고, 한컴 HWP 저장→PDF 출력했다.
표/테두리/배경/글자·문단 속성은 편의상 변경하지 않았다. 한컴 재저장 자체의
정규화와 HWPX 경유를 포함하므로 원본 byte-identical 입력으로 부르지 않는다.
같은 원본 출처라도 한 경로의 성공을 다른 경로의 성공으로 대체하지 않는다.

이 정상 대조군은 이전 CENTER 조사 때 생성된 산출물을 재사용했다.
HWP job `12fab34c-9cee-44e1-9664-ac7600d3e09d`,
PDF job `5160a1a1-9764-46d3-8251-886b6539d589`. 환경은 위와 같다.
원장: `output/7353/r19/stored-paint/reflow-{status,pdf-status}.json`.

| 파일 | SHA-256 |
| --- | --- |
| reflow-input.hwpx | 96860154c8dab865c5fe7ca7ec06467e2cac68e2bfb74f0b27d69cc373387321 |
| reflow-saved.hwp | 447f18c16f22f570d3dd81a010d49cb35df2d7b0e40c62072b194da7a96fe307 |
| reflow-2020.pdf | ae97244a64f336c19baf97e1ed1eb82a3cc47266094c7c4eacc1acac5956e4d8 |

원본 줄 보존 입력과 재생성 입력의 파싱 PageDef는 같지만 host 줄 vpos는
20892→3380HU로 바뀌었다. HWP 직접 재저장본은 host 폭42520HU,
HWPX 경유 정상 저장본은48188HU다. 위치/폭 차이의 원인 계층 전체는 아직 미검증이다.
