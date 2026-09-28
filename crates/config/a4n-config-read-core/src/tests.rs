use super::{
    embedding_provider_config as embedding_provider_config_from_model,
    image_connectivity_config as image_config_for_connectivity_test_from_model,
    llm_provider_config as llm_provider_config_from_model, pricing_from_json,
};
use a4n_db_models::models::ai::{embedding_config, image_gen_config, llm_config};
use serde_json::json;

#[test]
fn invalid_pricing_json_is_rejected_instead_of_defaulting() {
    assert!(pricing_from_json(&json!({"currency": "US"})).is_err());
}

#[test]
fn connectivity_config_mapping_preserves_provider_fields() {
    let llm = llm_provider_config_from_model(llm_config::Model {
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
        created_at: Default::default(),
        updated_at: Default::default(),
    })
    .unwrap();
    assert_eq!(llm.id, 7);
    assert_eq!(llm.api_key.as_deref(), Some("secret"));
    assert_eq!(
        llm.extra_body_json,
        Some(json!({"reasoning": {"effort": "low"}}))
    );

    let embedding = embedding_provider_config_from_model(embedding_config::Model {
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
        created_at: Default::default(),
        updated_at: Default::default(),
    })
    .unwrap();
    assert_eq!(embedding.id, 9);
    assert_eq!(embedding.api_key.as_deref(), Some("embedding-secret"));
    assert_eq!(embedding.local_model_path.as_deref(), Some("/tmp/model"));

    let image = image_config_for_connectivity_test_from_model(image_gen_config::Model {
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
        created_at: Default::default(),
        updated_at: Default::default(),
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
