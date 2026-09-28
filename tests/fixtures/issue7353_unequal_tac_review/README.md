# #7353 서로 다른 높이의 중첩 TAC 대조군

## 출처

`issue7353_tac_noop_review/noop-saved.hwp`의 정상 글꼴·용지·부모 표·앞뒤 문단을
보존하고, 내부 TAC를 두 표로 변경한 독립 대조군이다. #6601 원본을 고친 파일이 아니다.
변경한 carrier와 자식 문단의 LineSeg는 비워 `review-input.hwpx`로 저장했다.
한컴 MCP engine2020으로 이를 저장한 **수정 없는** `review-saved.hwp`와 그 저장본의
한컴 출력 `review-2020.pdf`를 비교한다. PDF는 rhwp가 생성하지 않았다.

- 생성 코드: `create.rs` (저장소 루트에서 실행; output 경로에 HWPX 생성)
- HWP job: `a65fe6ee-c8a1-492a-9bad-0a8c93e4b37d`
- PDF job: `17f09645-1c3e-4a39-b849-56c38201e543`
- Hancom11.0.0.9136, managed-direct-dll-host, input_preprocess none, 1쪽
- font_scope session0 verified; mapped/registered2, failed0

| 파일 | SHA256 |
| --- | --- |
| review-input.hwpx | 71ec0e63669990881203d6491bbd09b06da9bdee31aab7a1b91144fab549c2ce |
| review-saved.hwp | 400f8f0349b86cb7f123a701147bca49e56b866336b805928f416c3faf1a9248 |
| review-2020.pdf | 8629c914d6b221e7e4f497aec41d899a58ccf80e04c94e9f44412aa029ff0455 |

## 독립 기대값

두 표의 너비10000HU, 높이5000/8000HU, 바깥여백 각200HU.
한컴 저장 carrier: vpos1760, height8400, spacing660, width31432HU.
부모 원점(3969,8787), 안여백283, 높이18000HU.
따라서 carrier 원점10830HU에서 공통 baseline은7000HU 아래이며,
작은 표/큰 표의 위쪽은2750/200HU 아래다. 실제 표 y13580/11030HU.
저장 줄 `CELL AFTER` vpos10820 → y19890HU, 뒤 본문 y27354HU.

`mutool draw -F trace review-2020.pdf 1`의 clip을 페이지 위 기준 pt로 바꾸면:

| 표 | x | y | width | height |
| --- | ---: | ---: | ---: | ---: |
| 부모 | 39.671 | 87.793 | 319.643 | 179.903 |
| SHORT TABLE | 97.559 | 135.647 | 99.956 | 50.013 |
| TALL TABLE | 201.470 | 110.221 | 99.836 | 79.877 |

PDF trace의 text transform은 x=.119851, y=.119935로 .12pt 장치 단위 대비
인쇄 축척이 각각 .119851/.12, .119935/.12다. 이를 좌표·크기에 적용한 뒤
장치 양자화 차이를0.25pt 이내로 검사한다. 최초 축척 미적용 검사는 부모 너비
320pt 대319.643pt로 실패했으며 허용 오차를 늘리는 대신 독립 PDF의 축척을 반영했다.
비교 PNG/overlay에는 이 축척 변환을 적용하지 않아 미세한 오른쪽 외곽 차이가 남는다.
저장 HU 기반 실제 y·내용 보존·뒤 문단은 부동소수점 오차만 허용한다.
`issue_7353_table_v2_document_flow`는 변경 없는 저장본과 carrier LineSeg만 지운
별도 재조판 계약을 검사한다. 두 경로를 동일 입력이라고 주장하지 않는다.
Top/Center/Bottom 문단 세로정렬, 혼합 보이는 텍스트, #6601 전체는 이 대조군의 범위가 아니다.
