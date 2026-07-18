use a4n_core::models::{
    ai::generated_video,
    chapter::{chapter, chapter_video_prompt},
    project::novel_project,
};
use sea_orm::{ConnectionTrait, EntityTrait};

use crate::video::VideoDbError;

async fn video_prompt_with_valid_chapter_record<C>(
    db: &C,
    prompt_id: i64,
) -> Result<chapter_video_prompt::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let prompt = chapter_video_prompt::Entity::find_by_id(prompt_id)
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound("视频提示词不存在".to_string()))?;
    let project = novel_project::Entity::find_by_id(prompt.project_id)
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound(format!("项目不存在: {}", prompt.project_id)))?;
    let chapter = chapter::Entity::find_by_id(prompt.chapter_id)
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound(format!("章节不存在: {}", prompt.chapter_id)))?;
    match project.project_kind.as_str() {
        "text" => {
            if chapter.project_id != project.id {
                return Err(VideoDbError::InvalidArgument(format!(
                    "章节 #{} 不属于文本项目 {}",
                    prompt.chapter_id, project.id
                )));
            }
        }
        "video" => {
            if project.root_project_id != Some(chapter.project_id) {
                return Err(VideoDbError::InvalidArgument(format!(
                    "章节 #{} 不属于视频项目 {} 绑定的根文本项目",
                    prompt.chapter_id, project.id
                )));
            }
        }
        other => {
            return Err(VideoDbError::InvalidArgument(format!(
                "视频链路不支持项目类型 {other}: project_id={}",
                project.id
            )));
        }
    }
    Ok(prompt)
}

pub(in crate::video::generated_videos) async fn prompt_for_generated_video_record<C>(
    db: &C,
    prompt_id: i64,
    video: &generated_video::Model,
) -> Result<chapter_video_prompt::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let prompt = video_prompt_with_valid_chapter_record(db, prompt_id).await?;
    if video.project_id != prompt.project_id {
        return Err(VideoDbError::InvalidArgument(format!(
            "视频 #{} 不属于提示词项目 {}",
            video.id, prompt.project_id
        )));
    }
    if video.chapter_id != Some(prompt.chapter_id) {
        return Err(VideoDbError::InvalidArgument(format!(
            "视频 #{} 不属于提示词章节 {}",
            video.id, prompt.chapter_id
        )));
    }
    Ok(prompt)
}
