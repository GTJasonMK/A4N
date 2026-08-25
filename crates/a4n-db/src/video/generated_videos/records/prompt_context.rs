use a4n_core::models::{
    ai::generated_video,
    chapter::{chapter, chapter_video_prompt},
    project::novel_project,
};
use sea_orm::{ConnectionTrait, EntityTrait, QuerySelect};

use crate::video::VideoDbError;

async fn video_prompt_scope_records<C>(
    db: &C,
    prompt_id: i64,
) -> Result<(chapter_video_prompt::Model, (i64, String, Option<i64>), i64), VideoDbError>
where
    C: ConnectionTrait,
{
    let prompt = chapter_video_prompt::Entity::find_by_id(prompt_id)
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound("视频提示词不存在".to_string()))?;
    let project = novel_project::Entity::find_by_id(prompt.project_id)
        .select_only()
        .column(novel_project::Column::Id)
        .column(novel_project::Column::ProjectKind)
        .column(novel_project::Column::RootProjectId)
        .into_tuple::<(i64, String, Option<i64>)>()
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound(format!("项目不存在: {}", prompt.project_id)))?;
    let chapter = chapter::Entity::find_by_id(prompt.chapter_id)
        .select_only()
        .column(chapter::Column::ProjectId)
        .into_tuple::<i64>()
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound(format!("章节不存在: {}", prompt.chapter_id)))?;
    validate_prompt_project_chapter_identity(&prompt, project.0, &project.1, project.2, chapter)?;
    Ok((prompt, project, chapter))
}

fn validate_generated_video_prompt_link(
    video: &generated_video::Model,
    prompt: &chapter_video_prompt::Model,
) -> Result<(), VideoDbError> {
    validate_generated_video_prompt_link_fields(
        video.id,
        video.project_id,
        video.chapter_id,
        prompt.project_id,
        prompt.chapter_id,
    )
}

pub(in crate::video::generated_videos) fn validate_generated_video_prompt_link_fields(
    video_id: i64,
    video_project_id: i64,
    video_chapter_id: Option<i64>,
    prompt_project_id: i64,
    prompt_chapter_id: i64,
) -> Result<(), VideoDbError> {
    if video_project_id != prompt_project_id {
        return Err(VideoDbError::InvalidArgument(format!(
            "视频 #{} 不属于提示词项目 {}",
            video_id, prompt_project_id
        )));
    }
    if video_chapter_id != Some(prompt_chapter_id) {
        return Err(VideoDbError::InvalidArgument(format!(
            "视频 #{} 不属于提示词章节 {}",
            video_id, prompt_chapter_id
        )));
    }
    Ok(())
}

fn validate_prompt_project_chapter_identity(
    prompt: &chapter_video_prompt::Model,
    project_id: i64,
    project_kind: &str,
    root_project_id: Option<i64>,
    chapter_project_id: i64,
) -> Result<(), VideoDbError> {
    validate_prompt_project_chapter_identity_fields(
        prompt.chapter_id,
        project_id,
        project_kind,
        root_project_id,
        chapter_project_id,
    )
}

pub(in crate::video::generated_videos) fn validate_prompt_project_chapter_identity_fields(
    chapter_id: i64,
    project_id: i64,
    project_kind: &str,
    root_project_id: Option<i64>,
    chapter_project_id: i64,
) -> Result<(), VideoDbError> {
    match project_kind {
        "text" if chapter_project_id == project_id => Ok(()),
        "text" => Err(VideoDbError::InvalidArgument(format!(
            "章节 #{} 不属于文本项目 {}",
            chapter_id, project_id
        ))),
        "video" if root_project_id == Some(chapter_project_id) => Ok(()),
        "video" => Err(VideoDbError::InvalidArgument(format!(
            "章节 #{} 不属于视频项目 {} 绑定的根文本项目",
            chapter_id, project_id
        ))),
        other => Err(VideoDbError::InvalidArgument(format!(
            "视频链路不支持项目类型 {other}: project_id={}",
            project_id
        ))),
    }
}

pub(in crate::video::generated_videos) async fn prompt_for_generated_video_record<C>(
    db: &C,
    prompt_id: i64,
    video: &generated_video::Model,
) -> Result<chapter_video_prompt::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let (prompt, _, _) = video_prompt_scope_records(db, prompt_id).await?;
    validate_generated_video_prompt_link(video, &prompt)?;
    Ok(prompt)
}
