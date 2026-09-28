# 셀 최소 높이의 분할 소비 대조군

`cell-tall-input.hwpx`는 `../stored-frame-end/base2-input.hwpx`의 셀 높이만
1000→200000HU로 변경한 작성 입력이다. 입력에는 저장 LineSeg가 없다.
`cell-tall-saved.hwp`는 이 입력을 한컴2020 profile로 정상 저장한 결과이고,
`cell-tall-2020.pdf`는 그 HWP를 같은 profile로 출력한 독립 기준이다.

- HWP 변환 job: `ab19142e-5b86-44a1-97c5-9ae696f6dc58`
- PDF 변환 job: `f415c10f-a732-4700-ab42-df16048d099b`
- 로컬 생성 절차와 job 증적: `output/7353/r19/frame-band/`
- 원래65줄과 줄 분할은 유지하며,3쪽에 걸쳐 셀의 물리 높이를 소비한다.
- 첫 조각의 저장 common.height는67913HU, 기본 대조군은67482HU다.

`issue_7353_fragment_minimum_band`는 첫 조각의 추가 공간과 총 최소 높이,
65줄의 순서·보존을 검사한다. 이 계약의 통과를 PDF 외곽과의 정확한 일치로
간주하지 않는다. 쪽 하단 예산/바깥여백 차이는 stage19의 별도 미해결 항목이다.
