use a4n_provider_contract::{
    EmbeddingProviderConfig, ImageProviderConfig, LlmProviderConfig, VideoProviderConfig,
};

use super::{RuntimeEmbeddingConfig, RuntimeImageConfig, RuntimeLlmConfig, RuntimeVideoConfig};

pub(super) fn image_provider_config_from_runtime(cfg: RuntimeImageConfig) -> ImageProviderConfig {
    ImageProviderConfig {
        id: cfg.id,
        name: cfg.name,
        backend: cfg.backend,
        base_url: cfg.base_url,
        api_key: cfg.api_key_encrypted,
        model: cfg.model,
        max_concurrent: cfg.max_concurrent,
        supported_sizes_json: serde_json::to_value(cfg.supported_sizes)
            .expect("ImageSize 只包含 JSON 可序列化字段"),
        pricing: cfg.pricing,
    }
}

pub(super) fn video_provider_config_from_runtime(cfg: RuntimeVideoConfig) -> VideoProviderConfig {
    VideoProviderConfig {
        id: cfg.id,
        name: cfg.name,
        backend: cfg.backend,
        base_url: cfg.base_url,
        api_key: cfg.api_key_encrypted,
        model: cfg.model,
        max_concurrent: cfg.max_concurrent,
        default_width: cfg.default_width,
        default_height: cfg.default_height,
        default_duration_secs: cfg.default_duration_secs,
        min_duration_secs: cfg.min_duration_secs,
        max_duration_secs: cfg.max_duration_secs,
        duration_mode: cfg.duration_mode,
        supported_durations_json: cfg.supported_durations_json,
        default_fps: cfg.default_fps,
        is_default: cfg.is_default,
        pricing: cfg.pricing,
    }
}

pub(super) fn llm_provider_config_from_runtime(cfg: RuntimeLlmConfig) -> LlmProviderConfig {
    LlmProviderConfig {
        id: cfg.id,
        name: cfg.name,
        provider: cfg.provider,
        base_url: cfg.base_url,
        api_key: cfg.api_key_encrypted,
        model: cfg.model,
        max_concurrent: cfg.max_concurrent,
        timeout_secs: cfg.timeout_secs,
        max_context_tokens: cfg.max_context_tokens,
        max_output_tokens: cfg.max_output_tokens,
        extra_body_json: cfg.extra_body_json,
        pricing: cfg.pricing,
    }
}

pub(super) fn embedding_provider_config_from_runtime(
    cfg: RuntimeEmbeddingConfig,
) -> EmbeddingProviderConfig {
    EmbeddingProviderConfig {
        id: cfg.id,
        name: cfg.name,
        backend: cfg.backend,
        base_url: cfg.base_url,
        api_key: cfg.api_key_encrypted,
        model: cfg.model,
        dimension: cfg.dimension,
        batch_size: cfg.batch_size,
        local_model_path: cfg.local_model_path,
        pricing: cfg.pricing,
    }
}
