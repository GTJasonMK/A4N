use super::DuplicateProjectInput;
use super::media::RuntimeProjectDuplicationHostPort;
use crate::AppState;
use crate::actions::app_error_message;
use a4n_api_contract::project::ProjectSummary;
use a4n_orchestrator::project::DuplicateProjectApplicationInput;

pub async fn duplicate_project(
    state: &AppState,
    user_id: i64,
    input: DuplicateProjectInput,
) -> Result<ProjectSummary, String> {
    let mut host_port = RuntimeProjectDuplicationHostPort::new(state);
    state
        .project_duplication_application()
        .duplicate_project(
            DuplicateProjectApplicationInput {
                user_id,
                project_id: input.project_id,
                new_title: input.new_title,
            },
            &mut host_port,
        )
        .await
        .map_err(app_error_message)
}

pub async fn duplicate_project_summary(
    state: &AppState,
    user_id: i64,
    input: DuplicateProjectInput,
) -> Result<ProjectSummary, String> {
    duplicate_project(state, user_id, input).await
}
