# 페이지 경계를 넘는 줄간격과 문단 상대 표 앵커

#7353의 독립 한컴 정상 저장 대조군이다. 실제 문서 발췌나 원본 전체 피델리티의 증거는 아니다.

## 생성 경로와 기준

기존 `../issue7353_stored_anchor_review/anchor-review-input.hwpx`에서 용지 높이를
43000HU, host 문단만 줄간격4000%, 표를 앞 두 행으로 변경했다. 표는 나누지 않음으로
설정하고 높이를4652HU로 맞췄다. 용지 폭41953HU, 상하 여백5669HU, 표 세로 오프셋
2835HU 및 바깥 위283HU/아래567HU는 유지했다. 별도 문단 스타일을 만들어 셀과 뒤 문단의
줄간격은 바꾸지 않았다. 입력 생성 코드는 `output/7353/r19/anchor-gap-origin/generate.rs`다.

1. `anchor-input.hwpx`를 MCP engine2020에서 열어 `anchor-saved.hwp`로 정상 저장했다.
   job: `4036257a-760a-4027-ad5d-badb0aba7ea8`.
2. **그 저장 HWP 그대로** PDF를 만들었다. job: `8a33ca1a-a70a-430b-964e-12ecd34d06f4`.
   결과는 `anchor-saved-2020.pdf`2쪽이다. Hancom11.0.0.9136, 전처리 없음.
3. Native/WASM도 같은 저장 HWP를 사용한다. 저장 LineSeg를 수동 수정하지 않는다.

| 파일 | SHA256 |
| --- | --- |
| anchor-input.hwpx | `f665773a0ff4d560e77307a44a87f04a51a32a5431d1c1130b3e70516d9a4e62` |
| anchor-saved.hwp | `6de66e997888954909aa0e058713ad377043f88aff37819fbc72ce3888139d46` |
| anchor-saved-2020.pdf | `0e55c6aec850033dde7dec383131df6c9a717c772d6ee207a8c0046d185235b2` |

## 판정 지점

- 1쪽 제목은 본문 상단5669/75=75.586667px. 표 시작은 제목 기준2835HU와 바깥 위283HU를
  더한117.16px다. 제목의 긴 줄간격을 줄였다고 표가 위로 이동하거나 없어지면 안 된다.
- 표의 두 행·외곽을 보존하며 높이는4652/75=62.026667px다.
- 2쪽에는 뒤 문단이 본문 상단에서 한 번 표시된다. host나 셀 내용이 중복되지 않는다.
- PDF path의 표 위/아래는96dpi로117.130667/179.217333px다. rhwp는117.16/179.186667px이며
  차이는 각각 약0.03px다. PDF 제목 기준선88.008067px와 rhwp88.053333px도 대응한다.
  `output/7353/r19/anchor-gap-origin/pdf-trace.xml`의 실제 stroke/text에서 측정했다.
- `hancom_long_host_gap_preserves_anchor_and_next_page_prose`가 위 계약을 직접 검사한다.
  수정 전 Native와 WASM은 `InconsistentAtomicPlan`으로 첫 페이지 생성에 실패했다.

초기 실험은 세로 용지 높이24000HU가 폭41953HU보다 작아 한컴 PDF와 엔진의 용지 방향이
달라졌다. 이를 시각 통과 근거로 쓰지 않았다. 초기 입력·한컴 저장·PDF는
`output/7353/r19/anchor-gap-origin/orientation-diagnostic/`에 보존했다. 현재 대조군의
용지 모순을 바로잡으면서 경계가 계속 발동하도록 줄간격을2000%에서4000%로 변경했다.
이 변경은 fixture 생성 단계이며 production의 저장 정보 수용 조건은 완화하지 않았다.

글꼴의 폭·굵기·미세한 잉크 위치는 별도 차이로 남는다. 여기서 검증하는 것은 표의 앵커,
두 행의 보존 및 뒤 문단의 페이지 시작이다. 합성 빈 줄/분할/물리 paragraph-after 경계
계약은 정식 test 파일에 별도로 있으며 PDF 일치 근거와 혼동하지 않는다.
