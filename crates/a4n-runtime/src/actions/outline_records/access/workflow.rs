use super::super::{OutlineProgressReporter, OutlineTaskScope};
use crate::AppState;
use crate::actions::access as project_access;
use a4n_core::error::AppError;
use a4n_core::state::status::ProjectStatus;

pub(in crate::actions::outline_records) async fn outline_project_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    total: u32,
) -> Result<OutlineTaskScope, AppError> {
    project_access::verify_project_editor(state, user_id, project_id).await?;
    Ok(OutlineTaskScope { project_id, total })
}

pub(in crate::actions::outline_records) async fn advance_status(
    state: &AppState,
    project_id: i64,
    next_status: ProjectStatus,
) -> Result<(), AppError> {
    state
        .orchestrator
        .advance_project_status_if_allowed(project_id, next_status)
        .await
}

pub(in crate::actions::outline_records) fn report(
    progress: Option<OutlineProgressReporter<'_>>,
    current: u32,
    total: u32,
    message: &str,
) -> Result<(), AppError> {
    if let Some(progress) = progress {
        progress(current, total.max(1), message)?;
    }
    Ok(())
}
