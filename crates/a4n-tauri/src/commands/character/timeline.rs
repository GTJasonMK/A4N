//! 角色快照时间线命令。
//!
//! Tauri 层只保留 command 签名；快照、差异和回滚逻辑统一在
//! `a4n_runtime::host_api::character_records`。

use a4n_runtime::AppState;
use a4n_runtime::host_api::character_records as character_records_actions;
use character_records_actions::{
    CheckProfileSyncConflictInput, DiffProfileChaptersInput, GetProfileStateAtChapterInput,
    GetSnapshotInput, ListSnapshotsInput, ProfileChapterStateSummary, ProfileSyncConflictSummary,
    RollbackResultSummary, RollbackToChapterInput, SnapshotDeltaSummary, SnapshotReferenceSummary,
    SnapshotSummary,
};
use tauri::State;

#[tauri::command]
pub async fn list_snapshots(
    state: State<'_, AppState>,
    user_id: i64,
    input: ListSnapshotsInput,
) -> Result<Vec<SnapshotReferenceSummary>, String> {
    character_records_actions::list_snapshots(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn check_profile_sync_conflict(
    state: State<'_, AppState>,
    user_id: i64,
    input: CheckProfileSyncConflictInput,
) -> Result<ProfileSyncConflictSummary, String> {
    character_records_actions::check_profile_sync_conflict(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn get_profile_state_at_chapter(
    state: State<'_, AppState>,
    user_id: i64,
    input: GetProfileStateAtChapterInput,
) -> Result<ProfileChapterStateSummary, String> {
    character_records_actions::get_profile_state_at_chapter(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn get_snapshot(
    state: State<'_, AppState>,
    user_id: i64,
    input: GetSnapshotInput,
) -> Result<SnapshotSummary, String> {
    character_records_actions::get_snapshot(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn diff_profile_chapters(
    state: State<'_, AppState>,
    user_id: i64,
    input: DiffProfileChaptersInput,
) -> Result<SnapshotDeltaSummary, String> {
    character_records_actions::diff_profile_chapters(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn rollback_to_chapter(
    state: State<'_, AppState>,
    user_id: i64,
    input: RollbackToChapterInput,
) -> Result<RollbackResultSummary, String> {
    character_records_actions::rollback_to_chapter(state.inner(), user_id, input).await
}
