# 본문 문단의 저장 페이지 좌표 리셋 대조군

`portrait-saved.hwp`는 수동 LineSeg 입력이 아니라 한컴이 정상 저장한 문서다.
`portrait-2020.pdf`는 같은 저장본을 한컴 PDF 드라이버로 출력한 독립 기준(2쪽)이다.
원래 #7158 문서 전체의 피델리티를 입증하는 자료는 아니다.

## 생성과 출처

- `create.rs`는 저장소 루트에서 실행한다. 기존 `issue7353_picture_space_review/picture-saved.hwp`의
  정상 글꼴 리소스를 재사용하고 본문을 BEFORE / 여러 줄 문단 / AFTER로 대체한다.
  새 문단에는 LineSeg를 직접 지정하지 않는다. 출력은 `output/7353/r19/body-reset/portrait-input.hwpx`다.
- `portrait-input.hwpx`를 MCP 한컴 2020으로 HWP 저장한 job:
  `dfaec71c-9e09-4ef4-8079-9d1af956904a`.
- 그 HWP를 PDF로 출력한 job: `87faa4e5-c44a-4866-a803-1ea239f9b344`.
- 한컴 `11.0.0.9136`, `hwp-managed-direct-dll-host`, 입력 전처리 없음,
  `hancom2020_pdf_driver_one_up`. 생성일 2026-09-28.
- 상세 job 결과는 `output/7353/r19/body-reset/portrait-{save,pdf}-status.json`에 보존한다.

## 독립 기대값

본문 위 여백1500HU, 정상 저장 줄 높이1200HU, 줄간격600HU다.
두 번째 문단의 저장 vpos는 `[1800,3600,5400,7200,0,1800]`이다.
1쪽은 BEFORE / ALPHA / BRAVO / **빈 줄** / CHARLIE,
2쪽은 DELTA / ECHO / AFTER 순서여야 한다.
96dpi의 줄 상자 높이는16px, 각 쪽의 첫 줄 y=20px, 줄 시작 간격24px이다.
`tests/cases/issue_7353_table_v2_document_flow.rs`는 HWP와 HWPX 직렬화 경로의
최종 줄 위치·빈 줄·이어지는 문단·쪽 수를 검사한다.

초기 가로형 용지 대조군은 PDF 인쇄 방향과 달라 비교용으로 채택하지 않았다.
그 입력·출력은 `output/7353/r19/body-reset/landscape-draft-fixture`와 작업 output에 보존했다.
그 자료에 맞추어 엔진 좌표나 기대값을 변경하지 않았다.

## SHA256

```text
a175dd01a1e5ee30211524fb6e646fa3ed2b8979bd6337c5352f5888445a5f15  portrait-input.hwpx
0177d1983fec266e82165506a8658a6f4a7438e1e1a223753e6fabc9a37f7887  portrait-saved.hwp
ab9f1ac629a7a7bb04ee83e4c825c7655fe7975d19f9b9df024c733b78a8d376  portrait-2020.pdf
```
