use super::contracts::{
    CheckProfileSyncConflictInput, DiffProfileChaptersInput, GetProfileStateAtChapterInput,
    GetSnapshotInput, ListSnapshotsInput, ProfileChapterStateSummary, ProfileSyncConflictSummary,
    SnapshotDeltaSummary, SnapshotReferenceSummary, SnapshotSummary,
};
use super::projections::{profile_state_summary_from_snapshot, snapshot_reference_summary};
use crate::state::AppState;

pub use rollback::rollback_to_chapter;

pub async fn list_snapshots(
    state: &AppState,
    user_id: i64,
    input: ListSnapshotsInput,
) -> Result<Vec<SnapshotReferenceSummary>, String> {
    let snapshots = state
        .character()
        .snapshot_references_for_user_validated(
            user_id,
            input.profile_id,
            input.start_chapter,
            input.end_chapter,
        )
        .await
        .map_err(|error| error.to_string())?;
    snapshots.iter().map(snapshot_reference_summary).collect()
}

pub async fn get_profile_state_at_chapter(
    state: &AppState,
    user_id: i64,
    input: GetProfileStateAtChapterInput,
) -> Result<ProfileChapterStateSummary, String> {
    let snapshot = state
        .character()
        .profile_snapshot_for_user_at_chapter_validated(
            user_id,
            input.profile_id,
            input.chapter_number,
        )
        .await
        .map_err(|error| error.to_string())?;
    profile_state_summary_from_snapshot(&snapshot)
}

pub async fn get_snapshot(
    state: &AppState,
    user_id: i64,
    input: GetSnapshotInput,
) -> Result<SnapshotSummary, String> {
    let snapshot = state
        .character()
        .character_snapshot_for_user_by_hash_validated(user_id, &input.content_hash)
        .await
        .map_err(|error| error.to_string())?;
    Ok(SnapshotSummary {
        content_hash: snapshot.content_hash,
        entity_type: snapshot.entity_type,
        entity_id: snapshot.entity_id,
        sequence: snapshot.sequence,
        data: snapshot.data,
        created_at: Some(snapshot.created_at.to_string()),
    })
}

pub async fn diff_profile_chapters(
    state: &AppState,
    user_id: i64,
    input: DiffProfileChaptersInput,
) -> Result<SnapshotDeltaSummary, String> {
    let delta = state
        .character()
        .diff_profile_snapshots_for_user_validated(
            user_id,
            input.profile_id,
            input.from_chapter,
            input.to_chapter,
        )
        .await
        .map_err(|error| error.to_string())?;
    Ok(SnapshotDeltaSummary {
        from_hash: delta.from_hash,
        to_hash: delta.to_hash,
        delta: delta.delta,
    })
}

pub async fn check_profile_sync_conflict(
    state: &AppState,
    user_id: i64,
    input: CheckProfileSyncConflictInput,
) -> Result<ProfileSyncConflictSummary, String> {
    let conflict = state
        .character()
        .character_sync_conflict_for_user_validated(user_id, input.profile_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(ProfileSyncConflictSummary {
        profile_id: conflict.profile_id,
        has_conflict: conflict.has_conflict,
        last_synced_chapter: conflict.last_synced_chapter,
        max_available_chapter: conflict.max_available_chapter,
        available_snapshot_chapters: conflict.available_snapshot_chapters,
    })
}

mod rollback;
