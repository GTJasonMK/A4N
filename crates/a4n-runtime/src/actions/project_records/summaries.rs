mod batch;
mod projection;
mod single;

use crate::AppState;
use crate::actions::app_error_message;
use a4n_core::{error::AppError, models::project::novel_project};
use a4n_orchestrator::project::ProjectProgressQuery;

pub use batch::project_summaries_with_family_for_user;
pub use single::project_summary_with_family_for_user;

pub(super) fn project_progress_query(
    project: &novel_project::Model,
) -> Result<ProjectProgressQuery, String> {
    ProjectProgressQuery::from_project(project)
        .map_err(|error| AppError::Internal(anyhow::anyhow!(error.to_string())))
        .map_err(app_error_message)
}

pub(super) async fn project_record_by_id(
    state: &AppState,
    project_id: i64,
) -> Result<novel_project::Model, String> {
    state
        .orchestrator()
        .project_summary_by_id(project_id)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| format!("项目不存在: {project_id}"))
}
