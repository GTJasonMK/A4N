use crate::galgame::{GalgameMediaRecordError, validate_galgame_media_asset_id};
use a4n_db_models::models::media::{media_asset, media_provenance};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, TransactionTrait};

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
