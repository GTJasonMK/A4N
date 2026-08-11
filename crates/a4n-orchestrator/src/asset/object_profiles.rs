//! 物体档案应用合同。

use a4n_core::error::AppError;
use a4n_core::profile_text::{
    normalize_aliases, normalize_required_text_set, normalize_text_set, required_text,
    validate_importance,
};
use a4n_db::asset::ObjectProfileRecordInput;
use serde_json::json;
use std::collections::BTreeSet;

pub const OBJECT_TYPES: &[&str] = &[
    "prop",
    "tool",
    "weapon",
    "token",
    "vehicle",
    "clothing",
    "location_object",
    "environment_object",
    "other",
];

#[derive(Debug, Clone)]
pub struct ObjectProfileDraft {
    pub name: String,
    pub aliases: Vec<String>,
    pub object_type: String,
    pub importance: String,
    pub visual_description: String,
    pub material_and_state: String,
    pub story_usage: String,
    pub visual_usage: String,
    pub generation_usage: String,
    pub location_context: Option<String>,
    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,
    pub evidence: Vec<String>,
    pub image_prompt_seed: String,
}

/// 批量规范化物体档案草稿（含重复名检测），供 runtime 直接交给 orchestrator upsert。
pub fn normalize_object_profile_record_inputs(
    drafts: Vec<ObjectProfileDraft>,
) -> Result<Vec<ObjectProfileRecordInput>, AppError> {
    normalize_record_inputs(drafts)
}

fn normalize_record_inputs(
    drafts: Vec<ObjectProfileDraft>,
) -> Result<Vec<ObjectProfileRecordInput>, AppError> {
    let mut inputs = Vec::with_capacity(drafts.len());
    let mut seen = BTreeSet::new();
    for draft in drafts {
        let normalized = normalize_object_name(&draft.name)?;
        if !seen.insert(normalized.clone()) {
            return Err(AppError::InvalidArgument(format!(
                "物体发现结果存在重复物体: {}",
                draft.name
            )));
        }
        let draft = normalize_draft(draft)?;
        inputs.push(object_profile_record_input(draft, normalized));
    }
    Ok(inputs)
}

/// 将物体档案草稿规范化为 DB 写入 DTO（runtime 可直接交给 orchestrator）。
pub fn normalize_object_profile_record_input(
    draft: ObjectProfileDraft,
) -> Result<ObjectProfileRecordInput, AppError> {
    normalize_record_input(draft)
}

fn normalize_record_input(draft: ObjectProfileDraft) -> Result<ObjectProfileRecordInput, AppError> {
    let normalized = normalize_object_name(&draft.name)?;
    let draft = normalize_manual_draft(draft)?;
    Ok(object_profile_record_input(draft, normalized))
}

fn object_profile_record_input(
    draft: ObjectProfileDraft,
    normalized_name: String,
) -> ObjectProfileRecordInput {
    ObjectProfileRecordInput {
        name: draft.name,
        normalized_name,
        aliases_json: json!(draft.aliases),
        object_type: draft.object_type,
        importance: draft.importance,
        visual_description: draft.visual_description,
        material_and_state: draft.material_and_state,
        story_usage: draft.story_usage,
        visual_usage: draft.visual_usage,
        generation_usage: draft.generation_usage,
        location_context: draft.location_context,
        first_seen_chapter: draft.first_seen_chapter,
        last_seen_chapter: draft.last_seen_chapter,
        evidence_json: json!(draft.evidence),
        image_prompt_seed: draft.image_prompt_seed,
    }
}

pub fn normalize_object_name(name: &str) -> Result<String, AppError> {
    let normalized = name.trim().to_lowercase();
    if normalized.is_empty() {
        return Err(AppError::InvalidArgument("物体名称不能为空".to_string()));
    }
    Ok(normalized)
}

fn normalize_draft(mut draft: ObjectProfileDraft) -> Result<ObjectProfileDraft, AppError> {
    draft.name = required_text(&draft.name, "物体名称")?;
    draft.aliases = normalize_aliases(draft.aliases);
    draft.object_type = normalize_object_type_value(&draft.object_type)?;
    draft.importance = validate_importance(&draft.importance, "物体重要性")?;
    draft.visual_description = required_text(&draft.visual_description, "物体外观描述")?;
    draft.material_and_state = required_text(&draft.material_and_state, "物体材质与状态")?;
    draft.story_usage = required_text(&draft.story_usage, "物体叙事用途")?;
    draft.visual_usage = required_text(&draft.visual_usage, "物体视觉用途")?;
    draft.generation_usage = required_text(&draft.generation_usage, "物体生成用途")?;
    draft.location_context = draft
        .location_context
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    validate_object_chapter_range(&draft)?;
    draft.evidence = normalize_required_text_set(draft.evidence, "物体证据")?;
    draft.image_prompt_seed = required_text(&draft.image_prompt_seed, "物体图片提示词种子")?;
    Ok(draft)
}

fn normalize_manual_draft(mut draft: ObjectProfileDraft) -> Result<ObjectProfileDraft, AppError> {
    draft.name = required_text(&draft.name, "物体名称")?;
    draft.aliases = normalize_aliases(draft.aliases);
    draft.object_type = normalize_optional_object_type(&draft.object_type)?;
    draft.importance = normalize_optional_importance(&draft.importance, "物体重要性")?;
    draft.visual_description = draft.visual_description.trim().to_string();
    draft.material_and_state = draft.material_and_state.trim().to_string();
    draft.story_usage = draft.story_usage.trim().to_string();
    draft.visual_usage = draft.visual_usage.trim().to_string();
    draft.generation_usage = draft.generation_usage.trim().to_string();
    draft.location_context = draft
        .location_context
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    validate_object_chapter_range(&draft)?;
    draft.evidence = normalize_text_set(draft.evidence);
    draft.image_prompt_seed = draft.image_prompt_seed.trim().to_string();
    Ok(draft)
}

fn validate_object_chapter_range(draft: &ObjectProfileDraft) -> Result<(), AppError> {
    if draft.first_seen_chapter <= 0 || draft.last_seen_chapter <= 0 {
        return Err(AppError::InvalidArgument(
            "物体首次/最近出现章节必须大于 0".to_string(),
        ));
    }
    if draft.last_seen_chapter < draft.first_seen_chapter {
        return Err(AppError::InvalidArgument(
            "物体最近出现章节不能早于首次出现章节".to_string(),
        ));
    }
    Ok(())
}

fn normalize_optional_object_type(value: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Ok(String::new())
    } else {
        normalize_object_type_value(value)
    }
}

fn normalize_optional_importance(value: &str, field_name: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Ok(String::new())
    } else {
        validate_importance(value, field_name)
    }
}

pub fn normalize_object_type_value(value: &str) -> Result<String, AppError> {
    let value = required_text(value, "物体类型")?;
    if OBJECT_TYPES.contains(&value.as_str()) {
        return Ok(value);
    }
    Err(AppError::InvalidArgument(format!(
        "物体类型无效: {value}；允许值：{}",
        OBJECT_TYPES.join("/")
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_object_type_accepts_canonical_values_only() {
        assert_eq!(normalize_object_type_value(" prop ").unwrap(), "prop");
        assert_eq!(normalize_object_type_value("token").unwrap(), "token");
        assert_eq!(
            normalize_object_type_value("environment_object").unwrap(),
            "environment_object"
        );
        for alias in [
            "item",
            "equipment",
            "knife",
            "key-item",
            "vehicle object",
            "uniform",
            "furniture",
            "puddle",
            "关键道具",
        ] {
            let err =
                normalize_object_type_value(alias).expect_err("object type aliases must fail");
            assert!(
                matches!(err, AppError::InvalidArgument(message) if message.contains("物体类型无效")),
                "{alias} should be rejected"
            );
        }
    }

    #[test]
    fn normalize_object_type_rejects_unknown_values() {
        let err = normalize_object_type_value("memory")
            .expect_err("unknown object type should remain visible");
        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("物体类型无效"))
        );
    }

    #[test]
    fn manual_object_profile_allows_name_only() {
        let input = normalize_object_profile_record_input(ObjectProfileDraft {
            name: " 旧怀表 ".to_string(),
            aliases: Vec::new(),
            object_type: String::new(),
            importance: String::new(),
            visual_description: String::new(),
            material_and_state: String::new(),
            story_usage: String::new(),
            visual_usage: String::new(),
            generation_usage: String::new(),
            location_context: None,
            first_seen_chapter: 1,
            last_seen_chapter: 1,
            evidence: Vec::new(),
            image_prompt_seed: String::new(),
        })
        .expect("manual profile should accept sparse known data");

        assert_eq!(input.name, "旧怀表");
        assert!(input.object_type.is_empty());
        assert!(input.evidence_json.as_array().is_some_and(Vec::is_empty));
    }
}
