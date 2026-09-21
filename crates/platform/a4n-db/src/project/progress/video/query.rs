use super::facts::video_progress_facts_from_rows;
use super::identity::{GeneratedVideoProgressRow, VideoPromptProgressRow};
use super::status::video_project_progress_from_counts;
use crate::project::progress::{ProjectProgressReadError, ProjectProgressSummary};
use a4n_db_models::models::{ai::generated_video, chapter::chapter_video_prompt};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect};
use std::collections::HashMap;

pub(in crate::project::progress) async fn video_project_progress_by_project(
    db: &DatabaseConnection,
    project_ids: &[i64],
) -> Result<HashMap<i64, ProjectProgressSummary>, ProjectProgressReadError> {
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let (prompts, videos) = futures::join!(
        async {
            chapter_video_prompt::Entity::find()
                .select_only()
                .column(chapter_video_prompt::Column::Id)
                .column(chapter_video_prompt::Column::ProjectId)
                .column(chapter_video_prompt::Column::ChapterId)
                .column(chapter_video_prompt::Column::WorkflowMode)
                .column(chapter_video_prompt::Column::PromptType)
                .column(chapter_video_prompt::Column::ShotNumber)
                .column(chapter_video_prompt::Column::Status)
                .column(chapter_video_prompt::Column::UpdatedAt)
                .filter(chapter_video_prompt::Column::ProjectId.is_in(project_ids.iter().copied()))
                .into_model::<VideoPromptProgressRow>()
                .all(db)
                .await
                .map_err(|error| ProjectProgressReadError::database("查询视频提示词失败", error))
        },
        async {
            generated_video::Entity::find()
                .select_only()
                .column(generated_video::Column::Id)
                .column(generated_video::Column::ProjectId)
                .column(generated_video::Column::ChapterId)
                .column(generated_video::Column::WorkflowMode)
                .column(generated_video::Column::Status)
                .column(generated_video::Column::MetadataJson)
                .column(generated_video::Column::UpdatedAt)
                .filter(generated_video::Column::ProjectId.is_in(project_ids.iter().copied()))
                .filter(
                    generated_video::Column::VideoType
                        .eq(a4n_domain::video_workflow::VIDEO_TYPE_CHAPTER),
                )
                .into_model::<GeneratedVideoProgressRow>()
                .all(db)
                .await
                .map_err(|error| ProjectProgressReadError::database("查询章节生成视频失败", error))
        },
    );
    let prompts = prompts?;
    let videos = videos?;

    let mut prompts_by_project: HashMap<i64, Vec<VideoPromptProgressRow>> = HashMap::new();
    for prompt in prompts {
        prompts_by_project
            .entry(prompt.project_id)
            .or_default()
            .push(prompt);
    }
    let mut videos_by_project: HashMap<i64, Vec<GeneratedVideoProgressRow>> = HashMap::new();
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
