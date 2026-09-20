//! 配置转换与 provider 合同 helper。

use super::*;

pub(in crate::context) fn has_configured_api_key(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}

pub(in crate::context) fn pricing_json(
    pricing: a4n_api_contract::config::PricingPlan,
) -> Result<Value, AppError> {
    let pricing = pricing.validate().map_err(AppError::InvalidArgument)?;
    serde_json::to_value(pricing).map_err(AppError::Serialization)
}

pub(in crate::context) fn pricing_from_json(
    value: &Value,
) -> Result<a4n_api_contract::config::PricingPlan, AppError> {
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
pub(in crate::context) fn validate_positive_config_id(config_id: i64) -> Result<(), AppError> {
    if config_id > 0 {
        return Ok(());
    }
    Err(AppError::InvalidArgument(
        "配置 ID 必须为正整数".to_string(),
    ))
}

pub(in crate::context) fn llm_provider_config_from_model(
    model: llm_config::Model,
) -> Result<LlmProviderConfig, AppError> {
    Ok(LlmProviderConfig {
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
        agent_protocol: model.agent_protocol,
        extra_body_json: model.extra_body_json,
        pricing: pricing_from_json(&model.pricing_json)?,
    })
}

pub(in crate::context) fn embedding_provider_config_from_model(
    model: embedding_config::Model,
) -> Result<EmbeddingProviderConfig, AppError> {
    Ok(EmbeddingProviderConfig {
        id: model.id,
        name: model.name,
        backend: model.backend,
        base_url: model.base_url,
        api_key: model.api_key_encrypted,
        model: model.model,
        dimension: model.dimension,
        batch_size: model.batch_size,
        local_model_path: model.local_model_path,
        pricing: pricing_from_json(&model.pricing_json)?,
    })
}

pub(in crate::context) fn image_provider_config_from_model(
    model: image_gen_config::Model,
) -> Result<ImageProviderConfig, AppError> {
    Ok(ImageProviderConfig {
        id: model.id,
        name: model.name,
        backend: model.backend,
        base_url: model.base_url,
        api_key: model.api_key_encrypted,
        model: model.model,
        max_concurrent: model.max_concurrent,
        supported_sizes_json: model.supported_sizes_json,
        pricing: pricing_from_json(&model.pricing_json)?,
    })
}

pub(in crate::context) fn image_config_for_connectivity_test_from_model(
    model: image_gen_config::Model,
) -> Result<ImageConfigForConnectivityTest, AppError> {
    Ok(ImageConfigForConnectivityTest {
        id: model.id,
        name: model.name,
        backend: model.backend,
        base_url: model.base_url,
        api_key: model.api_key_encrypted,
        model: model.model,
        max_concurrent: model.max_concurrent,
        supported_sizes_json: model.supported_sizes_json,
        pricing: pricing_from_json(&model.pricing_json)?,
    })
}

pub(in crate::context) fn llm_config_summary(
    config: &llm_config::Model,
) -> Result<LlmConfigSummary, AppError> {
    Ok(LlmConfigSummary {
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
        agent_protocol: config.agent_protocol.clone(),
        extra_body_json: config.extra_body_json.clone(),
        is_default: config.is_default,
        pricing: pricing_from_json(&config.pricing_json)?,
    })
}

pub(in crate::context) fn image_config_summary(
    config: &image_gen_config::Model,
) -> Result<ImageConfigSummary, AppError> {
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
        pricing: pricing_from_json(&config.pricing_json)?,
    })
}

#[async_trait::async_trait]
impl VideoProviderConfigPort for OrchestratorContext {
    async fn by_id(&self, config_id: i64) -> Result<VideoProviderConfig, AppError> {
        self.video_config_by_id(config_id)
            .await
            .and_then(video_provider_config_checked)
    }

    async fn default(&self) -> Result<VideoProviderConfig, AppError> {
        self.default_video_config()
            .await
            .and_then(video_provider_config_checked)
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

pub(in crate::context) fn video_provider_config_checked(
    config: video_gen_config::Model,
) -> Result<VideoProviderConfig, AppError> {
    video_provider_config_from_model(&config).map_err(|error| AppError::InvalidState {
        current: error,
        operation: "读取视频 Provider 价格配置".to_string(),
    })
}

#[async_trait::async_trait]
impl a4n_api_contract::image::ImageProviderConfigPort for OrchestratorContext {
    async fn by_id(&self, config_id: i64) -> Result<ImageProviderConfig, AppError> {
        self.image_config_by_id(config_id)
            .await
            .and_then(image_provider_config_from_model)
    }

    async fn default(&self) -> Result<ImageProviderConfig, AppError> {
        self.resolve_image_config(None)
            .await
            .and_then(image_provider_config_from_model)
    }
}

pub(in crate::context) fn video_config_summary(
    config: &video_gen_config::Model,
) -> Result<VideoConfigSummary, AppError> {
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
        pricing: pricing_from_json(&config.pricing_json)?,
    })
}

pub(in crate::context) fn embedding_config_summary(
    config: &embedding_config::Model,
) -> Result<EmbeddingConfigSummary, AppError> {
    Ok(EmbeddingConfigSummary {
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
        pricing: pricing_from_json(&config.pricing_json)?,
    })
}

pub(in crate::context) fn normalize_llm_config_input(
    input: CreateLlmInput,
) -> Result<NormalizedLlmConfig, AppError> {
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
        agent_protocol,
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
        agent_protocol,
        extra_body_json,
    })?;
    normalized.pricing_json = pricing_json(pricing)?;
    Ok(normalized)
}

pub(in crate::context) fn image_config_params(
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

pub(in crate::context) fn video_config_params(
    input: VideoConfigInput,
) -> Result<(VideoConfigParams, Value), AppError> {
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

pub(in crate::context) fn embedding_config_params(
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

pub(in crate::context) fn llm_record_input(
    normalized: NormalizedLlmConfig,
) -> LlmConfigRecordInput {
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
        agent_protocol: normalized.agent_protocol,
        extra_body_json: normalized.extra_body_json,
        pricing_json: normalized.pricing_json,
    }
}

pub(in crate::context) fn llm_config_snapshot(model: &llm_config::Model) -> LlmConfigSnapshot {
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
        agent_protocol: model.agent_protocol.clone(),
        extra_body_json: model.extra_body_json.clone(),
    }
}

pub(in crate::context) fn image_record_input(
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

pub(in crate::context) fn video_record_input(
    params: VideoConfigParams,
    pricing_json: Value,
) -> VideoConfigRecordInput {
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

pub(in crate::context) fn embedding_record_input(
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

pub(in crate::context) fn merge_api_key_update(
    update: Option<Option<String>>,
    existing: Option<String>,
) -> Option<String> {
    update.unwrap_or(existing)
}
