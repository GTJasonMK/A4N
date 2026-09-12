use super::connectivity::validate_llm_runtime_snapshot_config_id;
use super::contracts::{CreateLlmInput, UpdateLlmInput};
use super::*;
use a4n_provider_contract::LlmProviderConfig;

#[test]
fn create_llm_input_requires_model_token_limits() {
    let err = serde_json::from_value::<CreateLlmInput>(serde_json::json!({
        "name": "missing context cap",
        "provider": "openai",
        "base_url": "https://api.example/v1",
        "api_key_encrypted": "sk-test",
        "model": "gpt-test",
        "max_concurrent": 1,
        "timeout_secs": 30,
        "max_output_tokens": 8192,
        "extra_body_json": null
    }))
    .expect_err("max_context_tokens is required by the current create contract");
    assert!(err.to_string().contains("max_context_tokens"));

    let err = serde_json::from_value::<CreateLlmInput>(serde_json::json!({
        "name": "missing output cap",
        "provider": "openai",
        "base_url": "https://api.example/v1",
        "api_key_encrypted": "sk-test",
        "model": "gpt-test",
        "max_concurrent": 1,
        "timeout_secs": 30,
        "max_context_tokens": 128000,
        "extra_body_json": null
    }))
    .expect_err("max_output_tokens is required by the current create contract");
    assert!(err.to_string().contains("max_output_tokens"));
}

#[test]
fn update_llm_input_distinguishes_api_key_update_intents() {
    let base = serde_json::json!({
        "name": "keep existing key",
        "provider": "openai",
        "base_url": "https://api.example/v1",
        "model": "gpt-test",
        "max_concurrent": 1,
        "timeout_secs": 30,
        "max_context_tokens": 128000,
        "max_output_tokens": 8192,
        "extra_body_json": null
    });
    let missing_key = serde_json::from_value::<UpdateLlmInput>(base.clone())
        .expect("missing api_key_encrypted should preserve the old key");
    assert_eq!(missing_key.api_key_encrypted, None);

    let mut clear_value = base.clone();
    clear_value["api_key_encrypted"] = serde_json::Value::Null;
    let clear_key = serde_json::from_value::<UpdateLlmInput>(clear_value)
        .expect("null api_key_encrypted should be an explicit clear request");
    assert_eq!(clear_key.api_key_encrypted, Some(None));

    let mut replace_value = base;
    replace_value["api_key_encrypted"] = serde_json::Value::String("sk-new".to_string());
    let replace_key = serde_json::from_value::<UpdateLlmInput>(replace_value)
        .expect("string api_key_encrypted should replace the old key");
    assert_eq!(
        replace_key.api_key_encrypted,
        Some(Some("sk-new".to_string()))
    );
}

#[test]
fn llm_connectivity_test_requires_snapshot_to_match_config_id() {
    let config = test_llm_config_model(7, 4096);

    validate_llm_runtime_snapshot_config_id(&config, 7)
        .expect("matching snapshot config id should be accepted");
    let err = validate_llm_runtime_snapshot_config_id(&config, 8)
        .expect_err("mismatched snapshot config id must be rejected");

    assert!(err.contains("runtime_config_snapshot.llm.id 必须等于 config_id"));
    assert!(err.contains("snapshot_id=7"));
    assert!(err.contains("config_id=8"));
}

#[test]
fn runtime_snapshot_config_id_error_names_non_llm_config_kind() {
    let err = validate_runtime_snapshot_config_id("image", 3, 4)
        .expect_err("mismatched non-LLM snapshot config id must be rejected");

    assert!(err.contains("runtime_config_snapshot.image.id 必须等于 config_id"));
    assert!(err.contains("snapshot_id=3"));
    assert!(err.contains("config_id=4"));
}

fn test_llm_config_model(id: i64, max_output_tokens: i32) -> LlmProviderConfig {
    LlmProviderConfig {
        id,
        name: format!("test llm {id}"),
        provider: "openai".to_string(),
        base_url: "https://api.example/v1".to_string(),
        api_key: Some("sk-test".to_string()),
        model: "gpt-test".to_string(),
        max_concurrent: 1,
        timeout_secs: 30,
        max_context_tokens: 128_000,
        max_output_tokens,
        extra_body_json: None,
    }
}
