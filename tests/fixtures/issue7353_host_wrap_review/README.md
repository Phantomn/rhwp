# 빈 호스트 줄 옆에서 시작하는 본문 어울림 표

## 독립 입력과 기준 출력

원본 `samples/issue4090/156492236_규제샌드박스_min.hwpx`의 section0,
paragraph51~55를 스타일·표 속성·내용과 함께 분리했다. 첫 문단의 앞쪽 나눔만
`column_type=None`, `raw_break_type=0`으로 제거했으며 원본은 변경하지 않았다.
`host-input.hwpx`를 한컴으로 HWP 저장하고 그 `host-saved.hwp`를 PDF로 인쇄했다.
저장 줄 메트릭은 수동으로 작성하거나 수정하지 않았다.

- 생성 코드: `output/7353/r19/host-side-wrap/extract.rs`.
- engine2020 / Hancom11.0.0.9136 / `hwp-managed-direct-dll-host`, 전처리 없음.
- HWP 저장 job: `32461f7f-0968-444b-9e71-cd67ad8a263d`.
- PDF job: `2a7b02a8-0fb8-430c-8692-9f3e10551ebe`.
- PDF1쪽, `hancom2020_pdf_driver_one_up`.
- 응답: 같은 output 폴더의 `save-{job,download}.json`, `pdf-{job,status,download}.json`.

| 파일 | SHA256 |
| --- | --- |
| host-input.hwpx | `4fc8c8a508dbaf91fe52d6228771b926ba689d133af6e1fc24679360306cebbc` |
| host-saved.hwp | `d3ead0469dc235ed6a503c8f287849c54201ea19f32c77eea379b43d6ead784e` |
| host-2020.pdf | `6d8d24b399c00d031804dde26d0745c74348dbddf5f7bb119712bab8c46bd025` |

## 관측과 기대값

본문 원점은(5669,5668)HU다. 분리본 문단1의 빈 호스트 줄은 본문 y6764,
높이800, 폭26319HU다. Square/BothSides 표는 Para/Left, Para/Top 기준
offset(26319,498), 외부여백 L1417/R0/T138/B138, 크기21022x13241HU다.
따라서 표 상단은 호스트 원점+636HU이며 호스트 높이800HU의 안쪽에서 시작한다.
호스트 줄의 오른쪽은 표 왼쪽 외부여백 시작과 접한다. 세로 구간 공유는
2차원 점유 영역의 충돌을 뜻하지 않는다. 표 안쪽이 빈 것은 원본의 빈1x1 표다.

후속5줄은 y7964+i*2252, 폭26319HU를 유지한다. 그 뒤 작은 빈 줄은
y19224/높이100, 주석은 y19376/높이1200HU다. 제목 표·주석·쪽 번호를
함께 비교하며 폰트 글립·굵기·따옴표 advance의 차이는 배치와 구분한다.

## 계약 범위

`tests/cases/issue_7353_host_square_wrap.rs`는 정상 HWP/HWPX round-trip의
96/192dpi 최종 좌표·본문 보존·빈 줄·주석·쪽 번호·종료를 검사한다.
다음은 정상 입력의 속성을 변경한 합성 반례이며 별도의 한컴 출력 관측이 아니다.

- 호스트를 전체 폭으로 넓히면 빈 문자열이어도 실제 겹침으로 거부한다.
- PageNumberPos 선언을 제외하고 HOST 글자를 넣어도 같은 안전한 옆 공간을 허용한다.
- TopAndBottom으로 바꾸면 호스트 세로 구간 공유를 계속 거부한다.
- 표+바깥여백의 실제 점유 끝20779HU 전후1HU 예산에서 수용/거부한다.
- 여러 쪽에 나뉜 호스트의 마지막 줄만을 문단 전체 원점으로 오인하지 않는다.
  해당 경로는 미지원이며 오류 페이지를 commit하지 않는다.

같은 쪽에 온전히 배치되는 저장 어울림 경로의 증거다. fresh 어울림 재조판,
어울림 객체의 여러 쪽 분할, 원본 전체 문서의 통과 증거로 확대하지 않는다.
