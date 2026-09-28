# 중첩 셀의 빈 그림 프레임과 정상 그림

## 입력과 독립 기준

`create.rs`는 `issue7353_picture_space_review/picture-saved.hwp`의 첫 그림만
빈 참조(ID0)로 바꾸고 group-local offset `(3745,-1509)`HU를 넣는다. 다른 그림,
탭, 부모/자식 표와 앞뒤 문단은 보존한다. LineSeg를 지운 입력을 한컴에 정상 저장하여
새 줄 메트릭을 생성한다. 저장 이후 좌표를 변경하지 않는다. #2470 원본은 수정하지 않았다.

- HWP job: `6cf1a859-f26c-4029-9ca7-2fb890dcf375`
- PDF job: `0c5df994-ced3-43f7-9548-9e295c14f2dd`
- engine2020 / Hancom11.0.0.9136, managed-direct-dll-host, preprocess none
- PDF 입력은 위 정상 저장 HWP이며 한컴 PDF driver one-up 출력1쪽이다.
- 입력 SHA256: `a17461c38a8ff24b59bcb523234e398825f91cae9d26732ea530e2de09de7462`
- HWP SHA256: `6a22e9f798a171ded98bdc86d5bd1b95cfd145295bc1e3eadbd46e6bef480a9c`
- PDF SHA256: `082304da77a437513ef6ce01d6eec7e7ebedb29380528e8d1ca07cb062e57684`

한컴 저장 후에도 첫 그림의ID0, 비영 offset, group_level0은 유지된다.
저장 줄은 폭23432HU, 높이7087HU, baseline6024HU이고 탭은913HU다.
새 PDF에는 왼쪽 빈 프레임의 잉크가 없지만 오른쪽 정상 그림의 위치는 변하지 않는다.
PDF trace에서 정상 그림 stripe 합집합은 `(219.448,137.20601,39.910,39.93899)`pt다.
기존 #2225의 편집기 표시/인쇄 미출력 계약과 독립 PDF가 같은 의미를 보여 준다.

## 기대값과 검사

빈 프레임 `(13967,11113,7087,7087)`HU, 정상 그림
`(21967,13736.95,4000,4000)`HU. 저장 줄의 가운데 정렬 폭과 TAC 기준선 정렬로
결정되며 이전 정상 그림2개 대조군과 같은 위치다. 부모
`(3969,8787,32000,24000)`, 자식 `(7968,10830,24000,10000)`HU,
CELL BEFORE y9070, CELL AFTER y21490, 뒤 본문 y33354HU를 유지한다.

`tests/cases/issue_7353_table_v2_export.rs`는 정상 HWP와 같은 IR의 HWPX에서
MissingPicture 노드·정상 그림·최종 좌표·표 외곽·후속 문단·종료를 검사한다.
합성 경계는24px 그림 줄이 fit하거나1px 부족해 전체가 이월되는 경우를 검사한다.
실제 데이터가 없는 nonzero 참조, 외부 경로, 회전·그룹·변환은 별도 거부를 유지한다.

## 시각 판정 범위

왼쪽 빈 그림 자리로 인해 오른쪽 그림이 왼쪽으로 당겨지지 않는지, 줄 높이·내부 표·
부모 표·뒤 문단의 위치가 유지되는지 확인한다. PNG 정합 이동은 하지 않는다.
글꼴 잉크와 PDF 인쇄 양자화 차이는 남는다. 검증은 Native와 fresh WASM DocumentV2
SVG 출력이며 Studio Canvas의 선택/그림 지정/편집 동작은 검증하지 않는다.

원본 #2470에 동봉된 로고가 보이는 PDF는 그림 참조가 비어 있는 마스킹 HWPX와
내용이 달라 이 절편의 시각 기준으로 쓰지 않았다. 대조군 통과는 원본 전체 일치가 아니다.
