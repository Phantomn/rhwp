//! Engine-independent source ownership of cell and textbox render nodes.

/// 표 경로의 단일 레벨 (표 → 셀 → 문단)
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct CellPathEntry {
    /// 문단 내 컨트롤 인덱스 (표)
    pub control_index: usize,
    /// 표 내 셀 인덱스
    pub cell_index: usize,
    /// 셀 내 문단 인덱스
    pub cell_para_index: usize,
    /// 텍스트 방향 (0=가로, 1=세로/영문눕힘, 2=세로/영문세움)
    pub text_direction: u8,
}

/// 표 셀 내부 문단 편집용 컨텍스트 (중첩 표 경로 지원)
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct CellContext {
    /// 최외곽 표를 소유한 구역 문단 인덱스
    pub parent_para_index: usize,
    /// 표 경로 (depth 1=단일 표, depth 2+=중첩 표)
    pub path: Vec<CellPathEntry>,
    /// [#5820] 글상자(drawText) 내부 문단 여부 — 표 셀과 달리 한글은 글상자
    /// 안에서도 셀 밖 규칙(오른쪽 정렬 말미 공백 제외)을 적용한다.
    pub in_textbox: bool,
}

impl CellContext {
    /// 최외곽 표의 컨트롤 인덱스 — 빈 경로면 None (HWP 변조/편집 API로 빈 경로 생성 가능).
    pub fn outermost_control(&self) -> Option<usize> {
        self.path.first().map(|e| e.control_index)
    }
    /// 최외곽 표의 셀 인덱스 — 빈 경로면 None.
    pub fn outermost_cell(&self) -> Option<usize> {
        self.path.first().map(|e| e.cell_index)
    }
    /// 최외곽 표의 셀 문단 인덱스 — 빈 경로면 None.
    pub fn outermost_cell_para(&self) -> Option<usize> {
        self.path.first().map(|e| e.cell_para_index)
    }
    /// 최내곽 레벨의 엔트리 — 빈 경로면 None.
    pub fn innermost(&self) -> Option<&CellPathEntry> {
        self.path.last()
    }
    /// 텍스트 방향 (최내곽 기준) — 빈 경로면 None.
    pub fn text_direction(&self) -> Option<u8> {
        self.innermost().map(|e| e.text_direction)
    }

    /// [#4334] 이 경로가 가리키는 **중첩 표 자신의** `(para_index, control_index)` —
    /// `layout_table`/`layout_partial_table_item` 의 `table_meta` 인자 형태 그대로다.
    /// 경로 마지막 항목이 그 중첩 표 컨트롤이고, 한 단계 바깥 항목의 `cell_para_index`
    /// 가 그 표를 담은 셀 문단이다. depth 1(최외곽 표)은 바깥 레벨이 없어 `None`.
    ///
    /// 재귀 중첩 표를 배치하는 세 곳(`table_layout.rs` 2곳, `table_partial.rs` 1곳)이
    /// `table_meta: None` 을 넘겨 `TableNode.para_index`/`control_index` 가 항상 비어
    /// 있었다 — #4334 stage3 가 실측한 "문서 위치 없는 노드" 의 주된 원인이다.
    pub fn nested_table_meta(&self) -> Option<(usize, usize)> {
        let table_entry = self.path.last()?;
        let parent_entry = self.path.get(self.path.len().checked_sub(2)?)?;
        Some((parent_entry.cell_para_index, table_entry.control_index))
    }

    /// (cell_index, cell_para_index, outer_table_control_index) — 최내곽 entry 의 3 필드.
    /// ImageNode / RectangleNode 등의 cell context 3 필드 매핑 boilerplate 통합용.
    /// path 가 비어있으면 (None, None, None).
    pub fn last_image_indices(&self) -> (Option<usize>, Option<usize>, Option<usize>) {
        match self.path.last() {
            Some(e) => (
                Some(e.cell_index),
                Some(e.cell_para_index),
                Some(e.control_index),
            ),
            None => (None, None, None),
        }
    }
}
