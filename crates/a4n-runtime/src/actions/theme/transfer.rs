use super::access::ensure_theme_access;
use super::contracts::{ImportThemeInput, ThemeImportPayload, ThemeSummary};
use super::projections::to_summary;
use super::rules::{ensure_theme_css_variables_object, normalize_theme_mode, normalize_theme_name};
use crate::AppState;
use a4n_core::error::AppError;

pub async fn export_theme_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
) -> Result<String, AppError> {
    ensure_theme_access(state, user_id, theme_id).await?;
    export_theme(state, theme_id).await
}

pub async fn import_theme_for_user(
    state: &AppState,
    user_id: i64,
    input: ImportThemeInput,
) -> Result<ThemeSummary, AppError> {
    import_theme(state, user_id, input).await
}

async fn export_theme(state: &AppState, theme_id: i64) -> Result<String, AppError> {
    let theme = state
        .orchestrator
        .theme_by_id(theme_id)
        .await?
        .ok_or_else(|| AppError::NotFound("主题不存在".to_string()))?;
    serde_json::to_string_pretty(&to_summary(&theme)).map_err(AppError::Serialization)
}

async fn import_theme(
    state: &AppState,
    user_id: i64,
    input: ImportThemeInput,
) -> Result<ThemeSummary, AppError> {
    let incoming: ThemeImportPayload = serde_json::from_str(&input.theme_json)
        .map_err(|error| AppError::InvalidArgument(format!("JSON 解析失败: {error}")))?;
    let name = normalize_theme_name(incoming.name).map_err(AppError::InvalidArgument)?;
    let mode = normalize_theme_mode(incoming.mode).map_err(AppError::InvalidArgument)?;
    ensure_theme_css_variables_object(&incoming.css_variables)
        .map_err(AppError::InvalidArgument)?;
    let theme = state
        .orchestrator
        .create_theme(Some(user_id), &name, &mode, incoming.css_variables)
        .await?;
    Ok(to_summary(&theme))
}
