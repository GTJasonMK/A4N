use crate::{
    AppState,
    actions::{access::verify_project_access_string, app_error_message},
};
use a4n_api_contract::project::ProjectStatusMachineSummary;

pub async fn get_project_status_machine(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ProjectStatusMachineSummary, String> {
    verify_project_access_string(state, user_id, project_id).await?;
    state
        .project_status()
        .project_status_machine_record(project_id)
        .await
        .map_err(app_error_message)
}
