use a4n_core::{
    config::{
        CreateEmbeddingConfigParams, CreateImageConfigParams, UpdateEmbeddingConfigParams,
        UpdateImageConfigParams, app_config::LlmMaxTokensConfig,
        ensure_default_config_delete_allowed, validate_embedding_config_model,
        validate_embedding_config_params, validate_image_config_model,
        validate_image_config_params,
    },
    error::AppError,
    models::{
        ai::{embedding_config, image_gen_config, llm_config, system_settings, video_gen_config},
        user::theme_config,
    },
};
use a4n_db::DbHandle;
use a4n_llm::config::{
    CreateLlmConfigInput, NormalizedLlmConfig, UpdateLlmConfigInput,
    normalize_create_llm_config_input, normalize_update_llm_config_input,
    validate_llm_config_model,
};
use serde_json::Value;

use super::OrchestratorContext;
use crate::config::{
    BaselineImageConfigSeed, BaselineLlmConfigSeed, BaselineVideoConfigSeed,
    DefaultLlmOutputCapRow, EmbeddingConfigCreateRecordInput, EmbeddingConfigUpdateRecordInput,
    ImageConfigCreateRecordInput, ImageConfigUpdateRecordInput, LlmConfigCreateRecordInput,
    LlmConfigUpdateRecordInput, SettingsAllConfigImportCounts, SettingsAllConfigImportInput,
    VideoConfigCreateRecordInput, VideoConfigUpdateRecordInput,
};
use crate::theme::{
    ensure_theme_can_be_deleted, ensure_theme_css_variables_object, normalize_theme_mode,
    normalize_theme_name,
};
use crate::video::{
    UpdateVideoConfigParams, VideoConfigParams, validate_video_config_model,
    validate_video_config_params,
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

    async fn list_user_themes(&self, user_id: i64) -> Result<Vec<theme_config::Model>, AppError> {
        a4n_db::config::list_user_themes(self.database.connection(), user_id).await
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
        mode: &str,
        css_variables: Value,
    ) -> Result<theme_config::Model, AppError> {
        a4n_db::config::create_theme(
            self.database.connection(),
            user_id,
            name,
            mode,
            css_variables,
        )
        .await
    }

    /// 校验主题输入并创建（业务 owner）。
    async fn create_theme_validated(
        &self,
        user_id: Option<i64>,
        name: String,
        mode: String,
        css_variables: Value,
    ) -> Result<theme_config::Model, AppError> {
        let name = normalize_theme_name(name).map_err(AppError::InvalidArgument)?;
        let mode = normalize_theme_mode(mode).map_err(AppError::InvalidArgument)?;
        ensure_theme_css_variables_object(&css_variables).map_err(AppError::InvalidArgument)?;
        self.create_theme_record(user_id, &name, &mode, css_variables)
            .await
    }

    async fn update_theme_record(
        &self,
        theme_id: i64,
        name: Option<&str>,
        mode: Option<&str>,
        css_variables: Option<Value>,
    ) -> Result<theme_config::Model, AppError> {
        a4n_db::config::update_theme(
            self.database.connection(),
            theme_id,
            name,
            mode,
            css_variables,
        )
        .await
    }

    /// 校验主题更新字段并落库（业务 owner）。
    async fn update_theme_validated(
        &self,
        theme_id: i64,
        name: Option<String>,
        mode: Option<String>,
        css_variables: Option<Value>,
    ) -> Result<theme_config::Model, AppError> {
        let name = name
            .map(normalize_theme_name)
            .transpose()
            .map_err(AppError::InvalidArgument)?;
        let mode = mode
            .map(normalize_theme_mode)
            .transpose()
            .map_err(AppError::InvalidArgument)?;
        if let Some(css_variables) = css_variables.as_ref() {
            ensure_theme_css_variables_object(css_variables).map_err(AppError::InvalidArgument)?;
        }
        self.update_theme_record(theme_id, name.as_deref(), mode.as_deref(), css_variables)
            .await
    }

    async fn delete_theme_record(&self, theme_id: i64) -> Result<(), AppError> {
        a4n_db::config::delete_theme(self.database.connection(), theme_id).await
    }

    /// 加载主题、校验可删门禁后删除（业务 owner）。
    async fn delete_theme_validated(&self, theme_id: i64) -> Result<(), AppError> {
        let theme = self
            .theme_by_id(theme_id)
            .await?
            .ok_or_else(|| AppError::NotFound("主题不存在".to_string()))?;
        ensure_theme_can_be_deleted(theme.is_system_default, theme.is_active)
            .map_err(AppError::InvalidArgument)?;
        self.delete_theme_record(theme_id).await
    }

    async fn activate_theme(&self, theme_id: i64) -> Result<theme_config::Model, AppError> {
        a4n_db::config::activate_theme(self.database.connection(), theme_id).await
    }

    async fn list_llm_config_records(&self) -> Result<Vec<llm_config::Model>, AppError> {
        a4n_db::config::list_llm_config_records(self.database.connection()).await
    }

    async fn llm_config_record_by_id(&self, config_id: i64) -> Result<llm_config::Model, AppError> {
        a4n_db::config::llm_config_record_by_id(self.database.connection(), config_id).await
    }

    async fn create_llm_config_record(
        &self,
        input: LlmConfigCreateRecordInput,
    ) -> Result<llm_config::Model, AppError> {
        a4n_db::config::create_llm_config_record(self.database.connection(), input).await
    }

    async fn update_llm_config_record(
        &self,
        existing: llm_config::Model,
        input: LlmConfigUpdateRecordInput,
    ) -> Result<llm_config::Model, AppError> {
        a4n_db::config::update_llm_config_record(self.database.connection(), existing, input).await
    }

    async fn set_default_llm_config_record(
        &self,
        model: llm_config::Model,
    ) -> Result<llm_config::Model, AppError> {
        a4n_db::config::set_default_llm_config_record(self.database.connection(), model).await
    }

    async fn delete_llm_config_record(&self, config_id: i64) -> Result<(), AppError> {
        a4n_db::config::delete_llm_config_record(self.database.connection(), config_id).await
    }

    /// 规范化 LLM 创建输入后落库。
    async fn create_llm_config_validated(
        &self,
        input: CreateLlmConfigInput<'_>,
    ) -> Result<llm_config::Model, AppError> {
        let normalized = normalize_create_llm_config_input(input)?;
        self.create_llm_config_record(llm_create_record_input(normalized))
            .await
    }

    /// 读库、规范化更新、默认配置与全局 max_tokens 门禁后落库。
    async fn update_llm_config_validated(
        &self,
        config_id: i64,
        input: UpdateLlmConfigInput<'_>,
    ) -> Result<llm_config::Model, AppError> {
        let existing = self.llm_config_record_by_id(config_id).await?;
        let normalized = normalize_update_llm_config_input(&existing, input)?;
        if existing.is_default {
            self.ensure_llm_max_tokens_fits_model_output(normalized.max_output_tokens)
                .await?;
        }
        self.update_llm_config_record(existing, llm_update_record_input(normalized))
            .await
    }

    /// 模型校验与 max_tokens 门禁后设为默认 LLM 配置。
    async fn set_default_llm_config_validated(
        &self,
        config_id: i64,
    ) -> Result<llm_config::Model, AppError> {
        let target = self.llm_config_record_by_id(config_id).await?;
        validate_llm_config_model(&target)?;
        self.ensure_llm_max_tokens_fits_model_output(target.max_output_tokens)
            .await?;
        self.set_default_llm_config_record(target).await
    }

    /// 非默认门禁后删除 LLM 配置。
    async fn delete_llm_config_if_allowed(&self, config_id: i64) -> Result<(), AppError> {
        let target = self.llm_config_record_by_id(config_id).await?;
        ensure_default_config_delete_allowed(" LLM 配置", target.is_default)?;
        self.delete_llm_config_record(config_id).await
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

    async fn image_config_record_by_id(
        &self,
        config_id: i64,
    ) -> Result<image_gen_config::Model, AppError> {
        a4n_db::config::image_config_record_by_id(self.database.connection(), config_id).await
    }

    async fn create_image_config_record(
        &self,
        input: ImageConfigCreateRecordInput,
    ) -> Result<image_gen_config::Model, AppError> {
        a4n_db::config::create_image_config_record(self.database.connection(), input).await
    }

    async fn update_image_config_record(
        &self,
        existing: image_gen_config::Model,
        input: ImageConfigUpdateRecordInput,
    ) -> Result<image_gen_config::Model, AppError> {
        a4n_db::config::update_image_config_record(self.database.connection(), existing, input)
            .await
    }

    async fn set_default_image_config_record(
        &self,
        model: image_gen_config::Model,
    ) -> Result<image_gen_config::Model, AppError> {
        a4n_db::config::set_default_image_config_record(self.database.connection(), model).await
    }

    async fn delete_image_config_record(&self, config_id: i64) -> Result<(), AppError> {
        a4n_db::config::delete_image_config_record(self.database.connection(), config_id).await
    }

    /// 校验图片配置参数后创建。
    async fn create_image_config_validated(
        &self,
        params: CreateImageConfigParams,
    ) -> Result<image_gen_config::Model, AppError> {
        let params = validate_image_config_params(params)?;
        self.create_image_config_record(ImageConfigCreateRecordInput {
            name: params.name,
            backend: params.backend,
            base_url: params.base_url,
            api_key_encrypted: params.api_key_encrypted,
            model: params.model,
            max_concurrent: params.max_concurrent,
            default_width: params.default_width,
            default_height: params.default_height,
            provider_options_json: params.provider_options_json,
        })
        .await
    }

    /// 按 id 读库、合并 api_key、校验后更新图片配置。
    async fn update_image_config_validated(
        &self,
        config_id: i64,
        params: UpdateImageConfigParams,
    ) -> Result<image_gen_config::Model, AppError> {
        let existing = self.image_config_record_by_id(config_id).await?;
        let api_key_encrypted = params
            .api_key_encrypted
            .clone()
            .unwrap_or_else(|| existing.api_key_encrypted.clone());
        let params = validate_image_config_params(CreateImageConfigParams {
            name: params.name,
            backend: params.backend,
            base_url: params.base_url,
            api_key_encrypted,
            model: params.model,
            max_concurrent: params.max_concurrent,
            default_width: params.default_width,
            default_height: params.default_height,
            provider_options_json: params.provider_options_json,
        })?;
        self.update_image_config_record(
            existing,
            ImageConfigUpdateRecordInput {
                name: params.name,
                backend: params.backend,
                base_url: params.base_url,
                api_key_encrypted: params.api_key_encrypted,
                model: params.model,
                max_concurrent: params.max_concurrent,
                default_width: params.default_width,
                default_height: params.default_height,
                provider_options_json: params.provider_options_json,
            },
        )
        .await
    }

    /// 校验模型可用后设为默认图片配置。
    async fn set_default_image_config_validated(
        &self,
        config_id: i64,
    ) -> Result<image_gen_config::Model, AppError> {
        let target = self.image_config_record_by_id(config_id).await?;
        validate_image_config_model(&target)?;
        self.set_default_image_config_record(target).await
    }

    /// 非默认门禁通过后删除图片配置。
    async fn delete_image_config_if_allowed(&self, config_id: i64) -> Result<(), AppError> {
        let target = self.image_config_record_by_id(config_id).await?;
        ensure_default_config_delete_allowed("图片配置", target.is_default)?;
        self.delete_image_config_record(config_id).await
    }

    async fn resolve_image_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<image_gen_config::Model, AppError> {
        a4n_db::config::resolve_image_config(self.database.connection(), config_id).await
    }

    async fn video_config(&self, config_id: i64) -> Result<video_gen_config::Model, AppError> {
        a4n_db::config::video_config_by_id(self.database.connection(), config_id).await
    }

    async fn list_video_config_records(&self) -> Result<Vec<video_gen_config::Model>, AppError> {
        a4n_db::config::list_video_config_records(self.database.connection()).await
    }

    async fn video_config_record_by_id(
        &self,
        config_id: i64,
    ) -> Result<video_gen_config::Model, AppError> {
        a4n_db::config::video_config_record_by_id(self.database.connection(), config_id).await
    }

    async fn create_video_config_record(
        &self,
        input: VideoConfigCreateRecordInput,
    ) -> Result<video_gen_config::Model, AppError> {
        a4n_db::config::create_video_config_record(self.database.connection(), input).await
    }

    async fn update_video_config_record(
        &self,
        existing: video_gen_config::Model,
        input: VideoConfigUpdateRecordInput,
    ) -> Result<video_gen_config::Model, AppError> {
        a4n_db::config::update_video_config_record(self.database.connection(), existing, input)
            .await
    }

    async fn set_default_video_config_record(
        &self,
        model: video_gen_config::Model,
    ) -> Result<video_gen_config::Model, AppError> {
        a4n_db::config::set_default_video_config_record(self.database.connection(), model).await
    }

    async fn delete_video_config_record(&self, config_id: i64) -> Result<(), AppError> {
        a4n_db::config::delete_video_config_record(self.database.connection(), config_id).await
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
        params: VideoConfigParams,
    ) -> Result<video_gen_config::Model, AppError> {
        let params = validate_video_config_params(params).map_err(AppError::InvalidArgument)?;
        self.create_video_config_record(VideoConfigCreateRecordInput {
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
            provider_options_json: params.provider_options_json,
        })
        .await
    }

    /// 按 id 读库、合并 api_key、校验后更新视频配置。
    async fn update_video_config_validated(
        &self,
        config_id: i64,
        params: UpdateVideoConfigParams,
    ) -> Result<video_gen_config::Model, AppError> {
        let existing = self.video_config_record_by_id(config_id).await?;
        let api_key_encrypted = params
            .api_key_encrypted
            .clone()
            .unwrap_or_else(|| existing.api_key_encrypted.clone());
        let params = validate_video_config_params(VideoConfigParams {
            name: params.name,
            backend: params.backend,
            base_url: params.base_url,
            api_key_encrypted,
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
            provider_options_json: params.provider_options_json,
        })
        .map_err(AppError::InvalidArgument)?;
        self.update_video_config_record(
            existing,
            VideoConfigUpdateRecordInput {
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
                provider_options_json: params.provider_options_json,
            },
        )
        .await
    }

    /// 校验模型可用后设为默认视频配置。
    async fn set_default_video_config_validated(
        &self,
        config_id: i64,
    ) -> Result<video_gen_config::Model, AppError> {
        let target = self.video_config_record_by_id(config_id).await?;
        validate_video_config_model(&target).map_err(AppError::InvalidArgument)?;
        self.set_default_video_config_record(target).await
    }

    /// 非默认且无进行中任务时删除视频配置。
    async fn delete_video_config_if_allowed(&self, config_id: i64) -> Result<(), AppError> {
        let target = self.video_config_record_by_id(config_id).await?;
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
        self.delete_video_config_record(config_id).await
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

    async fn create_embedding_config_record(
        &self,
        input: EmbeddingConfigCreateRecordInput,
    ) -> Result<embedding_config::Model, AppError> {
        a4n_db::config::create_embedding_config_record(self.database.connection(), input).await
    }

    async fn update_embedding_config_record(
        &self,
        existing: embedding_config::Model,
        input: EmbeddingConfigUpdateRecordInput,
    ) -> Result<embedding_config::Model, AppError> {
        a4n_db::config::update_embedding_config_record(self.database.connection(), existing, input)
            .await
    }

    async fn set_default_embedding_config_record(
        &self,
        model: embedding_config::Model,
    ) -> Result<embedding_config::Model, AppError> {
        a4n_db::config::set_default_embedding_config_record(self.database.connection(), model).await
    }

    async fn delete_embedding_config_record(&self, config_id: i64) -> Result<(), AppError> {
        a4n_db::config::delete_embedding_config_record(self.database.connection(), config_id).await
    }

    /// 校验嵌入配置参数后创建。
    async fn create_embedding_config_validated(
        &self,
        params: CreateEmbeddingConfigParams,
    ) -> Result<embedding_config::Model, AppError> {
        let params = validate_embedding_config_params(params)?;
        self.create_embedding_config_record(EmbeddingConfigCreateRecordInput {
            name: params.name,
            backend: params.backend,
            base_url: params.base_url,
            api_key_encrypted: params.api_key_encrypted,
            model: params.model,
            dimension: params.dimension,
            batch_size: params.batch_size,
            local_model_path: params.local_model_path,
        })
        .await
    }

    /// 读库、合并补丁、校验后更新嵌入配置。
    async fn update_embedding_config_validated(
        &self,
        config_id: i64,
        patch: UpdateEmbeddingConfigParams,
    ) -> Result<embedding_config::Model, AppError> {
        let existing = self.embedding_config_by_id(config_id).await?;
        // UpdateEmbeddingConfigParams 为完整替换字段（非 partial Option 补丁）。
        let requested_api_key = match &patch.api_key_encrypted {
            Some(inner) => inner.clone(),
            None => existing.api_key_encrypted.clone(),
        };
        let params = validate_embedding_config_params(CreateEmbeddingConfigParams {
            name: patch.name,
            backend: patch.backend,
            base_url: patch.base_url.or_else(|| existing.base_url.clone()),
            api_key_encrypted: requested_api_key,
            model: patch.model,
            dimension: patch.dimension,
            batch_size: patch.batch_size,
            local_model_path: patch
                .local_model_path
                .or_else(|| existing.local_model_path.clone()),
        })?;
        let api_key_encrypted = if params.backend == "api" {
            params.api_key_encrypted
        } else {
            None
        };
        self.update_embedding_config_record(
            existing,
            EmbeddingConfigUpdateRecordInput {
                name: params.name,
                backend: params.backend,
                base_url: params.base_url,
                api_key_encrypted,
                model: params.model,
                dimension: params.dimension,
                batch_size: params.batch_size,
                local_model_path: params.local_model_path,
            },
        )
        .await
    }

    /// 模型校验后设为默认嵌入配置。
    async fn set_default_embedding_config_validated(
        &self,
        config_id: i64,
    ) -> Result<embedding_config::Model, AppError> {
        let target = self.embedding_config_by_id(config_id).await?;
        validate_embedding_config_model(&target)?;
        self.set_default_embedding_config_record(target).await
    }

    /// 非默认门禁后删除嵌入配置。
    async fn delete_embedding_config_if_allowed(&self, config_id: i64) -> Result<(), AppError> {
        let target = self.embedding_config_by_id(config_id).await?;
        ensure_default_config_delete_allowed(" Embedding 配置", target.is_default)?;
        self.delete_embedding_config_record(config_id).await
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
        seed: BaselineLlmConfigSeed,
    ) -> Result<bool, AppError> {
        a4n_db::config::insert_baseline_llm_config_if_empty(self.database.connection(), seed).await
    }

    async fn insert_baseline_image_config_if_empty(
        &self,
        seed: BaselineImageConfigSeed,
    ) -> Result<bool, AppError> {
        a4n_db::config::insert_baseline_image_config_if_empty(self.database.connection(), seed)
            .await
    }

    async fn insert_baseline_video_config_if_empty(
        &self,
        seed: BaselineVideoConfigSeed,
    ) -> Result<bool, AppError> {
        a4n_db::config::insert_baseline_video_config_if_empty(self.database.connection(), seed)
            .await
    }

    async fn settings_all_export_llm_configs(&self) -> Result<Vec<llm_config::Model>, AppError> {
        a4n_db::config::settings_all_export_llm_configs(self.database.connection()).await
    }

    async fn settings_all_export_embedding_configs(
        &self,
    ) -> Result<Vec<embedding_config::Model>, AppError> {
        a4n_db::config::settings_all_export_embedding_configs(self.database.connection()).await
    }

    async fn settings_all_export_image_configs(
        &self,
    ) -> Result<Vec<image_gen_config::Model>, AppError> {
        a4n_db::config::settings_all_export_image_configs(self.database.connection()).await
    }

    async fn settings_all_export_video_configs(
        &self,
    ) -> Result<Vec<video_gen_config::Model>, AppError> {
        a4n_db::config::settings_all_export_video_configs(self.database.connection()).await
    }

    async fn settings_all_export_themes(&self) -> Result<Vec<theme_config::Model>, AppError> {
        a4n_db::config::settings_all_export_themes(self.database.connection()).await
    }

    async fn settings_all_export_system_settings(
        &self,
    ) -> Result<Vec<system_settings::Model>, AppError> {
        a4n_db::config::settings_all_export_system_settings(self.database.connection()).await
    }

    pub(super) async fn llm_max_tokens_config(&self) -> Result<LlmMaxTokensConfig, AppError> {
        a4n_db::config::llm_max_tokens_config(self.database.connection()).await
    }

    async fn save_llm_max_tokens_config(
        &self,
        config: &LlmMaxTokensConfig,
    ) -> Result<(), AppError> {
        a4n_db::config::save_llm_max_tokens_config(self.database.connection(), config).await
    }

    /// 校验全局 max_tokens 预算与默认模型输出上限后落库。
    async fn save_llm_max_tokens_config_validated(
        &self,
        config: LlmMaxTokensConfig,
    ) -> Result<LlmMaxTokensConfig, AppError> {
        config.validate().map_err(AppError::InvalidArgument)?;
        for model in self.default_llm_output_cap_rows().await? {
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
        self.save_llm_max_tokens_config(&config).await?;
        Ok(config)
    }

    pub(crate) async fn default_llm_output_cap_rows(
        &self,
    ) -> Result<Vec<DefaultLlmOutputCapRow>, AppError> {
        a4n_db::config::default_llm_output_cap_rows(self.database.connection()).await
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
    pub async fn list_user_themes(
        &self,
        user_id: i64,
    ) -> Result<Vec<theme_config::Model>, AppError> {
        self.settings.list_user_themes(user_id).await
    }

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

    /// 创建用户主题：校验 name/mode/css 后落库。
    pub async fn create_user_theme(
        &self,
        user_id: i64,
        name: String,
        mode: String,
        css_variables: Value,
    ) -> Result<theme_config::Model, AppError> {
        self.settings
            .create_theme_validated(Some(user_id), name, mode, css_variables)
            .await
    }

    /// 更新主题字段：校验后落库。
    pub async fn update_theme_fields(
        &self,
        theme_id: i64,
        name: Option<String>,
        mode: Option<String>,
        css_variables: Option<Value>,
    ) -> Result<theme_config::Model, AppError> {
        self.settings
            .update_theme_validated(theme_id, name, mode, css_variables)
            .await
    }

    /// 删除主题：校验系统默认/激活门禁后落库。
    pub async fn delete_theme_if_allowed(&self, theme_id: i64) -> Result<(), AppError> {
        self.settings.delete_theme_validated(theme_id).await
    }

    pub async fn activate_theme(&self, theme_id: i64) -> Result<theme_config::Model, AppError> {
        self.settings.activate_theme(theme_id).await
    }

    pub async fn list_llm_config_records(&self) -> Result<Vec<llm_config::Model>, AppError> {
        self.settings.list_llm_config_records().await
    }

    pub async fn llm_config_record_by_id(
        &self,
        config_id: i64,
    ) -> Result<llm_config::Model, AppError> {
        self.settings.llm_config_record_by_id(config_id).await
    }

    /// 创建 LLM 配置：规范化后落库。
    pub async fn create_llm_config_validated(
        &self,
        input: CreateLlmConfigInput<'_>,
    ) -> Result<llm_config::Model, AppError> {
        self.settings.create_llm_config_validated(input).await
    }

    /// 更新 LLM 配置：规范化、默认 max_tokens 门禁后落库。
    pub async fn update_llm_config_validated(
        &self,
        config_id: i64,
        input: UpdateLlmConfigInput<'_>,
    ) -> Result<llm_config::Model, AppError> {
        self.settings
            .update_llm_config_validated(config_id, input)
            .await
    }

    /// 设为默认 LLM 配置：模型与 max_tokens 门禁后落库。
    pub async fn set_default_llm_config_validated(
        &self,
        config_id: i64,
    ) -> Result<llm_config::Model, AppError> {
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

    pub async fn image_config_record_by_id(
        &self,
        config_id: i64,
    ) -> Result<image_gen_config::Model, AppError> {
        self.settings.image_config_record_by_id(config_id).await
    }

    /// 创建图片配置：校验参数后落库。
    pub async fn create_image_config_validated(
        &self,
        params: CreateImageConfigParams,
    ) -> Result<image_gen_config::Model, AppError> {
        self.settings.create_image_config_validated(params).await
    }

    /// 更新图片配置：读库、合并 api_key、校验后落库。
    pub async fn update_image_config_validated(
        &self,
        config_id: i64,
        params: UpdateImageConfigParams,
    ) -> Result<image_gen_config::Model, AppError> {
        self.settings
            .update_image_config_validated(config_id, params)
            .await
    }

    /// 设为默认图片配置：模型校验后落库。
    pub async fn set_default_image_config_validated(
        &self,
        config_id: i64,
    ) -> Result<image_gen_config::Model, AppError> {
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

    pub async fn video_config(&self, config_id: i64) -> Result<video_gen_config::Model, AppError> {
        self.settings.video_config(config_id).await
    }

    pub async fn list_video_config_records(
        &self,
    ) -> Result<Vec<video_gen_config::Model>, AppError> {
        self.settings.list_video_config_records().await
    }

    pub async fn video_config_record_by_id(
        &self,
        config_id: i64,
    ) -> Result<video_gen_config::Model, AppError> {
        self.settings.video_config_record_by_id(config_id).await
    }

    /// 创建视频配置：校验参数后落库。
    pub async fn create_video_config_validated(
        &self,
        params: VideoConfigParams,
    ) -> Result<video_gen_config::Model, AppError> {
        self.settings.create_video_config_validated(params).await
    }

    /// 更新视频配置：读库、合并 api_key、校验后落库。
    pub async fn update_video_config_validated(
        &self,
        config_id: i64,
        params: UpdateVideoConfigParams,
    ) -> Result<video_gen_config::Model, AppError> {
        self.settings
            .update_video_config_validated(config_id, params)
            .await
    }

    /// 设为默认视频配置：模型校验后落库。
    pub async fn set_default_video_config_validated(
        &self,
        config_id: i64,
    ) -> Result<video_gen_config::Model, AppError> {
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
        params: CreateEmbeddingConfigParams,
    ) -> Result<embedding_config::Model, AppError> {
        self.settings
            .create_embedding_config_validated(params)
            .await
    }

    /// 更新嵌入配置：合并补丁并校验后落库。
    pub async fn update_embedding_config_validated(
        &self,
        config_id: i64,
        patch: UpdateEmbeddingConfigParams,
    ) -> Result<embedding_config::Model, AppError> {
        self.settings
            .update_embedding_config_validated(config_id, patch)
            .await
    }

    /// 设为默认嵌入配置：模型校验后落库。
    pub async fn set_default_embedding_config_validated(
        &self,
        config_id: i64,
    ) -> Result<embedding_config::Model, AppError> {
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
        seed: BaselineLlmConfigSeed,
    ) -> Result<bool, AppError> {
        self.settings
            .insert_baseline_llm_config_if_empty(seed)
            .await
    }

    pub async fn insert_baseline_image_config_if_empty(
        &self,
        seed: BaselineImageConfigSeed,
    ) -> Result<bool, AppError> {
        self.settings
            .insert_baseline_image_config_if_empty(seed)
            .await
    }

    pub async fn insert_baseline_video_config_if_empty(
        &self,
        seed: BaselineVideoConfigSeed,
    ) -> Result<bool, AppError> {
        self.settings
            .insert_baseline_video_config_if_empty(seed)
            .await
    }

    pub(crate) async fn settings_all_export_llm_configs(
        &self,
    ) -> Result<Vec<llm_config::Model>, AppError> {
        self.settings.settings_all_export_llm_configs().await
    }

    pub(crate) async fn settings_all_export_embedding_configs(
        &self,
    ) -> Result<Vec<embedding_config::Model>, AppError> {
        self.settings.settings_all_export_embedding_configs().await
    }

    pub(crate) async fn settings_all_export_image_configs(
        &self,
    ) -> Result<Vec<image_gen_config::Model>, AppError> {
        self.settings.settings_all_export_image_configs().await
    }

    pub(crate) async fn settings_all_export_video_configs(
        &self,
    ) -> Result<Vec<video_gen_config::Model>, AppError> {
        self.settings.settings_all_export_video_configs().await
    }

    pub(crate) async fn settings_all_export_themes(
        &self,
    ) -> Result<Vec<theme_config::Model>, AppError> {
        self.settings.settings_all_export_themes().await
    }

    pub(crate) async fn settings_all_export_system_settings(
        &self,
    ) -> Result<Vec<system_settings::Model>, AppError> {
        self.settings.settings_all_export_system_settings().await
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

fn llm_create_record_input(normalized: NormalizedLlmConfig) -> LlmConfigCreateRecordInput {
    LlmConfigCreateRecordInput {
        name: normalized.name,
        provider: normalized.provider,
        base_url: normalized.base_url,
        api_key_encrypted: normalized.api_key_encrypted,
        model: normalized.model,
        max_concurrent: normalized.max_concurrent,
        timeout_secs: normalized.timeout_secs,
        max_context_tokens: normalized.max_context_tokens,
        max_output_tokens: normalized.max_output_tokens,
        request_options_json: normalized.request_options_json,
    }
}

fn llm_update_record_input(normalized: NormalizedLlmConfig) -> LlmConfigUpdateRecordInput {
    LlmConfigUpdateRecordInput {
        name: normalized.name,
        provider: normalized.provider,
        base_url: normalized.base_url,
        api_key_encrypted: normalized.api_key_encrypted,
        model: normalized.model,
        max_concurrent: normalized.max_concurrent,
        timeout_secs: normalized.timeout_secs,
        max_context_tokens: normalized.max_context_tokens,
        max_output_tokens: normalized.max_output_tokens,
        request_options_json: normalized.request_options_json,
    }
}
