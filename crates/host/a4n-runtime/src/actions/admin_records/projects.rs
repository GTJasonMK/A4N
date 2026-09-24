use super::AdminProjectSummary;
use crate::AppState;

pub(super) async fn list_all_projects(
    state: &AppState,
    caller_user_id: i64,
) -> Result<Vec<AdminProjectSummary>, String> {
    state
        .admin_read_application()
        .all_project_summaries(caller_user_id)
        .await
        .map_err(|error| error.to_string())
}
