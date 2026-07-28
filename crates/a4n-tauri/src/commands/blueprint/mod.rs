//! 故事蓝图命令。
//!
//! Tauri 层只保留命令签名；蓝图生成/优化统一走 generation task owner。

use crate::commands::request_preview::ModelRequestPreview;
use a4n_api_contract::{
    blueprint::{StartBlueprintGenerationInput, StartBlueprintRefinementInput},
    generation_tasks::{GenerationTaskHandle, HostErrorResponse},
};
use a4n_runtime::AppState;
use a4n_runtime::host_api::blueprint_records as blueprint_actions;
use blueprint_actions::{
    BlueprintDiffSummary, BlueprintSummary, BlueprintVersionSummary, DiffBlueprintVersionsInput,
    PreviewBlueprintGenerationInput, PreviewBlueprintRefinementInput, RestoreBlueprintVersionInput,
    UpdateWorldSettingInput,
};
use tauri::State;

#[tauri::command]
pub async fn get_blueprint(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<Option<BlueprintSummary>, String> {
    blueprint_actions::get_blueprint(state.inner(), user_id, project_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_blueprint_generation(
    state: State<'_, AppState>,
    user_id: i64,
    input: StartBlueprintGenerationInput,
) -> Result<GenerationTaskHandle, HostErrorResponse> {
    blueprint_actions::start_blueprint_generation_typed(state.inner(), user_id, input)
        .await
        .map_err(crate::commands::generation_tasks::map_access_error)
}

#[tauri::command]
pub async fn start_blueprint_refinement(
    state: State<'_, AppState>,
    user_id: i64,
    input: StartBlueprintRefinementInput,
) -> Result<GenerationTaskHandle, HostErrorResponse> {
    blueprint_actions::start_blueprint_refinement_typed(state.inner(), user_id, input)
        .await
        .map_err(crate::commands::generation_tasks::map_access_error)
}

#[tauri::command]
pub async fn preview_generate_blueprint_request(
    state: State<'_, AppState>,
    user_id: i64,
    input: PreviewBlueprintGenerationInput,
) -> Result<ModelRequestPreview, String> {
    blueprint_actions::preview_generate_blueprint_request(state.inner(), user_id, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn preview_refine_blueprint_request(
    state: State<'_, AppState>,
    user_id: i64,
    input: PreviewBlueprintRefinementInput,
) -> Result<ModelRequestPreview, String> {
    blueprint_actions::preview_refine_blueprint_request(state.inner(), user_id, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_blueprint_versions(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<BlueprintVersionSummary>, String> {
    blueprint_actions::list_blueprint_versions(state.inner(), user_id, project_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn diff_blueprint_versions(
    state: State<'_, AppState>,
    user_id: i64,
    input: DiffBlueprintVersionsInput,
) -> Result<BlueprintDiffSummary, String> {
    blueprint_actions::diff_blueprint_versions(state.inner(), user_id, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn restore_blueprint_version(
    state: State<'_, AppState>,
    user_id: i64,
    input: RestoreBlueprintVersionInput,
) -> Result<BlueprintSummary, String> {
    blueprint_actions::restore_blueprint_version(state.inner(), user_id, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_world_setting(
    state: State<'_, AppState>,
    user_id: i64,
    input: UpdateWorldSettingInput,
) -> Result<serde_json::Value, String> {
    blueprint_actions::update_world_setting(state.inner(), user_id, input)
        .await
        .map_err(|e| e.to_string())
}
