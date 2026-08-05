use super::*;

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
    let missing_key = serde_json::from_value::<UpdateLlmInput>(serde_json::json!({
        "name": "keep existing key"
    }))
    .expect("missing api_key_encrypted should preserve the old key");
    assert_eq!(missing_key.api_key_encrypted, None);

    let clear_key = serde_json::from_value::<UpdateLlmInput>(serde_json::json!({
        "api_key_encrypted": null
    }))
    .expect("null api_key_encrypted should be an explicit clear request");
    assert_eq!(clear_key.api_key_encrypted, Some(None));

    let replace_key = serde_json::from_value::<UpdateLlmInput>(serde_json::json!({
        "api_key_encrypted": "sk-new"
    }))
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

#[test]
fn llm_connectivity_test_rejects_connection_tokens_above_tested_model_cap() {
    let config = test_llm_config_model(7, 4);

    let err = validate_llm_connection_test_max_tokens(8, &config)
        .expect_err("connection_test above tested model output cap must fail");

    assert!(err.contains("LLM 连接测试 max_tokens 不能超过被测配置模型最大输出"));
    assert!(err.contains("config_id=7"));
    assert!(err.contains("connection_test=8"));
    assert!(err.contains("model_max_output_tokens=4"));
}

fn test_llm_config_model(id: i64, max_output_tokens: i32) -> llm_config::Model {
    let now = chrono::Utc::now().naive_utc();
    llm_config::Model {
        id,
        name: format!("test llm {id}"),
        provider: "openai".to_string(),
        base_url: "https://api.example/v1".to_string(),
        api_key_encrypted: Some("sk-test".to_string()),
        model: "gpt-test".to_string(),
        max_concurrent: 1,
        timeout_secs: 30,
        max_context_tokens: 128_000,
        max_output_tokens,
        extra_body_json: None,
        is_default: false,
        created_at: now,
        updated_at: now,
    }
}
