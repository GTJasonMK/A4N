use super::super::{SourceSyncDbError, validate_positive_id};
use super::chapter_diff::diff_snapshot_chapter_records;
use super::contracts::SourceSnapshotDiffRecord;
use super::profile_diff::diff_snapshot_profile_records;
use sea_orm::ConnectionTrait;

pub async fn diff_source_snapshots<C>(
    db: &C,
    from_snapshot_id: i64,
    to_snapshot_id: i64,
) -> Result<SourceSnapshotDiffRecord, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(from_snapshot_id, "from_snapshot_id")?;
    validate_positive_id(to_snapshot_id, "to_snapshot_id")?;
    Ok(SourceSnapshotDiffRecord {
        chapter_changes: diff_snapshot_chapter_records(db, from_snapshot_id, to_snapshot_id)
            .await?,
        profile_changes: diff_snapshot_profile_records(db, from_snapshot_id, to_snapshot_id)
            .await?,
    })
}
