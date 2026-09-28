# #7353 셀 시작의 단 정의와 TAC 그림

## 생성 출처

`issue6601/36331407_side_by_side_tac_tables.hwpx`의 두 번째 본문 문단 안에
`ColumnDef + Picture`가 선언된 중첩 셀이 있다. 그림은7087×7087HU, 글자처럼 취급이다.
원본의 그림·단 정의·리소스를 `issue7353_tac_noop_review/noop-saved.hwp`의
부모/자식 표와 앞뒤 문단에 옮긴 **독립 대조군**이다. 원본을 고친 문서가 아니다.

`create.rs`는 원래 그림·crop과 단 슬롯 순서를 보존하고 문단/글자 스타일 ID만
대조군에 맞춰 지정한다. 자식 높이를10000HU, 부모를24000HU로 바꾸고 모든 LineSeg를
비워 입력 HWPX를 생성한다. 한컴이 정상 저장한 HWP와 그 파일에서 인쇄한 PDF를 비교한다.
저장 후 줄 좌표나 스타일을 수정하지 않았다. 표 위 영어 제목은 재사용한 대조군의 제목이다.

- HWP job: `09ea18b4-03c7-47da-9408-cf49ff3ef113`
- PDF job: `20edc761-05d3-4e9e-95a2-7f04b250a559`
- Hancom11.0.0.9136, engine2020, managed-direct-dll-host, input_preprocess none
- font_scope session0 verified, mapped/registered2, failed0; PDF1쪽

| 파일 | SHA256 |
| --- | --- |
| picture-input.hwpx | 18f096fbeb606213e5bbd55c79b5265c1f37f2a6cd798ea3666454135036a3de |
| picture-saved.hwp | d8ad612d579e1203fdde5db4830a877f0df157a8b516946ec5ded9870caf9e29 |
| picture-2020.pdf | 4ec956e23550c5a6d3e49a6bd5f2218e38f2d305caf905ade211d70613652fe2 |

## 독립 기대값과 검사 범위

부모 원점(3969,8787), 크기32000×24000HU, 안여백283HU.
자식 carrier의 저장 vpos1760HU로 자식 원점은(7968,10830), 크기24000×10000HU다.
그림은 자식 안여백283HU 후 시작하며,23432HU 줄에7087HU 그림을 가운데 정렬한다.
따라서 원점은(16423.5,11113), 크기7087×7087HU다.
CELL BEFORE y9070, CELL AFTER y21490, 뒤 본문 y33354HU가 보존되어야 한다.

독립 한컴 PDF trace의 그림 stripe 합집합은
`x164.076/y111.060/width70.712/height70.762pt`다. PDF의 text device transform
`(.119851,.119935)`를 `.12pt` 장치 단위 대비 인쇄 축척으로 적용하고 양자화 오차0.25pt
이내인지 검사한다. PNG/overlay에는 좌표 보정을 하지 않는다.

`issue_7353_table_v2_export`는 정상 저장 HWP와 그 IR의 HWPX 직렬화본을96/144DPI로
조판하여 그림·부모·자식·앞뒤 문단의 실제 최종 좌표,1회 출력과 종료를 검사한다.
별도 합성 계약은 한 줄/두 줄의 두 그림에서 단 슬롯 추가 전후 실제 render tree가 같은지,
다단 정의가 여전히 거부되는지를 검사한다. 합성 계약을 한컴 출력 증거로 대체하지 않는다.

원본 표지 전체는 별도 페이지 배치 차이가 남는다. 이 대조군 통과는 원본 전체 통과나
다단·텍스트와 그림 혼합·floating 그림·편집 후 재조판의 지원을 뜻하지 않는다.
