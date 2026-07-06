use a4n_core::error::AppError;
use a4n_core::models::character::protagonist_behavior_record;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use super::CharacterService;

#[derive(Debug)]
pub struct BehaviorRecordInput<'a> {
    pub profile_id: i64,
    pub chapter_number: i32,
    pub behavior_type: &'a str,
    pub behavior_description: &'a str,
    pub original_text: &'a str,
    pub behavior_tags: serde_json::Value,
    pub classification_results: serde_json::Value,
    pub related_characters: Option<serde_json::Value>,
    pub impact_analysis: Option<&'a str>,
}

pub(super) fn validate_behavior_classification_results(
    classification_results: &serde_json::Value,
) -> Result<(), AppError> {
    let object = classification_results
        .as_object()
        .ok_or_else(|| AppError::InvalidArgument("行为分类结果必须是 JSON 对象".to_string()))?;
    for (key, value) in object {
        if key.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                "行为分类结果包含空属性 key".to_string(),
            ));
        }
        let Some(text) = value.as_str().map(str::trim) else {
            return Err(AppError::InvalidArgument(format!(
                "行为分类结果 {key} 必须是 conform/non-conform"
            )));
        };
        if !matches!(text, "conform" | "non-conform") {
            return Err(AppError::InvalidArgument(format!(
                "行为分类结果 {key} 必须是 conform/non-conform"
            )));
        }
    }
    Ok(())
}
impl CharacterService {
    pub async fn record_behavior_in<C>(
        &self,
        db: &C,
        input: BehaviorRecordInput<'_>,
    ) -> Result<protagonist_behavior_record::Model, AppError>
    where
        C: ConnectionTrait,
    {
        let BehaviorRecordInput {
            profile_id,
            chapter_number,
            behavior_type,
            behavior_description,
            original_text,
            behavior_tags,
            classification_results,
            related_characters,
            impact_analysis,
        } = input;
        if chapter_number <= 0 {
            return Err(AppError::InvalidArgument(
                "行为记录章节号必须大于 0".to_string(),
            ));
        }
        let behavior_type = behavior_type.trim();
        let behavior_description = behavior_description.trim();
        let original_text = original_text.trim();
        if behavior_type.is_empty() {
            return Err(AppError::InvalidArgument("行为类型不能为空".to_string()));
        }
        if behavior_description.is_empty() {
            return Err(AppError::InvalidArgument("行为描述不能为空".to_string()));
        }
        if original_text.is_empty() {
            return Err(AppError::InvalidArgument("行为原文不能为空".to_string()));
        }
        if !behavior_tags.is_array() {
            return Err(AppError::InvalidArgument(
                "行为标签必须是 JSON 数组".to_string(),
            ));
        }
        if let Some(related_characters) = related_characters.as_ref()
            && !related_characters.is_array()
        {
            return Err(AppError::InvalidArgument(
                "相关角色必须是 JSON 数组".to_string(),
            ));
        }
        validate_behavior_classification_results(&classification_results)?;

        let now = Utc::now().naive_utc();
        protagonist_behavior_record::ActiveModel {
            profile_id: Set(profile_id),
            chapter_number: Set(chapter_number),
            behavior_type: Set(behavior_type.to_string()),
            behavior_description: Set(behavior_description.to_string()),
            original_text: Set(original_text.to_string()),
            behavior_tags: Set(behavior_tags),
            classification_results: Set(classification_results),
            related_characters: Set(related_characters),
            impact_analysis: Set(impact_analysis.map(|s| s.to_string())),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(AppError::Database)
    }
}
