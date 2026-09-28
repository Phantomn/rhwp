# #7353 문단 앞 간격을 포함한 저장 셀 프레임 재시작

## 입력 생성과 독립 근거

직전 승인된 `../local-column-anchor/prefix163-saved.hwp`의 본문p161/표cell13/
문단0/control1/자식cell0에서 마지막 문단 앞에 실제 빈 문단을 하나 삽입했다.
문단모양·글자모양은 같은 셀의 두 번째 문단을 사용하며 다른 내용/표 속성은 보존했다.
생성 소스는 `output/7353/r19/frame-origin/create.rs`다. LineSeg를 임의로100으로
수정하지 않았다. 생성 HWPX를 한컴에서 정상 저장하고 **그 HWP**를 PDF로 변환했다.

- HWP 정상 저장 job: `72519ed2-8371-481a-b496-77da33eb3a32`
- PDF job: `352d7bb8-365e-43f8-b519-6e9de0068b2f`
- engine2020, 정상 완료, PDF26쪽. 상세 영수증은 같은 output의 `save-*`/`pdf-*`다.
- HWPX SHA256: `068a4a2a699a3d7a0d43cd216c900dad79ee7324e367021f4a28d20ae8b65e19`
- 저장 HWP SHA256: `13fa2b347eecb4e2ff9f65f3d7ca4635fe5762c5c6a24e94eb9d09286f32ffbf`
- PDF SHA256: `f677b9660857ab2e3301cac0fbaf215f04ccabe8922fbbbe76cc5918a3747901`

원본 전체를 그대로 출력한 자료가 아니라 **빈 문단을 추가한 정상 저장 대조군**이다.
원본 `samples/86712_regulatory_analysis.hwp`의 같은 셀도 마지막 문단 첫 줄이100HU로
재시작하지만 앞선 줄 구성은 다르다. 이 대조군의 일치를 원본 전체 통과로 해석하지 않는다.

## 관측과 계약

정상 저장된 자식p10/l0는v100, 문단 앞 간격 raw200(URC 2배 스케일)이다.
PDF25쪽에는p9의 빈 줄이 남고, PDF26쪽에는 마지막 문단 두 줄이 이어진다.
문단 경계에서는 문단 앞 간격이 적용되지만 문단 내부 이어받기에는 다시 적용되지 않는다.
`100` 자체를 특례로 쓰지 않고 공통 style resolver의 spacing_before와 비교한다.

`mutool draw -F trace`로 얻은 PDF26쪽 독립 측정(pt):

- 부모 위/아래 선: y782.864 /731.680 (용지 높이841, 좌표 반전).
- 첫 글줄 기준선:608 × .119869pt. 인쇄 세로 계수는 명목 .12와 다르다.
- 입력의 자식 위 여백141HU + 앞 간격100HU가 실제 첫 줄 원점을 만든다.
- 둘째 줄 원점은 첫 줄에서1800HU 아래, 부모 후속 빈 문단은1300HU다.

정식 검사 `tests/cases/issue_7353_stored_frame_origin.rs`는96/144dpi에서 실제
DocumentV2의 페이지 소유·최종 줄 좌표·표 외곽·빈 줄과 원문 유닛 보존을 검사한다.
임의의 비영 재시작과 문단 내부100HU 재시작은 typed IR 반례로 거부한다.
이 반례들은 추가 한컴 정상 생성본이라고 주장하지 않는다.

## 검증 범위

수정 전 동일 정상 입력은p161의 `unqualified stored cell frame reset`으로 실패한다.
신규 정상본 회귀 검사도 수정 전 같은 이유로FAIL, 수정 후PASS다. 기록은
`output/7353/r19/frame-origin/test-before.log`, `test-final.log`다.
Native/fresh Docker WASM 비교와 source manifest는 같은 output의 `review/`,
`source.sha256` 및 stage19에 연결한다.

대체 글꼴의 외형·농도와 인쇄 잔차는 별도다. 원본 전체의 다음 미지원p172/
cell80/childcell0/p21의v9213 재시작은 이 계약으로 수용하지 않는다.
