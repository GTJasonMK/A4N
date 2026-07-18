use super::access::ensure_theme_write_access;
use super::contracts::{CreateThemeInput, ThemeSummary, UpdateThemeInput};
use super::projections::to_summary;
use super::rules::{
    ensure_theme_can_be_deleted, ensure_theme_css_variables_object, normalize_theme_mode,
    normalize_theme_name,
};
use crate::AppState;
use a4n_core::error::AppError;

pub async fn create_theme_for_user(
    state: &AppState,
    user_id: i64,
    input: CreateThemeInput,
) -> Result<ThemeSummary, AppError> {
    create_theme(state, user_id, input).await
}

pub async fn update_theme_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
    input: UpdateThemeInput,
) -> Result<ThemeSummary, AppError> {
    ensure_theme_write_access(state, user_id, theme_id, "只能修改当前登录用户主题").await?;
    update_theme(state, theme_id, input).await
}

pub async fn delete_theme_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
) -> Result<(), AppError> {
    ensure_theme_write_access(state, user_id, theme_id, "只能删除当前登录用户主题").await?;
    delete_theme(state, theme_id).await
}

pub async fn activate_theme_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
) -> Result<ThemeSummary, AppError> {
    ensure_theme_write_access(state, user_id, theme_id, "只能激活当前登录用户主题").await?;
    activate_theme(state, theme_id).await
}

async fn create_theme(
    state: &AppState,
    user_id: i64,
    input: CreateThemeInput,
) -> Result<ThemeSummary, AppError> {
    let name = normalize_theme_name(input.name).map_err(AppError::InvalidArgument)?;
    let mode = normalize_theme_mode(input.mode).map_err(AppError::InvalidArgument)?;
    ensure_theme_css_variables_object(&input.css_variables).map_err(AppError::InvalidArgument)?;
    let theme = state
        .orchestrator
        .create_theme(Some(user_id), &name, &mode, input.css_variables)
        .await?;
    Ok(to_summary(&theme))
}

async fn update_theme(
    state: &AppState,
    theme_id: i64,
    input: UpdateThemeInput,
) -> Result<ThemeSummary, AppError> {
    let name = input
        .name
        .map(normalize_theme_name)
        .transpose()
        .map_err(AppError::InvalidArgument)?;
    let mode = input
        .mode
        .map(normalize_theme_mode)
        .transpose()
        .map_err(AppError::InvalidArgument)?;
    if let Some(css_variables) = input.css_variables.as_ref() {
        ensure_theme_css_variables_object(css_variables).map_err(AppError::InvalidArgument)?;
    }
    let theme = state
        .orchestrator
        .update_theme(
            theme_id,
            name.as_deref(),
            mode.as_deref(),
            input.css_variables,
        )
        .await?;
    Ok(to_summary(&theme))
}

async fn delete_theme(state: &AppState, theme_id: i64) -> Result<(), AppError> {
    let theme = state
        .orchestrator
        .theme_by_id(theme_id)
        .await?
        .ok_or_else(|| AppError::NotFound("主题不存在".to_string()))?;
    ensure_theme_can_be_deleted(theme.is_system_default, theme.is_active)
        .map_err(AppError::InvalidArgument)?;
    state.orchestrator.delete_theme(theme_id).await
}

async fn activate_theme(state: &AppState, theme_id: i64) -> Result<ThemeSummary, AppError> {
    let updated = state.orchestrator.activate_theme(theme_id).await?;
    Ok(to_summary(&updated))
}
