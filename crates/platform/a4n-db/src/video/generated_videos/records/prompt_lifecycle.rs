use a4n_db_models::models::{ai::generated_video, chapter::chapter_video_prompt};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use super::prompt_context::prompt_for_generated_video_record;
use crate::video::VideoDbError;

pub(in crate::video::generated_videos) async fn mark_prompt_video_generation_running_for_video_record<
    C,
>(
    db: &C,
    prompt_id: i64,
    video: &generated_video::Model,
) -> Result<chapter_video_prompt::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let prompt = prompt_for_generated_video_record(db, prompt_id, video).await?;
    if prompt.generated_video_id.is_some() {
        return Err(VideoDbError::InvalidArgument(
            "已存在由该视频提示词生成的视频，不能再次生成；请先删除对应视频".to_string(),
        ));
    }
    let mut video_active: generated_video::ActiveModel = video.clone().into();
    video_active.timeline_item_id = Set(prompt.timeline_item_id);
    video_active.update(db).await?;
    let mut active: chapter_video_prompt::ActiveModel = prompt.into();
    active.status = Set(chapter_video_prompt::STATUS_RUNNING.to_string());
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(VideoDbError::Database)
}
