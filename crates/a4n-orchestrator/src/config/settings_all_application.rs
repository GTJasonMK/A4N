use a4n_core::error::AppError;
use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;

use crate::OrchestratorContext;

use super::settings_export_rules::{
    AllConfigsExport, EmbeddingRow, ImageRow, ImportAllConfigsInput, ImportAllResult, LlmRow,
    PromptRow, SystemSettingRow, ThemeRow, VideoRow, import_result_from_counts,
    normalize_all_configs_export, settings_all_config_import_input, validate_all_configs_export,
};

#[async_trait]
pub trait SettingsAllHostPort: Send + Sync {
    async fn modified_prompts(&self) -> Result<Vec<PromptRow>, AppError>;

    async fn render_export_json(&self, payload: Value) -> Result<String, AppError>;

    async fn normalize_import_json(&self, content: String) -> Result<Value, AppError>;

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
        let export = AllConfigsExport {
            version: "1.0".to_string(),
            export_time: Utc::now().to_rfc3339(),
            export_type: "all".to_string(),
            llm_configs: self
                .settings_all_export_llm_configs()
                .await?
                .into_iter()
                .map(|model| LlmRow {
                    name: model.name,
                    provider: model.provider,
                    base_url: model.base_url,
                    api_key_encrypted: model.api_key_encrypted,
                    model: model.model,
                    max_concurrent: model.max_concurrent,
                    timeout_secs: model.timeout_secs,
                    max_context_tokens: model.max_context_tokens,
                    max_output_tokens: model.max_output_tokens,
                    request_options_json: model.request_options_json,
                    is_default: model.is_default,
                })
                .collect(),
            embedding_configs: self
                .settings_all_export_embedding_configs()
                .await?
                .into_iter()
                .map(|model| EmbeddingRow {
                    name: model.name,
                    backend: model.backend,
                    base_url: model.base_url,
                    api_key_encrypted: model.api_key_encrypted,
                    model: model.model,
                    dimension: model.dimension,
                    batch_size: model.batch_size,
                    local_model_path: model.local_model_path,
                    is_default: model.is_default,
                })
                .collect(),
            image_configs: self
                .settings_all_export_image_configs()
                .await?
                .into_iter()
                .map(|model| ImageRow {
                    name: model.name,
                    backend: model.backend,
                    base_url: model.base_url,
                    api_key_encrypted: model.api_key_encrypted,
                    model: model.model,
                    max_concurrent: model.max_concurrent,
                    default_width: model.default_width,
                    default_height: model.default_height,
                    provider_options_json: model.provider_options_json,
                    is_default: model.is_default,
                })
                .collect(),
            video_configs: self
                .settings_all_export_video_configs()
                .await?
                .into_iter()
                .map(|model| VideoRow {
                    name: model.name,
                    backend: model.backend,
                    base_url: model.base_url,
                    api_key_encrypted: model.api_key_encrypted,
                    model: model.model,
                    max_concurrent: model.max_concurrent,
                    default_width: model.default_width,
                    default_height: model.default_height,
                    default_duration_secs: model.default_duration_secs,
                    min_duration_secs: model.min_duration_secs,
                    max_duration_secs: model.max_duration_secs,
                    duration_mode: model.duration_mode,
                    supported_durations_json: model.supported_durations_json,
                    default_fps: model.default_fps,
                    provider_options_json: model.provider_options_json,
                    is_default: model.is_default,
                })
                .collect(),
            prompts_modified: host.modified_prompts().await?,
            themes: self
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
                .collect(),
            system_settings: self
                .settings_all_export_system_settings()
                .await?
                .into_iter()
                .map(|model| SystemSettingRow {
                    key: model.key,
                    value: model.value,
                    category: model.category,
                    description: model.description,
                })
                .collect(),
        };
        host.render_export_json(serde_json::to_value(export)?).await
    }

    pub async fn import_all_configs_validated(
        &self,
        host: &dyn SettingsAllHostPort,
        input: ImportAllConfigsInput,
    ) -> Result<ImportAllResult, AppError> {
        let normalized = host.normalize_import_json(input.settings_json).await?;
        let mut incoming: AllConfigsExport = serde_json::from_value(normalized)
            .map_err(|error| AppError::InvalidArgument(format!("JSON 解析失败：{error}")))?;
        validate_all_configs_export(&incoming).map_err(AppError::InvalidArgument)?;
        normalize_all_configs_export(&mut incoming).map_err(AppError::InvalidArgument)?;

        let prompt_snapshots = host
            .prepare_prompt_import(&incoming.prompts_modified)
            .await?;
        let config_import = settings_all_config_import_input(&incoming, input.overwrite);
        self.validate_settings_all_config_import_defaults(&config_import)
            .await?;
        let prompt_rows = std::mem::take(&mut incoming.prompts_modified);
        let prompt_imported = host.import_prompts(prompt_rows).await?;

        match self.import_settings_all_configs(config_import).await {
            Ok(counts) => Ok(import_result_from_counts(counts, prompt_imported)),
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
