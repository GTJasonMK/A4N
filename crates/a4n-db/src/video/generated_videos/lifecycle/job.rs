use a4n_core::models::{ai::generated_video, chapter::chapter_video_prompt};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set, TransactionTrait};
use serde_json::Value;

use super::context::{
    ensure_generated_video_status_mutable, generated_video_with_prompt_record,
    update_prompt_status_record,
};
use crate::video::{VideoDbError, merge_video_provider_response_metadata};

async fn mark_generated_video_job_and_prompt_record_in<C>(
    db: &C,
    video_id: i64,
    provider_job_id: &str,
    metadata_json: Option<Value>,
    prompt_id: Option<i64>,
) -> Result<generated_video::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let provider_job_id = provider_job_id.trim();
    if provider_job_id.is_empty() {
        return Err(VideoDbError::InvalidArgument(
            "provider_job_id 不能为空".to_string(),
        ));
    }
    let (video, prompt) = generated_video_with_prompt_record(db, video_id, prompt_id).await?;
    ensure_generated_video_status_mutable(&video.status, "绑定视频 provider job")?;
    let existing_metadata = video.metadata_json.clone();
    let mut active: generated_video::ActiveModel = video.into();
    active.status = Set(generated_video::STATUS_RUNNING.to_string());
    active.provider_job_id = Set(Some(provider_job_id.to_string()));
    if metadata_json.is_some() {
        active.metadata_json = Set(merge_video_provider_response_metadata(
            existing_metadata,
            metadata_json,
        )?);
    }
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(db).await?;
    if let Some(prompt) = prompt {
        update_prompt_status_record(db, prompt, chapter_video_prompt::STATUS_RUNNING).await?;
    }
    Ok(updated)
}

pub async fn mark_generated_video_job_and_prompt_record<C>(
    db: &C,
    video_id: i64,
    provider_job_id: &str,
    metadata_json: Option<Value>,
    prompt_id: Option<i64>,
) -> Result<generated_video::Model, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let updated = mark_generated_video_job_and_prompt_record_in(
        &txn,
        video_id,
        provider_job_id,
        metadata_json,
        prompt_id,
    )
    .await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(updated)
}
