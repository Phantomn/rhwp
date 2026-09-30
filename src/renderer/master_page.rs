//! Shared section master-page selection. Both rendering hosts bind the same
//! physical page number and first/last-page policy before painting.
use super::pagination::{MasterPageRef, PaginationResult};
use crate::model::document::Section;

pub(crate) fn assign_master_pages_for_section(
    result: &mut PaginationResult,
    section_index: usize,
    section: &Section,
    carry_master_odd: &Option<MasterPageRef>,
    carry_master_even: &Option<MasterPageRef>,
) {
    use crate::model::header_footer::HeaderFooterApply;

    for page in &mut result.pages {
        page.active_master_page = None;
        page.extra_master_pages.clear();
    }

    let mps = &section.section_def.master_pages;
    if mps.is_empty() {
        return;
    }

    let base_mp_indices: Vec<usize> = mps
        .iter()
        .enumerate()
        .filter(|(_, m)| !m.is_extension)
        .map(|(i, _)| i)
        .collect();
    let single_base_mp = if base_mp_indices.len() == 1 {
        base_mp_indices.first().copied()
    } else {
        None
    };
    let mp_both = base_mp_indices
        .iter()
        .copied()
        .find(|&i| mps[i].apply_to == HeaderFooterApply::Both);
    let mp_odd = base_mp_indices
        .iter()
        .copied()
        .find(|&i| mps[i].apply_to == HeaderFooterApply::Odd);
    let mp_even = base_mp_indices
        .iter()
        .copied()
        .find(|&i| mps[i].apply_to == HeaderFooterApply::Even);
    let ext_mp_indices: Vec<usize> = mps
        .iter()
        .enumerate()
        .filter(|(_, m)| m.is_extension)
        .map(|(i, _)| i)
        .collect();

    let section_page_count = result.pages.len();
    for (page_idx_in_section, page) in result.pages.iter_mut().enumerate() {
        let is_last = page_idx_in_section + 1 == section_page_count;
        let is_first_page = page_idx_in_section == 0;

        if is_first_page && section.section_def.hide_master_page {
            continue;
        }

        let selected = if page.page_number % 2 == 1 {
            mp_odd
                .or(mp_both)
                .map(|mi| MasterPageRef {
                    section_index,
                    master_page_index: mi,
                })
                .or_else(|| carry_master_odd.clone())
                .or_else(|| {
                    single_base_mp.map(|mi| MasterPageRef {
                        section_index,
                        master_page_index: mi,
                    })
                })
        } else {
            mp_even
                .or(mp_both)
                .map(|mi| MasterPageRef {
                    section_index,
                    master_page_index: mi,
                })
                .or_else(|| carry_master_even.clone())
                .or_else(|| {
                    single_base_mp.map(|mi| MasterPageRef {
                        section_index,
                        master_page_index: mi,
                    })
                })
        };
        page.active_master_page = selected;

        if is_last && !ext_mp_indices.is_empty() {
            let replace_exts: Vec<usize> = ext_mp_indices
                .iter()
                .filter(|&&i| !mps[i].overlap || mps[i].replace_base)
                .copied()
                .collect();
            let overlap_exts: Vec<usize> = ext_mp_indices
                .iter()
                .filter(|&&i| mps[i].overlap && !mps[i].replace_base)
                .copied()
                .collect();

            if let Some(&replace_idx) = replace_exts.last() {
                page.active_master_page = Some(MasterPageRef {
                    section_index,
                    master_page_index: replace_idx,
                });
            }

            let active_apply = page
                .active_master_page
                .as_ref()
                .and_then(|mp_ref| {
                    if mp_ref.section_index == section_index {
                        mps.get(mp_ref.master_page_index)
                    } else {
                        None
                    }
                })
                .map(|m| m.apply_to);
            let mut remaining_overlap_exts: Vec<usize> = Vec::new();
            for &i in &overlap_exts {
                if Some(mps[i].apply_to) == active_apply {
                    page.active_master_page = Some(MasterPageRef {
                        section_index,
                        master_page_index: i,
                    });
                } else {
                    remaining_overlap_exts.push(i);
                }
            }
            if !remaining_overlap_exts.is_empty() {
                page.extra_master_pages = remaining_overlap_exts
                    .iter()
                    .map(|&mi| MasterPageRef {
                        section_index,
                        master_page_index: mi,
                    })
                    .collect();
            }
        }
    }
}
