# 셀 Dash 테두리 정상 저장 기준

- 원문: `samples/86712_regulatory_analysis.hwp`.
- `prefix31-saved.hwp`: 첫 section/첫31문단을 그대로 추출하여 HWPX 직렬화 → 한컴 정상 저장.
  p30의4행3열 표와 앞 문단들을 보존했다. 표·셀·LineSeg·스타일은 수동 수정하지 않았다.
- `prefix31-2020.pdf`: 위 **동일 반환 HWP**의 독립 PDF,10쪽. 대상 표는10쪽 하단.
- `control-saved.hwp`: p30 표를4회 복제하고 셀 네 면을 Dash,굵기 index0/1/7/11로 변경한
  독립 대조군의 정상 저장본. 문단/셀 내용은 유지했다. 대응 `control-2020.pdf`는1쪽.
- `pen-catalog-saved.hwp`: 같은 방법으로16개 표준 굵기 index0..15를 열거한 정상 저장본.
  대응 `pen-catalog-2020.pdf`는3쪽. 이 문서 전체의 host paragraph inset은 V2 미지원이다.
  catalog는 독립 pen 관측과 **선택 표** 검사에 사용하며, 문서 전체 조판 통과를 주장하지 않는다.
- 변환: engine2020, Hancom11.0.0.9136, 전처리none. 모두 정상 저장 HWP→PDF 순서다.

| 기준 | 정상 저장 job | PDF job |
| --- | --- | --- |
| prefix31 | c7e8ba34-f212-4414-b788-f3838ba9614d | f091ac1e-1598-422c-a5e2-daee0b7a3d84 |
| control | f97d5a6c-cb9b-4958-9c3a-8f8b8e102aa0 | a3654a27-7ddb-41de-a587-4e40995c1cd7 |
| pen-catalog | 60175b02-47b4-40ae-b96c-3af73cf703e5 | 897b34dc-9faa-42a7-9c73-37888fd684fd |

생성 코드와 job응답/해시는 `output/7353/r19/cell-dash/`에 보존했다.
최종 `control.rs`는16개 catalog 생성기이며 초기4개는 `for width in [0,1,7,11]`로
실행한 동일 생성 절차다. 입력 HW​​PX도 각각 보존했다.

## 독립 관측

`mutool draw -F trace`로 선분을 읽었다. 원본10쪽의 Dash는 약2.398pt 획/1.439pt 공백,
두 경계는 오른쪽 두 열 전체에서 연속된다. 공유 셀 선언을 그대로 두 번 칠할 필요는 없다.
불투명 동일 선분의 중복을 제거해도 보이는 결과는 같다.

표준 pen별600dpi 정수 격자(획/공백):
`17/9,20/12,26/14,34/21,43/27,52/29,69/42,86/51,104/62,121/72,173/105,259/156,347/206,520/311,693/417,866/519`.
굵기 index에 대응하는 paint 사전이며 문서 ID·셀 폭·페이지 수 조건이 아니다.
PDF 출력의 미세한 축별 축소 때문에 격자 간격은 x약0.11994pt/y약0.11988pt로 관측된다.
이를 engine 좌표 보정으로 복제하지 않으며, 선언된600dpi 물리 pen을 DPI 비례로 사용한다.
테스트는 두 DPI의 최종 line endpoints, 공유 경계 중복 제거, 마지막 부분 획과 공백,
반복 제목·중첩 분할·뒤 문단의 기존 geometry 보존을 검사한다.

색/굵기/선종류가 충돌하는 공유 경계, Dash zone perimeter, mixed Double junction은
별도 규칙 검증이 필요해 계속 명시적 미지원이다. 원본 전체 지원이나 모든 backend의
실행 검증을 의미하지 않는다. 실제 browser 검증 범위는 Native/fresh WASM DocumentV2 SVG다.
