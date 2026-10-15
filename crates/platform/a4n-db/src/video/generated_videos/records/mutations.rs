use a4n_db_models::models::ai::generated_video;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DbBackend, EntityTrait, QuerySelect, Set, TransactionTrait,
};

use super::metadata::validate_video_metadata_update_preserves_system_fields;
use crate::video::{
    VideoDbError, normalize_optional_video_record_text, required_video_record_text,
};
use a4n_domain::media::UpdateGeneratedVideoRecordInput;

pub async fn update_generated_video_record<C>(
    db: &C,
    video_id: i64,
    input: UpdateGeneratedVideoRecordInput,
) -> Result<generated_video::Model, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let initial = generated_video::Entity::find_by_id(video_id)
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound("视频不存在".to_string()))?;
    let tx = db.begin().await?;
    crate::generation::require_project_owner_lifecycle_active(&tx, initial.project_id).await?;
    let mut query = generated_video::Entity::find_by_id(video_id);
    if tx.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let video = query
        .one(&tx)
        .await?
        .ok_or_else(|| VideoDbError::NotFound("视频不存在".to_string()))?;
    if video.project_id != initial.project_id {
        return Err(VideoDbError::InvalidArgument(
            "视频所属项目已变化".to_string(),
        ));
    }
    if input
        .expected_updated_at
        .as_deref()
        .is_some_and(|expected| expected.trim() != video.updated_at.to_string())
    {
        return Err(VideoDbError::InvalidArgument(
            "视频记录已被其他操作修改，请刷新后重试".to_string(),
        ));
    }
    if video.status == a4n_domain::video_workflow::VIDEO_STATUS_RUNNING {
        return Err(VideoDbError::InvalidArgument(
            "视频生成中，不能编辑视频记录".to_string(),
        ));
    }
    validate_video_metadata_update_preserves_system_fields(
        video.metadata_json.as_ref(),
        input.metadata_json.as_ref(),
        input.clear_metadata_json,
    )?;
    let mut active: generated_video::ActiveModel = video.into();
    if let Some(prompt) = input.prompt {
        active.prompt = Set(required_video_record_text(&prompt, "prompt")?);
    }
    if input.clear_negative_prompt {
        active.negative_prompt = Set(None);
    } else if let Some(negative_prompt) = input.negative_prompt {
        active.negative_prompt = Set(normalize_optional_video_record_text(Some(
            negative_prompt.as_str(),
        )));
    }
    if let Some(is_favorite) = input.is_favorite {
        active.is_favorite = Set(is_favorite);
    }
    if input.clear_metadata_json {
        active.metadata_json = Set(None);
    } else if input.metadata_json.is_some() {
        active.metadata_json = Set(input.metadata_json);
    }
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(&tx).await?;
    tx.commit().await?;
    Ok(updated)
}
