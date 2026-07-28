use super::contracts::{ImportThemeInput, ThemeSummary};
use super::projections::to_summary;
use crate::AppState;
use a4n_core::error::AppError;

pub async fn export_theme_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
) -> Result<String, AppError> {
    let theme = state
        .settings()
        .export_theme_for_user_validated(user_id, theme_id)
        .await?;
    serde_json::to_string_pretty(&to_summary(&theme)).map_err(AppError::Serialization)
}

pub async fn import_theme_for_user(
    state: &AppState,
    user_id: i64,
    input: ImportThemeInput,
) -> Result<ThemeSummary, AppError> {
    state
        .settings()
        .import_theme_for_user_validated(user_id, input)
        .await
        .map(|theme| to_summary(&theme))
}
