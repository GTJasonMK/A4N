use super::projections::to_summary;
use super::{CreateThemeInput, ThemeSummary, UpdateThemeInput};
use crate::AppState;
use a4n_core::error::AppError;

pub async fn create_theme_for_user(
    state: &AppState,
    user_id: i64,
    input: CreateThemeInput,
) -> Result<ThemeSummary, AppError> {
    let theme = state
        .orchestrator()
        .create_theme_for_user_validated(user_id, input)
        .await?;
    to_summary(theme)
}

pub async fn update_theme_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
    input: UpdateThemeInput,
) -> Result<ThemeSummary, AppError> {
    let theme = state
        .orchestrator()
        .update_theme_for_user_validated(user_id, theme_id, input)
        .await?;
    to_summary(theme)
}

pub async fn delete_theme_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
) -> Result<(), AppError> {
    state
        .orchestrator()
        .delete_theme_for_user_validated(user_id, theme_id)
        .await
}

pub async fn activate_theme_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
) -> Result<ThemeSummary, AppError> {
    let theme = state
        .orchestrator()
        .activate_theme_for_user_validated(user_id, theme_id)
        .await?;
    to_summary(theme)
}
