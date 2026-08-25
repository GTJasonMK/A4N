use a4n_api_contract::config::ImportAllConfigsInput;
use a4n_core::error::AppError;
use a4n_db::config::{SettingsAllConfigImportInput, SettingsAllConfigRows};
use async_trait::async_trait;
use chrono::Utc;

use crate::OrchestratorContext;

use super::settings_export_rules::{
    AllConfigsExport, PromptRow, SETTINGS_ALL_FORMAT_VERSION, import_details_from_counts,
    normalize_all_configs_export, validate_all_configs_export,
};

#[async_trait]
pub trait SettingsAllHostPort: Send + Sync {
    async fn modified_prompts(&self) -> Result<Vec<PromptRow>, AppError>;

    async fn prepare_prompt_import(
        &self,
        rows: &[PromptRow],
    ) -> Result<Vec<(String, String)>, AppError>;

    async fn import_prompts(&self, rows: Vec<PromptRow>) -> Result<usize, AppError>;

    async fn rollback_prompts(&self, snapshots: Vec<(String, String)>) -> Result<(), AppError>;
}

impl OrchestratorContext {
    pub async fn export_all_configs_to_json_validated(
        &self,
        host: &dyn SettingsAllHostPort,
    ) -> Result<String, AppError> {
        let rows = self.settings_all_config_rows().await?;
        serde_json::to_string_pretty(&AllConfigsExport {
            version: SETTINGS_ALL_FORMAT_VERSION.to_string(),
            export_time: Utc::now().to_rfc3339(),
            llm_configs: rows.llm_configs,
            embedding_configs: rows.embedding_configs,
            image_configs: rows.image_configs,
            video_configs: rows.video_configs,
            prompts_modified: host.modified_prompts().await?,
            themes: rows.themes,
            system_settings: rows.system_settings,
        })
        .map_err(AppError::Serialization)
    }

    pub async fn import_all_configs_validated(
        &self,
        host: &dyn SettingsAllHostPort,
        input: ImportAllConfigsInput,
    ) -> Result<Vec<String>, AppError> {
        let mut incoming: AllConfigsExport = serde_json::from_str(&input.settings_json)
            .map_err(|error| AppError::InvalidArgument(format!("JSON 解析失败：{error}")))?;
        normalize_all_configs_export(&mut incoming).map_err(AppError::InvalidArgument)?;
        validate_all_configs_export(&incoming).map_err(AppError::InvalidArgument)?;

        let prompt_snapshots = host
            .prepare_prompt_import(&incoming.prompts_modified)
            .await?;
        let AllConfigsExport {
            llm_configs,
            embedding_configs,
            image_configs,
            video_configs,
            prompts_modified,
            themes,
            system_settings,
            ..
        } = incoming;
        let config_import = SettingsAllConfigImportInput {
            overwrite: input.overwrite,
            rows: SettingsAllConfigRows {
                llm_configs,
                embedding_configs,
                image_configs,
                video_configs,
                themes,
                system_settings,
            },
        };
        self.validate_settings_all_config_import_defaults(&config_import)
            .await?;
        let prompt_imported = host.import_prompts(prompts_modified).await?;

        match self.import_settings_all_configs(config_import).await {
            Ok(counts) => Ok(import_details_from_counts(counts, prompt_imported)),
            Err(error) => {
                let db_error = error.to_string();
                if prompt_snapshots.is_empty() {
                    return Err(AppError::InvalidArgument(db_error));
                }
                match host.rollback_prompts(prompt_snapshots).await {
                    Ok(()) => Err(AppError::InvalidArgument(format!(
                        "{db_error}；已回滚本次提示词导入"
                    ))),
                    Err(rollback_error) => Err(AppError::InvalidArgument(format!(
                        "{db_error}；提示词已更新但回滚失败：{rollback_error}"
                    ))),
                }
            }
        }
    }
}
