use a4n_core::error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type BlueprintProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintSummary {
    pub id: i64,
    pub project_id: i64,
    pub title: String,
    pub genre: String,
    pub style: Option<String>,
    pub tone: Option<String>,
    pub target_audience: Option<String>,
    pub one_sentence_summary: Option<String>,
    pub full_synopsis: Option<String>,
    pub world_setting: Option<Value>,
    pub current_version: String,
    pub needs_part_outlines: bool,
    pub total_chapters: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub avatar_svg: Option<String>,
    pub avatar_animal: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintVersionSummary {
    pub id: i64,
    pub project_id: i64,
    pub version: String,
    pub change_description: Option<String>,
    pub blueprint_data: Value,
    pub world_setting_snapshot: Option<Value>,
    pub characters_snapshot: Option<Value>,
    pub relationships_snapshot: Option<Value>,
    pub synopsis_snapshot: Option<String>,
    pub created_at: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintDiffSummary {
    pub version_a: String,
    pub version_b: String,
    pub diff: Value,
}

#[derive(Debug, Clone, Copy)]
pub struct BlueprintTaskScope {
    pub project_id: i64,
    pub total: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateBlueprintInput {
    pub project_id: i64,
    pub title: String,
    pub genre: String,
    pub target_length: i32,
    pub task_id: Option<String>,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefineBlueprintInput {
    pub project_id: i64,
    pub focus: Option<String>,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiffBlueprintVersionsInput {
    pub project_id: i64,
    pub version_a: String,
    pub version_b: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreBlueprintVersionInput {
    pub project_id: i64,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateWorldSettingInput {
    pub project_id: i64,
    pub world_setting: Value,
}
