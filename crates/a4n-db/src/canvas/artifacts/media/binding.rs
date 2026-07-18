use crate::canvas::CanvasDbError;
use a4n_core::models::media::media_binding;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter,
    QuerySelect, Set,
};

pub(in crate::canvas::artifacts) async fn ensure_canvas_media_binding<C>(
    db: &C,
    media_asset_id: i64,
    target_kind: &str,
    target_id: i64,
    role: &str,
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
    let mut query = media_binding::Entity::find()
        .filter(media_binding::Column::MediaAssetId.eq(media_asset_id))
        .filter(media_binding::Column::TargetKind.eq(target_kind))
        .filter(media_binding::Column::TargetId.eq(target_id))
        .filter(media_binding::Column::Role.eq(role));
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let exists = query.one(db).await?.is_some();
    if exists {
        return Ok(());
    }
    media_binding::ActiveModel {
        media_asset_id: Set(media_asset_id),
        target_kind: Set(target_kind.to_string()),
        target_id: Set(target_id),
        role: Set(role.to_string()),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}
