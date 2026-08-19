use super::locking::generated_video_for_delete;
use super::references::clear_generated_video_references_record;
use crate::video::VideoDbError;
use a4n_core::models::ai::generated_video;
use sea_orm::{ConnectionTrait, EntityTrait, TransactionTrait};

async fn delete_locked_generated_video_record_in<C>(
    db: &C,
    video: &generated_video::Model,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    clear_generated_video_references_record(db, std::slice::from_ref(video)).await?;
    crate::media::delete_generated_video_target_media_bindings(db, &[video.id]).await?;
    crate::generation::delete_generation_run_targets_for(db, "generated_video", &[video.id])
        .await?;
    generated_video::Entity::delete_by_id(video.id)
        .exec(db)
        .await?;
    Ok(())
}

pub async fn delete_generated_video_record_with_cleanup<C>(
    db: &C,
    video_id: i64,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let video = generated_video_for_delete(&txn, video_id).await?;
    let paths = [video.file_path.as_deref(), video.thumbnail_path.as_deref()]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(ToString::to_string)
        .collect();
    let payload =
        crate::cleanup_outbox::validated_file_artifact_cleanup_payload(video.project_id, paths)?;
    if !payload.paths.is_empty() {
        crate::cleanup_outbox::insert_file_artifact_cleanup_intent(
            &txn,
            format!("generated-video-delete:{}", video.id),
            payload,
        )
        .await?;
    }
    delete_locked_generated_video_record_in(&txn, &video).await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(())
}
