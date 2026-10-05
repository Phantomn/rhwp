# PR #7491 편집 후 독립 출력 입력

기존 public sample과 빈 문서를 실제 DocumentCore 편집 API로 수정한 저장본이다.
LineSeg·문단 속성을 수동 작성하지 않는다. 생성기는
[generate_inputs.rs](../../../mydocs/pr/assets/pr7491/generate_inputs.rs)다.

- typed-flat/first/hanging: 새 문서에 동일 본문을 입력하고 indent 0/3000/-3000 적용.
- 6190-edited: 기존 samples/issue6190/center_align_first_line_indent.hwp 문단 4의 끝과 문단 7의 앞에 각각 `가` 입력.
- biz-edited: samples/biz_plan.hwp 문단 51의 끝(67)에 `가` 입력.

각 HWP와 같은 이름의 `-2020.pdf`는 해당 HWP의 동일 바이트를 한컴 MCP engine 2020으로 변환한 독립 기준이다.
변환 결과를 rhwp 산출물로 대체하지 않는다. 입력/출력 해시는 PR review 증적에 연결한다.
