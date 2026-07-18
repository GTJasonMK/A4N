use super::*;

pub(in crate::actions::character_ai) fn validate_attribute_change(
    index: usize,
    change: &Value,
) -> Result<(), String> {
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

pub(in crate::actions::character_ai) fn validate_behavior(
    index: usize,
    behavior: &Value,
) -> Result<(), String> {
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
