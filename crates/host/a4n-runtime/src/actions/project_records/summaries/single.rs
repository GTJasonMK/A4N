use crate::AppState;
use crate::actions::app_error_message;
use a4n_api_contract::project::ProjectSummary;

pub async fn project_summary_with_family_for_user(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ProjectSummary, String> {
    state
        .project_read_application()
        .summary(user_id, project_id)
        .await
        .map_err(app_error_message)
}
