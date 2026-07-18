use serde_json::Value;

use super::super::json_utils::required_i32_at;
use super::super::state_projection::project_attribute_changes;
use super::super::sync_types::CharacterSyncExtraction;

pub(in crate::actions::character_ai) fn validate_materialized_character_sync_projection(
    extraction: &CharacterSyncExtraction,
    baseline_state: &Value,
) -> Result<(), String> {
    project_attribute_changes(baseline_state.clone(), &extraction.changes)?;
    Ok(())
}

pub(in crate::actions::character_ai) fn validate_sync_evidence_against_chapters(
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
