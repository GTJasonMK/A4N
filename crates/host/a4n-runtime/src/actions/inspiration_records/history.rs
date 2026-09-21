use super::{InspirationHistoryInput, InspirationMessageSummary};
use crate::AppState;
use a4n_error::AppError;

pub async fn get_inspiration_history(
    state: &AppState,
    user_id: i64,
    input: InspirationHistoryInput,
) -> Result<Vec<InspirationMessageSummary>, AppError> {
    state
        .inspiration_history_read_application()
        .history(user_id, input)
        .await
}
