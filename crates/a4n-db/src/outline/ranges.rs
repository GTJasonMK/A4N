use std::collections::{BTreeSet, HashSet};

use a4n_core::models::chapter::{chapter_outline, part_outline};

use super::OutlineDbError;

pub(super) fn expected_part_id_for_existing_outline(
    parts: &[part_outline::Model],
    outline: &chapter_outline::Model,
) -> Result<Option<i64>, OutlineDbError> {
    part_id_for_chapter_number(parts, outline.chapter_number)
}

/// 章节可以先于分部存在；没有覆盖该章节的分部时保持未归属。
pub fn part_id_for_chapter_number(
    parts: &[part_outline::Model],
    chapter_number: i32,
) -> Result<Option<i64>, OutlineDbError> {
    let mut matched = parts
        .iter()
        .filter(|part| chapter_number >= part.start_chapter && chapter_number <= part.end_chapter);
    let first = matched.next();
    if matched.next().is_some() {
        return Err(OutlineDbError::InvalidArgument(format!(
            "第 {} 章匹配到多个分部大纲，无法生成章节大纲",
            chapter_number
        )));
    }
    Ok(first.map(|part| part.id))
}

pub fn validate_part_outline_records(parts: &[part_outline::Model]) -> Result<(), OutlineDbError> {
    validate_part_outline_records_with_action(parts, "请先清理分部状态后重试")
}

fn validate_part_outline_records_with_action(
    parts: &[part_outline::Model],
    action: &str,
) -> Result<(), OutlineDbError> {
    let mut seen = HashSet::new();
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
