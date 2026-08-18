use a4n_core::models::chapter::chapter_video_prompt;
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder};

pub async fn chapter_video_prompt_sibling_records<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    prompt_id: i64,
) -> Result<Vec<chapter_video_prompt::Model>, DbErr>
where
    C: ConnectionTrait,
{
    chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::Id.ne(prompt_id))
        .order_by_asc(chapter_video_prompt::Column::Id)
        .all(db)
        .await
}

pub async fn chapter_video_prompt_previous_frame_candidates<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    prompt_type: &str,
    frame_number: i32,
) -> Result<Vec<chapter_video_prompt::Model>, DbErr>
where
    C: ConnectionTrait,
{
    chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq(workflow_mode))
        .filter(chapter_video_prompt::Column::PromptType.eq(prompt_type))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(frame_number))
        .filter(chapter_video_prompt::Column::Status.ne(chapter_video_prompt::STATUS_RUNNING))
        .order_by_desc(chapter_video_prompt::Column::UpdatedAt)
        .order_by_desc(chapter_video_prompt::Column::Id)
        .all(db)
        .await
}
