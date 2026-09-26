use std::collections::HashSet;

use super::resources::{prepare_theme_resource_file, remove_theme_resource_file_if_unused};
use super::{CreateThemeInput, ThemeSummary, UpdateThemeInput};
use crate::AppState;
use a4n_error::AppError;
use a4n_storage::remove_data_file_in_data_dir;

pub async fn create_theme_for_user(
    state: &AppState,
    user_id: i64,
    input: CreateThemeInput,
) -> Result<ThemeSummary, AppError> {
    state
        .theme_mutation_application()
        .create_theme(user_id, input)
        .await
}

pub async fn update_theme_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
    mut input: UpdateThemeInput,
) -> Result<ThemeSummary, AppError> {
    let previous = if input.resource_uploads.is_empty() && input.delete_resource_ids.is_empty() {
        Vec::new()
    } else {
        state
            .theme_read_application()
            .list_resources(user_id, theme_id)
            .await?
    };
    let mut created_paths = Vec::new();
    let result = async {
        let mut resources = Vec::new();
        for upload in std::mem::take(&mut input.resource_uploads) {
            if upload.theme_id != theme_id {
                return Err(AppError::InvalidArgument("主题资源归属不一致".into()));
            }
            let (prepared, created) = prepare_theme_resource_file(state, upload)?;
            if created {
                created_paths.push(prepared.file_path.clone());
            }
            resources.push(prepared);
        }
        state
            .theme_mutation_application()
            .update_theme(user_id, theme_id, input, resources)
            .await
    }
    .await;
    let saved = match result {
        Ok(saved) => saved,
        Err(error) => {
            let mut cleanup_errors = Vec::new();
            for path in created_paths {
                if let Err(cleanup) =
                    remove_theme_resource_file_if_unused(state, user_id, theme_id, &path).await
                {
                    cleanup_errors.push(cleanup.to_string());
                }
            }
            if !cleanup_errors.is_empty() {
                return Err(AppError::Internal(anyhow::anyhow!(
                    "{error}; 清理未保存主题资源失败: {}",
                    cleanup_errors.join("; ")
                )));
            }
            return Err(error);
        }
    };
    for previous in previous {
        if !saved
            .resources
            .iter()
            .any(|resource| resource.file_path == previous.file_path)
            && let Err(error) =
                remove_theme_resource_file_if_unused(state, user_id, theme_id, &previous.file_path)
                    .await
        {
            tracing::warn!(theme_id, file_path = previous.file_path, %error, "清理被替换的主题资源文件失败");
        }
    }
    Ok(saved)
}

pub async fn delete_theme_for_user(
    state: &AppState,
    user_id: i64,
    theme_id: i64,
) -> Result<(), AppError> {
    let resources = state
        .theme_read_application()
        .list_resources(user_id, theme_id)
        .await?;
    state
        .theme_mutation_application()
        .delete_theme(user_id, theme_id)
        .await?;

    let mut first_cleanup_error = None;
    let mut resource_paths = HashSet::new();
    for resource in resources {
        if !resource_paths.insert(resource.file_path.clone()) {
            continue;
        }
        if let Err(error) =
            remove_data_file_in_data_dir(state.data_dir(), &resource.file_path, "主题背景图片")
        {
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
    state
        .theme_mutation_application()
        .activate_theme(user_id, theme_id)
        .await
}
