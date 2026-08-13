use super::DuplicateProjectInput;
use super::media::RuntimeProjectDuplicationHostPort;
use crate::AppState;
use crate::actions::app_error_message;
use a4n_core::models::project::novel_project;
use a4n_orchestrator::project::DuplicateProjectApplicationInput;

pub async fn duplicate_project(
    state: &AppState,
    user_id: i64,
    input: DuplicateProjectInput,
) -> Result<novel_project::Model, String> {
    let mut host_port = RuntimeProjectDuplicationHostPort::new(state);
    state
        .orchestrator()
        .duplicate_project_validated(
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
