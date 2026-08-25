use super::super::ProjectCleanupDbError;
use crate::project::validate_positive_i64;
use a4n_core::models::generation::{workflow_checkpoint, workflow_progress_event, workflow_run};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};

pub(crate) async fn delete_project_workflow_runs<C>(
    db: &C,
    project_id: i64,
) -> Result<(), ProjectCleanupDbError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectCleanupDbError::InvalidArgument)?;
    let workflow_run_ids = workflow_run::Entity::find()
        .select_only()
        .column(workflow_run::Column::Id)
        .filter(workflow_run::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .all(db)
        .await?;

    if !workflow_run_ids.is_empty() {
        workflow_checkpoint::Entity::delete_many()
            .filter(workflow_checkpoint::Column::WorkflowRunId.is_in(workflow_run_ids.clone()))
            .exec(db)
            .await?;
        workflow_progress_event::Entity::delete_many()
            .filter(workflow_progress_event::Column::WorkflowRunId.is_in(workflow_run_ids))
            .exec(db)
            .await?;
    }
    workflow_run::Entity::delete_many()
        .filter(workflow_run::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?;

    Ok(())
}
