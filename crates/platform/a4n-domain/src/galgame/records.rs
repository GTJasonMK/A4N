use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct GalgameLineRecord {
    pub id: i64,
    pub scene_id: i64,
    pub line_index: i32,
    pub line_type: String,
    pub speaker_profile_id: Option<i64>,
    pub text: String,
    pub expression: Option<String>,
    pub pose: Option<String>,
    pub position: Option<String>,
    pub presentation_json: Value,
    pub text_anchor_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GalgameSceneRecord {
    pub id: i64,
    pub script_id: i64,
    pub source_snapshot_chapter_id: i64,
    pub adaptation_unit_id: Option<String>,
    pub adaptation_plan_id: Option<i64>,
    pub scene_index: i32,
    pub title: String,
    pub location: Option<String>,
    pub time_of_day: Option<String>,
    pub participants_json: Value,
    pub background_note: Option<String>,
    pub entry_kind: String,
    pub next_scene_id: Option<i64>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GalgameChoiceRecord {
    pub id: i64,
    pub line_id: i64,
    pub option_index: i32,
    pub option_text: String,
    pub target_scene_id: Option<i64>,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameGenerationSpecUpsertInput {
    pub canonical_key: String,
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_snapshot_id: i64,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub target_kind: String,
    pub target_entity_kind: String,
    pub target_entity_id: Option<i64>,
    pub params_json: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameAssetPlanSpecRecordInput {
    pub spec: GalgameGenerationSpecUpsertInput,
    pub text_anchor_ids: Vec<i64>,
    pub replace_entity_refs: bool,
    pub entity_profile_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GalgamePromptArtifactUpsertInput {
    pub generation_spec_id: i64,
    pub prompt_kind: String,
    pub prompt_text: String,
    pub renderer_version: String,
}
