use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use crate::character::{
    optional_string, optional_string_or_null, project_character_attribute_changes, required_array,
    required_attribute_change_operation, required_category, required_i32_at, required_string,
    required_value,
};

use super::fields::{
    optional_limited_string_or_null, reject_legacy_anchor_field, required_limited_string,
    required_limited_string_array, required_positive_i64,
};
use crate::character::profile_sync_stages::contracts::{
    CharacterProfileSyncChapterStateDraft, CharacterProfileSyncExtraction,
};

const CHARACTER_SYNC_DESCRIPTION_MAX_CHARS: usize = 120;
const CHARACTER_SYNC_IMPACT_ANALYSIS_MAX_CHARS: usize = 160;
const CHARACTER_SYNC_STATE_SUMMARY_MAX_CHARS: usize = 160;
const CHARACTER_SYNC_BEHAVIOR_TAGS_MAX_ITEMS: usize = 5;
const CHARACTER_SYNC_RELATED_CHARACTERS_MAX_ITEMS: usize = 6;

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

pub(super) fn chapter_state_drafts(
    value: &Value,
    chapter_numbers: &[i32],
) -> Result<Vec<CharacterProfileSyncChapterStateDraft>, String> {
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
        let draft = CharacterProfileSyncChapterStateDraft {
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

pub(super) fn chapter_state_drafts_json(states: &[CharacterProfileSyncChapterStateDraft]) -> Value {
    Value::Array(
        states
            .iter()
            .map(|state| {
                json!({
                    "chapter_number": state.chapter_number,
                    "state_summary": state.state_summary,
                    "current_location": state.current_location,
                    "current_goal": state.current_goal,
                    "emotional_state": state.emotional_state,
                })
            })
            .collect(),
    )
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

pub(super) fn validate_materialized_character_sync_projection(
    extraction: &CharacterProfileSyncExtraction,
    baseline_state: &Value,
) -> Result<(), String> {
    project_character_attribute_changes(baseline_state.clone(), &extraction.changes)?;
    Ok(())
}
