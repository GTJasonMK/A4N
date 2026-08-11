use super::ThemeSummary;
use super::projections::to_summary;
use crate::AppState;
use a4n_core::error::AppError;

pub async fn list_themes_for_user(
    state: &AppState,
    user_id: i64,
) -> Result<Vec<ThemeSummary>, AppError> {
    let themes = state
        .orchestrator()
        .list_themes_for_user_validated(user_id)
        .await?;
    themes.into_iter().map(to_summary).collect()
}
