use super::super::shared::has_configured_api_key;
use a4n_api_contract::config::EmbeddingConfigSummary;
use a4n_core::models::ai::embedding_config;

pub(super) fn embedding_config_summary(config: &embedding_config::Model) -> EmbeddingConfigSummary {
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
    }
}

#[cfg(test)]
mod tests {
    use a4n_api_contract::config::{CreateEmbeddingInput, UpdateEmbeddingInput};

    #[test]
    fn create_embedding_input_requires_batch_size() {
        let err = serde_json::from_value::<CreateEmbeddingInput>(serde_json::json!({
            "name": "missing batch",
            "backend": "api",
            "base_url": "https://api.openai.com/v1",
            "model": "text-embedding-3-small",
            "dimension": 1536
        }))
        .expect_err("batch_size is required by the current create contract");
        assert!(err.to_string().contains("batch_size"));
    }

    #[test]
    fn update_embedding_input_distinguishes_api_key_update_intents() {
        let base = serde_json::json!({
            "name": "embedding",
            "backend": "api",
            "base_url": "https://api.example/v1",
            "model": "embedding-model",
            "dimension": 1536,
            "batch_size": 32,
            "local_model_path": null
        });
        let missing_key = serde_json::from_value::<UpdateEmbeddingInput>(base.clone())
            .expect("missing api_key_encrypted should preserve the old key");
        assert_eq!(missing_key.api_key_encrypted, None);
        let mut clear_value = base.clone();
        clear_value["api_key_encrypted"] = serde_json::Value::Null;
        let clear_key = serde_json::from_value::<UpdateEmbeddingInput>(clear_value)
            .expect("null api_key_encrypted should be an explicit clear request");
        assert_eq!(clear_key.api_key_encrypted, Some(None));
        let mut replace_value = base;
        replace_value["api_key_encrypted"] = serde_json::Value::String("sk-new".into());
        let replace_key = serde_json::from_value::<UpdateEmbeddingInput>(replace_value)
            .expect("string api_key_encrypted should replace the old key");
        assert_eq!(
            replace_key.api_key_encrypted,
            Some(Some("sk-new".to_string()))
        );
    }
}
