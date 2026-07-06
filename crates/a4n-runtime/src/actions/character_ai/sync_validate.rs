use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use super::json_utils::required_i32_at;
use super::state_projection::project_attribute_changes;
use super::sync_types::CharacterSyncExtraction;

pub(super) fn validate_materialized_character_sync_projection(
    extraction: &CharacterSyncExtraction,
    baseline_state: &Value,
) -> Result<(), String> {
    project_attribute_changes(baseline_state.clone(), &extraction.changes)?;
    Ok(())
}

pub(super) fn validate_contiguous_sync_range(
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

pub(super) fn validate_sync_item_chapters(
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

pub(super) fn validate_sync_item_count_by_chapter(
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

pub(super) fn validate_sync_evidence_against_chapters(
    changes: &mut [Value],
    behaviors: &mut [Value],
    deletion_candidates: &mut [Value],
    chapter_data: &[Value],
) -> Result<(), String> {
    if chapter_data.is_empty() {
        return Ok(());
    }

    for (index, change) in changes.iter_mut().enumerate() {
        materialize_item_evidence(
            "attribute_changes",
            index,
            change,
            "evidence_anchor_id",
            "evidence",
            chapter_data,
        )?;
    }
    for (index, behavior) in behaviors.iter_mut().enumerate() {
        materialize_item_evidence(
            "behaviors",
            index,
            behavior,
            "original_text_anchor_id",
            "original_text",
            chapter_data,
        )?;
    }
    for (index, candidate) in deletion_candidates.iter_mut().enumerate() {
        materialize_item_evidence(
            "deletion_candidates",
            index,
            candidate,
            "evidence_anchor_id",
            "evidence",
            chapter_data,
        )?;
    }

    Ok(())
}

fn materialize_item_evidence(
    label: &str,
    index: usize,
    item: &mut Value,
    anchor_field: &str,
    target_field: &str,
    chapter_data: &[Value],
) -> Result<(), String> {
    let chapter_number = required_i32_at(
        item,
        "chapter_number",
        &format!("{label}[{index}].chapter_number"),
    )?;
    let anchor_id = required_anchor_id(
        item,
        anchor_field,
        &format!("{label}[{index}].{anchor_field}"),
    )?;
    let chapter = chapter_data
        .iter()
        .find(|chapter| {
            chapter
                .get("chapter_number")
                .and_then(Value::as_i64)
                .is_some_and(|value| value == i64::from(chapter_number))
        })
        .ok_or_else(|| format!("{label}[{index}] 缺少可核对证据的第 {chapter_number} 章正文"))?;
    let text_anchors = chapter
        .get("text_anchors")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            format!("{label}[{index}] 第 {chapter_number} 章上下文缺少 text_anchors，无法核对证据")
        })?;
    let text = text_anchors
        .iter()
        .find(|anchor| {
            anchor
                .get("id")
                .and_then(Value::as_i64)
                .is_some_and(|value| value == anchor_id)
        })
        .and_then(|anchor| anchor.get("quote").and_then(Value::as_str))
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| {
            format!("{label}[{index}].{anchor_field} 不在第 {chapter_number} 章 text_anchors 中：{anchor_id}")
        })?
        .to_string();

    if let Some(object) = item.as_object_mut() {
        object.insert(target_field.to_string(), Value::String(text));
        object.remove(anchor_field);
    }
    Ok(())
}

fn required_anchor_id(item: &Value, field: &str, path: &str) -> Result<i64, String> {
    let id = item
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("角色 AI 字段 {path} 必须是 JSON 整数数字"))?;
    if id <= 0 {
        return Err(format!("角色 AI 字段 {path} 必须大于 0"));
    }
    Ok(id)
}

pub(super) fn sort_sync_items_by_chapter(items: &mut [Value]) -> Result<(), String> {
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
