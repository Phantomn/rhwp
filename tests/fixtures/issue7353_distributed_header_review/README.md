# 배분 정렬 제목 셀의 말미 공백 — #7353

원본은 `samples/issue6601/36331407_side_by_side_tac_tables.hwpx`다.
`create.py`는 top-level 문단만 제거하며, 남긴 문단의 텍스트·스타일·LineSeg·표 속성과
ZIP의 다른 리소스는 바꾸지 않는다. 저장 줄 정보를 수동 작성한 입력이 아니다.

- `python3 tests/fixtures/issue7353_distributed_header_review/create.py`:
  문단 index0,1,2,3,22,23,24를 남긴 `table.hwpx`(2쪽). 표지·본문 앞 두 표·대상 표의
  앞뒤 제목을 보존한다. 시각 판정과 정식 회귀 검사의 입력이다.
- 같은 명령에 `--prefix`: 앞25문단 전체를 남긴 `prefix.hwpx`(3쪽). 조사 범위와
  남은 차이를 보존하는 진단 입력이다.

## 독립 기준 PDF

두 PDF 모두 해당 HWPX를 한컴 MCP 변환 서비스로 출력했다. rhwp 출력이 아니다.
2026-09-28 KST, engine2020 / Hancom11.0.0.9136 / managed direct DLL / 32bit /
preprocess none. 영수증은 `output/7353/r19/header-runs/`에 보존한다.

| 입력 | 작업 ID | 영수증 접두사 |
|---|---|---|
| table.hwpx | 9d8b64c3-3ca4-4ebf-9df4-4f02bb5702d9 | table-pdf |
| prefix.hwpx | f9a6ddcf-5c36-44d7-975d-6325efd15191 | pdf |

SHA256:

- 원본: `1a34d18a4a334b52bc0dd00c1ee8c18f632316cf777699639f10980a60be98f8`
- table.hwpx: `a9cff1cb99c3125bf9e58160afa5ab5267866e700a722e2bd4891185c38619e4`
- table-2020.pdf: `46fb5fe3da872d51e6563e309a9247c49baa7ac13dc74b8c4960305e500116a9`
- prefix.hwpx: `3ce794c5570615e2f5cb5731e636d4031c19203995aefaa00d26018d917cf0ac`
- prefix-2020.pdf: `e8423797c42e7de0ef82bb4e59320b56479846f8a106cc2d68eeb953ef1561e6`

## 판정 영역과 한계

`table` 2쪽 마지막 표의 `안전/교육`, `하절기/재난` 두 줄 제목, 셀·표 외곽과 뒤의
`□ 점검 사진` 위치를 비교한다. 원본은 배분 정렬, 13pt이며, `교육 ` 및 `하절기 `의
말미 공백도 논리 텍스트로 보존한다. 실제 glyph 위치와 저장 행 높이를 검사하며,
공백 뒤 caret 간격을 글자 잉크의 셀 초과로 오인하지 않아야 한다.
정식 검사는 `distributed_header_spaces_preserve_table_rows_and_following_heading`이다.

`prefix` 3쪽에는 표 위 별표 마스킹 문단의 줄바꿈·위치 차이가 남는다. 해당 원본 문단21에는
저장 LineSeg가 없어 재조판되며 이번 제목 셀 수정 범위에 포함하지 않았다. 전체 prefix가
한컴과 일치한다고 판정하지 않는다. 표 중심 발췌는 이 차이를 수정하거나 원본 전체를
인증한 것이 아니며, 두 입력과 각각의 출력은 모두 보존한다.

Native/fresh WASM 실행과 시각 증적은 `mydocs/working/task_m100_7353_stage19.md`에 기록한다.
