use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use a4n_prompt_contract::PromptEntry;

/// Host-provided prompt rendering capability used by application owners.
#[async_trait::async_trait]
pub trait PromptRenderPort: Send + Sync {
    async fn render_prompt(
        &self,
        template_name: &str,
        variables: &HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, a4n_error::AppError>;
}

/// Runtime-supplied execution context for the prompt owner.
#[derive(Debug, Clone)]
pub struct PromptExecutionContext {
    use_sidecar: bool,
    user_id: Option<i64>,
    runtime_config_snapshot: Option<Value>,
}

/// Host-facing application boundary for prompt records and rendering.
///
/// The prompt owner chooses embedded versus sidecar execution and owns prompt
/// persistence; Runtime only supplies execution context and routes contracts.
#[async_trait::async_trait]
pub trait PromptApplicationPort: Send + Sync {
    async fn list_records(
        &self,
        context: &PromptExecutionContext,
    ) -> Result<Vec<PromptEntrySummary>, a4n_error::AppError>;

    async fn load_defaults(
        &self,
        context: &PromptExecutionContext,
    ) -> Result<usize, a4n_error::AppError>;

    async fn update_record(
        &self,
        context: &PromptExecutionContext,
        user_id: i64,
        input: UpdatePromptInput,
    ) -> Result<PromptEntrySummary, a4n_error::AppError>;

    async fn reset_record(
        &self,
        context: &PromptExecutionContext,
        name: &str,
    ) -> Result<(), a4n_error::AppError>;

    async fn version_history(
        &self,
        context: &PromptExecutionContext,
        name: &str,
    ) -> Result<Vec<PromptVersionSummary>, a4n_error::AppError>;

    async fn render_record(
        &self,
        context: &PromptExecutionContext,
        user_id: i64,
        input: RenderInput,
    ) -> Result<String, a4n_error::AppError>;

    async fn render_template(
        &self,
        context: &PromptExecutionContext,
        template_name: &str,
        variables: HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, a4n_error::AppError>;

    async fn render_ab(
        &self,
        context: &PromptExecutionContext,
        user_id: i64,
        input: RenderInput,
    ) -> Result<AbRenderResult, a4n_error::AppError>;

    async fn set_project_override(
        &self,
        context: &PromptExecutionContext,
        user_id: i64,
        input: SetOverrideInput,
    ) -> Result<(), a4n_error::AppError>;

    async fn remove_project_override(
        &self,
        context: &PromptExecutionContext,
        user_id: i64,
        input: RemoveOverrideInput,
    ) -> Result<(), a4n_error::AppError>;

    async fn list_project_overrides(
        &self,
        context: &PromptExecutionContext,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ProjectPromptOverrideSummary>, a4n_error::AppError>;

    async fn version_diff(
        &self,
        context: &PromptExecutionContext,
        input: VersionDiffInput,
    ) -> Result<String, a4n_error::AppError>;

    async fn validate_variables(
        &self,
        context: &PromptExecutionContext,
        input: ValidateVarsInput,
    ) -> Result<Vec<String>, a4n_error::AppError>;

    async fn list_categories(
        &self,
        context: &PromptExecutionContext,
    ) -> Result<Vec<CategoryInfo>, a4n_error::AppError>;

    async fn runtime_entry(
        &self,
        context: &PromptExecutionContext,
        name: &str,
    ) -> Result<PromptEntry, a4n_error::AppError>;

    async fn runtime_entries(
        &self,
        context: &PromptExecutionContext,
    ) -> Result<Vec<PromptEntry>, a4n_error::AppError>;

    async fn update_many_runtime(
        &self,
        context: &PromptExecutionContext,
        updates: Vec<(String, String)>,
        author: &str,
    ) -> Result<Vec<PromptEntry>, a4n_error::AppError>;
}

impl PromptExecutionContext {
    pub fn new(
        use_sidecar: bool,
        user_id: Option<i64>,
        runtime_config_snapshot: Option<Value>,
    ) -> Self {
        Self {
            use_sidecar,
            user_id,
            runtime_config_snapshot,
        }
    }

    pub fn use_sidecar(&self) -> bool {
        self.use_sidecar
    }

    pub fn user_id(&self) -> Option<i64> {
        self.user_id
    }

    pub fn runtime_config_snapshot(&self) -> Option<&Value> {
        self.runtime_config_snapshot.as_ref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PromptEntrySummary {
    pub name: String,
    pub title: String,
    pub content: String,
    pub description: String,
    pub tags: Vec<String>,
    pub category: String,
    pub dependencies: Vec<String>,
    pub is_modified: bool,
    pub version: String,
    pub used_by: Vec<String>,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PromptVersionSummary {
    pub version: String,
    pub timestamp: String,
    pub author: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AbRenderResult {
    pub current: String,
    pub original: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
#[schemars(extend("required" = ["project_id", "template_name", "variables"]))]
pub struct RenderInput {
    #[schemars(length(min = 1, max = 255))]
    pub template_name: String,
    pub variables: HashMap<String, Value>,
    #[serde(deserialize_with = "crate::serde_update::deserialize_required_nullable_field")]
    #[schemars(range(min = 1))]
    pub project_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UpdatePromptInput {
    #[schemars(length(min = 1, max = 255))]
    pub name: String,
    #[schemars(length(min = 1))]
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SetOverrideInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1, max = 255))]
    pub name: String,
    #[schemars(length(min = 1))]
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RemoveOverrideInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1, max = 255))]
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VersionDiffInput {
    #[schemars(length(min = 1, max = 255))]
    pub name: String,
    #[schemars(length(min = 1, max = 64))]
    pub version_a: String,
    #[schemars(length(min = 1, max = 64))]
    pub version_b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ValidateVarsInput {
    #[schemars(length(min = 1, max = 255))]
    pub template_name: String,
    pub variables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CategoryInfo {
    pub key: String,
    pub label: String,
    pub order: i32,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectPromptOverrideSummary {
    pub prompt_name: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PromptNameArgs {
    #[schemars(length(min = 1, max = 255))]
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::{RenderInput, UpdatePromptInput};

    #[test]
    fn render_scope_is_required_even_when_global() {
        serde_json::from_value::<RenderInput>(serde_json::json!({
            "template_name": "writing",
            "variables": {}
        }))
        .expect_err("project_id must be explicit");

        let input = serde_json::from_value::<RenderInput>(serde_json::json!({
            "template_name": "writing",
            "variables": {},
            "project_id": null
        }))
        .unwrap();
        assert_eq!(input.project_id, None);
    }

    #[test]
    fn update_author_is_owned_by_authenticated_host() {
        serde_json::from_value::<UpdatePromptInput>(serde_json::json!({
            "name": "writing",
            "content": "content",
            "author": "client"
        }))
        .expect_err("client author must be rejected");
    }
}
