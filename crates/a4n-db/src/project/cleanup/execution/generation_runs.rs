use super::super::ProjectCleanupDbError;
use crate::project::validate_positive_i64;
use a4n_db_models::models::generation::{generation_run, generation_run_target};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};

pub(crate) async fn delete_project_generation_runs<C>(
    db: &C,
    project_id: i64,
    generated_image_ids: &[i64],
    generated_video_ids: &[i64],
) -> Result<(), ProjectCleanupDbError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectCleanupDbError::InvalidArgument)?;
    let generation_run_ids = generation_run::Entity::find()
        .select_only()
        .column(generation_run::Column::Id)
        .filter(generation_run::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .all(db)
        .await?;

    if !generation_run_ids.is_empty() {
        generation_run_target::Entity::delete_many()
            .filter(generation_run_target::Column::GenerationRunId.is_in(generation_run_ids))
            .exec(db)
            .await?;
    }
    if !generated_image_ids.is_empty() {
        generation_run_target::Entity::delete_many()
            .filter(generation_run_target::Column::TargetTable.eq("generated_image"))
            .filter(generation_run_target::Column::TargetId.is_in(generated_image_ids.to_vec()))
            .exec(db)
            .await?;
    }
    if !generated_video_ids.is_empty() {
        generation_run_target::Entity::delete_many()
            .filter(generation_run_target::Column::TargetTable.eq("generated_video"))
            .filter(generation_run_target::Column::TargetId.is_in(generated_video_ids.to_vec()))
            .exec(db)
            .await?;
    }
    generation_run::Entity::delete_many()
        .filter(generation_run::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?;

    Ok(())
}
