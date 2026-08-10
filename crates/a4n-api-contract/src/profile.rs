use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

fn default_downstream_strategy() -> String {
    "reject".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ListObjectProfilesInput {
    pub project_id: i64,
}

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
    #[serde(default = "default_downstream_strategy")]
    pub downstream_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ObjectProfileSummary {
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
    pub generated_image_count: i64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ObjectProfileMutationResult {
    pub profile: ObjectProfileSummary,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ObjectProfileDeleteResult {
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ListSceneProfilesInput {
    pub project_id: i64,
}

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
    #[serde(default = "default_downstream_strategy")]
    pub downstream_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SceneProfileSummary {
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
    pub occurrence_count: u64,
    pub generated_image_count: i64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SceneProfileMutationResult {
    pub profile: SceneProfileSummary,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SceneProfileDeleteResult {
    #[serde(default)]
    pub warnings: Vec<String>,
}
