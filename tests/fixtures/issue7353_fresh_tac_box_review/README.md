# 저장 줄 없는 본문 TAC의 실제 자식 높이

## 출처와 독립 출력

`create.rs`는 기존 한컴 정상 저장 대조군
`../issue7353_tac_noop_review/noop-saved.hwp`의 스타일과 단순 표를 재사용한다.
중첩 표를 본문으로 옮기고 BEFORE TABLES → 두 TAC 표 → AFTER TABLES로 구성한다.
모든 LineSeg를 지워 한컴과 rhwp가 각각 재조판한다. 저장 줄을 손으로 만들지 않는다.
이것은 편집 후 재조판 계약이며, 원본 #2470 전체 피델리티의 대용이 아니다.

두 표의 common.height는 동일한11565HU지만 셀 최소 높이는2700/5400HU다.
폭은 각각15000HU이고 내용은 SHORT TABLE / TALL TABLE이다. 한컴도 선언 높이로
표를 늘리지 않고 실제 셀 높이36/72px로 배치했다. 더 짧은 표는 기준선 정렬 때문에
아래에서 시작한다. 부모 셀 없이 본문에 직접 들어가는 TAC 경로를 검증한다.

MCP `engine:2020`, job `9a3a3282-a877-4cf6-bdf8-cecd8cc2b662`로 **carrier.hwpx 자체**를
PDF 출력했다. 별도 재저장본이 아니다. 한컴11.0.0.9136, 물리1쪽, 전처리none.

| 파일 | SHA256 |
| --- | --- |
| carrier.hwpx | e6500c0c5553f399f646b96d288d550a41693621632392974a3d7bfed6e70b69 |
| carrier-2020.pdf | 25cba4895540d0a2bc34c5b474ddaf0b69cecdacb5fd94fd8675d68cdd10dd7f |

## 독립 기대값과 검사

PDF MediaBox419×595pt, `mutool draw -F trace`의 실제 테두리 중심선:

| 표 | 왼쪽x(pt) | 위y(pt, 페이지 위 기준) | 아래y(pt) |
| --- | ---: | ---: | ---: |
| SHORT | 59.686 | 97.148 | 124.133 |
| TALL | 209.5 | 74.240 | 128.211 |

AFTER TABLES의 글자 기준선은 PDF 변환행렬의1202×0.119935pt다.
정식 `issue_7353_table_v2_document_flow`는 원본 셀 최소 높이36/72px,
최종 테두리·뒤 문단 기준선을 위 PDF에96dpi 기준1px 이내로 대조한다.
출력 영상에는 정렬·축척 보정을 하지 않는다. 내용의 순서·한 번씩 표시·세션 종료도 검사한다.
합성 HWP는 오래된 선언 높이의 확대/축소, 명시적 개행, 너비 부족에 따른 개행과
페이지 소유를 따로 검사한다. 합성 검사를 독립 한컴 출력이라고 하지 않는다.

변경 전 Native 실행 파일은 동일 carrier.hwpx의 문단1에서
`TAC content changed stored occupied box`로 거부했고 변경 후는1쪽으로 완료한다.
전후 실행 로그·PDF trace·Native/fresh WASM compare/standalone overlay/review와
source SHA manifest는 `output/7353/r19/tac-box/`에 있다.

## 범위와 한계

본문 fresh TAC만 실제 prepared child plan의 높이를 소비한다. 저장 LineSeg가 있는
호스트의 불일치 거부는 유지하며, 셀 내부 fresh TAC 어댑터의 확장은 이번 범위가 아니다.
글꼴 외형과 미세한 인쇄 축척/래스터 차이는 남는다. 테스트 통과가 메인테이너의
시각 판정을 대체하지 않는다. Legacy 기본 경로와 Studio 편집 엔진은 변경하지 않는다.
