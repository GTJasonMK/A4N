use super::contracts::MessageSummary;
use super::projections::message_summary;
use crate::AppState;
use crate::actions::access;
use a4n_core::error::AppError;
use a4n_orchestrator::inspiration::validate_inspiration_history_limit;

pub async fn get_inspiration_history(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    limit: u64,
) -> Result<Vec<MessageSummary>, AppError> {
    validate_inspiration_history_limit(limit)?;
    access::verify_project_editor(state, user_id, project_id).await?;
    let messages = state
        .orchestrator()
        .inspiration_messages(project_id, limit)
        .await?;
    Ok(messages.iter().map(message_summary).collect())
}
