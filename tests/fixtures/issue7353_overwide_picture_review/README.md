# 저장 줄보다 넓은 단일 TAC 그림

## 출처와 생성

`samples/issue4090/156492236_규제샌드박스_min.hwpx`의 section0,
paragraph136~139를 분리했다. 첫 문단의 앞쪽 나눔만 제거했다.
`generate.rs`는 기존 rhwp library로 실행하며 `output/7353/r19/tac-width-next/`에
입력을 저장한다. 원본 자체는 변경하지 않았다.

최소화 원본의 흰색 그림은 위치 판독이 어려우므로 `visible-input.hwpx`에서
그림 BinData만4색/검정 테두리 패턴으로 교체했다. 문단·셀·표 속성,
그림 크기/crop/offset/여백과 저장 줄 정보는 그대로다. 이 명시적 대조군을
한컴으로 HWP 저장한 후 그 저장본으로 PDF를 출력했다.

- HWP job `11e604cb-c455-4309-98c4-1a19ecd2d861`.
- PDF job `5d87c93b-265c-4597-a643-902201be5fcc`.
- engine2020, Hancom11.0.0.9136, `hwp-managed-direct-dll-host`, 전처리 없음.
- PDF1쪽. 상태/다운로드 응답은 위 output 폴더에 보존한다.

| 파일 | SHA256 |
| --- | --- |
| picture-input.hwpx | `77a985c0217eed03887c5808f5e9aa465e9248d5e39c4f77692a7639cb9ba3a1` |
| visible-input.hwpx | `014246b75fff96b62eac5fa68cd02d653e0e1ff48b55ca91f0f0c4e80798421d` |
| visible-saved.hwp | `e82aae96464cc7ba6dc6abc1a7ed5d43d7333ba5e5ba8deb530c6d675db6e94a` |
| visible-2020.pdf | `b723fea6bba93b87575b7df47a3fd547f00336c5151b9d8a8ed4315da6a44bdb` |

## 독립 관측

외부 표는 Square이고 셀 안 그림은 TAC다. 한컴 재저장 후에도 줄 폭19604HU,
그림 폭19686HU, 셀 폭19607HU가 보존된다. 중앙 정렬 문단이어도 그림을
축소하거나 음수 offset으로 옮기지 않고 줄 시작점에 배치한다.
그림은 셀 오른쪽을79HU 넘어가며 높이는9461HU다.

문서 원점(5669,5668)HU + host vpos5296HU + 표 offset(27166,834)HU +
표 바깥여백(1417,138)HU에서 그림 원점(34252,11936)HU를 얻는다.
한컴 PDF의 `fill_image` transform은 x342.296/y119.14902/w196.814/h94.457pt다.
저장HU 환산과 인쇄 차이는0.25pt 이내다. 제목 표와 왼쪽 본문2문단도 보존한다.

`issue_7353_overwide_picture.rs`는 정상HWP/HWPX96/192dpi의 최종 그림·줄·셀
사각형,79HU 돌출과 전체 내용 보존을 검사한다. 합성 변이에서 폭 경계 전후와
Left/Center/Right, 여러 객체·선행 공백 비적용, 높이 부족 시 원자적 이월과
후속 빈 문단/종료를 확인한다. 합성 변이의 결과를 별도 한컴 관측으로 주장하지 않는다.
이 자료는 그림 내용의 원본 일치나 전체 문서 통과 증거가 아니다.
