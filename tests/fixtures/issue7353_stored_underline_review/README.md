# 저장 자동 줄바꿈의 밑줄 공백

#7353의 `issue7353_tac_blank_row_review/create.rs`를 **`--simple` 없이** 실행한
`output/7353/r19/tac-next/carrier-input.hwpx`의 정상 한컴 저장본이다. 원본 #6923의
`s0/p5/t0/c0/p69`를 분리하며 자식 표의 네 셀·문단·글자모양을 보존한다. 부모 표 높이와
뒤 본문을 검증용으로 지정하고 저장 LineSeg는 제거한 뒤 한컴이 다시 생성했다.

- HWP 정상 저장 job: `2aa91c6d-8364-45bc-a0a1-c13fa0ac635d`.
- **동일 HWP** PDF job: `16a5f793-857f-45a0-9e29-ffb4ecbc7117`.
- 한컴11.0.0.9136, engine2020,32-bit directDLL, preprocess none.
- 원본의 음수 여백은 한컴 재저장에서0으로 정규화되었다. 이 입력의 통과를 원본 음수
  여백 지원 또는 원본 전체 통과로 바꾸어 보고하지 않는다.

비교 대상은 첫 공백 줄과 둘째 줄 표, 두 법령 셀의 밑줄 자동 줄바꿈, 표 외곽과 뒤 본문이다.
기존 #6028/#6117의 공통 paint 규칙에 따라 soft-wrap이 소비한 마지막 가시 run의 구분
공백은 장식선에서 제외한다. 문단 끝·명시 개행·별도의 공백-only run의 작성자 공백은
제외하지 않는다. 줄 높이·줄 소속·논리 공백 텍스트는 바꾸지 않는다.

동일 입력과 기준 출력의 SHA-256:

- `carrier-saved.hwp`: `fbe1aea8b1f6f5b44ec3c98713c44e76a5d14f70d305cb364cdeb43e914009c3`
- `carrier-2020.pdf`: `12249a0aab0f9ec95de425dc4d69a64a571e30653f3f8f956df7c3ddbedf719c`
