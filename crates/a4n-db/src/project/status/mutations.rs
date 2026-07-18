use super::{
    ProjectStatusHistoryInput, ProjectStatusWriteError, UpdateProjectStatusRecordInput,
    insert_project_status_history,
};
use crate::project::{project_by_id_for_update, required_text, validate_positive_i64};
use a4n_core::models::project::novel_project;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set, TransactionTrait};

pub async fn update_project_status_record<C>(
    db: &C,
    project: novel_project::Model,
    input: UpdateProjectStatusRecordInput,
) -> Result<novel_project::Model, ProjectStatusWriteError>
where
    C: ConnectionTrait + TransactionTrait,
{
    validate_positive_i64(project.id, "project_id")
        .map_err(ProjectStatusWriteError::InvalidArgument)?;
    let txn = db.begin().await?;
    let locked = project_by_id_for_update(&txn, project.id)
        .await
        .map_err(ProjectStatusWriteError::from)?
        .ok_or(ProjectStatusWriteError::ProjectNotFound {
            project_id: project.id,
        })?;
    if locked.status != project.status {
        return Err(ProjectStatusWriteError::StatusChanged {
            expected: project.status,
            actual: locked.status,
        });
    }
    let updated = update_project_status_record_in(&txn, locked, input).await?;
    txn.commit().await?;
    Ok(updated)
}

pub(crate) async fn update_project_status_record_in<C>(
    db: &C,
    project: novel_project::Model,
    input: UpdateProjectStatusRecordInput,
) -> Result<novel_project::Model, ProjectStatusWriteError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(project.id, "project_id")
        .map_err(ProjectStatusWriteError::InvalidArgument)?;
    let to_status = required_text(input.to_status, "to_status")
        .map_err(ProjectStatusWriteError::InvalidArgument)?;
    let history_from_status = required_text(input.history_from_status, "from_status")
        .map_err(ProjectStatusWriteError::InvalidArgument)?;
    let history_reason = required_text(input.history_reason, "reason")
        .map_err(ProjectStatusWriteError::InvalidArgument)?;

    let mut active: novel_project::ActiveModel = project.into();
    active.status = Set(to_status.clone());
    active.updated_at = Set(input.changed_at);
    if input.clear_draft_fields {
        active.raw_blueprint = Set(None);
        active.total_chapters = Set(None);
    }
    let updated = active.update(db).await?;

    insert_project_status_history(
        db,
        ProjectStatusHistoryInput {
            project_id: updated.id,
            from_status: history_from_status,
            to_status,
            reason: Some(history_reason),
            changed_by: input.changed_by,
            changed_at: input.changed_at,
        },
    )
    .await?;

    Ok(updated)
}
