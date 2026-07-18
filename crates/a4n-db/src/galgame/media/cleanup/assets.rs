use crate::galgame::{GalgameMediaRecordError, validate_galgame_media_asset_id};
use a4n_core::models::{
    generation::generation_spec_media_ref,
    media::{media_asset, media_binding, media_provenance},
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QuerySelect,
    TransactionTrait,
};
use std::collections::BTreeSet;

pub async fn delete_media_asset_with_provenance_records(
    db: &DatabaseConnection,
    media_asset_id: i64,
) -> Result<(), GalgameMediaRecordError> {
    validate_galgame_media_asset_id(Some(media_asset_id))?;
    let tx = db
        .begin()
        .await
        .map_err(GalgameMediaRecordError::Database)?;
    media_provenance::Entity::delete_many()
        .filter(media_provenance::Column::MediaAssetId.eq(media_asset_id))
        .exec(&tx)
        .await
        .map_err(GalgameMediaRecordError::Database)?;
    media_asset::Entity::delete_by_id(media_asset_id)
        .exec(&tx)
        .await
        .map_err(GalgameMediaRecordError::Database)?;
    tx.commit()
        .await
        .map_err(GalgameMediaRecordError::Database)?;
    Ok(())
}

pub async fn delete_unbound_project_media_assets_record<C>(
    db: &C,
    project_id: i64,
    candidate_media_asset_ids: BTreeSet<i64>,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    if candidate_media_asset_ids.is_empty() {
        return Ok(());
    }
    let candidates = candidate_media_asset_ids
        .iter()
        .copied()
        .collect::<Vec<_>>();
    let still_bound = media_binding::Entity::find()
        .select_only()
        .column(media_binding::Column::MediaAssetId)
        .filter(media_binding::Column::MediaAssetId.is_in(candidates))
        .into_tuple::<i64>()
        .all(db)
        .await?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let delete_media_asset_ids = candidate_media_asset_ids
        .difference(&still_bound)
        .copied()
        .collect::<Vec<_>>();
    if delete_media_asset_ids.is_empty() {
        return Ok(());
    }
    let delete_media_asset_ids = media_asset::Entity::find()
        .select_only()
        .column(media_asset::Column::Id)
        .filter(media_asset::Column::ProjectId.eq(project_id))
        .filter(media_asset::Column::Id.is_in(delete_media_asset_ids))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    if delete_media_asset_ids.is_empty() {
        return Ok(());
    }
    media_provenance::Entity::delete_many()
        .filter(media_provenance::Column::MediaAssetId.is_in(delete_media_asset_ids.clone()))
        .exec(db)
        .await?;
    generation_spec_media_ref::Entity::delete_many()
        .filter(
            generation_spec_media_ref::Column::MediaAssetId.is_in(delete_media_asset_ids.clone()),
        )
        .exec(db)
        .await?;
    media_asset::Entity::delete_many()
        .filter(media_asset::Column::Id.is_in(delete_media_asset_ids))
        .exec(db)
        .await?;
    Ok(())
}
