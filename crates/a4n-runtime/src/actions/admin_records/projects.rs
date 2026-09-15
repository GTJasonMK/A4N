use super::AdminProjectSummary;
use crate::AppState;

pub(super) async fn list_all_projects(
    state: &AppState,
) -> Result<Vec<AdminProjectSummary>, String> {
    state
        .admin_read_application()
        .all_project_summaries()
        .await
        .map_err(|error| error.to_string())
}
