use super::rules::validate_chapter_video_prompt_duration;
use crate::video::VideoDbError;
use a4n_db_models::models::chapter::chapter_video_prompt;
use a4n_domain::video_prompt::UpdateChapterVideoPromptRecordInput;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use serde_json::Value;

pub async fn update_chapter_video_prompt_record<C>(
    db: &C,
    prompt: chapter_video_prompt::Model,
    input: UpdateChapterVideoPromptRecordInput,
) -> Result<chapter_video_prompt::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let mut active: chapter_video_prompt::ActiveModel = prompt.into();
    if let Some(value) = input.visual_prompt {
        active.visual_prompt = Set(value);
    }
    if let Some(value) = input.motion_prompt {
        active.motion_prompt = Set(value);
    }
    if let Some(value) = input.negative_prompt {
        active.negative_prompt = Set(value);
    }
    if let Some(value) = input.duration_secs {
        validate_chapter_video_prompt_duration(value)?;
        active.duration_secs = Set(value);
    }
    if let Some(value) = input.reference_image_ids_json {
        active.reference_image_ids_json = Set(value);
    }
    if let Some(value) = input.metadata_json {
        active.metadata_json = Set(Some(value));
    }
    if input.reset_video_binding_to_pending {
        active.generated_video_id = Set(None);
        active.status = Set(chapter_video_prompt::STATUS_PENDING.to_string());
    }
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(VideoDbError::Database)
}

pub(crate) async fn update_chapter_video_prompt_metadata_record<C>(
    db: &C,
    prompt: chapter_video_prompt::Model,
    metadata_json: Value,
) -> Result<chapter_video_prompt::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let mut active: chapter_video_prompt::ActiveModel = prompt.into();
    active.metadata_json = Set(Some(metadata_json));
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(VideoDbError::Database)
}
