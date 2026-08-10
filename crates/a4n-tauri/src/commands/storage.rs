use a4n_runtime::host_api::storage_paths;
use storage_paths::{
    CleanupOrphanProjectDataDirsInput, OrphanProjectDataDirsSummary, StoragePathsSummary,
};
use tauri::State;

use a4n_runtime::AppState;

#[tauri::command]
pub async fn get_storage_paths(
    state: State<'_, AppState>,
    caller_user_id: i64,
) -> Result<StoragePathsSummary, String> {
    storage_paths::get_storage_paths(&state, caller_user_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_orphan_project_data_dirs(
    state: State<'_, AppState>,
    caller_user_id: i64,
) -> Result<OrphanProjectDataDirsSummary, String> {
    storage_paths::list_orphan_project_data_dirs(&state, caller_user_id).await
}

#[tauri::command]
pub async fn cleanup_orphan_project_data_dirs(
    state: State<'_, AppState>,
    input: CleanupOrphanProjectDataDirsInput,
) -> Result<OrphanProjectDataDirsSummary, String> {
    storage_paths::cleanup_orphan_project_data_dirs(&state, input).await
}
