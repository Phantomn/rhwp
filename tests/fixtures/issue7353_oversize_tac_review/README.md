# 빈 본문 페이지의 초과 높이 TAC

## 입력과 독립 근거

`create.rs`는 `../issue7353_fresh_tac_box_review/carrier.hwpx`에서 SHORT TABLE 하나를
남기고 같은 스타일·폭·내용으로 셀 최소 높이를 늘린다. 저장 LineSeg를 만들지 않는다.
본문 높이는48190HU다. first/preceded는51190HU, grown은66190HU의 셀을 가진다.
preceded에만 BEFORE TABLES가 있으며 세 입력 모두 AFTER TABLES가 뒤따른다.

각 `.hwpx` **자체**를 MCP engine2020(한컴11.0.0.9136, 전처리none)으로 PDF 출력했다.
이 PDF들은 HWP 재저장본의 출력이 아니다.

| 입력 | MCP job | 한컴 물리 쪽 소유 |
| --- | --- | --- |
| first.hwpx | e1830056-e253-406f-91aa-33e1938ed075 | 1쪽 표, 2쪽 AFTER |
| preceded.hwpx | 366f561b-6a3f-4019-bf6d-c394af20d134 | 1쪽 BEFORE, 2쪽 표, 3쪽 AFTER |
| grown.hwpx | 287534c6-5154-4386-a905-ffcbac184bee | 1쪽 표, 2쪽 AFTER |

PDF `mutool draw -F trace`의 표 위쪽은 페이지 위56.610pt다.
first의 아래쪽은568.254pt, grown은718.174pt로595pt 종이보다 아래에 있다.
한컴은 종이 밖 부분을 종이 클립으로 표시하지 않지만 다음 쪽 표 조각으로 이월하지 않는다.
높이 증가가 한 줄 허용치 이내라는 가정은 이 대조군에 해당하지 않는다.

## 정식 계약과 한계

`.hwp`는 같은 생성기에서 secd/cold의 명시적 구조 슬롯을 보존해 직렬화한 합성 계약이다.
V2의 fresh TAC 경로가 첫 문단 HWPX의 구조 축 이동을 아직 수용하지 않아, 그 입력 검증을
완화하지 않고 HWP 계약을 분리했다. **HWPX/PDF 통과를 같은 HWP 입력의 피델리티 통과로
바꾸어 주장하지 않는다.** HWP 자체의 독립 PDF 변환은 별도로 요청했으며 완료 증거 없이는
정답지로 사용하지 않는다.

`tests/cases/issue_7353_table_v2_document_flow.rs`는 HWP 계약의 실제 표 원점·전체 높이,
쪽 소유, 한 번씩 출력, 뒤 문단의 다음 쪽 원점과 종료를 검사한다. 기대 높이는 작성한 셀
최소 높이, 원점은 용지 여백과 중앙 정렬에서 구한다. 이 계약은 기존 `DoesNotFit`를
원본 HWPX와 함께 재현하며 수정 후 원본2쪽과 계약2/3/2쪽이 모두 완료된다.
`issue_7353_table_v2_nested.rs`는 셀 내부 TAC가49px 예산에50px를 넘겨 배치하지 않는
비적용 경계를 검사한다.

동일 원본 전체 문서의 별도 시각 증거는
`samples/issue2470/36382471_masked.hwpx`와 그 자체의 한컴 PDF를 사용한다.
`output/7353/r19/oversize-body/`에 수정 전후 실행·Native/fresh WASM 증적이 있다.
이번 규칙은 본문 원점의 indivisible TAC 줄에 한정한다. 일반 행 분할, 셀 내부 overflow,
양수 문단 앞 간격을 동반한 oversized TAC의 일반화는 이 증거로 완료 판정하지 않는다.

## SHA256

| 파일 | SHA256 |
| --- | --- |
| first.hwpx | 0378f33bc6019aa1614394b83e33fef2f4a9b1a3ecc542657b3d50bae940e5e0 |
| preceded.hwpx | 6a54f578fa5f874dd4b34903ab1cb1421dcd58c8d290e25b80cdd47ad6cf9222 |
| grown.hwpx | 7102db3891c277b3f58ad53ace4c325a7bf02b7341b6a7072c654c2e01dc16bb |
| first-2020.pdf | e899dd8bd0a547a6cff54eef48a0c0f2c4fabc54bd2fb6f366d484d522727486 |
| preceded-2020.pdf | 8b1dd0b7ea0dfc174d09b239941ff79474d0d6587e309a82cd88fcefc4d9bc79 |
| grown-2020.pdf | 47c35ecf327a3e89dcf8eed30ad8592f2c0a463a5bab6dd7841fdf4c96ddb3d2 |
| first.hwp | bc1416209df1abf4dc36804914147395e10fddc0a50206ab2771c9fd87ece9b2 |
| preceded.hwp | fa092b6e06cde183c23f097ca6b662a8bc03cd2bd955d26108314b6ef5603e55 |
| grown.hwp | d8548e6a9399762be57143aa7951fb0c0b3452083457fa5bf894ab9b116b7e19 |
