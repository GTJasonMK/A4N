use crate::project::dependency_graph::AssetLineageRecord;
use crate::project::dependency_graph::details::records::video_prompt_record;
use a4n_db_models::models::chapter::chapter_video_prompt;
use a4n_error::AppError;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

pub(in crate::project::dependency_graph) async fn video_prompt_detail_records<C>(
    db: &C,
    project_ids: &[i64],
    prompt_types: &[&str],
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let prompts = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(chapter_video_prompt::Column::PromptType.is_in(prompt_types.iter().copied()))
        .order_by_asc(chapter_video_prompt::Column::ChapterId)
        .order_by_asc(chapter_video_prompt::Column::WorkflowMode)
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .order_by_asc(chapter_video_prompt::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(prompts.into_iter().map(video_prompt_record).collect())
}
