# #7353 표 오른쪽 경계 정상 저장 대조군

## 입력과 독립 기준

원본은 `samples/86712_regulatory_analysis.hwp`다. 첫 section의 첫12문단을
그대로 남기고 HWPX로 직렬화한 `input.hwpx`를 한컴 MCP에서 정상 저장했다.
원본 표의 셀 너비·병합·LineSeg·서식을 수동 수정하지 않았다. 원본의 나머지 문단은
포함하지 않으므로 이 대조군의 통과는 원본 전체 통과가 아니다.

- 생성 코드: `output/7353/r19/grid-boundary/create.rs`
- HWP 저장: engine2020, job `6a3765eb-5613-420c-8d55-2fcd8cebb2bd`
- 같은 HWP의 PDF: engine2020, job `9a85b48c-c0f7-4ef5-bed0-1e35438ae7fe`
- 두 작업 모두 succeeded, Hancom11.0.0.9136. status/download 영수증은 같은 output 폴더.
- `grid-saved.hwp`와 `grid-2020.pdf`가 동일 입력의 정상 저장본/독립 출력이다.

## 관측과 계약

본문 p10의6행11열 작성자 표는 앞5행의 셀 너비 합47,953HU, 마지막 행47,958HU,
표 선언 너비47,958HU다. 정상 저장 후에도 이 차이가 유지된다. V2의 모든 경계에 대한
정수 완전 일치 가정 때문에 이전에는 `inconsistent grid boundary`로 거부됐다.

표 선언 너비와 완전한 행이 오른쪽 외곽을 함께 확인해 주는 경우, 짧은 행의 마지막 셀에
잔여 너비를 반영한다. 내부 열 경계는 그대로이며 source IR은 바꾸지 않는다. 선언 너비가
없거나 최대 행 너비와 다르면 여전히 거부한다. 문서 ID나 임의 오차 허용치를 쓰지 않는다.
PDF trace의 첫 행 셀 왼쪽 좌표는58.049/164.911/226.918/337.618/377.797/429.489pt,
공유 오른쪽은537.311pt다. 마지막 행 하단은상단 기준723.768pt(841−117.232)다.
96dpi 변환 후1px 이내로 실제 셀 배치를 검사한다. PDF 래스터 정밀도만으로5HU 자체를
입증하지 않으며, 저장 구조의 선언 너비/완전 행과 PDF의 외곽 연결을 함께 근거로 사용한다.

표 수용 뒤 드러난 “ 정책책임자 직위 : ”의 오른쪽 정렬은 전체 run의 소수 폭에서
정수 반올림된 말미 공백 폭을 빼는 문제가 있었다. 가시 advance가 저장 줄 오른쪽을
0.5px 넘었다. 같은 소수 측정을 사용하며 원래 공백·자간·줄바꿈을 보존한다.
독립적인 오른쪽/가운데 정렬 불변식과 다른 글자모양으로 나뉜 말미 공백도 검사한다.

## 증거와 한계

- 정식 검사: `tests/cases/issue_7353_grid_terminal.rs`,
  `issue_7353_table_v2_text.rs::right_and_center_alignment_use_exact_style_owned_suffix_advances`.
- 수정 전 grid 거부: `output/7353/r19/grid-boundary/before.log`.
- grid 수정 후/공백 측정 수정 전 실제 paint 거부: `before-text.log`.
- 시각 대조: 같은 output의 `review/` Native/fresh WASM compare/overlay/review.
- 글꼴 외형·굵기·자폭은 한컴 PDF와 차이가 남는다. 원본 전체와 Studio 편집 세션의
  검증 자료가 아니며 다른 명시적 페이지 나누기 등 미지원 입력을 제거한 통과로 보고하지 않는다.

## SHA-256

| 파일 | SHA-256 |
| --- | --- |
| input.hwpx | `39898d7cbd27b97af411da02c01d9ba78ff1b53f0af5d09fc45d357b91a4dafc` |
| grid-saved.hwp | `8c8aebff3585ac4d26c981714a08def3a8cf466ae1b3cbe2f7c15b69f6aa7663` |
| grid-2020.pdf | `a939bcebcc329521e79558e953f5aef5f82a1d4345a22508ff2c22657b4e96fb` |
