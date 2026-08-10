use super::generation_tasks::GenerationStartRequestMeta;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Start a structured blueprint generation through the canonical generation task path.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StartBlueprintGenerationInput {
    pub meta: GenerationStartRequestMeta,
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1, max = 1024))]
    pub title: String,
    #[schemars(length(min = 1, max = 256))]
    pub genre: String,
    #[schemars(range(min = 1))]
    pub target_length: i32,
    pub variables: Option<BTreeMap<String, Value>>,
}

/// Start a structured blueprint refinement through the canonical generation task path.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StartBlueprintRefinementInput {
    pub meta: GenerationStartRequestMeta,
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(max = 32768))]
    pub focus: Option<String>,
    pub variables: Option<BTreeMap<String, Value>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreviewBlueprintGenerationInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1, max = 1024))]
    pub title: String,
    #[schemars(length(min = 1, max = 256))]
    pub genre: String,
    #[schemars(range(min = 1))]
    pub target_length: i32,
    pub variables: Option<Value>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreviewBlueprintRefinementInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(max = 32768))]
    pub focus: Option<String>,
    pub variables: Option<Value>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListBlueprintVersionsInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiffBlueprintVersionsInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1, max = 64))]
    pub version_a: String,
    #[schemars(length(min = 1, max = 64))]
    pub version_b: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RestoreBlueprintVersionInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1, max = 64))]
    pub version: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveBlueprintInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1, max = 256))]
    pub genre: String,
    #[schemars(length(max = 1024))]
    pub style: Option<String>,
    #[schemars(length(max = 1024))]
    pub tone: Option<String>,
    #[schemars(length(max = 1024))]
    pub target_audience: Option<String>,
    #[schemars(length(max = 4096))]
    pub one_sentence_summary: Option<String>,
    #[schemars(length(max = 131072))]
    pub full_synopsis: Option<String>,
    #[schemars(range(min = 5, max = 10000))]
    pub total_chapters: i32,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateWorldSettingInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub world_setting: Value,
}

#[derive(Serialize, JsonSchema)]
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
    pub total_chapters: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub source_review_status: String,
}

#[derive(Serialize, JsonSchema)]
pub struct BlueprintVersionSummary {
    pub id: i64,
    pub version: String,
    pub change_description: Option<String>,
    pub created_at: String,
    pub created_by: Option<String>,
}

#[derive(Serialize, JsonSchema)]
pub struct BlueprintDiffSummary {
    pub diff: Value,
}

#[derive(Serialize, JsonSchema)]
pub struct BlueprintMutationResult {
    pub warnings: Vec<String>,
}
