use super::projections::to_summary;
use super::{CreateThemeInput, ThemeSummary};
use crate::AppState;
use a4n_core::error::AppError;

pub async fn export_theme_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
) -> Result<String, AppError> {
    let theme = state
        .orchestrator()
        .export_theme_for_user_validated(user_id, theme_id)
        .await?;
    let theme = to_summary(theme)?;
    serde_json::to_string_pretty(&CreateThemeInput {
        name: theme.name,
        mode: theme.mode,
        css_variables: theme.css_variables,
    })
    .map_err(AppError::Serialization)
}

pub async fn import_theme_for_user(
    state: &AppState,
    user_id: i64,
    input: CreateThemeInput,
) -> Result<ThemeSummary, AppError> {
    let theme = state
        .orchestrator()
        .import_theme_for_user_validated(user_id, input)
        .await?;
    to_summary(theme)
}
