use a4n_core::models::chapter::chapter_video_prompt;
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder};

pub async fn chapter_video_prompt_records_for_workflow_types<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    prompt_types: &[&str],
) -> Result<Vec<chapter_video_prompt::Model>, DbErr>
where
    C: ConnectionTrait,
{
    if prompt_types.is_empty() {
        return Ok(Vec::new());
    }
    chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq(workflow_mode))
        .filter(chapter_video_prompt::Column::PromptType.is_in(prompt_types.iter().copied()))
        .all(db)
        .await
}

pub async fn chapter_video_prompt_records_for_workflow_types_by_prompt_and_shot<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    prompt_types: &[&str],
) -> Result<Vec<chapter_video_prompt::Model>, DbErr>
where
    C: ConnectionTrait,
{
    if prompt_types.is_empty() {
        return Ok(Vec::new());
    }
    chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq(workflow_mode))
        .filter(chapter_video_prompt::Column::PromptType.is_in(prompt_types.iter().copied()))
        .order_by_asc(chapter_video_prompt::Column::PromptType)
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .all(db)
        .await
}

pub async fn chapter_video_prompt_records_for_types<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    prompt_types: &[&str],
) -> Result<Vec<chapter_video_prompt::Model>, DbErr>
where
    C: ConnectionTrait,
{
    if prompt_types.is_empty() {
        return Ok(Vec::new());
    }
    chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::PromptType.is_in(prompt_types.iter().copied()))
        .all(db)
        .await
}
