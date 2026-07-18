use super::locking::generated_image_for_delete;
use super::references::clear_generated_image_references_record;
use crate::video::VideoDbError;
use a4n_core::models::{ai::generated_image, asset::profile_primary_material};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, TransactionTrait};

async fn delete_generated_image_record_in<C>(db: &C, image_id: i64) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    generated_image_for_delete(db, image_id).await?;

    clear_generated_image_references_record(db, &[image_id]).await?;
    profile_primary_material::Entity::delete_many()
        .filter(profile_primary_material::Column::PrimaryImageId.eq(image_id))
        .exec(db)
        .await?;
    crate::media::delete_generated_image_target_media_bindings(db, &[image_id]).await?;
    crate::generation::delete_generation_run_targets_for(db, "generated_image", &[image_id])
        .await?;
    generated_image::Entity::delete_by_id(image_id)
        .exec(db)
        .await?;
    Ok(())
}

pub async fn delete_generated_image_record<C>(db: &C, image_id: i64) -> Result<(), VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    delete_generated_image_record_in(&txn, image_id).await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(())
}
