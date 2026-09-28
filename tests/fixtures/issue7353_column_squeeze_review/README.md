# 셀 첫 문단의 1단 정의와 SQUEEZE

## 생성과 독립 기준

`create.rs`는 이전 정상 저장 `issue7353_squeeze_review/squeeze-saved.hwp`의 자식 셀
첫 문단에 정상1단 ColumnDef만 추가한다. source 슬롯8HU가 아니라 **UTF-16 위치8개**를
보존하며 텍스트·셀 폭·높이·안여백은 바꾸지 않는다. LineSeg를 지운 HWPX를 한컴에서
정상 HWP로 저장하고 그 HWP의 PDF를 만든다. 저장 후 좌표 조작은 없다.

- HWP job: `be930bb0-c5e3-4ae3-a4c1-75acaa519a9a`
- PDF job: `ec97cc6e-afc7-48c1-891c-f359f6ddd4a7`
- engine2020 / Hancom11.0.0.9136, managed-direct-dll-host, preprocess none
- PDF1쪽, Hancom PDF driver one-up, font scope verified
- 입력 SHA256: `e9252314c263a93f5122f5ac796b99731ee15c75fddc49354fd2c0c369ed867d`
- HWP SHA256: `58cfaab25b0e5d63093c566585f7ea189ebfbc107f6e951f7f4e286c562cf880`
- PDF SHA256: `cf3b0b1fa9a99c32d39b555979e454da8f1894be3bfe93158e3bd88157c4db75`

정상 저장에도 첫 문단의 ColumnDef와 SQUEEZE 설정은 남는다. 한컴 PDF를96DPI로
래스터화하면 1단 정의가 없던 기존 대조군 PDF와 동일하다(양쪽 PNG SHA256
`d60f191789e9f0499d99c71edf2d349739072aad009c2bd9ff248b843ba2ca7c`).
이 자동 비교만으로 시각 판정을 대신하지 않으며 Native/WASM review도 직접 확인한다.

## 기대와 범위

원래 셀10000HU, 좌우 안여백283HU, 저장 줄 폭9432HU와 한 줄 문구
`ONE TWO THREE FOUR FIVE`를 유지한다. 자식 y10830HU/높이5000HU,
글줄 y11113HU, CELL AFTER y16490HU, 뒤 본문 y27354HU도 동일하다.
ColumnDef는 이 셀의 단을 정의하며 인라인 개체처럼 공간을 차지하지 않는다.

정식 `issue_7353_table_v2_document_flow`에서 정상 HWP/HWPX의 전체 노드·SVG,
표 외곽·앞뒤 문단·종료를 기존 독립 저장본과 비교한다. 합성 source 슬롯 추가 전후도
전체 출력이 같아야 하고 다단 선언·저장 줄 없는 SQUEEZE는 계속 거부한다.
기존 dirty/KEEP/미지 wrap/세로쓰기 거부 계약은 유지한다.

## 시각 판정 한계

기존 SQUEEZE의 글리프 축소·자간 외형은 한컴 PDF와 다르며 이번 변경은 이를 고치지 않는다.
한 줄 유지와 표/문단 기하를 이번 절편의 판정 범위로 구분한다. 이 대조군은 #2470
마스킹 원본의 수정본이나 원본 전체 일치 증거가 아니다. 편집 후 fresh SQUEEZE 조판,
개체 혼합, Studio Canvas 편집은 이번에 입증하지 않는다.
