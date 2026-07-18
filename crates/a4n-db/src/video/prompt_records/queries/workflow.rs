use a4n_core::models::chapter::chapter_video_prompt;
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder};

pub async fn completed_chapter_video_prompt_workflow_records<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    prompt_type: &str,
    workflow_mode: &str,
) -> Result<Vec<chapter_video_prompt::Model>, DbErr>
where
    C: ConnectionTrait,
{
    chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::PromptType.eq(prompt_type))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq(workflow_mode))
        .filter(chapter_video_prompt::Column::Status.eq("completed"))
        .order_by_desc(chapter_video_prompt::Column::UpdatedAt)
        .order_by_desc(chapter_video_prompt::Column::Id)
        .all(db)
        .await
}

pub async fn chapter_video_prompt_workflow_records<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    prompt_type: &str,
    workflow_mode: &str,
    exclude_running: bool,
) -> Result<Vec<chapter_video_prompt::Model>, DbErr>
where
    C: ConnectionTrait,
{
    let mut query = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::PromptType.eq(prompt_type))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq(workflow_mode));
    if exclude_running {
        query = query.filter(chapter_video_prompt::Column::Status.ne("running"));
    }
    query
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .order_by_desc(chapter_video_prompt::Column::UpdatedAt)
        .order_by_desc(chapter_video_prompt::Column::Id)
        .all(db)
        .await
}

pub async fn chapter_video_prompt_segment_plan_records<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    prompt_type: &str,
    only_plan_row: bool,
) -> Result<Vec<chapter_video_prompt::Model>, DbErr>
where
    C: ConnectionTrait,
{
    let mut query = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq(workflow_mode))
        .filter(chapter_video_prompt::Column::PromptType.eq(prompt_type));
    if only_plan_row {
        query = query.filter(chapter_video_prompt::Column::ShotNumber.eq(0));
    }
    query
        .order_by_desc(chapter_video_prompt::Column::UpdatedAt)
        .order_by_asc(chapter_video_prompt::Column::Id)
        .all(db)
        .await
}

pub async fn chapter_video_prompt_workflow_shot_records<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    prompt_type: &str,
    shot_number: i32,
) -> Result<Vec<chapter_video_prompt::Model>, DbErr>
where
    C: ConnectionTrait,
{
    chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq(workflow_mode))
        .filter(chapter_video_prompt::Column::PromptType.eq(prompt_type))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(shot_number))
        .order_by_desc(chapter_video_prompt::Column::UpdatedAt)
        .order_by_asc(chapter_video_prompt::Column::Id)
        .all(db)
        .await
}
