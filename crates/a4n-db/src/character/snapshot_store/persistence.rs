use super::Sha256SnapshotStore;
use crate::DatabaseConnectionTrait as ConnectionTrait;
use crate::character::CharacterSnapshotReferenceUpsertInput;
use a4n_core::error::AppError;
use a4n_core::event::snapshot::Snapshot;

impl Sha256SnapshotStore {
    pub(super) async fn store_snapshot_in<C>(
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
        let chapter_number = i32::try_from(sequence).map_err(|_| {
            AppError::InvalidArgument(format!("角色快照 sequence 超出 i32 章节号范围：{sequence}"))
        })?;
        let model = crate::character::upsert_snapshot_reference(
            db,
            CharacterSnapshotReferenceUpsertInput {
                profile_id: entity_id,
                chapter_number,
                snapshot_hash: content_hash,
                data,
            },
        )
        .await
        .map_err(AppError::Database)?;

        Ok(Snapshot {
            content_hash: model.snapshot_hash,
            entity_type: entity_type.to_string(),
            entity_id: model.profile_id,
            sequence: model.chapter_number as i64,
            data: model.data,
            created_at: model.created_at,
        })
    }
}
