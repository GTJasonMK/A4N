use std::collections::HashMap;

use a4n_prompt::engine::PromptEntry;
use serde_json::Value;

use crate::AppState;

pub use a4n_orchestrator::prompt_records::{
    AbRenderInput, AbRenderResult, CategoryInfo, DependencyGraphOutput, PromptEntrySummary,
    PromptVersionSummary, RemoveOverrideInput, RenderInput, SetOverrideInput, UpdatePromptInput,
    ValidateVarsInput, VersionDiffInput,
};
pub type ProjectPromptOverrideSummary =
    a4n_orchestrator::prompt_override::ProjectPromptOverrideSummary;

pub async fn prompt_list(state: &AppState) -> Result<Vec<PromptEntrySummary>, String> {
    state.prompt_records().list().await.map_err(prompt_error)
}

pub async fn prompt_load_defaults(state: &AppState) -> Result<usize, String> {
    state
        .prompt_records()
        .load_defaults()
        .await
        .map_err(prompt_error)
}

pub async fn prompt_update(
    state: &AppState,
    input: UpdatePromptInput,
) -> Result<PromptEntrySummary, String> {
    state
        .prompt_records()
        .update(input)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_reset(state: &AppState, name: &str) -> Result<(), String> {
    state
        .prompt_records()
        .reset(name)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_version_history(
    state: &AppState,
    name: &str,
) -> Result<Vec<PromptVersionSummary>, String> {
    state
        .prompt_records()
        .version_history(name)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_render(
    state: &AppState,
    user_id: i64,
    input: RenderInput,
) -> Result<String, String> {
    state
        .prompt_records()
        .render(user_id, input)
        .await
        .map_err(prompt_error)
}

pub(crate) async fn render_prompt_template(
    state: &AppState,
    template_name: &str,
    variables: HashMap<String, Value>,
    project_id: Option<i64>,
) -> Result<String, String> {
    state
        .prompt_records()
        .render_template(template_name, variables, project_id)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_render_ab(
    state: &AppState,
    user_id: i64,
    input: AbRenderInput,
) -> Result<AbRenderResult, String> {
    state
        .prompt_records()
        .render_ab(user_id, input)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_set_project_override(
    state: &AppState,
    user_id: i64,
    input: SetOverrideInput,
) -> Result<(), String> {
    state
        .prompt_records()
        .set_project_override(user_id, input)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_remove_project_override(
    state: &AppState,
    user_id: i64,
    input: RemoveOverrideInput,
) -> Result<(), String> {
    state
        .prompt_records()
        .remove_project_override(user_id, input)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_list_project_overrides(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ProjectPromptOverrideSummary>, String> {
    state
        .prompt_records()
        .list_project_overrides(user_id, project_id)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_version_diff(
    state: &AppState,
    input: VersionDiffInput,
) -> Result<String, String> {
    state
        .prompt_records()
        .version_diff(input)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_validate_variables(
    state: &AppState,
    input: ValidateVarsInput,
) -> Result<Vec<String>, String> {
    state
        .prompt_records()
        .validate_variables(input)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_dependency_graph(state: &AppState) -> Result<DependencyGraphOutput, String> {
    state
        .prompt_records()
        .dependency_graph()
        .await
        .map_err(prompt_error)
}

pub async fn prompt_get_dependencies(state: &AppState, name: &str) -> Result<Vec<String>, String> {
    state
        .prompt_records()
        .dependencies(name)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_get_reverse_dependencies(
    state: &AppState,
    name: &str,
) -> Result<Vec<String>, String> {
    state
        .prompt_records()
        .reverse_dependencies(name)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_list_categories(state: &AppState) -> Result<Vec<CategoryInfo>, String> {
    state
        .prompt_records()
        .categories()
        .await
        .map_err(prompt_error)
}

pub async fn prompt_get_runtime_entry(state: &AppState, name: &str) -> Result<PromptEntry, String> {
    state
        .prompt_records()
        .runtime_entry(name)
        .await
        .map_err(prompt_error)
}

pub(crate) async fn prompt_list_runtime_entries(
    state: &AppState,
) -> Result<Vec<PromptEntry>, String> {
    state
        .prompt_records()
        .runtime_entries()
        .await
        .map_err(prompt_error)
}

pub async fn prompt_update_many_runtime(
    state: &AppState,
    updates: Vec<(String, String)>,
    author: &str,
) -> Result<Vec<PromptEntry>, String> {
    state
        .prompt_records()
        .update_many_runtime(updates, author)
        .await
        .map_err(prompt_error)
}

fn prompt_error(error: a4n_core::error::AppError) -> String {
    error.to_string()
}
