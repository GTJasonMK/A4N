use super::super::types::{PromptRow, SystemSettingRow, ThemeRow};
use crate::AppState;
use a4n_core::error::AppError;

pub(super) struct UserStateExportRows {
    pub(super) prompts: Vec<PromptRow>,
    pub(super) themes: Vec<ThemeRow>,
    pub(super) settings: Vec<SystemSettingRow>,
}

pub(super) async fn user_state_export_rows(
    state: &AppState,
) -> Result<UserStateExportRows, AppError> {
    let prompts = crate::actions::prompt_records::prompt_list_runtime_entries(state)
        .await
        .map_err(|error| AppError::Internal(anyhow::anyhow!("读取提示词运行时状态失败：{error}")))?
        .into_iter()
        .filter(|prompt| prompt.is_modified)
        .map(|prompt| PromptRow {
            name: prompt.name,
            content: prompt.content,
            title: Some(prompt.title),
            description: prompt.description,
        })
        .collect();
    let themes = state
        .orchestrator
        .settings_all_export_themes()
        .await?
        .into_iter()
        .map(|model| ThemeRow {
            user_id: model.user_id,
            name: model.name,
            mode: model.mode,
            css_variables: model.css_variables,
            is_system_default: model.is_system_default,
            is_active: model.is_active,
        })
        .collect();
    let settings = state
        .orchestrator
        .settings_all_export_system_settings()
        .await?
        .into_iter()
        .map(|model| SystemSettingRow {
            key: model.key,
            value: model.value,
            category: model.category,
            description: model.description,
        })
        .collect();
    Ok(UserStateExportRows {
        prompts,
        themes,
        settings,
    })
}
