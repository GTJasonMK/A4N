use a4n_db_models::models::{ai::generated_video, chapter::chapter_video_prompt};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set, TransactionTrait};
use serde_json::Value;

use super::context::{
    ensure_generated_video_status_allows_completion, generated_video_with_prompt_record,
};
use crate::video::{VideoDbError, merge_video_provider_response_metadata};

pub(super) fn completed_video_write_is_idempotent(
    status: &str,
    existing_file_path: Option<&str>,
    requested_file_path: &str,
) -> Result<bool, VideoDbError> {
    if status != a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED {
        return Ok(false);
    }
    if existing_file_path == Some(requested_file_path) {
        return Ok(true);
    }
    Err(VideoDbError::InvalidState {
        current: status.to_string(),
        operation: "用不同文件重复完成视频生成".to_string(),
    })
}

async fn mark_generated_video_completed_and_link_prompt_record_in<C>(
    db: &C,
    video_id: i64,
    file_path: &str,
    thumbnail_path: Option<&str>,
    file_size: i64,
    metadata_json: Option<Value>,
    prompt_id: Option<i64>,
) -> Result<generated_video::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let (video, prompt) = generated_video_with_prompt_record(db, video_id, prompt_id).await?;
    if completed_video_write_is_idempotent(&video.status, video.file_path.as_deref(), file_path)? {
        if let Some(prompt) = prompt.as_ref()
            && (prompt.status != chapter_video_prompt::STATUS_COMPLETED
                || prompt.generated_video_id != Some(video.id))
        {
            return Err(VideoDbError::InvalidArgument(format!(
                "视频 #{} 已完成，但提示词 #{} 未关联该视频",
                video.id, prompt.id
            )));
        }
        return Ok(video);
    }
    ensure_generated_video_status_allows_completion(&video.status)?;
    let existing_metadata = video.metadata_json.clone();
    let mut active: generated_video::ActiveModel = video.into();
    active.file_path = Set(Some(file_path.to_string()));
    if let Some(path) = thumbnail_path {
        active.thumbnail_path = Set(Some(path.to_string()));
    }
    active.file_size = Set(Some(file_size));
    active.status = Set(a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED.to_string());
    active.error_message = Set(None);
    if metadata_json.is_some() {
        active.metadata_json = Set(merge_video_provider_response_metadata(
            existing_metadata,
            metadata_json,
        )?);
    }
    if let Some(prompt) = prompt.as_ref() {
        active.timeline_item_id = Set(prompt.timeline_item_id);
    }
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(db).await?;
    if let Some(prompt) = prompt {
        let mut active: chapter_video_prompt::ActiveModel = prompt.into();
        active.generated_video_id = Set(Some(updated.id));
        active.status = Set(chapter_video_prompt::STATUS_COMPLETED.to_string());
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(db).await?;
    }
    Ok(updated)
}

pub async fn mark_generated_video_completed_and_link_prompt_record<C>(
    db: &C,
    video_id: i64,
    file_path: &str,
    thumbnail_path: Option<&str>,
    file_size: i64,
    metadata_json: Option<Value>,
    prompt_id: Option<i64>,
) -> Result<generated_video::Model, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let updated = mark_generated_video_completed_and_link_prompt_record_in(
        &txn,
        video_id,
        file_path,
        thumbnail_path,
        file_size,
        metadata_json,
        prompt_id,
    )
    .await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(updated)
}
