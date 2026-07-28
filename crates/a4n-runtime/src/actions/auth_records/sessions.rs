use super::contracts::SessionSummary;
use super::projections::to_session_summary;
use crate::AppState;
use a4n_core::error::AppError;

pub async fn logout_user(
    state: &AppState,
    user_id: Option<i64>,
    session_id: Option<&str>,
) -> Result<(), AppError> {
    state.auth().logout_user_session(user_id, session_id).await
}

pub async fn list_sessions_for_user(
    state: &AppState,
    caller_user_id: i64,
    target_user_id: i64,
) -> Result<Vec<SessionSummary>, AppError> {
    let sessions = state
        .auth()
        .list_own_sessions(caller_user_id, target_user_id)
        .await?;
    Ok(sessions.iter().map(to_session_summary).collect())
}

pub async fn revoke_session_for_user(
    state: &AppState,
    caller_user_id: i64,
    target_user_id: i64,
    session_id: &str,
) -> Result<(), AppError> {
    state
        .auth()
        .revoke_own_session(caller_user_id, target_user_id, session_id)
        .await
}
