use super::SessionSummary;
use crate::AppState;
use a4n_error::AppError;

pub async fn list_sessions(
    state: &AppState,
    user_id: i64,
) -> Result<Vec<SessionSummary>, AppError> {
    state.auth_read_application().list_sessions(user_id).await
}

pub async fn revoke_session(
    state: &AppState,
    user_id: i64,
    session_id: &str,
) -> Result<(), AppError> {
    state
        .auth_mutation_application()
        .revoke_session(user_id, session_id)
        .await
}
