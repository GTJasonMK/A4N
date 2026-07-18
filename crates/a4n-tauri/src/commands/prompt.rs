//! 提示词管理命令。
//!
//! 保持 `commands::prompt::*` 的 Tauri 命令入口稳定，具体实现委托 runtime actions。

use a4n_runtime::AppState;
use a4n_runtime::host_api::prompt_records::{
    self as prompt_records_actions, AbRenderInput, AbRenderResult, CategoryInfo,
    DependencyGraphOutput, ProjectPromptOverrideSummary, PromptEntrySummary, PromptVersionSummary,
    RemoveOverrideInput, RenderInput, SetOverrideInput, UpdatePromptInput, ValidateVarsInput,
    VersionDiffInput,
};
use tauri::State;

#[tauri::command]
pub async fn prompt_render(
    state: State<'_, AppState>,
    user_id: i64,
    input: RenderInput,
) -> Result<String, String> {
    prompt_records_actions::prompt_render(&state, user_id, input).await
}

#[tauri::command]
pub async fn prompt_list(state: State<'_, AppState>) -> Result<Vec<PromptEntrySummary>, String> {
    prompt_records_actions::prompt_list(&state).await
}

#[tauri::command]
pub async fn prompt_load_defaults(state: State<'_, AppState>) -> Result<usize, String> {
    prompt_records_actions::prompt_load_defaults(&state).await
}

#[tauri::command]
pub async fn prompt_update(
    state: State<'_, AppState>,
    input: UpdatePromptInput,
) -> Result<PromptEntrySummary, String> {
    prompt_records_actions::prompt_update(&state, input).await
}

#[tauri::command]
pub async fn prompt_reset(state: State<'_, AppState>, name: String) -> Result<(), String> {
    prompt_records_actions::prompt_reset(&state, &name).await
}

#[tauri::command]
pub async fn prompt_version_history(
    state: State<'_, AppState>,
    name: String,
) -> Result<Vec<PromptVersionSummary>, String> {
    prompt_records_actions::prompt_version_history(&state, &name).await
}

#[tauri::command]
pub async fn prompt_set_project_override(
    state: State<'_, AppState>,
    user_id: i64,
    input: SetOverrideInput,
) -> Result<(), String> {
    prompt_records_actions::prompt_set_project_override(&state, user_id, input).await
}

#[tauri::command]
pub async fn prompt_remove_project_override(
    state: State<'_, AppState>,
    user_id: i64,
    input: RemoveOverrideInput,
) -> Result<(), String> {
    prompt_records_actions::prompt_remove_project_override(&state, user_id, input).await
}

#[tauri::command]
pub async fn prompt_version_diff(
    state: State<'_, AppState>,
    input: VersionDiffInput,
) -> Result<String, String> {
    prompt_records_actions::prompt_version_diff(&state, input).await
}

#[tauri::command]
pub async fn prompt_validate_variables(
    state: State<'_, AppState>,
    input: ValidateVarsInput,
) -> Result<Vec<String>, String> {
    prompt_records_actions::prompt_validate_variables(&state, input).await
}

#[tauri::command]
pub async fn prompt_dependency_graph(
    state: State<'_, AppState>,
) -> Result<DependencyGraphOutput, String> {
    prompt_records_actions::prompt_dependency_graph(&state).await
}

#[tauri::command]
pub async fn prompt_render_ab(
    state: State<'_, AppState>,
    user_id: i64,
    input: AbRenderInput,
) -> Result<AbRenderResult, String> {
    prompt_records_actions::prompt_render_ab(&state, user_id, input).await
}

#[tauri::command]
pub async fn prompt_get_dependencies(
    state: State<'_, AppState>,
    name: String,
) -> Result<Vec<String>, String> {
    prompt_records_actions::prompt_get_dependencies(&state, &name).await
}

#[tauri::command]
pub async fn prompt_get_reverse_dependencies(
    state: State<'_, AppState>,
    name: String,
) -> Result<Vec<String>, String> {
    prompt_records_actions::prompt_get_reverse_dependencies(&state, &name).await
}

#[tauri::command]
pub async fn prompt_list_project_overrides(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ProjectPromptOverrideSummary>, String> {
    prompt_records_actions::prompt_list_project_overrides(&state, user_id, project_id).await
}

#[tauri::command]
pub async fn prompt_list_categories(
    state: State<'_, AppState>,
) -> Result<Vec<CategoryInfo>, String> {
    prompt_records_actions::prompt_list_categories(&state).await
}
