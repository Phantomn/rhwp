# 문단 나누지 않음(keep_lines) 정상 저장 대조군

`keep-saved.hwp`와 같은 입력을 한컴으로 인쇄한 `keep-2020.pdf`(2쪽)를 비교한다.
이 자료는 #7158 전체 문서의 통과 증거가 아니다.

## 생성

2026-09-28 `issue7353_body_frame_review/portrait-saved.hwp`의 정상 글꼴과 용지를
재사용했다. 본문을 BEFORE/BEFORE2/BEFORE3의3줄 문단, ALPHA/빈 줄/CHARLIE의
3줄 문단, AFTER로 교체하고 저장 LineSeg 없이 HWPX로 직렬화했다.
새 문단 스타일은 Fixed 줄간격3600URC이며, 두 번째 스타일만 attr1 bit18을 설정했다.
`keep-input.hwpx`가 그 입력이다. 글자크기1200HU, 본문 위 여백1500HU,
본문 높이9000HU이며 원본 생성 코드는 `output/7353/r19/keep-lines/create.rs`에 있다.

- HWP 정상 저장 MCP job: `f11a0f76-98f9-4e67-9938-3767836e274e`
- 그 HWP의 PDF 출력 job: `518c2b0a-f54b-456f-b859-b31d3b59cd0a`
- engine2020, 한컴11.0.0.9136, `hwp-managed-direct-dll-host`, 전처리 없음,
  PDF `hancom2020_pdf_driver_one_up`.

## 기대값과 범위

1쪽에는 BEFORE/BEFORE2/BEFORE3만 있고, ALPHA/빈 줄/CHARLIE는 모두2쪽으로 이동한다.
2쪽 AFTER도 보존한다. 한컴이 저장한 보호 문단의 vpos는0/1800/3600HU,
줄 높이는1200HU다. 96dpi의2쪽 줄 상자 시작은20/44/68px, AFTER는92px다.
문단을 나눠서 첫 쪽 빈 공간을 채우면 안 된다.

정식 회귀는 `issue_7353_table_v2_document_flow.rs`의 정상 HWP와 합성 fresh HWPX,
`issue_7353_table_v2_text.rs`의 셀 경계/겹친 줄/문단 간격을 구분해 검사한다.
이 HWP를 rhwp로 HWPX 재직렬화한 경로는 첫 번째 비보호 문단의 저장 줄 검증에서
거부되어 이 자료로 round-trip 통과를 주장하지 않는다. 입력이나 수용 조건을 바꿔
해당 오류를 숨기지 않았다. 한 페이지보다 큰 보호 문단, 다음 문단과 함께,
외톨이줄 보호, 개체 혼합 문단은 이번 지원 범위에 포함하지 않는다.

```text
c282784f9fefffd895d8b7dc7d8cd91b960c1ac995bc6a5ef9c99a19daa62a75  keep-input.hwpx
337b36e68ac9f62de47d28f83746ec32f805e04c986c6efa8982f013ce21d875  keep-saved.hwp
13943e453c3078b8e8c1785be6cd2ba1f3bb9eebc2a27a6a4b4cd700132bcbb6  keep-2020.pdf
```
