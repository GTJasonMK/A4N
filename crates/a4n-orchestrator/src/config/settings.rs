use a4n_core::{
    config::app_config::LlmMaxTokensConfig,
    error::AppError,
    models::{
        ai::{embedding_config, image_gen_config, llm_config, system_settings, video_gen_config},
        user::theme_config,
    },
};

use crate::{
    OrchestratorContext,
    config::{
        BaselineImageConfigSeed, BaselineLlmConfigSeed, BaselineVideoConfigSeed,
        DefaultLlmOutputCapRow, SettingsAllConfigImportCounts, SettingsAllConfigImportInput,
    },
};

impl OrchestratorContext {
    pub async fn llm_configs_exist(&self) -> Result<bool, AppError> {
        a4n_db::config::llm_configs_exist(self.db.connection()).await
    }

    pub async fn image_configs_exist(&self) -> Result<bool, AppError> {
        a4n_db::config::image_configs_exist(self.db.connection()).await
    }

    pub async fn video_configs_exist(&self) -> Result<bool, AppError> {
        a4n_db::config::video_configs_exist(self.db.connection()).await
    }

    pub async fn insert_baseline_embedding_config_if_empty(&self) -> Result<bool, AppError> {
        a4n_db::config::insert_baseline_embedding_config_if_empty(self.db.connection()).await
    }

    pub async fn insert_baseline_llm_config_if_empty(
        &self,
        seed: BaselineLlmConfigSeed,
    ) -> Result<bool, AppError> {
        a4n_db::config::insert_baseline_llm_config_if_empty(self.db.connection(), seed).await
    }

    pub async fn insert_baseline_image_config_if_empty(
        &self,
        seed: BaselineImageConfigSeed,
    ) -> Result<bool, AppError> {
        a4n_db::config::insert_baseline_image_config_if_empty(self.db.connection(), seed).await
    }

    pub async fn insert_baseline_video_config_if_empty(
        &self,
        seed: BaselineVideoConfigSeed,
    ) -> Result<bool, AppError> {
        a4n_db::config::insert_baseline_video_config_if_empty(self.db.connection(), seed).await
    }

    pub async fn settings_all_export_llm_configs(
        &self,
    ) -> Result<Vec<llm_config::Model>, AppError> {
        a4n_db::config::settings_all_export_llm_configs(self.db.connection()).await
    }

    pub async fn settings_all_export_embedding_configs(
        &self,
    ) -> Result<Vec<embedding_config::Model>, AppError> {
        a4n_db::config::settings_all_export_embedding_configs(self.db.connection()).await
    }

    pub async fn settings_all_export_image_configs(
        &self,
    ) -> Result<Vec<image_gen_config::Model>, AppError> {
        a4n_db::config::settings_all_export_image_configs(self.db.connection()).await
    }

    pub async fn settings_all_export_video_configs(
        &self,
    ) -> Result<Vec<video_gen_config::Model>, AppError> {
        a4n_db::config::settings_all_export_video_configs(self.db.connection()).await
    }

    pub async fn settings_all_export_themes(&self) -> Result<Vec<theme_config::Model>, AppError> {
        a4n_db::config::settings_all_export_themes(self.db.connection()).await
    }

    pub async fn settings_all_export_system_settings(
        &self,
    ) -> Result<Vec<system_settings::Model>, AppError> {
        a4n_db::config::settings_all_export_system_settings(self.db.connection()).await
    }

    pub async fn llm_max_tokens_config(&self) -> Result<LlmMaxTokensConfig, AppError> {
        a4n_db::config::llm_max_tokens_config(self.db.connection()).await
    }

    pub async fn save_llm_max_tokens_config(
        &self,
        config: &LlmMaxTokensConfig,
    ) -> Result<(), AppError> {
        a4n_db::config::save_llm_max_tokens_config(self.db.connection(), config).await
    }

    pub async fn default_llm_output_cap_rows(
        &self,
    ) -> Result<Vec<DefaultLlmOutputCapRow>, AppError> {
        a4n_db::config::default_llm_output_cap_rows(self.db.connection()).await
    }

    pub async fn validate_settings_all_config_import_defaults(
        &self,
        input: &SettingsAllConfigImportInput,
    ) -> Result<(), AppError> {
        a4n_db::config::validate_settings_all_config_import_defaults(self.db.connection(), input)
            .await
            .map_err(settings_all_config_import_error)
    }

    pub async fn import_settings_all_configs(
        &self,
        input: SettingsAllConfigImportInput,
    ) -> Result<SettingsAllConfigImportCounts, AppError> {
        a4n_db::config::import_settings_all_configs(self.db.connection(), input)
            .await
            .map_err(settings_all_config_import_error)
    }
}

fn settings_all_config_import_error(
    error: a4n_db::config::SettingsAllConfigImportError,
) -> AppError {
    match error {
        a4n_db::config::SettingsAllConfigImportError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        a4n_db::config::SettingsAllConfigImportError::Database { source, .. } => {
            AppError::Database(source)
        }
    }
}
