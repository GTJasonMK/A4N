use super::contracts::SessionSummary;
use super::projections::to_session_summary;
use super::rules::{
    ensure_same_user, required_logout_identity, required_session_id, required_session_user_id,
};
use crate::AppState;
use a4n_core::error::AppError;

pub async fn logout_user(
    state: &AppState,
    user_id: Option<i64>,
    session_id: Option<&str>,
) -> Result<(), AppError> {
    let (user_id, session_id) = required_logout_identity(user_id, session_id)?;
    state
        .orchestrator
        .revoke_user_session(user_id, &session_id)
        .await
}

pub async fn list_sessions_for_user(
    state: &AppState,
    caller_user_id: i64,
    target_user_id: i64,
) -> Result<Vec<SessionSummary>, AppError> {
    ensure_same_user(caller_user_id, target_user_id, "只能列出当前登录用户会话")?;
    let user_id = required_session_user_id(target_user_id, "列出会话")?;
    let sessions = state.orchestrator.user_sessions(user_id).await?;
    Ok(sessions.iter().map(to_session_summary).collect())
}

pub async fn revoke_session_for_user(
    state: &AppState,
    caller_user_id: i64,
    target_user_id: i64,
    session_id: &str,
) -> Result<(), AppError> {
    ensure_same_user(caller_user_id, target_user_id, "只能撤销当前登录用户会话")?;
    let user_id = required_session_user_id(target_user_id, "撤销会话")?;
    let session_id = required_session_id(session_id, "撤销会话")?;
    state
        .orchestrator
        .revoke_user_session(user_id, &session_id)
        .await
}
