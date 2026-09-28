# 본문 자리차지 표의 저장 선언 줄

원본: `samples/86712_regulatory_analysis.hwp`. 첫10문단만 보존한 분리본이다.
표/문단/저장 줄 메트릭을 손으로 수정하지 않았다. 뒤 원본 내용은 포함하지 않으므로
원본 문서 전체 지원의 근거가 아니다.

## 생성과 독립 기준

1. `output/7353/r19/body-exclusion/create.rs`는 원본을 파싱하고 section1개/paragraph10개로
   자른 뒤 HWPX로 저장한다(`contents-input.hwpx`).
2. 한컴 MCP `engine:2020` 정상 HWP 저장: job `6362ce55-c931-4b95-b456-00d1dc90818e`.
3. 같은 `contents-saved.hwp`의 PDF: job `2cf92a11-6f5c-49f6-872f-1fb8ba57cfca`.
4. `marker.rs`는 정상 저장 분리본 뒤에 “목차 표와 빈 문단 다음 위치 확인” 새 문단을
   넣어 HWPX로 저장한다. 이 추가 문단만 저장 줄을 비우고 한컴이 재조판하도록 했다.
   원래 빈 문단/표 셀 문단은 지우거나 바꾸지 않았다.
5. `marked-input.hwpx` 정상 HWP 저장 job `46499e21-ef39-4c3e-8e17-34818b2ded24`,
   같은 저장 HWP의 PDF job `44c3f174-ab8d-4dff-b524-6ab0c30a4b54`.

2026-09-27 실행, runtime `11.0.0.9136`, PDF 각1쪽. `marked-*`는 편집자 원문이 아닌
관측 가능한 후속 위치 대조군이며, 원본 분리본 `contents-*`와 별도 캡처한다.

## 기대 결과의 근거

- 원본 p8: 너비0 선언 줄, 높이1600HU/줄간격320HU/vpos25995.
- 표 높이16442HU+바깥 상하141HU씩=16724HU. p9의 저장 원점42719와 차이가 일치한다.
- 선언 줄은 표와 원점을 공유하되 실제 줄 상자를 보존한다. 다음 독립 빈 문단은
  1600+320=1920HU를 전진시키며, `marked-*`의 확인 문단으로 효과를 볼 수 있다.
- 표 셀의8문단은1600HU 줄높이/480HU 줄간격으로 이어진다. 마지막5개는 실제 빈 문단이다.
- 독립 `contents-2020.pdf` trace: 셀 clip x84.315..509.246pt,
  y(841−523.228)..(841−359.007)pt. 목차1/2/3 글자 원점은
  (746,2776)/(746,2949)/(746,3123), text transform(.119935,.119869).
  PDF 장치 양자화는96dpi 기준1px 미만 오차로 검사한다.
- `marked-2020.pdf` 확인 문단 기준선4306×.119869pt를 검사한다. 가운데 정렬 문단의
  글자 시작 x는 PDF 약177.99px, V2 약174.97px로 약3.02px 차이가 남는다. 글꼴 폭에
  종속된 이 좌표를 일치로 주장하지 않는다. 정렬 중심 불변식과 세로 기준선은 별도로 검사한다.
- 합성 작은 용지/CellBreak/Top 대조군은 내용·host의 단일 소비와 본문 경계 계약을 검사한다.
  해당 변형의 한컴 페이지네이션 일치 증거는 아니다.

## SHA-256

| 파일 | SHA-256 |
| --- | --- |
| contents-input.hwpx | de684af3aaa834e19b7b1208c2e9df2c19997a27545032d92112beb086d2fbc4 |
| contents-saved.hwp | 22ac8e39ece31b1a551f47ff6469ebd62b301fe7146cd6cf8eabccd9ebc2ca16 |
| contents-2020.pdf | 816191155f13d92676954887f8b93df568c7b3fc7682a4ce1f251641bc8c55ff |
| marked-input.hwpx | a5e0e28061493a5891fe808cec3bd8aa77774920978c3559d0c2cc9440f17e60 |
| marked-saved.hwp | a892eb68acd4a92472392eacda2ec59eba9574f2471f304facbd7572044e380d |
| marked-2020.pdf | 1f6af2e3b41b2110dd27eedd9e17c984de08d0a44c4cf8061524c702ba9b7127 |
