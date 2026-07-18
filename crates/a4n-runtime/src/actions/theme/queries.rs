use super::contracts::ThemeSummary;
use super::projections::to_summary;
use super::rules::ensure_same_user;
use crate::AppState;
use crate::actions::app_error_message;
use a4n_core::error::AppError;

pub async fn list_themes(state: &AppState) -> Result<Vec<ThemeSummary>, String> {
    let themes = state
        .orchestrator
        .list_themes()
        .await
        .map_err(app_error_message)?;
    Ok(themes.iter().map(to_summary).collect())
}

pub async fn list_themes_for_user(
    state: &AppState,
    user_id: i64,
) -> Result<Vec<ThemeSummary>, AppError> {
    let themes = state
        .orchestrator
        .list_themes_visible_to_user(user_id)
        .await?;
    Ok(themes.iter().map(to_summary).collect())
}

pub async fn list_user_themes_for_user(
    state: &AppState,
    user_id: i64,
    target_user_id: i64,
) -> Result<Vec<ThemeSummary>, AppError> {
    ensure_same_user(user_id, target_user_id, "只能列出当前登录用户主题")?;
    list_user_themes(state, target_user_id).await
}

pub async fn get_system_default_theme(state: &AppState) -> Result<Option<ThemeSummary>, String> {
    let theme = state
        .orchestrator
        .system_default_theme()
        .await
        .map_err(app_error_message)?;
    Ok(theme.as_ref().map(to_summary))
}

async fn list_user_themes(state: &AppState, user_id: i64) -> Result<Vec<ThemeSummary>, AppError> {
    let themes = state.orchestrator.list_user_themes(user_id).await?;
    Ok(themes.iter().map(to_summary).collect())
}
