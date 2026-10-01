use a4n_db_models::models::ai::{embedding_config, image_gen_config, llm_config, video_gen_config};
use a4n_domain::config::{
    EmbeddingConfigReadRecord, ImageConfigReadRecord, LlmConfigReadRecord, VideoConfigReadRecord,
};

pub fn llm_config_read_record(model: &llm_config::Model) -> LlmConfigReadRecord {
    LlmConfigReadRecord {
        id: model.id,
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
        agent_protocol: model.agent_protocol.clone(),
        pricing_json: model.pricing_json.clone(),
        is_default: model.is_default,
    }
}

pub fn embedding_config_read_record(model: &embedding_config::Model) -> EmbeddingConfigReadRecord {
    EmbeddingConfigReadRecord {
        id: model.id,
        name: model.name.clone(),
        backend: model.backend.clone(),
        base_url: model.base_url.clone(),
        api_key_encrypted: model.api_key_encrypted.clone(),
        model: model.model.clone(),
        dimension: model.dimension,
        batch_size: model.batch_size,
        local_model_path: model.local_model_path.clone(),
        pricing_json: model.pricing_json.clone(),
        is_default: model.is_default,
    }
}

pub fn image_config_read_record(model: &image_gen_config::Model) -> ImageConfigReadRecord {
    ImageConfigReadRecord {
        id: model.id,
        name: model.name.clone(),
        backend: model.backend.clone(),
        base_url: model.base_url.clone(),
        api_key_encrypted: model.api_key_encrypted.clone(),
        model: model.model.clone(),
        max_concurrent: model.max_concurrent,
        supported_sizes_json: model.supported_sizes_json.clone(),
        pricing_json: model.pricing_json.clone(),
        is_default: model.is_default,
    }
}

pub fn video_config_read_record(model: &video_gen_config::Model) -> VideoConfigReadRecord {
    VideoConfigReadRecord {
        id: model.id,
        name: model.name.clone(),
        backend: model.backend.clone(),
        base_url: model.base_url.clone(),
        api_key_encrypted: model.api_key_encrypted.clone(),
        model: model.model.clone(),
        max_concurrent: model.max_concurrent,
        default_width: model.default_width,
        default_height: model.default_height,
        default_duration_secs: model.default_duration_secs,
        min_duration_secs: model.min_duration_secs,
        max_duration_secs: model.max_duration_secs,
        duration_mode: model.duration_mode.clone(),
        supported_durations_json: model.supported_durations_json.clone(),
        default_fps: model.default_fps,
        is_default: model.is_default,
        pricing_json: model.pricing_json.clone(),
    }
}
