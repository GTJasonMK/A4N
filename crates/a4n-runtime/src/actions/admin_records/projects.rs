use super::ProjectSummary;
use crate::{AppState, actions::app_error_message};

pub async fn list_all_projects(state: &AppState) -> Result<Vec<ProjectSummary>, String> {
    state
        .orchestrator
        .all_project_summaries()
        .await
        .map_err(app_error_message)
}
