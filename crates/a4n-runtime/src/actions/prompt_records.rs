use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_orchestrator::prompt_records::PromptExecutionContext;
use a4n_prompt::{engine::PromptEntry, rendering::PromptRenderer};
use async_trait::async_trait;
use serde_json::Value;

use crate::AppState;

pub use a4n_orchestrator::prompt_records::{
    AbRenderResult, CategoryInfo, ProjectPromptOverrideSummary, PromptEntrySummary,
    PromptVersionSummary, RemoveOverrideInput, RenderInput, SetOverrideInput, UpdatePromptInput,
    ValidateVarsInput, VersionDiffInput,
};

pub async fn prompt_list(state: &AppState) -> Result<Vec<PromptEntrySummary>, String> {
    let context = prompt_context(state);
    state
        .orchestrator()
        .list_prompt_records_validated(&context)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_load_defaults(state: &AppState) -> Result<usize, String> {
    let context = prompt_context(state);
    state
        .orchestrator()
        .load_default_prompt_records_validated(&context)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_update(
    state: &AppState,
    input: UpdatePromptInput,
) -> Result<PromptEntrySummary, String> {
    let context = prompt_context(state);
    state
        .orchestrator()
        .update_prompt_record_validated(&context, input)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_reset(state: &AppState, name: &str) -> Result<(), String> {
    let context = prompt_context(state);
    state
        .orchestrator()
        .reset_prompt_record_validated(&context, name)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_version_history(
    state: &AppState,
    name: &str,
) -> Result<Vec<PromptVersionSummary>, String> {
    let context = prompt_context(state);
    state
        .orchestrator()
        .prompt_version_history_validated(&context, name)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_render(
    state: &AppState,
    user_id: i64,
    input: RenderInput,
) -> Result<String, String> {
    let context = prompt_context(state);
    state
        .orchestrator()
        .render_prompt_record_validated(&context, user_id, input)
        .await
        .map_err(prompt_error)
}

pub(crate) async fn render_prompt_template(
    state: &AppState,
    template_name: &str,
    variables: HashMap<String, Value>,
    project_id: Option<i64>,
) -> Result<String, String> {
    let context = prompt_context(state);
    state
        .orchestrator()
        .render_prompt_record_template_validated(&context, template_name, variables, project_id)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_render_ab(
    state: &AppState,
    user_id: i64,
    input: RenderInput,
) -> Result<AbRenderResult, String> {
    let context = prompt_context(state);
    state
        .orchestrator()
        .render_prompt_record_ab_validated(&context, user_id, input)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_set_project_override(
    state: &AppState,
    user_id: i64,
    input: SetOverrideInput,
) -> Result<(), String> {
    let context = prompt_context(state);
    state
        .orchestrator()
        .set_prompt_project_override_validated(&context, user_id, input)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_remove_project_override(
    state: &AppState,
    user_id: i64,
    input: RemoveOverrideInput,
) -> Result<(), String> {
    let context = prompt_context(state);
    state
        .orchestrator()
        .remove_prompt_project_override_validated(&context, user_id, input)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_list_project_overrides(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ProjectPromptOverrideSummary>, String> {
    let context = prompt_context(state);
    state
        .orchestrator()
        .list_prompt_project_overrides_validated(&context, user_id, project_id)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_version_diff(
    state: &AppState,
    input: VersionDiffInput,
) -> Result<String, String> {
    let context = prompt_context(state);
    state
        .orchestrator()
        .prompt_version_diff_validated(&context, input)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_validate_variables(
    state: &AppState,
    input: ValidateVarsInput,
) -> Result<Vec<String>, String> {
    let context = prompt_context(state);
    state
        .orchestrator()
        .validate_prompt_variables_validated(&context, input)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_list_categories(state: &AppState) -> Result<Vec<CategoryInfo>, String> {
    let context = prompt_context(state);
    state
        .orchestrator()
        .list_prompt_categories_validated(&context)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_get_runtime_entry(state: &AppState, name: &str) -> Result<PromptEntry, String> {
    let context = prompt_context(state);
    state
        .orchestrator()
        .prompt_runtime_entry_validated(&context, name)
        .await
        .map_err(prompt_error)
}

pub(crate) async fn prompt_list_runtime_entries(
    state: &AppState,
) -> Result<Vec<PromptEntry>, String> {
    let context = prompt_context(state);
    state
        .orchestrator()
        .prompt_runtime_entries_validated(&context)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_update_many_runtime(
    state: &AppState,
    updates: Vec<(String, String)>,
    author: &str,
) -> Result<Vec<PromptEntry>, String> {
    let context = prompt_context(state);
    state
        .orchestrator()
        .update_many_prompt_records_validated(&context, updates, author)
        .await
        .map_err(prompt_error)
}

#[async_trait]
impl PromptRenderer for AppState {
    async fn render_prompt(
        &self,
        template_name: &str,
        variables: &HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        render_prompt_template(self, template_name, variables.clone(), project_id)
            .await
            .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))
    }
}

fn prompt_context(state: &AppState) -> PromptExecutionContext {
    PromptExecutionContext::new(
        state.runtime_profile().uses_sidecars(),
        crate::runtime_config::current_user_id(),
        crate::runtime_config::redacted_current_snapshot_value(),
    )
}

fn prompt_error(error: AppError) -> String {
    error.to_string()
}
