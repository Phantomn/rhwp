# #7353 직선 대각선 대조

## 생성과 기준

`create.rs`는 앞선 영역 대조의 한컴 저장본을 읽어 대각선 속성을 지정하고 모든 수정
문단의 LineSeg를 비운 HWPX를 만든다. HWPX → 한컴 저장 HWP → 같은 HWP의 PDF 순서다.
저장 HWP/PDF는 다운로드 그대로이며 수동 줄 메트릭은 넣지 않았다.
원본 #6923을 재작성한 정답지가 아니라 독립적인 대각선 효과 대조군이다.

```sh
rustc --edition=2021 tests/fixtures/issue7353_diagonal_review/create.rs \
  --extern rhwp=/home/edward/mygithub/rhwp/target/pr-review/debug/deps/librhwp.rlib \
  -L dependency=/home/edward/mygithub/rhwp/target/pr-review/debug/deps \
  -o /tmp/rhwp-7353-diagonal-create
/tmp/rhwp-7353-diagonal-create
```

변환은 canonical MCP의 `start → status → download`를 사용했다. engine2020,
Hancom11.0.0.9136, direct DLL32bit, 전처리none, 폰트등록2/실패0,
PDF printmethod0/one-up이다.

| 입력 | HWP 작업 | PDF 작업 |
| --- | --- | --- |
| `diagonal-input.hwpx` | `41d2ce91-3306-4674-bf95-91496706ae32` | `47484484-85a9-42df-a8cd-2d288295bed9` |
| `diagonal-cell-input.hwpx` | `9d2b2a00-3597-4560-b538-1d018aed45a7` | `0c88fec7-f84a-4c79-bf04-613bad66f4a8` |

## 판정 범위

`diagonal-saved.hwp` / `diagonal-2020.pdf`:

- 1쪽1행 왼쪽 빨간 `/`(attr8),2행 파란 `\`(attr64),3행 초록 `X`(attr72).
- 4행은 방향만,5행은 선 종류만 있어 대각선이 없어야 한다.
- 18~21행 두 열에 걸친 영역 대각선은1쪽18~19행 사각형과2쪽20~21행 사각형에서 각각 다시 그린다.
- 24행 병합 셀은 두 열 전체 폭의 대각선을 그린다. 후속 본문이 유지된다.
- 독립 좌표: 표x3969HU, 폭32000HU, 행높이2326HU, 첫 쪽top8787HU,
  다음 쪽top5952HU, 뒤 문단y18149HU. 96dpi에서75HU=1px.
- 사양은 `mydocs/tech/한글문서파일형식_5.0_revision1.3.md` 표24의 방향 비트를 참조한다.
  방향·분할 시 paint 범위는 위 PDF로 별도 확인한다.

## 미검증/불일치 대조 보존

`diagonal-cell-saved.hwp` / `diagonal-cell-2020.pdf`는 긴1×1 셀에40개 문단과
대각선을 넣은 **미통과 진단**이다. 한컴 PDF는 제목/뒤 문단을1쪽에 두고,
표를2쪽으로 넘기며 내용이 아래 용지 밖까지 이어진다. 최초 V2 진단은 셀을 두 쪽에
분할했다. 이 출력을 정상적인 셀 내부 분할 정답으로 인정하지 않는다.
이번 V2는 셀 내부 분할 정책에서 활성 셀/영역 대각선을 명시적으로 거부한다.
온전한 셀·행 단위 분할 영역의 성공을 이 진단의 통과로 바꾸지 않는다.

전체 표 배경의 대각선, 꺾은선, 중심선, 다중 ray, 비실선 pen, 셀/영역의 중첩 대각선
우선순위도 아직 수용하지 않는다. Legacy 기본 경로는 변경하지 않는다.

## 다운로드 파일 고정

| 파일 | SHA-256 |
| --- | --- |
| diagonal-input.hwpx | `f4fa97a60d9dacd915175e8f89a81dd30d73b384ae62f635edbc6e53b4b7d4a4` |
| diagonal-saved.hwp | `44d720b1543cf913f8cefe4c75983a42c3fef539f7dd8c34e367c66f067d7243` |
| diagonal-2020.pdf | `d9c5fd2e45e1bd8acee30d9d932f7468201a1c42be8920a74afacbe6f336e383` |
| diagonal-cell-input.hwpx | `e1db50a6fefe15030f2a9373154687c637d492825b603d607efb0932553a7ab2` |
| diagonal-cell-saved.hwp | `1132da53e4cd8c37cb77361720bd58dcf91e9eb894b23d4ae908a05dd4c60eca` |
| diagonal-cell-2020.pdf | `b05481d3258059777a0b1a01eec788a40c0f59ce9a440b4126d58d001385e638` |
