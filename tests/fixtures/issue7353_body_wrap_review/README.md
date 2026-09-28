# 본문 Square/BothSides 표와 저장 줄 구성

2026-09-28 생성한 한컴 저장 HWP와 동일 HWP의 PDF다. 원본 전체 문서의
완료 증거가 아니라 원본 문단43~46을 분리한 독립 출력 대조군이다.

## 생성과 출처

- 원본: `samples/issue4090/156492236_규제샌드박스_min.hwpx`.
- `wrap-input.hwpx`: section0의 paragraph43..46을 스타일·컨트롤·줄 정보와
  함께 추출했다. 독립 문서 시작에 불필요한 첫 문단의 앞쪽 나눔만
  `column_type=None`, `raw_break_type=0`으로 제거했다. 원본은 변경하지 않았다.
- 한컴으로 HWP 저장한 후 그 HWP를 PDF로 인쇄했다. 저장 LineSeg 수치를
  수동 생성하거나 수용 조건에 맞추어 수정하지 않았다.
- engine2020, Hancom11.0.0.9136, `hwp-managed-direct-dll-host`, 전처리 없음,
  `hancom2020_pdf_driver_one_up`, PDF1쪽이다.
- HWP job: `ad93fc20-c145-47f7-ac7b-dbb49a494c95`.
- PDF job: `ddd24b41-f9c7-425f-b562-ef422ea6fda1`.
- 재현 코드: `output/7353/r19/body-column-anchor/extract.rs`.
  최종 응답은 같은 폴더의 `save-{job,download}3.json`,
  `pdf-{job,download}3.json`이다. 출력의 `wrap2-saved.hwp`와
  `wrap2-2020.pdf`를 여기서는 각각 `wrap-saved.hwp`, `wrap-2020.pdf`로 보존한다.
  초안의 시작 페이지 나눔 오류와 최종 정상 입력을 혼용하지 않는다.

## 독립 기대값과 범위

원본 paragraph44의 표는 TAC가 아니라 Square/BothSides, Para/Left,
Para/Top이다. 오른쪽 사각형은 원래 내용이 빈 1x1 표이며 이미지 누락이 아니다.
호스트의 빈 글줄을 보존하고, 후속 문단의 첫6줄은 왼쪽에, 마지막 줄은
표 아래 전체 폭에 배치한다. 제목 표·빈 후속 문단·쪽 번호도 보존한다.

정상 저장 HWP의 HU 메트릭과 해당 PDF를 기대 근거로 사용한다.
본문 원점(5669,5668), 호스트 줄 y4696/높이800, 표 offset(26319,834),
외부여백 L1417/R0/T138/B138, 표 크기21022x13241이다. 본문7줄의 y는
5896+i*2252이며 처음6줄 폭26319, 마지막 폭48188이다. 표의 우측은 본문 폭을
조금 넘지만 용지 내부다. 객체 가용 범위를 확장해도 본문 줄 폭을 확장하지 않는다.

`tests/cases/issue_7353_body_square_wrap.rs`는 HWP/HWPX와96/192dpi에서 최종
표·줄 좌표, 줄 내용·빈 줄·쪽 번호를 검사한다. stale full-width 줄, 표의
왼쪽 침범, 용지 밖 앵커, 저장 호스트 정보 부재 및 같은 쪽에 객체 전체를
담을 수 없는 예산은 페이지를 부분 commit하지 않고 명시적으로 거부한다.
이 반례는 정상 저장본의 속성을 바꾼 계약 입력이며 한컴 출력 관측과 구별한다.

이번 범위는 저장 줄을 사용하는 온전한 같은 쪽 Square 배치다. fresh 어울림
재조판, 페이지를 건너는 어울림 객체, 호스트와 겹치는 앵커, Tight/contour는
미지원으로 남는다. 글꼴 외형·굵기·잉크 baseline 차이까지 해결한 증거는 아니다.

## SHA256

| 파일 | SHA256 |
| --- | --- |
| wrap-input.hwpx | `83901f9154e30d6b930667ea07d282ab1de4c4034e0dd5b76abe17580acfe930` |
| wrap-saved.hwp | `00975c3541d57ab4471bc5f4205aec6293df19c4de6335d05ccc10acc5be4966` |
| wrap-2020.pdf | `98748e21906d5569bbafdabcb69d269d6aafcd13126d8b6956678e5a18ebc515` |
