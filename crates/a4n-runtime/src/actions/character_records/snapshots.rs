use super::access::{
    require_profile_for_user, require_profile_snapshot_ref, validate_snapshot_range,
};
use super::contracts::{
    CheckProfileSyncConflictInput, DiffProfileChaptersInput, GetProfileStateAtChapterInput,
    GetSnapshotInput, ListSnapshotsInput, ProfileChapterStateSummary, ProfileSyncConflictSummary,
    SnapshotDeltaSummary, SnapshotReferenceSummary, SnapshotSummary,
};
use super::projections::{
    initial_snapshot_data, profile_state_summary_from_snapshot, required_text,
    snapshot_reference_summary,
};
use crate::state::AppState;
use a4n_core::event::snapshot::SnapshotStore;
use a4n_db::character::snapshot_store::Sha256SnapshotStore;

pub use rollback::rollback_to_chapter;

pub async fn list_snapshots(
    state: &AppState,
    user_id: i64,
    input: ListSnapshotsInput,
) -> Result<Vec<SnapshotReferenceSummary>, String> {
    validate_snapshot_range(input.start_chapter, input.end_chapter)?;
    let profile = require_profile_for_user(state, user_id, input.profile_id).await?;
    let snapshots = state
        .orchestrator
        .snapshot_references_by_profile(profile.id, input.start_chapter, input.end_chapter)
        .await
        .map_err(|error| error.to_string())?;
    snapshots.iter().map(snapshot_reference_summary).collect()
}

pub async fn get_profile_state_at_chapter(
    state: &AppState,
    user_id: i64,
    input: GetProfileStateAtChapterInput,
) -> Result<ProfileChapterStateSummary, String> {
    if input.chapter_number <= 0 {
        return Err("章节号必须大于 0".to_string());
    }
    require_profile_for_user(state, user_id, input.profile_id).await?;
    let snapshot =
        require_profile_snapshot_ref(state, input.profile_id, input.chapter_number, "").await?;
    profile_state_summary_from_snapshot(&snapshot)
}

pub async fn get_snapshot(
    state: &AppState,
    user_id: i64,
    input: GetSnapshotInput,
) -> Result<SnapshotSummary, String> {
    let content_hash = required_text(&input.content_hash, "快照哈希")?;
    let snapshot = state
        .snapshot_store
        .load_snapshot(&content_hash)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("快照不存在: {content_hash}"))?;
    let profile = require_profile_for_user(state, user_id, snapshot.entity_id).await?;
    if snapshot.entity_type != "protagonist_profile" {
        return Err(format!(
            "快照类型不匹配: expected protagonist_profile, got {}",
            snapshot.entity_type
        ));
    }
    Ok(SnapshotSummary {
        content_hash: snapshot.content_hash,
        entity_type: snapshot.entity_type,
        entity_id: profile.id,
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
    if input.from_chapter < 0 {
        return Err("起始章节号不能小于 0".to_string());
    }
    if input.to_chapter <= 0 {
        return Err("目标章节号必须大于 0".to_string());
    }
    if input.from_chapter > input.to_chapter {
        return Err("起始章节号不能大于目标章节号".to_string());
    }
    require_profile_for_user(state, user_id, input.profile_id).await?;

    let to_ref =
        require_profile_snapshot_ref(state, input.profile_id, input.to_chapter, "").await?;
    let to_snapshot = state
        .snapshot_store
        .load_snapshot(&to_ref.snapshot_hash)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("快照不存在: {}", to_ref.snapshot_hash))?;
    let (from_hash, from_data) = if input.from_chapter == 0 {
        ("initial-state".to_string(), initial_snapshot_data())
    } else {
        let from_ref =
            require_profile_snapshot_ref(state, input.profile_id, input.from_chapter, "").await?;
        let from_snapshot = state
            .snapshot_store
            .load_snapshot(&from_ref.snapshot_hash)
            .await
            .map_err(|err| err.to_string())?
            .ok_or_else(|| format!("快照不存在: {}", from_ref.snapshot_hash))?;
        (from_ref.snapshot_hash, from_snapshot.data)
    };

    Ok(SnapshotDeltaSummary {
        from_hash,
        to_hash: to_ref.snapshot_hash,
        delta: Sha256SnapshotStore::diff_snapshot_data(&from_data, &to_snapshot.data)
            .map_err(|err| err.to_string())?,
    })
}

pub async fn check_profile_sync_conflict(
    state: &AppState,
    user_id: i64,
    input: CheckProfileSyncConflictInput,
) -> Result<ProfileSyncConflictSummary, String> {
    let profile = require_profile_for_user(state, user_id, input.profile_id).await?;
    let available_chapters = state
        .orchestrator
        .character_sync_chapters(profile.project_id)
        .await
        .map_err(|err| err.to_string())?;
    let max_available_chapter = available_chapters
        .iter()
        .map(|chapter| chapter.chapter_number)
        .max()
        .unwrap_or_default();
    let snapshots = state
        .orchestrator
        .snapshot_references_by_profile(input.profile_id, None, Some(max_available_chapter))
        .await
        .map_err(|error| error.to_string())?;
    Ok(ProfileSyncConflictSummary {
        profile_id: input.profile_id,
        has_conflict: profile.last_synced_chapter > max_available_chapter,
        last_synced_chapter: profile.last_synced_chapter,
        max_available_chapter,
        available_snapshot_chapters: snapshots
            .iter()
            .map(|snapshot| snapshot.chapter_number)
            .collect(),
    })
}

mod rollback;
