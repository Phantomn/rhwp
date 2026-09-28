# #7353 누름틀 시작과 TAC 표의 소유권

## 원본과 독립 대조군의 구분

원본은 `samples/issue6601/36331407_side_by_side_tac_tables.hwpx`다.
세 번째 본문 문단에 ClickHere 시작(본문, dirty=1)과 TAC 표가 차례로 있으며,
문자 수17(시작8+표8+문단 끝1), 표시 문자열 없음, 로컬 field range/end 없음이다.
필드는 표시 객체가 아니지만 원본 슬롯을 삭제하면 뒤 표의 줄 소속과 컨트롤 인덱스가 바뀐다.

- `create.py` / `prefix.hwpx`: 원본 앞 네 문단만 남긴 진단본. 속성·줄 메트릭은 보존한다.
- `prefix-saved.hwp`: 위 진단본을 한컴으로 저장한 별도 진단본.
  job `6ed795f8-ee50-412a-9d36-bd70d70630fb`.
  필드 절편 당시에는 원본과 이 저장본 모두 필드 진입을 통과한 뒤 표 내부의
  `text preview run outside occupied line`에서 거부됐다. 아래 후속 절편과 구분한다.
- `create-control.py` / `control-input.hwpx`: 원본 표지 두 문단을 남기고 원본의 누름틀
  시작 컨트롤을 표지 둘째 문단의 TAC 앞에 복사한 독립 대조군이다. 모든 LineSeg를
  제거한 뒤 한컴이 다시 구성·저장한다. 원본 본문 표를 고쳐 통과시킨 자료가 아니다.
- `control-saved.hwp`: 한컴 정상 저장 job `7b2a17bc-20fa-4467-a426-0587bbd75f83`.
- `control-2020.pdf`: **동일 control-saved.hwp**의 PDF 출력
  job `d39274e9-8849-46c9-a30d-a220a0460d04`, 1쪽.

한컴11.0.0.9136 / engine2020 /32-bit direct DLL / preprocess none.
생성 요청·상태·다운로드 기록은 `output/7353/r19/body-field/`에 보존한다.
PDF는 rhwp가 생성한 것이 아니다.

## 후속 절편: 배분 정렬 연락처 행

`prefix-2020.pdf`는 **동일 prefix.hwpx**를 한컴으로 PDF 출력한 2쪽 자료다.
job `688605b1-8cfb-4624-8571-3f38e3289358`, 한컴11.0.0.9136,
engine2020/direct DLL/32bit/preprocess none. 요청·완료 기록은
`output/7353/r19/run-extent/pdf-*.json`에 보존한다. 원본 앞 네 문단의 속성·LineSeg는
변경하지 않았고, 뒤 문단만 발췌에서 제외했다. PDF 2쪽에서 제목 두 줄, 연락처 한 줄과
뒤 요약 표 두 줄을 직접 대조한다. 원본 전체 통과 자료는 아니다.

연락처는 배분 정렬이며 저장 줄 폭48324HU·높이1200HU다. 분배 간격은 N-1개이므로
마지막 글자 뒤 caret advance는 잉크 점유가 아니다. 기존 SVG/Canvas의 공통
`glyph_fit_advance`와 replay positions를 V2 검증이 소비하도록 고친다.
수정 전 끝650.900529px / 줄 끝648.32px / 양수 간격2.580529px였다(셀 로컬 좌표).
run advance·저장 줄·표 크기·다음 표 원점은 변경하지 않는다. 장식이 있는 run은 여전히
장식의 전체 advance를 검사한다. 실제 SVG 글리프 끝과 다음 내용은 정식 text/document_flow
계약으로 검사한다. 이 값은 구현에서 임의로 줄인 폭이 아니라 배분 정렬 규칙과 저장 폭이다.

## 독립 기대값과 검증 범위

정상 저장본에도 `[Field, Table]`, char_count17, LineSeg 시작0, 로컬 range/end 없음이
남았다. 표는 원본 index1/UTF-16 offset8을 유지하고, 누름틀은 가시 안내문이나 추가 높이를
만들지 않아야 한다. PDF는 결재란·표지 제목·로고·부서명을 모두1쪽에 출력한다.
부모 위치21682HU는 body7088+앞 줄13136+간격1320+표 위여백138이다.
표 높이56490HU, 전체 표6개·그림1개와 문서 종료를 검사한다.

정식 `issue_7353_table_v2_document_flow`의 정상 저장본 검사는 위 소유 인덱스·최종 좌표·
내용/안내문·종료를 확인한다. 별도 합성 HWPX 계약은 본문/중첩 셀, 시작 필드1/2개,
같은 줄/다른 줄 표, 뒤 문단1회 출력의 기하를 필드 없는 대조군과 비교한다.
날짜 필드·dirty=0·표 뒤 필드·fresh 줄·누락 슬롯·빈 명령은 거부한다.
합성 생성 과정에서 첫 문단에 자동 추가되는 구조 슬롯의 축 모호성을 피하려고
선행 BEFORE 문단을 둔다. 저장 정보 수용 조건을 완화하지 않는다.

수정 전 동일 정상 저장본은 `body control or multiple anchors`로 거부됐고,
수정 후1쪽으로 수용된다. 원본 전체, 닫힌/교차 필드 범위, 편집 후 필드+TAC 재조판,
다른 종류의 객체 혼재는 이번 통과 범위가 아니다. 대체 글꼴 외형 차이는 남는다.

## 파일 해시 (SHA256)

| 파일 | SHA256 |
| --- | --- |
| prefix.hwpx | cff5673cfa3c5ff44ab5303f01c2b8999aeead30ddbc2a9757706a720540c185 |
| prefix-saved.hwp | c66791ac1b54a80f08fa1b9b2e777da321efc996546a1bcfd3d14fa348ffcf8f |
| control-input.hwpx | 617dc0d0119973cdf6f1877d0a30f6c9be1f3e94cd2307766c037a687ce58184 |
| control-saved.hwp | 43aae8c16f3a9dbb156deb78852783f658b6a0c9461766f95e0ff7248ca59d38 |
| control-2020.pdf | 2a9e1d03287c8bc0bc152ab55a9a89f8af58cebd68d2f78881ce84e3fcd1856d |
