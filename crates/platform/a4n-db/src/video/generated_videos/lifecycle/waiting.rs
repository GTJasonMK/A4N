use a4n_db_models::models::{ai::generated_video, chapter::chapter_video_prompt};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set, TransactionTrait};

use super::context::{
    ensure_generated_video_status_mutable, generated_video_with_prompt_record,
    update_prompt_status_record,
};
use crate::video::VideoDbError;

async fn update_generated_video_waiting_status_and_prompt_record_in<C>(
    db: &C,
    video_id: i64,
    prompt_id: Option<i64>,
    status: &str,
) -> Result<generated_video::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let (video, prompt) = generated_video_with_prompt_record(db, video_id, prompt_id).await?;
    ensure_generated_video_status_mutable(&video.status, "更新视频等待状态")?;
    let mut active: generated_video::ActiveModel = video.into();
    active.status = Set(status.to_string());
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(db).await?;
    if let Some(prompt) = prompt {
        update_prompt_status_record(db, prompt, chapter_video_prompt::STATUS_RUNNING).await?;
    }
    Ok(updated)
}

pub(super) fn validate_generated_video_waiting_status(status: &str) -> Result<(), VideoDbError> {
    if matches!(
        status,
        a4n_domain::video_workflow::VIDEO_STATUS_PENDING
            | a4n_domain::video_workflow::VIDEO_STATUS_RUNNING
    ) {
        Ok(())
    } else {
        Err(VideoDbError::InvalidArgument(format!(
            "视频等待状态无效: {status}"
        )))
    }
}

pub async fn update_generated_video_waiting_status_and_prompt_record<C>(
    db: &C,
    video_id: i64,
    prompt_id: Option<i64>,
    status: &str,
) -> Result<generated_video::Model, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    validate_generated_video_waiting_status(status)?;
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let updated = update_generated_video_waiting_status_and_prompt_record_in(
        &txn, video_id, prompt_id, status,
    )
    .await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(updated)
}
