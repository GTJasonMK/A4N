use crate::actions::dependency_conflict::DownstreamConflictStrategy;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListSceneProfilesInput {
    pub project_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
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
    #[serde(default)]
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneProfileSummary {
    pub id: i64,
    pub project_id: i64,
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
    pub occurrence_count: u64,
    pub generated_image_count: i64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}
