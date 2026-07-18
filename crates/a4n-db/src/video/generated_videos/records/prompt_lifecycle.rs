use a4n_core::models::{ai::generated_video, chapter::chapter_video_prompt};
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
            "已存在由该视频提示词生成的视频，不能重新生成视频；请先删除对应视频".to_string(),
        ));
    }
    let mut active: chapter_video_prompt::ActiveModel = prompt.into();
    active.generated_video_id = Set(None);
    active.status = Set("running".to_string());
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(VideoDbError::Database)
}
