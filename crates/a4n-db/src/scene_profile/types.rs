use sea_orm::DbErr;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum SceneProfileDbError {
    #[error("数据库错误: {0}")]
    Database(#[from] DbErr),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidArgument(String),
}

impl From<crate::generation::GenerationDbError> for SceneProfileDbError {
    fn from(error: crate::generation::GenerationDbError) -> Self {
        match error {
            crate::generation::GenerationDbError::InvalidArgument(message) => {
                Self::InvalidArgument(message)
            }
            crate::generation::GenerationDbError::NotFound(message) => Self::NotFound(message),
            crate::generation::GenerationDbError::Database(error) => Self::Database(error),
        }
    }
}

pub type SceneProfileDbResult<T> = Result<T, SceneProfileDbError>;

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct SceneProfileRecordDraft {
    pub scene_ref: String,
    pub scene_profile_id: Option<i64>,
    pub input: SceneProfileRecordInput,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, Default)]
pub struct SceneSyncRecordInput {
    pub profiles: Vec<SceneProfileRecordDraft>,
    pub occurrences: Vec<SceneOccurrenceRecordInput>,
    pub transitions: Vec<SceneTransitionRecordInput>,
    pub entity_states: Vec<SceneEntityStateRecordInput>,
}

#[derive(Debug, Clone, Default)]
pub struct SceneSyncRecordResult {
    pub profile_ids: Vec<i64>,
    pub profile_ref_ids: BTreeMap<String, i64>,
    pub occurrence_count: usize,
    pub transition_count: usize,
    pub entity_state_count: usize,
}
