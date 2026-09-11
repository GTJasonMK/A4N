use crate::character::{CharacterDbError, validate_timeline_source_kind};
use a4n_domain::character::InsertBehaviorRecordInput;
use serde_json::Value;

pub(super) fn validate_behavior_classification_results(
    classification_results: &Value,
) -> Result<(), CharacterDbError> {
    let object = classification_results.as_object().ok_or_else(|| {
        CharacterDbError::InvalidArgument("行为分类结果必须是 JSON 对象".to_string())
    })?;
    for (key, value) in object {
        if key.trim().is_empty() {
            return Err(CharacterDbError::InvalidArgument(
                "行为分类结果包含空属性 key".to_string(),
            ));
        }
        let Some(text) = value.as_str().map(str::trim) else {
            return Err(CharacterDbError::InvalidArgument(format!(
                "行为分类结果 {key} 必须是 conform/non-conform"
            )));
        };
        if !matches!(text, "conform" | "non-conform") {
            return Err(CharacterDbError::InvalidArgument(format!(
                "行为分类结果 {key} 必须是 conform/non-conform"
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_behavior_record_input(
    input: InsertBehaviorRecordInput,
) -> Result<InsertBehaviorRecordInput, CharacterDbError> {
    if input.chapter_number <= 0 {
        return Err(CharacterDbError::InvalidArgument(
            "行为记录章节号必须大于 0".to_string(),
        ));
    }
    let source_kind = validate_timeline_source_kind(&input.source_kind)?;
    let behavior_type = input.behavior_type.trim().to_string();
    let behavior_description = input.behavior_description.trim().to_string();
    let original_text = input.original_text.trim().to_string();
    if behavior_type.is_empty() {
        return Err(CharacterDbError::InvalidArgument(
            "行为类型不能为空".to_string(),
        ));
    }
    if behavior_description.is_empty() {
        return Err(CharacterDbError::InvalidArgument(
            "行为描述不能为空".to_string(),
        ));
    }
    if original_text.is_empty() {
        return Err(CharacterDbError::InvalidArgument(
            "行为原文不能为空".to_string(),
        ));
    }
    if !input.behavior_tags.is_array() {
        return Err(CharacterDbError::InvalidArgument(
            "行为标签必须是 JSON 数组".to_string(),
        ));
    }
    if let Some(related_characters) = input.related_characters.as_ref()
        && !related_characters.is_array()
    {
        return Err(CharacterDbError::InvalidArgument(
            "相关角色必须是 JSON 数组".to_string(),
        ));
    }
    validate_behavior_classification_results(&input.classification_results)?;

    Ok(InsertBehaviorRecordInput {
        profile_id: input.profile_id,
        chapter_number: input.chapter_number,
        source_kind,
        behavior_type,
        behavior_description,
        original_text,
        behavior_tags: input.behavior_tags,
        classification_results: input.classification_results,
        related_characters: input.related_characters,
        impact_analysis: input.impact_analysis,
    })
}
