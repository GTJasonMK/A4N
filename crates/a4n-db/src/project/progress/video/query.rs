use super::CHAPTER_VIDEO_TYPE;
use super::facts::video_progress_facts_from_rows;
use super::status::video_project_progress_from_counts;
use crate::project::progress::{ProjectProgressReadError, ProjectProgressSummary};
use a4n_core::models::{ai::generated_video, chapter::chapter_video_prompt};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::collections::HashMap;

pub(in crate::project::progress) async fn video_project_progress_by_project(
    db: &DatabaseConnection,
    project_ids: &[i64],
) -> Result<HashMap<i64, ProjectProgressSummary>, ProjectProgressReadError> {
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let prompts = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.is_in(project_ids.iter().copied()))
        .all(db)
        .await
        .map_err(|error| ProjectProgressReadError::database("查询视频提示词失败", error))?;
    let videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(generated_video::Column::VideoType.eq(CHAPTER_VIDEO_TYPE))
        .all(db)
        .await
        .map_err(|error| ProjectProgressReadError::database("查询章节生成视频失败", error))?;

    let mut prompts_by_project: HashMap<i64, Vec<chapter_video_prompt::Model>> = HashMap::new();
    for prompt in prompts {
        prompts_by_project
            .entry(prompt.project_id)
            .or_default()
            .push(prompt);
    }
    let mut videos_by_project: HashMap<i64, Vec<generated_video::Model>> = HashMap::new();
    for video in videos {
        videos_by_project
            .entry(video.project_id)
            .or_default()
            .push(video);
    }

    let mut progress = HashMap::with_capacity(project_ids.len());
    for project_id in project_ids {
        let prompts = prompts_by_project.remove(project_id).unwrap_or_default();
        let videos = videos_by_project.remove(project_id).unwrap_or_default();
        progress.insert(
            *project_id,
            video_project_progress_from_counts(video_progress_facts_from_rows(&prompts, &videos)?),
        );
    }
    Ok(progress)
}
