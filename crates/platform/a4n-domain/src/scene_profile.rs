//! Pure scene-profile values and normalization shared by scene owners.

use chrono::NaiveDateTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

use crate::profile_text::{
    normalize_aliases, normalize_text_set, required_text, validate_importance,
};

pub const SCENE_TYPES: &[&str] = &[
    "room",
    "building",
    "street",
    "city",
    "wilderness",
    "route",
    "landmark",
    "other",
];

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SceneProfileInput {
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub scene_type: String,
    pub importance: String,
    #[serde(default)]
    pub parent_scene_id: Option<i64>,
    pub base_description: String,
    pub visual_description: String,
    pub spatial_layout: String,
    pub atmosphere: String,
    #[serde(default)]
    pub entry_points_json: Option<Value>,
    #[serde(default)]
    pub rules_json: Option<Value>,
    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,
    #[serde(default)]
    pub evidence: Vec<String>,
    pub image_prompt_seed: String,
}

#[derive(Debug, thiserror::Error)]
pub enum SceneProfileInputError {
    #[error("{0}")]
    Invalid(String),
}

pub fn normalize_scene_type_value(value: &str) -> Result<String, SceneProfileInputError> {
    let value = required_text(value, "场景类型")
        .map_err(|error| SceneProfileInputError::Invalid(error.to_string()))?;
    if SCENE_TYPES.contains(&value.as_str()) {
        return Ok(value);
    }
    Err(SceneProfileInputError::Invalid(format!(
        "场景类型无效: {value}；允许值：{}",
        SCENE_TYPES.join("/")
    )))
}

pub fn normalize_scene_profile_record_input(
    input: SceneProfileInput,
) -> Result<SceneProfileRecordInput, SceneProfileInputError> {
    let normalized_name = input.name.trim().to_lowercase();
    if normalized_name.is_empty() {
        return Err(SceneProfileInputError::Invalid(
            "场景名称不能为空".to_string(),
        ));
    }
    let name = required_text(&input.name, "场景名称")
        .map_err(|error| SceneProfileInputError::Invalid(error.to_string()))?;
    let aliases = normalize_aliases(input.aliases);
    let scene_type = normalize_optional_scene_type(&input.scene_type)?;
    let importance = normalize_optional_importance(&input.importance)?;
    let base_description = input.base_description.trim().to_string();
    let visual_description = input.visual_description.trim().to_string();
    let spatial_layout = input.spatial_layout.trim().to_string();
    let atmosphere = input.atmosphere.trim().to_string();
    let image_prompt_seed = input.image_prompt_seed.trim().to_string();
    let entry_points_json = input
        .entry_points_json
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let rules_json = input.rules_json.unwrap_or_else(|| Value::Array(Vec::new()));
    if !entry_points_json.is_array() {
        return Err(SceneProfileInputError::Invalid(
            "场景出入口必须是 JSON 数组".to_string(),
        ));
    }
    if !rules_json.is_array() {
        return Err(SceneProfileInputError::Invalid(
            "场景规则必须是 JSON 数组".to_string(),
        ));
    }
    if input.first_seen_chapter <= 0 || input.last_seen_chapter <= 0 {
        return Err(SceneProfileInputError::Invalid(
            "场景首次/最近出现章节必须大于 0".to_string(),
        ));
    }
    if input.last_seen_chapter < input.first_seen_chapter {
        return Err(SceneProfileInputError::Invalid(
            "场景最近出现章节不能早于首次出现章节".to_string(),
        ));
    }
    let evidence = normalize_text_set(input.evidence);
    Ok(SceneProfileRecordInput {
        name,
        normalized_name,
        aliases_json: json!(aliases),
        scene_type,
        importance,
        parent_scene_id: input.parent_scene_id,
        base_description,
        visual_description,
        spatial_layout,
        atmosphere,
        entry_points_json,
        rules_json,
        first_seen_chapter: input.first_seen_chapter,
        last_seen_chapter: input.last_seen_chapter,
        evidence_json: json!(evidence),
        image_prompt_seed,
    })
}

fn normalize_optional_scene_type(value: &str) -> Result<String, SceneProfileInputError> {
    let value = value.trim();
    if value.is_empty() {
        Ok(String::new())
    } else {
        normalize_scene_type_value(value)
    }
}

fn normalize_optional_importance(value: &str) -> Result<String, SceneProfileInputError> {
    let value = value.trim();
    if value.is_empty() {
        Ok(String::new())
    } else {
        validate_importance(value, "场景重要性")
            .map_err(|error| SceneProfileInputError::Invalid(error.to_string()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneProfileRecordInput {
    pub name: String,
    pub normalized_name: String,
    pub aliases_json: Value,
    pub scene_type: String,
    pub importance: String,
    pub parent_scene_id: Option<i64>,
    pub base_description: String,
    pub visual_description: String,
    pub spatial_layout: String,
    pub atmosphere: String,
    pub entry_points_json: Value,
    pub rules_json: Value,
    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,
    pub evidence_json: Value,
    pub image_prompt_seed: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneProfileRecord {
    pub id: i64,
    pub project_id: i64,
    pub ownership_kind: String,
    pub name: String,
    pub normalized_name: String,
    pub aliases_json: Value,
    pub scene_type: String,
    pub importance: String,
    pub parent_scene_id: Option<i64>,
    pub base_description: String,
    pub visual_description: String,
    pub spatial_layout: String,
    pub atmosphere: String,
    pub entry_points_json: Value,
    pub rules_json: Value,
    pub current_state_json: Value,
    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,
    pub evidence_json: Value,
    pub image_prompt_seed: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneOccurrenceRecord {
    pub id: i64,
    pub project_id: i64,
    pub scene_profile_id: i64,
    pub chapter_id: i64,
    pub chapter_number: i32,
    pub occurrence_order: i32,
    pub timeline_position: Option<String>,
    pub summary: String,
    pub mood: Option<String>,
    pub weather_or_lighting: Option<String>,
    pub source_span_ids_json: Value,
    pub evidence_json: Value,
    pub characters_json: Value,
    pub objects_json: Value,
    pub key_events_json: Value,
    pub entry_state_json: Value,
    pub exit_state_json: Value,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneOccurrenceRecordInput {
    pub occurrence_ref: String,
    pub scene_ref: String,
    pub occurrence_order: i32,
    pub timeline_position: Option<String>,
    pub summary: String,
    pub mood: Option<String>,
    pub weather_or_lighting: Option<String>,
    pub source_span_ids_json: Value,
    pub evidence_json: Value,
    pub characters_json: Value,
    pub objects_json: Value,
    pub key_events_json: Value,
    pub entry_state_json: Value,
    pub exit_state_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneTransitionRecordInput {
    pub from_scene_ref: Option<String>,
    pub to_scene_ref: String,
    pub occurrence_ref: Option<String>,
    pub transition_order: i32,
    pub movement_type: String,
    pub route_description: String,
    pub time_cost: Option<String>,
    pub distance_hint: Option<String>,
    pub participants_json: Value,
    pub object_refs_json: Value,
    pub source_span_ids_json: Value,
    pub evidence_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEntityStateRecordInput {
    pub scene_ref: String,
    pub occurrence_ref: String,
    pub entity_type: String,
    pub entity_id: Option<i64>,
    pub entity_name_snapshot: String,
    pub state_key: String,
    pub state_before_json: Value,
    pub state_after_json: Value,
    pub position_description: Option<String>,
    pub action_description: String,
    pub source_span_ids_json: Value,
    pub evidence_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneProfileEvidenceRecordInput {
    pub scene_ref: String,
    pub text_anchor_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneProfileRecordDraft {
    pub scene_ref: String,
    pub scene_profile_id: Option<i64>,
    pub input: SceneProfileRecordInput,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SceneSyncRecordInput {
    pub profiles: Vec<SceneProfileRecordDraft>,
    pub occurrences: Vec<SceneOccurrenceRecordInput>,
    pub transitions: Vec<SceneTransitionRecordInput>,
    pub entity_states: Vec<SceneEntityStateRecordInput>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SceneSyncRecordResult {
    pub profile_ids: Vec<i64>,
    pub profile_ref_ids: BTreeMap<String, i64>,
    pub occurrence_count: usize,
    pub transition_count: usize,
    pub entity_state_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneProfileSyncRecordsInput {
    pub project_id: i64,
    pub chapter_id: i64,
    pub chapter_number: i32,
    pub source: crate::chapter::ChapterSourceSnapshot,
    pub records: SceneSyncRecordInput,
    pub profile_evidence: Vec<SceneProfileEvidenceRecordInput>,
    pub generation_run: crate::generation::RecordCompletedGenerationRunInput,
}
