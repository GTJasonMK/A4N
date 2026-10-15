use super::generation_tasks::GenerationStartRequestMeta;
use a4n_domain::project::{
    BLUEPRINT_MAX_TOTAL_CHAPTERS, BLUEPRINT_MIN_GENERATION_CHAPTERS, BLUEPRINT_MIN_TOTAL_CHAPTERS,
};
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
    #[schemars(range(
        min = BLUEPRINT_MIN_GENERATION_CHAPTERS,
        max = BLUEPRINT_MAX_TOTAL_CHAPTERS
    ))]
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
    #[schemars(range(
        min = BLUEPRINT_MIN_GENERATION_CHAPTERS,
        max = BLUEPRINT_MAX_TOTAL_CHAPTERS
    ))]
    pub target_length: i32,
    pub variables: Option<BTreeMap<String, Value>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreviewBlueprintRefinementInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(max = 32768))]
    pub focus: Option<String>,
    pub variables: Option<BTreeMap<String, Value>>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ListBlueprintVersionsInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
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
    #[schemars(length(min = 1, max = 64))]
    pub expected_current_version: Option<String>,
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
    #[schemars(range(
        min = BLUEPRINT_MIN_TOTAL_CHAPTERS,
        max = BLUEPRINT_MAX_TOTAL_CHAPTERS
    ))]
    pub total_chapters: i32,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateWorldSettingInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1, max = 64))]
    pub expected_current_version: String,
    pub world_setting: Value,
}

#[derive(Serialize, Deserialize, JsonSchema)]
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

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct BlueprintVersionSummary {
    pub id: i64,
    pub version: String,
    pub change_description: String,
    pub created_at: String,
    pub created_by: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct BlueprintDiffSummary {
    pub diff: Value,
}

#[derive(Serialize, JsonSchema)]
pub struct BlueprintMutationResult {
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
pub struct VersionedBlueprintMutationResult {
    pub current_version: String,
    pub warnings: Vec<String>,
}

/// Host-facing blueprint read boundary.
///
/// Permission checks, persistence reads and API projections stay with the
/// blueprint owner; Runtime only routes the typed reads.
#[async_trait::async_trait]
pub trait BlueprintReadApplicationPort: Send + Sync {
    async fn summary(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Option<BlueprintSummary>, a4n_error::AppError>;

    async fn versions(
        &self,
        user_id: i64,
        input: ListBlueprintVersionsInput,
    ) -> Result<Vec<BlueprintVersionSummary>, a4n_error::AppError>;

    async fn diff(
        &self,
        user_id: i64,
        input: DiffBlueprintVersionsInput,
    ) -> Result<BlueprintDiffSummary, a4n_error::AppError>;
}

/// Host-facing blueprint mutation boundary.
///
/// Permission checks, durable writes and version events stay with the
/// blueprint owner; Runtime only routes the typed commands and handles its
/// separate RAG warning path.
#[async_trait::async_trait]
pub trait BlueprintMutationApplicationPort: Send + Sync {
    async fn save(
        &self,
        user_id: i64,
        input: SaveBlueprintInput,
    ) -> Result<VersionedBlueprintMutationResult, a4n_error::AppError>;

    async fn update_world_setting(
        &self,
        user_id: i64,
        input: UpdateWorldSettingInput,
    ) -> Result<VersionedBlueprintMutationResult, a4n_error::AppError>;

    async fn restore(
        &self,
        user_id: i64,
        input: RestoreBlueprintVersionInput,
    ) -> Result<(), a4n_error::AppError>;
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{PreviewBlueprintGenerationInput, PreviewBlueprintRefinementInput};

    #[test]
    fn blueprint_preview_variables_require_the_canonical_object_shape() {
        assert!(
            serde_json::from_value::<PreviewBlueprintGenerationInput>(json!({
                "project_id": 7,
                "title": "雾城",
                "genre": "悬疑",
                "target_length": 30,
                "variables": ["legacy"]
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<PreviewBlueprintRefinementInput>(json!({
                "project_id": 7,
                "focus": "加强动机",
                "variables": "legacy"
            }))
            .is_err()
        );
    }
}
