# PR #7482 공통 TAC 줄 배치 시각 증적

코드 source: `d86b35184ab50deefbe999125c532422cdb8a3c4`. Native와 root wrapper로 생성한 fresh WASM을 동일 입력/PDF·96dpi·검증된 글꼴 공급으로 출력했다. `provenance.json`은 각 PNG와 실행 산출물 해시를 고정한다. 이 폴더는 공개 fixture의 PNG만 포함한다.

- `tac-mixed-one-row`: `samples/issue7482/tac-mixed-one-row.hwpx`, `pdf/issue7482/tac-mixed-one-row-hwpx-2020.pdf`; B/첫 표/A/둘째 표/C가 같은 줄에 순서대로 놓인다.
- `tac-mixed-explicit-break`: 같은 이름의 `samples/issue7482/` 입력과 `pdf/issue7482/`의 `-hwpx-2020.pdf`; 명시적 개행 뒤 둘째 표/C가 다음 줄에 놓인다.
- `full-width`: `samples/issue7481/synth_square_host_full_width_table_no_ls.hwp`, `pdf/synth_square_host_full_width_table_no_ls-2020.pdf`; 공개 NO_LS 합성 입력과 독립 한컴 PDF다. 빈 host의 줄 점유와 뒤 표 위치를 확인하며 Paper 장식의 잔여 차이가 있다.
- `distribution`: `samples/task1768/distribution_doc.hwpx` p3; 상단 목록·라벨/표/뒤 문단 순서를 확인했다. 실루엣 85.83661%와 표 왼쪽 약 7.8px 등 잔여 차이는 #7482 전용 사용자 예외로 평가하며 완전 일치를 주장하지 않는다.

입력 생성과 독립 PDF job/hash는 `samples/issue7482/{README.md,provenance.json}`, 검증 집계와 남은 범위는 `validation-summary.json`, 상세 기록은 별도 로컬 review branch의 `mydocs/working/davindev_7482_7518_requested_verification.md` 최종 검증 절을 따른다. ignored 로컬 증적은 `output/pr-review/davindev-20261003/candidate-final9-{native,wasm}-visual/<key>/{run_manifest.json,pages,review,overlay}`다.

PR 본문에는 이 asset을 포함한 정확한 head SHA의 raw URL로 대표 review/overlay PNG를 표시한다. 이 폴더 준비만으로 원격 push·PR 생성·review·merge가 승인되지는 않는다.
