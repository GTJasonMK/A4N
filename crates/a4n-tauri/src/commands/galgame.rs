//! Galgame commands.

use crate::progress_task::WorkflowProgressTask;
use a4n_core::event::channel::TaskType;
use a4n_core::models::galgame::{galgame_choice, galgame_line, galgame_scene};
use a4n_runtime::AppState;
use a4n_runtime::host_api::export_files::ExportFileData;
use a4n_runtime::host_api::galgame_records;
use a4n_runtime::host_api::galgame_records::{
    BindGalgameMediaInput, DeleteGalgameLineInput, GALGAME_PIPELINE_TOTAL_STEPS,
    GalgameAssetGenerationSummary, GalgameAssetPlan, GalgameMediaBindingResult,
    GalgamePipelineProgressUpdate, GalgamePipelineStatusSummary, GalgameProjectInput,
    GalgameScriptDetail, ImportGalgameAudioInput, InsertGalgameLineInput, ReorderGalgameLinesInput,
    RunGalgameAssetGenerationInput, StartGalgamePipelineInput, UnbindGalgameMediaInput,
    UpdateGalgameChoicesInput, UpdateGalgameLineInput, UpdateGalgameSceneInput,
};
use a4n_runtime::host_api::request_preview::ModelRequestPreview;
use std::sync::Arc;
use tauri::{State, Window};

#[tauri::command]
pub async fn get_galgame_script(
    state: State<'_, AppState>,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<GalgameScriptDetail, String> {
    galgame_records::get_galgame_script(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn start_galgame_pipeline(
    window: Window,
    state: State<'_, AppState>,
    user_id: i64,
    input: StartGalgamePipelineInput,
) -> Result<GalgameScriptDetail, String> {
    let progress_task = Arc::new(WorkflowProgressTask::new(
        window,
        state.inner(),
        input.task_id.clone(),
        TaskType::GalgameGeneration,
        Some(input.project_id),
        GALGAME_PIPELINE_TOTAL_STEPS,
        "正在准备 Galgame 管线",
    ));
    let reporter_task = progress_task.clone();
    let progress_reporter = Arc::new(move |event: GalgamePipelineProgressUpdate| {
        reporter_task.progress_with_meta(
            event.current,
            event.total,
            &event.message,
            Some(event.meta),
        );
        Ok(())
    });
    let result = galgame_records::start_galgame_pipeline_with_progress(
        state.inner(),
        user_id,
        input,
        Some(progress_reporter),
    )
    .await;
    match &result {
        Ok(_) => progress_task.complete("Galgame 管线已完成"),
        Err(err) => progress_task.error("galgame_pipeline_failed", err),
    }
    progress_task.remove(state.inner());
    result
}

#[tauri::command]
pub async fn cancel_galgame_pipeline(
    state: State<'_, AppState>,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<GalgamePipelineStatusSummary, String> {
    galgame_records::cancel_galgame_pipeline(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn preview_galgame_pipeline_request(
    state: State<'_, AppState>,
    user_id: i64,
    input: StartGalgamePipelineInput,
) -> Result<ModelRequestPreview, String> {
    galgame_records::preview_galgame_pipeline_request(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn get_galgame_pipeline_status(
    state: State<'_, AppState>,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<GalgamePipelineStatusSummary, String> {
    galgame_records::get_galgame_pipeline_status(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn update_galgame_line(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    input: UpdateGalgameLineInput,
) -> Result<galgame_line::Model, String> {
    galgame_records::update_galgame_line(state.inner(), user_id, project_id, input).await
}

#[tauri::command]
pub async fn insert_galgame_line(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    input: InsertGalgameLineInput,
) -> Result<galgame_line::Model, String> {
    galgame_records::insert_galgame_line(state.inner(), user_id, project_id, input).await
}

#[tauri::command]
pub async fn update_galgame_scene(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    input: UpdateGalgameSceneInput,
) -> Result<galgame_scene::Model, String> {
    galgame_records::update_galgame_scene(state.inner(), user_id, project_id, input).await
}

#[tauri::command]
pub async fn delete_galgame_line(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    input: DeleteGalgameLineInput,
) -> Result<(), String> {
    galgame_records::delete_galgame_line(state.inner(), user_id, project_id, input).await
}

#[tauri::command]
pub async fn reorder_galgame_lines(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    input: ReorderGalgameLinesInput,
) -> Result<Vec<galgame_line::Model>, String> {
    galgame_records::reorder_galgame_lines(state.inner(), user_id, project_id, input).await
}

#[tauri::command]
pub async fn update_galgame_choices(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    input: UpdateGalgameChoicesInput,
) -> Result<Vec<galgame_choice::Model>, String> {
    galgame_records::update_galgame_choices(state.inner(), user_id, project_id, input).await
}

#[tauri::command]
pub async fn plan_galgame_assets(
    state: State<'_, AppState>,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<GalgameAssetPlan, String> {
    galgame_records::plan_galgame_assets(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn run_galgame_asset_generation(
    state: State<'_, AppState>,
    user_id: i64,
    input: RunGalgameAssetGenerationInput,
) -> Result<GalgameAssetGenerationSummary, String> {
    galgame_records::run_galgame_asset_generation(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn bind_galgame_media(
    state: State<'_, AppState>,
    user_id: i64,
    input: BindGalgameMediaInput,
) -> Result<GalgameMediaBindingResult, String> {
    galgame_records::bind_galgame_media(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn unbind_galgame_media(
    state: State<'_, AppState>,
    user_id: i64,
    input: UnbindGalgameMediaInput,
) -> Result<u64, String> {
    galgame_records::unbind_galgame_media(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn import_galgame_audio(
    state: State<'_, AppState>,
    user_id: i64,
    input: ImportGalgameAudioInput,
) -> Result<GalgameMediaBindingResult, String> {
    galgame_records::import_galgame_audio(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn export_galgame_webgal(
    state: State<'_, AppState>,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<ExportFileData, String> {
    galgame_records::export_galgame_webgal(state.inner(), user_id, input).await
}
