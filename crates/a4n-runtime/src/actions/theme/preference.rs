use a4n_api_contract::theme::{ThemePreferenceSummary, UpdateThemePreferenceInput};
use a4n_error::AppError;

use crate::AppState;

pub async fn get_theme_preference_for_user(
    state: &AppState,
    user_id: i64,
) -> Result<ThemePreferenceSummary, AppError> {
    state.theme_application().get_preference(user_id).await
}

pub async fn update_theme_preference_for_user(
    state: &AppState,
    user_id: i64,
    input: UpdateThemePreferenceInput,
) -> Result<ThemePreferenceSummary, AppError> {
    state
        .theme_application()
        .update_preference(user_id, input)
        .await
}
