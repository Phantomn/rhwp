# 단과 쪽 경계의 빈 줄 감추기 대조군

`hide_empty_line` ON/OFF만 달리한 정상 한컴 HWP 저장본과 같은 파일의 PDF다.
페이지 끝을 넘는 빈 문단 두 개만 감추고 세 번째 빈 문단은 다음 단/쪽에서
높이를 차지하는지 검사한다. 본문 안 빈 줄을 무조건 제거하는 규칙이 아니다.

## 생성과 독립 기준

`create.rs`는 기존 fixture의 글꼴 정보를 재사용하고 본문·용지·문단 스타일을
새로 만든다. LineSeg는 작성하지 않았다. HWPX 작성 입력을 한컴으로 HWP 저장하고,
그 HWP를 PDF로 출력했다. 2026-09-30 engine2020/runtime11.0.0.9136,
전처리 없음, 각 PDF 2쪽이다. 원본 #7008 전체 지원의 증거는 아니다.

| 대조군 | HWP 저장 job | 해당 HWP의 PDF job |
| --- | --- | --- |
| on | 2c25c31f-4fb5-465d-8943-fb78ced92271 | f1f1549e-20e0-41bd-8dac-bd8101d4346a |
| off | c9edfe86-4a38-46af-b6b1-af8049e353b9 | dd72f7df-83e3-4dd1-9da0-e6ead5560be7 |
| columns-on | 89ce3efa-af3a-4487-b404-13e1c8d0e9fb | d47c90ac-6113-470e-875c-f9ddff21c0c5 |
| columns-off | 7130c4b9-062a-46c3-b8d4-9c36a1fe767a | 9f9ad297-cf49-4c00-8845-a71fbca3e61d |

변환은 `mydocs/manual/mcp_hwp2024Convert_usage.md`의 비동기
`start --target hwp|pdf --engine 2020 --output-filename ...` → `status` → `download`다.
`on/off.hwpx`, `columns-on/off.hwpx`는 생성 입력이고 `*-saved.hwp`가 판정 입력이다.

## 판독점과 기대 좌표

본문 위는8000HU, 높이12600HU다. 줄 높이1200HU/간격600HU인 일곱 줄 뒤에
서로 다른 빈 문단 세 개를 작성했다. 첫 빈 문단의 원래 시작은12600HU다.

- 단일 단 ON: 2쪽 첫 빈 문단 하나 뒤에 AFTER. 저장 시작1800HU → 용지9800HU.
- 단일 단 OFF: 2쪽 빈 문단 세 개 뒤에 AFTER. 저장 시작5400HU → 용지13400HU.
- 두 단 ON: 1쪽 오른쪽 단 AFTER는9800HU, 2쪽 왼쪽 NEXT PAGE도9800HU다.
  따라서 두 줄 감춤 한도는 물리 페이지 전체가 아니라 각 단마다 다시 시작한다.
- 두 단 OFF: 오른쪽 AFTER는13400HU, 2쪽 NEXT PAGE는17000HU다.

PDF 텍스트 잉크 위쪽은 ON AFTER/NEXT97.708934pt, OFF AFTER133.622534pt,
OFF NEXT169.536134pt다. 용지 인쇄의 세로 비율0.9976과 글꼴 잉크 상단 차이는
줄 상자 좌표와 구별한다. PNG 정렬/좌표 보정은 하지 않는다.

정식 `tests/cases/issue_7353_host_hide_empty.rs`는 위 실제 최종 줄 좌표·소유·높이와
본문 보존을 검사한다. 정확히 fit하는 줄/넘치는 뒤 간격, 명시적 쪽 나누기,
제어 소유 빈 문단, 저장 줄을 지운 재조판은 합성 경계 계약이며 별도 한컴 일치 주장이 아니다.

## 확인하지 못한 범위

한컴 HWPX 출력 `on-hancom.hwpx`와 `columns-on-hancom.hwpx`도 보존했다.
job은 각각 `db5a2691-3a74-4a74-adb6-0fea91d54198`,
`8f5f90d8-25da-40c6-8e25-4fd02ea854f8`이다. 둘 다 현재 V2에서
`stored text requires intact single-segment rows`로 거부된다.
rhwp의 HWP→HWPX 왕복에서도 같은 거부를 관측했다. HWP 저장본 통과를
이 HWPX 경로 통과로 바꾸어 보고하지 않으며 수용 조건은 완화하지 않았다.
직접 생성한 저장 줄 없는 HWPX는 공통 paragraph_layout에서 별도 panic도 발생했다.
이 입력들과 실패 로그는 후속 조사 자료이며 이번 HWP 규칙 구현으로 해결하지 않았다.
공백 문자만 있는 줄·다중 빈 개행·특수 문단 장식의 감춤은 별도 한컴 근거가 없다.

## 판정 입력 해시

```text
d552671b9dc820530b7ab741328de6087bcc57ea9bfc7c0ba3766eee3b0e785e  on-saved.hwp
21e5baf9d007104d684e7ac9f52483ae7bbf8aa60f14a419176b58fe95c429f9  off-saved.hwp
630fa063353be0843e0490dd19ed17bf18bb6b334460cb4b038d80a8dd78a00d  columns-on-saved.hwp
ce037cf838748ba850990e7e36dc2571dd7c5033a3f3234895f8fe92d0c8801b  columns-off-saved.hwp
a8ce85af1fffe3b7a886a77954c0168893f1d9db8f58703470a266599dae6077  on-2020.pdf
e1cdeec42578555a90b2218921f068b997af8559333574f84f1c82bd105e36e9  off-2020.pdf
b45871cc96d0313be5f5f76964acee05db809b673bef4b0b4ad805c1434c836a  columns-on-2020.pdf
fa3f9d7c9f73ea9da4c0533cbc5a7eaa206179a293df4f20c491cefbd16ba58d  columns-off-2020.pdf
```
