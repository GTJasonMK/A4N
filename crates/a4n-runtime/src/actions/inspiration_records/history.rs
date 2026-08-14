use super::{InspirationHistoryInput, InspirationMessageSummary};
use crate::AppState;
use crate::actions::access;
use a4n_core::error::AppError;

const INSPIRATION_HISTORY_LIMIT: u64 = 80;

pub async fn get_inspiration_history(
    state: &AppState,
    user_id: i64,
    input: InspirationHistoryInput,
) -> Result<Vec<InspirationMessageSummary>, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let messages = state
        .orchestrator()
        .inspiration_messages(input.project_id, INSPIRATION_HISTORY_LIMIT)
        .await?;
    Ok(messages
        .into_iter()
        .map(|message| InspirationMessageSummary {
            id: message.id,
            role: message.role,
            content: message.content,
            created_at: message.created_at.to_string(),
        })
        .collect())
}
