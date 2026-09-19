//! Pure projections for persisted provider configuration records.

use std::str::FromStr;

use a4n_api_contract::config::{
    EmbeddingConfigSummary, ImageConfigForConnectivityTest, ImageConfigSummary, LlmConfigSummary,
    VideoConfigSummary, VideoDurationMode,
};
use a4n_db_models::models::ai::{embedding_config, image_gen_config, llm_config, video_gen_config};
use a4n_error::AppError;
use a4n_provider_contract::{
    EmbeddingProviderConfig, ImageProviderConfig, LlmProviderConfig, VideoProviderConfig,
};
use serde_json::Value;

fn has_configured_api_key(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}

fn pricing_from_json(value: &Value) -> Result<a4n_domain::cost::PricingPlan, AppError> {
    let pricing: a4n_domain::cost::PricingPlan =
        serde_json::from_value(value.clone()).map_err(|error| AppError::InvalidState {
            current: error.to_string(),
            operation: "读取 Provider 价格计划".to_string(),
        })?;
    pricing.validate().map_err(|error| AppError::InvalidState {
        current: error,
        operation: "校验 Provider 价格计划".to_string(),
    })
}

pub fn llm_provider_config(model: llm_config::Model) -> Result<LlmProviderConfig, AppError> {
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

pub fn embedding_provider_config(
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

pub fn image_provider_config(
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

pub fn image_connectivity_config(
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

pub fn video_provider_config(
    model: video_gen_config::Model,
) -> Result<VideoProviderConfig, AppError> {
    Ok(VideoProviderConfig {
        id: model.id,
        name: model.name,
        backend: model.backend,
        base_url: model.base_url,
        api_key: model.api_key_encrypted,
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
        is_default: model.is_default,
        pricing: pricing_from_json(&model.pricing_json)?,
    })
}

pub fn llm_summary(model: &llm_config::Model) -> Result<LlmConfigSummary, AppError> {
    Ok(LlmConfigSummary {
        id: model.id,
        name: model.name.clone(),
        provider: model.provider.clone(),
        base_url: model.base_url.clone(),
        has_api_key: has_configured_api_key(model.api_key_encrypted.as_deref()),
        model: model.model.clone(),
        max_concurrent: model.max_concurrent,
        timeout_secs: model.timeout_secs,
        max_context_tokens: model.max_context_tokens,
        max_output_tokens: model.max_output_tokens,
        agent_protocol: model.agent_protocol.clone(),
        extra_body_json: model.extra_body_json.clone(),
        is_default: model.is_default,
        pricing: pricing_from_json(&model.pricing_json)?,
    })
}

pub fn image_summary(model: &image_gen_config::Model) -> Result<ImageConfigSummary, AppError> {
    Ok(ImageConfigSummary {
        id: model.id,
        name: model.name.clone(),
        backend: model.backend.clone(),
        base_url: model
            .base_url
            .clone()
            .ok_or_else(|| AppError::InvalidState {
                current: format!("图片配置 #{} 缺少 base_url", model.id),
                operation: "投影图片配置摘要".to_string(),
            })?,
        has_api_key: has_configured_api_key(model.api_key_encrypted.as_deref()),
        model: model.model.clone(),
        max_concurrent: model.max_concurrent,
        supported_sizes: a4n_config::image_sizes_from_json(&model.supported_sizes_json).map_err(
            |error| AppError::InvalidState {
                current: error.to_string(),
                operation: "投影图片配置尺寸".to_string(),
            },
        )?,
        is_default: model.is_default,
        pricing: pricing_from_json(&model.pricing_json)?,
    })
}

pub fn video_summary(model: &video_gen_config::Model) -> Result<VideoConfigSummary, AppError> {
    Ok(VideoConfigSummary {
        id: model.id,
        name: model.name.clone(),
        backend: model.backend.clone(),
        base_url: model
            .base_url
            .clone()
            .ok_or_else(|| AppError::InvalidState {
                current: format!("视频配置 #{} 缺少 base_url", model.id),
                operation: "投影视频配置摘要".to_string(),
            })?,
        has_api_key: has_configured_api_key(model.api_key_encrypted.as_deref()),
        model: model.model.clone(),
        max_concurrent: model.max_concurrent,
        default_width: model.default_width,
        default_height: model.default_height,
        default_duration_secs: model.default_duration_secs,
        min_duration_secs: model.min_duration_secs,
        max_duration_secs: model.max_duration_secs,
        duration_mode: VideoDurationMode::from_str(&model.duration_mode).map_err(|error| {
            AppError::InvalidState {
                current: error.to_string(),
                operation: "投影视频配置 duration_mode".to_string(),
            }
        })?,
        supported_durations_json: model.supported_durations_json.clone(),
        default_fps: model.default_fps,
        is_default: model.is_default,
        pricing: pricing_from_json(&model.pricing_json)?,
    })
}

pub fn embedding_summary(
    model: &embedding_config::Model,
) -> Result<EmbeddingConfigSummary, AppError> {
    Ok(EmbeddingConfigSummary {
        id: model.id,
        name: model.name.clone(),
        backend: model.backend.clone(),
        base_url: model.base_url.clone(),
        has_api_key: has_configured_api_key(model.api_key_encrypted.as_deref()),
        model: model.model.clone(),
        dimension: model.dimension,
        batch_size: model.batch_size,
        local_model_path: model.local_model_path.clone(),
        is_default: model.is_default,
        pricing: pricing_from_json(&model.pricing_json)?,
    })
}
