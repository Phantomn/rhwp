# URC 글자 상대 단위 내어쓰기

폴더명 `half-unit-indent`는 폐기한 초기 가설의 이름이다. 홀수 URC는 반 HWPUNIT이
아니라 CHAR 단위다. -3001은 signed shift로 -1501/100ch이며, 바탕글 기준으로 해소한다.

- 원문: `samples/86712_regulatory_analysis.hwp`
- `prefix21-saved.hwp`: 첫 section/첫21문단만 추출해 HWPX 직렬화 후 한컴 정상 저장.
  문단·표·스타일·LineSeg를 수동 수정하지 않았다.
- `normal12-saved.hwp`: 반환본의 바탕글 글자모양만 복제하여10pt→12pt로 바꾼 대조군.
  본문p20의run크기는14/15pt 그대로다. 정상 저장 과정에서 한컴이 줄바꿈을 갱신했다.
- 대응 `*-2020.pdf`: 각각 반환된 **동일 HWP**의 한컴 PDF, 각각10쪽.
- engine2020, Hancom11.0.0.9136, 전처리none.

생성 절차/응답/해시는 `output/7353/r19/half-unit-indent/`의 `prepare.rs`,
`normal-control.rs`, `*-start.json`, `*-status.json`, `*-download.json`에 보존한다.

| 입력 | 정상 저장 job | PDF job |
| --- | --- | --- |
| prefix21 | 601a3b1a-b59f-479b-a403-f7c442c9b2c0 | e78338ac-9e71-4412-921f-29b3c4e14d1e |
| normal12 | 4f61f952-ec7f-4853-853a-2586ec05bd3f | dd20863f-355e-41b3-b8b8-1a106b47d78c |

독립 기준: [한컴 URC 정의](https://forum.developer.hancom.com/t/topic/2209),
[ch 기준](https://help.hancom.com/hoffice/multi/ko_kr/hwp/format/paragraph/paragraph%28indenting%29.htm),
두 PDF10쪽의 약75/90pt 내어쓰기와 각 정상 저장본의 줄 구성.
원본 전체를 지원한다는 증거가 아니며, 글꼴 외형·프린터 좌표 잔차는 별도다.
