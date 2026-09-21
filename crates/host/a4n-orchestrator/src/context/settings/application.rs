//! 配置数据库 owner。

use super::*;

impl ConfigApplication {
    pub(in crate::context) async fn start_model_usage_record(
        &self,
        input: ModelUsageRecordStart,
    ) -> Result<a4n_db_models::models::generation::model_usage_record::Model, AppError> {
        a4n_db::model_usage::start_model_usage_record(self.database.connection(), input).await
    }

    pub(in crate::context) async fn finish_model_usage_record(
        &self,
        record_id: i64,
        input: ModelUsageRecordCompletion,
    ) -> Result<(), AppError> {
        a4n_db::model_usage::finish_model_usage_record(self.database.connection(), record_id, input)
            .await
    }

    pub(in crate::context) async fn set_default_provider_config(
        &self,
        kind: ProviderConfigKind,
        config_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::config::set_default_provider_config(self.database.connection(), kind, config_id)
            .await
    }

    pub(in crate::context) async fn list_themes_visible_to_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<theme_config::Model>, AppError> {
        a4n_db::config::list_themes_visible_to_user(self.database.connection(), user_id).await
    }

    pub(in crate::context) async fn theme_by_id(
        &self,
        theme_id: i64,
    ) -> Result<Option<theme_config::Model>, AppError> {
        a4n_db::config::theme_by_id(self.database.connection(), theme_id).await
    }

    pub(in crate::context) async fn create_theme_record(
        &self,
        user_id: Option<i64>,
        name: &str,
        mode: ThemeMode,
        document: ThemeDocument,
    ) -> Result<theme_config::Model, AppError> {
        a4n_db::config::create_theme(self.database.connection(), user_id, name, mode, document)
            .await
    }

    pub(in crate::context) async fn update_theme_record(
        &self,
        theme: theme_config::Model,
        name: Option<&str>,
        mode: Option<ThemeMode>,
        document: Option<ThemeDocument>,
    ) -> Result<theme_config::Model, AppError> {
        a4n_db::config::update_theme(self.database.connection(), theme, name, mode, document).await
    }

    pub(in crate::context) async fn delete_theme_record(
        &self,
        theme_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::config::delete_theme(self.database.connection(), theme_id).await
    }

    pub(in crate::context) async fn activate_theme(
        &self,
        theme_id: i64,
    ) -> Result<theme_config::Model, AppError> {
        a4n_db::config::activate_theme(self.database.connection(), theme_id).await
    }

    pub(in crate::context) async fn list_theme_resources(
        &self,
        theme_id: i64,
    ) -> Result<Vec<a4n_db_models::models::user::theme_resource::Model>, AppError> {
        a4n_db::config::list_theme_resources(self.database.connection(), theme_id).await
    }

    pub(in crate::context) async fn theme_resource_by_id(
        &self,
        resource_id: i64,
    ) -> Result<Option<a4n_db_models::models::user::theme_resource::Model>, AppError> {
        a4n_db::config::theme_resource_by_id(self.database.connection(), resource_id).await
    }

    pub(in crate::context) async fn upsert_theme_resource(
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

    pub(in crate::context) async fn delete_theme_resource(
        &self,
        resource_id: i64,
    ) -> Result<a4n_db_models::models::user::theme_resource::Model, AppError> {
        a4n_db::config::delete_theme_resource(self.database.connection(), resource_id).await
    }

    pub(in crate::context) async fn upsert_theme_preference(
        &self,
        user_id: i64,
        appearance: a4n_domain::ThemeAppearance,
    ) -> Result<a4n_db_models::models::user::theme_preference::Model, AppError> {
        a4n_db::config::upsert_theme_preference(self.database.connection(), user_id, appearance)
            .await
    }

    pub(in crate::context) async fn list_llm_config_records(
        &self,
    ) -> Result<Vec<llm_config::Model>, AppError> {
        a4n_db::config::list_llm_config_records(self.database.connection()).await
    }

    pub(in crate::context) async fn llm_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<llm_config::Model, AppError> {
        a4n_db::config::llm_config_by_id(self.database.connection(), config_id).await
    }

    /// 规范化 LLM 创建输入后落库。
    pub(in crate::context) async fn create_llm_config_validated(
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
    pub(in crate::context) async fn update_llm_config_validated(
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
                agent_protocol: input.agent_protocol.clone(),
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
    pub(in crate::context) async fn set_default_llm_config_validated(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        let target = self.llm_config_by_id(config_id).await?;
        validate_llm_config(&llm_config_snapshot(&target))?;
        self.set_default_provider_config(ProviderConfigKind::Llm, config_id)
            .await
    }

    /// 非默认门禁后删除 LLM 配置。
    pub(in crate::context) async fn delete_llm_config_if_allowed(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        let target = self.llm_config_by_id(config_id).await?;
        ensure_default_config_delete_allowed(" LLM 配置", target.is_default)?;
        a4n_db::config::delete_llm_config_record(self.database.connection(), config_id).await
    }

    pub(in crate::context) async fn resolve_llm_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<llm_config::Model, AppError> {
        a4n_db::config::resolve_llm_config(self.database.connection(), config_id).await
    }

    pub(in crate::context) async fn list_image_config_records(
        &self,
    ) -> Result<Vec<image_gen_config::Model>, AppError> {
        a4n_db::config::list_image_config_records(self.database.connection()).await
    }

    pub(in crate::context) async fn image_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<image_gen_config::Model, AppError> {
        a4n_db::config::image_config_by_id(self.database.connection(), config_id).await
    }

    /// 校验图片配置参数后创建。
    pub(in crate::context) async fn create_image_config_validated(
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
    pub(in crate::context) async fn update_image_config_validated(
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
    pub(in crate::context) async fn set_default_image_config_validated(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        let target = self.image_config_by_id(config_id).await?;
        validate_image_config_model(&target)?;
        self.set_default_provider_config(ProviderConfigKind::Image, config_id)
            .await
    }

    /// 非默认门禁通过后删除图片配置。
    pub(in crate::context) async fn delete_image_config_if_allowed(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        let target = self.image_config_by_id(config_id).await?;
        ensure_default_config_delete_allowed("图片配置", target.is_default)?;
        a4n_db::config::delete_image_config_record(self.database.connection(), config_id).await
    }

    pub(in crate::context) async fn resolve_image_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<image_gen_config::Model, AppError> {
        a4n_db::config::resolve_image_config(self.database.connection(), config_id).await
    }

    pub(in crate::context) async fn video_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<video_gen_config::Model, AppError> {
        a4n_db::config::video_config_by_id(self.database.connection(), config_id).await
    }

    pub(in crate::context) async fn list_video_config_records(
        &self,
    ) -> Result<Vec<video_gen_config::Model>, AppError> {
        a4n_db::config::list_video_config_records(self.database.connection()).await
    }

    pub(in crate::context) async fn default_video_config(
        &self,
    ) -> Result<video_gen_config::Model, AppError> {
        a4n_db::config::default_video_config(self.database.connection()).await
    }

    pub(in crate::context) async fn active_generated_video_ids_for_config(
        &self,
        config_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::config::active_generated_video_ids_for_config(self.database.connection(), config_id)
            .await
    }

    /// 校验视频配置参数后创建。
    pub(in crate::context) async fn create_video_config_validated(
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
    pub(in crate::context) async fn update_video_config_validated(
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
    pub(in crate::context) async fn set_default_video_config_validated(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        let target = self.video_config_by_id(config_id).await?;
        validate_video_config_model(&target).map_err(AppError::InvalidArgument)?;
        self.set_default_provider_config(ProviderConfigKind::Video, config_id)
            .await
    }

    /// 非默认且无进行中任务时删除视频配置。
    pub(in crate::context) async fn delete_video_config_if_allowed(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
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

    pub(in crate::context) async fn default_embedding_config_optional(
        &self,
    ) -> Result<Option<embedding_config::Model>, AppError> {
        a4n_db::config::default_embedding_config_optional(self.database.connection()).await
    }

    pub(in crate::context) async fn default_embedding_config(
        &self,
    ) -> Result<embedding_config::Model, AppError> {
        a4n_db::config::default_embedding_config(self.database.connection()).await
    }

    pub(in crate::context) async fn list_embedding_configs(
        &self,
    ) -> Result<Vec<embedding_config::Model>, AppError> {
        a4n_db::config::list_embedding_configs(self.database.connection()).await
    }

    pub(in crate::context) async fn embedding_config_by_id(
        &self,
        config_id: i64,
    ) -> Result<embedding_config::Model, AppError> {
        a4n_db::config::embedding_config_by_id(self.database.connection(), config_id).await
    }

    /// 校验嵌入配置参数后创建。
    pub(in crate::context) async fn create_embedding_config_validated(
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
    pub(in crate::context) async fn update_embedding_config_validated(
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
    pub(in crate::context) async fn set_default_embedding_config_validated(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        let target = self.embedding_config_by_id(config_id).await?;
        validate_embedding_config_model(&target)?;
        self.set_default_provider_config(ProviderConfigKind::Embedding, config_id)
            .await
    }

    /// 非默认门禁后删除嵌入配置。
    pub(in crate::context) async fn delete_embedding_config_if_allowed(
        &self,
        config_id: i64,
    ) -> Result<(), AppError> {
        let target = self.embedding_config_by_id(config_id).await?;
        ensure_default_config_delete_allowed(" Embedding 配置", target.is_default)?;
        a4n_db::config::delete_embedding_config_record(self.database.connection(), config_id).await
    }

    pub(in crate::context) async fn llm_configs_exist(&self) -> Result<bool, AppError> {
        a4n_db::config::llm_configs_exist(self.database.connection()).await
    }

    pub(in crate::context) async fn image_configs_exist(&self) -> Result<bool, AppError> {
        a4n_db::config::image_configs_exist(self.database.connection()).await
    }

    pub(in crate::context) async fn video_configs_exist(&self) -> Result<bool, AppError> {
        a4n_db::config::video_configs_exist(self.database.connection()).await
    }

    pub(in crate::context) async fn insert_baseline_embedding_config_if_empty(
        &self,
    ) -> Result<bool, AppError> {
        a4n_db::config::insert_baseline_embedding_config_if_empty(self.database.connection()).await
    }

    pub(in crate::context) async fn insert_baseline_llm_config_if_empty(
        &self,
        config: NormalizedLlmConfig,
    ) -> Result<bool, AppError> {
        a4n_db::config::insert_baseline_llm_config_if_empty(
            self.database.connection(),
            llm_record_input(config),
        )
        .await
    }

    pub(in crate::context) async fn insert_baseline_image_config_if_empty(
        &self,
        config: CreateImageConfigParams,
    ) -> Result<bool, AppError> {
        a4n_db::config::insert_baseline_image_config_if_empty(
            self.database.connection(),
            image_record_input(config, serde_json::json!({})),
        )
        .await
    }

    pub(in crate::context) async fn insert_baseline_video_config_if_empty(
        &self,
        config: VideoConfigParams,
    ) -> Result<bool, AppError> {
        a4n_db::config::insert_baseline_video_config_if_empty(
            self.database.connection(),
            video_record_input(config, serde_json::json!({})),
        )
        .await
    }
}
