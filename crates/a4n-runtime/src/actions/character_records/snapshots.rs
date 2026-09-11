use crate::state::AppState;
use a4n_api_contract::character::{
    CheckProfileSyncConflictInput, DiffProfileChaptersInput, GetProfileStateAtChapterInput,
    GetSnapshotInput, ListSnapshotsInput, ProfileChapterStateSummary, ProfileSyncConflictSummary,
    SnapshotDeltaSummary, SnapshotReferenceSummary, SnapshotSummary,
};

pub use rollback::rollback_to_chapter;

pub async fn list_snapshots(
    state: &AppState,
    user_id: i64,
    input: ListSnapshotsInput,
) -> Result<Vec<SnapshotReferenceSummary>, String> {
    state
        .character_profile_read_application()
        .snapshots(user_id, input)
        .await
        .map_err(crate::actions::app_error_message)
}

pub async fn get_profile_state_at_chapter(
    state: &AppState,
    user_id: i64,
    input: GetProfileStateAtChapterInput,
) -> Result<ProfileChapterStateSummary, String> {
    state
        .character_profile_read_application()
        .state_at_chapter(user_id, input)
        .await
        .map_err(crate::actions::app_error_message)
}

pub async fn get_snapshot(
    state: &AppState,
    user_id: i64,
    input: GetSnapshotInput,
) -> Result<SnapshotSummary, String> {
    state
        .character_profile_read_application()
        .snapshot_by_hash(user_id, input)
        .await
        .map_err(crate::actions::app_error_message)
}

pub async fn diff_profile_chapters(
    state: &AppState,
    user_id: i64,
    input: DiffProfileChaptersInput,
) -> Result<SnapshotDeltaSummary, String> {
    state
        .character_profile_read_application()
        .diff_chapters(user_id, input)
        .await
        .map_err(crate::actions::app_error_message)
}

pub async fn check_profile_sync_conflict(
    state: &AppState,
    user_id: i64,
    input: CheckProfileSyncConflictInput,
) -> Result<ProfileSyncConflictSummary, String> {
    state
        .character_profile_read_application()
        .sync_conflict(user_id, input)
        .await
        .map_err(crate::actions::app_error_message)
}

mod rollback;
