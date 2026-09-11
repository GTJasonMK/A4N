use a4n_db_models::models::chapter::chapter_video_prompt;
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder};

pub(crate) async fn chapter_video_prompt_sync_candidates<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    prompt_types: &[String],
    workflow_modes: &[String],
) -> Result<Vec<chapter_video_prompt::Model>, DbErr>
where
    C: ConnectionTrait,
{
    if prompt_types.is_empty() || workflow_modes.is_empty() {
        return Ok(Vec::new());
    }
    chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::PromptType.is_in(prompt_types.iter().cloned()))
        .filter(chapter_video_prompt::Column::WorkflowMode.is_in(workflow_modes.iter().cloned()))
        .order_by_asc(chapter_video_prompt::Column::WorkflowMode)
        .order_by_asc(chapter_video_prompt::Column::PromptType)
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .order_by_asc(chapter_video_prompt::Column::Id)
        .all(db)
        .await
}
