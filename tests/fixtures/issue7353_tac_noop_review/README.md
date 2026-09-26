# #7353 비표시 문단 테두리 참조와 중첩 TAC

이 파일은 #6923 원본 발췌본이 아닌, 수용 조건을 확인하기 위한 읽을 수 있는 독립 대조군이다.

## 입력·기준 출력의 출처

`create.rs`가 기존 anchor 대조군에서 저장 LineSeg 없는 HWPX를 작성한다.
한컴에서 이를 정상 저장한 `noop-saved.hwp`를 수정 없이 사용하며, **그 저장본**을 한컴으로
PDF 출력한 것이 `noop-2020.pdf`다. rhwp 출력 PDF가 아니다.

- 한컴 MCP CLI `start → status → download`, engine `2020`
- Hancom `11.0.0.9136`, `hwp-managed-direct-dll-host`, 32-bit, input preprocess `none`
- 폰트 mapped/registered 2, failed 0, 검증 완료
- HWP job `3d767bc1-0b1c-4586-83ed-87d474ba3625`
- PDF job `9f279ce8-8f4c-4c5e-ba12-36cd6114d83b`, 1쪽, printMethod0(one-up)

| 파일 | SHA-256 |
| --- | --- |
| `noop-input.hwpx` | `267e5b091ef440e12d495a34c6f80a9a33cc549cdf3575087251c0d9ed39c1a7` |
| `noop-saved.hwp` | `62e36c3689f34a2e2e95db75be37e65085cac8bc5ee08064a56610528f7f3839` |
| `noop-2020.pdf` | `276f0d55a90bd7d33199b286a404a9c2fc8f06f3c6498056f370734e1106a371` |

## 확인 대상과 독립 기대값

부모 표 안의 `CELL BEFORE` 다음에 `INLINE CHILD TABLE` TAC가 있고, 이어 `CELL AFTER`가
나온다. 부모 표 밖에는 `AFTER PARENT TABLE`이 있다. TAC carrier는 borderFill1을 참조하지만
해당 문단 스타일에는 실제 선·채움이 없다. 표 자체의 외곽선은 별도 셀 서식이며 보존해야 한다.

원본 속성과 한컴 저장 좌표(HU, 1/7200 inch):

- 본문 원점 (3969,5669), 부모 표 세로 offset2835, 위 바깥여백283 → 표 y8787
- 부모 표 32000×18000(선언 최소 높이), 내부 padding283
- TAC carrier local vpos1760, 높이5000, 저장 줄 폭31432, 가운데 정렬
- 자식 표 24000×5000 → x3969+283+(31432−24000)/2, y8787+283+1760
- `CELL AFTER` local vpos7420, 뒤 본문 stored vpos21685

`tests/cases/issue_7353_table_v2_document_flow.rs`는 원본 참조·저장 줄을 확인하고
실제 표/줄 좌표·높이·순서·종료를 검사한다. 픽셀 비교에서 대체 글꼴의 폭·굵기 차이는
문단 테두리나 표 기하 차이와 구분한다. 이 자료는 원본 #6923 전체, 혼합 TAC 텍스트,
임의 들여쓰기, 셀 내부 분할 또는 그림의 한컴 피델리티를 입증하지 않는다.
