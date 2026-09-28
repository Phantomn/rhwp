# #7353 본문 문단 앞 명시적 쪽 나누기

## 생성과 범위

원본 `samples/86712_regulatory_analysis.hwp`의 첫13문단만 남겨 HWPX로 직렬화했다.
선택한 문단의 저장 LineSeg, 표, 글자모양과 `column_type`은 수동 수정하지 않았다.
`output/7353/r19/page-break/prefix13.rs`가 생성 코드다. 원본 첫14문단을 먼저
시도한 `input.hwpx`/`break-saved.hwp`는 뒤 규제 개요 표 안의 명시적 문단 나누기가
아직 미지원이므로 보존하고 성공 사례로 보고하지 않는다.

- `prefix13-input.hwpx` → 정상 HWP 저장: MCP2020 job `d80b8c14-6470-4801-83c2-527679d6f312`
- 같은 `prefix13-saved.hwp` → PDF: MCP2020 job `6d1e4a5d-8b75-4d0d-899f-e1a9b6c9630f`
- 둘 다 succeeded, Hancom11.0.0.9136. 입력/상태/다운로드 영수증은 같은 output 폴더.
- PDF는2쪽이며, 2쪽에는 원본 p12 `< 규제 개요 >`와 쪽 번호가 있다. 뒤 표는 이 입력에
  없으며 원본 전체의 조판 완료 자료가 아니다.

## 독립 계약

HWP5 사양 표59의 `0x04`는 쪽 나누기다. 정상 저장 IR의 p12에도 `Page`가 남고
첫 저장 LineSeg vertical_pos는0이다. PDF2쪽 제목 기준선은 trace 변환으로
`579 × 0.119869 pt`, 첫 글자 원점은 `472 × 0.119935 pt`다.96dpi 변환 후 실제
TextLine 기준선/왼쪽 원점을1px 이내로 검사한다. 본문 상단은 입력 PageDef에서 정한다.
앞 p11의 공백 문단은1쪽에 보존하며 p12 제목이1쪽에 중복되지 않아야 한다.

`tests/cases/issue_7353_table_v2_document_flow.rs`의 `explicit_body_*`가 실제 문서
fit/paint를 검사한다. 빈 첫 문단, 정확한 페이지 fit, 앞 표 분할/지연된 표 이어받기,
연속 나누기와 snapshot, TAC 한 줄, zero-width 자리차지 선언 줄을 별도 경계로 검사한다.
변형/합성 경계는 위 정상 저장 원본의 시각 일치 증거와 구분한다.

첫 문단의 Page, 본문 단/다단/구역 나누기, 셀 내부 문단 나누기는 이번 지원 범위가 아니다.
일괄 거부를 삭제하지 않고 미지원 경계를 유지한다. 테스트 페이지 수만으로 한컴 일치를
판정하지 않으며 Native/fresh WASM의 앞뒤 페이지를 같은 PDF와 비교한다.

## 증거와 한계

- 기존 Native 바이너리의 정상 저장본 거부: `output/7353/r19/page-break/before-native.log`.
- 기존 코드의 핵심 계약3건 거부: `before.log`. 임시 harness 의존성 누락은 결함 증거가 아니다.
- Native/fresh WASM: `review/`의 compare/standalone overlay/review와 `run.json`.
- 글꼴 외형·자폭 차이는 남는다.2쪽 비교값은 제목/쪽 번호만 있는 제한된 영역의 지표이며
  큰 흰 공간의 높은 전체 픽셀 일치율을 조판 완료 근거로 쓰지 않는다.

## SHA-256

| 파일 | SHA-256 |
| --- | --- |
| prefix13-input.hwpx | `7197927ab2539e34ee6130a63c7d92f23e3c02f838c64d1260aef8947b6fb574` |
| prefix13-saved.hwp | `f44556067eb17ef8e75cbc33e0fb25f4e951d9713cf8fd835567979a7acb8092` |
| prefix13-2020.pdf | `a04e9872fec73baa879ca1c2ef79062761d2180390e313b6d581c1a7ac9d041b` |
