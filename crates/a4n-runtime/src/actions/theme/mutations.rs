use std::collections::HashSet;

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
    to_summary(state.orchestrator(), user_id, theme).await
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
    to_summary(state.orchestrator(), user_id, theme).await
}

pub async fn delete_theme_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
) -> Result<(), AppError> {
    let resources = state
        .orchestrator()
        .list_theme_resources_for_user_validated(user_id, theme_id)
        .await?;
    state
        .orchestrator()
        .delete_theme_for_user_validated(user_id, theme_id)
        .await?;

    let mut first_cleanup_error = None;
    let mut resource_paths = HashSet::new();
    for resource in resources {
        if !resource_paths.insert(resource.file_path.clone()) {
            continue;
        }
        if let Err(error) = crate::file_utils::remove_data_file_in_data_dir(
            state.data_dir(),
            &resource.file_path,
            "主题背景图片",
        ) {
            tracing::error!(theme_id, resource_id = resource.id, %error, "删除主题资源文件失败");
            first_cleanup_error.get_or_insert(error);
        }
    }
    if let Some(error) = first_cleanup_error {
        return Err(AppError::Internal(anyhow::anyhow!(error)));
    }
    Ok(())
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
    to_summary(state.orchestrator(), user_id, theme).await
}
