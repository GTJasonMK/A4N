use super::super::queries::active_prompt_names_for_state;
use super::ProjectPromptOverrideSummary;
use crate::{AppState, actions::access};

pub async fn prompt_list_project_overrides(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ProjectPromptOverrideSummary>, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())?;
    let active_prompt_names = active_prompt_names_for_state(state).await?;
    state
        .orchestrator
        .project_override_summaries(project_id, &active_prompt_names)
        .await
        .map_err(crate::actions::app_error_message)
}
