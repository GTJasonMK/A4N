use crate::AppState;
use a4n_core::error::AppError;

pub async fn get_theme_css_variables_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
) -> Result<String, AppError> {
    state
        .settings()
        .theme_css_variables_for_user_validated(user_id, theme_id)
        .await
}
