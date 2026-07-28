use super::StatusHistorySummary;
use crate::{
    AppState,
    actions::{access::verify_project_access_string, app_error_message},
};

pub async fn get_project_status_history(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<StatusHistorySummary>, String> {
    verify_project_access_string(state, user_id, project_id).await?;
    let history = state
        .project_read()
        .project_status_history(project_id)
        .await
        .map_err(app_error_message)?;
    Ok(history
        .into_iter()
        .map(|item| StatusHistorySummary {
            id: item.id,
            project_id: item.project_id,
            from_status: item.from_status,
            to_status: item.to_status,
            reason: item.reason,
            created_at: Some(item.changed_at),
        })
        .collect())
}
