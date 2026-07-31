use a4n_core::models::ai::generated_video;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    Set, TransactionTrait,
};

use super::context::{
    ensure_generated_video_status_mutable, generated_video_with_prompt_record,
    update_prompt_status_record,
};
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
    if video.status == status {
        return Ok(video);
    }
    ensure_generated_video_status_mutable(&video.status, "终止视频生成")?;
    let mut active: generated_video::ActiveModel = video.into();
    active.status = Set(status.to_string());
    active.error_message = Set(Some(message.to_string()));
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(db).await?;
    if let Some(prompt) = prompt {
        let prompt_status = if prompt.generated_video_id.is_some() {
            "completed"
        } else {
            status
        };
        update_prompt_status_record(db, prompt, prompt_status).await?;
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
    if !matches!(status, "failed" | "interrupted" | "cancelled") {
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

pub async fn recover_stale_generated_videos_without_provider_job(
    db: &DatabaseConnection,
) -> Result<u64, VideoDbError> {
    let stale = generated_video::Entity::find()
        .filter(generated_video::Column::Status.is_in(["pending", "running"]))
        .all(db)
        .await?
        .into_iter()
        .filter(|video| {
            video
                .provider_job_id
                .as_deref()
                .map(str::trim)
                .is_none_or(str::is_empty)
        })
        .collect::<Vec<_>>();
    let recovered = u64::try_from(stale.len()).map_err(|_| {
        VideoDbError::InvalidArgument("待恢复视频记录数量超出 u64 范围".to_string())
    })?;
    for video in stale {
        let message = "应用重启时视频尚未提交 provider，已中断；请重新生成";
        crate::generation::interrupt_runs_for_target(
            db,
            "generated_video",
            video.id,
            message.to_string(),
        )
        .await?;
        let prompt_id = video
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("prompt_id"))
            .and_then(serde_json::Value::as_i64);
        mark_generated_video_error_status_and_prompt_record(
            db,
            video.id,
            prompt_id,
            "interrupted",
            message,
        )
        .await?;
    }
    Ok(recovered)
}
