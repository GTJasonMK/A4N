use super::super::{ProjectCleanupDbError, ProjectGenerationRunCleanupResult};
use crate::project::validate_positive_i64;
use a4n_core::models::generation::{generation_run, generation_run_target};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub(crate) async fn delete_project_generation_runs<C>(
    db: &C,
    project_id: i64,
    generated_image_ids: &[i64],
    generated_video_ids: &[i64],
) -> Result<ProjectGenerationRunCleanupResult, ProjectCleanupDbError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectCleanupDbError::InvalidArgument)?;
    let generation_run_ids = generation_run::Entity::find()
        .filter(generation_run::Column::ProjectId.eq(project_id))
        .all(db)
        .await?
        .into_iter()
        .map(|row| row.id)
        .collect::<Vec<_>>();

    let mut deleted_generation_run_targets = 0;
    if !generation_run_ids.is_empty() {
        deleted_generation_run_targets += generation_run_target::Entity::delete_many()
            .filter(generation_run_target::Column::GenerationRunId.is_in(generation_run_ids))
            .exec(db)
            .await?
            .rows_affected;
    }
    if !generated_image_ids.is_empty() {
        deleted_generation_run_targets += generation_run_target::Entity::delete_many()
            .filter(generation_run_target::Column::TargetTable.eq("generated_image"))
            .filter(generation_run_target::Column::TargetId.is_in(generated_image_ids.to_vec()))
            .exec(db)
            .await?
            .rows_affected;
    }
    if !generated_video_ids.is_empty() {
        deleted_generation_run_targets += generation_run_target::Entity::delete_many()
            .filter(generation_run_target::Column::TargetTable.eq("generated_video"))
            .filter(generation_run_target::Column::TargetId.is_in(generated_video_ids.to_vec()))
            .exec(db)
            .await?
            .rows_affected;
    }
    let deleted_generation_runs = generation_run::Entity::delete_many()
        .filter(generation_run::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?
        .rows_affected;

    Ok(ProjectGenerationRunCleanupResult {
        deleted_generation_runs,
        deleted_generation_run_targets,
    })
}
