use super::ThemeSummary;
use super::projections::to_summary;
use crate::AppState;
use a4n_core::error::AppError;

pub async fn list_themes_for_user(
    state: &AppState,
    user_id: i64,
) -> Result<Vec<ThemeSummary>, AppError> {
    let themes = state
        .settings()
        .list_themes_for_user_validated(user_id)
        .await?;
    Ok(themes.iter().map(to_summary).collect())
}

pub async fn list_user_themes_for_user(
    state: &AppState,
    user_id: i64,
    target_user_id: i64,
) -> Result<Vec<ThemeSummary>, AppError> {
    let themes = state
        .settings()
        .list_user_themes_for_user_validated(user_id, target_user_id)
        .await?;
    Ok(themes.iter().map(to_summary).collect())
}
