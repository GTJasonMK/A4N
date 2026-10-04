use super::super::{galgame_bindable_media_asset, galgame_media_target_context_in};
use super::rules::{validate_galgame_media_asset_id, validate_galgame_media_target_ids};
use crate::galgame::{
    GalgameMediaAssetReadRecord, GalgameMediaBindingReadRecord, GalgameMediaBindingResult,
    GalgameMediaRecordError, require_galgame_media_text,
};
use a4n_db_models::models::media::media_binding;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set, TransactionTrait,
};

pub async fn bind_galgame_media_records<C>(
    db: &C,
    project_id: i64,
    media_asset_id: i64,
    target_kind: &str,
    target_id: i64,
    role: &str,
    required_media_kind: &str,
) -> Result<GalgameMediaBindingResult, GalgameMediaRecordError>
where
    C: ConnectionTrait + TransactionTrait,
{
    validate_galgame_media_asset_id(Some(media_asset_id))?;
    validate_galgame_media_target_ids(project_id, target_id)?;
    let target_kind = require_galgame_media_text(target_kind, "target_kind")?;
    let role = require_galgame_media_text(role, "role")?;
    let required_media_kind =
        require_galgame_media_text(required_media_kind, "required_media_kind")?;
    let tx = db
        .begin()
        .await
        .map_err(GalgameMediaRecordError::Database)?;
    let target_context =
        galgame_media_target_context_in(&tx, project_id, target_kind, target_id, true).await?;
    let asset = galgame_bindable_media_asset(
        &tx,
        project_id,
        media_asset_id,
        required_media_kind,
        Some(&target_context),
    )
    .await?;
    media_binding::Entity::delete_many()
        .filter(media_binding::Column::TargetKind.eq(target_kind.to_string()))
        .filter(media_binding::Column::TargetId.eq(target_id))
        .filter(media_binding::Column::Role.eq(role.to_string()))
        .exec(&tx)
        .await
        .map_err(GalgameMediaRecordError::Database)?;
    let binding = media_binding::ActiveModel {
        media_asset_id: Set(asset.id),
        target_kind: Set(target_kind.to_string()),
        target_id: Set(target_id),
        role: Set(role.to_string()),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&tx)
    .await
    .map_err(GalgameMediaRecordError::Database)?;
    tx.commit()
        .await
        .map_err(GalgameMediaRecordError::Database)?;
    Ok(GalgameMediaBindingResult {
        binding: GalgameMediaBindingReadRecord {
            id: binding.id,
            media_asset_id: binding.media_asset_id,
            target_kind: binding.target_kind,
            target_id: binding.target_id,
            role: binding.role,
        },
        asset: GalgameMediaAssetReadRecord {
            id: asset.id,
            media_kind: asset.media_kind,
            file_path: asset.file_path,
            thumbnail_path: asset.thumbnail_path,
        },
    })
}

pub async fn unbind_galgame_media_records<C>(
    db: &C,
    project_id: i64,
    target_kind: &str,
    target_id: i64,
    role: &str,
    media_asset_id: Option<i64>,
) -> Result<u64, GalgameMediaRecordError>
where
    C: ConnectionTrait + TransactionTrait,
{
    validate_galgame_media_target_ids(project_id, target_id)?;
    let target_kind = require_galgame_media_text(target_kind, "target_kind")?;
    let role = require_galgame_media_text(role, "role")?;
    let tx = db
        .begin()
        .await
        .map_err(GalgameMediaRecordError::Database)?;
    galgame_media_target_context_in(&tx, project_id, target_kind, target_id, true).await?;
    validate_galgame_media_asset_id(media_asset_id)?;
    let mut delete = media_binding::Entity::delete_many()
        .filter(media_binding::Column::TargetKind.eq(target_kind.to_string()))
        .filter(media_binding::Column::TargetId.eq(target_id))
        .filter(media_binding::Column::Role.eq(role.to_string()));
    if let Some(media_asset_id) = media_asset_id {
        delete = delete.filter(media_binding::Column::MediaAssetId.eq(media_asset_id));
    }
    let result = delete
        .exec(&tx)
        .await
        .map_err(GalgameMediaRecordError::Database)?;
    tx.commit()
        .await
        .map_err(GalgameMediaRecordError::Database)?;
    Ok(result.rows_affected)
}
