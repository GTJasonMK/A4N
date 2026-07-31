use a4n_core::{
    config::{
        CreateEmbeddingConfigParams, CreateImageConfigParams, UpdateEmbeddingConfigParams,
        UpdateImageConfigParams, app_config::LlmMaxTokensConfig,
    },
    error::AppError,
    models::{
        ai::{embedding_config, image_gen_config, llm_config, video_gen_config},
        user::theme_config,
    },
};
use a4n_llm::config::{CreateLlmConfigInput, UpdateLlmConfigInput};
use a4n_orchestrator::{
    OrchestratorContext,
    config::{ImportAllConfigsInput, ImportAllResult, SettingsAllHostPort},
    theme::{CreateThemeInput, ImportThemeInput, UpdateThemeInput},
    video::{UpdateVideoConfigParams, VideoConfigParams},
};

/// Config/主题/系统设置领域的 runtime 窄 facade。
///
/// 不提供 context、orchestrator、数据库或完整 owner getter。
pub(crate) struct SettingsRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> SettingsRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn create_theme_for_user_validated(
        &self,
        user_id: i64,
        input: CreateThemeInput,
    ) -> Result<theme_config::Model, AppError> {
        self.orchestrator
            .create_theme_for_user_validated(user_id, input)
            .await
    }

    pub(crate) async fn update_theme_for_user_validated(
        &self,
        user_id: i64,
        theme_id: i64,
        input: UpdateThemeInput,
    ) -> Result<theme_config::Model, AppError> {
        self.orchestrator
            .update_theme_for_user_validated(user_id, theme_id, input)
            .await
    }

    pub(crate) async fn delete_theme_for_user_validated(
        &self,
        user_id: i64,
        theme_id: i64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .delete_theme_for_user_validated(user_id, theme_id)
            .await
    }

    pub(crate) async fn activate_theme_for_user_validated(
        &self,
        user_id: i64,
        theme_id: i64,
    ) -> Result<theme_config::Model, AppError> {
        self.orchestrator
            .activate_theme_for_user_validated(user_id, theme_id)
            .await
    }

    pub(crate) async fn list_themes_for_user_validated(
        &self,
        user_id: i64,
    ) -> Result<Vec<theme_config::Model>, AppError> {
        self.orchestrator
            .list_themes_for_user_validated(user_id)
            .await
    }

    pub(crate) async fn list_user_themes_for_user_validated(
        &self,
        caller_user_id: i64,
        target_user_id: i64,
    ) -> Result<Vec<theme_config::Model>, AppError> {
        self.orchestrator
            .list_user_themes_for_user_validated(caller_user_id, target_user_id)
            .await
    }

    pub(crate) async fn export_theme_for_user_validated(
        &self,
        user_id: i64,
        theme_id: i64,
    ) -> Result<theme_config::Model, AppError> {
        self.orchestrator
            .export_theme_for_user_validated(user_id, theme_id)
            .await
    }

    pub(crate) async fn import_theme_for_user_validated(
        &self,
        user_id: i64,
        input: ImportThemeInput,
    ) -> Result<theme_config::Model, AppError> {
        self.orchestrator
            .import_theme_for_user_validated(user_id, input)
            .await
    }

    pub(crate) async fn theme_css_variables_for_user_validated(
        &self,
        user_id: i64,
        theme_id: i64,
    ) -> Result<String, AppError> {
        self.orchestrator
            .theme_css_variables_for_user_validated(user_id, theme_id)
            .await
    }

    pub(crate) async fn list_llm_config_records(&self) -> Result<Vec<llm_config::Model>, AppError> {
        self.orchestrator.list_llm_config_records().await
    }

    pub(crate) async fn llm_config_record_by_id(
        &self,
        config_id: i64,
    ) -> Result<llm_config::Model, AppError> {
        self.orchestrator.llm_config_record_by_id(config_id).await
    }

    pub(crate) async fn create_llm_config_validated(
        &self,
        input: CreateLlmConfigInput<'_>,
    ) -> Result<llm_config::Model, AppError> {
        self.orchestrator.create_llm_config_validated(input).await
    }

    pub(crate) async fn update_llm_config_validated(
        &self,
        config_id: i64,
        input: UpdateLlmConfigInput<'_>,
    ) -> Result<llm_config::Model, AppError> {
        self.orchestrator
            .update_llm_config_validated(config_id, input)
            .await
    }

    pub(crate) async fn set_default_llm_config_validated(
        &self,
        config_id: i64,
    ) -> Result<llm_config::Model, AppError> {
        self.orchestrator
            .set_default_llm_config_validated(config_id)
            .await
    }

    pub(crate) async fn delete_llm_config_if_allowed(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .delete_llm_config_if_allowed(config_id)
            .await
    }

    pub(crate) async fn resolve_llm_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<llm_config::Model, AppError> {
        self.orchestrator.resolve_llm_config(config_id).await
    }

    pub(crate) async fn llm_max_tokens_config(&self) -> Result<LlmMaxTokensConfig, AppError> {
        self.orchestrator.llm_max_tokens_config().await
    }

    pub(crate) async fn save_llm_max_tokens_config_validated(
        &self,
        config: LlmMaxTokensConfig,
    ) -> Result<LlmMaxTokensConfig, AppError> {
        self.orchestrator
            .save_llm_max_tokens_config_validated(config)
            .await
    }

    pub(crate) async fn list_image_config_records(
        &self,
    ) -> Result<Vec<image_gen_config::Model>, AppError> {
        self.orchestrator.list_image_config_records().await
    }

    pub(crate) async fn image_config_record_by_id(
        &self,
        config_id: i64,
    ) -> Result<image_gen_config::Model, AppError> {
        self.orchestrator.image_config_record_by_id(config_id).await
    }

    pub(crate) async fn create_image_config_validated(
        &self,
        params: CreateImageConfigParams,
    ) -> Result<image_gen_config::Model, AppError> {
        self.orchestrator
            .create_image_config_validated(params)
            .await
    }

    pub(crate) async fn update_image_config_validated(
        &self,
        config_id: i64,
        params: UpdateImageConfigParams,
    ) -> Result<image_gen_config::Model, AppError> {
        self.orchestrator
            .update_image_config_validated(config_id, params)
            .await
    }

    pub(crate) async fn set_default_image_config_validated(
        &self,
        config_id: i64,
    ) -> Result<image_gen_config::Model, AppError> {
        self.orchestrator
            .set_default_image_config_validated(config_id)
            .await
    }

    pub(crate) async fn delete_image_config_if_allowed(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .delete_image_config_if_allowed(config_id)
            .await
    }

    pub(crate) async fn resolve_image_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<image_gen_config::Model, AppError> {
        self.orchestrator.resolve_image_config(config_id).await
    }

    pub(crate) async fn video_config(
        &self,
        config_id: i64,
    ) -> Result<video_gen_config::Model, AppError> {
        self.orchestrator.video_config(config_id).await
    }

    pub(crate) async fn list_video_config_records(
        &self,
    ) -> Result<Vec<video_gen_config::Model>, AppError> {
        self.orchestrator.list_video_config_records().await
    }

    pub(crate) async fn video_config_record_by_id(
        &self,
        config_id: i64,
    ) -> Result<video_gen_config::Model, AppError> {
        self.orchestrator.video_config_record_by_id(config_id).await
    }

    pub(crate) async fn create_video_config_validated(
        &self,
        params: VideoConfigParams,
    ) -> Result<video_gen_config::Model, AppError> {
        self.orchestrator
            .create_video_config_validated(params)
            .await
    }

    pub(crate) async fn update_video_config_validated(
        &self,
        config_id: i64,
        params: UpdateVideoConfigParams,
    ) -> Result<video_gen_config::Model, AppError> {
        self.orchestrator
            .update_video_config_validated(config_id, params)
            .await
    }

    pub(crate) async fn set_default_video_config_validated(
        &self,
        config_id: i64,
    ) -> Result<video_gen_config::Model, AppError> {
        self.orchestrator
            .set_default_video_config_validated(config_id)
            .await
    }

    pub(crate) async fn delete_video_config_if_allowed(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .delete_video_config_if_allowed(config_id)
            .await
    }

    pub(crate) async fn default_video_config(&self) -> Result<video_gen_config::Model, AppError> {
        self.orchestrator.default_video_config().await
    }

    pub(crate) async fn list_embedding_configs(
        &self,
    ) -> Result<Vec<embedding_config::Model>, AppError> {
        self.orchestrator.list_embedding_configs().await
    }

    pub(crate) async fn embedding_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<embedding_config::Model, AppError> {
        self.orchestrator.embedding_config_by_id(config_id).await
    }

    pub(crate) async fn create_embedding_config_validated(
        &self,
        params: CreateEmbeddingConfigParams,
    ) -> Result<embedding_config::Model, AppError> {
        self.orchestrator
            .create_embedding_config_validated(params)
            .await
    }

    pub(crate) async fn update_embedding_config_validated(
        &self,
        config_id: i64,
        patch: UpdateEmbeddingConfigParams,
    ) -> Result<embedding_config::Model, AppError> {
        self.orchestrator
            .update_embedding_config_validated(config_id, patch)
            .await
    }

    pub(crate) async fn set_default_embedding_config_validated(
        &self,
        config_id: i64,
    ) -> Result<embedding_config::Model, AppError> {
        self.orchestrator
            .set_default_embedding_config_validated(config_id)
            .await
    }

    pub(crate) async fn delete_embedding_config_if_allowed(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .delete_embedding_config_if_allowed(config_id)
            .await
    }

    pub(crate) async fn export_all_configs_to_json_validated(
        &self,
        host: &dyn SettingsAllHostPort,
    ) -> Result<String, AppError> {
        self.orchestrator
            .export_all_configs_to_json_validated(host)
            .await
    }

    pub(crate) async fn import_all_configs_validated(
        &self,
        host: &dyn SettingsAllHostPort,
        input: ImportAllConfigsInput,
    ) -> Result<ImportAllResult, AppError> {
        self.orchestrator
            .import_all_configs_validated(host, input)
            .await
    }
}
