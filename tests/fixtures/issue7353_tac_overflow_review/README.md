# #7353 저장 줄보다 넓은 단일 TAC 표 — 수용 경계

## 저도주 표: 다음 미지원 기능을 보존한 입력

원본은 `tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp`,
독립 기준은 같은 stem의 `-2020.pdf` physical p7이다. 문단35의 표는48083HU,
좌우 바깥여백은 각각141HU다. 저장 줄 폭48188HU보다 큰 객체를 한컴은 축소하지 않고
왼쪽58.05pt 부근부터 조판한다. 표 뒤 공백도 원본에 존재한다.

아래 코드로 원본 문단33..36을 추출했다. LineSeg, PageDef, 표·글자·문단 속성은
수동 수정하지 않았다. HWPX는 원래 첫 문단의 SectionDef가 없어도 Section PageDef를
직렬화한다. 추출본을 한컴에서 HWP로 정상 저장한 뒤 그 HWP를 PDF로 출력했다.
재저장을 포함한 대조군이며 원본 전체 파일과 byte-identical하다고 주장하지 않는다.

```rust
let mut d = rhwp::parse_document(&std::fs::read(
    "tests/fixtures/issue6923/148738070_wrapper_table_stored_page_frame.hwp"
).unwrap()).unwrap();
d.sections[0].paragraphs = d.sections[0].paragraphs[33..37].to_vec();
let input = rhwp::serializer::hwpx::serialize_hwpx(&d).unwrap();
```

2026-09-26 MCP engine2020, Hancom11.0.0.9136, direct32bit,
input_preprocess=none, PDF one-up. HWP job `45cea874-0cfb-4153-b401-027ad84b3986`,
PDF job `10caf2d9-e947-44ad-aba6-40183f6e6185`.
원장: `output/7353/r19/tac-overflow/sales-{status,pdf-status}.json`.

| 파일 | SHA-256 |
| --- | --- |
| sales-input.hwpx | 59629709eb38d127cc4f122b6033b50fe928ba1b3cd167446f1733c434a6d4cf |
| sales-saved.hwp | ecbdc0b00f96e703521dfe2cc38475d4232fcdc1cb4a1a1f3e8d2da8a11e287f |
| sales-2020.pdf | 134a4b0e3504a2f423a59b076715619e7a9e82bcf3f4ee4453fbb6c342bc28ab |

**이 입력의 전체 V2 출력은 아직 미지원이다.** 폭 검사 통과 후 셀72 문단0의
`Field(Formula, =SUM(ABOVE)??%g,;;100)`에서 명시적으로 거부한다.
이 필드를 지우거나 결과 문자열만으로 대체하지 않았다. 정식 회귀 검사는 다음 거부
경계를 고정하며, 정상 배치/시각 통과 검사로 세지 않는다.

## 실제 시각 판정 입력

기존 `tests/fixtures/issue7353_shared_border_review/table-saved.hwp`와
`table-2020.pdf`를 재사용한다. 해당 디렉터리 README의 생성·출처·해시를 따른다.
이 파일은 Right 문단이며 표46149HU+바깥여백282HU가 저장 줄42520HU보다 크다.
PDF는 왼쪽86.473pt 부근부터 원래 폭으로 배치한다. 즉 음수 오른쪽 정렬 offset으로
옮기거나 축소하지 않는다. 수정 전에는 폭 거부, 수정 후에는 이 파일 전체1쪽을 출력한다.
테스트는 실제 표 원점·폭·높이와 뒤 자료출처의 유일한 배치 및 원점을 검사한다.

합성 계약은 Left/Justify/Right, 후행 공백의 소유, 다음 문단 원점, 셀 안쪽 물리 폭 제한을
검사한다. 여러 객체·선행 공백·후행 공백만의 초과·Center 정렬로 이 수용을 확대하지
않는다. 합성 계약은 해당 조합별 한컴 시각 일치 증거가 아니다.
