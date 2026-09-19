use crate::AppState;
use a4n_api_contract::project::{
    ProjectRollbackPreview, ProjectStatus, ProjectStatusMachineSummary,
    ProjectStatusMutationResult, ProjectSummary,
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
    target: ProjectStatus,
) -> Result<ProjectRollbackPreview, String> {
    state
        .project_status_read_application()
        .preview_rollback(user_id, project_id, target)
        .await
        .map_err(|error| error.to_string())
}

pub async fn forward_project_status(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    target: ProjectStatus,
) -> Result<ProjectSummary, String> {
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
    target: ProjectStatus,
) -> Result<ProjectSummary, String> {
    let result = state
        .project_status_mutation_application()
        .rollback_status(user_id, project_id, target)
        .await
        .map_err(|error| error.to_string())?;
    finish_status_mutation(state, result).await
}

async fn finish_status_mutation(
    state: &AppState,
    result: ProjectStatusMutationResult,
) -> Result<ProjectSummary, String> {
    state
        .event_bus()
        .publish(InternalEvent::ProjectStatusChanged {
            project_id: result.project.id,
            from: result.from,
            to: result.to,
            timestamp: result.changed_at,
        })
        .map_err(|error| error.to_string())?;
    if result.needs_rag_clear {
        state
            .rag_ingest()
            .clear_project_rag(result.project.id)
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(result.project)
}
