use super::super::SourceSyncDbError;
use super::contracts::SourceSnapshotProfileChangeRecord;
use a4n_core::models::project::source_snapshot_profile;
use a4n_domain::source_sync::SourceSyncChangeKind;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use serde_json::json;
use std::collections::HashMap;

pub(in crate::source_sync::snapshots) async fn diff_snapshot_profile_records<C>(
    db: &C,
    from_snapshot_id: i64,
    to_snapshot_id: i64,
) -> Result<Vec<SourceSnapshotProfileChangeRecord>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let from_rows = source_snapshot_profile::Entity::find()
        .filter(source_snapshot_profile::Column::SnapshotId.eq(from_snapshot_id))
        .all(db)
        .await?;
    let to_rows = source_snapshot_profile::Entity::find()
        .filter(source_snapshot_profile::Column::SnapshotId.eq(to_snapshot_id))
        .all(db)
        .await?;
    let key =
        |row: &source_snapshot_profile::Model| (row.profile_kind.clone(), row.source_profile_id);
    let to_by_profile = to_rows
        .iter()
        .map(|row| (key(row), row))
        .collect::<HashMap<_, _>>();
    let from_by_profile = from_rows
        .iter()
        .map(|row| (key(row), row))
        .collect::<HashMap<_, _>>();
    let mut changes = Vec::new();
    for old in &from_rows {
        match to_by_profile
            .get(&(old.profile_kind.clone(), old.source_profile_id))
            .copied()
        {
            Some(new) if new.payload_hash != old.payload_hash => {
                changes.push(SourceSnapshotProfileChangeRecord {
                    profile_kind: old.profile_kind.clone(),
                    source_profile_id: old.source_profile_id,
                    change_kind: SourceSyncChangeKind::Updated,
                    source_summary_json: json!({
                        "profile_kind": old.profile_kind,
                        "source_profile_id": old.source_profile_id,
                        "old_payload_hash": old.payload_hash,
                        "new_payload_hash": new.payload_hash,
                    }),
                });
            }
            None => changes.push(SourceSnapshotProfileChangeRecord {
                profile_kind: old.profile_kind.clone(),
                source_profile_id: old.source_profile_id,
                change_kind: SourceSyncChangeKind::Deleted,
                source_summary_json: json!({
                    "profile_kind": old.profile_kind,
                    "source_profile_id": old.source_profile_id,
                    "old_payload_hash": old.payload_hash,
                }),
            }),
            _ => {}
        }
    }
    for new in &to_rows {
        if !from_by_profile.contains_key(&(new.profile_kind.clone(), new.source_profile_id)) {
            changes.push(SourceSnapshotProfileChangeRecord {
                profile_kind: new.profile_kind.clone(),
                source_profile_id: new.source_profile_id,
                change_kind: SourceSyncChangeKind::Added,
                source_summary_json: json!({
                    "profile_kind": new.profile_kind,
                    "source_profile_id": new.source_profile_id,
                    "new_payload_hash": new.payload_hash,
                }),
            });
        }
    }
    Ok(changes)
}
