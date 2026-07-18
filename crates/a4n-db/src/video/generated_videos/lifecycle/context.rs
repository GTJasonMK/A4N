use a4n_core::models::{ai::generated_video, chapter::chapter_video_prompt};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};

use super::super::records::prompt_for_generated_video_record;
use crate::video::VideoDbError;

pub(super) async fn generated_video_with_prompt_record<C>(
    db: &C,
    video_id: i64,
    prompt_id: Option<i64>,
) -> Result<(generated_video::Model, Option<chapter_video_prompt::Model>), VideoDbError>
where
    C: ConnectionTrait,
{
    let video = generated_video::Entity::find_by_id(video_id)
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound("视频不存在".to_string()))?;
    let prompt = match prompt_id {
        Some(prompt_id) => Some(prompt_for_generated_video_record(db, prompt_id, &video).await?),
        None => None,
    };
    Ok((video, prompt))
}

pub(super) async fn update_prompt_status_record<C>(
    db: &C,
    prompt: chapter_video_prompt::Model,
    status: &str,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    let mut active: chapter_video_prompt::ActiveModel = prompt.into();
    active.status = Set(status.to_string());
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await?;
    Ok(())
}
