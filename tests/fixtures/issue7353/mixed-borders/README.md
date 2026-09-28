# 실선–이중선 내부 T/십자 접합

원본 전체 통과 자료가 아니라 두 접합 규칙을 독립적으로 확인하는 정상 한컴 저장본이다.
원본 `samples/86712_regulatory_analysis.hwp` s0/p13의 r28–29/c4 경계에는
0.5mm 검정 세로 이중선과 0.12mm 검정 가로 실선이 만난다. 같은 원본 첫14문단의
정상 저장 PDF3쪽 `14.비용감축제`에서 관측한 형태를 두 방향으로 검증한다.

## 생성과 출처

기존 `../../issue7353_double_review/grid-7-input.hwpx`의 부모/자식 그리드와
글자·폭·높이·뒤 문단을 보존했다. 2×2 자식 표의 외곽과 나머지 내부선은 실선으로,
중앙 경계만 이중선으로 지정했다. 세로형 실선은0.12mm, 가로형은0.5mm이며
이중선은 모두0.5mm다. 재귀적으로 저장 LineSeg를 지운 뒤 한컴이 새로 저장했다.
입력 HWPX도 함께 보존하며 수동 저장 LineSeg는 주입하지 않았다.

생성기: [create.rs](create.rs). 저장소 루트에서 현재 rhwp 라이브러리에 연결해 실행하며
출력 경로는 `output/7353/r19/mixed-borders/`다. 기존 입력을 덮어쓰지 않는다.
`mcp_hwp2024Convert_usage.md` 절차로 HWPX→HWP→해당 HWP의 PDF를 각각
`engine2020`/전처리none으로 변환했다. 관측 한컴 버전은11.0.0.9136이다.

| 자료 | HWP job | PDF job |
| --- | --- | --- |
| vertical-inner | ccaa0cce-4ed6-467b-9b22-39e7f88e9040 | 3043a6eb-f436-4fbd-8c6b-0faf5a5b8de1 |
| horizontal-inner | 80f3525d-13f5-4787-a6a8-a78d41c17254 | aa26d6d4-1da3-4ccf-a1a6-5eb3b27f1661 |

## 독립 관측과 계약

PDF path는 자식 표 원점(68.123pt, 841−765.962pt), 각 셀15000×4000HU에
대응한다. 세로 이중선의 두 pen은 x217.443/218.522pt, 굵기0.36pt다.
가로 실선0.36pt 근처에서 pen이 끝나고 실선은 이중선 사이를 관통한다.
가로형은 실선1.439pt와 이중선0.36pt이며 중심 간격1.079pt다.
한컴 PDF600dpi 좌표 반올림과 겹쳐 그린 선의 비가시 구간을 고려하여, rhwp는
실제 공통 실선 폭의 절반에서 double pen을 끝낸다. line ink의 외형이 기준이다.
표 원점/셀 크기/부모 높이/후속 문단은 변경하지 않는다.

정식 검사는 `tests/cases/issue_7353_table_v2_borders.rs`에 있다. 실제 최종
LineNode의 시작·끝·pen 간격, 교차 실선의 연속성, 표 bbox와 내용의 소유를 검사한다.
반복 제목/분할의 동일 기하 계약과 색 충돌 거부도 별도로 실행한다.
한쪽만 실선인 바깥 모서리, 서로 다른 색/실선 폭 접합, 다른 굵기 이중선 우선순위는
이 자료로 수용하지 않는다. 기존 명시적 미지원과 별도 접합 검증이 필요하다.

## SHA256

| 파일 | SHA256 |
| --- | --- |
| vertical-inner-input.hwpx | 443e302221edbb45aa3611400774d53d913be4ef0fa2377392f186547781fea4 |
| vertical-inner-saved.hwp | 9d92bf1f9caa33b23e281de305c15ceef4394946db6f05d50c5cebb0b16296b2 |
| vertical-inner-2020.pdf | 1286fe4e6dae87dd998850db08b1674fbaa0afb32999e30308d9a67591f8e4a4 |
| horizontal-inner-input.hwpx | a7de2148b7c795f7247a789eed91d311e11b50ef88cffa5c6830e12bb7dba3ff |
| horizontal-inner-saved.hwp | 9024c6d6a8bcc108d78afb7be9526d3d5bc55c03d6f704aa2403404d5b769ea1 |
| horizontal-inner-2020.pdf | de33c2222af738a13365d84e28ab4906ac8f19ec0c89ddd423a545151420202e |
