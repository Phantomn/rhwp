# #7353 양쪽 정렬 말미 공백의 폭

`create.rs`는 #6923 원본의 `s0/p5/t0/c0/p4/t0/c6/p1` 텍스트와 문단·글자 서식을
사용해 독립 1쪽 대조군을 만든다. 원본 전체 출력이나 발췌 페이지가 아니다.
저장 줄은 수동 작성하지 않고 한컴이 정상 저장한 `justify-saved.hwp`를 그대로 검사한다.
`justify-2020.pdf`는 **동일 저장 HWP를 한컴으로 출력**한 기준이며 rhwp PDF가 아니다.

## 출처

- MCP CLI `start → status → download`, engine2020, Hancom11.0.0.9136
- backend `hwp-managed-direct-dll-host`, 32bit, preprocess none
- HWP job `58432e1e-b0f8-480b-8f6a-9425051781a2`
- PDF job `98dec785-86d5-4f54-b463-8ba29991523e`, 1쪽, printMethod0
- 폰트 mapped/registered2, failed0, verified

| 파일 | SHA-256 |
| --- | --- |
| justify-input.hwpx | 905e4e65b6316a8ff8e153bd4c0f20d2943904c286b087bca2bacd9957f1516b |
| justify-saved.hwp | 82014c9c526745e98b8837c00a19ca3566ccd8052bdf23ccffbf87bbcfa0e6a5 |
| justify-2020.pdf | 3147a3cdc485b5abb49b03c150bd777fd27f8113e7a7a5a7c10bd107e4eadffd |

## 독립 기대값과 확인점

- 본문 원점3969,5669HU. TAC 표23741×7500HU, 셀 안 여백283HU.
- 셀 첫 줄23172HU, 높이1100HU, 다음 줄 원점1364HU, text_start30.
- 첫 줄은 `병당 `까지, 둘째 줄은 `2.6%`부터. 둘째 줄의 내어쓰기2064HU를 보존한다.
- `AFTER CELL`은 저장 vpos7764HU: 표 아래 간격264HU 뒤에 한 번만 나타난다.
- 양쪽 정렬은 말미 공백을 빼고 가시 텍스트를 저장 오른쪽 경계에 맞춘다.
  자연 폭과 말미 공백 폭은 같은 소수 정밀도·각 run의 서식으로 계산해야 한다.
  이 정렬 불변식과 원본 문단의 분할/이어받기는 `tests/cases/issue_7353_table_v2_text.rs`,
  정상 저장본의 최종 표·글줄·뒤 본문 좌표는 `issue_7353_table_v2_document_flow.rs`가 검사한다.

첫 시도는 부유 부모 표의 뒤 본문 앵커 차이를 보여 판정 자료에서 제외했다. 그 입력·저장본·PDF와
review는 `output/7353/r19/run-box/anchor-diagnostic/`에 보존했다. 현재 대조군은 부모를 TAC로
새로 작성하고 한컴에서 다시 저장/출력한 것이다. 대조군 통과를 원본 전체 수용으로 보고하지 않는다.
글꼴 모양·굵기 차이는 별도이며, 0.055px 수준의 초과 해소 자체는 육안보다 좌표 계약으로 검증한다.
