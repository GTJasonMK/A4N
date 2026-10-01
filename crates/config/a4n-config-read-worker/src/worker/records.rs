use a4n_db::config::read_records::{
    embedding_config_read_record, image_config_read_record, llm_config_read_record,
    video_config_read_record,
};

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_db_models::models::ai::llm_config;

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
