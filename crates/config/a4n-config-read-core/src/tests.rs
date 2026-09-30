use super::{
    EmbeddingConfigReadRecord, ImageConfigReadRecord, LlmConfigReadRecord,
    embedding_provider_config, image_connectivity_config, llm_provider_config, pricing_from_json,
};
use serde_json::json;

#[test]
fn invalid_pricing_json_is_rejected_instead_of_defaulting() {
    assert!(pricing_from_json(&json!({"currency": "US"})).is_err());
}

#[test]
fn connectivity_config_mapping_preserves_provider_fields() {
    let llm = llm_provider_config(LlmConfigReadRecord {
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
        extra_body_json: Some(json!({"reasoning": {"effort": "low"}})),
        pricing_json: json!({}),
        is_default: true,
    })
    .unwrap();
    assert_eq!(llm.id, 7);
    assert_eq!(llm.api_key.as_deref(), Some("secret"));
    assert_eq!(
        llm.extra_body_json,
        Some(json!({"reasoning": {"effort": "low"}}))
    );

    let embedding = embedding_provider_config(EmbeddingConfigReadRecord {
        id: 9,
        name: "embedding".into(),
        backend: "api".into(),
        base_url: Some("https://example.test".into()),
        api_key_encrypted: Some("embedding-secret".into()),
        model: "embedding-model".into(),
        dimension: 768,
        batch_size: 16,
        local_model_path: Some("/tmp/model".into()),
        pricing_json: json!({}),
        is_default: false,
    })
    .unwrap();
    assert_eq!(embedding.id, 9);
    assert_eq!(embedding.api_key.as_deref(), Some("embedding-secret"));
    assert_eq!(embedding.local_model_path.as_deref(), Some("/tmp/model"));

    let image = image_connectivity_config(ImageConfigReadRecord {
        id: 11,
        name: "image".into(),
        backend: "openai-compatible-image".into(),
        base_url: Some("https://example.test/v1".into()),
        api_key_encrypted: Some("image-secret".into()),
        model: "image-model".into(),
        max_concurrent: 1,
        supported_sizes_json: json!([{"width": 1024, "height": 1024}]),
        pricing_json: json!({}),
        is_default: true,
    })
    .unwrap();
    assert_eq!(image.id, 11);
    assert_eq!(image.api_key.as_deref(), Some("image-secret"));
    assert_eq!(image.base_url.as_deref(), Some("https://example.test/v1"));
    assert_eq!(
        image.supported_sizes_json,
        json!([{"width": 1024, "height": 1024}])
    );
}
