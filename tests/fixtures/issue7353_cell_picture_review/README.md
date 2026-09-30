# 셀 안 자리차지 그림과 빈 호스트 줄

## 입력과 독립 기준 출력

`samples/issue4090/156492236_규제샌드박스_min.hwpx`의 section0,
paragraph60~63을 분리했다. 첫 문단의 앞쪽 나눔만 `column_type=None`,
`raw_break_type=0`으로 제거했으며 원본은 변경하지 않았다. 생성 코드는
`generate.rs`이며 기본 실행 시 `output/7353/r19/cell-floating-picture/`에
두 HWPX를 만든다. 기존 rhwp library와 링크하여 실행한다.

원본 최소화 문서의 그림은 흰색이라 위치 판독이 어렵다. `visible-input.hwpx`는
**그림의 BinData 바이트만** 검은 테두리와4색 패턴으로 교체한 명시적 대조군이다.
문단·표·셀·그림 크기·crop·offset·여백·저장 줄 정보는 변경하지 않았다.
원본 내용의 개선 증거와 시각 대조군을 혼동하지 않는다.

각 HWPX를 MCP converter engine2020으로 HWP 저장한 뒤, 그 HWP를 PDF로
출력했다. Hancom11.0.0.9136, `hwp-managed-direct-dll-host`, 전처리 없음,
`hancom2020_pdf_driver_one_up`, 각1쪽이다.

- 원본 분리본 HWP/PDF job: `b9039f01-3455-48fb-9ae2-f6c2c6ece7e6` /
  `4ca2a88d-750b-4f9e-b1cb-707e924b5df9`.
- 색상 대조군 HWP/PDF job: `c6e2c1b5-585f-454d-8dd8-0a6bc6fe545b` /
  `4efe209a-c8f5-4cc3-9508-5d134374bde6`.
- 응답과 실행 증거: `output/7353/r19/cell-floating-picture/`.

| 파일 | SHA256 |
| --- | --- |
| picture-input.hwpx | `b5728ca4e40b60ba49929b91ba17ac557d32c6fcc0f09bcf7709299101e19c38` |
| picture-saved.hwp | `8390f305e9a7f0739ff1963e784a73354a34191ae4c4f341b94242a66599cd76` |
| picture-2020.pdf | `f34dcabbf698bacde26a7ea34eefe5908f09c962c9a7cef0a1aa9dd23b50d1f7` |
| visible-input.hwpx | `ffc785cbf60a23baae5b33bf576411736d7829cd06dcf729cdda327ce9ba43e3` |
| visible-saved.hwp | `77d6f4ccf558de09d7dceaf10d752f740102abb0ff790361da43d18ac7f0f10c` |
| visible-2020.pdf | `39fb1c3c014fb0c6c420647ae6e499d1ecce4fdb49c8e10076537b85bffcbb61` |

## 근거와 범위

바깥 표는 Square/BothSides, 셀 안 그림은 **non-TAC TopAndBottom**이다.
그림은 Column/Left, Para/Top, offset(277,263)HU, 크기17764×13562HU다.
호스트는 폭0/높이1200/줄간격720HU의 실제 빈 줄이다. 점유는
`max(1200+720,263+13562)=13825HU`이며, 높이14377HU 셀의 중앙 정렬은
상단에276HU를 남긴다. 따라서 그림은 셀 위에서539HU 아래에 시작한다.
호스트 줄 자체의 높이를 그림 높이로 바꾸지 않는다.

한컴 PDF `mutool draw -F trace`의 색상 이미지 사각형은
x360.766/y118.79001/w177.505/h135.452pt다. 원본HU 좌표와 인쇄 드라이버
반올림/축척 차이는0.25pt 이내다. 제목 표·왼쪽7줄·오른쪽 표 외곽과 그림을
같이 비교하며 대체 글꼴 외형 차이는 이번 절편의 위치 판정과 구분한다.

정식 검사 `tests/cases/issue_7353_cell_floating_picture.rs`는 정상HWP와
HWPX 재직렬화,96/192dpi 최종 위치/내용/종료를 검사한다. 여백 증가,
호스트보다 작은 그림,1HU 부족한 예산, 후속 빈 문단 이월은 합성 경계 계약이다.
Square 그림·Page 기준·폭 초과·음수 offset·겹침 허용·가시 호스트·다중 줄은
이 제한된 경로의 지원 증거가 아니며 명시 거부한다. 원본 전체 문서는 아직
문단137의 별도 TAC 지원 경계에서 멈춘다.
