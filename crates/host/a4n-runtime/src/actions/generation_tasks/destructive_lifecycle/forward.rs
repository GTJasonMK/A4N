use a4n_domain::generation::ProjectLifecycleOperation;
use a4n_error::AppError;
use a4n_generation_host_contract::ProjectGeneratedDataResetApplicationResult;

use crate::AppState;

pub(super) enum DeleteProjectForwardError {
    Rejected(String),
    Retryable(String),
}

pub(super) async fn delete_project_forward(
    state: &AppState,
    project_id: i64,
) -> Result<(), DeleteProjectForwardError> {
    match state
        .generation_lifecycle_application()
        .delete_project_cascade_validated(project_id)
        .await
    {
        Ok(()) => Ok(()),
        Err(error @ AppError::InvalidArgument(_)) => {
            let error = error.to_string();
            match state
                .generation_lifecycle_application()
                .restore_generation_project_lifecycle_gate(
                    project_id,
                    ProjectLifecycleOperation::Delete,
                )
                .await
            {
                Ok(()) => {
                    tracing::warn!(project_id, error = %error, "项目删除不可执行，已恢复 lifecycle gate");
                    Err(DeleteProjectForwardError::Rejected(error))
                }
                Err(restore_error) => Err(DeleteProjectForwardError::Retryable(format!(
                    "{error}；恢复 project lifecycle gate 失败：{restore_error}"
                ))),
            }
        }
        Err(error) => Err(DeleteProjectForwardError::Retryable(error.to_string())),
    }
}

pub(super) async fn reset_project_forward(
    state: &AppState,
    project_id: i64,
) -> Result<ProjectGeneratedDataResetApplicationResult, String> {
    let result = state
        .generation_lifecycle_application()
        .reset_project_generated_data_validated(project_id)
        .await
        .map_err(|error| error.to_string())?;
    state
        .rag_ingest()
        .clear_project_rag(project_id)
        .await
        .map_err(|error| error.to_string())?;
    state
        .generation_lifecycle_application()
        .complete_generation_project_reset_gate(project_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(result)
}
