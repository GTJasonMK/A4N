use a4n_core::error::AppError;
use a4n_db::workflow::{self, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED};

use crate::OrchestratorContext;
use crate::workflow::workflow_write_app_error;

pub(super) async fn finalize_refine_workflow(
    context: &OrchestratorContext,
    workflow_run_id: i64,
    result: &Result<a4n_core::models::project::novel_blueprint::Model, AppError>,
) -> Result<(), AppError> {
    let (status, can_resume, error_message) = match result {
        Ok(_) => (WORKFLOW_STATUS_COMPLETED, false, None),
        Err(error) => (WORKFLOW_STATUS_FAILED, true, Some(error.to_string())),
    };
    workflow::mark_workflow_status(
        context.db.connection(),
        workflow_run_id,
        status,
        can_resume,
        error_message.as_deref(),
    )
    .await
    .map_err(workflow_write_app_error)?;
    Ok(())
}
