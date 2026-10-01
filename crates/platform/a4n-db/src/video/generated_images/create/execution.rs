use super::super::{
    CreateGeneratedImageRecordDraftInput, VideoDbError, lock_generated_media_project,
};
use super::record::insert_generated_image_record;
use super::validation::validate_create_generated_image_record_input;
use a4n_domain::media::GeneratedImageRecord;
use sea_orm::{ConnectionTrait, TransactionTrait};

use crate::video::media_mapping::generated_image_record_from_model;

pub async fn create_generated_image_record<C>(
    db: &C,
    input: CreateGeneratedImageRecordDraftInput,
) -> Result<GeneratedImageRecord, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    lock_generated_media_project(&txn, input.project_id).await?;
    let input = validate_create_generated_image_record_input(&txn, input).await?;
    let image = insert_generated_image_record(&txn, input)
        .await
        .map_err(VideoDbError::Database)?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(generated_image_record_from_model(image))
}
