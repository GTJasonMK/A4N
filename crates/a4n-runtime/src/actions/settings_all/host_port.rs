use a4n_core::error::AppError;
use a4n_export_core::{ExportDocumentKind, ExportRenderJsonInput};
use a4n_import_core::{ImportDocumentKind, ImportNormalizeJsonInput};
use a4n_orchestrator::config::{PromptRow, SettingsAllHostPort};
use async_trait::async_trait;
use serde_json::Value;
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
                title: Some(prompt.title),
                description: prompt.description,
            })
            .collect())
    }

    async fn render_export_json(&self, payload: Value) -> Result<String, AppError> {
        crate::sidecar_clients::render_export_json(
            self.state.runtime_profile(),
            self.state.capabilities(),
            ExportRenderJsonInput {
                kind: ExportDocumentKind::SettingsAll,
                payload,
                pretty: true,
            },
            None,
            None,
        )
        .await
        .map(|rendered| rendered.content)
        .map_err(AppError::ConfigError)
    }

    async fn normalize_import_json(&self, content: String) -> Result<Value, AppError> {
        crate::sidecar_clients::normalize_import_json(
            self.state.runtime_profile(),
            self.state.capabilities(),
            ImportNormalizeJsonInput {
                kind: ImportDocumentKind::SettingsAll,
                content,
            },
        )
        .await
        .map(|normalized| normalized.normalized)
        .map_err(|error| AppError::InvalidArgument(format!("JSON 解析失败：{error}")))
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

    async fn import_prompts(&self, rows: Vec<PromptRow>) -> Result<usize, AppError> {
        if rows.is_empty() {
            return Ok(0);
        }
        let count = rows.len();
        let updates = rows
            .into_iter()
            .map(|row| (row.name, row.content))
            .collect();
        prompt_update_many_runtime(self.state, updates, "settings_import")
            .await
            .map_err(|error| AppError::InvalidArgument(format!("提示词批量导入失败：{error}")))?;
        Ok(count)
    }

    async fn rollback_prompts(&self, snapshots: Vec<(String, String)>) -> Result<(), AppError> {
        if snapshots.is_empty() {
            return Ok(());
        }
        prompt_update_many_runtime(self.state, snapshots, "settings_import_rollback")
            .await
            .map(|_| ())
            .map_err(|error| AppError::InvalidArgument(error.to_string()))
    }
}
