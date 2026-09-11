use super::{CreateThemeInput, ThemeSummary};
use crate::AppState;
use a4n_error::AppError;

pub async fn export_theme_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
) -> Result<String, AppError> {
    let theme = state
        .theme_application()
        .export_theme(user_id, theme_id)
        .await?;
    serde_json::to_string_pretty(&CreateThemeInput {
        name: theme.name,
        mode: theme.mode,
        document: theme.document.without_resource_references(),
    })
    .map_err(AppError::Serialization)
}

pub async fn import_theme_for_user(
    state: &AppState,
    user_id: i64,
    input: CreateThemeInput,
) -> Result<ThemeSummary, AppError> {
    state.theme_application().import_theme(user_id, input).await
}
