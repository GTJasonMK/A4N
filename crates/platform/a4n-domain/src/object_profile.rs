//! Pure object-profile values and normalization shared by object owners.

use chrono::NaiveDateTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashSet;

use crate::profile_text::{
    normalize_aliases, normalize_required_text_set, normalize_text_set, required_text,
    validate_importance,
};

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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObjectProfileInput {
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub object_type: String,
    pub importance: String,
    pub visual_description: String,
    pub material_and_state: String,
    pub story_usage: String,
    pub visual_usage: String,
    pub generation_usage: String,
    #[serde(default)]
    pub location_context: Option<String>,
    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,
    #[serde(default)]
    pub evidence: Vec<String>,
    pub image_prompt_seed: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ObjectProfileInputError {
    #[error("{0}")]
    Invalid(String),
}

pub fn normalize_object_profile_record_input(
    draft: ObjectProfileInput,
) -> Result<ObjectProfileRecordInput, ObjectProfileInputError> {
    let normalized = normalize_object_name(&draft.name)?;
    let draft = normalize_manual_draft(draft)?;
    Ok(object_profile_record_input(draft, normalized))
}

pub fn normalize_object_profile_record_inputs(
    drafts: Vec<ObjectProfileInput>,
) -> Result<Vec<ObjectProfileRecordInput>, ObjectProfileInputError> {
    let mut inputs = Vec::with_capacity(drafts.len());
    let mut seen = HashSet::new();
    for draft in drafts {
        let normalized = normalize_object_name(&draft.name)?;
        if !seen.insert(normalized.clone()) {
            return Err(ObjectProfileInputError::Invalid(format!(
                "物体发现结果存在重复物体: {}",
                draft.name
            )));
        }
        let draft = normalize_discovery_draft(draft)?;
        inputs.push(object_profile_record_input(draft, normalized));
    }
    Ok(inputs)
}

pub fn normalize_object_name(name: &str) -> Result<String, ObjectProfileInputError> {
    let normalized = name.trim().to_lowercase();
    if normalized.is_empty() {
        return Err(ObjectProfileInputError::Invalid(
            "物体名称不能为空".to_string(),
        ));
    }
    Ok(normalized)
}

pub fn normalize_object_type_value(value: &str) -> Result<String, ObjectProfileInputError> {
    let value = required_text(value, "物体类型")
        .map_err(|error| ObjectProfileInputError::Invalid(error.to_string()))?;
    if OBJECT_TYPES.contains(&value.as_str()) {
        return Ok(value);
    }
    Err(ObjectProfileInputError::Invalid(format!(
        "物体类型无效: {value}；允许值：{}",
        OBJECT_TYPES.join("/")
    )))
}

fn object_profile_record_input(
    draft: ObjectProfileInput,
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

fn normalize_discovery_draft(
    mut draft: ObjectProfileInput,
) -> Result<ObjectProfileInput, ObjectProfileInputError> {
    draft.name = required_text(&draft.name, "物体名称")
        .map_err(|error| ObjectProfileInputError::Invalid(error.to_string()))?;
    draft.aliases = normalize_aliases(draft.aliases);
    draft.object_type = normalize_object_type_value(&draft.object_type)?;
    draft.importance = validate_importance(&draft.importance, "物体重要性")
        .map_err(|error| ObjectProfileInputError::Invalid(error.to_string()))?;
    draft.visual_description = required_text(&draft.visual_description, "物体外观描述")
        .map_err(|error| ObjectProfileInputError::Invalid(error.to_string()))?;
    draft.material_and_state = required_text(&draft.material_and_state, "物体材质与状态")
        .map_err(|error| ObjectProfileInputError::Invalid(error.to_string()))?;
    draft.story_usage = required_text(&draft.story_usage, "物体叙事用途")
        .map_err(|error| ObjectProfileInputError::Invalid(error.to_string()))?;
    draft.visual_usage = required_text(&draft.visual_usage, "物体视觉用途")
        .map_err(|error| ObjectProfileInputError::Invalid(error.to_string()))?;
    draft.generation_usage = required_text(&draft.generation_usage, "物体生成用途")
        .map_err(|error| ObjectProfileInputError::Invalid(error.to_string()))?;
    normalize_chapter_range(&draft)?;
    draft.evidence = normalize_required_text_set(draft.evidence, "物体证据")
        .map_err(|error| ObjectProfileInputError::Invalid(error.to_string()))?;
    draft.image_prompt_seed = required_text(&draft.image_prompt_seed, "物体图片提示词种子")
        .map_err(|error| ObjectProfileInputError::Invalid(error.to_string()))?;
    Ok(draft)
}

fn normalize_manual_draft(
    mut draft: ObjectProfileInput,
) -> Result<ObjectProfileInput, ObjectProfileInputError> {
    draft.name = required_text(&draft.name, "物体名称")
        .map_err(|error| ObjectProfileInputError::Invalid(error.to_string()))?;
    draft.aliases = normalize_aliases(draft.aliases);
    draft.object_type = normalize_optional_object_type(&draft.object_type)?;
    draft.importance = normalize_optional_importance(&draft.importance, "物体重要性")?;
    for value in [
        &mut draft.visual_description,
        &mut draft.material_and_state,
        &mut draft.story_usage,
        &mut draft.visual_usage,
        &mut draft.generation_usage,
        &mut draft.image_prompt_seed,
    ] {
        *value = value.trim().to_string();
    }
    draft.location_context = draft
        .location_context
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    normalize_chapter_range(&draft)?;
    draft.evidence = normalize_text_set(draft.evidence);
    Ok(draft)
}

fn normalize_chapter_range(draft: &ObjectProfileInput) -> Result<(), ObjectProfileInputError> {
    if draft.first_seen_chapter <= 0 || draft.last_seen_chapter <= 0 {
        return Err(ObjectProfileInputError::Invalid(
            "物体首次/最近出现章节必须大于 0".to_string(),
        ));
    }
    if draft.last_seen_chapter < draft.first_seen_chapter {
        return Err(ObjectProfileInputError::Invalid(
            "物体最近出现章节不能早于首次出现章节".to_string(),
        ));
    }
    Ok(())
}

fn normalize_optional_object_type(value: &str) -> Result<String, ObjectProfileInputError> {
    let value = value.trim();
    if value.is_empty() {
        Ok(String::new())
    } else {
        normalize_object_type_value(value)
    }
}

fn normalize_optional_importance(
    value: &str,
    field_name: &str,
) -> Result<String, ObjectProfileInputError> {
    let value = value.trim();
    if value.is_empty() {
        Ok(String::new())
    } else {
        validate_importance(value, field_name)
            .map_err(|error| ObjectProfileInputError::Invalid(error.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct ObjectProfileEvidenceRecordInput {
    pub normalized_name: String,
    pub text_anchor_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct ObjectProfileRecordInput {
    pub name: String,
    pub normalized_name: String,
    pub aliases_json: Value,
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
    pub evidence_json: Value,
    pub image_prompt_seed: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObjectProfileRecord {
    pub id: i64,
    pub project_id: i64,
    pub ownership_kind: String,
    pub name: String,
    pub normalized_name: String,
    pub aliases_json: Value,
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
    pub evidence_json: Value,
    pub image_prompt_seed: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile_material::resolve_downstream_image_ids;

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
            let error =
                normalize_object_type_value(alias).expect_err("object type aliases must fail");
            assert!(error.to_string().contains("物体类型无效"), "{alias}");
        }
    }

    #[test]
    fn manual_object_profile_allows_name_only() {
        let input = normalize_object_profile_record_input(ObjectProfileInput {
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

    #[test]
    fn downstream_strategy_preserves_existing_cleanup_contract() {
        let ids = vec![1, 2];
        assert!(
            resolve_downstream_image_ids(
                ids.clone(),
                crate::DownstreamConflictStrategy::Reject,
                "档案",
                "修改"
            )
            .is_err()
        );
        assert!(
            resolve_downstream_image_ids(
                ids.clone(),
                crate::DownstreamConflictStrategy::KeepDownstream,
                "档案",
                "修改"
            )
            .unwrap()
            .is_empty()
        );
        assert_eq!(
            resolve_downstream_image_ids(
                ids.clone(),
                crate::DownstreamConflictStrategy::DeleteDownstream,
                "档案",
                "修改"
            )
            .unwrap(),
            ids
        );
    }
}
