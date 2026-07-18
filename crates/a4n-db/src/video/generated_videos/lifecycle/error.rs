use a4n_core::models::ai::generated_video;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set, TransactionTrait};

use super::context::{generated_video_with_prompt_record, update_prompt_status_record};
use crate::video::VideoDbError;

async fn mark_generated_video_error_status_and_prompt_record_in<C>(
    db: &C,
    video_id: i64,
    prompt_id: Option<i64>,
    status: &str,
    message: &str,
) -> Result<generated_video::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let (video, prompt) = generated_video_with_prompt_record(db, video_id, prompt_id).await?;
    let mut active: generated_video::ActiveModel = video.into();
    active.status = Set(status.to_string());
    active.error_message = Set(Some(message.to_string()));
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(db).await?;
    if let Some(prompt) = prompt {
        update_prompt_status_record(db, prompt, status).await?;
    }
    Ok(updated)
}

pub(super) fn normalize_generated_video_error_message(message: &str) -> Result<&str, VideoDbError> {
    let message = message.trim();
    if message.is_empty() {
        Err(VideoDbError::InvalidArgument(
            "视频失败原因不能为空".to_string(),
        ))
    } else {
        Ok(message)
    }
}

pub async fn mark_generated_video_error_status_and_prompt_record<C>(
    db: &C,
    video_id: i64,
    prompt_id: Option<i64>,
    status: &str,
    message: &str,
) -> Result<generated_video::Model, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    if !matches!(status, "failed" | "interrupted") {
        return Err(VideoDbError::InvalidArgument(format!(
            "视频终止状态无效: {status}"
        )));
    }
    let message = normalize_generated_video_error_message(message)?;
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let updated = mark_generated_video_error_status_and_prompt_record_in(
        &txn, video_id, prompt_id, status, message,
    )
    .await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(updated)
}
