use super::{CreateThemeInput, ThemeSummary, UpdateThemeInput};
use super::projections::to_summary;
use crate::AppState;
use a4n_core::error::AppError;

pub async fn create_theme_for_user(
    state: &AppState,
    user_id: i64,
    input: CreateThemeInput,
) -> Result<ThemeSummary, AppError> {
    state
        .settings()
        .create_theme_for_user_validated(user_id, input)
        .await
        .map(|theme| to_summary(&theme))
}

pub async fn update_theme_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
    input: UpdateThemeInput,
) -> Result<ThemeSummary, AppError> {
    state
        .settings()
        .update_theme_for_user_validated(user_id, theme_id, input)
        .await
        .map(|theme| to_summary(&theme))
}

pub async fn delete_theme_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
) -> Result<(), AppError> {
    state
        .settings()
        .delete_theme_for_user_validated(user_id, theme_id)
        .await
}

pub async fn activate_theme_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
) -> Result<ThemeSummary, AppError> {
    state
        .settings()
        .activate_theme_for_user_validated(user_id, theme_id)
        .await
        .map(|theme| to_summary(&theme))
}
