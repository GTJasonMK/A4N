use crate::AppState;
use a4n_api_contract::project::{
    ProjectRollbackPreview, ProjectStatus, ProjectStatusMachineSummary, ProjectStatusMutationResult,
};
use a4n_event::InternalEvent;

pub async fn get_project_status_machine(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ProjectStatusMachineSummary, String> {
    state
        .project_status_read_application()
        .status_machine(user_id, project_id)
        .await
        .map_err(|error| error.to_string())
}

pub async fn preview_rollback(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    expected_from: ProjectStatus,
    target: ProjectStatus,
) -> Result<ProjectRollbackPreview, String> {
    state
        .project_status_read_application()
        .preview_rollback(user_id, project_id, expected_from, target)
        .await
        .map_err(|error| error.to_string())
}

pub async fn forward_project_status(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    target: ProjectStatus,
) -> Result<ProjectStatusMutationResult, String> {
    let result = state
        .project_status_mutation_application()
        .forward_status(user_id, project_id, target)
        .await
        .map_err(|error| error.to_string())?;
    finish_status_mutation(state, result).await
}

pub async fn rollback_project_status(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    expected_from: ProjectStatus,
    target: ProjectStatus,
) -> Result<ProjectStatusMutationResult, String> {
    let result = state
        .project_status_mutation_application()
        .rollback_status(user_id, project_id, expected_from, target)
        .await
        .map_err(|error| error.to_string())?;
    finish_status_mutation(state, result).await
}

async fn finish_status_mutation(
    state: &AppState,
    mut result: ProjectStatusMutationResult,
) -> Result<ProjectStatusMutationResult, String> {
    if let Err(error) = state
        .event_bus()
        .publish(InternalEvent::ProjectStatusChanged {
            project_id: result.project_id,
            from: result.from,
            to: result.to,
            timestamp: result.changed_at,
        })
    {
        result.post_commit_errors.push(error.to_string());
    }
    if result.needs_rag_clear {
        if let Err(error) = state
            .rag_ingest()
            .clear_project_rag(result.project_id)
            .await
        {
            result.post_commit_errors.push(error.to_string());
        }
    }
    Ok(result)
}
