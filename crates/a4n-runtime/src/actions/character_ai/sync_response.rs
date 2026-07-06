use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use super::json_utils::{
    optional_string, optional_string_or_null, required_array, required_attribute_change_operation,
    required_category, required_i32_at, required_string, required_string_array, required_value,
};

pub(super) const CHARACTER_SYNC_ATTRIBUTE_CHANGES_PER_CHAPTER_MAX: usize = 6;
pub(super) const CHARACTER_SYNC_BEHAVIORS_PER_CHAPTER_MAX: usize = 6;
pub(super) const CHARACTER_SYNC_DELETION_CANDIDATES_PER_CHAPTER_MAX: usize = 3;
const CHARACTER_SYNC_DESCRIPTION_MAX_CHARS: usize = 120;
const CHARACTER_SYNC_IMPACT_ANALYSIS_MAX_CHARS: usize = 160;
const CHARACTER_SYNC_STATE_SUMMARY_MAX_CHARS: usize = 160;
const CHARACTER_SYNC_BEHAVIOR_TAGS_MAX_ITEMS: usize = 5;
const CHARACTER_SYNC_RELATED_CHARACTERS_MAX_ITEMS: usize = 6;

#[derive(Debug, Clone)]
pub(super) struct ChapterStateDraft {
    pub(super) chapter_number: i32,
    pub(super) state_summary: String,
    pub(super) current_location: Option<String>,
    pub(super) current_goal: Option<String>,
    pub(super) emotional_state: Option<String>,
}

pub(super) fn validate_unique_deletion_candidates(candidates: &[Value]) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for (index, candidate) in candidates.iter().enumerate() {
        let chapter = required_i32_at(
            candidate,
            "chapter_number",
            &format!("deletion_candidates[{index}].chapter_number"),
        )?;
        let category = required_category(candidate, "category")?;
        let key = required_string(candidate, "key")?;
        if !seen.insert((chapter, category.clone(), key.clone())) {
            return Err(format!(
                "deletion_candidates[{index}] 与前序删除候选重复：第 {chapter} 章 {category}.{key}"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn parse_character_sync_output(content: &str) -> Result<Value, String> {
    let value: Value = serde_json::from_str(content.trim())
        .map_err(|e| format!("角色同步结果必须是 JSON 对象: {e}"))?;
    if !value.is_object() {
        return Err("角色同步结果必须是 JSON 对象".to_string());
    }
    if value.get("updated_attributes").is_some() {
        return Err(
            "角色 AI 顶层字段 updated_attributes 已废弃；最终状态由系统根据 attribute_changes 推导"
                .to_string(),
        );
    }
    for key in [
        "state_summary",
        "current_location",
        "current_goal",
        "emotional_state",
    ] {
        if value.get(key).is_some() {
            return Err(format!(
                "角色 AI 顶层字段 {key} 已废弃；每章状态必须写入 chapter_states"
            ));
        }
    }
    validate_top_level_fields(&value)?;
    for (index, change) in required_array(&value, "attribute_changes")?
        .iter()
        .enumerate()
    {
        validate_attribute_change(index, change)?;
    }
    for (index, behavior) in required_array(&value, "behaviors")?.iter().enumerate() {
        validate_behavior(index, behavior)?;
    }
    for (index, candidate) in required_array(&value, "deletion_candidates")?
        .iter()
        .enumerate()
    {
        validate_deletion_candidate(index, candidate)?;
    }
    validate_unique_deletion_candidates(required_array(&value, "deletion_candidates")?)?;
    for (index, state) in required_array(&value, "chapter_states")?.iter().enumerate() {
        validate_chapter_state(index, state)?;
    }
    Ok(value)
}

#[cfg(test)]
fn validate_top_level_fields(value: &Value) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| "角色同步结果必须是 JSON 对象".to_string())?;
    for key in object.keys() {
        if matches!(
            key.as_str(),
            "profile_name"
                | "attribute_changes"
                | "behaviors"
                | "deletion_candidates"
                | "chapter_states"
        ) {
            continue;
        }
        return Err(format!(
            "角色 AI 顶层字段 {key} 不在当前角色同步合同中；只允许 profile_name、attribute_changes、behaviors、deletion_candidates、chapter_states"
        ));
    }
    Ok(())
}

pub(super) fn validate_attribute_change(index: usize, change: &Value) -> Result<(), String> {
    if !change.is_object() {
        return Err(format!("attribute_changes[{index}] 必须是 JSON 对象"));
    }
    required_i32_at(
        change,
        "chapter_number",
        &format!("attribute_changes[{index}].chapter_number"),
    )?;
    required_category(change, "category")?;
    required_string(change, "key")?;
    required_attribute_change_operation(change, "operation")?;
    if change.get("old_value").is_some() {
        return Err(format!(
            "attribute_changes[{index}].old_value 由系统计算，不应由 LLM 输出"
        ));
    }
    let new_value = required_value(change, "new_value")?;
    if new_value.is_null() {
        return Err(format!(
            "attribute_changes[{index}].new_value 不能为空；删除必须进入 deletion_candidates"
        ));
    }
    required_limited_string(
        change,
        "change_description",
        &format!("attribute_changes[{index}].change_description"),
        CHARACTER_SYNC_DESCRIPTION_MAX_CHARS,
    )?;
    required_limited_string(
        change,
        "event_cause",
        &format!("attribute_changes[{index}].event_cause"),
        CHARACTER_SYNC_DESCRIPTION_MAX_CHARS,
    )?;
    reject_legacy_anchor_field(
        change,
        "evidence_span_id",
        &format!("attribute_changes[{index}].evidence_span_id"),
        "evidence_anchor_id",
    )?;
    required_positive_i64(
        change,
        "evidence_anchor_id",
        &format!("attribute_changes[{index}].evidence_anchor_id"),
    )?;
    if change.get("evidence").is_some() {
        return Err(format!(
            "attribute_changes[{index}].evidence 已废弃；必须输出 evidence_anchor_id"
        ));
    }
    Ok(())
}

pub(super) fn validate_behavior(index: usize, behavior: &Value) -> Result<(), String> {
    if !behavior.is_object() {
        return Err(format!("behaviors[{index}] 必须是 JSON 对象"));
    }
    required_i32_at(
        behavior,
        "chapter_number",
        &format!("behaviors[{index}].chapter_number"),
    )?;
    required_string(behavior, "behavior_type")?;
    required_limited_string(
        behavior,
        "behavior_description",
        &format!("behaviors[{index}].behavior_description"),
        CHARACTER_SYNC_DESCRIPTION_MAX_CHARS,
    )?;
    reject_legacy_anchor_field(
        behavior,
        "original_text_span_id",
        &format!("behaviors[{index}].original_text_span_id"),
        "original_text_anchor_id",
    )?;
    required_positive_i64(
        behavior,
        "original_text_anchor_id",
        &format!("behaviors[{index}].original_text_anchor_id"),
    )?;
    if behavior.get("original_text").is_some() {
        return Err(format!(
            "behaviors[{index}].original_text 已废弃；必须输出 original_text_anchor_id"
        ));
    }
    required_limited_string_array(
        behavior,
        "behavior_tags",
        &format!("behaviors[{index}].behavior_tags"),
        CHARACTER_SYNC_BEHAVIOR_TAGS_MAX_ITEMS,
        CHARACTER_SYNC_DESCRIPTION_MAX_CHARS,
    )?;
    if behavior.get("classification_results").is_some() {
        return Err(format!(
            "behaviors[{index}] 不应包含 classification_results，行为分类由独立阶段生成"
        ));
    }
    required_limited_string_array(
        behavior,
        "related_characters",
        &format!("behaviors[{index}].related_characters"),
        CHARACTER_SYNC_RELATED_CHARACTERS_MAX_ITEMS,
        CHARACTER_SYNC_DESCRIPTION_MAX_CHARS,
    )?;
    optional_limited_string_or_null(
        behavior,
        "impact_analysis",
        &format!("behaviors[{index}].impact_analysis"),
        CHARACTER_SYNC_IMPACT_ANALYSIS_MAX_CHARS,
    )?;
    Ok(())
}

pub(super) fn validate_deletion_candidate(index: usize, candidate: &Value) -> Result<(), String> {
    if !candidate.is_object() {
        return Err(format!("deletion_candidates[{index}] 必须是 JSON 对象"));
    }
    required_i32_at(
        candidate,
        "chapter_number",
        &format!("deletion_candidates[{index}].chapter_number"),
    )?;
    required_category(candidate, "category")?;
    required_string(candidate, "key")?;
    required_limited_string(
        candidate,
        "reason",
        &format!("deletion_candidates[{index}].reason"),
        CHARACTER_SYNC_DESCRIPTION_MAX_CHARS,
    )?;
    reject_legacy_anchor_field(
        candidate,
        "evidence_span_id",
        &format!("deletion_candidates[{index}].evidence_span_id"),
        "evidence_anchor_id",
    )?;
    required_positive_i64(
        candidate,
        "evidence_anchor_id",
        &format!("deletion_candidates[{index}].evidence_anchor_id"),
    )?;
    if candidate.get("evidence").is_some() {
        return Err(format!(
            "deletion_candidates[{index}].evidence 已废弃；必须输出 evidence_anchor_id"
        ));
    }
    Ok(())
}

pub(super) fn validate_chapter_state(index: usize, state: &Value) -> Result<(), String> {
    if !state.is_object() {
        return Err(format!("chapter_states[{index}] 必须是 JSON 对象"));
    }
    required_i32_at(
        state,
        "chapter_number",
        &format!("chapter_states[{index}].chapter_number"),
    )?;
    required_limited_string(
        state,
        "state_summary",
        &format!("chapter_states[{index}].state_summary"),
        CHARACTER_SYNC_STATE_SUMMARY_MAX_CHARS,
    )?;
    optional_string_or_null(state, "current_location")?;
    optional_string_or_null(state, "current_goal")?;
    optional_string_or_null(state, "emotional_state")?;
    Ok(())
}

fn reject_legacy_anchor_field(
    value: &Value,
    old_key: &str,
    old_path: &str,
    new_key: &str,
) -> Result<(), String> {
    if value.get(old_key).is_some() {
        return Err(format!(
            "角色 AI 字段 {old_path} 已废弃；必须输出 {new_key}"
        ));
    }
    Ok(())
}

fn required_positive_i64(value: &Value, key: &str, path: &str) -> Result<i64, String> {
    let number = value
        .get(key)
        .ok_or_else(|| format!("角色 AI 字段 {path} 缺少字段"))?
        .as_i64()
        .ok_or_else(|| format!("角色 AI 字段 {path} 必须是 JSON 整数数字"))?;
    if number <= 0 {
        return Err(format!("角色 AI 字段 {path} 必须大于 0"));
    }
    Ok(number)
}

fn required_limited_string(
    value: &Value,
    key: &str,
    path: &str,
    max_chars: usize,
) -> Result<String, String> {
    let text = required_string(value, key)?;
    validate_char_limit(&text, path, max_chars)?;
    Ok(text)
}

fn optional_limited_string_or_null(
    value: &Value,
    key: &str,
    path: &str,
    max_chars: usize,
) -> Result<(), String> {
    optional_string_or_null(value, key)?;
    if let Some(text) = value.get(key).and_then(Value::as_str) {
        validate_char_limit(text.trim(), path, max_chars)?;
    }
    Ok(())
}

fn required_limited_string_array(
    value: &Value,
    key: &str,
    path: &str,
    max_items: usize,
    max_item_chars: usize,
) -> Result<(), String> {
    required_string_array(value, key)?;
    let values = required_array(value, key)?;
    if values.len() > max_items {
        return Err(format!(
            "角色 AI 字段 {path} 最多 {max_items} 项，实际 {} 项",
            values.len()
        ));
    }
    for (index, item) in values.iter().enumerate() {
        let text = item
            .as_str()
            .expect("required_string_array already validated strings")
            .trim();
        validate_char_limit(text, &format!("{path}[{index}]"), max_item_chars)?;
    }
    Ok(())
}

fn validate_char_limit(text: &str, path: &str, max_chars: usize) -> Result<(), String> {
    let count = text.chars().count();
    if count > max_chars {
        return Err(format!(
            "角色 AI 字段 {path} 过长：{count} 字，最多 {max_chars} 字；只引用关键原文或简短摘要"
        ));
    }
    Ok(())
}

pub(super) fn chapter_state_drafts(
    value: &Value,
    chapter_numbers: &[i32],
) -> Result<Vec<ChapterStateDraft>, String> {
    let expected = chapter_numbers.iter().copied().collect::<BTreeSet<_>>();
    let mut by_chapter = BTreeMap::new();
    for (index, state) in required_array(value, "chapter_states")?.iter().enumerate() {
        validate_chapter_state(index, state)?;
        let chapter_number = required_i32_at(
            state,
            "chapter_number",
            &format!("chapter_states[{index}].chapter_number"),
        )?;
        if !expected.contains(&chapter_number) {
            return Err(format!(
                "chapter_states[{index}].chapter_number 超出本次同步章节: 第 {chapter_number} 章"
            ));
        }
        let draft = ChapterStateDraft {
            chapter_number,
            state_summary: required_string(state, "state_summary")?,
            current_location: optional_string(state, "current_location"),
            current_goal: optional_string(state, "current_goal"),
            emotional_state: optional_string(state, "emotional_state"),
        };
        if by_chapter.insert(chapter_number, draft).is_some() {
            return Err(format!(
                "chapter_states 包含重复章节: 第 {chapter_number} 章"
            ));
        }
    }

    let mut missing = Vec::new();
    let mut drafts = Vec::with_capacity(chapter_numbers.len());
    for chapter_number in chapter_numbers {
        if let Some(draft) = by_chapter.remove(chapter_number) {
            drafts.push(draft);
        } else {
            missing.push(*chapter_number);
        }
    }
    if !missing.is_empty() {
        return Err(format!(
            "chapter_states 缺少章节状态: {}",
            missing
                .iter()
                .map(|chapter| format!("第 {chapter} 章"))
                .collect::<Vec<_>>()
                .join("、")
        ));
    }
    Ok(drafts)
}
