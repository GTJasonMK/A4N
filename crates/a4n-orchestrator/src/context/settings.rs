use super::OrchestratorContext;
use crate::video::{VideoConfigParams, validate_video_config_model, validate_video_config_params};
use a4n_api_contract::config::{
    CreateEmbeddingInput, CreateLlmInput, ImageConfigInput, UpdateEmbeddingInput,
    UpdateImageConfigInput, UpdateLlmInput, UpdateVideoConfigInput, VideoConfigInput,
};
use a4n_core::{
    config::{
        CreateEmbeddingConfigParams, CreateImageConfigParams, app_config::LlmMaxTokensConfig,
        ensure_default_config_delete_allowed, validate_embedding_config_model,
        validate_embedding_config_params, validate_image_config_model,
        validate_image_config_params,
    },
    error::AppError,
    models::{
        ai::{embedding_config, image_gen_config, llm_config, video_gen_config},
        user::theme_config,
    },
};
use a4n_db::{
    DbHandle,
    config::{
        EmbeddingConfigRecordInput, ImageConfigRecordInput, LlmConfigRecordInput,
        ProviderConfigKind, SettingsAllConfigImportCounts, SettingsAllConfigImportInput,
        SettingsAllConfigRows, VideoConfigRecordInput,
    },
};
use a4n_domain::{ThemeDocument, ThemeMode};
use a4n_llm::config::{
    CreateLlmConfigInput, NormalizedLlmConfig, UpdateLlmConfigInput,
    normalize_create_llm_config_input, normalize_update_llm_config_input,
    validate_llm_config_model,
};

/// AI 配置、主题与系统设置的私有 application owner。
///
/// 与 `OrchestratorContext.config`（AppConfig 基础设施）区分：本 owner 只持有数据库句柄，
/// 负责配置记录的读写，不暴露完整连接或通用 getter。
#[derive(Clone)]
pub(super) struct ConfigApplication {
    database: DbHandle,
}

impl ConfigApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn set_default_provider_config(
        &self,
        kind: ProviderConfigKind,
        config_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::config::set_default_provider_config(self.database.connection(), kind, config_id)
            .await
    }

    async fn list_themes_visible_to_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<theme_config::Model>, AppError> {
        a4n_db::config::list_themes_visible_to_user(self.database.connection(), user_id).await
    }

    async fn theme_by_id(&self, theme_id: i64) -> Result<Option<theme_config::Model>, AppError> {
        a4n_db::config::theme_by_id(self.database.connection(), theme_id).await
    }

    async fn create_theme_record(
        &self,
        user_id: Option<i64>,
        name: &str,
        mode: ThemeMode,
        document: ThemeDocument,
    ) -> Result<theme_config::Model, AppError> {
        a4n_db::config::create_theme(self.database.connection(), user_id, name, mode, document)
            .await
    }

    async fn update_theme_record(
        &self,
        theme: theme_config::Model,
        name: Option<&str>,
        mode: Option<ThemeMode>,
        document: Option<ThemeDocument>,
    ) -> Result<theme_config::Model, AppError> {
        a4n_db::config::update_theme(self.database.connection(), theme, name, mode, document).await
    }

    async fn delete_theme_record(&self, theme_id: i64) -> Result<(), AppError> {
        a4n_db::config::delete_theme(self.database.connection(), theme_id).await
    }

    async fn activate_theme(&self, theme_id: i64) -> Result<theme_config::Model, AppError> {
        a4n_db::config::activate_theme(self.database.connection(), theme_id).await
    }

    async fn list_theme_resources(
        &self,
        theme_id: i64,
    ) -> Result<Vec<a4n_core::models::user::theme_resource::Model>, AppError> {
        a4n_db::config::list_theme_resources(self.database.connection(), theme_id).await
    }

    async fn list_theme_resources_for_themes(
        &self,
        theme_ids: &[i64],
    ) -> Result<Vec<a4n_core::models::user::theme_resource::Model>, AppError> {
        a4n_db::config::list_theme_resources_for_themes(self.database.connection(), theme_ids).await
    }

    async fn theme_resource_by_id(
        &self,
        resource_id: i64,
    ) -> Result<Option<a4n_core::models::user::theme_resource::Model>, AppError> {
        a4n_db::config::theme_resource_by_id(self.database.connection(), resource_id).await
    }

    async fn upsert_theme_resource(
        &self,
        theme_id: i64,
        resource_key: &str,
        file_name: &str,
        mime_type: &str,
        file_path: &str,
        sha256: &str,
        byte_size: i64,
    ) -> Result<a4n_core::models::user::theme_resource::Model, AppError> {
        a4n_db::config::upsert_theme_resource(
            self.database.connection(),
            theme_id,
            resource_key,
            file_name,
            mime_type,
            file_path,
            sha256,
            byte_size,
        )
        .await
    }

    async fn delete_theme_resource(
        &self,
        resource_id: i64,
    ) -> Result<a4n_core::models::user::theme_resource::Model, AppError> {
        a4n_db::config::delete_theme_resource(self.database.connection(), resource_id).await
    }

    async fn theme_preference(
        &self,
        user_id: i64,
    ) -> Result<Option<a4n_core::models::user::theme_preference::Model>, AppError> {
        a4n_db::config::theme_preference(self.database.connection(), user_id).await
    }

    async fn upsert_theme_preference(
        &self,
        user_id: i64,
        appearance: a4n_domain::ThemeAppearance,
    ) -> Result<a4n_core::models::user::theme_preference::Model, AppError> {
        a4n_db::config::upsert_theme_preference(self.database.connection(), user_id, appearance)
            .await
    }

    async fn list_llm_config_records(&self) -> Result<Vec<llm_config::Model>, AppError> {
        a4n_db::config::list_llm_config_records(self.database.connection()).await
    }

    async fn llm_config_by_id(&self, config_id: i64) -> Result<llm_config::Model, AppError> {
        a4n_db::config::llm_config_by_id(self.database.connection(), config_id).await
    }

    /// 规范化 LLM 创建输入后落库。
    async fn create_llm_config_validated(
        &self,
        input: CreateLlmInput,
    ) -> Result<llm_config::Model, AppError> {
        let normalized = normalize_create_llm_config_input(CreateLlmConfigInput {
            name: &input.name,
            provider: &input.provider,
            base_url: &input.base_url,
            api_key_encrypted: input.api_key_encrypted.as_deref(),
            model: &input.model,
            max_concurrent: input.max_concurrent,
            timeout_secs: input.timeout_secs,
            max_context_tokens: input.max_context_tokens,
            max_output_tokens: input.max_output_tokens,
            extra_body_json: input.extra_body_json,
        })?;
        a4n_db::config::create_llm_config_record(
            self.database.connection(),
            llm_record_input(normalized),
        )
        .await
    }

    /// 读库、规范化更新、默认配置与全局 max_tokens 门禁后落库。
    async fn update_llm_config_validated(
        &self,
        config_id: i64,
        input: UpdateLlmInput,
    ) -> Result<llm_config::Model, AppError> {
        let existing = self.llm_config_by_id(config_id).await?;
        let normalized = normalize_update_llm_config_input(
            &existing,
            UpdateLlmConfigInput {
                name: &input.name,
                provider: &input.provider,
                base_url: &input.base_url,
                api_key_encrypted: input
                    .api_key_encrypted
                    .as_ref()
                    .map(|value| value.as_deref()),
                model: &input.model,
                max_concurrent: input.max_concurrent,
                timeout_secs: input.timeout_secs,
                max_context_tokens: input.max_context_tokens,
                max_output_tokens: input.max_output_tokens,
                extra_body_json: input.extra_body_json,
            },
        )?;
        if existing.is_default {
            self.ensure_llm_max_tokens_fits_model_output(normalized.max_output_tokens)
                .await?;
        }
        a4n_db::config::update_llm_config_record(
            self.database.connection(),
            existing,
            llm_record_input(normalized),
        )
        .await
    }

    /// 模型校验与 max_tokens 门禁后设为默认 LLM 配置。
    async fn set_default_llm_config_validated(&self, config_id: i64) -> Result<(), AppError> {
        let target = self.llm_config_by_id(config_id).await?;
        validate_llm_config_model(&target)?;
        self.ensure_llm_max_tokens_fits_model_output(target.max_output_tokens)
            .await?;
        self.set_default_provider_config(ProviderConfigKind::Llm, config_id)
            .await
    }

    /// 非默认门禁后删除 LLM 配置。
    async fn delete_llm_config_if_allowed(&self, config_id: i64) -> Result<(), AppError> {
        let target = self.llm_config_by_id(config_id).await?;
        ensure_default_config_delete_allowed(" LLM 配置", target.is_default)?;
        a4n_db::config::delete_llm_config_record(self.database.connection(), config_id).await
    }

    async fn ensure_llm_max_tokens_fits_model_output(
        &self,
        max_output_tokens: i32,
    ) -> Result<(), AppError> {
        let model_cap = positive_model_output_cap(max_output_tokens)?;
        let max_tokens = self.llm_max_tokens_config().await?;
        max_tokens
            .validate_against_model_output_cap(model_cap)
            .map_err(AppError::InvalidArgument)
    }

    pub(super) async fn resolve_llm_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<llm_config::Model, AppError> {
        a4n_db::config::resolve_llm_config(self.database.connection(), config_id).await
    }

    async fn list_image_config_records(&self) -> Result<Vec<image_gen_config::Model>, AppError> {
        a4n_db::config::list_image_config_records(self.database.connection()).await
    }

    async fn image_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<image_gen_config::Model, AppError> {
        a4n_db::config::image_config_by_id(self.database.connection(), config_id).await
    }

    /// 校验图片配置参数后创建。
    async fn create_image_config_validated(
        &self,
        input: ImageConfigInput,
    ) -> Result<image_gen_config::Model, AppError> {
        let params = validate_image_config_params(CreateImageConfigParams {
            name: input.name,
            backend: input.backend,
            base_url: input.base_url,
            api_key_encrypted: input.api_key_encrypted,
            model: input.model,
            max_concurrent: input.max_concurrent,
            default_width: input.default_width,
            default_height: input.default_height,
        })?;
        a4n_db::config::create_image_config_record(
            self.database.connection(),
            image_record_input(params),
        )
        .await
    }

    /// 按 id 读库、合并 api_key、校验后更新图片配置。
    async fn update_image_config_validated(
        &self,
        config_id: i64,
        input: UpdateImageConfigInput,
    ) -> Result<image_gen_config::Model, AppError> {
        let existing = self.image_config_by_id(config_id).await?;
        let api_key_encrypted =
            merge_api_key_update(input.api_key_encrypted, existing.api_key_encrypted.clone());
        let params = validate_image_config_params(CreateImageConfigParams {
            name: input.name,
            backend: input.backend,
            base_url: input.base_url,
            api_key_encrypted,
            model: input.model,
            max_concurrent: input.max_concurrent,
            default_width: input.default_width,
            default_height: input.default_height,
        })?;
        a4n_db::config::update_image_config_record(
            self.database.connection(),
            existing,
            image_record_input(params),
        )
        .await
    }

    /// 校验模型可用后设为默认图片配置。
    async fn set_default_image_config_validated(&self, config_id: i64) -> Result<(), AppError> {
        let target = self.image_config_by_id(config_id).await?;
        validate_image_config_model(&target)?;
        self.set_default_provider_config(ProviderConfigKind::Image, config_id)
            .await
    }

    /// 非默认门禁通过后删除图片配置。
    async fn delete_image_config_if_allowed(&self, config_id: i64) -> Result<(), AppError> {
        let target = self.image_config_by_id(config_id).await?;
        ensure_default_config_delete_allowed("图片配置", target.is_default)?;
        a4n_db::config::delete_image_config_record(self.database.connection(), config_id).await
    }

    async fn resolve_image_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<image_gen_config::Model, AppError> {
        a4n_db::config::resolve_image_config(self.database.connection(), config_id).await
    }

    async fn video_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<video_gen_config::Model, AppError> {
        a4n_db::config::video_config_by_id(self.database.connection(), config_id).await
    }

    async fn list_video_config_records(&self) -> Result<Vec<video_gen_config::Model>, AppError> {
        a4n_db::config::list_video_config_records(self.database.connection()).await
    }

    async fn default_video_config(&self) -> Result<video_gen_config::Model, AppError> {
        a4n_db::config::default_video_config(self.database.connection()).await
    }

    async fn active_generated_video_ids_for_config(
        &self,
        config_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::config::active_generated_video_ids_for_config(self.database.connection(), config_id)
            .await
    }

    /// 校验视频配置参数后创建。
    async fn create_video_config_validated(
        &self,
        input: VideoConfigInput,
    ) -> Result<video_gen_config::Model, AppError> {
        let params = validate_video_config_params(VideoConfigParams {
            name: input.name,
            backend: input.backend,
            base_url: input.base_url,
            api_key_encrypted: input.api_key_encrypted,
            model: input.model,
            max_concurrent: input.max_concurrent,
            default_width: input.default_width,
            default_height: input.default_height,
            default_duration_secs: input.default_duration_secs,
            min_duration_secs: input.min_duration_secs,
            max_duration_secs: input.max_duration_secs,
            duration_mode: input.duration_mode.to_string(),
            supported_durations_json: input.supported_durations_json,
            default_fps: input.default_fps,
        })
        .map_err(AppError::InvalidArgument)?;
        a4n_db::config::create_video_config_record(
            self.database.connection(),
            video_record_input(params),
        )
        .await
    }

    /// 按 id 读库、合并 api_key、校验后更新视频配置。
    async fn update_video_config_validated(
        &self,
        config_id: i64,
        input: UpdateVideoConfigInput,
    ) -> Result<video_gen_config::Model, AppError> {
        let existing = self.video_config_by_id(config_id).await?;
        let api_key_encrypted =
            merge_api_key_update(input.api_key_encrypted, existing.api_key_encrypted.clone());
        let params = validate_video_config_params(VideoConfigParams {
            name: input.name,
            backend: input.backend,
            base_url: input.base_url,
            api_key_encrypted,
            model: input.model,
            max_concurrent: input.max_concurrent,
            default_width: input.default_width,
            default_height: input.default_height,
            default_duration_secs: input.default_duration_secs,
            min_duration_secs: input.min_duration_secs,
            max_duration_secs: input.max_duration_secs,
            duration_mode: input.duration_mode.to_string(),
            supported_durations_json: input.supported_durations_json,
            default_fps: input.default_fps,
        })
        .map_err(AppError::InvalidArgument)?;
        a4n_db::config::update_video_config_record(
            self.database.connection(),
            existing,
            video_record_input(params),
        )
        .await
    }

    /// 校验模型可用后设为默认视频配置。
    async fn set_default_video_config_validated(&self, config_id: i64) -> Result<(), AppError> {
        let target = self.video_config_by_id(config_id).await?;
        validate_video_config_model(&target).map_err(AppError::InvalidArgument)?;
        self.set_default_provider_config(ProviderConfigKind::Video, config_id)
            .await
    }

    /// 非默认且无进行中任务时删除视频配置。
    async fn delete_video_config_if_allowed(&self, config_id: i64) -> Result<(), AppError> {
        let target = self.video_config_by_id(config_id).await?;
        ensure_default_config_delete_allowed("视频配置", target.is_default)?;
        let active_video_ids = self
            .active_generated_video_ids_for_config(config_id)
            .await?;
        if !active_video_ids.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "视频配置 #{config_id} 仍有 {} 个进行中的视频任务，不能删除",
                active_video_ids.len()
            )));
        }
        a4n_db::config::delete_video_config_record(self.database.connection(), config_id).await
    }

    pub(super) async fn default_embedding_config_optional(
        &self,
    ) -> Result<Option<embedding_config::Model>, AppError> {
        a4n_db::config::default_embedding_config_optional(self.database.connection()).await
    }

    pub(super) async fn default_embedding_config(
        &self,
    ) -> Result<embedding_config::Model, AppError> {
        a4n_db::config::default_embedding_config(self.database.connection()).await
    }

    async fn list_embedding_configs(&self) -> Result<Vec<embedding_config::Model>, AppError> {
        a4n_db::config::list_embedding_configs(self.database.connection()).await
    }

    async fn embedding_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<embedding_config::Model, AppError> {
        a4n_db::config::embedding_config_by_id(self.database.connection(), config_id).await
    }

    /// 校验嵌入配置参数后创建。
    async fn create_embedding_config_validated(
        &self,
        input: CreateEmbeddingInput,
    ) -> Result<embedding_config::Model, AppError> {
        let params = validate_embedding_config_params(CreateEmbeddingConfigParams {
            name: input.name,
            backend: input.backend,
            base_url: input.base_url,
            api_key_encrypted: input.api_key_encrypted,
            model: input.model,
            dimension: input.dimension,
            batch_size: input.batch_size,
            local_model_path: input.local_model_path,
        })?;
        a4n_db::config::create_embedding_config_record(
            self.database.connection(),
            embedding_record_input(params),
        )
        .await
    }

    /// 读库、保留未提交的 API Key、校验后更新嵌入配置。
    async fn update_embedding_config_validated(
        &self,
        config_id: i64,
        input: UpdateEmbeddingInput,
    ) -> Result<embedding_config::Model, AppError> {
        let existing = self.embedding_config_by_id(config_id).await?;
        let requested_api_key =
            merge_api_key_update(input.api_key_encrypted, existing.api_key_encrypted.clone());
        let params = validate_embedding_config_params(CreateEmbeddingConfigParams {
            name: input.name,
            backend: input.backend,
            base_url: input.base_url,
            api_key_encrypted: requested_api_key,
            model: input.model,
            dimension: input.dimension,
            batch_size: input.batch_size,
            local_model_path: input.local_model_path,
        })?;
        a4n_db::config::update_embedding_config_record(
            self.database.connection(),
            existing,
            embedding_record_input(params),
        )
        .await
    }

    /// 模型校验后设为默认嵌入配置。
    async fn set_default_embedding_config_validated(&self, config_id: i64) -> Result<(), AppError> {
        let target = self.embedding_config_by_id(config_id).await?;
        validate_embedding_config_model(&target)?;
        self.set_default_provider_config(ProviderConfigKind::Embedding, config_id)
            .await
    }

    /// 非默认门禁后删除嵌入配置。
    async fn delete_embedding_config_if_allowed(&self, config_id: i64) -> Result<(), AppError> {
        let target = self.embedding_config_by_id(config_id).await?;
        ensure_default_config_delete_allowed(" Embedding 配置", target.is_default)?;
        a4n_db::config::delete_embedding_config_record(self.database.connection(), config_id).await
    }

    async fn llm_configs_exist(&self) -> Result<bool, AppError> {
        a4n_db::config::llm_configs_exist(self.database.connection()).await
    }

    async fn image_configs_exist(&self) -> Result<bool, AppError> {
        a4n_db::config::image_configs_exist(self.database.connection()).await
    }

    async fn video_configs_exist(&self) -> Result<bool, AppError> {
        a4n_db::config::video_configs_exist(self.database.connection()).await
    }

    async fn insert_baseline_embedding_config_if_empty(&self) -> Result<bool, AppError> {
        a4n_db::config::insert_baseline_embedding_config_if_empty(self.database.connection()).await
    }

    async fn insert_baseline_llm_config_if_empty(
        &self,
        config: NormalizedLlmConfig,
    ) -> Result<bool, AppError> {
        a4n_db::config::insert_baseline_llm_config_if_empty(
            self.database.connection(),
            llm_record_input(config),
        )
        .await
    }

    async fn insert_baseline_image_config_if_empty(
        &self,
        config: CreateImageConfigParams,
    ) -> Result<bool, AppError> {
        a4n_db::config::insert_baseline_image_config_if_empty(
            self.database.connection(),
            image_record_input(config),
        )
        .await
    }

    async fn insert_baseline_video_config_if_empty(
        &self,
        config: VideoConfigParams,
    ) -> Result<bool, AppError> {
        a4n_db::config::insert_baseline_video_config_if_empty(
            self.database.connection(),
            video_record_input(config),
        )
        .await
    }

    async fn settings_all_config_rows(&self) -> Result<SettingsAllConfigRows, AppError> {
        a4n_db::config::settings_all_config_rows(self.database.connection()).await
    }

    pub(super) async fn llm_max_tokens_config(&self) -> Result<LlmMaxTokensConfig, AppError> {
        a4n_db::config::llm_max_tokens_config(self.database.connection()).await
    }

    /// 校验全局 max_tokens 预算与默认模型输出上限后落库。
    async fn save_llm_max_tokens_config_validated(
        &self,
        config: LlmMaxTokensConfig,
    ) -> Result<LlmMaxTokensConfig, AppError> {
        config.validate().map_err(AppError::InvalidArgument)?;
        for model in a4n_db::config::default_llm_output_cap_rows(self.database.connection()).await?
        {
            let model_cap = positive_model_output_cap(model.max_output_tokens)?;
            config
                .validate_against_model_output_cap(model_cap)
                .map_err(|err| {
                    AppError::InvalidArgument(format!(
                        "默认 LLM 配置 #{} {}：{err}",
                        model.id, model.name
                    ))
                })?;
        }
        a4n_db::config::save_llm_max_tokens_config(self.database.connection(), &config).await?;
        Ok(config)
    }

    async fn validate_settings_all_config_import_defaults(
        &self,
        input: &SettingsAllConfigImportInput,
    ) -> Result<(), AppError> {
        a4n_db::config::validate_settings_all_config_import_defaults(
            self.database.connection(),
            input,
        )
        .await
        .map_err(settings_all_config_import_error)
    }

    async fn import_settings_all_configs(
        &self,
        input: SettingsAllConfigImportInput,
    ) -> Result<SettingsAllConfigImportCounts, AppError> {
        a4n_db::config::import_settings_all_configs(self.database.connection(), input)
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

impl OrchestratorContext {
    pub async fn list_themes_visible_to_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<theme_config::Model>, AppError> {
        self.settings.list_themes_visible_to_user(user_id).await
    }

    pub async fn theme_by_id(
        &self,
        theme_id: i64,
    ) -> Result<Option<theme_config::Model>, AppError> {
        self.settings.theme_by_id(theme_id).await
    }

    pub async fn create_user_theme_record(
        &self,
        user_id: i64,
        name: String,
        mode: ThemeMode,
        document: ThemeDocument,
    ) -> Result<theme_config::Model, AppError> {
        self.settings
            .create_theme_record(Some(user_id), &name, mode, document)
            .await
    }

    pub async fn update_theme_record(
        &self,
        theme: theme_config::Model,
        name: Option<String>,
        mode: Option<ThemeMode>,
        document: Option<ThemeDocument>,
    ) -> Result<theme_config::Model, AppError> {
        self.settings
            .update_theme_record(theme, name.as_deref(), mode, document)
            .await
    }

    pub async fn delete_theme_record(&self, theme_id: i64) -> Result<(), AppError> {
        self.settings.delete_theme_record(theme_id).await
    }

    pub async fn activate_theme(&self, theme_id: i64) -> Result<theme_config::Model, AppError> {
        self.settings.activate_theme(theme_id).await
    }

    pub async fn list_theme_resources(
        &self,
        theme_id: i64,
    ) -> Result<Vec<a4n_core::models::user::theme_resource::Model>, AppError> {
        self.settings.list_theme_resources(theme_id).await
    }

    pub async fn list_theme_resources_for_themes(
        &self,
        theme_ids: &[i64],
    ) -> Result<Vec<a4n_core::models::user::theme_resource::Model>, AppError> {
        self.settings
            .list_theme_resources_for_themes(theme_ids)
            .await
    }

    pub async fn theme_resource_by_id(
        &self,
        resource_id: i64,
    ) -> Result<Option<a4n_core::models::user::theme_resource::Model>, AppError> {
        self.settings.theme_resource_by_id(resource_id).await
    }

    pub async fn upsert_theme_resource(
        &self,
        theme_id: i64,
        resource_key: &str,
        file_name: &str,
        mime_type: &str,
        file_path: &str,
        sha256: &str,
        byte_size: i64,
    ) -> Result<a4n_core::models::user::theme_resource::Model, AppError> {
        self.settings
            .upsert_theme_resource(
                theme_id,
                resource_key,
                file_name,
                mime_type,
                file_path,
                sha256,
                byte_size,
            )
            .await
    }

    pub async fn delete_theme_resource(
        &self,
        resource_id: i64,
    ) -> Result<a4n_core::models::user::theme_resource::Model, AppError> {
        self.settings.delete_theme_resource(resource_id).await
    }

    pub async fn theme_preference(
        &self,
        user_id: i64,
    ) -> Result<Option<a4n_core::models::user::theme_preference::Model>, AppError> {
        self.settings.theme_preference(user_id).await
    }

    pub async fn upsert_theme_preference(
        &self,
        user_id: i64,
        appearance: a4n_domain::ThemeAppearance,
    ) -> Result<a4n_core::models::user::theme_preference::Model, AppError> {
        self.settings
            .upsert_theme_preference(user_id, appearance)
            .await
    }

    pub async fn list_llm_config_records(&self) -> Result<Vec<llm_config::Model>, AppError> {
        self.settings.list_llm_config_records().await
    }

    pub async fn llm_config_by_id(&self, config_id: i64) -> Result<llm_config::Model, AppError> {
        self.settings.llm_config_by_id(config_id).await
    }

    /// 创建 LLM 配置：规范化后落库。
    pub async fn create_llm_config_validated(
        &self,
        input: CreateLlmInput,
    ) -> Result<llm_config::Model, AppError> {
        self.settings.create_llm_config_validated(input).await
    }

    /// 更新 LLM 配置：规范化、默认 max_tokens 门禁后落库。
    pub async fn update_llm_config_validated(
        &self,
        config_id: i64,
        input: UpdateLlmInput,
    ) -> Result<llm_config::Model, AppError> {
        self.settings
            .update_llm_config_validated(config_id, input)
            .await
    }

    /// 设为默认 LLM 配置：模型与 max_tokens 门禁后落库。
    pub async fn set_default_llm_config_validated(&self, config_id: i64) -> Result<(), AppError> {
        self.settings
            .set_default_llm_config_validated(config_id)
            .await
    }

    /// 删除 LLM 配置：非默认门禁后落库。
    pub async fn delete_llm_config_if_allowed(&self, config_id: i64) -> Result<(), AppError> {
        self.settings.delete_llm_config_if_allowed(config_id).await
    }

    pub async fn resolve_llm_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<llm_config::Model, AppError> {
        self.settings.resolve_llm_config(config_id).await
    }

    pub async fn list_image_config_records(
        &self,
    ) -> Result<Vec<image_gen_config::Model>, AppError> {
        self.settings.list_image_config_records().await
    }

    pub async fn image_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<image_gen_config::Model, AppError> {
        self.settings.image_config_by_id(config_id).await
    }

    /// 创建图片配置：校验参数后落库。
    pub async fn create_image_config_validated(
        &self,
        input: ImageConfigInput,
    ) -> Result<image_gen_config::Model, AppError> {
        self.settings.create_image_config_validated(input).await
    }

    /// 更新图片配置：读库、合并 api_key、校验后落库。
    pub async fn update_image_config_validated(
        &self,
        config_id: i64,
        input: UpdateImageConfigInput,
    ) -> Result<image_gen_config::Model, AppError> {
        self.settings
            .update_image_config_validated(config_id, input)
            .await
    }

    /// 设为默认图片配置：模型校验后落库。
    pub async fn set_default_image_config_validated(&self, config_id: i64) -> Result<(), AppError> {
        self.settings
            .set_default_image_config_validated(config_id)
            .await
    }

    /// 删除图片配置：非默认门禁后落库。
    pub async fn delete_image_config_if_allowed(&self, config_id: i64) -> Result<(), AppError> {
        self.settings
            .delete_image_config_if_allowed(config_id)
            .await
    }

    pub async fn resolve_image_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<image_gen_config::Model, AppError> {
        self.settings.resolve_image_config(config_id).await
    }

    pub async fn video_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<video_gen_config::Model, AppError> {
        self.settings.video_config_by_id(config_id).await
    }

    pub async fn list_video_config_records(
        &self,
    ) -> Result<Vec<video_gen_config::Model>, AppError> {
        self.settings.list_video_config_records().await
    }

    /// 创建视频配置：校验参数后落库。
    pub async fn create_video_config_validated(
        &self,
        input: VideoConfigInput,
    ) -> Result<video_gen_config::Model, AppError> {
        self.settings.create_video_config_validated(input).await
    }

    /// 更新视频配置：读库、合并 api_key、校验后落库。
    pub async fn update_video_config_validated(
        &self,
        config_id: i64,
        input: UpdateVideoConfigInput,
    ) -> Result<video_gen_config::Model, AppError> {
        self.settings
            .update_video_config_validated(config_id, input)
            .await
    }

    /// 设为默认视频配置：模型校验后落库。
    pub async fn set_default_video_config_validated(&self, config_id: i64) -> Result<(), AppError> {
        self.settings
            .set_default_video_config_validated(config_id)
            .await
    }

    /// 删除视频配置：非默认且无进行中任务门禁后落库。
    pub async fn delete_video_config_if_allowed(&self, config_id: i64) -> Result<(), AppError> {
        self.settings
            .delete_video_config_if_allowed(config_id)
            .await
    }

    pub async fn default_video_config(&self) -> Result<video_gen_config::Model, AppError> {
        self.settings.default_video_config().await
    }

    pub async fn active_generated_video_ids_for_config(
        &self,
        config_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        self.settings
            .active_generated_video_ids_for_config(config_id)
            .await
    }

    pub async fn default_embedding_config_optional(
        &self,
    ) -> Result<Option<embedding_config::Model>, AppError> {
        self.settings.default_embedding_config_optional().await
    }

    pub async fn default_embedding_config(&self) -> Result<embedding_config::Model, AppError> {
        self.settings.default_embedding_config().await
    }

    pub async fn list_embedding_configs(&self) -> Result<Vec<embedding_config::Model>, AppError> {
        self.settings.list_embedding_configs().await
    }

    pub async fn embedding_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<embedding_config::Model, AppError> {
        self.settings.embedding_config_by_id(config_id).await
    }

    /// 创建嵌入配置：校验参数后落库。
    pub async fn create_embedding_config_validated(
        &self,
        input: CreateEmbeddingInput,
    ) -> Result<embedding_config::Model, AppError> {
        self.settings.create_embedding_config_validated(input).await
    }

    /// 更新嵌入配置：完整替换字段并保留未提交的 API Key。
    pub async fn update_embedding_config_validated(
        &self,
        config_id: i64,
        input: UpdateEmbeddingInput,
    ) -> Result<embedding_config::Model, AppError> {
        self.settings
            .update_embedding_config_validated(config_id, input)
            .await
    }

    /// 设为默认嵌入配置：模型校验后落库。
    pub async fn set_default_embedding_config_validated(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.settings
            .set_default_embedding_config_validated(config_id)
            .await
    }

    /// 删除嵌入配置：非默认门禁后落库。
    pub async fn delete_embedding_config_if_allowed(&self, config_id: i64) -> Result<(), AppError> {
        self.settings
            .delete_embedding_config_if_allowed(config_id)
            .await
    }

    pub async fn llm_configs_exist(&self) -> Result<bool, AppError> {
        self.settings.llm_configs_exist().await
    }

    pub async fn image_configs_exist(&self) -> Result<bool, AppError> {
        self.settings.image_configs_exist().await
    }

    pub async fn video_configs_exist(&self) -> Result<bool, AppError> {
        self.settings.video_configs_exist().await
    }

    pub async fn insert_baseline_embedding_config_if_empty(&self) -> Result<bool, AppError> {
        self.settings
            .insert_baseline_embedding_config_if_empty()
            .await
    }

    pub async fn insert_baseline_llm_config_if_empty(
        &self,
        config: NormalizedLlmConfig,
    ) -> Result<bool, AppError> {
        self.settings
            .insert_baseline_llm_config_if_empty(config)
            .await
    }

    pub async fn insert_baseline_image_config_if_empty(
        &self,
        config: CreateImageConfigParams,
    ) -> Result<bool, AppError> {
        self.settings
            .insert_baseline_image_config_if_empty(config)
            .await
    }

    pub async fn insert_baseline_video_config_if_empty(
        &self,
        config: VideoConfigParams,
    ) -> Result<bool, AppError> {
        self.settings
            .insert_baseline_video_config_if_empty(config)
            .await
    }

    pub(crate) async fn settings_all_config_rows(&self) -> Result<SettingsAllConfigRows, AppError> {
        self.settings.settings_all_config_rows().await
    }

    pub async fn llm_max_tokens_config(&self) -> Result<LlmMaxTokensConfig, AppError> {
        self.settings.llm_max_tokens_config().await
    }

    /// 保存 LLM max_tokens 预算：校验与默认模型上限后落库。
    pub async fn save_llm_max_tokens_config_validated(
        &self,
        config: LlmMaxTokensConfig,
    ) -> Result<LlmMaxTokensConfig, AppError> {
        self.settings
            .save_llm_max_tokens_config_validated(config)
            .await
    }

    pub(crate) async fn validate_settings_all_config_import_defaults(
        &self,
        input: &SettingsAllConfigImportInput,
    ) -> Result<(), AppError> {
        self.settings
            .validate_settings_all_config_import_defaults(input)
            .await
    }

    pub(crate) async fn import_settings_all_configs(
        &self,
        input: SettingsAllConfigImportInput,
    ) -> Result<SettingsAllConfigImportCounts, AppError> {
        self.settings.import_settings_all_configs(input).await
    }
}

fn positive_model_output_cap(value: i32) -> Result<usize, AppError> {
    usize::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| AppError::InvalidArgument("模型最大输出必须大于 0".into()))
}

fn llm_record_input(normalized: NormalizedLlmConfig) -> LlmConfigRecordInput {
    LlmConfigRecordInput {
        name: normalized.name,
        provider: normalized.provider,
        base_url: normalized.base_url,
        api_key_encrypted: normalized.api_key_encrypted,
        model: normalized.model,
        max_concurrent: normalized.max_concurrent,
        timeout_secs: normalized.timeout_secs,
        max_context_tokens: normalized.max_context_tokens,
        max_output_tokens: normalized.max_output_tokens,
        extra_body_json: normalized.extra_body_json,
    }
}

fn image_record_input(params: CreateImageConfigParams) -> ImageConfigRecordInput {
    ImageConfigRecordInput {
        name: params.name,
        backend: params.backend,
        base_url: params.base_url,
        api_key_encrypted: params.api_key_encrypted,
        model: params.model,
        max_concurrent: params.max_concurrent,
        default_width: params.default_width,
        default_height: params.default_height,
    }
}

fn video_record_input(params: VideoConfigParams) -> VideoConfigRecordInput {
    VideoConfigRecordInput {
        name: params.name,
        backend: params.backend,
        base_url: params.base_url,
        api_key_encrypted: params.api_key_encrypted,
        model: params.model,
        max_concurrent: params.max_concurrent,
        default_width: params.default_width,
        default_height: params.default_height,
        default_duration_secs: params.default_duration_secs,
        min_duration_secs: params.min_duration_secs,
        max_duration_secs: params.max_duration_secs,
        duration_mode: params.duration_mode,
        supported_durations_json: params.supported_durations_json,
        default_fps: params.default_fps,
    }
}

fn embedding_record_input(params: CreateEmbeddingConfigParams) -> EmbeddingConfigRecordInput {
    EmbeddingConfigRecordInput {
        name: params.name,
        backend: params.backend,
        base_url: params.base_url,
        api_key_encrypted: params.api_key_encrypted,
        model: params.model,
        dimension: params.dimension,
        batch_size: params.batch_size,
        local_model_path: params.local_model_path,
    }
}

fn merge_api_key_update(
    update: Option<Option<String>>,
    existing: Option<String>,
) -> Option<String> {
    update.unwrap_or(existing)
}

#[cfg(test)]
mod tests {
    use super::merge_api_key_update;

    #[test]
    fn api_key_update_preserves_clears_and_replaces_explicitly() {
        let existing = Some("saved".to_string());
        assert_eq!(merge_api_key_update(None, existing.clone()), existing);
        assert_eq!(merge_api_key_update(Some(None), Some("saved".into())), None);
        assert_eq!(
            merge_api_key_update(Some(Some("new".into())), Some("saved".into())),
            Some("new".into())
        );
    }
}
