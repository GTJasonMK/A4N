use std::collections::BTreeSet;

use a4n_core::models::chapter::{chapter_outline, part_outline};

use super::OutlineDbError;

pub(super) fn expected_part_id_for_existing_outline(
    parts: &[part_outline::Model],
    outline: &chapter_outline::Model,
) -> Result<Option<i64>, OutlineDbError> {
    if parts.is_empty() {
        return Ok(None);
    }
    let mut matched = parts.iter().filter(|part| {
        outline.chapter_number >= part.start_chapter && outline.chapter_number <= part.end_chapter
    });
    let first = matched.next();
    if matched.next().is_some() {
        return Err(OutlineDbError::InvalidArgument(format!(
            "第 {} 章匹配到多个分部大纲，无法生成章节大纲",
            outline.chapter_number
        )));
    }
    match first {
        Some(part) => Ok(Some(part.id)),
        None => Err(OutlineDbError::InvalidArgument(format!(
            "已有第 {} 章章纲不在任何分部范围内，请先删除旧章纲或重新生成覆盖该章节的分部",
            outline.chapter_number
        ))),
    }
}

pub(super) fn latest_part_ids_to_delete(
    parts: &[part_outline::Model],
    count: u64,
) -> Result<Vec<i64>, OutlineDbError> {
    if count == 0 {
        return Err(OutlineDbError::InvalidArgument(
            "删除分部数量必须大于 0".to_string(),
        ));
    }
    if count as usize > parts.len() {
        return Err(OutlineDbError::InvalidArgument(format!(
            "删除分部数量超过现有分部数量：count={count}, total={}",
            parts.len()
        )));
    }
    validate_existing_parts_for_latest_delete(parts)?;
    Ok(parts
        .iter()
        .rev()
        .take(count as usize)
        .map(|part| part.id)
        .collect())
}

fn validate_existing_parts_for_latest_delete(
    parts: &[part_outline::Model],
) -> Result<(), OutlineDbError> {
    let action = "请先清理分部状态后再删除";
    let mut seen = BTreeSet::new();
    let mut ranges = Vec::with_capacity(parts.len());
    for part in parts {
        if !seen.insert(part.part_number) {
            return Err(OutlineDbError::InvalidArgument(format!(
                "第 {} 部已有重复分部大纲，{}",
                part.part_number, action
            )));
        }
        if part.part_number <= 0 {
            return Err(OutlineDbError::InvalidArgument(format!(
                "已有第 {} 部分部编号无效，{}",
                part.part_number, action
            )));
        }
        if part.start_chapter <= 0 || part.end_chapter < part.start_chapter {
            return Err(OutlineDbError::InvalidArgument(format!(
                "已有第 {} 部章节范围 {}..={} 无效，{}",
                part.part_number, part.start_chapter, part.end_chapter, action
            )));
        }
        ranges.push((part.start_chapter, part.end_chapter, part.part_number));
    }
    ranges.sort_by_key(|(start_chapter, end_chapter, part_number)| {
        (*start_chapter, *end_chapter, *part_number)
    });
    for pair in ranges.windows(2) {
        let (prev_start, prev_end, prev_part) = pair[0];
        let (start, end, part) = pair[1];
        if start <= prev_end {
            return Err(OutlineDbError::InvalidArgument(format!(
                "已有分部章节范围重叠：第 {prev_part} 部 {prev_start}..={prev_end} 与第 {part} 部 {start}..={end}，{action}"
            )));
        }
    }
    Ok(())
}

pub fn chapter_outline_ids_in_part_range(
    outlines: &[chapter_outline::Model],
    part: &part_outline::Model,
) -> Vec<i64> {
    outlines
        .iter()
        .filter(|outline| {
            outline.chapter_number >= part.start_chapter
                && outline.chapter_number <= part.end_chapter
        })
        .map(|outline| outline.id)
        .collect()
}

pub fn chapter_outline_ids_in_part_ranges(
    outlines: &[chapter_outline::Model],
    parts: &[part_outline::Model],
) -> Vec<i64> {
    let mut ids = BTreeSet::new();
    for part in parts {
        ids.extend(chapter_outline_ids_in_part_range(outlines, part));
    }
    ids.into_iter().collect()
}
