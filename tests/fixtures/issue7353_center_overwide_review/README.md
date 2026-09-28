# #7353 줄보다 넓은 가운데 정렬 TAC

## 생성과 독립 기준

`create.rs`는 기존 정상 `issue7353_tac_noop_review/noop-saved.hwp`의 문단·부모 셀을
보존하고 자식 표 너비31500HU, 좌우 바깥여백100HU, host 가운데 정렬을 지정한다.
변경된 host/child 저장 줄은 지워 한컴이 새로 구성하도록 한다. 생성 HWPX를 한컴으로
저장한 HWP와 그 HWP의 PDF를 비교한다. #6601 원본을 대체하지 않는다.

- HWP job: `2c262f80-d6ff-4bbe-8a43-c799dba9432c`
- PDF job: `73cd9814-6145-4668-ae67-c7eeab0e66ac`
- engine2020, Hancom11.0.0.9136, managed-direct-dll-host, input_preprocess none
- fonts session0 verified(mapped/registered2, failed0), PDF1쪽

| 파일 | SHA256 |
| --- | --- |
| contained-input.hwpx | bee9d5470b20d94d377343598c6cefd8c30948d96267680bee3e0377b1e95301 |
| contained-saved.hwp | 5ede1c3bda64dc625e096a86b0924e6ffcccb14e78607c24930cdd93ad8f8255 |
| contained-2020.pdf | e3de29b0573d7eac5cb7a6f9b2787f1a99c3c84fedaff3de3f30e339fd299d8d |

## 관측과 계약

저장 줄 폭31432HU보다 자식 표의 바깥여백 포함 폭31700HU가 크다. 한컴 PDF에서
부모 왼쪽39.671pt, 자식 왼쪽43.506pt/오른쪽358.116pt, 부모 오른쪽359.314pt다.
음수 중앙 offset을 적용하지 않고 부모 왼쪽 + 안여백283HU + 표 바깥여백100HU에서
시작하며, 자식 표 폭31500HU를 보존한다. PDF trace의 인쇄 축척과 단위 양자화는
PNG에 역보정하지 않았다.

부모 y8787HU/높이18000HU, 자식 y10830HU/높이5000HU,
INLINE CHILD TABLE y11113HU, CELL AFTER y16490HU, 뒤 본문 y27354HU.
정식 `issue_7353_table_v2_document_flow`에서 무수정 HWP와 HWPX roundtrip의 실제
표 좌표·높이·폭·문구1회 보존 및 뒤 문단을 검사한다.

합성 경계는 부모32000−왼쪽 안여백283−자식 왼쪽 바깥여백100=31617HU에서
자식 오른쪽과 부모 오른쪽이 일치하는지 검사한다. 1HU 더 큰 자식은 아직 미지원인
부모 외곽 clipping 경로로 거부한다. 합성 변경본의 한컴 피델리티를 주장하지 않는다.
여러 표의 초과폭, 선행 공백, 후행 공백만 넘치는 행도 기존 거부 계약을 유지한다.

이번 자료는 저장된 단일 TAC의 가운데 정렬/셀 안여백 영역 점유 계약이다.
fresh overwide 재조판, 부모 외곽을 넘어선 자식의 clipping, #6601 전체 출력은 미완료다.
