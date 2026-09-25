use a4n_api_contract::config::{PromptRow, SettingsAllHostPort};
use a4n_error::AppError;
use async_trait::async_trait;
use tera::Tera;

use crate::{
    AppState,
    actions::prompt_records::{
        prompt_get_runtime_entry, prompt_list_runtime_entries, prompt_update_many_runtime,
    },
};

pub(super) struct RuntimeSettingsAllHostPort<'a> {
    pub(super) state: &'a AppState,
}

#[async_trait]
impl SettingsAllHostPort for RuntimeSettingsAllHostPort<'_> {
    async fn modified_prompts(&self) -> Result<Vec<PromptRow>, AppError> {
        Ok(prompt_list_runtime_entries(self.state)
            .await
            .map_err(|error| {
                AppError::Internal(anyhow::anyhow!("读取提示词运行时状态失败：{error}"))
            })?
            .into_iter()
            .filter(|prompt| prompt.is_modified)
            .map(|prompt| PromptRow {
                name: prompt.name,
                content: prompt.content,
            })
            .collect())
    }

    async fn prepare_prompt_import(
        &self,
        rows: &[PromptRow],
    ) -> Result<Vec<(String, String)>, AppError> {
        let mut snapshots = Vec::with_capacity(rows.len());
        for row in rows {
            let entry = prompt_get_runtime_entry(self.state, &row.name)
                .await
                .map_err(|_| {
                    AppError::InvalidArgument(format!(
                        "提示词 {} 导入失败：注册表中不存在",
                        row.name
                    ))
                })?;
            let mut tera = Tera::default();
            tera.add_raw_template(&row.name, &row.content)
                .map_err(|error| {
                    AppError::InvalidArgument(format!("提示词 {} 模板编译失败：{error}", row.name))
                })?;
            snapshots.push((row.name.clone(), entry.content));
        }
        Ok(snapshots)
    }

    async fn import_prompts(
        &self,
        rows: Vec<PromptRow>,
        overwrite: bool,
    ) -> Result<Vec<String>, AppError> {
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let updates = rows
            .into_iter()
            .map(|row| (row.name, row.content))
            .collect();
        let imported =
            prompt_update_many_runtime(self.state, updates, "settings_import", overwrite)
                .await
                .map_err(|error| {
                    AppError::InvalidArgument(format!("提示词批量导入失败：{error}"))
                })?;
        Ok(imported.into_iter().map(|entry| entry.name).collect())
    }

    async fn rollback_prompts(&self, snapshots: Vec<(String, String)>) -> Result<(), AppError> {
        if snapshots.is_empty() {
            return Ok(());
        }
        prompt_update_many_runtime(self.state, snapshots, "settings_import_rollback", true)
            .await
            .map(|_| ())
            .map_err(|error| AppError::InvalidArgument(error.to_string()))
    }
}
