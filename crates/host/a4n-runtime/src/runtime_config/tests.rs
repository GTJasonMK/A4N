use super::access::current_image_provider_config_for_id;
use super::redaction::redacted_snapshot_value;
use super::*;
use a4n_domain::ImageSize;
use serde_json::json;

fn test_llm_config(id: i64, is_default: bool) -> RuntimeLlmConfig {
    RuntimeLlmConfig {
        id,
        name: format!("llm {id}"),
        provider: "openai".to_string(),
        base_url: "https://example.test/v1".to_string(),
        api_key_encrypted: Some("sk-test".to_string()),
        model: format!("gpt-test-{id}"),
        max_concurrent: 1,
        timeout_secs: 30,
        max_context_tokens: 128_000,
        max_output_tokens: 8_192,
        agent_protocol: "native_tool_calling".to_string(),
        extra_body_json: None,
        is_default,
        pricing: Default::default(),
    }
}

fn test_image_config(id: i64, is_default: bool) -> RuntimeImageConfig {
    RuntimeImageConfig {
        id,
        name: format!("image {id}"),
        backend: "openai".to_string(),
        base_url: Some("https://example.test/v1".to_string()),
        api_key_encrypted: Some("sk-test".to_string()),
        model: format!("image-test-{id}"),
        max_concurrent: 1,
        supported_sizes: vec![ImageSize::new(1024, 1024)],
        is_default,
        pricing: Default::default(),
    }
}

fn test_video_config(id: i64, is_default: bool) -> RuntimeVideoConfig {
    RuntimeVideoConfig {
        id,
        name: format!("video {id}"),
        backend: "openai".to_string(),
        base_url: Some("https://example.test/v1".to_string()),
        api_key_encrypted: Some("sk-test".to_string()),
        model: format!("video-test-{id}"),
        max_concurrent: 1,
        default_width: 1280,
        default_height: 720,
        default_duration_secs: 5,
        min_duration_secs: 5,
        max_duration_secs: 5,
        duration_mode: "fixed".to_string(),
        supported_durations_json: None,
        default_fps: 24,
        is_default,
        pricing: Default::default(),
    }
}

#[tokio::test]
async fn explicit_config_id_lookup_accepts_active_or_list_config() {
    let snapshot = RuntimeConfigSnapshot {
        schema_version: 2,
        llm: Some(test_llm_config(1, true)),
        llm_configs: vec![test_llm_config(2, false)],
        image: Some(test_image_config(1, true)),
        image_configs: vec![test_image_config(2, false)],
        video: Some(test_video_config(1, true)),
        video_configs: vec![test_video_config(2, false)],
        embedding: None,
        web_fetch: None,
        web_search: None,
        llm_default_source: None,
        image_default_source: None,
        video_default_source: None,
        system_llm_api_key_encrypted: None,
        system_image_api_key_encrypted: None,
        system_video_api_key_encrypted: None,
        client_context: None,
    };

    scope(snapshot, async {
        assert_eq!(current_llm_provider_config().expect("default LLM").id, 1);
        assert_eq!(
            current_image_provider_config_for_id(None)
                .expect("default image")
                .id,
            1
        );
        assert_eq!(
            current_video_provider_config().expect("default video").id,
            1
        );

        assert_eq!(
            current_llm_provider_config_for_id(Some(1))
                .expect("active LLM by ID")
                .id,
            1
        );
        assert_eq!(
            current_image_provider_config_for_id(Some(1))
                .expect("active image by ID")
                .id,
            1
        );
        assert_eq!(
            current_video_provider_config_for_id(Some(1))
                .expect("active video by ID")
                .id,
            1
        );

        assert_eq!(
            current_llm_provider_config_for_id(Some(2))
                .expect("LLM by ID")
                .id,
            2
        );
        assert_eq!(
            current_image_provider_config_for_id(Some(2))
                .expect("image by ID")
                .id,
            2
        );
        assert_eq!(
            current_image_provider_config_for_id(Some(2))
                .expect("provider image by ID")
                .id,
            2
        );
        assert_eq!(
            current_video_provider_config_for_id(Some(2))
                .expect("video by ID")
                .id,
            2
        );
    })
    .await;
}

#[tokio::test]
async fn config_lists_do_not_define_runtime_defaults() {
    let snapshot = RuntimeConfigSnapshot {
        schema_version: 2,
        llm: None,
        llm_configs: vec![test_llm_config(2, true)],
        image: None,
        image_configs: vec![test_image_config(3, true)],
        video: None,
        video_configs: vec![test_video_config(4, true)],
        embedding: None,
        web_fetch: None,
        web_search: None,
        llm_default_source: None,
        image_default_source: None,
        video_default_source: None,
        system_llm_api_key_encrypted: None,
        system_image_api_key_encrypted: None,
        system_video_api_key_encrypted: None,
        client_context: None,
    };

    scope(snapshot, async {
        assert!(current_llm_provider_config().is_none());
        assert!(current_image_provider_config_for_id(None).is_none());
        assert!(current_video_provider_config().is_none());

        assert_eq!(
            current_llm_provider_config_for_id(Some(2))
                .expect("LLM by ID")
                .id,
            2
        );
        assert_eq!(
            current_image_provider_config_for_id(Some(3))
                .expect("image by ID")
                .id,
            3
        );
        assert_eq!(
            current_video_provider_config_for_id(Some(4))
                .expect("video by ID")
                .id,
            4
        );
    })
    .await;
}

#[test]
fn snapshot_from_args_rejects_malformed_snapshot() {
    let args = json!({
        "runtime_config_snapshot": {
            "schema_version": 2,
            "llm": {
                "name": "local",
                "provider": "openai",
                "base_url": "https://example.test/v1",
                "model": "gpt-test",
                "max_concurrent": 1,
                "timeout_secs": 30,
                "max_context_tokens": 128000,
                "max_output_tokens": 8192
            }
        }
    });

    let err = snapshot_from_args(&args).expect_err("malformed snapshot must fail explicitly");
    assert!(err.contains("runtime_config_snapshot 无效"));
    assert!(err.contains("missing field"), "{err}");
    assert!(err.contains("id"), "{err}");

    let mut invalid_id = args;
    invalid_id["runtime_config_snapshot"]["llm"]["id"] = json!(0);
    let err = snapshot_from_args(&invalid_id).expect_err("non-positive config ID must fail");
    assert!(err.contains("运行时配置 ID 必须是正整数"));
}

#[test]
fn snapshot_from_args_rejects_unsupported_schema_version() {
    let args = json!({
        "runtime_config_snapshot": {
            "schema_version": 1
        }
    });

    let err = snapshot_from_args(&args).expect_err("unsupported snapshot must fail explicitly");
    assert!(err.contains("schema_version 不支持: 1"), "{err}");
}

#[test]
fn snapshot_from_args_requires_runtime_llm_token_cap_fields() {
    let args = json!({
        "runtime_config_snapshot": {
            "schema_version": 2,
            "llm": {
                "id": 1,
                "name": "local",
                "provider": "openai",
                "base_url": "https://example.test/v1",
                "api_key_encrypted": "sk-test",
                "model": "gpt-test",
                "max_concurrent": 1,
                "timeout_secs": 30,
                "is_default": true
            }
        }
    });

    let err = snapshot_from_args(&args).expect_err("runtime LLM caps are required");

    assert!(err.contains("runtime_config_snapshot 无效"));
    assert!(err.contains("max_context_tokens"), "{err}");
}

#[test]
fn snapshot_from_args_rejects_camel_case_compatibility_fields() {
    let args = json!({
        "runtime_config_snapshot": {
            "schemaVersion": 1
        }
    });

    let err = snapshot_from_args(&args)
        .expect_err("camelCase runtime snapshot compatibility must be removed");

    assert!(err.contains("unknown field `schemaVersion`"), "{err}");
}

#[test]
fn snapshot_from_args_rejects_unknown_nested_config_fields() {
    let args = json!({
        "runtime_config_snapshot": {
            "schema_version": 2,
            "llm": {
                "id": 1,
                "name": "local",
                "provider": "openai",
                "base_url": "https://example.test/v1",
                "api_key_encrypted": "sk-test",
                "model": "gpt-test",
                "max_concurrent": 1,
                "timeout_secs": 30,
                "max_context_tokens": 128000,
                "max_output_tokens": 8192,
                "is_default": true,
                "legacy_field": true
            }
        }
    });

    let err = snapshot_from_args(&args)
        .expect_err("unknown nested runtime config fields must be rejected");

    assert!(err.contains("unknown field `legacy_field`"), "{err}");
}

#[test]
fn redacted_snapshot_value_uses_shared_sidecar_sensitive_field_policy() {
    let snapshot = RuntimeConfigSnapshot {
        schema_version: 2,
        llm: Some(RuntimeLlmConfig {
            id: 1,
            name: "local llm".to_string(),
            provider: "openai".to_string(),
            base_url: "https://example.test".to_string(),
            api_key_encrypted: Some("sk-secret".to_string()),
            model: "gpt-test".to_string(),
            max_concurrent: 1,
            timeout_secs: 30,
            max_context_tokens: 128_000,
            max_output_tokens: 8_192,
            agent_protocol: "native_tool_calling".to_string(),
            extra_body_json: None,
            is_default: true,
            pricing: Default::default(),
        }),
        llm_configs: Vec::new(),
        image: None,
        image_configs: Vec::new(),
        video: None,
        video_configs: Vec::new(),
        embedding: None,
        web_fetch: None,
        web_search: None,
        llm_default_source: None,
        image_default_source: Some("system".to_string()),
        video_default_source: Some("system".to_string()),
        system_llm_api_key_encrypted: Some("sk-system-secret".to_string()),
        system_image_api_key_encrypted: Some("sk-image-secret".to_string()),
        system_video_api_key_encrypted: Some("sk-video-secret".to_string()),
        client_context: Some(json!({
            "access_token": "token-secret",
            "safe": "visible"
        })),
    };

    let value = redacted_snapshot_value(&snapshot);
    assert_eq!(value["llm"]["base_url"], "[REDACTED]");
    assert_eq!(value["llm"]["api_key_encrypted"], "[REDACTED]");
    assert_eq!(value["system_llm_api_key_encrypted"], "[REDACTED]");
    assert_eq!(value["system_image_api_key_encrypted"], "[REDACTED]");
    assert_eq!(value["system_video_api_key_encrypted"], "[REDACTED]");
    assert_eq!(value["client_context"]["access_token"], "[REDACTED]");
    assert_eq!(value["client_context"]["safe"], "visible");
    let serialized = value.to_string();
    assert!(!serialized.contains("example.test"));
    assert!(!serialized.contains("sk-secret"));
    assert!(!serialized.contains("token-secret"));
}
