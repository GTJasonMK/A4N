use super::OrchestratorContext;
use crate::video::{validate_video_config_model, video_provider_config_from_model};
use a4n_api_contract::config::{
    ConfigProviderKind, CreateEmbeddingInput, CreateLlmInput, EmbeddingConfigSummary,
    ImageConfigForConnectivityTest, ImageConfigInput, ImageConfigSummary, LlmConfigSummary,
    UpdateEmbeddingInput, UpdateImageConfigInput, UpdateLlmInput, UpdateVideoConfigInput,
    VideoConfigInput, VideoConfigSummary,
};
use a4n_api_contract::video::VideoProviderConfigPort;
use a4n_config::{
    ensure_default_config_delete_allowed, image_sizes_to_json, validate_embedding_config_params,
    validate_image_config_params,
};
use a4n_db::{
    DbHandle,
    config::{
        EmbeddingConfigRecordInput, ImageConfigRecordInput, LlmConfigRecordInput,
        ProviderConfigKind, SettingsAllConfigImportCounts, SettingsAllConfigImportInput,
        SettingsAllConfigRows, VideoConfigRecordInput, validate_embedding_config_model,
        validate_image_config_model,
    },
};
use a4n_db_models::models::{
    ai::{embedding_config, image_gen_config, llm_config, video_gen_config},
    user::theme_config,
};
use a4n_domain::cost::{ModelUsageRecordCompletion, ModelUsageRecordStart};
use a4n_domain::{
    ThemeDocument, ThemeMode, embedding::CreateEmbeddingConfigParams,
    image::CreateImageConfigParams,
};
use a4n_error::AppError;
use a4n_llm_config::{
    CreateLlmConfigInput, LlmConfigSnapshot, NormalizedLlmConfig, UpdateLlmConfigInput,
    normalize_create_llm_config_input, normalize_update_llm_config_input, validate_llm_config,
};
use a4n_provider_contract::{
    EmbeddingProviderConfig, ImageProviderConfig, LlmProviderConfig, VideoProviderConfig,
};
use a4n_video::{VideoConfigParams, validate_video_config_params};
use serde_json::Value;
use std::str::FromStr;

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

    pub(super) async fn start_model_usage_record(
        &self,
        input: ModelUsageRecordStart,
    ) -> Result<a4n_db_models::models::generation::model_usage_record::Model, AppError> {
        a4n_db::model_usage::start_model_usage_record(self.database.connection(), input).await
    }

    pub(super) async fn finish_model_usage_record(
        &self,
        record_id: i64,
        input: ModelUsageRecordCompletion,
    ) -> Result<(), AppError> {
        a4n_db::model_usage::finish_model_usage_record(self.database.connection(), record_id, input)
            .await
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
    ) -> Result<Vec<a4n_db_models::models::user::theme_resource::Model>, AppError> {
        a4n_db::config::list_theme_resources(self.database.connection(), theme_id).await
    }

    async fn list_theme_resources_for_themes(
        &self,
        theme_ids: &[i64],
    ) -> Result<Vec<a4n_db_models::models::user::theme_resource::Model>, AppError> {
        a4n_db::config::list_theme_resources_for_themes(self.database.connection(), theme_ids).await
    }

    async fn theme_resource_by_id(
        &self,
        resource_id: i64,
    ) -> Result<Option<a4n_db_models::models::user::theme_resource::Model>, AppError> {
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
    ) -> Result<a4n_db_models::models::user::theme_resource::Model, AppError> {
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
    ) -> Result<a4n_db_models::models::user::theme_resource::Model, AppError> {
        a4n_db::config::delete_theme_resource(self.database.connection(), resource_id).await
    }

    async fn theme_preference(
        &self,
        user_id: i64,
    ) -> Result<Option<a4n_db_models::models::user::theme_preference::Model>, AppError> {
        a4n_db::config::theme_preference(self.database.connection(), user_id).await
    }

    async fn upsert_theme_preference(
        &self,
        user_id: i64,
        appearance: a4n_domain::ThemeAppearance,
    ) -> Result<a4n_db_models::models::user::theme_preference::Model, AppError> {
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
        let normalized = normalize_llm_config_input(input)?;
        a4n_db::config::create_llm_config_record(
            self.database.connection(),
            llm_record_input(normalized),
        )
        .await
    }

    /// 读库、规范化更新并落库。
    async fn update_llm_config_validated(
        &self,
        config_id: i64,
        input: UpdateLlmInput,
    ) -> Result<llm_config::Model, AppError> {
        let existing = self.llm_config_by_id(config_id).await?;
        let mut normalized = normalize_update_llm_config_input(
            existing.api_key_encrypted.as_deref(),
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
        normalized.pricing_json = pricing_json(input.pricing)?;
        a4n_db::config::update_llm_config_record(
            self.database.connection(),
            existing,
            llm_record_input(normalized),
        )
        .await
    }

    /// 校验模型能力后设为默认 LLM 配置。
    async fn set_default_llm_config_validated(&self, config_id: i64) -> Result<(), AppError> {
        let target = self.llm_config_by_id(config_id).await?;
        validate_llm_config(&llm_config_snapshot(&target))?;
        self.set_default_provider_config(ProviderConfigKind::Llm, config_id)
            .await
    }

    /// 非默认门禁后删除 LLM 配置。
    async fn delete_llm_config_if_allowed(&self, config_id: i64) -> Result<(), AppError> {
        let target = self.llm_config_by_id(config_id).await?;
        ensure_default_config_delete_allowed(" LLM 配置", target.is_default)?;
        a4n_db::config::delete_llm_config_record(self.database.connection(), config_id).await
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
        let (params, pricing) = image_config_params(input)?;
        a4n_db::config::create_image_config_record(
            self.database.connection(),
            image_record_input(params, pricing),
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
        let pricing = pricing_json(input.pricing)?;
        let params = validate_image_config_params(CreateImageConfigParams {
            name: input.name,
            backend: input.backend,
            base_url: input.base_url,
            api_key_encrypted,
            model: input.model,
            max_concurrent: input.max_concurrent,
            supported_sizes: input.supported_sizes,
        })?;
        a4n_db::config::update_image_config_record(
            self.database.connection(),
            existing,
            image_record_input(params, pricing),
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
        let (params, pricing) = video_config_params(input)?;
        a4n_db::config::create_video_config_record(
            self.database.connection(),
            video_record_input(params, pricing),
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
        let pricing = pricing_json(input.pricing)?;
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
            video_record_input(params, pricing),
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
        let (params, pricing) = embedding_config_params(input)?;
        a4n_db::config::create_embedding_config_record(
            self.database.connection(),
            embedding_record_input(params, pricing),
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
        let pricing = pricing_json(input.pricing)?;
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
            embedding_record_input(params, pricing),
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
            image_record_input(config, serde_json::json!({})),
        )
        .await
    }

    async fn insert_baseline_video_config_if_empty(
        &self,
        config: VideoConfigParams,
    ) -> Result<bool, AppError> {
        a4n_db::config::insert_baseline_video_config_if_empty(
            self.database.connection(),
            video_record_input(config, serde_json::json!({})),
        )
        .await
    }

    async fn settings_all_config_rows(&self) -> Result<SettingsAllConfigRows, AppError> {
        a4n_db::config::settings_all_config_rows(self.database.connection()).await
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
    pub(crate) async fn list_themes_visible_to_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<theme_config::Model>, AppError> {
        self.settings.list_themes_visible_to_user(user_id).await
    }

    pub(crate) async fn theme_by_id(
        &self,
        theme_id: i64,
    ) -> Result<Option<theme_config::Model>, AppError> {
        self.settings.theme_by_id(theme_id).await
    }

    pub(crate) async fn create_user_theme_record(
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

    pub(crate) async fn update_theme_record(
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

    pub(crate) async fn delete_theme_record(&self, theme_id: i64) -> Result<(), AppError> {
        self.settings.delete_theme_record(theme_id).await
    }

    pub(crate) async fn activate_theme(
        &self,
        theme_id: i64,
    ) -> Result<theme_config::Model, AppError> {
        self.settings.activate_theme(theme_id).await
    }

    pub(crate) async fn list_theme_resources(
        &self,
        theme_id: i64,
    ) -> Result<Vec<a4n_db_models::models::user::theme_resource::Model>, AppError> {
        self.settings.list_theme_resources(theme_id).await
    }

    pub(crate) async fn list_theme_resources_for_themes(
        &self,
        theme_ids: &[i64],
    ) -> Result<Vec<a4n_db_models::models::user::theme_resource::Model>, AppError> {
        self.settings
            .list_theme_resources_for_themes(theme_ids)
            .await
    }

    pub(crate) async fn theme_resource_by_id(
        &self,
        resource_id: i64,
    ) -> Result<Option<a4n_db_models::models::user::theme_resource::Model>, AppError> {
        self.settings.theme_resource_by_id(resource_id).await
    }

    pub(crate) async fn upsert_theme_resource(
        &self,
        theme_id: i64,
        resource_key: &str,
        file_name: &str,
        mime_type: &str,
        file_path: &str,
        sha256: &str,
        byte_size: i64,
    ) -> Result<a4n_db_models::models::user::theme_resource::Model, AppError> {
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

    pub(crate) async fn delete_theme_resource(
        &self,
        resource_id: i64,
    ) -> Result<a4n_db_models::models::user::theme_resource::Model, AppError> {
        self.settings.delete_theme_resource(resource_id).await
    }

    pub(crate) async fn theme_preference(
        &self,
        user_id: i64,
    ) -> Result<Option<a4n_db_models::models::user::theme_preference::Model>, AppError> {
        self.settings.theme_preference(user_id).await
    }

    pub(crate) async fn upsert_theme_preference(
        &self,
        user_id: i64,
        appearance: a4n_domain::ThemeAppearance,
    ) -> Result<a4n_db_models::models::user::theme_preference::Model, AppError> {
        self.settings
            .upsert_theme_preference(user_id, appearance)
            .await
    }

    pub(crate) async fn list_llm_config_records(&self) -> Result<Vec<llm_config::Model>, AppError> {
        self.settings.list_llm_config_records().await
    }

    pub(crate) async fn llm_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<llm_config::Model, AppError> {
        self.settings.llm_config_by_id(config_id).await
    }

    /// 创建 LLM 配置：规范化后落库。
    pub(crate) async fn create_llm_config_validated(
        &self,
        input: CreateLlmInput,
    ) -> Result<llm_config::Model, AppError> {
        self.settings.create_llm_config_validated(input).await
    }

    /// 更新 LLM 配置：规范化、默认 max_tokens 门禁后落库。
    pub(crate) async fn update_llm_config_validated(
        &self,
        config_id: i64,
        input: UpdateLlmInput,
    ) -> Result<llm_config::Model, AppError> {
        self.settings
            .update_llm_config_validated(config_id, input)
            .await
    }

    /// 设为默认 LLM 配置：模型与 max_tokens 门禁后落库。
    pub(crate) async fn set_default_llm_config_validated(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.settings
            .set_default_llm_config_validated(config_id)
            .await
    }

    /// 删除 LLM 配置：非默认门禁后落库。
    pub(crate) async fn delete_llm_config_if_allowed(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.settings.delete_llm_config_if_allowed(config_id).await
    }

    pub(crate) async fn list_image_config_records(
        &self,
    ) -> Result<Vec<image_gen_config::Model>, AppError> {
        self.settings.list_image_config_records().await
    }

    pub(crate) async fn image_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<image_gen_config::Model, AppError> {
        self.settings.image_config_by_id(config_id).await
    }

    /// 创建图片配置：校验参数后落库。
    pub(crate) async fn create_image_config_validated(
        &self,
        input: ImageConfigInput,
    ) -> Result<image_gen_config::Model, AppError> {
        self.settings.create_image_config_validated(input).await
    }

    /// 更新图片配置：读库、合并 api_key、校验后落库。
    pub(crate) async fn update_image_config_validated(
        &self,
        config_id: i64,
        input: UpdateImageConfigInput,
    ) -> Result<image_gen_config::Model, AppError> {
        self.settings
            .update_image_config_validated(config_id, input)
            .await
    }

    /// 设为默认图片配置：模型校验后落库。
    pub(crate) async fn set_default_image_config_validated(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.settings
            .set_default_image_config_validated(config_id)
            .await
    }

    /// 删除图片配置：非默认门禁后落库。
    pub(crate) async fn delete_image_config_if_allowed(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.settings
            .delete_image_config_if_allowed(config_id)
            .await
    }

    pub(crate) async fn resolve_image_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<image_gen_config::Model, AppError> {
        self.settings.resolve_image_config(config_id).await
    }

    pub(crate) async fn video_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<video_gen_config::Model, AppError> {
        self.settings.video_config_by_id(config_id).await
    }

    pub(crate) async fn list_video_config_records(
        &self,
    ) -> Result<Vec<video_gen_config::Model>, AppError> {
        self.settings.list_video_config_records().await
    }

    /// 创建视频配置：校验参数后落库。
    pub(crate) async fn create_video_config_validated(
        &self,
        input: VideoConfigInput,
    ) -> Result<video_gen_config::Model, AppError> {
        self.settings.create_video_config_validated(input).await
    }

    /// 更新视频配置：读库、合并 api_key、校验后落库。
    pub(crate) async fn update_video_config_validated(
        &self,
        config_id: i64,
        input: UpdateVideoConfigInput,
    ) -> Result<video_gen_config::Model, AppError> {
        self.settings
            .update_video_config_validated(config_id, input)
            .await
    }

    /// 设为默认视频配置：模型校验后落库。
    pub(crate) async fn set_default_video_config_validated(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.settings
            .set_default_video_config_validated(config_id)
            .await
    }

    /// 删除视频配置：非默认且无进行中任务门禁后落库。
    pub(crate) async fn delete_video_config_if_allowed(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.settings
            .delete_video_config_if_allowed(config_id)
            .await
    }

    pub(crate) async fn default_video_config(&self) -> Result<video_gen_config::Model, AppError> {
        self.settings.default_video_config().await
    }

    pub(crate) async fn active_generated_video_ids_for_config(
        &self,
        config_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        self.settings
            .active_generated_video_ids_for_config(config_id)
            .await
    }

    pub(crate) async fn list_embedding_configs(
        &self,
    ) -> Result<Vec<embedding_config::Model>, AppError> {
        self.settings.list_embedding_configs().await
    }

    pub(crate) async fn embedding_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<embedding_config::Model, AppError> {
        self.settings.embedding_config_by_id(config_id).await
    }

    /// 创建嵌入配置：校验参数后落库。
    pub(crate) async fn create_embedding_config_validated(
        &self,
        input: CreateEmbeddingInput,
    ) -> Result<embedding_config::Model, AppError> {
        self.settings.create_embedding_config_validated(input).await
    }

    /// 更新嵌入配置：完整替换字段并保留未提交的 API Key。
    pub(crate) async fn update_embedding_config_validated(
        &self,
        config_id: i64,
        input: UpdateEmbeddingInput,
    ) -> Result<embedding_config::Model, AppError> {
        self.settings
            .update_embedding_config_validated(config_id, input)
            .await
    }

    /// 设为默认嵌入配置：模型校验后落库。
    pub(crate) async fn set_default_embedding_config_validated(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.settings
            .set_default_embedding_config_validated(config_id)
            .await
    }

    /// 删除嵌入配置：非默认门禁后落库。
    pub(crate) async fn delete_embedding_config_if_allowed(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.settings
            .delete_embedding_config_if_allowed(config_id)
            .await
    }

    pub(crate) async fn llm_configs_exist(&self) -> Result<bool, AppError> {
        self.settings.llm_configs_exist().await
    }

    pub(crate) async fn image_configs_exist(&self) -> Result<bool, AppError> {
        self.settings.image_configs_exist().await
    }

    pub(crate) async fn video_configs_exist(&self) -> Result<bool, AppError> {
        self.settings.video_configs_exist().await
    }

    pub(crate) async fn insert_baseline_embedding_config_if_empty(&self) -> Result<bool, AppError> {
        self.settings
            .insert_baseline_embedding_config_if_empty()
            .await
    }

    pub(crate) async fn insert_baseline_llm_config_if_empty(
        &self,
        config: NormalizedLlmConfig,
    ) -> Result<bool, AppError> {
        self.settings
            .insert_baseline_llm_config_if_empty(config)
            .await
    }

    pub(crate) async fn insert_baseline_image_config_if_empty(
        &self,
        config: CreateImageConfigParams,
    ) -> Result<bool, AppError> {
        self.settings
            .insert_baseline_image_config_if_empty(config)
            .await
    }

    pub(crate) async fn insert_baseline_video_config_if_empty(
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

#[async_trait::async_trait]
impl a4n_api_contract::config::ConfigReadApplicationPort for OrchestratorContext {
    async fn llm_configs_exist(&self) -> Result<bool, AppError> {
        OrchestratorContext::llm_configs_exist(self).await
    }

    async fn image_configs_exist(&self) -> Result<bool, AppError> {
        OrchestratorContext::image_configs_exist(self).await
    }

    async fn video_configs_exist(&self) -> Result<bool, AppError> {
        OrchestratorContext::video_configs_exist(self).await
    }

    async fn saved_api_key(
        &self,
        caller_user_id: i64,
        kind: ConfigProviderKind,
        config_id: i64,
    ) -> Result<Option<String>, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        if config_id <= 0 {
            return Err(AppError::InvalidArgument(
                "配置 ID 必须为正整数".to_string(),
            ));
        }
        let api_key = match kind {
            ConfigProviderKind::Llm => self.llm_config_by_id(config_id).await?.api_key_encrypted,
            ConfigProviderKind::Image => {
                self.image_config_by_id(config_id).await?.api_key_encrypted
            }
            ConfigProviderKind::Video => {
                self.video_config_by_id(config_id).await?.api_key_encrypted
            }
        };
        Ok(api_key.and_then(|value| {
            let value = value.trim().to_string();
            (!value.is_empty()).then_some(value)
        }))
    }

    async fn llm_config_for_connectivity_test(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<LlmProviderConfig, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        validate_positive_config_id(config_id)?;
        Ok(llm_provider_config_from_model(
            self.llm_config_by_id(config_id).await?,
        ))
    }

    async fn embedding_config_for_connectivity_test(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<EmbeddingProviderConfig, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        validate_positive_config_id(config_id)?;
        Ok(embedding_provider_config_from_model(
            self.embedding_config_by_id(config_id).await?,
        ))
    }

    async fn image_config_for_connectivity_test(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<ImageConfigForConnectivityTest, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        validate_positive_config_id(config_id)?;
        Ok(image_config_for_connectivity_test_from_model(
            self.image_config_by_id(config_id).await?,
        ))
    }

    async fn list_llm(&self) -> Result<Vec<LlmConfigSummary>, AppError> {
        Ok(self
            .list_llm_config_records()
            .await?
            .iter()
            .map(llm_config_summary)
            .collect())
    }

    async fn list_image(&self) -> Result<Vec<ImageConfigSummary>, AppError> {
        self.list_image_config_records()
            .await?
            .iter()
            .map(image_config_summary)
            .collect()
    }

    async fn list_video(&self) -> Result<Vec<VideoConfigSummary>, AppError> {
        self.list_video_config_records()
            .await?
            .iter()
            .map(video_config_summary)
            .collect()
    }

    async fn list_embedding(&self) -> Result<Vec<EmbeddingConfigSummary>, AppError> {
        Ok(self
            .list_embedding_configs()
            .await?
            .iter()
            .map(embedding_config_summary)
            .collect())
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::config::ConfigMutationApplicationPort for OrchestratorContext {
    async fn insert_baseline_embedding_if_empty(&self) -> Result<bool, AppError> {
        OrchestratorContext::insert_baseline_embedding_config_if_empty(self).await
    }

    async fn insert_baseline_llm_if_empty(&self, input: CreateLlmInput) -> Result<bool, AppError> {
        OrchestratorContext::insert_baseline_llm_config_if_empty(
            self,
            normalize_llm_config_input(input)?,
        )
        .await
    }

    async fn insert_baseline_image_if_empty(
        &self,
        input: ImageConfigInput,
    ) -> Result<bool, AppError> {
        let (params, _) = image_config_params(input)?;
        OrchestratorContext::insert_baseline_image_config_if_empty(self, params).await
    }

    async fn insert_baseline_video_if_empty(
        &self,
        input: VideoConfigInput,
    ) -> Result<bool, AppError> {
        let (params, _) = video_config_params(input)?;
        OrchestratorContext::insert_baseline_video_config_if_empty(self, params).await
    }

    async fn create_llm(
        &self,
        caller_user_id: i64,
        input: CreateLlmInput,
    ) -> Result<LlmConfigSummary, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        Ok(llm_config_summary(
            &self.create_llm_config_validated(input).await?,
        ))
    }

    async fn update_llm(
        &self,
        caller_user_id: i64,
        config_id: i64,
        input: UpdateLlmInput,
    ) -> Result<LlmConfigSummary, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        Ok(llm_config_summary(
            &self.update_llm_config_validated(config_id, input).await?,
        ))
    }

    async fn set_default_llm(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.require_settings_admin(caller_user_id).await?;
        self.set_default_llm_config_validated(config_id).await
    }

    async fn delete_llm(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.require_settings_admin(caller_user_id).await?;
        self.delete_llm_config_if_allowed(config_id).await
    }

    async fn create_image(
        &self,
        caller_user_id: i64,
        input: ImageConfigInput,
    ) -> Result<ImageConfigSummary, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        image_config_summary(&self.create_image_config_validated(input).await?)
    }

    async fn update_image(
        &self,
        caller_user_id: i64,
        config_id: i64,
        input: UpdateImageConfigInput,
    ) -> Result<ImageConfigSummary, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        image_config_summary(&self.update_image_config_validated(config_id, input).await?)
    }

    async fn set_default_image(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.require_settings_admin(caller_user_id).await?;
        self.set_default_image_config_validated(config_id).await
    }

    async fn delete_image(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.require_settings_admin(caller_user_id).await?;
        self.delete_image_config_if_allowed(config_id).await
    }

    async fn create_video(
        &self,
        caller_user_id: i64,
        input: VideoConfigInput,
    ) -> Result<VideoConfigSummary, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        video_config_summary(&self.create_video_config_validated(input).await?)
    }

    async fn update_video(
        &self,
        caller_user_id: i64,
        config_id: i64,
        input: UpdateVideoConfigInput,
    ) -> Result<VideoConfigSummary, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        video_config_summary(&self.update_video_config_validated(config_id, input).await?)
    }

    async fn set_default_video(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.require_settings_admin(caller_user_id).await?;
        self.set_default_video_config_validated(config_id).await
    }

    async fn delete_video(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.require_settings_admin(caller_user_id).await?;
        self.delete_video_config_if_allowed(config_id).await
    }

    async fn create_embedding(
        &self,
        caller_user_id: i64,
        input: CreateEmbeddingInput,
    ) -> Result<EmbeddingConfigSummary, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        Ok(embedding_config_summary(
            &self.create_embedding_config_validated(input).await?,
        ))
    }

    async fn update_embedding(
        &self,
        caller_user_id: i64,
        config_id: i64,
        input: UpdateEmbeddingInput,
    ) -> Result<EmbeddingConfigSummary, AppError> {
        self.require_settings_admin(caller_user_id).await?;
        Ok(embedding_config_summary(
            &self
                .update_embedding_config_validated(config_id, input)
                .await?,
        ))
    }

    async fn set_default_embedding(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.require_settings_admin(caller_user_id).await?;
        self.set_default_embedding_config_validated(config_id).await
    }

    async fn delete_embedding(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.require_settings_admin(caller_user_id).await?;
        self.delete_embedding_config_if_allowed(config_id).await
    }
}

impl OrchestratorContext {
    async fn require_settings_admin(&self, user_id: i64) -> Result<(), AppError> {
        self.require_admin_permission(
            user_id,
            a4n_api_contract::admin::AdminPermission::ManageSettings,
        )
        .await
    }
}

fn has_configured_api_key(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}

fn pricing_json(pricing: a4n_api_contract::config::PricingPlan) -> Result<Value, AppError> {
    let pricing = pricing.validate().map_err(AppError::InvalidArgument)?;
    serde_json::to_value(pricing).map_err(AppError::Serialization)
}

fn pricing_from_json(value: &Value) -> Result<a4n_api_contract::config::PricingPlan, AppError> {
    let pricing: a4n_api_contract::config::PricingPlan = serde_json::from_value(value.clone())
        .map_err(|error| AppError::InvalidState {
            current: error.to_string(),
            operation: "读取 Provider 价格计划".to_string(),
        })?;
    pricing.validate().map_err(|error| AppError::InvalidState {
        current: error,
        operation: "校验 Provider 价格计划".to_string(),
    })
}
fn validate_positive_config_id(config_id: i64) -> Result<(), AppError> {
    if config_id > 0 {
        return Ok(());
    }
    Err(AppError::InvalidArgument(
        "配置 ID 必须为正整数".to_string(),
    ))
}

fn llm_provider_config_from_model(model: llm_config::Model) -> LlmProviderConfig {
    LlmProviderConfig {
        id: model.id,
        name: model.name,
        provider: model.provider,
        base_url: model.base_url,
        api_key: model.api_key_encrypted,
        model: model.model,
        max_concurrent: model.max_concurrent,
        timeout_secs: model.timeout_secs,
        max_context_tokens: model.max_context_tokens,
        max_output_tokens: model.max_output_tokens,
        extra_body_json: model.extra_body_json,
        pricing: pricing_from_json(&model.pricing_json).unwrap_or_default(),
    }
}

fn embedding_provider_config_from_model(model: embedding_config::Model) -> EmbeddingProviderConfig {
    EmbeddingProviderConfig {
        id: model.id,
        name: model.name,
        backend: model.backend,
        base_url: model.base_url,
        api_key: model.api_key_encrypted,
        model: model.model,
        dimension: model.dimension,
        batch_size: model.batch_size,
        local_model_path: model.local_model_path,
        pricing: pricing_from_json(&model.pricing_json).unwrap_or_default(),
    }
}

fn image_provider_config_from_model(model: image_gen_config::Model) -> ImageProviderConfig {
    ImageProviderConfig {
        id: model.id,
        name: model.name,
        backend: model.backend,
        base_url: model.base_url,
        api_key: model.api_key_encrypted,
        model: model.model,
        max_concurrent: model.max_concurrent,
        supported_sizes_json: model.supported_sizes_json,
        pricing: pricing_from_json(&model.pricing_json).unwrap_or_default(),
    }
}

fn image_config_for_connectivity_test_from_model(
    model: image_gen_config::Model,
) -> ImageConfigForConnectivityTest {
    ImageConfigForConnectivityTest {
        id: model.id,
        name: model.name,
        backend: model.backend,
        base_url: model.base_url,
        api_key: model.api_key_encrypted,
        model: model.model,
        max_concurrent: model.max_concurrent,
        supported_sizes_json: model.supported_sizes_json,
        pricing: pricing_from_json(&model.pricing_json).unwrap_or_default(),
    }
}

fn llm_config_summary(config: &llm_config::Model) -> LlmConfigSummary {
    LlmConfigSummary {
        id: config.id,
        name: config.name.clone(),
        provider: config.provider.clone(),
        base_url: config.base_url.clone(),
        has_api_key: has_configured_api_key(config.api_key_encrypted.as_deref()),
        model: config.model.clone(),
        max_concurrent: config.max_concurrent,
        timeout_secs: config.timeout_secs,
        max_context_tokens: config.max_context_tokens,
        max_output_tokens: config.max_output_tokens,
        extra_body_json: config.extra_body_json.clone(),
        is_default: config.is_default,
        pricing: pricing_from_json(&config.pricing_json).unwrap_or_default(),
    }
}

fn image_config_summary(config: &image_gen_config::Model) -> Result<ImageConfigSummary, AppError> {
    Ok(ImageConfigSummary {
        id: config.id,
        name: config.name.clone(),
        backend: config.backend.clone(),
        base_url: config
            .base_url
            .clone()
            .ok_or_else(|| AppError::InvalidState {
                current: format!("图片配置 #{} 缺少 base_url", config.id),
                operation: "投影图片配置摘要".to_string(),
            })?,
        has_api_key: has_configured_api_key(config.api_key_encrypted.as_deref()),
        model: config.model.clone(),
        max_concurrent: config.max_concurrent,
        supported_sizes: a4n_config::image_sizes_from_json(&config.supported_sizes_json).map_err(
            |error| AppError::InvalidState {
                current: error.to_string(),
                operation: "投影图片配置尺寸".to_string(),
            },
        )?,
        is_default: config.is_default,
        pricing: pricing_from_json(&config.pricing_json).unwrap_or_default(),
    })
}

#[async_trait::async_trait]
impl VideoProviderConfigPort for OrchestratorContext {
    async fn by_id(&self, config_id: i64) -> Result<VideoProviderConfig, AppError> {
        self.video_config_by_id(config_id)
            .await
            .map(|config| video_provider_config_from_model(&config))
    }

    async fn default(&self) -> Result<VideoProviderConfig, AppError> {
        self.default_video_config()
            .await
            .map(|config| video_provider_config_from_model(&config))
    }

    async fn for_provider_job(
        &self,
        project_id: Option<i64>,
        provider_kind: &str,
        provider_job_id: &str,
    ) -> Result<VideoProviderConfig, AppError> {
        self.resolve_video_config_for_provider_job(project_id, provider_kind, provider_job_id)
            .await
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::image::ImageProviderConfigPort for OrchestratorContext {
    async fn by_id(&self, config_id: i64) -> Result<ImageProviderConfig, AppError> {
        self.image_config_by_id(config_id)
            .await
            .map(image_provider_config_from_model)
    }

    async fn default(&self) -> Result<ImageProviderConfig, AppError> {
        self.resolve_image_config(None)
            .await
            .map(image_provider_config_from_model)
    }
}

fn video_config_summary(config: &video_gen_config::Model) -> Result<VideoConfigSummary, AppError> {
    Ok(VideoConfigSummary {
        id: config.id,
        name: config.name.clone(),
        backend: config.backend.clone(),
        base_url: config
            .base_url
            .clone()
            .ok_or_else(|| AppError::InvalidState {
                current: format!("视频配置 #{} 缺少 base_url", config.id),
                operation: "投影视频配置摘要".to_string(),
            })?,
        has_api_key: has_configured_api_key(config.api_key_encrypted.as_deref()),
        model: config.model.clone(),
        max_concurrent: config.max_concurrent,
        default_width: config.default_width,
        default_height: config.default_height,
        default_duration_secs: config.default_duration_secs,
        min_duration_secs: config.min_duration_secs,
        max_duration_secs: config.max_duration_secs,
        duration_mode: a4n_api_contract::config::VideoDurationMode::from_str(&config.duration_mode)
            .map_err(|error| AppError::InvalidState {
                current: error.to_string(),
                operation: "投影视频配置 duration_mode".to_string(),
            })?,
        supported_durations_json: config.supported_durations_json.clone(),
        default_fps: config.default_fps,
        is_default: config.is_default,
        pricing: pricing_from_json(&config.pricing_json).unwrap_or_default(),
    })
}

fn embedding_config_summary(config: &embedding_config::Model) -> EmbeddingConfigSummary {
    EmbeddingConfigSummary {
        id: config.id,
        name: config.name.clone(),
        backend: config.backend.clone(),
        base_url: config.base_url.clone(),
        has_api_key: has_configured_api_key(config.api_key_encrypted.as_deref()),
        model: config.model.clone(),
        dimension: config.dimension,
        batch_size: config.batch_size,
        local_model_path: config.local_model_path.clone(),
        is_default: config.is_default,
        pricing: pricing_from_json(&config.pricing_json).unwrap_or_default(),
    }
}

fn normalize_llm_config_input(input: CreateLlmInput) -> Result<NormalizedLlmConfig, AppError> {
    let CreateLlmInput {
        name,
        provider,
        base_url,
        api_key_encrypted,
        model,
        max_concurrent,
        timeout_secs,
        max_context_tokens,
        max_output_tokens,
        extra_body_json,
        pricing,
    } = input;
    let mut normalized = normalize_create_llm_config_input(CreateLlmConfigInput {
        name: &name,
        provider: &provider,
        base_url: &base_url,
        api_key_encrypted: api_key_encrypted.as_deref(),
        model: &model,
        max_concurrent,
        timeout_secs,
        max_context_tokens,
        max_output_tokens,
        extra_body_json,
    })?;
    normalized.pricing_json = pricing_json(pricing)?;
    Ok(normalized)
}

fn image_config_params(
    input: ImageConfigInput,
) -> Result<(CreateImageConfigParams, Value), AppError> {
    let pricing = pricing_json(input.pricing)?;
    let params = validate_image_config_params(CreateImageConfigParams {
        name: input.name,
        backend: input.backend,
        base_url: input.base_url,
        api_key_encrypted: input.api_key_encrypted,
        model: input.model,
        max_concurrent: input.max_concurrent,
        supported_sizes: input.supported_sizes,
    })?;
    Ok((params, pricing))
}

fn video_config_params(input: VideoConfigInput) -> Result<(VideoConfigParams, Value), AppError> {
    let pricing = pricing_json(input.pricing)?;
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
    Ok((params, pricing))
}

fn embedding_config_params(
    input: CreateEmbeddingInput,
) -> Result<(CreateEmbeddingConfigParams, Value), AppError> {
    let pricing = pricing_json(input.pricing)?;
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
    Ok((params, pricing))
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
        pricing_json: normalized.pricing_json,
    }
}

fn llm_config_snapshot(model: &llm_config::Model) -> LlmConfigSnapshot {
    LlmConfigSnapshot {
        name: model.name.clone(),
        provider: model.provider.clone(),
        base_url: model.base_url.clone(),
        api_key_encrypted: model.api_key_encrypted.clone(),
        model: model.model.clone(),
        max_concurrent: model.max_concurrent,
        timeout_secs: model.timeout_secs,
        max_context_tokens: model.max_context_tokens,
        max_output_tokens: model.max_output_tokens,
        extra_body_json: model.extra_body_json.clone(),
    }
}

fn image_record_input(
    params: CreateImageConfigParams,
    pricing_json: Value,
) -> ImageConfigRecordInput {
    ImageConfigRecordInput {
        name: params.name,
        backend: params.backend,
        base_url: params.base_url,
        api_key_encrypted: params.api_key_encrypted,
        model: params.model,
        max_concurrent: params.max_concurrent,
        supported_sizes_json: image_sizes_to_json(&params.supported_sizes)
            .expect("图片配置尺寸已在校验阶段验证"),
        pricing_json,
    }
}

fn video_record_input(params: VideoConfigParams, pricing_json: Value) -> VideoConfigRecordInput {
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
        pricing_json,
    }
}

fn embedding_record_input(
    params: CreateEmbeddingConfigParams,
    pricing_json: Value,
) -> EmbeddingConfigRecordInput {
    EmbeddingConfigRecordInput {
        name: params.name,
        backend: params.backend,
        base_url: params.base_url,
        api_key_encrypted: params.api_key_encrypted,
        model: params.model,
        dimension: params.dimension,
        batch_size: params.batch_size,
        local_model_path: params.local_model_path,
        pricing_json,
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
    use super::{
        embedding_provider_config_from_model, image_config_for_connectivity_test_from_model,
        llm_provider_config_from_model, merge_api_key_update, validate_positive_config_id,
    };
    use a4n_db_models::models::ai::{embedding_config, image_gen_config, llm_config};
    use serde_json::json;

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

    #[test]
    fn connectivity_config_mapping_preserves_provider_fields() {
        let llm = llm_provider_config_from_model(llm_config::Model {
            id: 7,
            name: "llm".into(),
            provider: "openai".into(),
            base_url: "https://example.test/v1".into(),
            api_key_encrypted: Some("secret".into()),
            model: "test-model".into(),
            max_concurrent: 2,
            timeout_secs: 30,
            max_context_tokens: 128_000,
            max_output_tokens: 8_192,
            extra_body_json: Some(json!({"reasoning": {"effort": "low"}})),
            pricing_json: json!({}),
            is_default: true,
            created_at: Default::default(),
            updated_at: Default::default(),
        });
        assert_eq!(llm.id, 7);
        assert_eq!(llm.api_key.as_deref(), Some("secret"));
        assert_eq!(
            llm.extra_body_json,
            Some(json!({"reasoning": {"effort": "low"}}))
        );

        let embedding = embedding_provider_config_from_model(embedding_config::Model {
            id: 9,
            name: "embedding".into(),
            backend: "api".into(),
            base_url: Some("https://example.test".into()),
            api_key_encrypted: Some("embedding-secret".into()),
            model: "embedding-model".into(),
            dimension: 768,
            batch_size: 16,
            local_model_path: Some("/tmp/model".into()),
            pricing_json: json!({}),
            is_default: false,
            created_at: Default::default(),
            updated_at: Default::default(),
        });
        assert_eq!(embedding.id, 9);
        assert_eq!(embedding.api_key.as_deref(), Some("embedding-secret"));
        assert_eq!(embedding.local_model_path.as_deref(), Some("/tmp/model"));

        let image = image_config_for_connectivity_test_from_model(image_gen_config::Model {
            id: 11,
            name: "image".into(),
            backend: "openai-compatible-image".into(),
            base_url: Some("https://example.test/v1".into()),
            api_key_encrypted: Some("image-secret".into()),
            model: "image-model".into(),
            max_concurrent: 1,
            supported_sizes_json: json!([{"width": 1024, "height": 1024}]),
            pricing_json: json!({}),
            is_default: true,
            created_at: Default::default(),
            updated_at: Default::default(),
        });
        assert_eq!(image.id, 11);
        assert_eq!(image.api_key.as_deref(), Some("image-secret"));
        assert_eq!(image.base_url.as_deref(), Some("https://example.test/v1"));
        assert_eq!(
            image.supported_sizes_json,
            json!([{"width": 1024, "height": 1024}])
        );
    }

    #[test]
    fn connectivity_config_id_must_be_positive() {
        assert!(validate_positive_config_id(1).is_ok());
        assert!(validate_positive_config_id(0).is_err());
        assert!(validate_positive_config_id(-1).is_err());
    }
}
