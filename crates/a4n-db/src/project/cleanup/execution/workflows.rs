use super::super::{ProjectCleanupDbError, ProjectWorkflowCleanupResult};
use crate::project::validate_positive_i64;
use a4n_core::models::generation::{workflow_checkpoint, workflow_progress_event, workflow_run};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub(crate) async fn delete_project_workflow_runs<C>(
    db: &C,
    project_id: i64,
) -> Result<ProjectWorkflowCleanupResult, ProjectCleanupDbError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectCleanupDbError::InvalidArgument)?;
    let workflow_run_ids = workflow_run::Entity::find()
        .filter(workflow_run::Column::ProjectId.eq(project_id))
        .all(db)
        .await?
        .into_iter()
        .map(|row| row.id)
        .collect::<Vec<_>>();

    let mut deleted_workflow_checkpoints = 0;
    let mut deleted_workflow_progress_events = 0;
    if !workflow_run_ids.is_empty() {
        deleted_workflow_checkpoints = workflow_checkpoint::Entity::delete_many()
            .filter(workflow_checkpoint::Column::WorkflowRunId.is_in(workflow_run_ids.clone()))
            .exec(db)
            .await?
            .rows_affected;
        deleted_workflow_progress_events = workflow_progress_event::Entity::delete_many()
            .filter(workflow_progress_event::Column::WorkflowRunId.is_in(workflow_run_ids))
            .exec(db)
            .await?
            .rows_affected;
    }
    let deleted_workflow_runs = workflow_run::Entity::delete_many()
        .filter(workflow_run::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?
        .rows_affected;

    Ok(ProjectWorkflowCleanupResult {
        deleted_workflow_runs,
        deleted_workflow_checkpoints,
        deleted_workflow_progress_events,
    })
}
