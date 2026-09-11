use super::locking::{GeneratedImageDeleteContext, generated_image_for_delete};
use super::references::clear_generated_image_references_record;
use crate::video::VideoDbError;
use a4n_db_models::models::{ai::generated_image, asset::profile_primary_material};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, TransactionTrait};

async fn delete_locked_generated_image_record_in<C>(
    db: &C,
    image: &GeneratedImageDeleteContext,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    clear_generated_image_references_record(db, &[image.id]).await?;
    profile_primary_material::Entity::delete_many()
        .filter(profile_primary_material::Column::PrimaryImageId.eq(image.id))
        .exec(db)
        .await?;
    crate::media::delete_generated_image_target_media_bindings(db, &[image.id]).await?;
    crate::generation::delete_generation_run_targets_for(db, "generated_image", &[image.id])
        .await?;
    generated_image::Entity::delete_by_id(image.id)
        .exec(db)
        .await?;
    Ok(())
}

pub async fn delete_generated_image_record_with_cleanup<C>(
    db: &C,
    image_id: i64,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let image = generated_image_for_delete(&txn, image_id).await?;
    let paths = [image.file_path.trim()]
        .into_iter()
        .filter(|path| !path.is_empty())
        .map(ToString::to_string)
        .collect();
    let payload =
        crate::cleanup_outbox::validated_file_artifact_cleanup_payload(image.project_id, paths)?;
    if !payload.paths.is_empty() {
        crate::cleanup_outbox::insert_file_artifact_cleanup_intent(
            &txn,
            format!("generated-image-delete:{}", image.id),
            payload,
        )
        .await?;
    }
    delete_locked_generated_image_record_in(&txn, &image).await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(())
}
