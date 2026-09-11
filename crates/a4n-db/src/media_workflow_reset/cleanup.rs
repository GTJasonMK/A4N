use a4n_db_models::models::chapter::{chapter_manga_prompt, chapter_video_prompt};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter};

use super::scope::MediaWorkflowV2ResetScope;
use crate::galgame::delete_galgame_script_records_for_project;

pub(super) async fn delete_media_workflow_v2_records<C>(
    db: &C,
    project_id: i64,
    scope: &MediaWorkflowV2ResetScope,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    crate::project::delete_project_generation_runs(
        db,
        project_id,
        &scope.image_ids,
        &scope.video_ids,
    )
    .await
    .map_err(|error| DbErr::Custom(error.to_string()))?;
    let _ = crate::project::delete_project_task_queue_records(db, project_id)
        .await
        .map_err(|error| DbErr::Custom(error.to_string()))?;
    crate::project::delete_project_workflow_runs(db, project_id)
        .await
        .map_err(|error| DbErr::Custom(error.to_string()))?;

    chapter_manga_prompt::Entity::delete_many()
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?;
    chapter_video_prompt::Entity::delete_many()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?;
    let _ = delete_galgame_script_records_for_project(db, project_id).await?;

    let _ = crate::project::delete_project_generated_artifact_links(
        db,
        project_id,
        &scope.media_asset_ids,
        &scope.generation_spec_ids,
        &scope.prompt_artifact_ids,
    )
    .await
    .map_err(|error| DbErr::Custom(error.to_string()))?;
    let _ = crate::project::delete_project_generated_artifact_rows(
        db,
        project_id,
        &scope.image_ids,
        &scope.video_ids,
        &scope.media_asset_ids,
        &scope.generation_spec_ids,
    )
    .await
    .map_err(|error| DbErr::Custom(error.to_string()))?;
    Ok(())
}
