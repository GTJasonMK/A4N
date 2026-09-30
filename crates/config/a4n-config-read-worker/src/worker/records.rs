use a4n_config_read_core::{
    EmbeddingConfigReadRecord, ImageConfigReadRecord, LlmConfigReadRecord, VideoConfigReadRecord,
};
use a4n_db_models::models::ai::{embedding_config, image_gen_config, llm_config, video_gen_config};

fn llm_config_read_record(model: &llm_config::Model) -> LlmConfigReadRecord {
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

fn embedding_config_read_record(model: &embedding_config::Model) -> EmbeddingConfigReadRecord {
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

fn image_config_read_record(model: &image_gen_config::Model) -> ImageConfigReadRecord {
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

fn video_config_read_record(model: &video_gen_config::Model) -> VideoConfigReadRecord {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_record_preserves_persisted_provider_fields() {
        let model = llm_config::Model {
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
            agent_protocol: "native_tool_calling".into(),
            extra_body_json: None,
            pricing_json: Default::default(),
            is_default: true,
            created_at: Default::default(),
            updated_at: Default::default(),
        };
        let record = llm_config_read_record(&model);
        assert_eq!(record.id, model.id);
        assert_eq!(record.api_key_encrypted, model.api_key_encrypted);
        assert_eq!(record.model, model.model);
        assert_eq!(record.pricing_json, model.pricing_json);
        assert_eq!(record.is_default, model.is_default);
    }
}
