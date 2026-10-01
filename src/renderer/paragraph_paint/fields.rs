use crate::model::{control::Control, paragraph::Paragraph};
use crate::renderer::composer::ComposedParagraph;

pub(crate) fn substitute_auto_numbers_in_composed(
    para: &Paragraph,
    comp: &mut ComposedParagraph,
    page_number: u32,
    total_pages: u32,
) {
    let mut replacements: Vec<(usize, String)> = Vec::new();
    for (an_type, value) in [
        (crate::model::control::AutoNumberType::Page, page_number),
        (
            crate::model::control::AutoNumberType::TotalPage,
            total_pages,
        ),
    ] {
        if value == 0 {
            continue;
        }
        let value_str = value.to_string();
        let mut positions = auto_number_placeholder_positions(para, an_type);
        positions.sort_unstable();
        positions.dedup();
        replacements.extend(positions.into_iter().map(|pos| (pos, value_str.clone())));
    }
    if replacements.is_empty() {
        return;
    }
    replacements.sort_unstable_by_key(|(pos, _)| *pos);
    replacements.dedup_by_key(|(pos, _)| *pos);
    replace_composed_chars_with_display(comp, &replacements);
}

fn auto_number_placeholder_positions(
    para: &Paragraph,
    an_type: crate::model::control::AutoNumberType,
) -> Vec<usize> {
    let ctrl_positions = para.control_text_positions();
    let text_chars: Vec<char> = para.text.chars().collect();
    let mut positions = Vec::new();
    let mut search_from = 0usize;

    // [#6986] 종류가 다른 `AutoNumber` 도 **자리를 소비한다.**
    //
    // 종전에는 다른 종류를 `continue` 로 건너뛰면서 `search_from` 을 전진시키지
    // 않았다. 그래서 한 문단에 `PAGE` 와 `TOTAL_PAGE` 가 같이 있으면, 두 번째
    // 종류의 폴백 탐색이 0 부터 시작해 **첫 번째 컨트롤의 자리**를 집었다.
    //
    // 법령 HWPX 의 꼬리말 표 셀이 그 형상이다 —
    // `<hp:t>- </hp:t><PAGE/><hp:t> / </hp:t><TOTAL_PAGE/><hp:t> -</hp:t>`.
    // 두 치환이 같은 자리를 쓰면 나중 것이 앞 것을 덮어, 한쪽은 총쪽수가 찍히고
    // 다른 쪽은 빈칸이 된다(`- / 187 -`). v0.8.3 에서 `TOTAL_PAGE` 치환이
    // 들어오면서 생긴 회귀다(`e69a2d286`).
    //
    // 그래서 **모든** `AutoNumber` 를 순서대로 돌며 자리를 하나씩 소비하고,
    // 그중 요청한 종류의 것만 돌려준다.
    for (ctrl_idx, ctrl) in para.controls.iter().enumerate() {
        let Control::AutoNumber(an) = ctrl else {
            continue;
        };
        let wanted = an.number_type == an_type;

        let direct_pos = ctrl_positions.get(ctrl_idx).copied().filter(|&pos| {
            is_auto_number_placeholder_at(para, &text_chars, pos)
                || text_chars
                    .get(pos)
                    .map_or(false, |ch| is_auto_number_placeholder_char(*ch))
        });

        let pos = direct_pos
            .or_else(|| find_auto_number_placeholder_char(para, &text_chars, search_from));

        if let Some(pos) = pos {
            if wanted {
                positions.push(pos);
            }
            search_from = pos.saturating_add(1);
        }
    }

    positions
}

fn is_auto_number_placeholder_char(ch: char) -> bool {
    ch == '\u{0015}' || ch.is_whitespace()
}

fn is_auto_number_placeholder_at(para: &Paragraph, text_chars: &[char], idx: usize) -> bool {
    if !text_chars
        .get(idx)
        .map_or(false, |ch| is_auto_number_placeholder_char(*ch))
    {
        return false;
    }

    let Some(&current) = para.char_offsets.get(idx) else {
        return false;
    };
    let next = para
        .char_offsets
        .get(idx.saturating_add(1))
        .copied()
        .unwrap_or_else(|| para.char_count.saturating_sub(1));

    next.saturating_sub(current) >= 8
}

fn find_auto_number_placeholder_char(
    para: &Paragraph,
    text_chars: &[char],
    search_from: usize,
) -> Option<usize> {
    let preferred = text_chars
        .iter()
        .enumerate()
        .skip(search_from)
        .find(|(idx, _)| is_auto_number_placeholder_at(para, text_chars, *idx))
        .map(|(idx, _)| idx);

    preferred.or_else(|| {
        if !para.char_offsets.is_empty() {
            return text_chars
                .iter()
                .enumerate()
                .rev()
                .find(|(idx, ch)| *idx >= search_from && is_auto_number_placeholder_char(**ch))
                .map(|(idx, _)| idx);
        }
        text_chars
            .iter()
            .enumerate()
            .skip(search_from)
            .find(|(_, ch)| is_auto_number_placeholder_char(**ch))
            .map(|(idx, _)| idx)
    })
}

fn replace_composed_chars_with_display(
    comp: &mut ComposedParagraph,
    replacements: &[(usize, String)],
) -> bool {
    if replacements.is_empty() {
        return false;
    }
    let mut applied = false;
    for line in &mut comp.lines {
        let mut run_start = line.char_start;
        for run_idx in 0..line.runs.len() {
            let run_len = line.runs[run_idx].text.chars().count();
            let run_end = run_start + run_len;

            let mut in_run: Vec<(usize, &str)> = replacements
                .iter()
                .filter(|(pos, _)| *pos >= run_start && *pos < run_end)
                .map(|(pos, rep)| (pos - run_start, rep.as_str()))
                .collect();
            if !in_run.is_empty() {
                in_run.sort_unstable_by_key(|(rel, _)| *rel);
                let chars: Vec<char> = line.runs[run_idx].text.chars().collect();
                let mut display = String::new();
                let mut cursor = 0usize;
                for (rel, rep) in in_run {
                    if rel >= chars.len() {
                        continue;
                    }
                    let plain: String = chars[cursor..rel].iter().collect();
                    display.push_str(&crate::renderer::composer::expand_pua_display_text(&plain));
                    display.push_str(rep);
                    cursor = rel + 1;
                }
                let tail: String = chars[cursor.min(chars.len())..].iter().collect();
                display.push_str(&crate::renderer::composer::expand_pua_display_text(&tail));
                // `line.runs[run_idx].text` 는 marker 를 포함한 원 모델 문자열이다.
                // 바꾸지 않아야 char_start/offset 이 표시 자릿수에 끌려가지 않는다.
                line.runs[run_idx].display_text = Some(display);
                applied = true;
            }
            run_start = run_end;
        }
    }
    applied
}
