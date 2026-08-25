use super::ThemeSummary;
use a4n_api_contract::theme::ThemeDocument;
use a4n_core::error::AppError;
use a4n_core::models::user::{theme_config, theme_resource};
use a4n_domain::ThemeMode;

pub(super) fn resource_to_summary(
    resource: a4n_core::models::user::theme_resource::Model,
) -> a4n_api_contract::theme::ThemeResourceSummary {
    a4n_api_contract::theme::ThemeResourceSummary {
        id: resource.id,
        theme_id: resource.theme_id,
        resource_key: resource.resource_key,
        file_name: resource.file_name,
        mime_type: resource.mime_type,
        file_path: resource.file_path,
        byte_size: resource.byte_size,
    }
}

pub(super) async fn to_summary(
    orchestrator: &a4n_orchestrator::OrchestratorContext,
    user_id: i64,
    theme: theme_config::Model,
) -> Result<ThemeSummary, AppError> {
    let resources = orchestrator
        .list_theme_resources_for_user_validated(user_id, theme.id)
        .await?;
    to_summary_with_resources(theme, resources)
}

pub(super) fn to_summary_with_resources(
    theme: theme_config::Model,
    resources: Vec<theme_resource::Model>,
) -> Result<ThemeSummary, AppError> {
    let mode = theme.mode.parse::<ThemeMode>().map_err(|error| {
        AppError::ConfigError(format!("主题 {} 的持久化 mode 无效：{error}", theme.id))
    })?;
    let document =
        serde_json::from_value::<ThemeDocument>(theme.theme_document.clone()).map_err(|error| {
            AppError::ConfigError(format!(
                "主题 {} 的持久化 theme_document 无效：{error}",
                theme.id
            ))
        })?;
    let resources = resources.into_iter().map(resource_to_summary).collect();
    Ok(ThemeSummary {
        id: theme.id,
        user_id: theme.user_id,
        name: theme.name,
        mode,
        document,
        resources,
        is_system_default: theme.is_system_default,
        is_active: theme.is_active,
    })
}
