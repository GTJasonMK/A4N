use a4n_runtime::actions::storage_paths;
use storage_paths::{
    CleanupOrphanProjectDataDirsInput, OrphanProjectDataDirsSummary, StoragePathsSummary,
};
use tauri::State;

use a4n_runtime::AppState;

#[tauri::command]
pub fn get_storage_paths(state: State<'_, AppState>) -> Result<StoragePathsSummary, String> {
    Ok(storage_paths::get_storage_paths(&state))
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
