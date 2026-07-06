use super::Sha256SnapshotStore;
use a4n_core::error::AppError;
use a4n_core::event::snapshot::Snapshot;
use a4n_core::models::character::snapshot_reference;
use chrono::Utc;
use sea_orm::*;

impl Sha256SnapshotStore {
    pub async fn store_snapshot_in<C>(
        &self,
        db: &C,
        entity_type: &str,
        entity_id: i64,
        sequence: i64,
        data: serde_json::Value,
    ) -> Result<Snapshot, AppError>
    where
        C: ConnectionTrait,
    {
        let content_hash = Self::compute_hash(&data)?;
        let now = Utc::now().naive_utc();

        // 同一实体同一序列只保留一个引用，重新同步章节时覆盖为最新快照。
        let existing = snapshot_reference::Entity::find()
            .filter(snapshot_reference::Column::ProfileId.eq(entity_id))
            .filter(snapshot_reference::Column::ChapterNumber.eq(sequence as i32))
            .one(db)
            .await
            .map_err(AppError::Database)?;

        if let Some(existing) = existing {
            let mut active: snapshot_reference::ActiveModel = existing.clone().into();
            active.snapshot_hash = Set(content_hash.clone());
            active.reference_count = Set(1);
            active.data = Set(data.clone());
            let model = active.update(db).await.map_err(AppError::Database)?;
            return Ok(Snapshot {
                content_hash: model.snapshot_hash,
                entity_type: entity_type.to_string(),
                entity_id: model.profile_id,
                sequence: model.chapter_number as i64,
                data,
                created_at: model.created_at,
            });
        }

        let model = snapshot_reference::ActiveModel {
            profile_id: Set(entity_id),
            snapshot_hash: Set(content_hash.clone()),
            chapter_number: Set(sequence as i32),
            reference_count: Set(1),
            data: Set(data.clone()),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(AppError::Database)?;

        Ok(Snapshot {
            content_hash: model.snapshot_hash,
            entity_type: entity_type.to_string(),
            entity_id,
            sequence: model.chapter_number as i64,
            data,
            created_at: model.created_at,
        })
    }
}
