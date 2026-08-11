use super::contracts::SessionSummary;
use super::projections::to_session_summary;
use crate::AppState;
use a4n_core::error::AppError;

pub async fn list_sessions(
    state: &AppState,
    user_id: i64,
) -> Result<Vec<SessionSummary>, AppError> {
    let sessions = state.orchestrator().list_sessions(user_id).await?;
    Ok(sessions.iter().map(to_session_summary).collect())
}

pub async fn revoke_session(
    state: &AppState,
    user_id: i64,
    session_id: &str,
) -> Result<(), AppError> {
    state
        .orchestrator()
        .revoke_session(user_id, session_id)
        .await
}
