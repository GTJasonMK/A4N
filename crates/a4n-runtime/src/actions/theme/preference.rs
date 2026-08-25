use a4n_api_contract::theme::{
    ThemeAppearance, ThemePreferenceSummary, UpdateThemePreferenceInput,
};
use a4n_core::error::AppError;

use crate::AppState;

pub async fn get_theme_preference_for_user(
    state: &AppState,
    user_id: i64,
) -> Result<ThemePreferenceSummary, AppError> {
    let appearance = state
        .orchestrator()
        .get_theme_preference_for_user_validated(user_id)
        .await?
        .map(|preference| preference.appearance.parse::<ThemeAppearance>())
        .transpose()
        .map_err(|error| AppError::ConfigError(format!("主题外观偏好无效：{error}")))?
        .unwrap_or(ThemeAppearance::System);
    Ok(ThemePreferenceSummary {
        user_id,
        appearance,
    })
}

pub async fn update_theme_preference_for_user(
    state: &AppState,
    user_id: i64,
    input: UpdateThemePreferenceInput,
) -> Result<ThemePreferenceSummary, AppError> {
    let preference = state
        .orchestrator()
        .update_theme_preference_for_user_validated(user_id, input.appearance)
        .await?;
    let appearance = preference
        .appearance
        .parse::<ThemeAppearance>()
        .map_err(|error| AppError::ConfigError(format!("主题外观偏好无效：{error}")))?;
    Ok(ThemePreferenceSummary {
        user_id,
        appearance,
    })
}
