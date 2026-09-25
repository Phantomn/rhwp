# #7353 영역 배경·테두리 독립 대조

`zone-lines-saved.hwp`는 수동 LineSeg를 넣은 입력이 아니라, `zone-lines-input.hwpx`를
한컴에 열어 저장한 파일 그대로다. `zone-lines-2020.pdf`는 그 HWP를 한컴에서 출력했다.
원본 #6923 문서가 아니며 이 대조군의 통과가 원본 전체 통과를 뜻하지 않는다.

## 생성과 출처

- `create.rs`: 기존 승인 anchor-review-saved 문서의 페이지·문단 스타일을 재사용하고
  텍스트·표를 구성한 HWPX 생성기. 입력에는 저장 LineSeg가 없다.
- 본문 2열24행, 행 높이2326HU, 표 폭32000HU, 마지막 행은2열 병합.
- 영역은 셀 주소 `(1,0)..(23,0)`. 끝 셀의 병합 폭까지 포함하는 파란 면/빨간 외곽이다.
  셀 `(2,0)`은 자체 노란 면, 다른 셀 면은 없음. 셀 자체 선은 검정.
- 한컴은1쪽1~19행,2쪽20~24행+후속 문단. 영역 배경 위에 셀 면이 그려지고
  영역 외곽은 셀 선을 덮어쓴다.2쪽에도 영역의 네 변이 닫힌다.
- 저장 job `7722cc21-c627-499a-b04d-5d090a25638d`, PDF job
  `1262cdbf-259d-45c4-a35f-43734353dc7f`.
- `engine=2020`, Hancom11.0.0.9136, direct-DLL32bit, preprocess=none,
  fonts verified2/failed0, PDF printMethod0(one-up), PDF2쪽.

| 파일 | SHA-256 |
| --- | --- |
| zone-lines-input.hwpx | `4b6bda78096f52896ace5ea3f7446526f6fd8619e184e2679c33c12714021db3` |
| zone-lines-saved.hwp | `783366c2f8970615a0214e2a153c916f172a9e5d58be2ff307eaa6eaa7c50229` |
| zone-lines-2020.pdf | `28981a7b3ca34603709f584fcc7ff04a4963bfb626057ec3b3f4e8669a599383` |

생성기는 repository root에서 rhwp 라이브러리에 `rustc --edition=2021`로 링크해 실행한다.
출력 디렉터리 `output/7353/r19/zones`를 먼저 만든다. 이후 canonical
`mydocs/manual/mcp_hwp2024Convert_usage.md`의 비동기 start/status/download로
HWPX→HWP→PDF를 순서대로 실행한다. 저장된 HWP의 내용이나 LineSeg를 다시 손대지 않는다.
실제 최종 검증 명령과 PNG는 `mydocs/working/task_m100_7353_stage19.md`의 영역 절편을 따른다.

## 미통과 진단 대조도 보존

`diagnostic/`의8행·높이7000HU 문서는 먼저 생성한 별도 대조다(`create --tall`).
한컴은1쪽7행의 빈 물리 공간을 잘라2쪽에서 이어받지만 V2 RowBreak는6행/2행으로
나눈다. 이것은 이번 영역 장식 구현의 시각 통과 자료가 아니며 높이/분할 규칙의 남은 차이다.
이 입력을 폐기하거나24행 대조 통과를 이 입력의 통과로 보고하지 않는다.
24행 대조는 이미 승인된2326HU 줄 프레임을 사용해 영역 장식의 범위를 분리한 것이다.

진단 문서에서 영역만 제거한 별도 입력을 이전/현재 V2로 렌더한 SVG는2쪽 모두 같다.
이는 높이 차이가 영역 paint 변경으로 발생하지 않았다는 대조이며, 수정한 진단 입력을
그대로의 원본/한컴 출력 일치로 승격하지 않는다.

- 저장 job `20d7006f-325c-430d-a9bb-131b5b389677`, PDF job
  `6f5b43ae-4072-4a5c-bd80-1abed84f3c06`; 동일 engine/version/font/preprocess/print 설정.
- HWP SHA `d5b76ac6f3418806a1768730088a88c569d7e1d8a916e7acd7e5edaa88d48474`.
- PDF SHA `d01c98f432d6ecfb224d60724847d3834688f2d525767137ab685d1821555627`.
- HWPX SHA `d8eb2ed110cc66ea216c97c34de938fc7df19f338be6d7e9386bc7923eda0d24`.

겹치거나 맞닿는 복수 영역의 우선순위, 병합 셀을 가르는 영역, 분리된 물리 영역,
gradient/패턴/이미지/대각선은 이번 V2 수용 범위가 아니다. 폰트 외형 차이 자체는
이번 조판 실패로 세지 않고 유효한 저장 줄바꿈·경계·소유 내용을 검사한다.
