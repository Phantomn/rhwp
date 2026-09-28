# #7353 셀 초기 단 정의 + 자리차지 자식 표의 페이지 경계

## 입력/기준 출처

`samples/86712_regulatory_analysis.hwp`를 파싱하여 첫 구역의 첫163문단을
보존하고 뒤 문단만 제외했다. 표/문단/단 속성이나 LineSeg는 수동으로 고치지 않았다.
생성 명령의 소스는 `output/7353/r19/nested-next/generate.rs`다.
HWPX 부분본을 한컴 MCP `engine:2020`에서 정상 HWP 저장한 뒤, **그 저장 HWP**를
같은 프로파일로 PDF 변환했다. 원본 전체의 출력 또는 미지원 범위 통과 증거가 아니다.

- 정상 저장 job: `281c6511-d465-4c40-8d3a-592e4491bd0d`
- PDF job: `fe79d4b4-ce6e-4bfa-a31e-59317b0f2aa2`
- 두 job succeeded, Hancom11.0.0.9136, input_preprocess:none, PDF26쪽.
- 영수증: 위 output의 `save-*`/`pdf-*` JSON. 서버 주소·인증 정보는 포함하지 않는다.

SHA-256:

- 원본 HWP: `ee82c7755617003cb972ba398da9cffadfed24ac0fa068eee1a5347da7658a88`
- 생성 prefix163.hwpx: `f4fb0c48a9eb940648804c9bf8e5bde94ef9db3a89bec14fbebbad75a57cb134`
- prefix163-saved.hwp: `3386067974071c35aa5244b143b7f32d9abd11e1ad601c959ea51de7aac9a933`
- prefix163-2020.pdf: `6dff0df026f6a0460fbc0d710a7cb442da6f40162c15b2062868ec8c2a626f13`

## 독립 관측 및 기대값

본문p161의7x2 표, 마지막 `근거설명` 셀에1x1 자리차지 자식이 있다.
셀p0는 `[ColumnDef,Table]`, 빈 저장 host 높이1500HU/간격300HU/폭0이다.
자식은 ParaTop/ColumnLeft, 모든 offset/margin0, TopAndBottom, non-TAC다.
일반1단이 셀의 로컬 영역을 설정하므로 control1을 보존하여 같은 셀 원점에 놓는다.
명시적 셀 단 정의 없는 Column 참조를 일반화하여 허용하지 않는다.

정상 저장본의 자식 마지막 문단은 첫 줄v37000, 둘째 줄v0이다.
PDF25쪽에는 `지정 전까지`까지, PDF26쪽에는
`각 주민대표단별로 약 1년간 운영하는 것으로 가정`만 이어진다.
`근거설명` 제목은25쪽 조각의 세로 가운데에 한 번만 표시된다.
자식 뒤에는1300HU 높이의 실제 빈 문단이 남고 부모 표가 그 뒤에서 끝난다.

`mutool draw -F trace ... 25-26`의 독립 부모 표 path 좌표(pt, y축 반전)는:

| PDF쪽 | 위쪽 y | 아래쪽 y |
| --- | --- | --- |
| 25 | 660.837 | 73.839 |
| 26 | 782.864 | 750.619 |

PDF paper height841pt에서 `(841-y)*4/3`으로96dpi 좌표를 얻는다.
인쇄 텍스트 변환의 세로 계수는`.119869`(명목`.12`)이므로 실제 stroke 비교에는
이 상대 오차와0.2px 프린터 양자화 범위만 허용한다. 구현이 계산한 높이를
그대로 PDF 기대값으로 재인용하지 않는다. PDF26쪽 글자 기준선은600*.119869pt다.
정확한 여백 관계는 독립 입력의 부모 상하223HU, 자식 상하141HU로 검사한다.

## 증거와 한계

정식 회귀: `tests/cases/issue_7353_local_column_anchor.rs`.
실제 공개 DocumentV2 진입, 최종 render tree의 외곽/텍스트/빈 줄/유닛 보존을 검사한다.
합성 경계는 별도로 1/2단계 자식, 여백 포함 최소 예산 미달, intact/Never,
동반 셀 Center/Bottom 정렬을 검사하며 추가 한컴 정상 생성본이라고 주장하지 않는다.

이전 `body-next/probe`는 동일 HWP를p161 속성 제한으로 거부한다.
이전 binary SHA256은 `1f90fd6bc34e8a94033afd24973df51147b881e3de0970f9e16503a9ce3b88cd`다.
단 슬롯만 연결한 중간본은 잘못된 전체 행 이월을 직접 Native 시각 비교로 검출했다.
그 자료는 `output/7353/r19/nested-next/anchor-only-review/`에 보존했다.
최종 Native/fresh Docker WASM 자료와 명령·source 해시는 같은 output의
`review/run.json`, `source.sha256`, stage19 기록에 연결한다.

대체 글꼴 외형/굵기와 인쇄 잔차는 남는다. 다단·비0 정렬 band를 가진 컷 셀·
rowspan 내부 저장 컷 전파 등은 이 절편의 시각 검증 범위가 아니다.
원본 전체는 같은p161 자식의v100 재시작에서 `unqualified stored cell frame reset`으로
명시적으로 거부된다. 정상 재저장본의v0 컷 통과를 이 원본 입력의 통과로 바꾸지 않는다.
