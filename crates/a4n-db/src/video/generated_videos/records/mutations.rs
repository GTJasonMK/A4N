use a4n_core::models::ai::generated_video;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, QuerySelect, Set};

use super::metadata::validate_video_metadata_update_preserves_system_fields;
use crate::video::{
    UpdateGeneratedVideoRecordInput, VideoDbError, normalize_optional_video_record_text,
    required_video_record_text,
};

pub async fn update_generated_video_record<C>(
    db: &C,
    video_id: i64,
    input: UpdateGeneratedVideoRecordInput,
) -> Result<generated_video::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let video = generated_video::Entity::find_by_id(video_id)
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound("视频不存在".to_string()))?;
    if video.status == generated_video::STATUS_RUNNING {
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
    active.update(db).await.map_err(VideoDbError::Database)
}

pub async fn update_generated_video_duration_record<C>(
    db: &C,
    video_id: i64,
    duration_secs: f32,
) -> Result<generated_video::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    if !duration_secs.is_finite() || duration_secs <= 0.0 {
        return Err(VideoDbError::InvalidArgument(
            "视频实际时长必须是大于 0 的有限数".to_string(),
        ));
    }
    let status = generated_video::Entity::find_by_id(video_id)
        .select_only()
        .column(generated_video::Column::Status)
        .into_tuple::<String>()
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound("视频不存在".to_string()))?;
    if status == generated_video::STATUS_RUNNING {
        return Err(VideoDbError::InvalidArgument(
            "视频生成中，不能回写视频实际时长".to_string(),
        ));
    }
    let mut active = generated_video::ActiveModel {
        id: Set(video_id),
        ..Default::default()
    };
    active.duration_secs = Set(duration_secs);
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(VideoDbError::Database)
}
