use super::rules::ensure_same_user;
use crate::AppState;
use a4n_core::error::AppError;

pub(super) async fn ensure_theme_access(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
) -> Result<(), AppError> {
    let theme_user_id = theme_user_id(state, theme_id).await?;
    if let Some(owner_id) = theme_user_id {
        ensure_same_user(user_id, owner_id, "只能访问当前登录用户主题")?;
    }
    Ok(())
}

pub(super) async fn ensure_theme_write_access(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
    message: &str,
) -> Result<(), AppError> {
    let theme_user_id = theme_user_id(state, theme_id).await?;
    let Some(owner_id) = theme_user_id else {
        return Err(AppError::Forbidden(message.to_string()));
    };
    ensure_same_user(user_id, owner_id, message)
}

async fn theme_user_id(state: &AppState, theme_id: i64) -> Result<Option<i64>, AppError> {
    let theme = state
        .orchestrator
        .theme_by_id(theme_id)
        .await?
        .ok_or_else(|| AppError::NotFound("主题不存在".to_string()))?;
    Ok(theme.user_id)
}
