//! 基于 protagonist_snapshot 表的快照读取与差异计算。

mod diff;

use crate::DbHandle;
use a4n_domain::snapshot::Snapshot;
use a4n_error::AppError;

/// SHA256 快照存储
pub struct Sha256SnapshotStore {
    db: DbHandle,
}

impl Sha256SnapshotStore {
    pub fn new(db: DbHandle) -> Self {
        Self { db }
    }

    pub fn diff_snapshot_data(
        from: &serde_json::Value,
        to: &serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        Self::diff_json(from, to)
    }

    pub async fn load_snapshot(&self, content_hash: &str) -> Result<Option<Snapshot>, AppError> {
        let model = super::snapshot_reference_by_hash(&self.db, content_hash)
            .await
            .map_err(AppError::database)?;

        Ok(model.map(|model| Snapshot {
            content_hash: model.snapshot_hash,
            entity_type: "character_profile".into(),
            entity_id: model.profile_id,
            sequence: i64::from(model.chapter_number),
            data: model.data,
            created_at: model.created_at,
        }))
    }
}
