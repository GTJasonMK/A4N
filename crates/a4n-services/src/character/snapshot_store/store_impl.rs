use super::Sha256SnapshotStore;
use a4n_core::error::AppError;
use a4n_core::event::snapshot::{Snapshot, SnapshotDelta, SnapshotStore};
use a4n_core::models::character::snapshot_reference;
use async_trait::async_trait;
use sea_orm::*;

#[async_trait]
impl SnapshotStore for Sha256SnapshotStore {
    async fn store_snapshot(
        &self,
        entity_type: &str,
        entity_id: i64,
        sequence: i64,
        data: serde_json::Value,
    ) -> Result<Snapshot, AppError> {
        self.store_snapshot_in(self.db.as_ref(), entity_type, entity_id, sequence, data)
            .await
    }

    async fn load_snapshot(&self, content_hash: &str) -> Result<Option<Snapshot>, AppError> {
        let model = snapshot_reference::Entity::find()
            .filter(snapshot_reference::Column::SnapshotHash.eq(content_hash))
            .order_by_desc(snapshot_reference::Column::CreatedAt)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        Ok(model.map(|m| Snapshot {
            content_hash: m.snapshot_hash,
            entity_type: "character_profile".into(),
            entity_id: m.profile_id,
            sequence: m.chapter_number as i64,
            data: m.data,
            created_at: m.created_at,
        }))
    }

    async fn exists(&self, content_hash: &str) -> Result<bool, AppError> {
        let count = snapshot_reference::Entity::find()
            .filter(snapshot_reference::Column::SnapshotHash.eq(content_hash))
            .count(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        Ok(count > 0)
    }

    async fn delta_between(
        &self,
        from_hash: &str,
        to_hash: &str,
    ) -> Result<SnapshotDelta, AppError> {
        let from = self
            .load_snapshot(from_hash)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("快照不存在: {from_hash}")))?;
        let to = self
            .load_snapshot(to_hash)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("快照不存在: {to_hash}")))?;

        Ok(SnapshotDelta {
            from_hash: from_hash.to_string(),
            to_hash: to_hash.to_string(),
            delta: Self::diff_json(&from.data, &to.data)?,
        })
    }

    async fn gc(&self, referenced_hashes: &[String]) -> Result<usize, AppError> {
        if referenced_hashes.is_empty() {
            return Ok(0);
        }

        let result = snapshot_reference::Entity::delete_many()
            .filter(snapshot_reference::Column::SnapshotHash.is_not_in(referenced_hashes.to_vec()))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        Ok(result.rows_affected as usize)
    }
}
