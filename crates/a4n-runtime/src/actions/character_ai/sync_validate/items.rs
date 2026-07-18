use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::super::json_utils::required_i32_at;

pub(in crate::actions::character_ai) fn validate_contiguous_sync_range(
    start_chapter: i32,
    target_chapter: i32,
    selected_chapter_numbers: &[i32],
) -> Result<(), String> {
    if start_chapter <= 0 || target_chapter < start_chapter {
        return Err(format!(
            "角色档案同步范围无效：第 {start_chapter} 章至第 {target_chapter} 章"
        ));
    }

    let available = selected_chapter_numbers
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let missing = (start_chapter..=target_chapter)
        .filter(|chapter| !available.contains(chapter))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }

    let missing_text = missing
        .iter()
        .map(|chapter| format!("第 {chapter} 章"))
        .collect::<Vec<_>>()
        .join("、");
    Err(format!(
        "角色档案同步范围不完整：第 {start_chapter} 章至第 {target_chapter} 章中缺少可同步章节：{missing_text}"
    ))
}

pub(in crate::actions::character_ai) fn validate_sync_item_chapters(
    label: &str,
    items: &[Value],
    chapter_numbers: &[i32],
) -> Result<(), String> {
    let allowed = chapter_numbers.iter().copied().collect::<BTreeSet<_>>();
    for (index, item) in items.iter().enumerate() {
        let chapter = required_i32_at(
            item,
            "chapter_number",
            &format!("{label}[{index}].chapter_number"),
        )?;
        if !allowed.contains(&chapter) {
            return Err(format!(
                "{label}[{index}].chapter_number 不在本次同步输入章节中: 第 {chapter} 章；本次章节为 {}",
                chapter_numbers
                    .iter()
                    .map(|chapter| format!("第 {chapter} 章"))
                    .collect::<Vec<_>>()
                    .join("、")
            ));
        }
    }
    Ok(())
}

pub(in crate::actions::character_ai) fn validate_sync_item_count_by_chapter(
    label: &str,
    items: &[Value],
    max_per_chapter: usize,
) -> Result<(), String> {
    let mut counts = BTreeMap::<i32, usize>::new();
    for (index, item) in items.iter().enumerate() {
        let chapter = required_i32_at(
            item,
            "chapter_number",
            &format!("{label}[{index}].chapter_number"),
        )?;
        let count = counts.entry(chapter).or_default();
        *count += 1;
        if *count > max_per_chapter {
            return Err(format!(
                "{label} 第 {chapter} 章最多 {max_per_chapter} 条，实际至少 {} 条；请只保留最重要的新变化或关键行为",
                *count
            ));
        }
    }
    Ok(())
}

pub(in crate::actions::character_ai) fn sort_sync_items_by_chapter(
    items: &mut [Value],
) -> Result<(), String> {
    let mut keyed = Vec::with_capacity(items.len());
    for (index, item) in items.iter().cloned().enumerate() {
        keyed.push((
            required_i32_at(
                &item,
                "chapter_number",
                &format!("sync_items[{index}].chapter_number"),
            )?,
            index,
            item,
        ));
    }
    keyed.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    for (slot, (_, _, item)) in items.iter_mut().zip(keyed) {
        *slot = item;
    }
    Ok(())
}
