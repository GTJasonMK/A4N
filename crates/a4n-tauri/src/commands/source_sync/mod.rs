use a4n_runtime::host_api::source_sync;
use tauri::State;

use a4n_runtime::AppState;

use source_sync::{ApplySourceSyncInput, SourceSyncPlanSummary, SourceSyncRunSummary};

#[tauri::command]
pub async fn preview_source_sync(
    state: State<'_, AppState>,
    user_id: i64,
    derived_project_id: i64,
) -> Result<SourceSyncPlanSummary, String> {
    source_sync::preview_source_sync(&state, user_id, derived_project_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn apply_source_sync_decisions(
    state: State<'_, AppState>,
    user_id: i64,
    input: ApplySourceSyncInput,
) -> Result<SourceSyncRunSummary, String> {
    source_sync::apply_source_sync_decisions(&state, user_id, input)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn detach_derived_project(
    state: State<'_, AppState>,
    user_id: i64,
    derived_project_id: i64,
) -> Result<a4n_core::models::project::novel_project::Model, String> {
    source_sync::detach_derived_project(&state, user_id, derived_project_id)
        .await
        .map_err(|err| err.to_string())
}
