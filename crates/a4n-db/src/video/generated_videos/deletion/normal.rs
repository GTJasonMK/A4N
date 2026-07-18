use super::locking::generated_video_for_delete;
use super::references::clear_generated_video_references_record;
use crate::video::VideoDbError;
use a4n_core::models::ai::generated_video;
use sea_orm::{ConnectionTrait, EntityTrait, TransactionTrait};

async fn delete_generated_video_record_in<C>(db: &C, video_id: i64) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    let video = generated_video_for_delete(db, video_id).await?;

    clear_generated_video_references_record(db, std::slice::from_ref(&video)).await?;
    crate::media::delete_generated_video_target_media_bindings(db, &[video_id]).await?;
    crate::generation::delete_generation_run_targets_for(db, "generated_video", &[video_id])
        .await?;
    generated_video::Entity::delete_by_id(video_id)
        .exec(db)
        .await?;
    Ok(())
}

pub async fn delete_generated_video_record<C>(db: &C, video_id: i64) -> Result<(), VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    delete_generated_video_record_in(&txn, video_id).await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(())
}
