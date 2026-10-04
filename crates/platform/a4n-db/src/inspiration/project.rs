use super::contracts::InspirationDbError;
use crate::project::{ProjectMutationError, project_summary_by_id_record};
use a4n_domain::project::ProjectSummaryRecord;

async fn inspiration_project_by_id(
    db: &impl crate::DbExecutor,
    project_id: i64,
) -> Result<Option<ProjectSummaryRecord>, InspirationDbError> {
    project_summary_by_id_record(db, project_id)
        .await
        .map_err(inspiration_project_read_error)
}

fn inspiration_project_read_error(error: ProjectMutationError) -> InspirationDbError {
    match error {
        ProjectMutationError::InvalidArgument(message)
        | ProjectMutationError::Forbidden(message)
        | ProjectMutationError::Internal(message) => InspirationDbError::InvalidArgument(message),
        ProjectMutationError::NotFound(message) => InspirationDbError::NotFound(message),
        ProjectMutationError::Database(error) => InspirationDbError::Database(error),
    }
}

pub(super) async fn ensure_inspiration_project_exists(
    db: &impl crate::DbExecutor,
    project_id: i64,
) -> Result<ProjectSummaryRecord, InspirationDbError> {
    inspiration_project_by_id(db, project_id)
        .await?
        .ok_or_else(|| InspirationDbError::NotFound(format!("项目不存在: {project_id}")))
}
