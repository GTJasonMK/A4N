use super::{ProjectStatusHistoryInput, ProjectStatusHistorySummary, ProjectStatusWriteError};
use crate::project::{
    ProjectCleanupDbError, ProjectMutationError, required_text, validate_positive_i64,
};
use a4n_core::models::project::project_status_history;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

pub async fn insert_project_status_history<C>(
    db: &C,
    input: ProjectStatusHistoryInput,
) -> Result<(), ProjectStatusWriteError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(input.project_id, "project_id")
        .map_err(ProjectStatusWriteError::InvalidArgument)?;
    let from_status = required_text(input.from_status, "from_status")
        .map_err(ProjectStatusWriteError::InvalidArgument)?;
    let to_status = required_text(input.to_status, "to_status")
        .map_err(ProjectStatusWriteError::InvalidArgument)?;
    project_status_history::Entity::insert(project_status_history::ActiveModel {
        project_id: Set(input.project_id),
        from_status: Set(from_status),
        to_status: Set(to_status),
        reason: Set(normalize_optional_text(input.reason)),
        changed_by: Set(normalize_optional_text(input.changed_by)),
        changed_at: Set(input.changed_at),
        ..Default::default()
    })
    .exec(db)
    .await?;
    Ok(())
}

pub async fn delete_project_status_history<C>(
    db: &C,
    project_id: i64,
) -> Result<u64, ProjectCleanupDbError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectCleanupDbError::InvalidArgument)?;
    project_status_history::Entity::delete_many()
        .filter(project_status_history::Column::ProjectId.eq(project_id))
        .exec(db)
        .await
        .map(|result| result.rows_affected)
        .map_err(|source| ProjectCleanupDbError::Database { source })
}

pub async fn project_status_history(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Vec<ProjectStatusHistorySummary>, ProjectMutationError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectMutationError::InvalidArgument)?;
    let history = project_status_history::Entity::find()
        .filter(project_status_history::Column::ProjectId.eq(project_id))
        .order_by_desc(project_status_history::Column::ChangedAt)
        .all(db)
        .await?;
    Ok(history
        .into_iter()
        .map(project_status_history_summary)
        .collect())
}

pub(super) fn project_status_history_summary(
    item: project_status_history::Model,
) -> ProjectStatusHistorySummary {
    ProjectStatusHistorySummary {
        id: item.id,
        project_id: item.project_id,
        from_status: item.from_status,
        to_status: item.to_status,
        reason: item.reason,
        changed_at: item.changed_at.to_string(),
    }
}

pub(super) fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
