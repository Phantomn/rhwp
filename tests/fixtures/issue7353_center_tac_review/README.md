# #7353 TAC 문단 가운데 정렬 대조군

`issue7353_unequal_tac_review/review-saved.hwp`의 정상 저장본에서 carrier의
ParaShape만 복제해 세로 CENTER로 바꾸고 그 문단 LineSeg만 비웠다. 생성 코드는
`create.rs`다. `--asymmetric`은 두 자식의 위/아래 바깥여백을 추가로 변경한다.
#7008 원본의 내용을 고치거나 이 파일을 원본으로 대체하지 않는다.

HWPX → MCP engine2020 HWP 저장 → 그 저장본의 한컴 PDF 출력 순서다.
Hancom11.0.0.9136 / managed-direct-dll-host / input_preprocess none /
font_scope session0 verified, mapped2 registered2 failed0 / one_up PDF 각1쪽.

| 대조군 | HWP job | PDF job |
| --- | --- | --- |
| center | bff26e4f-a613-4ab8-ac43-fc6eb7f81424 | fe57ab2a-bb2b-4fc4-bf50-1afa0a7418c7 |
| center-asymmetric | 0e9164c0-0fa3-476d-82bd-8224b17fddd9 | 70bc6656-0dfc-4f2c-89c4-9678f68bf55d |

## 독립 기대값

공통: 부모 원점3969/8787HU, 안여백283HU, carrier 시작1760HU.
따라서 carrier의 페이지 y는10830HU다. 두 표는10000HU 너비,5000/8000HU 높이다.

| 대조군 | 작은 표 위/아래 여백 | 큰 표 위/아래 여백 | 저장 줄 높이/중심 | 작은 표/큰 표의 실제 y | CELL AFTER y |
| --- | --- | --- | --- | --- | --- |
| center | 200/200 | 200/200 | 8400/4200 | 12530/11030 | 19890 |
| asymmetric | 400/100 | 100/600 | 8700/4350 | 12830/10930 | 20190 |

CENTER는 표 본체가 아니라 **바깥여백 포함 상자**의 중심을 맞춘다.
비대칭 대조군에서 작은 표 위는 `(8700-5500)/2+400=2000HU`다.
본체 중심을 맞추면1600HU가 되어 독립 PDF와4pt 차이가 난다.
부모 높이18000HU와 뒤 본문 y27354HU는 두 경우 모두 유지된다.

`mutool draw -F trace`의 clip을 페이지 위 기준pt로 변환한 값:

| 대조군/표 | x | y | width | height |
| --- | ---: | ---: | ---: | ---: |
| center/작은 표 | 97.559 | 125.213 | 99.956 | 49.893 |
| center/큰 표 | 201.470 | 110.221 | 99.836 | 79.877 |
| asymmetric/작은 표 | 97.559 | 128.211 | 99.956 | 49.893 |
| asymmetric/큰 표 | 201.470 | 109.141 | 99.836 | 79.997 |

텍스트 transform의 인쇄 축척 x=.119851/.12, y=.119935/.12를 적용한 뒤
장치 양자화0.25pt 이내를 검사한다. PNG에는 축척 보정을 하지 않는다.
원본 저장 줄 HU 계약과96/192dpi의 실제 RenderTree 좌표는1e-8px 이내다.
수동 생성한 잘못된 baseline 캐시는 별도 거부 반례이며 정상 저장본으로 취급하지 않는다.
CENTER 재조판, TOP/BOTTOM TAC 문단 정렬, 혼합 보이는 텍스트와 #7008 전체 문서는
이 대조군의 지원/시각 일치 주장 범위가 아니다.

## SHA256

- center-input.hwpx: `5841d423047eab03e51980f4e8ff23099404ba05961a9ca3d2e94270da71d283`
- center-asymmetric-input.hwpx: `5f7304dc6c1909af49cac79c11840c07917e0e35e240cf6596bc5cb9440ac1bc`
- center-saved.hwp: `177307afe45c6580cde417fe720e955f4c490dce5364db410a690dd44009dd0c`
- center-2020.pdf: `fe41912b76a1aa20ab2c94d6a446366e25a275ef03ba094f2d5905ae7af3b971`
- center-asymmetric-saved.hwp: `5ba23fccaa5836d962f235599410dcedf7ba40197c5a2c4c6aa5e76ee50f6f50`
- center-asymmetric-2020.pdf: `1fd738c2901018ef3e9070a1a61e327a60cd3d38af8f817f575711fc12a13369`
