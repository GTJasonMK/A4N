use crate::AppState;
use crate::actions::app_error_message;
use a4n_api_contract::project::ProjectSummary;

pub async fn project_summaries_with_family_for_user(
    state: &AppState,
    user_id: i64,
    project_ids: &[i64],
) -> Result<Vec<ProjectSummary>, String> {
    let mut summaries = Vec::with_capacity(project_ids.len());
    for &project_id in project_ids {
        summaries.push(
            state
                .project_read_application()
                .summary(user_id, project_id)
                .await
                .map_err(app_error_message)?,
        );
    }
    Ok(summaries)
}
