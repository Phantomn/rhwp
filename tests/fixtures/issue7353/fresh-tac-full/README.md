# fresh TAC 재조판과 부모 표 분할의 전체 문서 대조군

작업지시자가2026-09-27 시각 통과한 **파생 입력**이다. 원본 #6923의 전체 피델리티
정답지나 모든 표의 새 baseline이 아니다. 기존 기대값·ignore·래칫은 변경하지 않는다.

## 출처와 재생성

1. 원본은 `../../issue6923/148738070_wrapper_table_stored_page_frame.hwp`다.
2. 공통 IR의 `sections[0].paragraphs[5].controls[0]` 부모 표 첫 셀 높이를
   189665→1000HU로 변경하고 해당 셀의 직접 문단들에서 `line_segs`를 비운다.
3. 본문 문단29의 `line_segs`도 비운 뒤 HWPX로 직렬화한다. 다른 컨트롤을 제거하지 않는다.
   이 HWPX가 `refreshed-input.hwpx`다. 이전의 오래된 캐시를 남긴 실패 대조군은
   로컬 `output/7353/r19/frame-band/original-small-*`에 보존되어 있다.
4. MCP2020 profile에서 이 HWPX를 HWP로 정상 저장한다.
   job `2de06ae8-4829-41c3-8690-289cf50b7bdd` → `refreshed-saved.hwp`.
5. **그 HWP 자체**를 동일 profile로 PDF 출력한다.
   job `4d0b9fd5-d49f-442a-b163-3b5b104bb08f` → `refreshed-2020.pdf`.
   한컴11.0.0.9136, PDF1.4, MediaBox595×841pt, 물리7쪽.

| 자산 | SHA-256 |
| --- | --- |
| `refreshed-saved.hwp` | `043dc40710329fc1c599ba1986f480a617a8a62cc82a0cd0f6e98e035eb041c3` |
| `refreshed-2020.pdf` | `f4656bc71f271f89dfd4da04d295657aee2064b1c8e81e1789e33690c22d9f61` |

재저장 시 두 표의 바깥여백−1HU가0HU로 정규화되었다.4쪽 마지막 표의 호스트 줄 높이는
14845→14847HU이며 본문 문단29는 저장 줄이 없는 상태다. 이 차이를 원본 입력과
동일하다고 취급하지 않는다. `cell_end_policy=omit_final_paragraph_gap` 선택이 필요하다.

## 독립 기대값과 정식 검사

`tests/cases/issue_7353_fresh_tac_full_document.rs`는 다음을 검사한다.

- PDF의 부모 표는1–5쪽에만 존재하고 본문29번 TAC는6쪽에 한 번 배치된다.
- `mutool draw -F trace refreshed-2020.pdf`에서 추출한 부모 표 하단은96dpi에서
  998.588 / 1017.6066667 / 1008.976 / 1010.2546667 / 832.3693333px다.
  PDF841pt와 입력 용지84188HU의 축척을 숫자 비교에만 적용한다.1픽셀 이내 비교는
  96dpi 판정 해상도의 계약이며 subpixel 완전 일치 주장이 아니다. 출력은 보정하지 않는다.
- 부모 조각은 본문 영역 안에 있고 모든 자식 표를 감싼다.
- TAC의 폭45918HU·높이11156HU는 IR 선언값이다. 실제 배치 상자와 이 값을 대조하고
  뒤따르는 문단이 같은6쪽에서 표 아래에 유지되는지 검사한다.
- 본문 전체의 비공백·비제어 문자별 개수를 원본 IR의 재귀적 문단 내용과 대조한다.
  쪽번호 story는 제외한다. 이 검사는 내용 누락·중복의 문자 개수 축이며 모든 문단의
  순서를 입증하지 않는다. 순서는 위 후속 문단에서 별도로 검사한다.
- 종료 후 추가 페이지가 없음을 검사한다. 페이지 수만으로 성공을 판정하지 않는다.

기존 fresh TAC 구현 전 바이너리는 동일 HWP의 문단29에서 명시적으로 거부했다.
기존 거부→새 수용의 증거이지 기존 엔진의 오조판 수정 증거는 아니다.
직접 시각 증거는 stage19의 `fresh-tac-full/review` 기록을 참조한다. 글꼴·1쪽 로고
차이는 별도이며 이번 승인으로 해결했다고 간주하지 않는다.
