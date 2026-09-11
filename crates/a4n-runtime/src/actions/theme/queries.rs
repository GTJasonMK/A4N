use super::ThemeSummary;
use crate::AppState;
use a4n_error::AppError;

pub async fn list_themes_for_user(
    state: &AppState,
    user_id: i64,
) -> Result<Vec<ThemeSummary>, AppError> {
    state.theme_application().list_themes(user_id).await
}
