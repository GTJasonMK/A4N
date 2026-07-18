use a4n_core::models::ai::{embedding_config, image_gen_config, llm_config, video_gen_config};
use chrono::Utc;

use super::contracts::{
    RuntimeEmbeddingConfig, RuntimeImageConfig, RuntimeLlmConfig, RuntimeVideoConfig,
};

fn now() -> chrono::NaiveDateTime {
    Utc::now().naive_utc()
}

pub(super) fn llm_model_from_runtime(cfg: RuntimeLlmConfig) -> llm_config::Model {
    let timestamp = now();
    llm_config::Model {
        id: cfg.id.unwrap_or(-1),
        name: cfg.name,
        provider: cfg.provider,
        base_url: cfg.base_url,
        api_key_encrypted: cfg.api_key_encrypted,
        model: cfg.model,
        max_concurrent: cfg.max_concurrent,
        timeout_secs: cfg.timeout_secs,
        max_context_tokens: cfg.max_context_tokens,
        max_output_tokens: cfg.max_output_tokens,
        request_options_json: cfg.request_options_json,
        is_default: cfg.is_default,
        created_at: timestamp,
        updated_at: timestamp,
    }
}

pub(super) fn image_model_from_runtime(cfg: RuntimeImageConfig) -> image_gen_config::Model {
    let timestamp = now();
    image_gen_config::Model {
        id: cfg.id.unwrap_or(-1),
        name: cfg.name,
        backend: cfg.backend,
        base_url: cfg.base_url,
        api_key_encrypted: cfg.api_key_encrypted,
        model: cfg.model,
        max_concurrent: cfg.max_concurrent,
        default_width: cfg.default_width,
        default_height: cfg.default_height,
        provider_options_json: cfg.provider_options_json,
        is_default: cfg.is_default,
        created_at: timestamp,
        updated_at: timestamp,
    }
}

pub(super) fn video_model_from_runtime(cfg: RuntimeVideoConfig) -> video_gen_config::Model {
    let timestamp = now();
    video_gen_config::Model {
        id: cfg.id.unwrap_or(-1),
        name: cfg.name,
        backend: cfg.backend,
        base_url: cfg.base_url,
        api_key_encrypted: cfg.api_key_encrypted,
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
        provider_options_json: cfg.provider_options_json,
        is_default: cfg.is_default,
        created_at: timestamp,
        updated_at: timestamp,
    }
}

pub(super) fn embedding_model_from_runtime(cfg: RuntimeEmbeddingConfig) -> embedding_config::Model {
    let timestamp = now();
    embedding_config::Model {
        id: cfg.id.unwrap_or(-1),
        name: cfg.name,
        backend: cfg.backend,
        base_url: cfg.base_url,
        api_key_encrypted: cfg.api_key_encrypted,
        model: cfg.model,
        dimension: cfg.dimension,
        batch_size: cfg.batch_size,
        local_model_path: cfg.local_model_path,
        is_default: cfg.is_default,
        created_at: timestamp,
        updated_at: timestamp,
    }
}
